# FDK: Fn Function Development Kit

###### Disclaimer: This FDK is experimental and is not actively maintained. It is completely functional as of July 2021, but is not supported.

<a href="https://crates.io/crates/fdk"><img src="https://img.shields.io/crates/v/fdk.svg" alt="fdk’s current version badge" title="fdk’s current version badge" /></a>

The API provided hides the implementation details of the Fn platform
contract and allows a user to focus on the code and easily implement
function-as-a-service programs.

# Usage

The Fn platform offers a
[command line tool](https://github.com/fnproject/fn/blob/master/README.md#quickstart)
to initialize, build and deploy function projects. Follow the `fn` tool
quickstart to learn the basics of the Fn platform.

Boilerplate code can be generated using the following command:
`fn init --init-image=fnproject/rust:init`

The initializer will actually use cargo and generate a cargo binary project
for the function. It is then possible to specify a dependency as usual.

```toml
[dependencies]
fdk = ">=0.2.0"
```

# Examples

This is a simple function which greets the name provided as input. This code was generated using the above mentioned boilerplate code command.

```rust
use fdk::{Function, FunctionError, RuntimeContext};
use tokio; // Tokio for handling future.

#[tokio::main]
async fn main() -> Result<(), FunctionError> {
    if let Err(e) = Function::run(|_: &mut RuntimeContext, i: String| {
        Ok(format!(
            "Hello {}!",
            if i.is_empty() {
                "world"
            } else {
                i.trim_end_matches("\n")
            }
        ))
    })
    .await
    {
        eprintln!("{}", e);
    }
    Ok(())
}
```
