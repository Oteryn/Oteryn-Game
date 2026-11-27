//! ANALYSER-EMIT-1: bounded advisory facts, derived only from committed outcomes.
//! Rules never read these facts. Only selected, exact actor/session bindings have queues.
use std::collections::{HashMap, HashSet};
use std::num::{NonZeroU16, NonZeroU32};
use std::sync::{Arc, Mutex, OnceLock};

use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId};
use oteryn_protocol_oteryn::analyser::{
    AnalyserCharmSource, AnalyserFact, AnalyserImpact, AnalyserItemCount, AnalyserPendingQueue,
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
    charm_source: bool,
    pending: AnalyserPendingQueue,
    totals: super::analyser_party::Totals,
    hunt: Option<(
        super::analyser_party::shared::Boundary,
        super::analyser_party::Totals,
    )>,
    started: std::time::Instant,
    items: Option<Arc<crate::content::item_ref::ItemDefinitionIndex>>,
}

/// Activity runtimes share this leaf hub with their originating Channel. Terminal retirement
/// removes entries. No leaf guard is held while acquiring another owner or awaiting IO.
#[derive(Debug, Default)]
pub struct AnalyserStreams {
    streams: Mutex<HashMap<GameSessionId, Stream>>,
    chat: OnceLock<CombatChat>,
}

pub(super) struct CharmImpact<'a> {
    pub target: ExactActorRef,
    pub result: oteryn_gameplay_entities::OwnerDamageResult,
    pub element: oteryn_gameplay_progression::charm_effects::CharmElement,
    pub key: &'a str,
}

#[derive(Debug)]
struct CombatChat {
    chat: super::chat_intent::ChatFeedback,
    creature_names: HashMap<String, (String, String)>,
}

impl AnalyserStreams {
    pub(super) fn bind_chat(
        &self,
        chat: &super::chat_intent::ChatRuntime,
        content: Option<&crate::content::native_gameplay::NativeGameplayState>,
    ) {
        self.chat.get_or_init(|| CombatChat {
            chat: chat.feedback(),
            creature_names: content
                .into_iter()
                .flat_map(|content| &content.creature_profiles().records)
                .filter_map(creature_name_description)
                .collect(),
        });
    }

    #[cfg(test)]
    pub(super) fn snapshot(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        generation: u64,
        items: Option<Arc<crate::content::item_ref::ItemDefinitionIndex>>,
    ) -> Option<bool> {
        self.snapshot_attributed(actor, session, generation, items, false, false)
    }

    pub(super) fn snapshot_attributed(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        generation: u64,
        items: Option<Arc<crate::content::item_ref::ItemDefinitionIndex>>,
        attributed: bool,
        charm_source: bool,
    ) -> Option<bool> {
        let mut streams = self.streams.lock().ok()?;
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
                charm_source,
                pending: AnalyserPendingQueue::default(),
                totals: Default::default(),
                hunt: None,
                started: std::time::Instant::now(),
                items,
            },
        );
        Some(discarded)
    }

    pub(super) fn forget(&self, session: GameSessionId) {
        if let Ok(mut streams) = self.streams.lock() {
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
        if let Ok(mut streams) = self.streams.lock()
            && let Some(held) = streams.get_mut(&session).filter(|held| held.actor == actor)
        {
            held.observe(&fact);
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
        let streams = self.streams.lock().ok()?;
        let held = streams.get(&session).filter(|held| held.actor == actor)?;
        Some((
            held.started.elapsed().as_secs(),
            held.totals.clone(),
            held.items.clone(),
        ))
    }

    pub(super) fn hunt_accounting(
        &self,
        member: super::analyser_party::shared::LocalMember,
        interval: u64,
    ) -> Option<super::analyser_party::shared::Measurement> {
        let streams = self.streams.lock().ok()?;
        let held = streams
            .get(&member.session)
            .filter(|held| held.actor == member.actor)?;
        let totals = if interval == 0 {
            held.totals.clone()
        } else {
            held.hunt
                .as_ref()
                .filter(|(boundary, _)| *boundary == member.boundary)
                .map(|(_, totals)| totals.clone())
                .unwrap_or_else(super::analyser_party::Totals::missing_history)
        };
        Some((totals, held.items.clone()))
    }

    pub(super) fn set_hunt_boundaries(
        &self,
        members: &[super::analyser_party::shared::LocalMember],
        reset: bool,
    ) -> Option<()> {
        let mut streams = self.streams.lock().ok()?;
        for member in members {
            if let Some(held) = streams
                .get_mut(&member.session)
                .filter(|held| held.actor == member.actor)
                && held
                    .hunt
                    .as_ref()
                    .is_none_or(|(boundary, _)| *boundary != member.boundary)
            {
                let totals = if reset {
                    super::analyser_party::Totals::default()
                } else {
                    super::analyser_party::Totals::missing_history()
                };
                held.hunt = Some((member.boundary, totals));
            }
        }
        Some(())
    }

    pub(super) fn next(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        generation: u64,
    ) -> Option<Vec<u8>> {
        self.streams
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

    pub(super) fn charm_impact(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        impact: CharmImpact<'_>,
    ) {
        let Some(loss) = impact.result.committed_health_loss() else {
            return;
        };
        if let Ok(mut streams) = self.streams.lock()
            && let Some(held) = streams.get_mut(&session).filter(|held| held.actor == actor)
        {
            let mut remaining = loss.get();
            let charm = held.charm_source.then(|| AnalyserCharmSource {
                charm_key: impact.key.to_owned(),
                target: entity_ref(impact.target),
            });
            while let Some(value) = NonZeroU32::new(remaining.min(u64::from(u32::MAX)) as u32) {
                let fact = AnalyserFact::Impact(AnalyserImpact::Damage {
                    value,
                    element: charm_element(impact.element),
                });
                held.observe(&fact);
                if charm
                    .as_ref()
                    .is_none_or(|source| held.pending.push_charm_impact(&fact, source).is_err())
                {
                    let _ = held.pending.push(&fact);
                }
                remaining -= u64::from(value.get());
            }
        }
        let Some(binding) = self.chat.get() else {
            return;
        };
        let Ok(creature) = runtime.companion_snapshot_including_dead(impact.target) else {
            return;
        };
        let policy = &creature.state.policy;
        let name = binding
            .creature_names
            .get(&policy.definition_key)
            .filter(|(revision, _)| *revision == policy.definition_revision)
            .map_or(policy.display_name.as_str(), |(_, name)| name);
        let charm = impact
            .key
            .strip_prefix("oteryn:charm.")
            .unwrap_or(impact.key)
            .replace('_', " ");
        let points = if loss.get() == 1 {
            "hitpoint"
        } else {
            "hitpoints"
        };
        binding.chat.push(
            session,
            &format!("{name} loses {loss} {points} due to your attack. ({charm} charm)"),
        );
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
        let item_type = self.streams.lock().ok().and_then(|streams| {
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

impl Stream {
    fn observe(&mut self, fact: &AnalyserFact) {
        self.totals.observe(fact);
        if let Some((_, totals)) = &mut self.hunt {
            totals.observe(fact);
        }
    }
}

fn creature_name_description(
    record: &crate::content::native_gameplay::CreatureProfileRecord,
) -> Option<(String, (String, String))> {
    use crate::content::ProjectV2AuthoringProfileData;
    let ProjectV2AuthoringProfileData::Creature(profile) = &record.profile.data else {
        return None;
    };
    let details = profile.details.as_ref()?;
    let article = details.article.as_deref().unwrap_or_default();
    let mut chars = article.chars();
    let first = chars.next()?;
    let name = format!(
        "{}{} {}",
        first.to_uppercase(),
        chars.as_str(),
        details.display_name.to_lowercase()
    );
    let target = &record.profile.target;
    Some((target.key.clone(), (target.revision.clone(), name)))
}

/// Retained only as long as the exact bounded native corpse projection. Both queue retries
/// and a replay that re-captures a settled corpse share these advisory receipts.
#[derive(Debug, Default)]
pub(super) struct DeathReceipts {
    pub(super) boss_defeated: bool,
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
