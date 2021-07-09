use hyper::{Body, Request};
use lazy_static::lazy_static;
use object_pool::Pool;
use std::io::Write;

use crate::coercions::{ContentType, InputCoercible, OutputCoercible};
use crate::context::RuntimeContext;
use crate::errors::FunctionError;
use crate::socket::UDS;
use crate::utils::success_or_recoverable_error;

pub type Result<OutputCoercible> = core::result::Result<OutputCoercible, FunctionError>;

lazy_static! {
    static ref POOL: Pool<Vec<u8>> = Pool::new(1024, || Vec::with_capacity(4096));
}

/// Function is the first class primitive provided by FDK to run functions on Oracle Cloud Functions and FnProject.
pub struct Function;

impl Function {
    /// `run` accepts a function from the user. `run` is an async function and returns a future which should be awaited to accept
    /// user requests and execute passed function on the given input.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let function = Function::new(|_: &mut fdk::RuntimeContext, i: i32| -> Result<i32, fdk::FunctionError> {
    ///   Ok(i*i)
    /// });
    /// if let Err(e) = function.await {
    ///   eprintln!("{}", e);
    /// }
    /// ```
    pub async fn run<T, S, F>(function: F) -> Result<()>
    where
        T: InputCoercible + 'static,
        S: OutputCoercible + 'static,
        F: Fn(&mut RuntimeContext, T) -> Result<S> + Send + Sync + 'static,
    {
        Self::run_inner(std::sync::Arc::new(function)).await
    }

    async fn run_inner<T, S, F>(function: std::sync::Arc<F>) -> Result<()>
    where
        T: InputCoercible + 'static,
        S: OutputCoercible + 'static,
        F: Fn(&mut RuntimeContext, T) -> Result<S> + Send + Sync + 'static,
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
                        crate::logging::start_logging(req.headers());

                        let mut ctx = RuntimeContext::from_req(&req);

                        // We don't need buffer to live outside of the block we decode the request body
                        let arg = {
                            let mut buffer = match POOL.try_pull() {
                                Some(buf) => buf,
                                None => {
                                    return Ok(FunctionError::System {
                                        inner: "Failed to allocate memory".into(),
                                    }
                                    .into());
                                }
                            };
                            let _ = buffer.write(
                                match hyper::body::to_bytes(req.into_body()).await {
                                    Ok(data) => data.to_vec(),
                                    Err(e) => {
                                        return Ok(FunctionError::IO {
                                            inner: format!("Failed to read request body: {}", e),
                                        }
                                        .into());
                                    }
                                }
                                .as_ref(),
                            );

                            let decoded_arg_result = decode_body(ctx.content_type(), &buffer);

                            buffer.clear();

                            let decoded_arg = match decoded_arg_result {
                                Ok(v) => v,
                                Err(e) => {
                                    return Ok(FunctionError::Coercion {
                                        inner: format!(
                                            "Error while deserializing request body: {}",
                                            e
                                        ),
                                    }
                                    .into())
                                }
                            };

                            decoded_arg
                        };

                        let output_format = ctx.accept_type();

                        let output = match function(&mut ctx, arg) {
                            Ok(out) => out,
                            Err(e) => match e {
                                FunctionError::User { .. } => return Ok(e.into()),
                                _ => {
                                    return Ok(FunctionError::InvalidInput {
                                        inner: format!("Error executing user function: {}", e),
                                    }
                                    .into())
                                }
                            },
                        };

                        let response_body = match encode_body(&output_format, output) {
                            Ok(body) => body,
                            Err(e) => {
                                return Ok(FunctionError::Coercion {
                                    inner: format!("Error while serializing response body: {}", e),
                                }
                                .into())
                            }
                        };

                        let response_content_type = output_format.as_header_value();

                        ctx.add_response_header(
                            hyper::header::CONTENT_TYPE.as_str().to_owned(),
                            response_content_type,
                        );

                        Ok::<_, FunctionError>(success_or_recoverable_error(
                            ctx.get_status_code().unwrap_or(hyper::StatusCode::OK),
                            Option::from(Body::from(response_body)),
                            Option::from(ctx.response_headers()),
                        ))
                    }
                }))
            }
        });

        let _ = hyper::server::Server::builder(socket).serve(svc).await?;

        Ok(())
    }
}

fn encode_body<S: OutputCoercible>(content_type: &ContentType, s: S) -> Result<Vec<u8>> {
    match content_type {
        ContentType::JSON => S::try_encode_json(s),
        ContentType::YAML => S::try_encode_yaml(s),
        ContentType::XML => S::try_encode_xml(s),
        ContentType::Plain => S::try_encode_plain(s),
        ContentType::URLEncoded => S::try_encode_urlencoded(s),
    }
}

fn decode_body<T: InputCoercible>(
    content_type: ContentType,
    buffer: &object_pool::Reusable<Vec<u8>>,
) -> Result<T> {
    match content_type {
        ContentType::JSON => T::try_decode_json(buffer.to_vec()),
        ContentType::YAML => T::try_decode_yaml(buffer.to_vec()),
        ContentType::XML => T::try_decode_xml(buffer.to_vec()),
        ContentType::Plain => T::try_decode_plain(buffer.to_vec()),
        ContentType::URLEncoded => T::try_decode_urlencoded(buffer.to_vec()),
    }
}
