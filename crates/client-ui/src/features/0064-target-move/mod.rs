//! Native world target-walk controls; no local pathfinding or speculative movement.
pub use oteryn_client_runtime::target_move::{
    ActorPosition, CommandStatus, WalkDisposition, WalkResponse,
};

pub mod input;
pub mod protocol;
mod state;
pub mod view;
pub use input::request;
pub use protocol::apply;
pub use state::WalkState;

#[cfg(test)]
#[path = "tests/state.rs"]
mod tests;
