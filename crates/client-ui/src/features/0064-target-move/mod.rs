//! Native world target-walk controls; no local pathfinding or speculative movement.
pub use oteryn_client_runtime::target_move::{
    ActorPosition, CommandStatus, WalkDisposition, WalkResponse,
};

pub mod input;
pub mod protocol;
mod state;
pub mod view;
pub use input::{cancel, no_way, progress, request, retry, slowest_step};
pub use protocol::apply;
pub use state::{MAX_WALK_RETRIES, WALK_STALL, WalkState};

#[cfg(test)]
#[path = "tests/state.rs"]
mod tests;
