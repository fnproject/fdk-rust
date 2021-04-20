use hyper::HeaderMap;
use std::collections::HashMap;
use std::convert::TryFrom;

#[derive(Clone)]
/// A `RuntimeContext` contains configuration that is set up once per
/// function runtime and persists across invocations (in the case of a hot
/// function). This configuration can therefore be used to initialize the state
/// of the user's function.
pub struct RuntimeContext {
    config: HashMap<String, String>,
    headers: HeaderMap,
    method: hyper::Method,
    uri: hyper::Uri,
    call_id: String,
}

impl RuntimeContext {
    pub fn new() -> Self {
        Self {
            config: HashMap::new(),
            headers: HeaderMap::new(),
            method: hyper::Method::GET,
            uri: hyper::Uri::default(),
            call_id: "lol".into(),
        }
    }
    pub fn from_req<T>(req: &hyper::Request<T>) -> Self {
        Self {
            config: std::env::vars().fold(HashMap::new(), |mut m, k| {
                m.insert(k.0.clone(), k.1.clone());
                m
            }),
            headers: req.headers().clone(),
            method: hyper::Method::from(req.method()),
            uri: hyper::Uri::try_from(req.uri()).unwrap(),
            call_id: std::env::var("Fn-Call-Id").unwrap_or(String::from("")),
        }
    }

    pub fn get_app_id(&self) -> String {
        return (self.config.get("FN_APP_ID").unwrap_or(&String::from(""))).to_string();
    }

    pub fn get_function_id(&self) -> String {
        return (self.config.get("FN_FN_ID").unwrap_or(&String::from(""))).to_string();
    }

    pub fn get_app_name(&self) -> String {
        return (self.config.get("FN_APP_NAME").unwrap_or(&String::from(""))).to_string();
    }

    pub fn get_function_name(&self) -> String {
        return (self.config.get("FN_FN_NAME").unwrap_or(&String::from(""))).to_string();
    }

    pub fn get_call_id(&self) -> String {
        self.call_id.clone()
    }

    pub fn get_headers(&self) -> HeaderMap {
        self.headers.clone()
    }
    /// Access the map of configuration variables.
    pub fn config(&self) -> &HashMap<String, String> {
        &self.config
    }
}
