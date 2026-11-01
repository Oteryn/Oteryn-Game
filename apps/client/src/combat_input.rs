//! Compatibility exports and the shell's chronological command vocabulary.
use crate::input::StepDir;
pub use oteryn_client_runtime::combat::{CombatCommand, CombatReport, run_combat};

/// What the shell asks of the session task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayCommand {
    /// Capability-16 query encoded by its feature controls.
    QuestLog(Vec<u8>),
    Exiva(Vec<u8>),
    Prey(oteryn_client_runtime::prey::prey::Request),
    Wheel(oteryn_client_runtime::wheel_allocation::WheelCommand),
    Step(StepDir),
    Combat(CombatCommand),
    /// Leave the game (capability 21 `LOGOUT_INTENT`).
    Logout,
}
