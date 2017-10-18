use hyper;

use std::error::Error;
use std::fmt;

use hyper_utils::{client_error, server_error};

/// An `Error` which can occur during the execution of a function. Depending on
/// which kind of error it is, it could signify that the function runtime is
/// compromised or it could just represent an issue with the way the caller
/// provided input data.
#[derive(Debug)]
pub struct FunctionError {
    kind: FunctionErrorKind,
    error: Box<Error + Send + Sync>,
}

impl FunctionError {
    fn new<E>(kind: FunctionErrorKind, error: E) -> FunctionError
    where
        E: Into<Box<Error + Send + Sync>>,
    {
        FunctionError {
            kind: kind,
            error: error.into(),
        }
    }

    /// Create a new error signifying that the input provided to the function
    /// was genuinely invalid.
    pub fn invalid_input<E>(error: E) -> FunctionError
    where
        E: Into<Box<Error + Send + Sync>>,
    {
        FunctionError::new(FunctionErrorKind::InvalidInput, error)
    }

    /// Create a new error signifying that the request provided to the function
    /// was genuinely bad (for example, headers or data were missing).
    pub fn bad_request<E>(error: E) -> FunctionError
    where
        E: Into<Box<Error + Send + Sync>>,
    {
        FunctionError::new(FunctionErrorKind::BadRequest, error)
    }

    /// Create a new error signifying that the initializer code for the function
    /// has failed; this error compromises the function runtime.
    pub fn initialization<E>(error: E) -> FunctionError
    where
        E: Into<Box<Error + Send + Sync>>,
    {
        FunctionError::new(FunctionErrorKind::InitializationError, error)
    }

    /// Create a new error signifying that the input/output coercion code has
    /// encountered an unrecoverable problem; this error compromises the
    /// function runtime.
    pub fn coercion<E>(error: E) -> FunctionError
    where
        E: Into<Box<Error + Send + Sync>>,
    {
        FunctionError::new(FunctionErrorKind::CoercionError, error)
    }

    /// Create a new error signifying that an i/o error has occurred while
    /// reading or writing the i/o streams; this error compromises the function
    /// runtime.
    pub fn io<E>(error: E) -> FunctionError
    where
        E: Into<Box<Error + Send + Sync>>,
    {
        FunctionError::new(FunctionErrorKind::IOError, error)
    }

    /// Create a new error representing a totally unexpected situation; this
    /// error compromises the function runtime.
    pub fn other<E>(error: E) -> FunctionError
    where
        E: Into<Box<Error + Send + Sync>>,
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

impl Error for FunctionError {
    fn description(&self) -> &str {
        self.error.description()
    }

    fn cause(&self) -> Option<&Error> {
        self.error.cause()
    }
}

impl Into<hyper::Response> for FunctionError {
    fn into(self) -> hyper::Response {
        if self.is_user_error() {
            client_error(format!("{}", self.error).into_bytes())
        } else {
            server_error(format!("{}", self.error).into_bytes())
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
        match *self {
            FunctionErrorKind::InvalidInput |
            FunctionErrorKind::BadRequest => true,
            _ => false,
        }
    }
}
