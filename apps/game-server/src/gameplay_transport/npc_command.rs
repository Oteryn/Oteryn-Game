//! `NPC_TALK_INTENT` (command 7) and domain 7 `NPC_CONVERSATION` (NPC-CMD-1a, NPC-0 §4).
//!
//! The conversation rule is [`super::npc_talk::talk`], shared with the chat path. This module adds
//! what the command needs around it: the NPC is named by its spatial reference (a stale generation
//! or a vanished NPC is `STALE`), the speaker must be within the talk range, each GameSession is
//! limited to `NPC0-RL-04` intents per second, and the reply goes to the speaker alone as a whole
//! domain 7 payload. Domain 8 is a read-only trade window (NPC-CMD-1b-read): a talk intent that asks
//! to `trade` of a conversation with an NPC that has offers opens it for the speaker alone, and it
//! closes with the conversation. A trade intent (command 8) is still `REJECTED` until NPC-TRADE-1.
//! Nothing here is durable.

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

use oteryn_protocol_oteryn::npc_service::{
    MAX_OFFERS, MAX_TALK_INTENTS_PER_SECOND, NpcConversation, NpcConversationState,
    NpcIntentDisposition, NpcOffer, NpcTalkIntent, NpcTradeSide, NpcTradeWindow, TALK_RANGE_TILES,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use oteryn_simulation_determinism::SemanticTimeMicros;

use super::chat_intent::ChatRuntime;
use super::npc_talk::{TalkNpc, asks_to_trade, talk};
use crate::chat::{ChatPosition, ChatText};
use crate::content::item_ref::{ITEM_FAMILY, ItemDefinitionIndex};
use crate::content::{ProjectV2ServiceOffer, ProjectV2ServiceOfferDirection};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId};

/// `NPC0-RL-04`: the window of the per-session talk limit.
const RATE_WINDOW_MICROS: u64 = 1_000_000;

/// The revisions of the NPC domains a connection has sent, carried by a resume or transfer. Used
/// only with capability 3.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct NpcContinuity {
    /// Domain 7.
    pub(crate) conversation_revision: u64,
    /// Domain 8.
    pub(crate) window_revision: u64,
}

/// The instants of the talk intents of the last second of one session.
type RecentIntents = VecDeque<u64>;

/// What the server holds of one session's NPC state.
#[derive(Default)]
struct SessionNpc {
    recent: RecentIntents,
    /// The NPC whose trade window the session was shown (domain 8).
    window: Option<EntityRef>,
}

/// The NPC conversations of the admitted sessions of one Channel. The lock is never held across
/// an await and is taken after the runtime lock.
#[derive(Default)]
pub(crate) struct NpcConversations {
    sessions: Mutex<HashMap<GameSessionId, SessionNpc>>,
}

/// The whole payloads of domains 7 and 8 that changed, for the speaker alone.
#[derive(Default)]
pub(crate) struct NpcUpdate {
    pub(crate) conversation: Option<NpcConversation>,
    pub(crate) window: Option<NpcTradeWindow>,
}

/// The answer to one talk intent: its disposition and the domain 7 and 8 payloads to send after
/// the result, when the conversation or the window changed or the NPC answered.
pub(crate) struct NpcTalkOutcome {
    pub(crate) disposition: NpcIntentDisposition,
    pub(crate) conversation: Option<NpcConversation>,
    pub(crate) window: Option<NpcTradeWindow>,
}

impl NpcTalkOutcome {
    const fn only(disposition: NpcIntentDisposition) -> Self {
        Self {
            disposition,
            conversation: None,
            window: None,
        }
    }
}

/// The offers of `offers` the window shows, in order: an Item offer whose definition the pinned
/// index names at the offer's own revision and whose count and price fit the wire. The index in
/// the result is the offer index a trade intent will name, so the commit slice reads this same
/// projection. At most [`MAX_OFFERS`].
pub(crate) fn project_offers(
    offers: &[ProjectV2ServiceOffer],
    items: &ItemDefinitionIndex,
) -> Vec<NpcOffer> {
    offers
        .iter()
        .filter_map(|offer| {
            Some(NpcOffer {
                item_definition_ref: items
                    .definition_ref(ITEM_FAMILY, &offer.item.key, &offer.item.revision)?
                    .get(),
                side: match offer.direction {
                    ProjectV2ServiceOfferDirection::SellToPlayer => NpcTradeSide::Buy,
                    ProjectV2ServiceOfferDirection::BuyFromPlayer => NpcTradeSide::Sell,
                },
                count: offer.count.unwrap_or(1).max(1),
                unit_price: u32::try_from(offer.unit_price).ok()?,
            })
        })
        .take(MAX_OFFERS)
        .collect()
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
    ) -> Option<NpcUpdate> {
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
        let window = self.close_window(session, current.map(|npc| npc.identity));
        let conversation = (current != shown).then(|| NpcConversation {
            open: current.map(|npc| (npc, NpcConversationState::Open)),
            lines: farewell.into_iter().collect(),
        });
        (conversation.is_some() || window.is_some()).then_some(NpcUpdate {
            conversation,
            window,
        })
    }

    /// The closed window to send when the session was shown one for an NPC it no longer talks to.
    fn close_window(
        &self,
        session: GameSessionId,
        talking_to: Option<[u8; 16]>,
    ) -> Option<NpcTradeWindow> {
        let mut sessions = self.sessions.lock().ok()?;
        let state = sessions.get_mut(&session)?;
        let shown = state.window?;
        talking_to
            .is_none_or(|identity| identity != shown.identity)
            .then(|| {
                state.window = None;
                NpcTradeWindow::default()
            })
    }

    /// One talk intent of the admitted `actor`, under the runtime lock the caller holds. `items`
    /// is the pinned Item index of the Channel's generation; without it no window opens.
    #[allow(
        clippy::too_many_arguments,
        reason = "the intent's whole context under one runtime lock"
    )]
    pub(crate) fn talk_in(
        &self,
        chat: &ChatRuntime,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        intent: &NpcTalkIntent,
        now: SemanticTimeMicros,
        items: Option<&ItemDefinitionIndex>,
    ) -> NpcTalkOutcome {
        let mut outcome = self.converse(chat, runtime, actor, session, intent, now);
        let talking_to = chat.engaged(session);
        let opened = (outcome.disposition == NpcIntentDisposition::Ok
            && talking_to.is_some_and(|npc| npc.placement_identity() == intent.npc_actor.identity)
            && ChatText::parse(&intent.text).is_ok_and(|text| asks_to_trade(&text)))
        .then(|| open_window(chat, runtime, intent.npc_actor, items))
        .flatten();
        outcome.window = match opened {
            Some(window) => {
                if let Ok(mut sessions) = self.sessions.lock()
                    && let Some(state) = sessions.get_mut(&session)
                {
                    state.window = Some(intent.npc_actor);
                }
                Some(window)
            }
            None => self.close_window(session, talking_to.map(|npc| npc.placement_identity())),
        };
        outcome
    }

    /// A talk intent with no pinned Item index, so no window.
    #[cfg(test)]
    fn talk(
        &self,
        chat: &ChatRuntime,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        intent: &NpcTalkIntent,
        now: SemanticTimeMicros,
    ) -> NpcTalkOutcome {
        self.talk_in(chat, runtime, actor, session, intent, now, None)
    }

    fn converse(
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
        let recent = &mut sessions.entry(session).or_default().recent;
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
                window: None,
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
                window: None,
                // The walk-away closes the conversation here, so the farewell is sent now: the
                // follow tick sees nothing left to close (NPC-0 section 4.1).
                conversation: end(chat, session, intent).map(|mut closed| {
                    closed.lines.push(entry.farewell().to_owned());
                    closed
                }),
            };
        }
        let npcs = [TalkNpc {
            id: npc.actor,
            position: there,
            entry,
        }];
        let Some((reply, had, engaged)) = chat.converse(session, |engaged| {
            let prior = *engaged;
            let had = prior.is_some();
            let reply = talk(at, &text, &npcs, engaged);
            if reply.is_none() {
                // Only the NPC this command names was offered to the rule, so a command it does not
                // answer must not end a conversation held with another NPC.
                *engaged = prior;
            }
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
            window: None,
        }
    }
}

/// The trade window of `npc` for its conversation: its offers at the book's catalogue revision.
/// `None` when the NPC has no projectable offer or there is no pinned Item index.
fn open_window(
    chat: &ChatRuntime,
    runtime: &ChannelRuntimeV1,
    npc: EntityRef,
    items: Option<&ItemDefinitionIndex>,
) -> Option<NpcTradeWindow> {
    let (book, items) = (chat.npc_talk()?, items?);
    let visible = runtime.visible_entities();
    let placed = visible.npcs.iter().find(|placed| {
        placed.actor.placement_identity() == npc.identity && placed.generation == npc.generation
    })?;
    let entry = runtime
        .npc_key(placed.actor)
        .and_then(|key| book.get(key))?;
    let offers = project_offers(entry.offers(), items);
    (!offers.is_empty()).then(|| NpcTradeWindow {
        open: Some((npc, book.catalogue_revision())),
        offers,
    })
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
