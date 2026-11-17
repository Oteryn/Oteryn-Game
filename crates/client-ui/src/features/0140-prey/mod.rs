//! Server-confirmed Prey lists, difficulty choice and paid repeat selection.
pub use oteryn_client_runtime::prey::{CommandStatus, PreyResponse, prey};
mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::{Intent, apply_result, request};
pub use state::{PickConfirmation, PreyPanel};
pub use view::{GridLayout, creature_label, panel_text, slot_text, tier_choices};

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "tests/state.rs"]
mod existing_actions;

#[cfg(test)]
#[path = "tests/e2e_p03.rs"]
mod e2e_p03;

#[cfg(test)]
#[path = "tests/e2e_w22b.rs"]
mod e2e_w22b;
