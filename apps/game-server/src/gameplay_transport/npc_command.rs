//! `NPC_TALK_INTENT` (command 7) and domain 7 `NPC_CONVERSATION` (NPC-CMD-1a, NPC-0 §4).
//!
//! The conversation rule is [`super::npc_talk::talk`], shared with the chat path. This module adds
//! what the command needs around it: the NPC is named by its spatial reference (a stale generation
//! or a vanished NPC is `STALE`), the speaker must be within the talk range, each GameSession is
//! limited to `NPC0-RL-04` intents per second, and the reply goes to the speaker alone as a whole
//! domain 7 payload. Domain 8 is a read-only trade window (NPC-CMD-1b-read): a talk intent that asks
//! to `trade` of a conversation with an NPC that has offers opens it for the speaker alone, and it
//! closes with the conversation. A trade intent (command 8, NPC-TRADE-2) names an offer of the
//! window open on the NPC of the session's conversation; [`NpcConversations::trade_in`] resolves it
//! against the server catalogue and the connection commits it as one durable NPC trade
//! (NPC-TRADE-1). Nothing here is durable.

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

use oteryn_protocol_oteryn::npc_service::{
    MAX_OFFERS, MAX_TALK_INTENTS_PER_SECOND, NpcConversation, NpcConversationState,
    NpcIntentDisposition, NpcOffer, NpcTalkIntent, NpcTradeIntent, NpcTradeSide, NpcTradeWindow,
    TALK_RANGE_TILES,
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
    // D1103: confirmation and retained receipt intent use this existing session owner.
    vocation: Option<super::vocation_choice::NpcVocationConversation>,
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
/// the result is the offer index a trade intent names, so [`NpcConversations::trade_in`] reads this
/// same projection. At most [`MAX_OFFERS`].
pub(crate) fn project_offers(
    offers: &[ProjectV2ServiceOffer],
    items: &ItemDefinitionIndex,
) -> Vec<NpcOffer> {
    projected(offers, items).map(|(offer, _)| offer).collect()
}

/// Each projected offer with the catalogue offer it shows.
fn projected<'a>(
    offers: &'a [ProjectV2ServiceOffer],
    items: &'a ItemDefinitionIndex,
) -> impl Iterator<Item = (NpcOffer, &'a ProjectV2ServiceOffer)> {
    offers
        .iter()
        .filter_map(|offer| {
            let shown = NpcOffer {
                item_definition_ref: items
                    .definition_ref(ITEM_FAMILY, &offer.item.key, &offer.item.revision)?
                    .get(),
                side: match offer.direction {
                    ProjectV2ServiceOfferDirection::SellToPlayer => NpcTradeSide::Buy,
                    ProjectV2ServiceOfferDirection::BuyFromPlayer => NpcTradeSide::Sell,
                },
                count: offer.count.unwrap_or(1).max(1),
                unit_price: u32::try_from(offer.unit_price).ok()?,
            };
            Some((shown, offer))
        })
        .take(MAX_OFFERS)
}

/// The offer a trade intent names, as the server catalogue holds it at the window's revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NpcTradeOffer {
    pub(crate) npc_key: String,
    pub(crate) catalogue_revision: u64,
    pub(crate) offer_index: u32,
    pub(crate) side: NpcTradeSide,
    pub(crate) quantity: u32,
    /// Units per trade row.
    pub(crate) count: u32,
    pub(crate) unit_price: u32,
    /// The offered Item's definition key and revision.
    pub(crate) item_key: String,
    pub(crate) item_revision: String,
}

/// A trade intent that is not committed: its disposition and the domain 7 and 8 payloads to send.
pub(crate) struct NpcTradeRefusal {
    pub(crate) disposition: NpcIntentDisposition,
    pub(crate) update: NpcUpdate,
}

impl NpcTradeRefusal {
    pub(crate) fn only(disposition: NpcIntentDisposition) -> Self {
        Self {
            disposition,
            update: NpcUpdate::default(),
        }
    }
}

/// `NPC0` §5: at most this many offer rows in one trade.
const MAX_TRADE_QUANTITY: u32 = 100;

impl NpcConversations {
    pub(crate) fn dawnport_choice(
        &self,
        session: GameSessionId,
    ) -> Option<super::vocation_choice::ConfirmedNpcVocation> {
        self.sessions
            .lock()
            .ok()?
            .get(&session)?
            .vocation
            .as_ref()?
            .confirmed()
    }
    pub(crate) fn retain_dawnport_choice(
        &self,
        session: GameSessionId,
        occurrence: crate::durability::character_build::BuildOccurrence,
        request: super::vocation_choice::RetainedNpcVocation,
    ) -> bool {
        self.sessions
            .lock()
            .ok()
            .and_then(|mut sessions| {
                sessions
                    .get_mut(&session)?
                    .vocation
                    .as_mut()
                    .map(|topic| topic.retain(occurrence, request))
            })
            .unwrap_or(false)
    }
    pub(crate) fn rollback_dawnport_request(
        &self,
        session: GameSessionId,
        occurrence: crate::durability::character_build::BuildOccurrence,
    ) {
        if let Ok(mut sessions) = self.sessions.lock() {
            if let Some(topic) = sessions
                .get_mut(&session)
                .and_then(|state| state.vocation.as_mut())
            {
                topic.rollback_request(occurrence);
            }
        }
    }
    pub(crate) fn complete_dawnport_choice(
        &self,
        session: GameSessionId,
        occurrence: crate::durability::character_build::BuildOccurrence,
    ) {
        if let Ok(mut sessions) = self.sessions.lock() {
            if let Some(state) = sessions.get_mut(&session) {
                if state
                    .vocation
                    .as_ref()
                    .and_then(|topic| topic.confirmed())
                    .is_some_and(|choice| choice.occurrence == occurrence)
                {
                    state.vocation = None;
                }
            }
        }
    }

    /// The session is retired: its rate window goes with it. The conversation itself lives in
    /// the chat state of the session, shared with the chat path.
    pub(crate) fn forget(&self, session: GameSessionId) {
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.remove(&session);
        }
    }

    /// The domain 7 payload when the conversation of `session` differs from `shown`, the NPC the
    /// connection last showed: a conversation the chat path opened, or one a move closed or whose
    /// NPC is gone or out of `NPC0-RL-07` range, which ends here with the NPC's walk-away line
    /// (NPC-0 section 4.1, NPC-BEHAVIOUR-0 section 7). The range check runs on every committed
    /// step, relocation and respawn ([`ChatRuntime::close_out_of_range`]); here it also catches an
    /// NPC that moved or left.
    pub(crate) fn follow(
        &self,
        chat: &ChatRuntime,
        runtime: &ChannelRuntimeV1,
        session: GameSessionId,
        shown: Option<EntityRef>,
    ) -> Option<NpcUpdate> {
        chat.close_out_of_range(runtime, session);
        let current = chat.engaged(session).and_then(|npc_ref| {
            let visible = runtime.visible_entities();
            let npc = visible.npcs.iter().find(|npc| npc.actor == npc_ref)?;
            Some(EntityRef {
                identity: npc.actor.placement_identity(),
                generation: npc.generation,
            })
        });
        // A closed conversation is shown closed with its walk-away line first, even when it was
        // never shown open or another has opened since; that one follows on the next tick and
        // keeps its trade window. Every close since the last payload keeps its line, oldest first.
        let farewells = chat.take_walked_away(session);
        if !farewells.is_empty() {
            return Some(NpcUpdate {
                conversation: Some(NpcConversation {
                    open: None,
                    lines: farewells,
                }),
                window: self.close_window(session, current.map(|npc| npc.identity)),
            });
        }
        let window = self.close_window(session, current.map(|npc| npc.identity));
        let conversation = (current != shown).then(|| NpcConversation {
            open: current.map(|npc| (npc, NpcConversationState::Open)),
            lines: Vec::new(),
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

    /// One trade intent of the admitted `actor`, under the runtime lock the caller holds (NPC-0
    /// §5.1). The named NPC must be the visible one of this generation (else `STALE`), the one the
    /// session's conversation is held with (else `NO_CONVERSATION`) and within the talk range
    /// (else `NOT_IN_RANGE`). The window must be open on it and the quantity within 1..=100 (else
    /// `REJECTED`). The catalogue revision, the offer index, the side and the unit price must be
    /// the server's at that revision: any mismatch is `STALE` and re-projects the window. Charged,
    /// fluid and non-gold offers are outside the trade commit and `REJECTED`.
    pub(crate) fn trade_in(
        &self,
        chat: &ChatRuntime,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        intent: &NpcTradeIntent,
        items: Option<&ItemDefinitionIndex>,
    ) -> Result<NpcTradeOffer, Box<NpcTradeRefusal>> {
        use NpcIntentDisposition as Disposition;
        let visible = runtime.visible_entities();
        let Some(speaker) = visible.players.iter().find(|entry| entry.actor == actor) else {
            return Err(Box::new(NpcTradeRefusal::only(Disposition::Rejected)));
        };
        let Some(npc) = visible.npcs.iter().find(|npc| {
            npc.actor.placement_identity() == intent.npc_actor.identity
                && npc.generation == intent.npc_actor.generation
        }) else {
            return Err(Box::new(NpcTradeRefusal::only(Disposition::Stale)));
        };
        if chat.engaged(session) != Some(npc.actor) {
            return Err(Box::new(NpcTradeRefusal::only(Disposition::NoConversation)));
        }
        if speaker.position.floor != npc.position.floor
            || speaker.position.x.abs_diff(npc.position.x) > TALK_RANGE_TILES
            || speaker.position.y.abs_diff(npc.position.y) > TALK_RANGE_TILES
        {
            return Err(Box::new(NpcTradeRefusal::only(Disposition::NotInRange)));
        }
        let shown = self
            .sessions
            .lock()
            .ok()
            .and_then(|sessions| sessions.get(&session).and_then(|state| state.window));
        if shown != Some(intent.npc_actor) {
            // No window is open on this NPC: there is nothing to re-project.
            return Err(Box::new(NpcTradeRefusal::only(Disposition::Rejected)));
        }
        // NPC-0 §5.1: the client's revision, offer and price are only stale checks; a mismatch
        // is STALE and re-projects the window.
        let stale = || {
            let window = open_window(chat, runtime, intent.npc_actor, items).unwrap_or_default();
            if window.open.is_none()
                && let Ok(mut sessions) = self.sessions.lock()
                && let Some(state) = sessions.get_mut(&session)
            {
                state.window = None;
            }
            NpcTradeRefusal {
                disposition: Disposition::Stale,
                update: NpcUpdate {
                    conversation: None,
                    window: Some(window),
                },
            }
        };
        let rejected = || NpcTradeRefusal::only(Disposition::Rejected);
        let (Some(book), Some(items), Some(npc_key)) =
            (chat.npc_talk(), items, runtime.npc_key(npc.actor))
        else {
            return Err(Box::new(rejected()));
        };
        let Some(entry) = book.get(npc_key) else {
            return Err(Box::new(rejected()));
        };
        if !(1..=MAX_TRADE_QUANTITY).contains(&intent.quantity) {
            return Err(Box::new(rejected()));
        }
        if intent.catalogue_revision != book.catalogue_revision() {
            return Err(Box::new(stale()));
        }
        let Some((offer, source)) = usize::try_from(intent.offer_index)
            .ok()
            .and_then(|index| projected(entry.offers(), items).nth(index))
        else {
            return Err(Box::new(stale()));
        };
        if offer.side != intent.side || offer.unit_price != intent.expected_unit_price {
            return Err(Box::new(stale()));
        }
        if source.currency.is_some() || source.sub_type.is_some() {
            return Err(Box::new(rejected()));
        }
        Ok(NpcTradeOffer {
            npc_key: npc_key.to_owned(),
            catalogue_revision: intent.catalogue_revision,
            offer_index: intent.offer_index,
            side: offer.side,
            quantity: intent.quantity,
            count: offer.count,
            unit_price: offer.unit_price,
            item_key: source.item.key.clone(),
            item_revision: source.item.revision.clone(),
        })
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
        if runtime.npc_key(npc.actor) == Some(super::vocation_choice::DAWNPORT_CHOICE_NPC) {
            if runtime.owner_fence().is_err()
                || !runtime
                    .player_control_facts(actor, session)
                    .is_ok_and(|facts| facts.control_loss.is_none())
            {
                return NpcTalkOutcome::only(Disposition::Rejected);
            }
            if at
                .distance(there)
                .is_none_or(|distance| distance > TALK_RANGE_TILES)
            {
                return NpcTalkOutcome {
                    disposition: Disposition::NotInRange,
                    conversation: end(chat, session, intent),
                    window: None,
                };
            }
            // Dialogue-backed Oressa does not need a generated-reply book entry.
            // Rate, exact generation, present actor and range were checked by this owner.
            return super::vocation_choice::dawnport_choice_dialogue(
                &mut sessions.entry(session).or_default().vocation,
                chat,
                actor,
                session,
                intent.npc_actor,
                npc.actor,
                text.as_str(),
            );
        }
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
