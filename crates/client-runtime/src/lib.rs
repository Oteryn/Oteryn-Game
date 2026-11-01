//! Application-owned Tokio runtime and deterministic shutdown boundary.

pub mod connection;
pub mod hotkey_equip;

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

/// Transport-owned Bestiary and Charm values exposed to feature protocol adapters.
pub mod cyclopedia_protocol {
    pub use oteryn_session::{
        BestiaryRaceProgress, CharmAssignDisposition, CharmAssignIntent, CharmKind, CharmState,
        CharmUnlockDisposition, CharmUnlockStageIntent, CharmView,
    };
}

/// Attack command execution and disposition mapping.
#[path = "features/0017-attack/protocol.rs"]
pub mod attack;
/// Session-owned combat command vocabulary and routing.
pub mod combat;
/// Quest log state and selected-capability command construction.
#[path = "features/0016-quest-log/mod.rs"]
pub mod quest_log;
/// Spell command execution and feedback.
#[path = "features/local-spell-action-bar/mod.rs"]
pub mod spell;

/// Server-owned Exiva settings wire types.
pub use oteryn_session::exiva;

/// Guild command execution through the admitted session.
#[path = "features/0040-guild/protocol.rs"]
pub mod guild;
/// Party command execution through the admitted session.
#[path = "features/0042-party/protocol.rs"]
pub mod party;
/// Player-trade command execution through the admitted session.
#[path = "features/0060-player-trade/protocol.rs"]
pub mod player_trade;
/// Prey command execution through the admitted session.
#[path = "features/0140-prey/protocol.rs"]
pub mod prey;
/// Wheel and Atelier command execution through the admitted session.
#[path = "features/0148-wheel-allocation/protocol.rs"]
pub mod wheel_allocation;

/// Task Board command execution through the admitted session.
#[path = "features/0072-task-board/protocol.rs"]
pub mod task_board;

/// Stash command execution through the admitted session.
#[path = "features/0062-stash/protocol.rs"]
pub mod stash;

/// Capability-81 NPC shop command execution through the admitted session.
#[path = "features/0081-npc-shop/protocol.rs"]
pub mod npc_shop;

/// Imbuement commands executed through the admitted session.
#[path = "features/0068-imbue/protocol.rs"]
pub mod imbue;

/// VIP list, groups and consent commands.
#[path = "features/0043-vip/protocol.rs"]
pub mod vip;

/// Hireling command execution through the admitted session.
#[path = "features/0165-hireling/protocol.rs"]
pub mod hireling;

/// Market queries and settlement intents through the admitted session.
#[path = "features/0063-market/protocol.rs"]
pub mod market;
