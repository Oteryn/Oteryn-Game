//! Source offers can explicitly restart a prompt from any conversation topic.
use super::{ComposedFreshAdmission, GameSessionId};
use super::{Offer, quest_dialogue::Context};

impl Offer {
    pub(super) fn matches_topic(&self, current: u16) -> bool {
        self.any_topic || self.topic == current
    }
}

pub(super) const CYCLOPS: &str = "oteryn:npc.a_sweaty_cyclops";

/// Crystal checks the unlocked smithing branches before the amulet service.
pub(super) fn keyword<'a>(text: &'a str, unlocked: bool) -> &'a str {
    [
        "uth'lokr",
        "uth'kean",
        "za'ralator",
        "uth'prta",
        "soul orb",
        "melt",
        "gear whell",
        "amulet",
    ]
    .into_iter()
    .find(|word| {
        (unlocked || matches!(*word, "uth'lokr" | "melt" | "gear whell" | "amulet"))
            && super::crystal_keyword(text, word)
    })
    .unwrap_or(text)
}

pub(super) fn waiting_reply(remaining: i64) -> String {
    let mut parts = Vec::new();
    for (value, unit) in [
        (remaining / 3600, "hour"),
        (remaining % 3600 / 60, "minute"),
    ] {
        if value > 0 {
            let plural = if value == 1 { "" } else { "s" };
            parts.push(format!("{value} {unit}{plural}"));
        }
    }
    let time = if parts.is_empty() {
        "less than a minute".into()
    } else {
        parts.join(" and ")
    };
    format!("Big Ben still hammering! Come back in {time}!")
}

/// An admitted service entering at topic zero supersedes the previous quote.
pub(super) fn retire_cyclops_offer(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
    context: &Context,
    text: &str,
) -> Option<()> {
    if context.2 != CYCLOPS {
        return Some(());
    }
    if matches!(
        text,
        "forge royal steel"
            | "dragon slayer trial"
            | "dragon slayer report"
            | "forging friendship"
            | "deliver skirts"
    ) {
        let mut sessions = admission.states.npc_conversations.sessions.lock().ok()?;
        let state = &mut sessions.get_mut(&session)?.quest_dialogue;
        if !state.retire_for_source(context) {
            return None;
        }
    }
    Some(())
}
