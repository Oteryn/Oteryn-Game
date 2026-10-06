//! Immutable NPC service model over the data-only catalogue (NPC-CONTENT-1; NPC-0 section 3.4,
//! TIMED-ITEM-0 section 6.2, QUEST-GATE-0 route amendment). Pure data: no wire, no runtime
//! state, no persistence. Every offer is admitted or held with a closed reason; every route
//! loads. The model is a function of the catalogue and the item facts, built once, iterated in
//! ascending key order.
use super::{NpcDataCatalogue, replies};
use crate::content::{
    ProjectV2Declaration, ProjectV2DefinitionRef, ProjectV2Family, ProjectV2ServiceOffer,
    ProjectV2ServiceOfferDirection, ProjectV2TravelRoute, ReferenceDefinitionKind,
    ReferenceItemField, ReferenceItemStackClass, ReferencePlayableContentSource,
};
use std::collections::{BTreeMap, BTreeSet};

/// Most gold three coin stacks can hold (NPC-0 section 3.4).
pub const NPC_SELL_PRICE_COIN_CAPACITY: u64 = 1_009_999;
/// Largest purchase unit of a stackable item.
pub const NPC_STACK_COUNT_MAX: u32 = 100;
/// TIMEDITEM0-RL-01: most charges (or sub-type) of one item.
pub const NPC_CHARGES_MAX: u32 = 65_535;

/// The item facts the model needs, one method per fact, so the model depends on no item
/// storage. `key` is the Item definition key.
pub trait NpcItemFacts {
    fn known(&self, key: &str) -> bool;
    fn stackable(&self, key: &str) -> bool;
    fn charged(&self, key: &str) -> bool;
    fn fluid(&self, key: &str) -> bool;
    /// Charges of an admitted timed definition; `None` when the item is not timed.
    fn timed_charges(&self, key: &str) -> Option<u32>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ItemFactRow {
    stackable: bool,
    charged: bool,
    fluid: bool,
    timed_charges: Option<u32>,
}

/// Production item facts, read once from the Reference definitions of a World Project.
#[derive(Debug, Clone, Default)]
pub struct ReferenceNpcItemFacts {
    items: BTreeMap<String, ItemFactRow>,
}

impl ReferenceNpcItemFacts {
    pub fn from_reference_source(source: &ReferencePlayableContentSource) -> Self {
        let mut items = BTreeMap::new();
        for definition in &source.definitions {
            let ReferenceDefinitionKind::Item(item) = &definition.kind else {
                continue;
            };
            let semantics = &item.semantics;
            let stackable = matches!(item.stack_class, ReferenceItemStackClass::StackCapable)
                || matches!(
                    &semantics.stack,
                    ReferenceItemField::Known(stack)
                        if matches!(stack.stackable, ReferenceItemField::Known(true))
                );
            let charges = match &semantics.charges {
                ReferenceItemField::Known(charges) => match charges.count {
                    ReferenceItemField::Known(count) => Some(count),
                    _ => None,
                },
                _ => None,
            };
            let fluid = matches!(
                &semantics.fluid,
                ReferenceItemField::Known(fluid)
                    if matches!(fluid.fluid_type, ReferenceItemField::Known(_))
            );
            let timed = matches!(
                &semantics.temporal,
                ReferenceItemField::Known(temporal)
                    if matches!(temporal.duration, ReferenceItemField::Known(_))
            );
            items.insert(
                definition.definition.key().as_str().to_owned(),
                ItemFactRow {
                    stackable,
                    charged: charges.is_some(),
                    fluid,
                    timed_charges: if timed { charges } else { None },
                },
            );
        }
        Self { items }
    }
}

impl NpcItemFacts for ReferenceNpcItemFacts {
    fn known(&self, key: &str) -> bool {
        self.items.contains_key(key)
    }
    fn stackable(&self, key: &str) -> bool {
        self.items.get(key).is_some_and(|row| row.stackable)
    }
    fn charged(&self, key: &str) -> bool {
        self.items.get(key).is_some_and(|row| row.charged)
    }
    fn fluid(&self, key: &str) -> bool {
        self.items.get(key).is_some_and(|row| row.fluid)
    }
    fn timed_charges(&self, key: &str) -> Option<u32> {
        self.items.get(key).and_then(|row| row.timed_charges)
    }
}

/// Why an offer is held (NPC-0 section 3.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NpcOfferHeldReason {
    NonGoldCurrency,
    UnknownItem,
    CountOutOfRange,
    TimedCountMismatch,
    SellPriceAboveCoinCapacity,
    ArbitragePaying,
    ParityPending,
}

impl NpcOfferHeldReason {
    pub const ALL: [Self; 7] = [
        Self::NonGoldCurrency,
        Self::UnknownItem,
        Self::CountOutOfRange,
        Self::TimedCountMismatch,
        Self::SellPriceAboveCoinCapacity,
        Self::ArbitragePaying,
        Self::ParityPending,
    ];
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NonGoldCurrency => "NonGoldCurrency",
            Self::UnknownItem => "UnknownItem",
            Self::CountOutOfRange => "CountOutOfRange",
            Self::TimedCountMismatch => "TimedCountMismatch",
            Self::SellPriceAboveCoinCapacity => "SellPriceAboveCoinCapacity",
            Self::ArbitragePaying => "ArbitragePaying",
            Self::ParityPending => "ParityPending",
        }
    }
}

/// Why a whole NPC is held: a reference that does not resolve in the catalogue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NpcHeldReason {
    UnresolvedDialogue,
    UnresolvedService,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcHeldOffer {
    pub offer: ProjectV2ServiceOffer,
    pub reason: NpcOfferHeldReason,
}

/// One Service: its admitted offers and routes and its held offers, all in source order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NpcServiceEntry {
    pub offers: Vec<ProjectV2ServiceOffer>,
    pub held_offers: Vec<NpcHeldOffer>,
    /// Every route loads (the source has no gate field); the destination stays in the source
    /// frame.
    pub routes: Vec<ProjectV2TravelRoute>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcEntry {
    pub name: String,
    /// An admitted Dialogue replaces the generated replies.
    pub dialogue: Option<ProjectV2DefinitionRef>,
    pub services: Vec<ProjectV2DefinitionRef>,
    /// Present exactly when `dialogue` is absent.
    pub generated_replies: Option<replies::NpcGeneratedReplies>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcServiceModel {
    project_revision: String,
    source_tree_digest: String,
    npcs: BTreeMap<ProjectV2DefinitionRef, NpcEntry>,
    held_npcs: BTreeMap<ProjectV2DefinitionRef, NpcHeldReason>,
    services: BTreeMap<ProjectV2DefinitionRef, NpcServiceEntry>,
}

impl NpcServiceModel {
    pub fn project_revision(&self) -> &str {
        &self.project_revision
    }
    pub fn source_tree_digest(&self) -> &str {
        &self.source_tree_digest
    }
    pub fn npcs(&self) -> &BTreeMap<ProjectV2DefinitionRef, NpcEntry> {
        &self.npcs
    }
    pub fn held_npcs(&self) -> &BTreeMap<ProjectV2DefinitionRef, NpcHeldReason> {
        &self.held_npcs
    }
    pub fn services(&self) -> &BTreeMap<ProjectV2DefinitionRef, NpcServiceEntry> {
        &self.services
    }
    pub fn admitted_offer_count(&self) -> usize {
        self.services.values().map(|s| s.offers.len()).sum()
    }
    pub fn admitted_route_count(&self) -> usize {
        self.services.values().map(|s| s.routes.len()).sum()
    }
    /// Held offers per reason, every reason present.
    pub fn held_offer_counts(&self) -> BTreeMap<&'static str, usize> {
        let mut counts: BTreeMap<&'static str, usize> = NpcOfferHeldReason::ALL
            .iter()
            .map(|reason| (reason.as_str(), 0))
            .collect();
        for held in self.services.values().flat_map(|s| &s.held_offers) {
            *counts.entry(held.reason.as_str()).or_default() += 1;
        }
        counts
    }
    pub fn generated_reply_npc_count(&self) -> usize {
        self.npcs
            .values()
            .filter(|npc| npc.generated_replies.is_some())
            .count()
    }
}

impl NpcDataCatalogue {
    /// Builds the immutable service model from this catalogue and an item-facts source.
    pub fn service_model(&self, facts: &dyn NpcItemFacts) -> NpcServiceModel {
        let records: Vec<_> = self.records().collect();
        build(
            &records,
            self.project_revision(),
            self.source_tree_digest(),
            facts,
        )
    }

    /// Production entry point: item facts from the Reference definitions of a World Project
    /// (`WorldProject::lower_reference_source`).
    pub fn service_model_for_reference_source(
        &self,
        source: &ReferencePlayableContentSource,
    ) -> NpcServiceModel {
        self.service_model(&ReferenceNpcItemFacts::from_reference_source(source))
    }
}

/// Units per purchase unit: a missing `count` is one.
fn count_of(offer: &ProjectV2ServiceOffer) -> u32 {
    offer.count.unwrap_or(1)
}

/// The per-offer rules of NPC-0 section 3.4, in order, before the cross-offer arbitrage rule.
fn classify(offer: &ProjectV2ServiceOffer, facts: &dyn NpcItemFacts) -> Option<NpcOfferHeldReason> {
    if offer.currency.is_some() {
        return Some(NpcOfferHeldReason::NonGoldCurrency);
    }
    let key = offer.item.key.as_str();
    if offer.item.family != ProjectV2Family::Item || !facts.known(key) {
        return Some(NpcOfferHeldReason::UnknownItem);
    }
    let count = count_of(offer);
    if let Some(charges) = facts.timed_charges(key) {
        if count != charges {
            return Some(NpcOfferHeldReason::TimedCountMismatch);
        }
    } else if facts.charged(key) || facts.fluid(key) {
        if !(1..=NPC_CHARGES_MAX).contains(&count) {
            return Some(NpcOfferHeldReason::CountOutOfRange);
        }
    } else if facts.stackable(key) {
        if !(1..=NPC_STACK_COUNT_MAX).contains(&count) {
            return Some(NpcOfferHeldReason::CountOutOfRange);
        }
    } else if count != 1 {
        return Some(NpcOfferHeldReason::CountOutOfRange);
    }
    if offer.direction == ProjectV2ServiceOfferDirection::SellToPlayer
        && offer.unit_price > NPC_SELL_PRICE_COIN_CAPACITY
    {
        return Some(NpcOfferHeldReason::SellPriceAboveCoinCapacity);
    }
    if offer.parity_pending {
        return Some(NpcOfferHeldReason::ParityPending);
    }
    None
}

/// `left` pays strictly more per unit than `right`, as an exact rational.
fn pays_more(left: &ProjectV2ServiceOffer, right: (u64, u32)) -> bool {
    u128::from(left.unit_price) * u128::from(right.1)
        > u128::from(right.0) * u128::from(count_of(left))
}

type ClassifiedService<'a> = (
    &'a ProjectV2DefinitionRef,
    Vec<(&'a ProjectV2ServiceOffer, Option<NpcOfferHeldReason>)>,
);

pub(super) type Record<'a> = (&'a ProjectV2DefinitionRef, &'a ProjectV2Declaration);

pub(super) fn build(
    records: &[Record<'_>],
    project_revision: &str,
    source_tree_digest: &str,
    facts: &dyn NpcItemFacts,
) -> NpcServiceModel {
    // Pass 1: per-offer rules. Pass 2: arbitrage over the offers still admitted.
    let mut services: BTreeMap<ProjectV2DefinitionRef, NpcServiceEntry> = BTreeMap::new();
    let mut lowest_sell: BTreeMap<(&ProjectV2DefinitionRef, Option<u16>), (u64, u32)> =
        BTreeMap::new();
    let mut classified: Vec<ClassifiedService<'_>> = Vec::new();
    for &(reference, declaration) in records {
        let ProjectV2Declaration::Service { offers, routes, .. } = declaration else {
            continue;
        };
        services.insert(
            reference.clone(),
            NpcServiceEntry {
                routes: routes.clone(),
                ..NpcServiceEntry::default()
            },
        );
        let rows: Vec<_> = offers
            .iter()
            .map(|offer| (offer, classify(offer, facts)))
            .collect();
        for (offer, held) in &rows {
            if held.is_none() && offer.direction == ProjectV2ServiceOfferDirection::SellToPlayer {
                let unit = (offer.unit_price, count_of(offer));
                lowest_sell
                    .entry((&offer.item, offer.sub_type))
                    .and_modify(|best| {
                        // unit.0 / unit.1 < best.0 / best.1
                        if u128::from(unit.0) * u128::from(best.1)
                            < u128::from(best.0) * u128::from(unit.1)
                        {
                            *best = unit;
                        }
                    })
                    .or_insert(unit);
            }
        }
        classified.push((reference, rows));
    }
    for (reference, rows) in classified {
        let Some(entry) = services.get_mut(reference) else {
            continue;
        };
        for (offer, held) in rows {
            let held = held.or_else(|| {
                (offer.direction == ProjectV2ServiceOfferDirection::BuyFromPlayer
                    && lowest_sell
                        .get(&(&offer.item, offer.sub_type))
                        .is_some_and(|lowest| pays_more(offer, *lowest)))
                .then_some(NpcOfferHeldReason::ArbitragePaying)
            });
            match held {
                None => entry.offers.push(offer.clone()),
                Some(reason) => entry.held_offers.push(NpcHeldOffer {
                    offer: offer.clone(),
                    reason,
                }),
            }
        }
    }

    let dialogues: BTreeSet<&ProjectV2DefinitionRef> = records
        .iter()
        .filter(|(_, d)| matches!(d, ProjectV2Declaration::Dialogue { .. }))
        .map(|(r, _)| *r)
        .collect();
    let mut npcs = BTreeMap::new();
    let mut held_npcs = BTreeMap::new();
    for &(reference, declaration) in records {
        let ProjectV2Declaration::Npc {
            dialogue,
            services: service_refs,
            ..
        } = declaration
        else {
            continue;
        };
        if dialogue.as_ref().is_some_and(|d| !dialogues.contains(d)) {
            held_npcs.insert(reference.clone(), NpcHeldReason::UnresolvedDialogue);
            continue;
        }
        if service_refs.iter().any(|s| !services.contains_key(s)) {
            held_npcs.insert(reference.clone(), NpcHeldReason::UnresolvedService);
            continue;
        }
        let name = replies::npc_display_name(&reference.key);
        let generated_replies = dialogue.is_none().then(|| {
            replies::generate(
                &name,
                service_refs
                    .iter()
                    .filter_map(|s| services.get(s).map(|entry| (s, entry))),
            )
        });
        npcs.insert(
            reference.clone(),
            NpcEntry {
                name,
                dialogue: dialogue.clone(),
                services: service_refs.clone(),
                generated_replies,
            },
        );
    }

    NpcServiceModel {
        project_revision: project_revision.to_owned(),
        source_tree_digest: source_tree_digest.to_owned(),
        npcs,
        held_npcs,
        services,
    }
}
