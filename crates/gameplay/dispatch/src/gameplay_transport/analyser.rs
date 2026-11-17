//! ANALYSER-EMIT-1: bounded advisory facts, derived only from committed outcomes.
//! Rules never read these facts. Only selected, exact actor/session bindings have queues.
use std::collections::{HashMap, HashSet};
use std::num::{NonZeroU16, NonZeroU32};
use std::sync::{Arc, Mutex};

use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId};
use oteryn_protocol_oteryn::analyser::{
    AnalyserFact, AnalyserImpact, AnalyserItemCount, AnalyserPendingQueue,
};
use oteryn_protocol_oteryn::damage_element::DamageElement;

#[path = "analyser/production.rs"]
mod production;
#[cfg(test)]
#[path = "analyser/tests.rs"]
mod tests;

/// Advisory provenance of one hit the own character took. The attacker reaches only a
/// session that selected capability 486; rules never read it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::gameplay_transport) struct HitSource {
    pub(in crate::gameplay_transport) source:
        oteryn_protocol_oteryn::analyser::AnalyserDamageSource,
    pub(in crate::gameplay_transport) attacker: Option<ExactActorRef>,
}

impl From<oteryn_protocol_oteryn::analyser::AnalyserDamageSource> for HitSource {
    fn from(source: oteryn_protocol_oteryn::analyser::AnalyserDamageSource) -> Self {
        Self {
            source,
            attacker: None,
        }
    }
}

/// The client-visible spatial reference of an exact actor (`visible_entities` generation).
fn entity_ref(actor: ExactActorRef) -> oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
    oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
        identity: actor.placement_identity(),
        generation: actor.actor_local_generation(),
    }
}

#[derive(Debug)]
struct Stream {
    actor: ExactActorRef,
    generation: u64,
    /// Capability 486 selected: damage input may carry the attacker.
    attributed: bool,
    pending: AnalyserPendingQueue,
    totals: super::analyser_party::Totals,
    started: std::time::Instant,
    items: Option<Arc<crate::content::item_ref::ItemDefinitionIndex>>,
}

/// Activity runtimes share this leaf hub with their originating Channel. Terminal retirement
/// removes entries. No leaf guard is held while acquiring another owner or awaiting IO.
#[derive(Debug, Default)]
pub struct AnalyserStreams(Mutex<HashMap<GameSessionId, Stream>>);

impl AnalyserStreams {
    #[cfg(test)]
    pub(super) fn snapshot(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        generation: u64,
        items: Option<Arc<crate::content::item_ref::ItemDefinitionIndex>>,
    ) -> Option<bool> {
        self.snapshot_attributed(actor, session, generation, items, false)
    }

    pub(super) fn snapshot_attributed(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        generation: u64,
        items: Option<Arc<crate::content::item_ref::ItemDefinitionIndex>>,
        attributed: bool,
    ) -> Option<bool> {
        let mut streams = self.0.lock().ok()?;
        if streams
            .get(&session)
            .is_some_and(|held| held.generation > generation)
        {
            return None;
        }
        let discarded = streams
            .get(&session)
            .is_some_and(|held| !held.pending.is_empty());
        streams.insert(
            session,
            Stream {
                actor,
                generation,
                attributed,
                pending: AnalyserPendingQueue::default(),
                totals: Default::default(),
                started: std::time::Instant::now(),
                items,
            },
        );
        Some(discarded)
    }

    pub(super) fn forget(&self, session: GameSessionId) {
        if let Ok(mut streams) = self.0.lock() {
            streams.remove(&session);
        }
    }

    pub(super) fn push(&self, actor: ExactActorRef, session: GameSessionId, fact: AnalyserFact) {
        self.push_hit(actor, session, fact, None);
    }

    /// Capability 486: one own-character hit with its attacker, emitted from the committed
    /// damage path. Unselected sessions receive the original fact bytes.
    pub(super) fn push_hit(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        fact: AnalyserFact,
        attacker: Option<ExactActorRef>,
    ) {
        if let Ok(mut streams) = self.0.lock()
            && let Some(held) = streams.get_mut(&session).filter(|held| held.actor == actor)
        {
            held.totals.observe(&fact);
            let attacker = attacker
                .filter(|attacker| held.attributed && *attacker != actor)
                .filter(|_| {
                    matches!(
                        fact,
                        AnalyserFact::DamageInput {
                            source: oteryn_protocol_oteryn::analyser::AnalyserDamageSource::Creature(_)
                                | oteryn_protocol_oteryn::analyser::AnalyserDamageSource::Player,
                            ..
                        }
                    )
                })
                .map(entity_ref)
                .filter(|entity| entity.generation != 0);
            // Enrichment must never reject the gameplay outcome it observes.
            let _ = held.pending.push_attributed(&fact, attacker.as_ref());
        }
    }

    pub(super) fn accounting(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<super::analyser_party::AccountingSnapshot> {
        let streams = self.0.lock().ok()?;
        let held = streams.get(&session).filter(|held| held.actor == actor)?;
        Some((
            held.started.elapsed().as_secs(),
            held.totals.clone(),
            held.items.clone(),
        ))
    }

    pub(super) fn next(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        generation: u64,
    ) -> Option<Vec<u8>> {
        self.0
            .lock()
            .ok()?
            .get_mut(&session)
            .filter(|held| held.actor == actor && held.generation == generation)?
            .pending
            .take_batch()
    }

    pub(super) fn impact(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        value: u32,
        element: Option<DamageElement>,
    ) {
        if let Some(value) = NonZeroU32::new(value) {
            self.push(
                actor,
                session,
                AnalyserFact::Impact(match element {
                    Some(element) => AnalyserImpact::Damage { value, element },
                    None => AnalyserImpact::Healing(value),
                }),
            );
        }
    }

    pub(super) fn impact_amount(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        mut value: u64,
        element: Option<DamageElement>,
    ) {
        while value > 0 {
            let chunk = value.min(u64::from(u32::MAX)) as u32;
            self.impact(actor, session, chunk, element);
            value -= u64::from(chunk);
        }
    }

    pub(super) fn input_amount(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        mut value: u64,
        element: DamageElement,
        source: HitSource,
    ) {
        while let Some(chunk) = NonZeroU32::new(value.min(u64::from(u32::MAX)) as u32) {
            self.push_hit(
                actor,
                session,
                AnalyserFact::DamageInput {
                    value: chunk,
                    element,
                    source: source.source,
                },
                source.attacker,
            );
            value -= u64::from(chunk.get());
        }
    }

    pub(super) fn supply(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        definition: &TypedDefinitionRef,
        quantity: u32,
    ) {
        let Some(count) = u16::try_from(quantity).ok().and_then(NonZeroU16::new) else {
            return;
        };
        let item_type = self.0.lock().ok().and_then(|streams| {
            streams.get(&session).and_then(|held| {
                held.items.as_ref()?.definition_ref(
                    &definition.family,
                    &definition.production_key,
                    &definition.revision_ref,
                )
            })
        });
        if let Some(item_type) = item_type {
            self.push(
                actor,
                session,
                AnalyserFact::SupplyUsed(AnalyserItemCount { item_type, count }),
            );
        }
    }

    pub(super) fn supplies(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        operations: &[crate::durability::spell_items_abi::SpellItemOperation],
    ) {
        for operation in operations {
            if let crate::durability::spell_items_abi::SpellItemOperation::ConsumeInventory(
                source,
            ) = operation
            {
                self.supply(
                    actor,
                    session,
                    &source.definition.definition,
                    source.quantity_before.saturating_sub(source.quantity_after),
                );
            }
        }
    }
}

/// Retained only as long as the exact bounded native corpse projection. Both queue retries
/// and a replay that re-captures a settled corpse share these advisory receipts.
#[derive(Debug, Default)]
pub(super) struct DeathReceipts {
    pub(super) kill: bool,
    pub(super) loot: HashSet<[u8; 16]>,
    pub(super) xp: HashSet<crate::domain::CharacterId>,
    pub(super) recipients: Option<Vec<(crate::domain::CharacterId, ExactActorRef, GameSessionId)>>,
}

pub(super) fn race(key: &str) -> Option<NonZeroU32> {
    let races = super::charm_progression::canonical_races()?;
    let index = races.binary_search_by(|race| race.key().cmp(key)).ok()?;
    NonZeroU32::new(u32::try_from(index).ok()?.checked_add(1)?)
}

pub(super) fn element(kind: crate::player_lethal::PlayerHitKind) -> DamageElement {
    use crate::player_lethal::PlayerHitKind as K;
    match kind {
        K::Physical => DamageElement::Physical,
        K::Energy => DamageElement::Energy,
        K::Fire => DamageElement::Fire,
        K::Earth => DamageElement::Earth,
        K::Ice => DamageElement::Ice,
        K::Holy => DamageElement::Holy,
        K::Death => DamageElement::Death,
        K::LifeDrain => DamageElement::LifeDrain,
        K::ManaDrain => DamageElement::ManaDrain,
        K::Mitigated | K::Undefined => DamageElement::Untyped,
    }
}

pub(super) fn source(runtime: &ChannelRuntimeV1, actor: ExactActorRef) -> HitSource {
    use oteryn_protocol_oteryn::analyser::AnalyserDamageSource as S;
    HitSource {
        source: runtime
            .companion_snapshot_including_dead(actor)
            .ok()
            .map_or(S::Player, |view| {
                S::Creature(race(&view.state.policy.definition_key))
            }),
        attacker: Some(actor),
    }
}

pub(super) fn charm_element(
    element: oteryn_gameplay_progression::charm_effects::CharmElement,
) -> DamageElement {
    use oteryn_gameplay_progression::charm_effects::CharmElement as C;
    match element {
        C::Physical => DamageElement::Physical,
        C::Fire => DamageElement::Fire,
        C::Earth => DamageElement::Earth,
        C::Ice => DamageElement::Ice,
        C::Energy => DamageElement::Energy,
        C::Death => DamageElement::Death,
        C::Holy => DamageElement::Holy,
    }
}
