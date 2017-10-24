# FDK: Fn Function Development Kit

<a href="https://crates.io/crates/fdk"><img src="https://img.shields.io/crates/v/fdk.svg" alt="fdk’s current version badge" title="fdk’s current version badge" /></a>

This crate implements an experimental Function Development Kit for the
[Fn Project](http://www.fnproject.io) serverless platform.

The API provided hides the implementation details of the Fn platform
contract and allows a user to focus on the code and easily implement
function-as-a-service programs.

### [API Documentation](https://docs.rs/fdk)

# Usage

The Fn platform offers a
[command line tool](https://github.com/fnproject/fn/blob/master/README.md#quickstart)
to initialize, build and deploy function projects. Follow the `fn` tool
quickstart to learn the basics of the Fn platform. Then start a Rust
function project with:

```text
fn init --runtime=rust <other options to fn command>
```

The initializer will actually use cargo and generate a cargo binary project
for the function. It is then possible to specify a dependency as usual.

```toml
[dependencies]
fdk = "0.1"
```

# Simple example

This is a simple function which greets the name provided as input.

```rust
extern crate fdk;
use std::process;

fn main() {
    let exit_code = fdk::Function::new(fdk::STATELESS)
    .run(|_, i: String| {
        Ok(format!("Hello, {}!\n",
            if i.is_empty() { "world".to_string() } else { i }))
    });
    process::exit(exit_code);
}
```

More examples are available in the [API Documentation](https://docs.rs/fdk).
