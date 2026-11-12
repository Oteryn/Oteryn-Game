//! Native controls for Hotkey Equip.
mod input;
mod protocol;
mod state;
mod view;
pub use input::{ITEM_HOTKEY_SLOTS, assign, open_key};
pub use protocol::{apply_result, command, control_request, request};
pub use state::HotkeyEquipPanel;
pub use view::panel_text;
