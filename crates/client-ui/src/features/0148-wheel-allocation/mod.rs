//! Editable proposals and server-confirmed Wheel/Atelier state.
pub use oteryn_client_runtime::wheel_allocation::{
    CommandStatus, WheelCommand, WheelResponse, atelier, wheel,
};
mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::{Intent, apply_result, request};
pub use state::{SLOT_CAPACITIES, SLOT_DOMAINS, WheelPanel};
pub use view::{domain_name, gem_label, modifier_label, panel_text, slot_label};

#[cfg(test)]
#[path = "tests/state.rs"]
mod tests;
