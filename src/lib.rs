//! # FDK: Fn Function Development Kit
//!
//! This crate implements a Function Development Kit for the
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
//! quickstart to learn the basics of the Fn platform.
//!
//! The initializer will actually use cargo and generate a cargo binary project
//! for the function. It is then possible to specify a dependency as usual.
//!
//! ```toml
//! [dependencies]
//! fdk = ">=0.2.0"
//! ```
//!
//! # Examples
//!
//! This is a simple function which greets the name provided as input.
//!
//! ```rust,ignore
//! use fdk::{Function, FunctionError, RuntimeContext, Result};
//! use tokio; // Tokio for handling future.
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     if let Err(e) = Function::run(|_: &mut RuntimeContext, i: String| {
//!         Ok(format!(
//!             "Hello {}!",
//!             if i.is_empty() {
//!                 "world"
//!             } else {
//!                 i.trim_end_matches("\n")
//!             }
//!         ))
//!     })
//!     .await
//!     {
//!         eprintln!("{}", e);
//!     }
//!     Ok(())
//! }
//! ```

#![allow(clippy::upper_case_acronyms)]
extern crate clap;
extern crate futures;
extern crate hyper;
extern crate lazy_static;
extern crate object_pool;
extern crate serde_json;
extern crate serde_plain;
extern crate serde_urlencoded;
extern crate serde_xml_rs;
extern crate serde_yaml;
extern crate thiserror;
extern crate tokio;
extern crate url;

mod coercions;
mod context;
mod errors;
mod function;
mod logging;
mod socket;
mod utils;

pub use coercions::{InputCoercible, OutputCoercible};
pub use context::RuntimeContext;
pub use errors::FunctionError;
pub use function::{Function, Result};
