use std::collections::HashMap;

/// A `RuntimeContext` contains configuration that is set up once per
/// function runtime and persists across invocations (in the case of a hot
/// function). This configuration can therefore be used to initialize the state
/// of the user's function.
pub struct RuntimeContext {
    config: HashMap<String, String>,
}

impl RuntimeContext {
    #[doc(hidden)]
    pub fn with_environment(environment: &HashMap<String, String>) -> RuntimeContext {
        const HEADER_PREFIX: &'static str = "fn_header_";
        RuntimeContext {
            config: environment
                .iter()
                .filter(|kv| !kv.0.to_lowercase().starts_with(HEADER_PREFIX))
                .fold(HashMap::new(), |mut cfg, kv| {
                    cfg.insert(kv.0.clone(), kv.1.clone());
                    cfg
                }),
        }
    }

    /// Access the map of configuration variables.
    pub fn config(&self) -> &HashMap<String, String> {
        &self.config
    }
}
