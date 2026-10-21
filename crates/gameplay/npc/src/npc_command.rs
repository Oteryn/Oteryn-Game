//! NPC conversation payloads and the Channel-local talk ingress window.

use crate::content::item_ref::{ITEM_FAMILY, ItemDefinitionIndex};
use crate::content::{ProjectV2ServiceOffer, ProjectV2ServiceOfferDirection};
use oteryn_protocol_oteryn::npc_service::{MAX_OFFERS, NpcOffer, NpcTradeSide};

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

/// The offers of `offers` the window shows, in order: an Item offer whose definition the pinned
/// index names at the offer's own revision and whose count and price fit the wire. The index in
/// the result is the offer index a trade intent names, so `NpcConversations::trade_in` reads this
/// same projection. At most [`MAX_OFFERS`].
pub fn project_offers(
    offers: &[ProjectV2ServiceOffer],
    items: &ItemDefinitionIndex,
) -> Vec<NpcOffer> {
    projected(offers, items).map(|(offer, _)| offer).collect()
}

/// Each projected offer with the catalogue offer it shows.
pub fn projected<'a>(
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
pub struct NpcTradeOffer {
    pub npc_key: String,
    pub catalogue_revision: u64,
    pub offer_index: u32,
    pub side: NpcTradeSide,
    pub quantity: u32,
    /// Units per trade row.
    pub count: u32,
    pub unit_price: u32,
    /// The offered Item's definition key and revision.
    pub item_key: String,
    pub item_revision: String,
}

/// A trade intent that is not committed: its disposition and the domain 7 and 8 payloads to send.
pub struct NpcTradeRefusal {
    pub disposition: NpcIntentDisposition,
    pub update: NpcUpdate,
}

impl NpcTradeRefusal {
    pub fn only(disposition: NpcIntentDisposition) -> Self {
        Self {
            disposition,
            update: NpcUpdate::default(),
        }
    }
}
