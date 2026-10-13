//! `NPC_TALK_INTENT` (command 7) and domain 7 `NPC_CONVERSATION` (NPC-CMD-1a, NPC-0 §4).
//!
//! The conversation rule is [`super::npc_talk::talk`], shared with the chat path. This module adds
//! what the command needs around it: the NPC is named by its spatial reference (a stale generation
//! or a vanished NPC is `STALE`), the speaker must be within the talk range, each GameSession is
//! limited to `NPC0-RL-04` intents per second, and the reply goes to the speaker alone as a whole
//! domain 7 payload. Trade (command 8, domain 8) is NPC-CMD-1b: the window stays closed and a
//! trade intent is `REJECTED`. Nothing here is durable.

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

use oteryn_protocol_oteryn::npc_service::{
    MAX_TALK_INTENTS_PER_SECOND, NpcConversation, NpcConversationState, NpcIntentDisposition,
    NpcTalkIntent, TALK_RANGE_TILES,
};
use oteryn_simulation_determinism::SemanticTimeMicros;

use super::npc_talk::{NpcTalkBook, TalkNpc, talk};
use crate::chat::{ChatPosition, ChatText};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId};

/// `NPC0-RL-04`: the window of the per-session talk limit.
const RATE_WINDOW_MICROS: u64 = 1_000_000;

/// The revisions of the NPC domains a connection has sent, carried by a resume or transfer. Used
/// only with capability 3.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct NpcContinuity {
    /// Domain 7.
    pub(crate) conversation_revision: u64,
    /// Domain 8; the window is closed until NPC-CMD-1b, so it only has its snapshot.
    pub(crate) window_revision: u64,
}

#[derive(Default)]
struct NpcSession {
    /// The NPC this session is in conversation with.
    engaged: Option<ExactActorRef>,
    /// The instants of the intents of the last second.
    recent: VecDeque<u64>,
}

/// The NPC conversations of the admitted sessions of one Channel. The lock is never held across
/// an await and is taken after the runtime lock.
#[derive(Default)]
pub(crate) struct NpcConversations {
    sessions: Mutex<HashMap<GameSessionId, NpcSession>>,
}

/// The answer to one talk intent: its disposition and the domain 7 payload to send after the
/// result, when the conversation changed or the NPC answered.
pub(crate) struct NpcTalkOutcome {
    pub(crate) disposition: NpcIntentDisposition,
    pub(crate) conversation: Option<NpcConversation>,
}

impl NpcTalkOutcome {
    const fn only(disposition: NpcIntentDisposition) -> Self {
        Self {
            disposition,
            conversation: None,
        }
    }
}

impl NpcConversations {
    /// A fresh snapshot (admission, reconnect or transfer) shows no conversation: the session
    /// starts without one and keeps its rate window.
    pub(crate) fn reset(&self, session: GameSessionId) {
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.entry(session).or_default().engaged = None;
        }
    }

    /// One talk intent of the admitted `actor`, under the runtime lock the caller holds.
    pub(crate) fn talk(
        &self,
        book: Option<&NpcTalkBook>,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        intent: &NpcTalkIntent,
        now: SemanticTimeMicros,
    ) -> NpcTalkOutcome {
        use NpcIntentDisposition as Disposition;
        let Ok(mut sessions) = self.sessions.lock() else {
            return NpcTalkOutcome::only(Disposition::Rejected);
        };
        let state = sessions.entry(session).or_default();
        let now = now.get();
        while state
            .recent
            .front()
            .is_some_and(|at| now.saturating_sub(*at) >= RATE_WINDOW_MICROS)
        {
            state.recent.pop_front();
        }
        if state.recent.len() >= MAX_TALK_INTENTS_PER_SECOND as usize {
            return NpcTalkOutcome::only(Disposition::RateLimited);
        }
        state.recent.push_back(now);

        let (Some(book), Ok(text)) = (book, ChatText::parse(&intent.text)) else {
            return NpcTalkOutcome::only(Disposition::Rejected);
        };
        let visible = runtime.visible_entities();
        let Some(speaker) = visible.players.iter().find(|entry| entry.actor == actor) else {
            return NpcTalkOutcome::only(Disposition::Rejected);
        };
        let Some(npc) = visible.npcs.iter().find(|npc| {
            npc.actor.placement_identity() == intent.npc_actor.identity
                && npc.generation == intent.npc_actor.generation
        }) else {
            // The NPC is gone or is another generation: any conversation with it is over.
            return NpcTalkOutcome {
                disposition: Disposition::Stale,
                conversation: end(state, intent),
            };
        };
        let (Ok(speaker_floor), Ok(npc_floor)) = (
            u8::try_from(speaker.position.floor),
            u8::try_from(npc.position.floor),
        ) else {
            return NpcTalkOutcome::only(Disposition::Rejected);
        };
        let at = ChatPosition {
            x: speaker.position.x,
            y: speaker.position.y,
            floor: speaker_floor,
        };
        let there = ChatPosition {
            x: npc.position.x,
            y: npc.position.y,
            floor: npc_floor,
        };
        let Some(entry) = runtime.npc_key(npc.actor).and_then(|key| book.get(key)) else {
            return NpcTalkOutcome::only(Disposition::Rejected);
        };
        if at
            .distance(there)
            .is_none_or(|distance| distance > TALK_RANGE_TILES)
        {
            return NpcTalkOutcome {
                disposition: Disposition::NotInRange,
                conversation: end(state, intent),
            };
        }
        let had = state.engaged;
        let npcs = [TalkNpc {
            id: npc.actor,
            position: there,
            entry,
        }];
        let Some(reply) = talk(at, &text, &npcs, &mut state.engaged) else {
            // Nothing recognised: without a conversation there is nobody to answer.
            return NpcTalkOutcome::only(if had.is_some() {
                Disposition::Ok
            } else {
                Disposition::NoConversation
            });
        };
        let open = state
            .engaged
            .map(|_| (intent.npc_actor, NpcConversationState::Open));
        NpcTalkOutcome {
            disposition: Disposition::Ok,
            conversation: Some(NpcConversation {
                open,
                lines: vec![reply.text],
            }),
        }
    }
}

/// Ends the conversation of `state` when the intent named the NPC it was held with; returns the
/// closing payload then.
fn end(state: &mut NpcSession, intent: &NpcTalkIntent) -> Option<NpcConversation> {
    let held_with_it = state
        .engaged
        .is_some_and(|engaged| engaged.placement_identity() == intent.npc_actor.identity);
    if held_with_it {
        state.engaged = None;
    }
    held_with_it.then(NpcConversation::default)
}

#[cfg(test)]
#[path = "npc_command_tests.rs"]
mod tests;
