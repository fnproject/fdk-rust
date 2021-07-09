use crate::FunctionError;
use serde::{Deserialize, Serialize};

/// ContentType represents the supported content types in the FDK.
#[derive(Clone, Debug)]
pub enum ContentType {
    JSON,
    YAML,
    XML,
    Plain,
    URLEncoded,
}

impl ContentType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "application/json" => ContentType::JSON,
            "text/yaml" | "application/yaml" => ContentType::YAML,
            "text/xml" | "application/xml" => ContentType::XML,
            "text/plain" => ContentType::Plain,
            "application/x-www-form-urlencoded" => ContentType::URLEncoded,
            _ => ContentType::JSON,
        }
    }

    pub fn as_header_value(&self) -> String {
        match self {
            Self::JSON => String::from("application/json"),
            Self::YAML => String::from("text/yaml"),
            Self::XML => String::from("application/xml"),
            Self::Plain => String::from("text/plain"),
            Self::URLEncoded => String::from("application/x-www-form-urlencoded"),
        }
    }
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
            Err(e) => Err(FunctionError::Coercion {
                inner: e.to_string(),
            }),
        }
    }

    fn try_decode_json(input: Vec<u8>) -> Result<Self, FunctionError> {
        match serde_json::from_slice(input.as_slice()) {
            Ok(t) => Ok(t),
            Err(e) => Err(FunctionError::Coercion {
                inner: e.to_string(),
            }),
        }
    }

    fn try_decode_xml(input: Vec<u8>) -> Result<Self, FunctionError> {
        match serde_xml_rs::from_str(&input.iter().map(|&v| v as char).collect::<String>()) {
            Ok(t) => Ok(t),
            Err(e) => Err(FunctionError::Coercion {
                inner: e.to_string(),
            }),
        }
    }

    fn try_decode_yaml(input: Vec<u8>) -> Result<Self, FunctionError> {
        match serde_yaml::from_slice(input.as_slice()) {
            Ok(t) => Ok(t),
            Err(e) => Err(FunctionError::Coercion {
                inner: e.to_string(),
            }),
        }
    }

    fn try_decode_urlencoded(input: Vec<u8>) -> Result<Self, FunctionError> {
        match serde_urlencoded::from_str(&input.iter().map(|&v| v as char).collect::<String>()) {
            Ok(t) => Ok(t),
            Err(e) => Err(FunctionError::Coercion {
                inner: e.to_string(),
            }),
        }
    }
}

impl<T: Serialize> OutputCoercible for T {
    fn try_encode_json(self) -> Result<Vec<u8>, FunctionError> {
        match serde_json::to_vec(&self) {
            Ok(vector) => Ok(vector),
            Err(e) => Err(FunctionError::Coercion {
                inner: e.to_string(),
            }),
        }
    }
    fn try_encode_xml(self) -> Result<Vec<u8>, FunctionError> {
        match serde_xml_rs::to_string(&self) {
            Ok(vector) => Ok(vector.chars().map(|ch| ch as u8).collect()),
            Err(e) => Err(FunctionError::Coercion {
                inner: e.to_string(),
            }),
        }
    }
    fn try_encode_yaml(self) -> Result<Vec<u8>, FunctionError> {
        match serde_yaml::to_vec(&self) {
            Ok(vector) => Ok(vector),
            Err(e) => Err(FunctionError::Coercion {
                inner: e.to_string(),
            }),
        }
    }

    fn try_encode_plain(self) -> Result<Vec<u8>, FunctionError> {
        match serde_plain::to_string(&self) {
            Ok(vector) => Ok(vector.chars().map(|ch| ch as u8).collect()),
            Err(e) => Err(FunctionError::Coercion {
                inner: e.to_string(),
            }),
        }
    }

    fn try_encode_urlencoded(self) -> Result<Vec<u8>, FunctionError> {
        match serde_urlencoded::to_string(&self) {
            Ok(vector) => Ok(vector.chars().map(|ch| ch as u8).collect()),
            Err(e) => Err(FunctionError::Coercion {
                inner: e.to_string(),
            }),
        }
    }
}
