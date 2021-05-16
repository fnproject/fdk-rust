use crate::FunctionError;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub enum ContentType {
    JSON,
    YAML,
    XML,
    Plain,
    URLEncoded,
}

/// An `InputCoercible` type can be generated from a `Vec<u8>`.
pub trait InputCoercible: Sized {
    fn try_decode_plain(input: Vec<u8>) -> Result<Self, FunctionError>;
    fn try_decode_json(input: Vec<u8>) -> Result<Self, FunctionError>;
    fn try_decode_xml(input: Vec<u8>) -> Result<Self, FunctionError>;
    fn try_decode_yaml(input: Vec<u8>) -> Result<Self, FunctionError>;
    fn try_decode_urlencoded(input: Vec<u8>) -> Result<Self, FunctionError>;
}

/// An `OutputCoercible` type can be converted to a `Vec<u8>`.
pub trait OutputCoercible: Sized {
    fn try_encode_json(self) -> Result<Vec<u8>, FunctionError>;
    fn try_encode_xml(self) -> Result<Vec<u8>, FunctionError>;
    fn try_encode_yaml(self) -> Result<Vec<u8>, FunctionError>;
    fn try_encode_plain(self) -> Result<Vec<u8>, FunctionError>;
    fn try_encode_urlencoded(self) -> Result<Vec<u8>, FunctionError>;
}

impl<T: for<'de> Deserialize<'de>> InputCoercible for T {
    fn try_decode_plain(input: Vec<u8>) -> Result<Self, FunctionError> {
        match serde_plain::from_str(&input.iter().map(|&v| v as char).collect::<String>()) {
            Ok(t) => Ok(t),
            Err(e) => Err(FunctionError::coercion(e)),
        }
    }

    fn try_decode_json(input: Vec<u8>) -> Result<Self, FunctionError> {
        match serde_json::from_slice(input.as_slice()) {
            Ok(t) => Ok(t),
            Err(e) => Err(FunctionError::coercion(e)),
        }
    }

    fn try_decode_xml(input: Vec<u8>) -> Result<Self, FunctionError> {
        match serde_xml_rs::from_str(&input.iter().map(|&v| v as char).collect::<String>()) {
            Ok(t) => Ok(t),
            Err(e) => Err(FunctionError::coercion(e)),
        }
    }

    fn try_decode_yaml(input: Vec<u8>) -> Result<Self, FunctionError> {
        match serde_yaml::from_slice(input.as_slice()) {
            Ok(t) => Ok(t),
            Err(e) => Err(FunctionError::coercion(e)),
        }
    }

    fn try_decode_urlencoded(input: Vec<u8>) -> Result<Self, FunctionError> {
        match serde_urlencoded::from_str(&input.iter().map(|&v| v as char).collect::<String>()) {
            Ok(t) => Ok(t),
            Err(e) => Err(FunctionError::coercion(e)),
        }
    }
}

impl<T: Serialize> OutputCoercible for T {
    fn try_encode_json(self) -> Result<Vec<u8>, FunctionError> {
        match serde_json::to_vec(&self) {
            Ok(vector) => Ok(vector),
            Err(e) => Err(FunctionError::coercion(e)),
        }
    }
    fn try_encode_xml(self) -> Result<Vec<u8>, FunctionError> {
        match serde_xml_rs::to_string(&self) {
            Ok(vector) => Ok(vector.chars().map(|ch| ch as u8).collect()),
            Err(e) => Err(FunctionError::coercion(e)),
        }
    }
    fn try_encode_yaml(self) -> Result<Vec<u8>, FunctionError> {
        match serde_yaml::to_vec(&self) {
            Ok(vector) => Ok(vector),
            Err(e) => Err(FunctionError::coercion(e)),
        }
    }

    fn try_encode_plain(self) -> Result<Vec<u8>, FunctionError> {
        match serde_plain::to_string(&self) {
            Ok(vector) => Ok(vector.chars().map(|ch| ch as u8).collect()),
            Err(e) => Err(FunctionError::coercion(e)),
        }
    }

    fn try_encode_urlencoded(self) -> Result<Vec<u8>, FunctionError> {
        match serde_urlencoded::to_string(&self) {
            Ok(vector) => Ok(vector.chars().map(|ch| ch as u8).collect()),
            Err(e) => Err(FunctionError::coercion(e)),
        }
    }
}
