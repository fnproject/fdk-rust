use hyper;
use futures::{Future, Stream};

use std::io::Write;

use errors::FunctionError;

/// A utility function that consumes the body of a request or response and
/// returns it as a vector of bytes. Note: this consumes and buffers the stream.
pub fn body_as_bytes(b: hyper::Body) -> Result<Vec<u8>, FunctionError> {
    match b.concat2()
        .map(|chunk| chunk.iter().cloned().collect::<Vec<u8>>())
        .wait() {
        Ok(v) => Ok(v),
        Err(e) => Err(FunctionError::io(e)),
    }
}

/// A utility function that produces a successful response with no content.
pub fn no_content() -> hyper::Response {
    hyper::Response::new().with_status(hyper::StatusCode::NoContent)
}

/// A utility function that produces a successful response from a type that can
/// be converted to a vector of bytes.
pub fn success<T>(data: T) -> hyper::Response
where
    T: Into<Vec<u8>>,
{
    let bytes: Vec<u8> = data.into();
    hyper::Response::new()
        .with_status(hyper::StatusCode::Ok)
        .with_header(hyper::header::ContentLength(bytes.len() as u64))
        .with_body(bytes)
}

/// A utility function that produces a client error response from a type that
/// can be converted to a vector of bytes.
pub fn client_error<T>(data: T) -> hyper::Response
where
    T: Into<Vec<u8>>,
{
    let bytes: Vec<u8> = data.into();
    hyper::Response::new()
        .with_status(hyper::StatusCode::BadRequest)
        .with_header(hyper::header::ContentLength(bytes.len() as u64))
        .with_body(bytes)
}

/// A utility function that produces a server error response from a type that
/// can be converted to a vector of bytes.
pub fn server_error<T>(data: T) -> hyper::Response
where
    T: Into<Vec<u8>>,
{
    let bytes: Vec<u8> = data.into();
    hyper::Response::new()
        .with_status(hyper::StatusCode::InternalServerError)
        .with_header(hyper::header::ContentLength(bytes.len() as u64))
        .with_body(bytes)
}


/// A utility function to consume a hyper::Request and splat it into a Write.
/// Note: this buffers the stream.
pub fn write_request_full(req: hyper::Request, writer: &mut Write) -> Result<(), FunctionError> {
    let (method, uri, version, headers, body) = req.deconstruct();
    match writer.write_all(format!("{} {} {}\r\n", method, uri, version).as_bytes()) {
        Ok(_) => (),
        Err(e) => return Err(FunctionError::io(e)),
    };
    for hv in headers.iter() {
        match writer.write_all(
            format!("{}: {}\r\n", hv.name(), hv.value_string()).as_bytes(),
        ) {
            Ok(_) => (),
            Err(e) => return Err(FunctionError::io(e)),
        }
    }
    match writer.write_all(format!("\r\n").as_bytes()) {
        Ok(_) => (),
        Err(e) => return Err(FunctionError::io(e)),
    };
    match body_as_bytes(body) {
        Ok(bytes) => {
            match writer.write_all(&bytes) {
                Ok(_) => {
                    match writer.flush() {
                        Ok(_) => Ok(()),
                        Err(e) => Err(FunctionError::io(e)),
                    }
                }
                Err(e) => Err(FunctionError::io(e)),
            }
        }
        Err(e) => Err(e),
    }
}

/// A utility function to consume a hyper::Response and only write its Body into
/// a Write. Note: this buffers the stream.
pub fn write_response_body(resp: hyper::Response, writer: &mut Write) -> Result<(), FunctionError> {
    match body_as_bytes(resp.body()) {
        Ok(bytes) => {
            match writer.write_all(&bytes) {
                Ok(_) => {
                    match writer.flush() {
                        Ok(_) => Ok(()),
                        Err(e) => Err(FunctionError::io(e)),
                    }
                }
                Err(e) => Err(FunctionError::io(e)),
            }
        }
        Err(e) => Err(e),
    }
}

/// A utility function to consume a hyper::Response and splat it into a Write.
/// Note: this buffers the stream.
pub fn write_response_full(resp: hyper::Response, writer: &mut Write) -> Result<(), FunctionError> {
    match writer.write_all(
        format!("{} {}\r\n", resp.version(), resp.status()).as_bytes(),
    ) {
        Ok(_) => (),
        Err(e) => return Err(FunctionError::io(e)),
    };
    for hv in resp.headers().iter() {
        match writer.write_all(
            format!("{}: {}\r\n", hv.name(), hv.value_string()).as_bytes(),
        ) {
            Ok(_) => (),
            Err(e) => return Err(FunctionError::io(e)),
        }
    }
    match writer.write_all(format!("\r\n").as_bytes()) {
        Ok(_) => (),
        Err(e) => return Err(FunctionError::io(e)),
    }
    write_response_body(resp, writer)
}

/// A utility function to determine what should be the exit code of the process
/// after producing the specified response.
pub fn exit_code_from_response(resp: &hyper::Response) -> i32 {
    if resp.status().is_server_error() {
        2
    } else {
        if resp.status().is_client_error() {
            1
        } else {
            0
        }
    }
}

/// A utility function to clone a hyper::Response. Usually used for testing.
pub fn clone_response(resp: hyper::Response) -> (hyper::Response, hyper::Response) {
    let mut r1 = hyper::Response::new()
        .with_status(resp.status())
        .with_headers(resp.headers().clone());
    let mut r2 = hyper::Response::new()
        .with_status(resp.status())
        .with_headers(resp.headers().clone());
    let bbytes = body_as_bytes(resp.body()).unwrap();
    r1.set_body(bbytes.clone());
    r2.set_body(bbytes);
    (r1, r2)
}
