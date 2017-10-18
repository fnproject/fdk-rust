use futures;
use hyper;
use uuid;

use std::borrow::Cow;
use std::collections::HashMap;
use std::io::{Read, Write, BufReader};
use std::net;
use std::str::FromStr;
use std::sync::mpsc;
use std::thread;

use errors::FunctionError;
use hyper_utils::{write_response_body, write_response_full};

pub trait InputOutputCodec
    : Iterator<Item = Result<hyper::Request, FunctionError>> {
    fn try_write(&mut self, resp: hyper::Response, writer: &mut Write)
        -> Result<(), FunctionError>;
}


pub struct DefaultCodec<'a> {
    input: Box<Read>,
    environment: &'a HashMap<String, String>,
    read: bool,
}

impl<'a> DefaultCodec<'a> {
    pub fn new(input: Box<Read>, environment: &'a HashMap<String, String>) -> DefaultCodec<'a> {
        DefaultCodec {
            input: input,
            environment: environment,
            read: false,
        }
    }
}

impl<'a> Iterator for DefaultCodec<'a> {
    type Item = Result<hyper::Request, FunctionError>;
    fn next(&mut self) -> Option<Result<hyper::Request, FunctionError>> {
        match self.read {
            true => None,
            false => {
                self.read = true;
                let mut body = Vec::new();
                match self.input.read_to_end(&mut body) {
                    Ok(_) => {
                        // Method, URI, version
                        let method = match self.environment.get("FN_METHOD") {
                            Some(s) => {
                                match hyper::Method::from_str(s) {
                                    Ok(m) => m,
                                    Err(_) => {
                                        return Some(Err(FunctionError::other(
                                            "Fatal: FN_METHOD set to an invalid HTTP method.",
                                        )))
                                    }
                                }
                            }
                            None => {
                                return Some(Err(FunctionError::other("Fatal: FN_METHOD not set.")))
                            }
                        };
                        let uri = match self.environment.get("FN_REQUEST_URL") {
                            Some(s) => {
                                match hyper::Uri::from_str(s) {
                                    Ok(u) => u,
                                    Err(_) => {
                                        return Some(Err(FunctionError::other(
                                            "Fatal: FN_REQUEST_URL set to an invalid URL.",
                                        )))
                                    }
                                }
                            }
                            None => {
                                return Some(
                                    Err(FunctionError::other("Fatal: FN_REQUEST_URL not set.")),
                                )
                            }
                        };
                        let version = hyper::HttpVersion::Http11;
                        let mut req = hyper::Request::new(method, uri);
                        req.set_version(version);
                        // Construct headers
                        const HEADER_PREFIX: &'static str = "fn_header_";
                        self.environment
                            .iter()
                            .filter(|kv| kv.0.to_lowercase().starts_with(HEADER_PREFIX))
                            .fold(HashMap::new(), |mut hs, kv| {
                                let k: String = kv.0.clone().split_off(HEADER_PREFIX.len());
                                hs.insert(k, kv.1.clone());
                                hs
                            })
                            .iter()
                            .fold(req.headers_mut(), |hs, kv| {
                                hs.append_raw(
                                    Cow::Owned(String::from(kv.0.as_str())),
                                    kv.1.as_str(),
                                );
                                hs
                            });
                        // Body
                        req.set_body(hyper::Body::from(body));
                        // Return request
                        Some(Ok(req))
                    }
                    Err(e) => Some(Err(FunctionError::io(e))),
                }
            }
        }
    }
}

impl<'a> InputOutputCodec for DefaultCodec<'a> {
    fn try_write(
        &mut self,
        resp: hyper::Response,
        writer: &mut Write,
    ) -> Result<(), FunctionError> {
        // The 'default' contract for Fn does not allow to set headers or status
        // in the response. We can only write the body to stdout.
        write_response_body(resp, writer)
    }
}


pub struct HttpCodec {
    event_rx: mpsc::Receiver<Option<Result<hyper::Request, FunctionError>>>,
}

impl HttpCodec {
    pub fn new(input: Box<Read + Send>) -> HttpCodec {
        let (event_tx, event_rx) = mpsc::channel();
        let event_tx_clone = event_tx.clone();
        let shutdown_key_uuid = uuid::Uuid::new_v4();
        let shutdown_value_uuid = uuid::Uuid::new_v4();

        let codec = HttpCodec { event_rx: event_rx };

        let mut loopback_addr = "127.0.0.1:0".parse().unwrap();

        // Set up the server thread.
        let (ready_tx, ready_rx) = mpsc::channel();
        thread::spawn(move || {
            let server = hyper::server::Http::new()
                .bind(&loopback_addr, move || {
                    Ok(ChannelPoster {
                        event_tx: event_tx.clone(),
                        shutdown_key_uuid: shutdown_key_uuid,
                        shutdown_value_uuid: shutdown_value_uuid,
                    })
                })
                .unwrap();
            ready_tx.send(server.local_addr().unwrap()).unwrap();
            // The current implementation of run_until() seems broken and it
            // double-panics when the future resolves. This should be the
            // way to terminate the server with a message from another
            // thread, but we can't currently use it. This means that the
            // TCP socket bound to the server stays open until the process
            // ends - OK for production, where there's only one server, but
            // not very good for the test harness which instantiates several
            // servers in parallel. It's a bit of a waste of sockets.
            // let _ = server.run_until(
            //     shutdown_rx.into_future().then(|_| futures::future::ok(())));
            let _ = server.run();
        });
        loopback_addr = ready_rx.recv().unwrap();

        // Tcp streams to the server thread and back. If we cannot set up the
        // streams, send a failure message immediately and return the codec with
        // just the failure in the queue.
        let stream = match net::TcpStream::connect(loopback_addr) {
            Ok(s) => s,
            Err(e) => {
                event_tx_clone
                    .send(Some(Err(FunctionError::io(e))))
                    .unwrap();
                return codec;
            }
        };
        let mut stream_for_push = match stream.try_clone() {
            Ok(s) => s,
            Err(e) => {
                event_tx_clone
                    .send(Some(Err(FunctionError::io(e))))
                    .unwrap();
                return codec;
            }
        };
        let stream_for_pull = match stream.try_clone() {
            Ok(s) => s,
            Err(e) => {
                event_tx_clone
                    .send(Some(Err(FunctionError::io(e))))
                    .unwrap();
                return codec;
            }
        };

        // Push thread: read input and push it to the server thread.
        thread::spawn(move || {
            let bufinput = BufReader::new(input);
            bufinput.bytes().fold((), |_, maybe| {
                match maybe {
                    Ok(b) => {
                        // Probably very inefficient, but necessary to avoid delays
                        match stream_for_push.write(&[b]) {
                            Ok(_) => (),
                            Err(e) => {
                                event_tx_clone
                                    .send(Some(Err(FunctionError::io(e))))
                                    .unwrap();
                            }
                        }
                    }
                    Err(e) => {
                        event_tx_clone
                            .send(Some(Err(FunctionError::io(e))))
                            .unwrap();
                    }
                };
            });
            // Send the shutdown request since we've finished.
            match stream_for_push.write(
                format!(
                    "HEAD * HTTP/1.1\r\n{}: {}\r\n\r\n",
                    shutdown_key_uuid.hyphenated().to_string(),
                    shutdown_value_uuid.hyphenated().to_string()
                ).as_bytes(),
            ) {
                Ok(_) => (),
                Err(e) => {
                    event_tx_clone
                        .send(Some(Err(FunctionError::io(e))))
                        .unwrap();
                }
            }
            stream_for_push.flush().unwrap();
        });

        // Pull thread: just consume bytes from the stream.
        thread::spawn(move || { stream_for_pull.bytes().count(); });

        // Return the fully functional codec
        codec
    }
}

impl Iterator for HttpCodec {
    type Item = Result<hyper::Request, FunctionError>;
    fn next(&mut self) -> Option<Result<hyper::Request, FunctionError>> {
        match self.event_rx.recv() {
            Ok(maybe_ie) => maybe_ie,
            Err(e) => Some(Err(FunctionError::io(e))),
        }
    }
}

impl InputOutputCodec for HttpCodec {
    fn try_write(
        &mut self,
        resp: hyper::Response,
        writer: &mut Write,
    ) -> Result<(), FunctionError> {
        write_response_full(resp, writer)
    }
}

struct ChannelPoster {
    event_tx: mpsc::Sender<Option<Result<hyper::Request, FunctionError>>>,
    shutdown_key_uuid: uuid::Uuid,
    shutdown_value_uuid: uuid::Uuid,
}

impl hyper::server::Service for ChannelPoster {
    type Request = hyper::Request;
    type Response = hyper::Response;
    type Error = hyper::Error;
    type Future = Box<futures::Future<Item = Self::Response, Error = Self::Error>>;

    fn call(&self, req: hyper::Request) -> Self::Future {
        let local_tx = self.event_tx.clone();

        let is_shutdown = match req.headers().get_raw(&self.shutdown_key_uuid
            .hyphenated()
            .to_string()) {
            Some(v) => {
                match v.one() {
                    Some(vv) => vv == self.shutdown_value_uuid.hyphenated().to_string().as_bytes(),
                    None => false,
                }
            }
            None => false,
        };

        // If the codec has already died and has closed the channel, the
        // runtime is compromised anyway. This can be caused when a previous
        // unrecoverable error compromises the runtime in the main thread while
        // requests are still being processed here.
        // As a result, we can ignore errors here - if the channel is closed,
        // the program is exiting catastrophically anyway.
        if is_shutdown {
            let _ = local_tx.send(None);
        } else {
            let _ = local_tx.send(Some(Ok(req)));
        }

        Box::new(futures::future::ok(hyper::Response::new().with_body("OK")))
    }
}
