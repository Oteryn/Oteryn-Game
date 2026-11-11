//! Server-confirmed seven-day reward wall and one-click claim.
pub use oteryn_client_runtime::daily_reward::{
    CommandStatus, DailyRewardCommand, DailyRewardReply, DailyRewardResponse, daily_reward,
    decode_reply,
};
mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::{Intent, apply_result, request, request_details};
pub use state::DailyRewardPanel;
pub use view::{CYCLE_DAYS, cycle_day, panel_text, streak_bonus_percent};
#[cfg(test)]
#[path = "tests/state.rs"]
mod tests;

#[path = "state/returner.rs"]
mod returner;
pub use returner::{ReturnerOffer, ReturnerRewardPanel};

#[path = "state/details.rs"]
mod details;
pub use details::{apply_details, details_text};

#[path = "state/returner_wire.rs"]
mod returner_wire;
pub use protocol::request_returner;
pub use returner_wire::apply_returner;
