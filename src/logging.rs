use crate::context;
use hyper::HeaderMap;

/// start_logging enables logging for a user request.
pub fn start_logging(headers: &HeaderMap) {
    let config = context::CONFIG_FROM_ENV.clone();

    let framer = match config.get("FN_LOGFRAME_NAME") {
        Some(v) => v,
        None => return,
    };

    let value_src = match config.get("FN_LOGFRAME_HDR") {
        Some(v) => v,
        None => return,
    };

    if let Some(v) = headers.get(value_src) {
        if !v.is_empty() {
            println!("\n{}={}", framer, v.to_str().unwrap());
            eprintln!("\n{}={}", framer, v.to_str().unwrap());
        }
    }
}
