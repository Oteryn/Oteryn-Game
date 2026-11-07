//! Server-confirmed personal boss reward browsing and collection.
pub use oteryn_client_runtime::boss_rewards::{
    BossRewardsCommand, BossRewardsReply, BossRewardsResponse, CommandStatus, boss_rewards,
};
mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::{Intent, apply_result, request};
pub use state::BossRewardsPanel;
pub use view::{panel_text, row_text};
#[cfg(test)]
#[path = "tests/state.rs"]
mod tests;
