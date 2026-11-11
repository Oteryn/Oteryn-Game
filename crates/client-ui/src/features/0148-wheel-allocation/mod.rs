//! Editable proposals and server-confirmed Wheel/Atelier state.
pub use oteryn_client_runtime::wheel_allocation::{
    CommandStatus, WheelCommand, WheelResponse, atelier, wheel,
};
#[path = "state/diagram/mod.rs"]
pub mod diagram;
#[path = "state/gems/mod.rs"]
pub mod gems;
mod input;
#[path = "settings.rs"]
pub mod presets;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::{
    Intent, apply_result, gem_presentation_query, request, wheel_presentation_query,
};
pub use state::{SLOT_CAPACITIES, SLOT_DOMAINS, WheelPanel};
pub use view::{
    ReshapeBlock, gem_label, is_vessel_node, modifier_label, panel_text, reshape_block,
    reshape_cost, reshape_position, slot_label, vessel_name, vessel_node_perk, workshop_lines,
};

#[cfg(test)]
#[path = "tests/state.rs"]
mod tests;
