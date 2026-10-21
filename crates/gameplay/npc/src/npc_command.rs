//! NPC conversation payloads and the Channel-local talk ingress window.

use oteryn_protocol_oteryn::npc_service::{
    MAX_TALK_INTENTS_PER_SECOND, NpcConversation, NpcConversationState, NpcIntentDisposition,
    NpcTradeWindow,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use oteryn_simulation_determinism::SemanticTimeMicros;
use std::collections::VecDeque;

/// `NPC0-RL-04`: the window of the per-session talk limit.
pub const RATE_WINDOW_MICROS: u64 = 1_000_000;

/// The revisions of the NPC domains a connection has sent, carried by a resume or transfer. Used
/// only with capability 3.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NpcContinuity {
    /// Domain 7.
    pub conversation_revision: u64,
    /// Domain 8.
    pub window_revision: u64,
}

/// The whole payloads of domains 7 and 8 that changed, for the speaker alone.
#[derive(Default)]
pub struct NpcUpdate {
    pub conversation: Option<NpcConversation>,
    pub window: Option<NpcTradeWindow>,
}

/// The answer to one talk intent: its disposition and the domain 7 and 8 payloads to send after
/// the result, when the conversation or the window changed or the NPC answered.
pub struct NpcTalkOutcome {
    /// An unknown durable Bank outcome must not be sequenced as a refusal.
    pub unknown: bool,
    pub inventory_changed: bool,
    pub disposition: NpcIntentDisposition,
    pub conversation: Option<NpcConversation>,
    pub window: Option<NpcTradeWindow>,
}

impl NpcTalkOutcome {
    pub const fn only(disposition: NpcIntentDisposition) -> Self {
        Self {
            unknown: false,
            inventory_changed: false,
            disposition,
            conversation: None,
            window: None,
        }
    }
}

/// Admit at most `NPC0-RL-04` talk intents in the last second of one session.
pub fn admit_talk(recent: &mut VecDeque<u64>, now: SemanticTimeMicros) -> bool {
    let now = now.get();
    while recent
        .front()
        .is_some_and(|at| now.saturating_sub(*at) >= RATE_WINDOW_MICROS)
    {
        recent.pop_front();
    }
    if recent.len() >= MAX_TALK_INTENTS_PER_SECOND as usize {
        return false;
    }
    recent.push_back(now);
    true
}

impl NpcUpdate {
    /// Project the current conversation after the owner has resolved engagement and queued farewells.
    pub fn follow(
        current: Option<EntityRef>,
        shown: Option<EntityRef>,
        farewells: Vec<String>,
        window: Option<NpcTradeWindow>,
    ) -> Option<Self> {
        // A closed conversation is shown closed with its walk-away line first, even when it was
        // never shown open or another has opened since; that one follows on the next tick and
        // keeps its trade window. Every close since the last payload keeps its line, oldest first.
        if !farewells.is_empty() {
            return Some(NpcUpdate {
                conversation: Some(NpcConversation {
                    open: None,
                    lines: farewells,
                }),
                window,
            });
        }
        let conversation = (current != shown).then(|| NpcConversation {
            open: current.map(|npc| (npc, NpcConversationState::Open)),
            lines: Vec::new(),
        });
        (conversation.is_some() || window.is_some()).then_some(NpcUpdate {
            conversation,
            window,
        })
    }
}
