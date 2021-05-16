use crate::coercions::ContentType;
use crate::errors::FunctionError;
use hyper::{
    header::CONTENT_TYPE,
    header::{HeaderName, HeaderValue},
    HeaderMap, StatusCode,
};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::str::FromStr;

#[derive(Clone)]
/// A `RuntimeContext` contains configuration that is set up once per
/// function runtime and persists across invocations (in the case of a hot
/// function).
pub struct RuntimeContext {
    config: HashMap<String, String>,
    headers: HeaderMap,
    method: Option<hyper::Method>,
    content_type: ContentType,
    accept_type: ContentType,
    uri: Option<hyper::Uri>,
    call_id: String,
    response_headers: HeaderMap,
    response_status_code: Option<StatusCode>,
}

fn resolve_content_type(v: Option<&hyper::header::HeaderValue>) -> ContentType {
    match v {
        Some(value) => match value.to_str().unwrap() {
            "application/json" => ContentType::JSON,
            "text/yaml" | "application/yaml" => ContentType::YAML,
            "text/xml" | "application/xml" => ContentType::XML,
            "text/plain" => ContentType::Plain,
            "application/x-www-form-urlencoded" => ContentType::URLEncoded,
            _ => ContentType::JSON,
        },
        None => ContentType::JSON,
    }
}

fn get_accept_header_value(headers: &hyper::HeaderMap) -> Option<&HeaderValue> {
    if headers.get("Fn-Http-H-Accept").is_some() {
        headers.get("Fn-Http-H-Accept")
    } else if headers.get(hyper::header::ACCEPT).is_some() {
        headers.get(hyper::header::ACCEPT)
    } else {
        None
    }
}

impl RuntimeContext {
    /// from_req creates a RuntimeContext from a hyper Request reference.
    pub fn from_req<T>(req: &hyper::Request<T>) -> Self {
        let headers = {
            let fn_intent = req
                .headers()
                .get("Fn-Intent")
                .map(|value| value.to_str().unwrap())
                .unwrap_or_else(|| "");

            if fn_intent == "httprequest" {
                req.headers()
                    .iter()
                    .filter(|(k, _v)| *k == CONTENT_TYPE || k.as_str().starts_with("Fn-Http-H-"))
                    .map(|(k, v)| (k, v.to_owned()))
                    .fold(HeaderMap::new(), |mut m, (k, v)| {
                        m.insert(k, v);
                        m
                    })
            } else {
                req.headers().clone()
            }
        };

        Self {
            config: std::env::vars()
                .filter(|(_, v)| !v.as_bytes().is_empty())
                .fold(HashMap::new(), |mut m, k| {
                    m.insert(k.0, k.1);
                    m
                }),
            headers: headers.clone(),
            method: headers
                .get("Fn-Http-Method")
                .map(|value| hyper::Method::try_from(value.to_str().unwrap()).unwrap()),
            content_type: resolve_content_type(req.headers().get(CONTENT_TYPE)),
            accept_type: resolve_content_type(get_accept_header_value(req.headers())),
            uri: headers
                .get("Fn-Http-Request-Url")
                .map(|value| hyper::Uri::try_from(value.to_str().unwrap()).unwrap()),
            call_id: std::env::var("Fn-Call-Id").unwrap_or_else(|_| String::default()),
            response_headers: HeaderMap::new(),
            response_status_code: None,
        }
    }

    pub fn get_app_id(&self) -> String {
        return (self.config.get("FN_APP_ID").unwrap_or(&String::default())).to_string();
    }

    pub fn get_function_id(&self) -> String {
        return (self.config.get("FN_FN_ID").unwrap_or(&String::default())).to_string();
    }

    pub fn get_app_name(&self) -> String {
        return (self.config.get("FN_APP_NAME").unwrap_or(&String::default())).to_string();
    }

    pub fn get_function_name(&self) -> String {
        return (self.config.get("FN_FN_NAME").unwrap_or(&String::default())).to_string();
    }

    pub fn get_content_type(&self) -> ContentType {
        self.content_type.clone()
    }

    pub fn get_accept_type(&self) -> ContentType {
        self.accept_type.clone()
    }

    pub fn get_call_id(&self) -> String {
        self.call_id.clone()
    }

    pub fn get_headers(&self) -> HeaderMap {
        self.headers.clone()
    }

    pub fn get_header(&self, key: String) -> Option<String> {
        self.headers.get(key).map(|v| {
            v.as_bytes()
                .iter()
                .map(|&byte| byte as char)
                .collect::<String>()
        })
    }

    pub fn config(&self) -> &HashMap<String, String> {
        &self.config
    }

    pub fn add_header(&mut self, key: String, value: String) {
        self.response_headers.insert(
            HeaderName::from_str(key.as_str()).unwrap(),
            HeaderValue::from_str(value.as_str()).unwrap(),
        );
    }

    pub fn get_response_headers(&self) -> HeaderMap {
        self.response_headers.clone()
    }

    pub fn set_status_code(&mut self, status: u16) -> Result<(), FunctionError> {
        self.response_status_code = match StatusCode::from_u16(status) {
            Ok(v) => Some(v),
            Err(_) => {
                return Err(FunctionError::invalid_input(
                    "Invalid status code set by user",
                ))
            }
        };
        Ok(())
    }

    pub fn get_status_code(&self) -> Option<StatusCode> {
        self.response_status_code
    }
}
