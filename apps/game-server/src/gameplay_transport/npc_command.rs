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
//! (NPC-TRADE-1). A Premium service topic (PREM-5: `promotion` or a blessing name) of an NPC
//! that an installed [`PremiumNpcServiceCatalogue`] admits is prompted here and confirmed by the
//! next `yes` through [`super::npc_promotion`]. Nothing here is durable.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, OnceLock};

use oteryn_protocol_oteryn::npc_service::{
    MAX_OFFERS, MAX_TALK_INTENTS_PER_SECOND, NpcConversation, NpcConversationState,
    NpcIntentDisposition, NpcOffer, NpcTalkIntent, NpcTradeIntent, NpcTradeSide, NpcTradeWindow,
    TALK_RANGE_TILES,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use oteryn_simulation_determinism::SemanticTimeMicros;

use super::bank_npc::{BankConfirmation, BankDialogueContext, BankNpcOperation};
use super::chat_intent::ChatRuntime;
use super::npc_promotion::{
    PendingPremiumService, PremiumNpcServiceCatalogue, PremiumTalkAnswer, PremiumTalkStep,
    premium_prompt, premium_topic,
};
use super::npc_talk::{TalkNpc, asks_to_trade, talk};
use crate::chat::{ChatPosition, ChatText};
use crate::content::item_ref::{ITEM_FAMILY, ItemDefinitionIndex};
use crate::content::{ProjectV2ServiceOffer, ProjectV2ServiceOfferDirection};
use crate::durability::bank::BankCoinFacts;
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
    bank: Option<PendingBank>,
    /// PREM-5: the one Premium service offer awaiting `yes`.
    premium: Option<PendingPremiumService>,
}

pub(crate) struct PendingBank {
    pub(crate) confirmation: BankConfirmation,
    pub(crate) fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    pub(crate) node_id: [u8; 16],
    pub(crate) node_revision: u64,
    pub(crate) catalogue_revision: u64,
    pub(crate) coins: Option<BankCoinFacts>,
}

/// The NPC conversations of the admitted sessions of one Channel. The lock is never held across
/// an await and is taken after the runtime lock.
#[derive(Default)]
pub(crate) struct NpcConversations {
    sessions: Mutex<HashMap<GameSessionId, SessionNpc>>,
    /// PREM-5: the qualified Premium NPC service source of the Channel, once installed. Without
    /// it no promotion or blessing topic is offered.
    premium_services: OnceLock<Arc<dyn PremiumNpcServiceCatalogue>>,
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
    /// An unknown durable Bank outcome must not be sequenced as a refusal.
    pub(crate) unknown: bool,
    pub(crate) inventory_changed: bool,
    pub(crate) disposition: NpcIntentDisposition,
    pub(crate) conversation: Option<NpcConversation>,
    pub(crate) window: Option<NpcTradeWindow>,
}

impl NpcTalkOutcome {
    const fn only(disposition: NpcIntentDisposition) -> Self {
        Self {
            unknown: false,
            inventory_changed: false,
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
    #[allow(
        dead_code,
        reason = "D1220: A2 NPC caller must retrieve the retained confirmed choice; preserve this authored port until D1173 integration."
    )]
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
    #[allow(
        dead_code,
        reason = "D1220: A2 NPC caller retains the exact durable request across unknown commits; D1173 integration is pending."
    )]
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
    #[allow(
        dead_code,
        reason = "D1220: A2 NPC caller clears only the rolled-back retained request; D1173 integration is pending."
    )]
    pub(crate) fn rollback_dawnport_request(
        &self,
        session: GameSessionId,
        occurrence: crate::durability::character_build::BuildOccurrence,
    ) {
        if let Ok(mut sessions) = self.sessions.lock()
            && let Some(topic) = sessions
                .get_mut(&session)
                .and_then(|state| state.vocation.as_mut())
        {
            topic.rollback_request(occurrence);
        }
    }
    #[allow(
        dead_code,
        reason = "D1220: A2 NPC caller clears the confirmed topic after actual durable completion; D1173 integration is pending."
    )]
    pub(crate) fn complete_dawnport_choice(
        &self,
        session: GameSessionId,
        occurrence: crate::durability::character_build::BuildOccurrence,
    ) {
        if let Ok(mut sessions) = self.sessions.lock()
            && let Some(state) = sessions.get_mut(&session)
            && state
                .vocation
                .as_ref()
                .and_then(|topic| topic.confirmed())
                .is_some_and(|choice| choice.occurrence == occurrence)
        {
            state.vocation = None;
        }
    }
    /// Cancels the pending confirmation of `session`: a Bank operation or a Premium service.
    pub(crate) fn cancel_bank(&self, session: GameSessionId) {
        if let Ok(mut sessions) = self.sessions.lock()
            && let Some(state) = sessions.get_mut(&session)
        {
            state.bank = None;
            state.premium = None;
        }
    }

    /// Installs the Premium NPC service source once; a second install is refused. The node
    /// installs the source-backed one at composition (PREM-5, `serve_gameplay`).
    pub(crate) fn install_premium_services(
        &self,
        catalogue: Arc<dyn PremiumNpcServiceCatalogue>,
    ) -> bool {
        self.premium_services.set(catalogue).is_ok()
    }

    /// Whether a Premium NPC service source is installed: without one no Premium purchase
    /// can have been made, so a resume has nothing to reconcile.
    pub(crate) fn premium_services_installed(&self) -> bool {
        self.premium_services.get().is_some()
    }

    /// PREM-5: the Premium service step of one `OK` talk intent, under the Channel runtime lock.
    /// Every intent consumes the pending offer. A `yes` of the same NPC generation and
    /// conversation confirms it; an admitted topic opens a new prompt and cancels a pending Bank
    /// operation. Anything else is `None`: the Bank and ordinary conversation go on.
    pub(crate) fn premium_step(
        &self,
        chat: &ChatRuntime,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        intent: &NpcTalkIntent,
    ) -> Option<PremiumTalkStep> {
        let catalogue = self.premium_services.get()?.clone();
        let pending = self
            .sessions
            .lock()
            .ok()?
            .get_mut(&session)
            .and_then(|state| state.premium.take());
        let (generation, npc_key) =
            Self::current_service_npc(chat, runtime, actor, session, intent)?;
        let text = intent.text.trim_matches(' ');
        if text.eq_ignore_ascii_case("yes") {
            let pending = pending.filter(|pending| {
                pending.npc == intent.npc_actor && pending.conversation_generation == generation
            })?;
            return Some(PremiumTalkStep::Confirm { pending, catalogue });
        }
        let topic = premium_topic(text)?;
        let line = premium_prompt(catalogue.as_ref(), &npc_key, topic)?;
        let mut sessions = self.sessions.lock().ok()?;
        let state = sessions.entry(session).or_default();
        state.bank = None;
        state.premium = Some(PendingPremiumService {
            npc: intent.npc_actor,
            service: topic,
            conversation_generation: generation,
        });
        Some(PremiumTalkStep::Prompt(line))
    }

    pub(crate) fn take_bank(&self, session: GameSessionId) -> Option<PendingBank> {
        self.sessions.lock().ok()?.get_mut(&session)?.bank.take()
    }

    pub(crate) fn prompt_bank(&self, session: GameSessionId, pending: PendingBank) -> bool {
        let Ok(mut sessions) = self.sessions.lock() else {
            return false;
        };
        let state = sessions.entry(session).or_default();
        state.premium = None;
        state.bank = Some(pending);
        true
    }

    /// Current NPC owner facts under the Channel runtime lock. A Bank marker comes solely
    /// from an admitted typed Service; a name or profession can never supply it.
    pub(crate) fn current_bank(
        &self,
        chat: &ChatRuntime,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        intent: &NpcTalkIntent,
    ) -> Option<(u64, u64, String)> {
        let (generation, npc_key) =
            Self::current_service_npc(chat, runtime, actor, session, intent)?;
        let book = chat.npc_talk()?;
        if !book.get(&npc_key)?.bank() {
            return None;
        }
        Some((book.catalogue_revision(), generation, npc_key))
    }

    /// The conversation generation and native key of the NPC `intent` names, when it is the
    /// visible NPC `session` is engaged with and within talk range of the visible speaker.
    fn current_service_npc(
        chat: &ChatRuntime,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        intent: &NpcTalkIntent,
    ) -> Option<(u64, String)> {
        let visible = runtime.visible_entities();
        let speaker = visible
            .players
            .iter()
            .find(|player| player.actor == actor)?;
        let npc = visible.npcs.iter().find(|npc| {
            npc.actor.placement_identity() == intent.npc_actor.identity
                && npc.generation == intent.npc_actor.generation
        })?;
        if chat.engaged(session) != Some(npc.actor)
            || speaker.position.floor != npc.position.floor
            || speaker.position.x.abs_diff(npc.position.x) > TALK_RANGE_TILES
            || speaker.position.y.abs_diff(npc.position.y) > TALK_RANGE_TILES
        {
            return None;
        }
        Some((
            chat.npc_conversation_generation(session)?,
            runtime.npc_key(npc.actor)?.to_owned(),
        ))
    }

    pub(crate) fn bank_reply(intent: &NpcTalkIntent, line: String) -> NpcTalkOutcome {
        NpcTalkOutcome {
            unknown: false,
            inventory_changed: false,
            disposition: NpcIntentDisposition::Ok,
            conversation: Some(NpcConversation {
                open: Some((intent.npc_actor, NpcConversationState::Open)),
                lines: vec![line],
            }),
            window: None,
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
                unknown: false,
                inventory_changed: false,
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
                    unknown: false,
                    inventory_changed: false,
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
                unknown: false,
                inventory_changed: false,
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
            unknown: false,
            inventory_changed: false,
            disposition: Disposition::Ok,
            conversation: Some(NpcConversation {
                open,
                lines: vec![reply.text],
            }),
            window: None,
        }
    }
}

/// Command 7 Bank integration: native retained command first, then the current NPC owner's
/// conversation, and finally the native Bank writer. No talk text is persisted.
pub(super) async fn npc_bank_talk(
    admission: &super::ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: super::connection::UseCommand,
    intent: NpcTalkIntent,
) -> NpcTalkOutcome {
    use super::bank_npc::{self, BankNpcIntent};
    use crate::durability::bank::{BankError, BankNpcSource};
    use crate::foundation::{CommandId, CommandRef, RuntimeScopeRefV1};
    let session = command.game_session_id;
    let yes = intent.text.trim_matches(' ').eq_ignore_ascii_case("yes");
    let unknown = || NpcTalkOutcome {
        unknown: true,
        ..NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
    };
    let rejected = || NpcTalkOutcome::only(NpcIntentDisposition::Rejected);
    let reply = |text| NpcConversations::bank_reply(&intent, text);
    let reference = CommandId::new(command.command_id)
        .ok()
        .map(|id| CommandRef::new(session, id));
    let mut runtime = admission.runtime.lock().await;
    let ordinary = admission.npc_conversations.talk_in(
        &admission.chat,
        &runtime,
        actor,
        session,
        &intent,
        admission.owner_now(),
        super::item_ref_admission::pinned_index(&runtime, admission.active_generation),
    );
    if ordinary.disposition != NpcIntentDisposition::Ok {
        admission.npc_conversations.cancel_bank(session);
        return ordinary;
    }
    if let Some(step) =
        admission
            .npc_conversations
            .premium_step(&admission.chat, &runtime, actor, session, &intent)
    {
        // The purchase re-reads the actor, NPC, range and Content itself under its revision
        // slot, so no runtime lock is held across it.
        drop(runtime);
        return match step {
            PremiumTalkStep::Prompt(line) => reply(line),
            PremiumTalkStep::Confirm { pending, catalogue } => {
                let Some(reference) = reference else {
                    return rejected();
                };
                match super::npc_promotion::confirm_premium_service(
                    admission,
                    actor,
                    reference,
                    pending,
                    catalogue.as_ref(),
                )
                .await
                {
                    PremiumTalkAnswer::Line {
                        text,
                        inventory_changed,
                    } => NpcTalkOutcome {
                        inventory_changed,
                        ..reply(text)
                    },
                    PremiumTalkAnswer::Unknown => unknown(),
                    PremiumTalkAnswer::Rejected => rejected(),
                }
            }
        };
    }
    let Some((catalogue_revision, conversation_generation, npc_key)) = admission
        .npc_conversations
        .current_bank(&admission.chat, &runtime, actor, session, &intent)
    else {
        admission.npc_conversations.cancel_bank(session);
        return ordinary;
    };
    let (Some(fence), Some(reference)) = (command.item_fence, reference) else {
        return rejected();
    };
    let RuntimeScopeRefV1::Channel { world_id, .. } = fence.runtime_scope else {
        return rejected();
    };
    let Ok(world) = crate::domain::WorldId::from_bytes(*world_id.as_bytes()) else {
        return rejected();
    };
    let context = BankDialogueContext {
        session,
        character: fence.character_id,
        world,
        npc: intent.npc_actor,
        conversation_revision: conversation_generation,
    };
    let pending = admission.npc_conversations.take_bank(session);
    if yes {
        let Some(pending) = pending else {
            return ordinary;
        };
        if pending.catalogue_revision != catalogue_revision
            || pending.fence != fence
            || pending.node_id != *admission.holder.fact().node_id().as_bytes()
            || pending.node_revision != admission.holder.fact().registration_revision()
        {
            return ordinary;
        }
        let Some(operation) = pending.confirmation.respond(
            Some(&context),
            &intent.text,
            admission.owner_now().get() / 1000,
        ) else {
            return ordinary;
        };
        let recipient_name = match &operation {
            BankNpcOperation::Transfer { name, .. } => Some(name.clone()),
            _ => None,
        };
        let Some(request) = bank_request(&operation, pending.coins, session, command.command_id)
        else {
            return rejected();
        };
        let source = BankNpcSource {
            command: reference,
            npc_identity: intent.npc_actor.identity,
            npc_key: npc_key.clone(),
            npc_generation: intent.npc_actor.generation,
            catalogue_revision,
            prompt_generation: conversation_generation,
            recipient_name,
        };
        // The Channel owner retains its runtime lock across native enqueue acceptance: the NPC,
        // range and Content facts above cannot change before the Bank owner accepts this work.
        // No std::sync conversation/session mutex is held across this await.
        match admission
            .root
            .enqueue_npc_bank(
                admission.character,
                admission.holder,
                fence,
                source,
                request,
            )
            .await
        {
            Ok(()) => {}
            Err(BankError::Unavailable(_)) => return unknown(),
            Err(_) => return rejected(),
        }
        drop(runtime);
        return match admission
            .root
            .resume_npc_bank(admission.character, admission.holder, fence, reference)
            .await
        {
            Ok(Some(completion)) => {
                completed_bank_talk(
                    admission,
                    actor,
                    session,
                    &intent,
                    &completion.request,
                    &completion.outcome,
                    completion.recipient_name,
                )
                .await
            }
            _ => unknown(),
        };
    }
    // Any other response cancels the one pending invocation. A new valid operation can open
    // the next prompt, but can never confirm the old one.
    let Ok(parsed) = bank_npc::parse(&intent.text) else {
        return ordinary;
    };
    // Only deposit and withdraw move item custody; balance and transfer never read backpack facts.
    let coins = if parsed.uses_coin_custody() {
        bank_coin_definitions(admission, &runtime)
    } else {
        None
    };
    drop(runtime);
    let coins = if let Some((gold, platinum, crystal)) = coins {
        let Some(native) = admission
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay)
        else {
            return rejected();
        };
        match admission
            .root
            .read_character_backpack(admission.character, fence.character_id)
            .await
        {
            Ok(Some(current)) => match super::item_ref_admission::generation_item_facts(
                native,
                &current.backpack.definition,
            ) {
                Ok(backpack) => Some(BankCoinFacts {
                    gold,
                    platinum,
                    crystal,
                    backpack,
                }),
                Err(_) => return rejected(),
            },
            Ok(None) => None,
            Err(crate::durability::item_transfer::ItemTransferError::Unavailable(_)) => {
                return unknown();
            }
            Err(_) => return rejected(),
        }
    } else {
        None
    };
    let recipient = match &parsed {
        BankNpcIntent::Transfer { name, .. } => Some(name.clone()),
        _ => None,
    };
    let preview = match admission
        .root
        .preview_npc_bank(
            admission.character,
            admission.holder,
            fence,
            coins.clone(),
            recipient,
        )
        .await
    {
        Ok(preview) => preview,
        Err(BankError::Unavailable(_)) => return unknown(),
        Err(_) => return rejected(),
    };
    runtime = admission.runtime.lock().await;
    if admission
        .npc_conversations
        .current_bank(&admission.chat, &runtime, actor, session, &intent)
        != Some((catalogue_revision, conversation_generation, npc_key.clone()))
    {
        return rejected();
    }
    if matches!(parsed, BankNpcIntent::Balance) {
        return reply(format!("Your account balance is {} gold.", preview.balance));
    }
    if preview.junior {
        return reply("You can use the bank once you have left the island.".into());
    }
    if let BankNpcIntent::Transfer { amount, .. } = &parsed
        && *amount > preview.balance
    {
        return reply("There is not enough gold on your account.".into());
    }
    if let Some(refusal) = preview.recipient_refusal {
        return reply(bank_npc::refusal_reply(refusal).into());
    }
    let operation = match parsed {
        BankNpcIntent::Balance => return rejected(),
        BankNpcIntent::Deposit(amount) => BankNpcOperation::Deposit(amount),
        BankNpcIntent::DepositAll if preview.eligible_coins > 0 => {
            BankNpcOperation::Deposit(preview.eligible_coins)
        }
        BankNpcIntent::DepositAll => {
            return reply("You do not have enough gold in your backpack.".into());
        }
        BankNpcIntent::Withdraw(amount) => BankNpcOperation::Withdraw(amount),
        BankNpcIntent::Transfer { amount, .. } => {
            let Some((recipient, name)) = preview.recipient else {
                return reply("This player does not exist.".into());
            };
            BankNpcOperation::Transfer {
                amount,
                recipient,
                name,
            }
        }
    };
    if !matches!(operation, BankNpcOperation::Transfer { .. }) && coins.is_none() {
        return reply("There is not enough room in your backpack.".into());
    }
    let line = operation.prompt();
    let Ok(confirmation) =
        BankConfirmation::new(operation, context, admission.owner_now().get() / 1000)
    else {
        return rejected();
    };
    if !admission.npc_conversations.prompt_bank(
        session,
        PendingBank {
            confirmation,
            fence,
            node_id: *admission.holder.fact().node_id().as_bytes(),
            node_revision: admission.holder.fact().registration_revision(),
            catalogue_revision,
            coins,
        },
    ) {
        return rejected();
    }
    reply(line)
}

pub(super) async fn resume_bank_talk(
    admission: &super::ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: super::connection::UseCommand,
) -> Option<NpcTalkOutcome> {
    let (fence, reference) = match retained_bank_key(&command) {
        Ok(Some(key)) => key,
        Ok(None) => return None,
        Err(BankLookupHeld) => return Some(held_bank_outcome()),
    };
    match admission
        .root
        .resume_npc_bank(admission.character, admission.holder, fence, reference)
        .await
    {
        Ok(None) => None,
        Ok(Some(completion)) => {
            let intent = NpcTalkIntent {
                npc_actor: EntityRef {
                    identity: completion.npc_identity,
                    generation: completion.npc_generation,
                },
                text: String::new(),
            };
            Some(
                completed_bank_talk(
                    admission,
                    actor,
                    command.game_session_id,
                    &intent,
                    &completion.request,
                    &completion.outcome,
                    completion.recipient_name,
                )
                .await,
            )
        }
        // An accepted original pending command cannot be abandoned or sequenced as a new
        // command because current ownership is temporarily unavailable or storage is unknown.
        Err(_) => Some(held_bank_outcome()),
    }
}

/// PREM-5: the first command of a resumed connection that repeats an unsequenced CommandId.
/// The reconnect cancelled the pending Premium offer, so a resent `yes` cannot reach the
/// purchase replay; when a promotion or blessing purchase committed under this CommandRef
/// before the lost COMMIT outcome, its answer is the receipt's, with nothing bought again.
/// `None`: no Premium purchase committed under it; dispatch goes on.
pub(super) async fn resume_premium_talk(
    admission: &super::ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: &super::connection::UseCommand,
) -> Option<NpcTalkOutcome> {
    use super::npc_promotion::PremiumReconcile;
    if !admission.npc_conversations.premium_services_installed() {
        return None;
    }
    let id = crate::foundation::CommandId::new(command.command_id).ok()?;
    let reference = crate::foundation::CommandRef::new(command.game_session_id, id);
    match admission.reconcile_premium_purchase(actor, reference).await {
        PremiumReconcile::None => None,
        PremiumReconcile::Committed { npc, line } => {
            let intent = NpcTalkIntent {
                npc_actor: npc,
                text: String::new(),
            };
            Some(NpcTalkOutcome {
                inventory_changed: true,
                ..NpcConversations::bank_reply(&intent, line.to_owned())
            })
        }
        PremiumReconcile::Held => Some(held_bank_outcome()),
    }
}

/// The unknown outcome of an accepted original Bank command that cannot be classified yet: it
/// is held, never sequenced as a refusal or as a new command.
fn held_bank_outcome() -> NpcTalkOutcome {
    NpcTalkOutcome {
        unknown: true,
        ..NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
    }
}

/// A retained Bank command that cannot be classified with current authority; its outcome is
/// [`held_bank_outcome`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BankLookupHeld;

/// The current fence and original `CommandRef` under which a retained Bank command of
/// `command` is looked up. `Ok(None)`: the CommandId is invalid, so no Bank work can have been
/// retained for it. `Err`: the session has no current item fence (a resume whose post-commit
/// session read failed), so retained work cannot be classified with current authority and
/// the command stays unsequenced.
fn retained_bank_key(
    command: &super::connection::UseCommand,
) -> Result<
    Option<(
        crate::durability::item_transfer::CurrentCharacterItemFence,
        crate::foundation::CommandRef,
    )>,
    BankLookupHeld,
> {
    let Ok(id) = crate::foundation::CommandId::new(command.command_id) else {
        return Ok(None);
    };
    let Some(fence) = command.item_fence else {
        return Err(BankLookupHeld);
    };
    Ok(Some((
        fence,
        crate::foundation::CommandRef::new(command.game_session_id, id),
    )))
}

fn bank_coin_definitions(
    admission: &super::ComposedFreshAdmission<'_, '_, '_>,
    runtime: &ChannelRuntimeV1,
) -> Option<(
    crate::durability::item_mint::TypedDefinitionRef,
    crate::durability::item_mint::TypedDefinitionRef,
    crate::durability::item_mint::TypedDefinitionRef,
)> {
    use crate::domain::currency::{COIN_DEFINITION_FAMILY, Coin};
    let index = super::item_ref_admission::pinned_index(runtime, admission.active_generation)?;
    let coin = |coin: Coin| {
        Some(crate::durability::item_mint::TypedDefinitionRef {
            family: COIN_DEFINITION_FAMILY.into(),
            production_key: coin.production_key().into(),
            revision_ref: index.revision(coin.production_key())?.into(),
        })
    };
    Some((
        coin(Coin::Gold)?,
        coin(Coin::Platinum)?,
        coin(Coin::Crystal)?,
    ))
}

fn bank_request(
    operation: &BankNpcOperation,
    coins: Option<BankCoinFacts>,
    session: GameSessionId,
    command: u64,
) -> Option<crate::durability::bank::BankRequest> {
    use crate::durability::bank::*;
    let id = |tag: &[u8]| super::npc_trade_command::derived(tag, session, command);
    let identity = BankOperationIdentity {
        occurrence: BankOperationOccurrence::from_bytes(id(b"oteryn:npc-bank-occurrence:v1"))
            .ok()?,
        transaction_id: id(b"oteryn:npc-bank-transaction:v1"),
        event_id: id(b"oteryn:npc-bank-event:v1"),
        occurred_at_unix_ms: super::npc_trade_command::unix_ms_now(),
        server_build_id: env!("CARGO_PKG_VERSION").into(),
    };
    let request = match operation {
        BankNpcOperation::Deposit(amount) => BankRequest::Deposit(BankDepositRequest {
            identity,
            amount: *amount,
            coins: coins?,
            change_item_instance_ids: [
                id(b"oteryn:npc-bank-output-2:v1"),
                id(b"oteryn:npc-bank-output-3:v1"),
            ],
        }),
        BankNpcOperation::Withdraw(amount) => BankRequest::Withdraw(BankWithdrawRequest {
            identity,
            amount: *amount,
            coins: coins?,
            output_item_instance_ids: [
                id(b"oteryn:npc-bank-output-1:v1"),
                id(b"oteryn:npc-bank-output-2:v1"),
                id(b"oteryn:npc-bank-output-3:v1"),
            ],
        }),
        BankNpcOperation::Transfer {
            amount,
            recipient,
            name,
        } => BankRequest::Transfer(BankTransferRequest {
            identity,
            amount: *amount,
            recipient_character_id: *recipient,
            recipient_name_key: name.comparison_key(),
        }),
    };
    let matches = match &request {
        BankRequest::Deposit(request) => operation.matches_deposit(request),
        BankRequest::Withdraw(request) => operation.matches_withdraw(request),
        BankRequest::Transfer(request) => operation.matches_transfer(request),
    };
    matches.then_some(request)
}

async fn completed_bank_talk(
    admission: &super::ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    intent: &NpcTalkIntent,
    request: &crate::durability::bank::BankRequest,
    outcome: &crate::durability::bank::BankOperationOutcome,
    name: Option<crate::domain::character_name::CharacterName>,
) -> NpcTalkOutcome {
    let runtime = admission.runtime.lock().await;
    let mut reply =
        NpcConversations::bank_reply(intent, bank_completion_reply(request, outcome, name));
    let record = match outcome {
        crate::durability::bank::BankOperationOutcome::Committed(record)
        | crate::durability::bank::BankOperationOutcome::Replayed(record) => record,
    };
    reply.inventory_changed = !record.coin_lines.is_empty();
    // A historical receipt does not reopen a conversation or restore a missing NPC actor.
    if admission
        .npc_conversations
        .current_bank(&admission.chat, &runtime, actor, session, intent)
        .is_none()
        && let Some(conversation) = reply.conversation.as_mut()
    {
        conversation.open = None;
    }
    reply
}

fn bank_completion_reply(
    request: &crate::durability::bank::BankRequest,
    outcome: &crate::durability::bank::BankOperationOutcome,
    name: Option<crate::domain::character_name::CharacterName>,
) -> String {
    use crate::durability::bank::{BankOperationOutcome, BankRequest};
    let record = match outcome {
        BankOperationOutcome::Committed(record) | BankOperationOutcome::Replayed(record) => record,
    };
    let operation = match request {
        BankRequest::Deposit(r) => BankNpcOperation::Deposit(r.amount),
        BankRequest::Withdraw(r) => BankNpcOperation::Withdraw(r.amount),
        BankRequest::Transfer(r) => match name {
            Some(name) => BankNpcOperation::Transfer {
                amount: r.amount,
                recipient: r.recipient_character_id,
                name,
            },
            None => return "The bank operation could not be resolved.".into(),
        },
    };
    operation.reply(record.result)
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

#[cfg(test)]
#[allow(clippy::expect_used)]
mod bank_context_tests {
    use super::*;
    use crate::foundation::{MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements};
    use std::sync::Arc;
    #[test]
    fn a_retained_bank_lookup_without_a_current_fence_is_held_unsequenced() {
        use crate::durability::item_transfer::CurrentCharacterItemFence;
        use crate::foundation::{
            ChannelId, CommandId, CommandRef, ConnectionGeneration, RuntimeScopeRefV1,
            ScopeOwnershipGeneration, WorldId,
        };
        let uuid = |tag: u8| {
            let mut bytes = [tag; 16];
            bytes[6] = 0x70 | (tag & 0x0f);
            bytes[8] = 0x80 | (tag & 0x3f);
            bytes
        };
        let session = GameSessionId::decode(&uuid(0x21)).expect("session UUIDv7");
        // A reconnect whose post-commit session read failed leaves no item fence: the resent
        // original command cannot be classified, so it is held, never treated as complete.
        let unfenced = super::super::connection::UseCommand {
            game_session_id: session,
            command_id: 7,
            item_fence: None,
        };
        assert_eq!(
            retained_bank_key(&unfenced).expect_err("held without a current fence"),
            super::BankLookupHeld
        );
        let held = super::held_bank_outcome();
        assert!(held.unknown);
        assert!(!held.inventory_changed);
        assert!(held.conversation.is_none() && held.window.is_none());
        // CommandId 0 is never sequenced, so no Bank work can be retained for it.
        assert!(matches!(
            retained_bank_key(&super::super::connection::UseCommand {
                command_id: 0,
                ..unfenced
            }),
            Ok(None)
        ));
        let world = WorldId::decode(&uuid(0x22)).expect("World UUIDv7");
        let channel = ChannelId::decode(&uuid(0x23)).expect("Channel UUIDv7");
        let fence = CurrentCharacterItemFence {
            character_id: crate::domain::CharacterId::from_bytes(uuid(0x24))
                .expect("Character UUIDv7"),
            game_session_id: session,
            connection_generation: ConnectionGeneration::new(2).expect("generation"),
            character_lease_generation: 3,
            runtime_scope: RuntimeScopeRefV1::channel(world, channel),
            scope_ownership_generation: ScopeOwnershipGeneration::new(4).expect("generation"),
        };
        let (current, reference) = retained_bank_key(&super::super::connection::UseCommand {
            item_fence: Some(fence),
            ..unfenced
        })
        .ok()
        .flatten()
        .expect("lookup key under the current fence");
        assert_eq!(current, fence);
        assert_eq!(
            reference,
            CommandRef::new(session, CommandId::new(7).expect("CommandId"))
        );
    }

    #[test]
    fn actual_chat_reengagement_and_npc_generation_retire_unsent_confirmation() {
        let (mut runtime, actor, session) =
            crate::gameplay_transport::actor_spell::tests::runtime_with_capacity(0x71, 3);
        runtime
            .initialize_first_entry_position(actor)
            .expect("position");
        let position = runtime
            .read_actor_position(actor)
            .expect("position")
            .position();
        runtime
            .place_npcs(
                &NpcPlacementTable {
                    npcs: vec![NpcPlacements {
                        key: "oteryn:npc.paulie".into(),
                        placements: vec![NpcPlacement {
                            floor: -i8::try_from(position.floor).expect("floor"),
                            y: u16::try_from(position.y).expect("y"),
                            x: u16::try_from(position.x + 1).expect("x"),
                            direction: MovementFacing::South,
                        }],
                    }],
                },
                |_| true,
                true,
            )
            .expect("native NPC placement");
        let npc = runtime.visible_entities().npcs[0];
        let reference = EntityRef {
            identity: npc.actor.placement_identity(),
            generation: npc.generation,
        };
        let chat = ChatRuntime::default();
        chat.admit(session, "Fixture Hero", SemanticTimeMicros::from_micros(1));
        chat.set_npc_service(session);
        // A test catalogue fixture is data; no DB/current-money authority is supplied here.
        chat.install_npc_talk(Arc::new(
            super::super::npc_talk::NpcTalkBook::default().with(
                "oteryn:npc.paulie",
                super::super::npc_talk::NpcTalkEntry::new("Paulie", None).with_bank(),
            ),
        ));
        let conversations = NpcConversations::default();
        let greeting = NpcTalkIntent {
            npc_actor: reference,
            text: "hi".into(),
        };
        assert_eq!(
            conversations
                .talk_in(
                    &chat,
                    &runtime,
                    actor,
                    session,
                    &greeting,
                    SemanticTimeMicros::from_micros(10),
                    None
                )
                .disposition,
            NpcIntentDisposition::Ok
        );
        let first = conversations
            .current_bank(&chat, &runtime, actor, session, &greeting)
            .expect("actual engaged Bank");
        let mut character = [0; 16];
        character[6] = 0x70;
        character[8] = 0x80;
        character[15] = 1;
        let context = BankDialogueContext {
            session,
            character: crate::domain::CharacterId::from_bytes(character).expect("Character UUIDv7"),
            world: crate::domain::WorldId::from_bytes(*actor.world_id().as_bytes())
                .expect("World UUIDv7"),
            npc: reference,
            conversation_revision: first.1,
        };
        let confirmation =
            BankConfirmation::new(BankNpcOperation::Deposit(10), context, 1).expect("prompt");
        assert!(chat.end_engagement(session, npc.actor));
        assert!(
            conversations
                .current_bank(&chat, &runtime, actor, session, &greeting)
                .is_none()
        );
        conversations.talk_in(
            &chat,
            &runtime,
            actor,
            session,
            &greeting,
            SemanticTimeMicros::from_micros(20),
            None,
        );
        let second = conversations
            .current_bank(&chat, &runtime, actor, session, &greeting)
            .expect("reopened");
        assert_ne!(first.1, second.1);
        let current = BankDialogueContext {
            conversation_revision: second.1,
            ..context
        };
        assert!(confirmation.respond(Some(&current), "yes", 2).is_none());
        let mut stale = greeting;
        stale.npc_actor.generation += 1;
        assert!(
            conversations
                .current_bank(&chat, &runtime, actor, session, &stale)
                .is_none()
        );
    }
}
