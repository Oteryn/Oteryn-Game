//! In-game Channel switch (capability 512, GAME-CHANNEL-01 §7-§10): the Change Channel window
//! with Channel, Type, Ping, Load and Status columns, the confirmation with the Open PvP warning,
//! the wait notice, the arrival line and one error text per lock.
mod protocol;
mod state;
mod view;
pub use protocol::{apply_request, apply_status, confirm, request_status, select};
pub use state::{ChannelRow, ChannelSwitchPanel, PvpType};
pub use view::{
    arrival_text, confirm_text, cooldown_text, lock_text, panel_rows, recommended, wait_text,
};

#[cfg(test)]
mod tests;
