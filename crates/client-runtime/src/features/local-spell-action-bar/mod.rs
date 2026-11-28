//! Spell command construction and feedback over the session adapter.
mod bar;
mod protocol;
pub use bar::{ACTION_BAR_SLOTS, ActionBar};
mod state;
pub use oteryn_session::actor_spell_v2::{
    HouseEditorList, ParameterSpellCastResult, PrivateSpellEffect,
};
pub use oteryn_session::{
    ActorCooldown, ActorCooldownKind, ActorCooldownUpdate, ActorCooldowns, ActorVitals,
};
pub(crate) use protocol::run_cast;
pub use protocol::{cast_aimed, cast_selected, feedback_text, parameter_cast};
pub use state::SpellFeedback;
#[cfg(test)]
#[path = "tests/e2e_c04.rs"]
mod e2e_c04;
