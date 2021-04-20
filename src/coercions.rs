use crate::FunctionError;
use serde::{de::DeserializeOwned, Serialize};

/// An `InputCoercible` type can be generated from a `Vec<u8>`.
pub trait InputCoercible: Sized {
    fn try_decode(input: Vec<u8>) -> Result<Self, FunctionError>;
}

/// An `OutputCoercible` type can be converted to a `Vec<u8>`.
pub trait OutputCoercible: Sized {
    fn try_encode(self) -> Result<Vec<u8>, FunctionError>;
}

impl<T: DeserializeOwned> InputCoercible for T {
    fn try_decode(input: Vec<u8>) -> Result<Self, FunctionError> {
        match serde_json::from_slice(input.as_slice()) {
            Ok(t) => Ok(t),
            Err(e) => Err(FunctionError::coercion(e)),
        }
    }
}

impl<T: Serialize> OutputCoercible for T {
    fn try_encode(self) -> Result<Vec<u8>, FunctionError> {
        match serde_json::to_vec(&self) {
            Ok(vector) => Ok(vector),
            Err(e) => Err(FunctionError::coercion(e)),
        }
    }
}
