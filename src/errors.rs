use hyper::{Body, Response};

use crate::utils::{
    make_header_map_with_single_value, success_or_recoverable_error, unrecoverable_error,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FunctionError {
    #[error("Invalid input: {inner:?}")]
    InvalidInput { inner: String },

    #[error("Bad request")]
    BadRequest,

    #[error("Initialization failed: {inner:?}")]
    Initialization { inner: String },

    #[error("Coercion failed: {inner:?}")]
    Coercion { inner: String },

    #[error("IO Error: {inner:?}")]
    IO { inner: String },

    #[error("Server error: {inner:?}")]
    Server { inner: String },

    #[error("Internal system error: {inner:?}")]
    System { inner: String },

    #[error("User error: {inner:?}")]
    User { inner: String },
}

impl FunctionError {
    pub fn is_user_error(&self) -> bool {
        matches!(
            self,
            Self::InvalidInput { .. }
                | Self::BadRequest
                | Self::Coercion { .. }
                | Self::User { .. }
        )
    }

    pub fn new_user_error(error: String) -> Self {
        Self::User { inner: error }
    }
}

impl From<FunctionError> for hyper::Response<Body> {
    fn from(e: FunctionError) -> hyper::Response<Body> {
        if e.is_user_error() {
            client_error(format!("{}", e))
        } else {
            server_error(format!("{}", e))
        }
    }
}

impl From<std::io::Error> for FunctionError {
    fn from(e: std::io::Error) -> Self {
        Self::IO {
            inner: e.to_string(),
        }
    }
}

impl From<std::env::VarError> for FunctionError {
    fn from(e: std::env::VarError) -> Self {
        Self::Initialization {
            inner: e.to_string(),
        }
    }
}

impl From<url::ParseError> for FunctionError {
    fn from(e: url::ParseError) -> Self {
        Self::Initialization {
            inner: format!("Could not parse the URL: {}", e.to_string()),
        }
    }
}

impl From<hyper::Error> for FunctionError {
    fn from(e: hyper::Error) -> Self {
        Self::Server {
            inner: e.to_string(),
        }
    }
}

/// A utility function that produces a client error response from a type that
/// can be converted to a vector of bytes.
pub fn client_error<T>(data: T) -> Response<Body>
where
    T: Into<Vec<u8>>,
{
    let bytes: Vec<u8> = data.into();
    let content_length = bytes.len();
    success_or_recoverable_error(
        hyper::StatusCode::BAD_GATEWAY,
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
    let bytes: Vec<u8> = data.into();
    let content_length = bytes.len();
    unrecoverable_error(
        hyper::StatusCode::INTERNAL_SERVER_ERROR,
        Option::from(Body::from(bytes)),
        Option::from(make_header_map_with_single_value(
            hyper::header::CONTENT_LENGTH,
            content_length.into(),
        )),
    )
}
