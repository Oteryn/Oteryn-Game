//! Native CHARM-2/3 reader. Content indices belong to the supplied revision binding; durable
//! authority remains in DurabilityRoot. This module does not advertise or activate capability 1.

use std::collections::BTreeMap;
use std::num::NonZeroU32;

use oteryn_protocol_oteryn::bestiary::{
    BestiaryRaceProgress, MAX_BESTIARY_KILL_THRESHOLD, MAX_BESTIARY_VIEW_ENTRIES,
    encode_bestiary_view,
};
use oteryn_protocol_oteryn::charm::{
    CharmKind, CharmState, CharmView, MAX_CHARM_BALANCE, MAX_CHARM_STAGE_COST, MAX_CHARMS,
    encode_charm_view,
};

use super::CharmPortUnavailable;
use crate::combat::charm_effects::{
    CharmCatalogueRead, CharmCategory as EffectCategory, CharmDefinition as EffectDefinition,
};
use crate::domain::CharacterRevision;
use crate::domain::bestiary::BestiaryRace;
use crate::domain::charm::{
    BestiaryRaceKey, CharmCatalogue, CharmCategory, CharmCurrency, CharmKey,
};
use crate::durability::DurabilityRoot;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::charm_state::{
    CharacterCharmProgressionSnapshot, CharmFacts, CharmProgressionReadRequest,
};
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;

/// The supplier provides validated CHARM-4 definitions for both wire indices and combat lookup.
/// Comparing its revision with the Character root checks expected binding only. Content-owner
/// loader/digest integration must establish provenance before claiming a qualified generation.
pub(crate) struct CharmConnectionContent {
    // The standalone PostgreSQL suites exercise the native reader; lib tests cover projection.
    #[cfg_attr(test, allow(dead_code))]
    revision: String,
    catalogue: CharmCatalogue,
    charms: Vec<CharmKey>,
    races: Vec<(BestiaryRaceKey, BestiaryRace)>,
    effects: BTreeMap<String, EffectDefinition>,
}

impl CharmConnectionContent {
    pub(crate) fn new(
        revision: String,
        catalogue: CharmCatalogue,
        races: Vec<BestiaryRace>,
        effects: &impl CharmCatalogueRead,
    ) -> Result<Self, CharmPortUnavailable> {
        if races.len() > MAX_BESTIARY_VIEW_ENTRIES
            || catalogue.definitions().take(MAX_CHARMS + 1).count() > MAX_CHARMS
        {
            return Err(CharmPortUnavailable);
        }
        let mut bound_races = Vec::with_capacity(races.len());
        for race in races {
            if race.kill_thresholds().len() != 3
                || race.final_kill_threshold() > MAX_BESTIARY_KILL_THRESHOLD
            {
                return Err(CharmPortUnavailable);
            }
            let key = BestiaryRaceKey::new(race.key()).map_err(|_| CharmPortUnavailable)?;
            bound_races.push((key, race));
        }
        bound_races.sort_unstable_by(|left, right| left.0.cmp(&right.0));
        if bound_races.windows(2).any(|pair| pair[0].0 == pair[1].0) {
            return Err(CharmPortUnavailable);
        }
        let mut bound_effects = BTreeMap::new();
        let mut charms = Vec::new();
        for definition in catalogue.definitions() {
            let effect = effects
                .charm(definition.key.as_str())
                .ok_or(CharmPortUnavailable)?;
            let category = match definition.category {
                CharmCategory::Major => EffectCategory::Major,
                CharmCategory::Minor => EffectCategory::Minor,
            };
            if effect.key() != definition.key.as_str()
                || effect.category() != category
                || definition
                    .stage_costs
                    .iter()
                    .any(|cost| *cost > MAX_CHARM_STAGE_COST)
            {
                return Err(CharmPortUnavailable);
            }
            charms.push(definition.key.clone());
            bound_effects.insert(definition.key.as_str().to_owned(), effect.clone());
        }
        Ok(Self {
            revision,
            catalogue,
            charms,
            races: bound_races,
            effects: bound_effects,
        })
    }

    #[cfg_attr(test, allow(dead_code))] // Used by the native PostgreSQL reader, not lib projection.
    fn request(&self) -> CharmProgressionReadRequest {
        CharmProgressionReadRequest {
            catalogue_revision: self.revision.clone(),
            catalogue: self.catalogue.clone(),
            races: self.races.iter().map(|(key, _)| key.clone()).collect(),
        }
    }

    fn race_index(&self, key: &BestiaryRaceKey) -> Result<NonZeroU32, CharmPortUnavailable> {
        let index = self
            .races
            .binary_search_by(|(race, _)| race.cmp(key))
            .map_err(|_| CharmPortUnavailable)?;
        wire_index(index)
    }

    fn project(
        &self,
        snapshot: CharacterCharmProgressionSnapshot,
    ) -> Result<CharmConnectionViews, CharmPortUnavailable> {
        // An obsolete assignment has no generation index. Do not hide it or invent a new index.
        for key in snapshot
            .state
            .unlocks
            .keys()
            .chain(snapshot.state.assignments.keys())
        {
            self.catalogue.get(key).map_err(|_| CharmPortUnavailable)?;
        }
        let mut bestiary = Vec::new();
        for (index, (key, race)) in self.races.iter().enumerate() {
            if let Some(count) = snapshot
                .bestiary_counts
                .get(key)
                .copied()
                .filter(|count| *count > 0)
            {
                bestiary.push(BestiaryRaceProgress {
                    race: wire_index(index)?,
                    kill_count: count,
                    kill_thresholds: race
                        .kill_thresholds()
                        .try_into()
                        .map_err(|_| CharmPortUnavailable)?,
                });
            }
        }
        let mut charms = Vec::with_capacity(self.charms.len());
        for (index, key) in self.charms.iter().enumerate() {
            let definition = self.catalogue.get(key).map_err(|_| CharmPortUnavailable)?;
            let stage = snapshot
                .state
                .unlocks
                .get(key)
                .map_or(0, |stage| stage.get());
            charms.push(CharmState {
                charm: wire_index(index)?,
                kind: match definition.category {
                    CharmCategory::Major => CharmKind::Major,
                    CharmCategory::Minor => CharmKind::Minor,
                },
                unlocked_stage: stage,
                assigned_race: snapshot
                    .state
                    .assignments
                    .get(key)
                    .map(|race| self.race_index(race))
                    .transpose()?,
                next_stage_cost: definition
                    .stage_costs
                    .get(usize::from(stage))
                    .copied()
                    .unwrap_or(0),
                // This prepared reader has no connected production Charm effect consumer.
                // Calculation readiness in a definition cannot prove live effect availability.
                // A qualified owner connection must supply that availability before activation.
                effect_active: false,
            });
        }
        let charms = CharmView {
            charms,
            charm_points_available: wire_balance(
                snapshot.balance.available(CharmCurrency::CharmPoints),
            )?,
            minor_charm_echoes_available: wire_balance(
                snapshot.balance.available(CharmCurrency::MinorCharmEchoes),
            )?,
            assignment_slot_limit: snapshot
                .slot_entitlement
                .assignment_slots()
                .map(|slots| {
                    u32::try_from(slots)
                        .ok()
                        .and_then(NonZeroU32::new)
                        .ok_or(CharmPortUnavailable)
                })
                .transpose()?,
        };
        encode_bestiary_view(&bestiary).map_err(|_| CharmPortUnavailable)?;
        encode_charm_view(&charms).map_err(|_| CharmPortUnavailable)?;
        Ok(CharmConnectionViews {
            revision: snapshot.character_revision,
            bestiary,
            charms,
        })
    }
}

impl CharmCatalogueRead for CharmConnectionContent {
    fn charm(&self, key: &str) -> Option<&EffectDefinition> {
        self.effects.get(key)
    }
}

fn wire_index(zero_based: usize) -> Result<NonZeroU32, CharmPortUnavailable> {
    zero_based
        .checked_add(1)
        .and_then(|index| u32::try_from(index).ok())
        .and_then(NonZeroU32::new)
        .ok_or(CharmPortUnavailable)
}

fn wire_balance(value: i128) -> Result<u32, CharmPortUnavailable> {
    u32::try_from(value)
        .ok()
        .filter(|value| *value <= MAX_CHARM_BALANCE)
        .ok_or(CharmPortUnavailable)
}

/// Both snapshots have the same durable revision, ready for connection domain envelopes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CharmConnectionViews {
    pub(crate) revision: CharacterRevision,
    pub(crate) bestiary: Vec<BestiaryRaceProgress>,
    pub(crate) charms: CharmView,
}

/// The runtime independently supplies live session/lease/scope evidence. Only the expected
/// CharacterRevision can be refreshed from storage; an intent or old receipt cannot supply it.
#[cfg_attr(test, allow(dead_code))] // Constructed by the standalone PostgreSQL suites.
pub(crate) struct NativeCharmProgressionPort<'a, 'f, 's, F> {
    root: &'a DurabilityRoot,
    authority: &'a ReconciledCharacterAuthority<'f, 's>,
    node: &'a NodeIncarnationProof,
    fence: CurrentCharacterGameplayFence,
    content: &'a CharmConnectionContent,
    facts: F,
}

#[cfg_attr(test, allow(dead_code))] // Real DB bind/views run in the standalone PostgreSQL suites.
impl<'a, 'f, 's, F: CharmFacts + Clone> NativeCharmProgressionPort<'a, 'f, 's, F> {
    pub(crate) async fn bind(
        root: &'a DurabilityRoot,
        authority: &'a ReconciledCharacterAuthority<'f, 's>,
        node: &'a NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        content: &'a CharmConnectionContent,
        facts: F,
    ) -> Result<Self, CharmPortUnavailable> {
        let port = Self {
            root,
            authority,
            node,
            fence,
            content,
            facts,
        };
        // The supplied revision is checked, together with current live authority, before binding.
        port.snapshot(fence).await?;
        Ok(port)
    }

    async fn current_fence(&self) -> Result<CurrentCharacterGameplayFence, CharmPortUnavailable> {
        let state = self
            .root
            .read_character_progression(self.authority, self.fence.character_id)
            .await
            .map_err(|_| CharmPortUnavailable)?
            .ok_or(CharmPortUnavailable)?;
        if state.context.content != self.content.revision {
            return Err(CharmPortUnavailable);
        }
        Ok(CurrentCharacterGameplayFence {
            expected_character_revision: state.character_revision,
            ..self.fence
        })
    }

    async fn snapshot(
        &self,
        fence: CurrentCharacterGameplayFence,
    ) -> Result<CharacterCharmProgressionSnapshot, CharmPortUnavailable> {
        self.root
            .read_character_charm_progression(
                self.authority,
                self.node,
                fence,
                self.content.request(),
                self.facts.clone(),
            )
            .await
            .map_err(|_| CharmPortUnavailable)
    }

    pub(crate) async fn views(&self) -> Result<CharmConnectionViews, CharmPortUnavailable> {
        let snapshot = self.snapshot(self.current_fence().await?).await?;
        self.content.project(snapshot)
    }
}

#[cfg(test)]
#[path = "charm_native_tests.rs"]
mod tests;
