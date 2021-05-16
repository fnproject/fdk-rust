use hyper::HeaderMap;

/// start_logging enables logging for a user request.
pub fn start_logging(headers: &HeaderMap) {
    let framer = match std::env::var("FN_LOGFRAME_NAME") {
        Ok(v) => v,
        Err(_) => return,
    };

    let value_src = match std::env::var("FN_LOGFRAME_HDR") {
        Ok(v) => v,
        Err(_) => return,
    };

    if let Some(v) = headers.get(value_src) {
        if !v.is_empty() {
            println!("\n{}={}", framer, v.to_str().unwrap());
            eprintln!("\n{}={}", framer, v.to_str().unwrap());
        }
    }
}
