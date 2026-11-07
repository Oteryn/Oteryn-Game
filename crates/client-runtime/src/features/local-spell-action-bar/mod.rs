//! Spell command construction and feedback over the session adapter.
mod bar;
mod protocol;
pub use bar::ActionBar;
#[cfg(test)]
#[path = "tests/e2e_c04.rs"]
mod e2e_c04;
mod state;
pub use oteryn_session::ActorVitals;
pub(crate) use protocol::run_cast;
pub use protocol::{cast_selected, feedback_text};
pub use state::SpellFeedback;
