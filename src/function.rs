use hyper::{Body, Request};
use lazy_static::lazy_static;
use object_pool::Pool;

use crate::coercions::{ContentType, InputCoercible, OutputCoercible};
use crate::context::RuntimeContext;
use crate::errors::FunctionError;
use crate::socket::UDS;
use crate::utils::success_or_recoverable_error;
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
        F: Fn(&mut RuntimeContext, T) -> Result<S, FunctionError> + Send + Sync + 'static,
    {
        Self::run_inner(std::sync::Arc::new(function)).await
    }

    async fn run_inner<T, S, F>(function: std::sync::Arc<F>) -> Result<(), FunctionError>
    where
        T: InputCoercible + 'static,
        S: OutputCoercible + 'static,
        F: Fn(&mut RuntimeContext, T) -> Result<S, FunctionError> + Send + Sync + 'static,
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
                                    return Ok(
                                        FunctionError::other("Failed to allocate memory").into()
                                    );
                                }
                            };
                            let _ = buffer.write(
                                match hyper::body::to_bytes(req.into_body()).await {
                                    Ok(data) => data.to_vec(),
                                    Err(e) => {
                                        return Ok(FunctionError::io(format!(
                                            "Failed to read request body: {}",
                                            e
                                        ))
                                        .into());
                                    }
                                }
                                .as_ref(),
                            );

                            let decoded_arg_result = decode_body(ctx.get_content_type(), &buffer);

                            buffer.clear();

                            let decoded_arg = match decoded_arg_result {
                                Ok(v) => v,
                                Err(e) => {
                                    return Ok(FunctionError::coercion(format!(
                                        "Error while deserializing request body: {}",
                                        e
                                    ))
                                    .into())
                                }
                            };

                            decoded_arg
                        };

                        let output_format = ctx.get_accept_type();

                        let output = match function(&mut ctx, arg) {
                            Ok(out) => out,
                            Err(e) => {
                                return Ok(FunctionError::invalid_input(format!(
                                    "Error executing user function: {}",
                                    e
                                ))
                                .into());
                            }
                        };

                        let response_body = match encode_body(&output_format, output) {
                            Ok(body) => body,
                            Err(e) => {
                                return Ok(FunctionError::coercion(format!(
                                    "Error while serializing response body: {}",
                                    e
                                ))
                                .into())
                            }
                        };

                        let response_content_type = get_response_header(&output_format);

                        ctx.add_header(
                            hyper::header::CONTENT_TYPE.as_str().to_owned(),
                            response_content_type,
                        );

                        Ok::<_, FunctionError>(success_or_recoverable_error(
                            ctx.get_status_code().unwrap_or(hyper::StatusCode::OK),
                            Option::from(Body::from(response_body)),
                            Option::from(ctx.get_response_headers()),
                        ))
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

fn get_response_header(content_type: &ContentType) -> String {
    match content_type {
        ContentType::JSON => "application/json".into(),
        ContentType::YAML => "text/yaml".into(),
        ContentType::XML => "application/xml".into(),
        ContentType::Plain => "text/plain".into(),
        ContentType::URLEncoded => "application/x-www-form-urlencoded".into(),
    }
}

fn encode_body<S: OutputCoercible>(
    content_type: &ContentType,
    s: S,
) -> Result<Vec<u8>, FunctionError> {
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
) -> Result<T, FunctionError> {
    match content_type {
        ContentType::JSON => T::try_decode_json(buffer.to_vec()),
        ContentType::YAML => T::try_decode_yaml(buffer.to_vec()),
        ContentType::XML => T::try_decode_xml(buffer.to_vec()),
        ContentType::Plain => T::try_decode_plain(buffer.to_vec()),
        ContentType::URLEncoded => T::try_decode_urlencoded(buffer.to_vec()),
    }
}
