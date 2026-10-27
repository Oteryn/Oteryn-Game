//! Spell command construction and feedback over the session adapter.
mod protocol;
mod state;
pub use oteryn_session::ActorVitals;
pub(crate) use protocol::run_cast;
pub use protocol::{cast_selected, feedback_text};
pub use state::SpellFeedback;
