//! Battle panel for live spatial entities and server-owned combat state.
use oteryn_client_runtime::attack::{
    ActorCombatState, ChaseMode, EntityKind, FightMode, FightModes, WorldSpatialEntity,
};
use oteryn_client_runtime::combat::{CombatCommand, EntityRef};
use oteryn_input_actions::{ButtonState, Modifier, Modifiers, NormalizedInputEvent};

pub mod input;
pub mod protocol;
mod state;
pub mod view;
pub use input::{request, toggle_key};
pub use state::{Action, BattlePanel};
pub use view::row_text;
#[cfg(test)]
mod tests;
