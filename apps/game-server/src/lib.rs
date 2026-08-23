//! Foundation-only composition root for the native Oteryn Game Server.
//!
//! Bootstrap intentionally owns no gameplay listener, protocol, admission or persistence authority.

pub mod foundation;

#[cfg(test)]
#[path = "foundation/recovery_tests.rs"]
mod foundation_recovery_tests;

#[cfg(test)]
#[path = "foundation/final_review_regressions.rs"]
mod foundation_final_review_regressions;

#[cfg(test)]
#[path = "foundation/final_review_round2_regressions.rs"]
mod foundation_final_review_round2_regressions;

#[cfg(test)]
#[path = "foundation/final_review_round2_runtime_rollback.rs"]
mod foundation_final_review_round2_runtime_rollback;

use oteryn_foundation::CancellationToken;
use oteryn_simulation_determinism::{SimulationDeterminismProfile, active_profile};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use tokio::runtime::Builder;

pub const GAMEPLAY_UNAVAILABLE_REASON: &str =
    "native gameplay domains are not yet integrated with the foundation runtime";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameplayAvailability {
    UnavailableBootstrap,
}

#[derive(Debug, Clone)]
pub struct GameServerBootstrap {
    shutdown: CancellationToken,
    determinism_profile: SimulationDeterminismProfile,
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
            determinism_profile: active_profile(),
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

    #[must_use]
    pub const fn determinism_profile(&self) -> SimulationDeterminismProfile {
        self.determinism_profile
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
    use oteryn_simulation_determinism::SimulationDeterminismProfileRevision;

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
        assert_eq!(
            server.determinism_profile().revision(),
            SimulationDeterminismProfileRevision::V1
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
