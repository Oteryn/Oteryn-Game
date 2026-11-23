//! Source offers can explicitly restart a prompt from any conversation topic.
use super::{ComposedFreshAdmission, GameSessionId};
use super::{Offer, quest_dialogue::Context};

impl Offer {
    pub(super) fn matches_topic(&self, current: u16) -> bool {
        self.any_topic || self.topic == current
    }
}

// Switching services retires the current forge quote even if the requested
// service is unavailable or refuses its prerequisites.
pub(super) fn retire_cyclops_steel_quote(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
    context: &Context,
    text: &str,
) -> Option<()> {
    if context.2 == "oteryn:npc.a_sweaty_cyclops"
        && (matches!(
            text,
            "amulet" | "forge royal steel" | "dragon slayer trial" | "dragon slayer report"
        ) || ["soul orb", "melt", "gear whell"]
            .into_iter()
            .any(|word| super::crystal_keyword(text, word)))
    {
        let mut sessions = admission.states.npc_conversations.sessions.lock().ok()?;
        let state = &mut sessions.get_mut(&session)?.quest_dialogue;
        if matches!(state.topic(context).number, 5..=8) && !state.retire_for_source(context) {
            return None;
        }
    }
    Some(())
}
