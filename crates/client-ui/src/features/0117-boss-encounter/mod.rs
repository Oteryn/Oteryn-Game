//! Live boss encounter status from the admitted character's server snapshot.
#[path = "protocol/auxiliary.rs"]
mod auxiliary;
#[path = "view/browser.rs"]
mod browser;
pub use browser::{BossCategory, boss_records, browser_text, select_boss};
mod input;
mod protocol;
mod state;
mod view;
pub use input::{cooldowns_key, open_key};
pub use protocol::{Intent, apply_result, request};
pub use state::BossEncounterPanel;
pub use view::{cooldown_text, panel_text};
#[cfg(test)]
#[path = "tests/state.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/browser.rs"]
mod browser_tests;
