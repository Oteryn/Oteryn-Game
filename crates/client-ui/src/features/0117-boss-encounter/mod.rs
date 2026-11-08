//! Live boss encounter status from the admitted character's server snapshot.
#[path = "protocol/auxiliary.rs"]
mod auxiliary;
mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::{Intent, apply_result, request};
pub use state::BossEncounterPanel;
pub use view::panel_text;
#[cfg(test)]
#[path = "tests/e2e_p02.rs"]
mod e2e_p02;
#[cfg(test)]
#[path = "tests/e2e_w02.rs"]
mod e2e_w02;
#[cfg(test)]
#[path = "tests/e2e_w03.rs"]
mod e2e_w03;
#[cfg(test)]
#[path = "tests/e2e_w10.rs"]
mod e2e_w10;
#[cfg(test)]
#[path = "tests/e2e_w11.rs"]
mod e2e_w11;
#[cfg(test)]
#[path = "tests/state.rs"]
mod tests;
