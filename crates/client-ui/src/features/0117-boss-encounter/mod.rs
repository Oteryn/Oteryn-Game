//! Live boss encounter status from the admitted character's server snapshot.
#[path = "protocol/auxiliary.rs"]
mod auxiliary;
#[path = "input/chains.rs"]
mod chains;
mod input;
mod protocol;
mod state;
mod view;
pub use chains::{EndgameChain, select_next_boss};
pub use input::{cooldowns_key, open_key};
pub use protocol::{Intent, apply_result, request};
pub use state::BossEncounterPanel;
pub use view::{cooldown_text, panel_text};
#[cfg(test)]
#[path = "tests/e2e_n07b.rs"]
mod e2e_n07b;
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
