//! Compatibility exports and the shell's chronological command vocabulary.
use crate::input::StepDir;
pub use oteryn_client_runtime::combat::{CombatCommand, CombatReport, run_combat};

/// What the shell asks of the session task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayCommand {
    Step(StepDir),
    Combat(CombatCommand),
    /// Leave the game (capability 21 `LOGOUT_INTENT`).
    Logout,
}
