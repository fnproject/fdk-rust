use hyper;

use hyper::{Body, Request, Response};
use lazy_static::lazy_static;
use object_pool::Pool;

use crate::coercions::{InputCoercible, OutputCoercible};
use crate::context::RuntimeContext;
use crate::errors::FunctionError;
use crate::socket::UDS;
use std::io::Write;

lazy_static! {
    static ref POOL: Pool<Vec<u8>> = Pool::new(1024, || Vec::with_capacity(4096));
}

pub struct Function;

impl Function {
    pub async fn run<T, S, F>(function: F) -> Result<(), FunctionError>
    where
        T: InputCoercible + 'static,
        S: OutputCoercible + 'static,
        F: Fn(RuntimeContext, T) -> Result<S, FunctionError> + Send + Sync + 'static,
    {
        Self::run_inner(std::sync::Arc::new(function)).await
    }

    async fn run_inner<T, S, F>(function: std::sync::Arc<F>) -> Result<(), FunctionError>
    where
        T: InputCoercible + 'static,
        S: OutputCoercible + 'static,
        F: Fn(RuntimeContext, T) -> Result<S, FunctionError> + Send + Sync + 'static,
    {
        let socket = match UDS::new() {
            Ok(s) => s,
            Err(e) => return Err(e),
        };

        let svc = hyper::service::make_service_fn(|_| {
            let function = function.clone();
            async move {
                Ok::<_, FunctionError>(hyper::service::service_fn(move |req: Request<Body>| {
                    let function = function.clone();
                    async move {
                        // Taking the longest possible route here. This needs to be improved. Some tips are:
                        // - Use async closure when they are on stable toolchain
                        // - See if we can do a better job at determining what return type this closure returns. Currently returning impl Trait inside a function is not permitted;
                        let mut buffer = match POOL.try_pull() {
                            Some(buf) => buf,
                            None => {
                                return Err(FunctionError::other("Failed to allocate memory"));
                            }
                        };
                        let ctx = RuntimeContext::from_req(&req);
                        let _ = buffer.write(
                            match hyper::body::to_bytes(req.into_body()).await {
                                Ok(data) => data.to_vec(),
                                Err(e) => {
                                    return Err(FunctionError::io(format!(
                                        "Failed to read request body: {}",
                                        e
                                    )));
                                }
                            }
                            .as_ref(),
                        );

                        let arg = match T::try_decode(buffer.to_vec()) {
                          Ok(v) => v,
                          Err(e) => return Err(e),
                        };

                        buffer.clear();
                        let output = match function(ctx, arg) {
                            Ok(out) => out,
                            Err(e) => {
                                return Err(FunctionError::other(format!(
                                    "Error executing user function: {}",
                                    e
                                )));
                            }
                        };

                        let response_body = match S::try_encode(output) {
                          Ok(body) => body,
                          Err(e) => return Err(e),
                        };

                        Ok::<_, FunctionError>(Response::new(Body::from(response_body)))
                    }
                }))
            }
        });

        match hyper::server::Server::builder(socket).serve(svc).await {
            Ok(_) => Ok(()),
            Err(e) => Err(FunctionError::initialization(format!(
                "Error while starting server: {}",
                e
            ))),
        }
    }
}
