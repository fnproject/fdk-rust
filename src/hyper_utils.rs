use hyper::{body::Bytes, Body, HeaderMap, Response};

use crate::errors::FunctionError;
use hyper::header::HeaderName;
use hyper::http::HeaderValue;
use std::io::Write;

fn generic_response(
    status: hyper::StatusCode,
    body: Option<Body>,
    headers: Option<HeaderMap>,
) -> Response<Body> {
    let mut builder = Response::builder().status(status);
    if let Some(headers) = headers {
        let resp_headers = builder.headers_mut().unwrap();
        *resp_headers = headers;
    }
    let mut response_body = Body::empty();
    if let Some(body) = body {
        response_body = body;
    }
    builder.body(response_body).unwrap()
}

fn make_header_map_with_single_value(key: HeaderName, value: HeaderValue) -> HeaderMap {
    let mut header_map = HeaderMap::new();
    header_map.insert(key, value);
    header_map
}

/// A utility function that consumes the body of a request or response and
/// returns it as a vector of bytes. Note: this consumes and buffers the stream.
pub async fn body_as_bytes(b: Body) -> Result<Bytes, FunctionError> {
    match hyper::body::to_bytes(b).await {
        Ok(body_bytes) => Ok(body_bytes),
        Err(err) => Err(FunctionError::io(err)),
    }
}

/// A utility function that produces a successful response with no content.
pub fn no_content() -> hyper::Response<Body> {
    generic_response(hyper::StatusCode::NO_CONTENT, None, None)
}

/// A utility function that produces a successful response from a type that can
/// be converted to a vector of bytes.
pub fn success<T>(data: T) -> hyper::Response<Body>
where
    T: Into<Vec<u8>>,
{
    let bytes: Vec<u8> = data.into().to_owned();
    let content_length = bytes.len();
    generic_response(
        hyper::StatusCode::OK,
        Option::from(Body::from(bytes)),
        Option::from(make_header_map_with_single_value(
            hyper::header::CONTENT_LENGTH,
            content_length.into(),
        )),
    )
}

/// A utility function that produces a client error response from a type that
/// can be converted to a vector of bytes.
pub fn client_error<T>(data: T) -> Response<Body>
where
    T: Into<Vec<u8>>,
{
    let bytes: Vec<u8> = data.into().to_owned();
    let content_length = bytes.len();
    generic_response(
        hyper::StatusCode::BAD_REQUEST,
        Option::from(Body::from(bytes)),
        Option::from(make_header_map_with_single_value(
            hyper::header::CONTENT_LENGTH,
            content_length.into(),
        )),
    )
}

/// A utility function that produces a server error response from a type that
/// can be converted to a vector of bytes.
pub fn server_error<T>(data: T) -> Response<Body>
where
    T: Into<Vec<u8>>,
{
    let bytes: Vec<u8> = data.into().to_owned();
    let content_length = bytes.len();
    generic_response(
        hyper::StatusCode::INTERNAL_SERVER_ERROR,
        Option::from(Body::from(bytes)),
        Option::from(make_header_map_with_single_value(
            hyper::header::CONTENT_LENGTH,
            content_length.into(),
        )),
    )
}

/// A utility function to consume a hyper::Request and splat it into a Write.
/// Note: this buffers the stream.
pub async fn write_request_full(
    req: hyper::Request<Body>,
    writer: &mut dyn Write,
) -> Result<(), FunctionError> {
    match writer.write_all(
        format!(
            "{} {} {:?}\r\n",
            req.method().to_string(),
            req.uri().to_string(),
            req.version()
        )
        .as_bytes(),
    ) {
        Ok(_) => (),
        Err(e) => return Err(FunctionError::io(e)),
    };
    for hv in req.headers().iter() {
        match writer.write_all(format!("{}: {}\r\n", hv.0, hv.1.to_str().unwrap()).as_bytes()) {
            Ok(_) => (),
            Err(e) => return Err(FunctionError::io(e)),
        }
    }
    match writer.write_all(format!("\r\n").as_bytes()) {
        Ok(_) => (),
        Err(e) => return Err(FunctionError::io(e)),
    };
    match body_as_bytes(req.into_body()).await {
        Ok(bytes) => match writer.write_all(&bytes) {
            Ok(_) => match writer.flush() {
                Ok(_) => Ok(()),
                Err(e) => Err(FunctionError::io(e)),
            },
            Err(e) => Err(FunctionError::io(e)),
        },
        Err(e) => Err(e),
    }
}

/// A utility function to consume a hyper::Response and only write its Body into
/// a Write. Note: this buffers the stream.
pub async fn write_response_body(
    resp: hyper::Response<Body>,
    writer: &mut dyn Write,
) -> Result<(), FunctionError> {
    match body_as_bytes(resp.into_body()).await {
        Ok(bytes) => match writer.write_all(&bytes) {
            Ok(_) => match writer.flush() {
                Ok(_) => Ok(()),
                Err(e) => Err(FunctionError::io(e)),
            },
            Err(e) => Err(FunctionError::io(e)),
        },
        Err(e) => Err(e),
    }
}

/// A utility function to consume a hyper::Response and splat it into a Write.
/// Note: this buffers the stream.
pub async fn write_response_full(
    resp: hyper::Response<Body>,
    writer: &mut dyn Write,
) -> Result<(), FunctionError> {
    match writer.write_all(format!("{:?} {}\r\n", resp.version(), resp.status()).as_bytes()) {
        Ok(_) => (),
        Err(e) => return Err(FunctionError::io(e)),
    };
    for hv in resp.headers().iter() {
        match writer.write_all(format!("{}: {}\r\n", hv.0, hv.1.to_str().unwrap()).as_bytes()) {
            Ok(_) => (),
            Err(e) => return Err(FunctionError::io(e)),
        }
    }
    match writer.write_all(format!("\r\n").as_bytes()) {
        Ok(_) => (),
        Err(e) => return Err(FunctionError::io(e)),
    }
    write_response_body(resp, writer).await
}

/// A utility function to determine what should be the exit code of the process
/// after producing the specified response.
pub fn exit_code_from_response(resp: &hyper::Response<Body>) -> i32 {
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
pub async fn clone_response(
    resp: hyper::Response<Body>,
) -> (hyper::Response<Body>, hyper::Response<Body>) {
    let headers = resp.headers().clone();
    let status = resp.status();
    if let Ok(bbytes) = body_as_bytes(resp.into_body()).await {
        (
            generic_response(
                status,
                Option::from(Body::from(bbytes.clone())),
                Option::from(headers.clone()),
            ),
            generic_response(
                status,
                Option::from(Body::from(bbytes.clone())),
                Option::from(headers.clone()),
            ),
        )
    } else {
        panic!("Error while cloning the response")
    }
}
