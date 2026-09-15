//! Application-owned Tokio runtime and deterministic shutdown boundary.

use oteryn_foundation::CancellationToken;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::time::Duration;
use tokio::runtime::{Builder, Runtime};
use tokio::task::JoinHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeError {
    Creation,
    AlreadyShutDown,
}

impl Display for RuntimeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Creation => "client runtime creation failed",
            Self::AlreadyShutDown => "client runtime is already shut down",
        })
    }
}

impl std::error::Error for RuntimeError {}

pub struct ClientRuntime {
    runtime: Option<Runtime>,
    cancellation: CancellationToken,
}

impl ClientRuntime {
    pub fn new() -> Result<Self, RuntimeError> {
        let runtime = Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("oteryn-client-runtime")
            .enable_io()
            .enable_time()
            .build()
            .map_err(|_error| RuntimeError::Creation)?;
        Ok(Self {
            runtime: Some(runtime),
            cancellation: CancellationToken::new(),
        })
    }

    #[must_use]
    pub fn cancellation(&self) -> CancellationToken {
        self.cancellation.clone()
    }

    pub fn block_on<F>(&self, future: F) -> Result<F::Output, RuntimeError>
    where
        F: Future,
    {
        self.runtime
            .as_ref()
            .map(|runtime| runtime.block_on(future))
            .ok_or(RuntimeError::AlreadyShutDown)
    }

    pub fn spawn<F>(&self, future: F) -> Result<JoinHandle<F::Output>, RuntimeError>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.runtime
            .as_ref()
            .map(|runtime| runtime.spawn(future))
            .ok_or(RuntimeError::AlreadyShutDown)
    }

    pub fn shutdown(mut self, timeout: Duration) {
        self.cancellation.cancel();
        if let Some(runtime) = self.runtime.take() {
            runtime.shutdown_timeout(timeout);
        }
    }
}

impl Drop for ClientRuntime {
    fn drop(&mut self) {
        self.cancellation.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_owns_and_completes_work() -> Result<(), RuntimeError> {
        let runtime = ClientRuntime::new()?;
        let result = runtime.block_on(async { 7_u8 })?;
        assert_eq!(result, 7);
        runtime.shutdown(Duration::from_millis(50));
        Ok(())
    }
}
