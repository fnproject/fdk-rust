//! # FDK: Fn Function Development Kit
//!
//! This crate implements an experimental Function Development Kit for the
//! [Fn Project](http://www.fnproject.io) serverless platform.
//!
//! The API provided hides the implementation details of the Fn platform
//! contract and allows a user to focus on the code and easily implement
//! function-as-a-service programs.
//!
//! # Usage
//!
//! The Fn platform offers a
//! [command line tool](https://github.com/fnproject/fn/blob/master/README.md#quickstart)
//! to initialize, build and deploy function projects. Follow the `fn` tool
//! quickstart to learn the basics of the Fn platform. Then start a Rust
//! function project with:
//!
//! ```text
//! fn init --runtime=rust <other options to fn command>
//! ```
//!
//! The initializer will actually use cargo and generate a cargo binary project
//! for the function. It is then possible to specify a dependency as usual.
//!
//! ```toml
//! [dependencies]
//! fdk = "0.1"
//! ```
//!
//! # Examples
//!
//! ## Stateless function with input
//!
//! This is a simple function which greets the name provided as input.
//!
//! ```no_run
//! extern crate fdk;
//! use std::process;
//!
//! fn main() {
//!     let exit_code = fdk::Function::new(fdk::STATELESS)
//!     .run(|_, i: String| {
//!         Ok(format!("Hello, {}!\n",
//!             if i.is_empty() { "world".to_string() } else { i }))
//!     });
//!     process::exit(exit_code);
//! }
//! ```
//!
//! ## Function with a full testable implementation
//!
//! This function takes advantage of features of the FDK such as configuration
//! and state management, error handling, and the testbench which provides a
//! wrapper to test the function code as if it was running on the Fn platform.
//!
//! ```no_run
//! extern crate fdk;
//! use std::process;
//!
//! struct MyState {
//!     greeting: String
//! }
//! impl MyState {
//!     pub fn greeting(&self) -> &str { &self.greeting }
//! }
//!
//! fn init(context: &fdk::RuntimeContext) -> Result<MyState, fdk::FunctionError> {
//!     match context.config().get("GREETING") {
//!         Some(s) => Ok(MyState {
//!             greeting: s.clone()
//!         }),
//!         None => Err(fdk::FunctionError::initialization(
//!             "Missing greeting in configuration\n",
//!         )),
//!     }
//! }
//!
//! fn code(state: &mut MyState, i: String) -> Result<String, fdk::FunctionError> {
//!     if !i.is_empty() {
//!         Ok(format!("{}, {}!\n", state.greeting(), i).into())
//!     } else {
//!         Err(fdk::FunctionError::invalid_input(
//!             "Requires a non-empty UTF-8 string\n",
//!         ))
//!     }
//! }
//!
//! fn main() {
//!     let exit_code = fdk::Function::new(init).run(code);
//!     process::exit(exit_code);
//! }
//!
//! #[cfg(test)]
//! mod tests {
//!     use fdk;
//!
//!     use init;
//!     use code;
//!
//!     #[test]
//!     fn test_normal_functionality() {
//!         let mut testbench =
//!             fdk::FunctionTestbench::new(init).with_config("GREETING", "Salutations");
//!         let exit_code = testbench.enqueue_simple("Blah").run(code);
//!         assert_eq!(exit_code, 0);
//!         let mut responses = testbench.drain_responses();
//!         assert_eq!(responses.len(), 1);
//!         let rb = fdk::body_as_bytes(responses.pop().unwrap().body()).unwrap();
//!         assert_eq!(String::from_utf8_lossy(&rb), "Salutations, Blah!\n");
//!     }
//! }
//! ```
//!
//! ## Function handling http requests and responses directly
//!
//! While input and output coercions can be performed so that your function can
//! just handle your own types, it is sometimes useful to manipulate requests
//! and responses directly.
//!
//! The FDK uses the `hyper::Request` and `hyper::Response` types from the
//! well-known `hyper` crate to this effect, and your function can therefore
//! receive a `hyper::Request` as input and produce a `hyper::Response` as
//! output.
//!
//! Note that this allows you to set custom headers and status code on the
//! response directly rather than relying on the helper implementations in the
//! FDK which associate errors with certain http statuses.
//!
//! ```no_run
//! extern crate hyper;
//! extern crate fdk;
//! use std::process;
//!
//! struct MyState {
//!     greeting: String
//! }
//! impl MyState {
//!     pub fn greeting(&self) -> &str { &self.greeting }
//! }
//!
//! fn init(context: &fdk::RuntimeContext) -> Result<MyState, fdk::FunctionError> {
//!     match context.config().get("GREETING") {
//!         Some(s) => Ok(MyState {
//!             greeting: s.clone()
//!         }),
//!         None => Err(fdk::FunctionError::initialization(
//!             "Missing greeting in configuration\n",
//!         )),
//!     }
//! }
//!
//! fn main() {
//!     let exit_code = fdk::Function::new(init)
//!     .run(|state, req: hyper::Request| {
//!         // Since we have raw access to the request we can inspect the
//!         // headers and extract some data.
//!         let host = match req.headers().get::<hyper::header::Host>() {
//!             Some(h) => h.hostname().to_string(),
//!             None => "NO HOST".to_string(),
//!         };
//!         let i = match fdk::body_as_bytes(req.body()) {
//!             Ok(b) => {
//!                 match String::from_utf8(b) {
//!                     Ok(s) => s,
//!                     Err(_) => String::new()
//!                 }
//!             }
//!             Err(e) => {
//!                 return Err(e);
//!             }
//!         };
//!         if i.is_empty() {
//!             // We can produce an "error response" of our own instead of an
//!             // Err value which would be converted to a 400 response with no
//!             // headers.
//!             let message = "Requires a non-empty UTF-8 string!";
//!             let message_length = message.as_bytes().len() as u64;
//!             Ok(
//!                 hyper::Response::new()
//!                     .with_status(hyper::StatusCode::BadRequest)
//!                     .with_header(hyper::header::Host::new(host.clone(), None))
//!                     .with_header(hyper::header::ContentLength(message_length))
//!                     .with_body(message)
//!             )
//!         } else {
//!             let message = format!("{}, {}!\n", state.greeting(), i);
//!             let message_length = message.as_bytes().len() as u64;
//!             Ok(
//!                 hyper::Response::new()
//!                     .with_status(hyper::StatusCode::Ok)
//!                     .with_header(hyper::header::Host::new(host.clone(), None))
//!                     .with_header(hyper::header::ContentLength(message_length))
//!                     .with_body(message)
//!             )
//!         }
//!     });
//!     process::exit(exit_code);
//! }
//! ```
extern crate futures;
extern crate hyper;
extern crate lazy_static;
extern crate object_pool;
extern crate serde_json;
extern crate tokio;
extern crate url;

//mod codecs;
mod coercions;
mod context;
mod errors;
mod function;
mod hyper_utils;
mod socket;

pub use coercions::{InputCoercible, OutputCoercible};
pub use context::RuntimeContext;
pub use errors::FunctionError;
pub use function::Function;
