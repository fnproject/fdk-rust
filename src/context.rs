use crate::coercions::ContentType;
use crate::errors::FunctionError;
use hyper::{
    header::CONTENT_TYPE,
    header::{HeaderName, HeaderValue},
    HeaderMap, StatusCode,
};
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::str::FromStr;
use std::sync::Arc;

lazy_static! {
    pub static ref CONFIG_FROM_ENV: Arc<HashMap<String, String>> = Arc::from(
        std::env::vars()
            .filter(|(_, v)| !v.as_bytes().is_empty())
            .fold(HashMap::new(), |mut m, k| {
                m.insert(k.0, k.1);
                m
            })
    );
}

#[derive(Clone)]
/// `RuntimeContext` contains the config and metadata of request and response. A mutable reference
/// to RuntimeContext gets passed into the user function for accessing request metadata and adding
/// response headers.
pub struct RuntimeContext {
    config: Arc<HashMap<String, String>>,
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
        Some(value) => ContentType::from_str(value.to_str().unwrap_or("")),
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
            config: CONFIG_FROM_ENV.clone(),
            headers: headers.clone(),
            method: headers
                .get("Fn-Http-Method")
                .map(|value| hyper::Method::try_from(value.to_str().unwrap()).unwrap()),
            content_type: resolve_content_type(req.headers().get(CONTENT_TYPE)),
            accept_type: resolve_content_type(get_accept_header_value(req.headers())),
            uri: headers
                .get("Fn-Http-Request-Url")
                .map(|value| hyper::Uri::try_from(value.to_str().unwrap()).unwrap()),
            call_id: headers
                .get("Fn-Call-Id")
                .map(|v| v.to_str().unwrap_or_default())
                .unwrap_or_default()
                .to_owned(),
            response_headers: HeaderMap::new(),
            response_status_code: None,
        }
    }

    /// Returns the app ID
    pub fn app_id(&self) -> String {
        return (self.config.get("FN_APP_ID").unwrap_or(&String::default())).to_string();
    }

    /// Returns the function ID
    pub fn function_id(&self) -> String {
        return (self.config.get("FN_FN_ID").unwrap_or(&String::default())).to_string();
    }

    /// Returns the app name
    pub fn app_name(&self) -> String {
        return (self.config.get("FN_APP_NAME").unwrap_or(&String::default())).to_string();
    }

    /// Returns the function name
    pub fn function_name(&self) -> String {
        return (self.config.get("FN_FN_NAME").unwrap_or(&String::default())).to_string();
    }

    /// Returns the `Content-Type` header from request. This header is used to choose a deserializer for request body.
    pub fn content_type(&self) -> ContentType {
        self.content_type.clone()
    }

    /// Returns the `Accept` header from request. This header is used to choose a serializer for response body.
    pub fn accept_type(&self) -> ContentType {
        self.accept_type.clone()
    }

    /// Returns the call ID
    pub fn call_id(&self) -> String {
        self.call_id.clone()
    }

    /// Returns request headers
    pub fn headers(&self) -> HeaderMap {
        self.headers.clone()
    }

    /// Returns an `Option<String>` based on the value of header present in headers.
    /// `header` returns None if the header with key is not found.
    pub fn header(&self, key: String) -> Option<String> {
        self.headers.get(key).map(|v| {
            v.as_bytes()
                .iter()
                .map(|&byte| byte as char)
                .collect::<String>()
        })
    }

    /// Returns the config injected at the runtime from the environment variables.
    pub fn config(&self) -> &HashMap<String, String> {
        &self.config
    }

    /// Adds a custom header to the response.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// ctx.add_response_header("X-COOLNESS-METER-SAYS", "OVER-9000")
    /// ```
    pub fn add_response_header(&mut self, key: String, value: String) {
        self.response_headers.insert(
            HeaderName::from_str(key.as_str()).unwrap(),
            HeaderValue::from_str(value.as_str()).unwrap(),
        );
    }

    /// Helper to return the response headers
    pub fn response_headers(&self) -> HeaderMap {
        self.response_headers.clone()
    }

    /// Sets the status code in the response headers under Fn-Http-Status key.
    /// Default value is 200.
    pub fn set_status_code(&mut self, status: u16) -> Result<(), FunctionError> {
        self.response_status_code = match StatusCode::from_u16(status) {
            Ok(v) => Some(v),
            Err(_) => {
                return Err(FunctionError::InvalidInput {
                    inner: "Invalid http code added".into(),
                })
            }
        };
        Ok(())
    }

    /// Helper function to return status code set by user.
    pub fn get_status_code(&self) -> Option<StatusCode> {
        self.response_status_code
    }
}
