use clap::crate_version;
use hyper::{
    header::{HeaderName, HeaderValue},
    Body, HeaderMap, Response, StatusCode,
};
use std::str::FromStr;

pub fn make_header_map_with_single_value(key: HeaderName, value: HeaderValue) -> HeaderMap {
    let mut header_map = HeaderMap::new();
    header_map.insert(key, value);
    header_map
}

fn generic_response(status: StatusCode, body: Option<Body>, headers: HeaderMap) -> Response<Body> {
    let mut builder = Response::builder().status(status);
    {
        let mut headers = headers;
        headers.insert(
            "Fn-Fdk-Version",
            HeaderValue::from_str(&format!("fdk-rust/{}", crate_version!())).unwrap(),
        );
        let resp_headers = builder.headers_mut().unwrap();
        *resp_headers = headers;
    }

    let mut response_body = Body::empty();
    if let Some(body) = body {
        response_body = body;
    }
    builder.body(response_body).unwrap()
}

fn add_status_header(header: Option<HeaderMap>, status: StatusCode) -> HeaderMap {
    header
        .map(|mut hdrs| {
            hdrs.insert(
                HeaderName::from_str("Fn-Http-Status").unwrap(),
                status.as_u16().into(),
            );
            hdrs
        })
        .unwrap_or_else(|| {
            make_header_map_with_single_value(
                HeaderName::from_str("Fn-Http-Status").unwrap(),
                status.as_u16().into(),
            )
        })
}

pub fn success_or_recoverable_error(
    status: StatusCode,
    body: Option<Body>,
    headers: Option<HeaderMap>,
) -> Response<Body> {
    let response_headers = add_status_header(headers, status);
    generic_response(StatusCode::OK, body, response_headers)
}

pub fn unrecoverable_error(
    status: StatusCode,
    body: Option<Body>,
    headers: Option<HeaderMap>,
) -> Response<Body> {
    let response_headers = add_status_header(headers, status);
    generic_response(StatusCode::BAD_GATEWAY, body, response_headers)
}
