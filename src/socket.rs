use crate::FunctionError;
use hyper::server::accept::Accept;
use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::Path;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::net::UnixListener;
use url::Url;

/// UDS is a wrapper over a UnixListener. It is a `hyper::server::accept::Accept` and can be used with hyper.
pub struct UDS(UnixListener);

impl UDS {
    pub fn new() -> Result<Self, FunctionError> {
        let fn_format = std::env::var("FN_FORMAT").unwrap_or_default();
        if fn_format.as_str() != "http-stream" && fn_format.as_str() != "" {
            return Err(FunctionError::Initialization {
                inner: format!("Unsupported FN_FORMAT specified: {}", fn_format),
            });
        };

        let fn_listener = std::env::var("FN_LISTENER")?;
        if fn_listener.is_empty() {
            return Err(FunctionError::Initialization {
                inner: "FN_LISTENER not found in env".to_owned(),
            });
        };

        let socket_url = Url::parse(&fn_listener)?;

        if socket_url.scheme() != "unix" || socket_url.path() == "" {
            return Err(FunctionError::Initialization {
                inner: format!("Malformed FN_LISTENER specified: {}", socket_url.as_str()),
            });
        }

        let socket_file_path = Path::new(socket_url.path());
        let phony_socket_file_path = Path::new(socket_file_path.parent().unwrap()).join(format!(
            "phony{}",
            socket_file_path.file_name().unwrap().to_str().unwrap()
        ));

        // Try to clean up old sockets
        {
            let _ = fs::remove_file(&socket_file_path);
            let _ = fs::remove_file(&phony_socket_file_path);
        }

        let listener = UnixListener::bind(&phony_socket_file_path.to_str().unwrap())?;

        let socket = UDS(listener);
        // Set permissions to 0o666 and set symlink
        {
            let _ = std::fs::set_permissions(
                &phony_socket_file_path,
                fs::Permissions::from_mode(0o666),
            )?;

            let _ = symlink(
                &phony_socket_file_path
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap(),
                socket_file_path,
            )?;
        }
        Ok(socket)
    }
}

impl Accept for UDS {
    type Conn = tokio::net::UnixStream;
    type Error = FunctionError;

    fn poll_accept(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Self::Conn, Self::Error>>> {
        match self.0.poll_accept(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok((socket, _address))) => Poll::Ready(Some(Ok(socket))),
            Poll::Ready(Err(err)) => Poll::Ready(Some(Err(err.into()))),
        }
    }
}
