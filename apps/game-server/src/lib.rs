//! Foundation-only composition root for the native Oteryn Game Server.
//!
//! Bootstrap intentionally owns no gameplay listener, protocol, admission or persistence authority.

use oteryn_foundation::CancellationToken;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use tokio::runtime::Builder;

pub const GAMEPLAY_UNAVAILABLE_REASON: &str =
    "native gameplay foundation is not allocated to the bootstrap lane";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameplayAvailability {
    UnavailableBootstrap,
}

#[derive(Debug, Clone)]
pub struct GameServerBootstrap {
    shutdown: CancellationToken,
}

impl Default for GameServerBootstrap {
    fn default() -> Self {
        Self::new()
    }
}

impl GameServerBootstrap {
    #[must_use]
    pub fn new() -> Self {
        Self {
            shutdown: CancellationToken::new(),
        }
    }

    #[must_use]
    pub const fn gameplay_availability(&self) -> GameplayAvailability {
        GameplayAvailability::UnavailableBootstrap
    }

    #[must_use]
    pub const fn gameplay_unavailable_reason(&self) -> &'static str {
        GAMEPLAY_UNAVAILABLE_REASON
    }

    pub fn request_shutdown(&self) {
        self.shutdown.cancel();
    }

    #[must_use]
    pub fn is_shutdown_requested(&self) -> bool {
        self.shutdown.is_cancelled()
    }

    pub async fn run_until_shutdown(&self) {
        self.shutdown.cancelled().await;
    }
}

#[derive(Debug)]
pub enum BootstrapSmokeError {
    Runtime(std::io::Error),
    GameplayUnexpectedlyAvailable,
}

impl Display for BootstrapSmokeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Runtime(error) => write!(formatter, "cannot create bootstrap runtime: {error}"),
            Self::GameplayUnexpectedlyAvailable => {
                formatter.write_str("bootstrap unexpectedly reported gameplay availability")
            }
        }
    }
}

impl Error for BootstrapSmokeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Runtime(error) => Some(error),
            Self::GameplayUnexpectedlyAvailable => None,
        }
    }
}

pub fn bootstrap_smoke() -> Result<(), BootstrapSmokeError> {
    let server = GameServerBootstrap::new();
    if server.gameplay_availability() != GameplayAvailability::UnavailableBootstrap {
        return Err(BootstrapSmokeError::GameplayUnexpectedlyAvailable);
    }

    server.request_shutdown();
    let runtime = Builder::new_current_thread()
        .build()
        .map_err(BootstrapSmokeError::Runtime)?;
    runtime.block_on(server.run_until_shutdown());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_is_explicitly_gameplay_unavailable() {
        let server = GameServerBootstrap::new();
        assert_eq!(
            server.gameplay_availability(),
            GameplayAvailability::UnavailableBootstrap
        );
        assert_eq!(
            server.gameplay_unavailable_reason(),
            GAMEPLAY_UNAVAILABLE_REASON
        );
        assert!(!server.is_shutdown_requested());
    }

    #[test]
    fn shutdown_is_deterministic() -> Result<(), BootstrapSmokeError> {
        let server = GameServerBootstrap::new();
        server.request_shutdown();

        let runtime = Builder::new_current_thread()
            .build()
            .map_err(BootstrapSmokeError::Runtime)?;
        runtime.block_on(server.run_until_shutdown());

        assert!(server.is_shutdown_requested());
        Ok(())
    }

    #[test]
    fn bootstrap_smoke_stays_fail_closed() -> Result<(), BootstrapSmokeError> {
        bootstrap_smoke()
    }
}
