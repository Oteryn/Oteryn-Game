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
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use oteryn_simulation_determinism::SemanticTimeMicros;

use super::chat_intent::ChatRuntime;
use super::npc_talk::{TalkNpc, talk};
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

/// The instants of the talk intents of the last second of one session.
type RecentIntents = VecDeque<u64>;

/// The NPC conversations of the admitted sessions of one Channel. The lock is never held across
/// an await and is taken after the runtime lock.
#[derive(Default)]
pub(crate) struct NpcConversations {
    sessions: Mutex<HashMap<GameSessionId, RecentIntents>>,
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
    /// The session is retired: its rate window goes with it. The conversation itself lives in
    /// the chat state of the session, shared with the chat path.
    pub(crate) fn forget(&self, session: GameSessionId) {
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.remove(&session);
        }
    }

    /// The domain 7 payload when the conversation of `session` differs from `shown`, the NPC the
    /// connection last showed: a conversation the chat path opened, or one whose NPC is gone or
    /// out of `NPC0-RL-07` range, which ends here (NPC-0 section 4.1).
    pub(crate) fn follow(
        &self,
        chat: &ChatRuntime,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        shown: Option<EntityRef>,
    ) -> Option<NpcConversation> {
        let engaged = chat.engaged(session);
        let mut current = None;
        let mut farewell = None;
        if let Some(npc_ref) = engaged {
            let visible = runtime.visible_entities();
            let near = visible
                .npcs
                .iter()
                .find(|npc| npc.actor == npc_ref)
                .zip(visible.players.iter().find(|entry| entry.actor == actor))
                .filter(|(npc, speaker)| {
                    speaker.position.floor == npc.position.floor
                        && speaker.position.x.abs_diff(npc.position.x) <= TALK_RANGE_TILES
                        && speaker.position.y.abs_diff(npc.position.y) <= TALK_RANGE_TILES
                });
            if let Some((npc, _)) = near {
                current = Some(EntityRef {
                    identity: npc.actor.placement_identity(),
                    generation: npc.generation,
                });
            } else {
                chat.end_engagement(session, npc_ref);
                farewell = chat
                    .npc_talk()
                    .and_then(|book| runtime.npc_key(npc_ref).and_then(|key| book.get(key)))
                    .map(|entry| entry.farewell().to_owned());
            }
        }
        if current == shown {
            return None;
        }
        Some(NpcConversation {
            open: current.map(|npc| (npc, NpcConversationState::Open)),
            lines: farewell.into_iter().collect(),
        })
    }

    /// One talk intent of the admitted `actor`, under the runtime lock the caller holds.
    pub(crate) fn talk(
        &self,
        chat: &ChatRuntime,
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
        let recent = sessions.entry(session).or_default();
        let now = now.get();
        while recent
            .front()
            .is_some_and(|at| now.saturating_sub(*at) >= RATE_WINDOW_MICROS)
        {
            recent.pop_front();
        }
        if recent.len() >= MAX_TALK_INTENTS_PER_SECOND as usize {
            return NpcTalkOutcome::only(Disposition::RateLimited);
        }
        recent.push_back(now);

        let (Some(book), Ok(text)) = (chat.npc_talk(), ChatText::parse(&intent.text)) else {
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
                conversation: end(chat, session, intent),
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
            // A placed NPC with no talk entry (a Dialogue-backed NPC) is silent.
            return NpcTalkOutcome::only(Disposition::NoConversation);
        };
        if at
            .distance(there)
            .is_none_or(|distance| distance > TALK_RANGE_TILES)
        {
            return NpcTalkOutcome {
                disposition: Disposition::NotInRange,
                conversation: end(chat, session, intent),
            };
        }
        let npcs = [TalkNpc {
            id: npc.actor,
            position: there,
            entry,
        }];
        let Some((reply, had, engaged)) = chat.converse(session, |engaged| {
            let had = engaged.is_some();
            let reply = talk(at, &text, &npcs, engaged);
            (reply, had, engaged.is_some())
        }) else {
            // An unknown session, or an NPC already serving `NPC0-RL-05` customers.
            return NpcTalkOutcome::only(Disposition::Rejected);
        };
        let Some(reply) = reply else {
            // Nothing recognised: without a conversation there is nobody to answer.
            return NpcTalkOutcome::only(if had {
                Disposition::Ok
            } else {
                Disposition::NoConversation
            });
        };
        let open = engaged.then_some((intent.npc_actor, NpcConversationState::Open));
        NpcTalkOutcome {
            disposition: Disposition::Ok,
            conversation: Some(NpcConversation {
                open,
                lines: vec![reply.text],
            }),
        }
    }
}

/// Ends the conversation of `session` when the intent named the NPC it was held with; returns the
/// closing payload then.
fn end(
    chat: &ChatRuntime,
    session: GameSessionId,
    intent: &NpcTalkIntent,
) -> Option<NpcConversation> {
    let engaged = chat
        .engaged(session)
        .filter(|engaged| engaged.placement_identity() == intent.npc_actor.identity)?;
    chat.end_engagement(session, engaged)
        .then(NpcConversation::default)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, reason = "test fixtures")]
#[path = "npc_command_tests.rs"]
mod tests;
