//! Source offers can explicitly restart a prompt from any conversation topic.
use super::{ComposedFreshAdmission, GameSessionId};
use super::{Offer, quest_dialogue::Context};

impl Offer {
    pub(super) fn matches_topic(&self, current: u16) -> bool {
        self.any_topic || self.topic == current
    }
}

pub(super) const CYCLOPS: &str = "oteryn:npc.a_sweaty_cyclops";

pub(super) fn amulet_keyword(text: &str) -> &str {
    if super::crystal_keyword(text, "amulet") {
        "amulet"
    } else {
        text
    }
}

/// The donor recognizes soul-orb requests only after the forging unlock.
fn forging_unlocked(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
    offers: &[Offer],
) -> Option<bool> {
    let catalogue = admission.quest_catalogue.as_deref()?;
    let sessions = admission.states.quest_sessions.lock().ok()?;
    let tracks = sessions
        .get(&session)?
        .copy
        .as_ref()?
        .read_tracks(catalogue)?;
    // Topic 6 is the unlocked draconian steel quote. Use its admitted storage gate.
    Some(offers.iter().any(|offer| {
        offer.next_topic == 6 && offer.admits(CYCLOPS, 0, "uth'lokr", catalogue, &tracks)
    }))
}

/// An admitted service entering at topic zero supersedes the previous quote.
pub(super) fn retire_cyclops_offer(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
    context: &Context,
    offers: &[Offer],
    text: &str,
) -> Option<()> {
    if context.2 != CYCLOPS {
        return Some(());
    }
    if matches!(
        text,
        "amulet"
            | "forge royal steel"
            | "dragon slayer trial"
            | "dragon slayer report"
            | "forging friendship"
            | "deliver skirts"
    ) || ["melt", "gear whell"]
        .into_iter()
        .any(|word| super::crystal_keyword(text, word))
        || (super::crystal_keyword(text, "soul orb")
            && forging_unlocked(admission, session, offers)?)
    {
        let mut sessions = admission.states.npc_conversations.sessions.lock().ok()?;
        let state = &mut sessions.get_mut(&session)?.quest_dialogue;
        if !state.retire_for_source(context) {
            return None;
        }
    }
    Some(())
}
