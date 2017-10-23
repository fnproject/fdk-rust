# Rusty functions

After the Gophers came the Rustaceans, and I for one welcome our arthropod
overlords in their oxidized iron carapaces.

What am I talking about? Well, I am making geeky [Rust](https://rust-lang.org) references of course. My colleagues in the [Fn project](https://fnproject.io) have already been subjected to my obsession with the Rust language, and therefore I have decided to inflict it onto the world of Fn users too.

Let me introduce you to [fdk-rust](https://github.com/fnproject/fdk-rust) and the corresponding [fdk crate](https://crates.io/crates/fdk).

The purpose of a Function Development Kit is to provide an easy-to-use and idiomatic API for writing Fn functions, so that developers can focus on their code and let the library take care of pesky tasks like input/output encoding/decoding, access to Fn configuration, and so on.

This work is still experimental and we would definitely welcome input from the Rust community to improve the implementation and the design.

That said, the FDK currently provides:

- A function runtime that wraps user code and takes care of the details of the Fn contract
- A testing runtime that allows `#[test]` Rust tests to run the function code as if it were invoked from Fn
- Extension Traits to implement new I/O coercions from requests and responses to user types

## Hello, somebody!

Now, just how good is this FDK at reducing boilerplate and hiding complexity from the developer? Let's look at some code:

```rust
extern crate fdk;
use std::process;

fn main() {
    let exit_code = fdk::Function::new(fdk::STATELESS)
        .run(|_, i: String| {
            let name = if i.is_empty() { "world".to_string() } else { i };
            Ok(format!("Hello, {}!\n", name))
        });
    process::exit(exit_code);
}
```

The FDK boilerplate, other than the dependency declaration, is essentially just `fdk::Function::new(fdk::STATELESS).run(` ... `);`. The meat of the user code is the closure / function passed to the `run` method.

This simple boilerplate code creates a new function runtime with no state and when the runtime executes it takes care of:

- Determining whether the Fn function is cold ("default" format) or hot ("http")
- Decoding the input accordingly
- Converting the body of the incoming http request to a String
- Passing it to the user code
- Running the user code
- Spotting that the result was "Ok" and therefore generating a 200 http response
- Populating the body of the 200 response with the content of the hello message
- Serializing the http response according to the Fn function format

Not bad for less than 80 characters of boilerplate.

## Configuration, error handling, testing...

But let's consider a slightly more involved program.

First of all, we want to be able to customize the greeting by reading some Fn configuration, but in a hot function we don't want to have to read the configuration for every invocation, so we want to store it in some state.
We also want to handle errors. For example we no longer want to support empty input, the caller will have to provide a valid name to be greeted, otherwise they should receive an appropriate http error response.

To handle state, we first have to define a type for our state and write an initializer for our function, which is basically a factory of the state type.

```rust
extern crate fdk;

struct MyState {
    greeting: String
}
impl MyState {
    pub fn greeting(&self) -> &str {
        &self.greeting
    }
}

fn init(context: &fdk::RuntimeContext) -> Result<MyState, fdk::FunctionError> {
    match context.config().get("GREETING") {
        Some(s) => Ok(MyState {
            greeting: s.clone()
        }),
        None => Err(fdk::FunctionError::initialization(
            "Missing greeting in configuration\n",
        )),
    }
}
```

Do we have to use a struct? Not really, we can use any `Sized` type, so even just having a `String` would work. A struct however is how things would be usually done in a real world situation, where you have more state to keep track of.

The initializer must take a reference to a `RuntimeContext` which provides it with the already parsed Fn configuration, which can then be accessed.

Then we write the implementation of our function invocation:

```rust
fn handle(state: &mut MyState, i: String) -> Result<String, fdk::FunctionError> {
    if !i.is_empty() {
        Ok(format!("{}, {}!\n", state.greeting(), i).into())
    } else {
        Err(fdk::FunctionError::invalid_input(
            "Requires a non-empty UTF-8 string\n",
        ))
    }
}
```

And finally, the boilerplate:

```rust
fn main() {
    use std::process;
    let exit_code = fdk::Function::new(init).run(handle);
    process::exit(exit_code);
}
```

By passing the initializer as the parameter of `new`, we instruct the runtime to create an instance of our state by calling the factory, and then use that instance of the state in every function invocation.

This is all quite neat, but how can we test our function? We can't just run the output executable because it needs to be run in the context of an Fn container.

Well, here is where the testing library comes in. Have a look at this:

```rust
#[cfg(test)]
mod tests {
    use fdk;

    use init;
    use code;

    #[test]
    fn test_normal_functionality() {
        // Create a testbench
        let mut testbench = fdk::FunctionTestbench::new(init);
        // Set some test configuration
        testbench.set_config("GREETING", "Salutations");
        // Enqueue a simple request. This is a helper for a POST request with
        // no custom headers and a string body.
        testbench.enqueue_simple("Blah");
        // Run the function implementation with the provided conditions
        let exit_code = testbench.run(code);
        // Perform some checks!
        assert_eq!(exit_code, 0);
        let mut responses = testbench.drain_responses();
        assert_eq!(responses.len(), 1);
        let body = fdk::body_as_bytes(responses.pop().unwrap().body()).unwrap();
        assert_eq!(String::from_utf8_lossy(&body), "Salutations, Blah!\n");
    }
}
```

And in one fell swoop, we have tested our function without even needing to spin up a Fn environment. The `FunctionTestbench` simulates all the conditions of the Fn contract and then invokes the runtime - so we are genuinely testing the production behavior.

If you have read this far and you are a Rustacean like me, I hope I have piqued your interest. Head to [fdk-rust](https://github.com/fnproject/fdk-rust) and feel free to tell us that the implementation is bad or not idiomatic enough. This is all still experimental and subject to change.
