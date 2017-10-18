use hyper;

use std::collections::HashMap;
use std::env;
use std::io;
use std::io::{Read, Write};
use std::ops;
use std::str::FromStr;
use std::sync::mpsc;

use codecs::{InputOutputCodec, DefaultCodec, HttpCodec};
use coercions::{InputCoercible, OutputCoercible};
use context::RuntimeContext;
use errors::FunctionError;
use hyper_utils::{clone_response, exit_code_from_response, write_request_full};

fn stateless(_: &RuntimeContext) -> Result<(), FunctionError> {
    Ok(())
}

/// This constant can be used as the initializer of a `Function` to indicate
/// that the function does not need or handle state.
///
/// For example:
///
/// ```no_run
/// let exit_code = fdk::Function::new(fdk::STATELESS)
///     .run(|_, i: String| {
///         Ok(i)
///     });
/// ```
pub const STATELESS: fn(&RuntimeContext) -> Result<(), FunctionError> = stateless;

/// A function runtime which wraps a simpler function written by the user and
/// deals with all the complexity of the Fn platform contract while providing an
/// easy to use API.
pub struct Function<S: Sized> {
    initializer: fn(&RuntimeContext) -> Result<S, FunctionError>,
}

impl<S: Sized> Function<S> {
    /// Create a new `Function` with an initializer that is basically a factory
    /// for the function's state. The initializer takes a `RuntimeContext`
    /// from which configuration data can be extracted.
    ///
    /// ```no_run
    /// struct MyState {
    ///     greeting: String
    /// }
    ///
    /// let func = fdk::Function::new(|context| Ok(MyState {
    ///         greeting: context.config().get("GREETING")
    ///             .unwrap_or(&"Hello!\n".to_string()).clone()
    ///     }));
    /// ```
    pub fn new(func: fn(&RuntimeContext) -> Result<S, FunctionError>) -> Function<S> {
        Function { initializer: func }
    }

    /// Runs the function runtime and processes any request to pass it down to
    /// the code provided. When the execution finishes, returns the exit code
    /// the program should exit with.
    ///
    /// The code provided must take the function state as the first parameter
    /// and the function input as the second. It must then return a result where
    /// Ok provides the function output and the error type is a `FunctionError`.
    ///
    /// ```no_run
    /// struct MyState {
    ///     greeting: String
    /// }
    ///
    /// let function = fdk::Function::new(|_| Ok(MyState {
    ///     greeting: "Hello".to_string()
    /// }));
    /// let exit_code = function.run(|state, i: String| {
    ///     Ok(format!("{}, {}!", state.greeting, i).to_string())
    /// });
    /// ```
    ///
    /// If the function was initialized with the `STATELESS` constant, the state
    /// parameter can be ignored (it is the empty type anyway).
    ///
    /// ```no_run
    /// let exit_code = fdk::Function::new(fdk::STATELESS).run(|_, i: String| {
    ///     Ok(format!("Hello, {}!", i).to_string())
    /// });
    /// ```
    ///
    /// Most types can be coerced from `Request`s and to `Response`s by the
    /// runtime without need for explicit conversions. If a type is not already
    /// convertible, implement the `InputCoercible` or `OutputCoercible` trait
    /// for the type.
    ///
    /// Input and output coercions are performed so that the code does not need
    /// to handle `Request`s or `Response`s directly, but it is possible to do
    /// so in cases where more control is needed.
    ///
    /// ```no_run
    /// extern crate fdk;
    /// extern crate hyper;
    /// // ...
    /// let exit_code = fdk::Function::new(fdk::STATELESS).run(|_, r: hyper::Request| {
    ///     Ok(hyper::Response::new().with_body(r.body()))
    /// });
    /// ```
    pub fn run<T, U>(self, func: fn(&mut S, T) -> Result<U, FunctionError>) -> i32
    where
        T: InputCoercible,
        U: OutputCoercible,
    {
        self.run_impl(
            func,
            env::vars(),
            Box::new(io::stdin()),
            &mut io::stdout(),
            &mut io::stderr(),
            None,
        )
    }

    fn run_impl<I, T, U>(
        self,
        func: fn(&mut S, T) -> Result<U, FunctionError>,
        environment: I,
        input: Box<Read + Send>,
        output: &mut Write,
        error_log: &mut Write,
        responses_hook: Option<mpsc::Sender<hyper::Response>>, // Only used for testing
    ) -> i32
    where
        I: Iterator<Item = (String, String)>,
        T: InputCoercible,
        U: OutputCoercible,
    {

        let env = environment.fold(HashMap::new(), |mut e, kv| {
            e.insert(kv.0, kv.1);
            e
        });
        let rc = RuntimeContext::with_environment(&env);

        let mut codec: Box<InputOutputCodec<Item = Result<hyper::Request, FunctionError>>> =
            match env.get("FN_FORMAT") {
                Some(format) => {
                    match format.as_ref() {
                        "" | "default" => Box::new(DefaultCodec::new(input, &env)),
                        "http" => Box::new(HttpCodec::new(Box::new(input))),
                        _ => {
                            error_log
                                .write(&format!("Unrecognized function format '{}'\n", format)
                                    .as_bytes())
                                .unwrap();
                            return 2;
                        }
                    }
                }
                None => Box::new(DefaultCodec::new(input, &env)),
            };

        let initializer = self.initializer;
        let mut state = match initializer(&rc) {
            Ok(s) => s,
            Err(e) => {
                let resp = match responses_hook {
                    Some(ref hook) => {
                        let (r1, r2) = clone_response(e.into());
                        hook.send(r2).unwrap();
                        r1
                    }
                    None => e.into(),
                };
                match codec.try_write(resp, output) {
                    Ok(_) => (),
                    Err(e) => {
                        error_log.write(&format!("{}\n", e).into_bytes()).unwrap();
                    }
                }
                return 2;
            }
        };

        let mut last_status = 0;
        while let Some(maybe_evt) = codec.next() {
            let mut resp = match maybe_evt {
                Ok(req) => {
                    match T::try_decode(req) {
                        Ok(i) => {
                            let maybe_res = func(&mut state, i);
                            match maybe_res {
                                Ok(res) => {
                                    match U::try_encode(res) {
                                        Ok(o) => o,
                                        Err(e) => e.into(),
                                    }
                                }
                                Err(e) => e.into(),
                            }
                        }
                        Err(e) => e.into(),
                    }
                }
                Err(e) => e.into(),
            };
            last_status = exit_code_from_response(&resp);
            resp = match responses_hook {
                Some(ref hook) => {
                    let (r1, r2) = clone_response(resp);
                    hook.send(r2).unwrap();
                    r1
                }
                None => resp,
            };
            match codec.try_write(resp, output) {
                Ok(_) => (),
                Err(e) => {
                    error_log.write(&format!("{}\n", e).into_bytes()).unwrap();
                    last_status = 2;
                }
            }

            if last_status > 1 {
                break;
            }
        }

        last_status
    }
}


/// A testing wrapper that behaves like a `Function` but additionally provides
/// methods to create testing conditions (including setting configuration and
/// enqueuing requests) and to read results back in order for tests to check
/// the behaviour of a function.
///
/// ```no_run
/// use std::process;
///
/// fn code(_: &mut (), i: String) -> Result<String, fdk::FunctionError> {
///     if !i.is_empty() {
///         Ok(format!("Hello, {}!\n", i).into())
///     } else {
///         Err(fdk::FunctionError::invalid_input(
///             "Requires a non-empty UTF-8 string\n",
///         ))
///     }
/// }
///
/// fn main() {
///     let exit_code = fdk::Function::new(fdk::STATELESS).run(code);
///     process::exit(exit_code);
/// }
///
/// #[test]
/// fn test_some_functionality() {
///     let mut testbench = fdk::FunctionTestbench::new(fdk::STATELESS);
///     // Enqueue a request: enqueue_simple() is a helper for simplicity,
///     // but you can enqueue() a full hyper::Request if needed.
///     testbench.enqueue_simple("Blah");
///     // Run the test
///     let exit_code = testbench.run(code);
///     assert_eq!(exit_code, 0);
///     // Perform some checks on the responses
///     let mut responses = testbench.drain_responses();
///     assert_eq!(responses.len(), 1);
///     let rb = fdk::body_as_bytes(responses.pop().unwrap().body()).unwrap();
///     assert_eq!(String::from_utf8_lossy(&rb), "Hello, Blah!\n");
/// }
pub struct FunctionTestbench<S: Sized> {
    initializer: fn(&RuntimeContext) -> Result<S, FunctionError>,
    environment: HashMap<String, String>,
    requests: Vec<hyper::Request>,
    responses: Vec<hyper::Response>,
    test_out: Vec<u8>,
    test_err: Vec<u8>,
}

impl<S: Sized> FunctionTestbench<S> {
    /// Create a `FunctionTestbench` for a function with the provided
    /// initializer.
    pub fn new(code: fn(&RuntimeContext) -> Result<S, FunctionError>) -> FunctionTestbench<S> {
        FunctionTestbench {
            initializer: code,
            environment: HashMap::new(),
            requests: Vec::new(),
            responses: Vec::new(),
            test_out: Vec::new(),
            test_err: Vec::new(),
        }
    }

    /// Adds a configuration variable to the environment in which the function
    /// under test will run.
    pub fn set_config(&mut self, key: &str, value: &str) {
        self.environment.insert(key.to_string(), value.to_string());
    }

    /// Adds a configuration variable to the environment in which the function
    /// under test will run and return self. Used for the owned builder pattern.
    pub fn with_config(mut self, key: &str, value: &str) -> Self {
        self.set_config(key, value);
        self
    }

    /// Enqueues a request (represented as a `hyper::Request`) to the function.
    pub fn enqueue(&mut self, req: hyper::Request) -> &mut Self {
        self.requests.push(req);
        self
    }

    /// Helper to enqueue a very simple GET request with a string body and the
    /// appropriate content length.
    pub fn enqueue_simple(&mut self, body: &str) -> &mut Self {
        let mut req = hyper::Request::new(hyper::Method::Get, hyper::Uri::from_str("/").unwrap());
        req.headers_mut().set(hyper::header::ContentLength(
            body.as_bytes().len() as u64,
        ));
        req.set_body(body.to_string());
        self.requests.push(req);
        self
    }

    /// Runs the specified function code and stores the resulting responses.
    /// This clears the list of enqueued requests and overwrites the list of
    /// responses.
    pub fn run<T, U>(&mut self, code: fn(&mut S, T) -> Result<U, FunctionError>) -> i32
    where
        T: InputCoercible,
        U: OutputCoercible,
    {
        self.responses.clear();
        self.test_out.clear();
        self.test_err.clear();
        let mut mock_in = io::Cursor::new(Vec::new());
        for r in self.requests.drain(ops::RangeFull) {
            write_request_full(r, &mut mock_in).unwrap();
        }
        mock_in.set_position(0);
        let mut mock_out = io::Cursor::new(Vec::new());
        let mut mock_err = io::Cursor::new(Vec::new());
        let mut env = self.environment.clone();
        // Force http format
        env.insert("FN_FORMAT".to_string(), "http".to_string());
        let function_under_test = Function::new(self.initializer);
        let (responses_tx, responses_rx) = mpsc::channel();
        let exit_code = function_under_test.run_impl(
            code,
            env.drain(),
            Box::new(mock_in),
            &mut mock_out,
            &mut mock_err,
            Some(responses_tx),
        );
        loop {
            match responses_rx.try_recv() {
                Ok(r) => self.responses.push(r),
                Err(_) => {
                    break;
                }
            }
        }
        mock_out.set_position(0);
        mock_err.set_position(0);
        self.test_out = mock_out.into_inner();
        self.test_err = mock_err.into_inner();
        exit_code
    }

    /// Gets the list of responses that a function run has just produced.
    pub fn responses(&self) -> &Vec<hyper::Response> {
        &self.responses
    }

    /// Drains the list of responses that a function run has just produced.
    pub fn drain_responses(&mut self) -> Vec<hyper::Response> {
        self.responses.drain(ops::RangeFull).collect()
    }

    /// Gets the raw output of the function run (i.e. the serialized responses).
    pub fn output(&self) -> &Vec<u8> {
        &self.test_out
    }

    /// Gets the raw error log of the function run.
    pub fn errlog(&self) -> &Vec<u8> {
        &self.test_err
    }
}
