use crate::FunctionError;
use hyper::server::accept::Accept;
use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::Path;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::net::UnixListener;
use url::Url;

pub struct UDS(UnixListener);

impl UDS {
    pub fn new() -> Result<Self, FunctionError> {
        if let Ok(fn_format) = std::env::var("FN_FORMAT") {
            if fn_format.as_str() != "http-stream" && fn_format.as_str() != "" {
                return Err(FunctionError::initialization(format!(
                    "Unsupported FN_FORMAT specified: {}",
                    fn_format
                )));
            }
        };

        let socket_url = match std::env::var("FN_LISTENER") {
            Ok(value) => Url::parse(&value)
                .unwrap_or_else(|_| panic!("Malformed FN_LISTENER specified: {}", value)),
            Err(_) => {
                return Err(FunctionError::initialization(
                    "FN_LISTENER not found in env",
                ))
            }
        };

        if socket_url.scheme() != "unix" || socket_url.path() == "" {
            return Err(FunctionError::initialization(format!(
                "Malformed FN_LISTENER specified: {}",
                socket_url.as_str()
            )));
        }

        let socket_file_path = Path::new(socket_url.path());
        let phony_socket_file_path = Path::new(socket_file_path.parent().unwrap()).join(format!(
            "phony{}",
            socket_file_path.file_name().unwrap().to_str().unwrap()
        ));

        // Try to clean up old sockets
        {
            let _ = fs::remove_file(&socket_file_path);
            let _ = fs::remove_file(&phony_socket_file_path);
        }

        let listener = match UnixListener::bind(&phony_socket_file_path.to_str().unwrap()) {
            Ok(value) => value,
            Err(e) => {
                return Err(FunctionError::initialization(format!(
                    "Error while creating the listener: {}",
                    e
                )));
            }
        };

        let socket = UDS(listener);
        // Set permissions to 0o666 and set symlink
        {
            let _ = match std::fs::set_permissions(
                &phony_socket_file_path,
                fs::Permissions::from_mode(0o666),
            ) {
                Ok(_) => (),
                Err(e) => {
                    return Err(FunctionError::initialization(format!(
                        "Error while giving permissions to socket file: {}",
                        e
                    )))
                }
            };

            let _ = match symlink(
                &phony_socket_file_path
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap(),
                socket_file_path,
            ) {
                Ok(_) => (),
                Err(e) => {
                    return Err(FunctionError::initialization(format!(
                        "Error while creating symlink: {}",
                        e
                    )))
                }
            };
        }
        Ok(socket)
    }
}

impl Accept for UDS {
    type Conn = tokio::net::UnixStream;
    type Error = FunctionError;

    fn poll_accept(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Self::Conn, Self::Error>>> {
        match self.0.poll_accept(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok((socket, _address))) => Poll::Ready(Some(Ok(socket))),
            Poll::Ready(Err(err)) => Poll::Ready(Some(Err(err.into()))),
        }
    }
}
