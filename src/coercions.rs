use hyper;
use serde_json;

use errors::FunctionError;
use hyper_utils::{no_content, success, body_as_bytes};


/// An `InputCoercible` type can be generated from a `hyper::Request`.
pub trait InputCoercible: Sized {
    /// Consume the request and try to convert it into an instance of the type.
    /// If this fails, report an appropriate FunctionError.
    fn try_decode(req: hyper::Request) -> Result<Self, FunctionError>;
}

/// An `OutputCoercible` type can be converted to a `hyper::Response`.
pub trait OutputCoercible {
    /// Consume an instance of the type and try to convert it into a response.
    /// If this fails, report an appropriate FunctionError.
    fn try_encode(self) -> Result<hyper::Response, FunctionError>;
}

/// Request is coercible to itself.
impl InputCoercible for hyper::Request {
    fn try_decode(req: hyper::Request) -> Result<hyper::Request, FunctionError> {
        Ok(req)
    }
}

/// Response is coercible to itself.
impl OutputCoercible for hyper::Response {
    fn try_encode(self) -> Result<hyper::Response, FunctionError> {
        Ok(self)
    }
}

/// The empty type is InputCoercible, for simplicity
impl InputCoercible for () {
    fn try_decode(_: hyper::Request) -> Result<(), FunctionError> {
        Ok(())
    }
}

/// The empty type is OutputCoercible, for simplicity
impl OutputCoercible for () {
    fn try_encode(self) -> Result<hyper::Response, FunctionError> {
        Ok(no_content())
    }
}

/// A vector of bytes is InputCoercible, for simplicity
impl InputCoercible for Vec<u8> {
    fn try_decode(req: hyper::Request) -> Result<Vec<u8>, FunctionError> {
        body_as_bytes(req.body())
    }
}

/// A vector of bytes is OutputCoercible, for simplicity
impl OutputCoercible for Vec<u8> {
    fn try_encode(self) -> Result<hyper::Response, FunctionError> {
        Ok(success(self))
    }
}

/// String is InputCoercible, for simplicity
impl InputCoercible for String {
    fn try_decode(req: hyper::Request) -> Result<String, FunctionError> {
        match body_as_bytes(req.body()) {
            Ok(v) => {
                match String::from_utf8(v) {
                    Ok(s) => Ok(s),
                    Err(e) => Err(FunctionError::invalid_input(e)),
                }
            }
            Err(e) => Err(e),
        }
    }
}

/// String is OutputCoercible, for simplicity
impl OutputCoercible for String {
    fn try_encode(self) -> Result<hyper::Response, FunctionError> {
        Ok(success(self))
    }
}

/// serde_json::Value is InputCoercible, for simplicity
impl InputCoercible for serde_json::Value {
    fn try_decode(req: hyper::Request) -> Result<serde_json::Value, FunctionError> {
        match body_as_bytes(req.body()) {
            Ok(v) => {
                match serde_json::from_slice(&v) {
                    Ok(obj) => Ok(obj),
                    Err(e) => Err(FunctionError::invalid_input(e)),
                }
            }
            Err(e) => Err(e),
        }
    }
}

/// serde_json::Value is OutputCoercible, for simplicity
impl OutputCoercible for serde_json::Value {
    fn try_encode(self) -> Result<hyper::Response, FunctionError> {
        match serde_json::to_vec(&self) {
            Ok(v) => Ok(success(v)),
            Err(e) => Err(FunctionError::io(e)),
        }
    }
}
