use hyper::{Body, Response};

use crate::utils::{
    make_header_map_with_single_value, success_or_recoverable_error, unrecoverable_error,
};
use std::env::VarError;
use std::error::Error;
use std::fmt;

/// An `Error` which can occur during the execution of a function. Depending on
/// which kind of error it is, it could signify that the function runtime is
/// compromised or it could just represent an issue with the way the caller
/// provided input data.
#[derive(Debug)]
pub struct FunctionError {
    kind: FunctionErrorKind,
    error: Box<dyn Error + Send + Sync>,
}

impl FunctionError {
    fn new<E>(kind: FunctionErrorKind, error: E) -> FunctionError
    where
        E: Into<Box<dyn Error + Send + Sync>>,
    {
        FunctionError {
            kind,
            error: error.into(),
        }
    }

    /// Create a new error signifying that the input provided to the function
    /// was genuinely invalid.
    pub fn invalid_input<E>(error: E) -> FunctionError
    where
        E: Into<Box<dyn Error + Send + Sync>>,
    {
        FunctionError::new(FunctionErrorKind::InvalidInput, error)
    }

    /// Create a new error signifying that the request provided to the function
    /// was genuinely bad (for example, headers or data were missing).
    pub fn bad_request<E>(error: E) -> FunctionError
    where
        E: Into<Box<dyn Error + Send + Sync>>,
    {
        FunctionError::new(FunctionErrorKind::BadRequest, error)
    }

    /// Create a new error signifying that the initializer code for the function
    /// has failed; this error compromises the function runtime.
    pub fn initialization<E>(error: E) -> FunctionError
    where
        E: Into<Box<dyn Error + Send + Sync>>,
    {
        FunctionError::new(FunctionErrorKind::InitializationError, error)
    }

    /// Create a new error signifying that the input/output coercion code has
    /// encountered an unrecoverable problem; this error compromises the
    /// function runtime.
    pub fn coercion<E>(error: E) -> FunctionError
    where
        E: Into<Box<dyn Error + Send + Sync>>,
    {
        FunctionError::new(FunctionErrorKind::CoercionError, error)
    }

    /// Create a new error signifying that an i/o error has occurred while
    /// reading or writing the i/o streams; this error compromises the function
    /// runtime.
    pub fn io<E>(error: E) -> FunctionError
    where
        E: Into<Box<dyn Error + Send + Sync>>,
    {
        FunctionError::new(FunctionErrorKind::IOError, error)
    }

    /// Create a new error representing a totally unexpected situation; this
    /// error compromises the function runtime.
    pub fn other<E>(error: E) -> FunctionError
    where
        E: Into<Box<dyn Error + Send + Sync>>,
    {
        FunctionError::new(FunctionErrorKind::OtherError, error)
    }

    /// Returns true if the error can be reported to the user as a client error
    /// (a 400-series http error).
    pub fn is_user_error(&self) -> bool {
        self.kind.is_user_error()
    }
}

impl fmt::Display for FunctionError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        self.error.fmt(fmt)
    }
}

impl Error for FunctionError {}

impl From<VarError> for FunctionError {
    fn from(e: VarError) -> Self {
        FunctionError::initialization(e)
    }
}

impl From<hyper::Error> for FunctionError {
    fn from(e: hyper::Error) -> Self {
        FunctionError::other(format!("hyper error: {}", e))
    }
}

impl From<std::io::Error> for FunctionError {
    fn from(e: std::io::Error) -> Self {
        FunctionError::io(e)
    }
}

impl From<FunctionError> for hyper::Response<Body> {
    fn from(e: FunctionError) -> hyper::Response<Body> {
        if e.is_user_error() {
            client_error(format!("{}", e.error))
        } else {
            server_error(format!("{}", e.error))
        }
    }
}

fn _assert_error_is_sync_send() {
    fn _is_sync_send<T: Sync + Send>() {}
    _is_sync_send::<FunctionError>();
}

/// A kind for function errors. Some of these errors can be reported to the
/// caller without compromising the function runtime, while others represent
/// situations in which the function runtime is compromised and must be shut
/// down.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum FunctionErrorKind {
    // User errors which can be reported without compromising the function
    /// Genuinely invalid input from the caller
    InvalidInput,
    /// Generic bad request
    BadRequest,

    // Internal errors which compromise the integrity of the function
    /// Unrecoverable error during initialization of the function
    InitializationError,
    /// Unrecoverable error during input/output coercion
    CoercionError,
    /// Unrecoverable error on input/output streams
    IOError,
    /// Unrecoverable generic error
    OtherError,
}

impl FunctionErrorKind {
    /// True if the error is a user error and can be reported as such.
    pub fn is_user_error(&self) -> bool {
        matches!(
            *self,
            FunctionErrorKind::InvalidInput
                | FunctionErrorKind::BadRequest
                | FunctionErrorKind::CoercionError
        )
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
