//! Raid placement borrows the existing physical Item/Map transaction and the
//! current Channel creature owner. No detached map snapshot grants a spawn.
mod cake;

use super::ComposedFreshAdmission;
use crate::durability::DurabilityError;
use crate::durability::raid_runner::{
    CommittedRaidAction, CommittedRaidStep, RaidCreaturePlan, RaidPlacementOutcome,
};
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::map::boot::BundleCollisionIndexExt;
use crate::map::view::MapFacts;
use crate::movement::monster_map::CurrentMonsterMap;
use oteryn_gameplay_encounter_rules::raid::{
    CatalogueSpawnMode, MAX_CREATURES, RAID_PLACEMENT_ATTEMPTS, RaidPosition, RaidRandom,
    SelectedRaid,
};
use oteryn_gameplay_entities::carrier::{NativeRaidSpawnSpec, RaidSpawnIdentity, RaidSpawnOutcome};
use std::collections::BTreeSet;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn apply_raid_wave(
        &self,
        step: &CommittedRaidStep,
        selected: &SelectedRaid,
        seed: [u8; 32],
    ) -> Result<Vec<RaidPlacementOutcome>, ()> {
        let CommittedRaidAction::Wave {
            creatures, wave, ..
        } = &step.action
        else {
            return Err(());
        };
        if creatures.len() > usize::from(MAX_CREATURES) {
            return Err(());
        }
        let mut identities = BTreeSet::new();
        for creature in creatures {
            let identity = creature.identity;
            let entry = selected
                .definition
                .waves
                .get(usize::from(identity.wave))
                .and_then(|wave| wave.entries.get(usize::from(identity.entry)))
                .ok_or(())?;
            if identity.wave != *wave
                || identity.index >= entry.count
                || creature.creature != entry.creature
                || creature.area_anchor != entry.area_anchor
                || !identities.insert(identity)
            {
                return Err(());
            }
        }
        // Preserve the existing physical owner's lock order: lane, runtime,
        // Item transaction, then LocalObjectRuntime.
        let permit = self.spell_lane_permit().await.ok_or(())?;
        let mut runtime = self.runtime.lock().await;
        let binding = runtime.binding();
        if binding.channel_id() != Some(step.identity.channel)
            || binding.scope_generation() != step.identity.generation
            || binding.world_id() != self.world_id
        {
            return Err(());
        }
        let room = self.rooms().gate_monster_ai().ok_or(())?;
        let native = self
            .active_generation
            .and_then(|generation| generation.native_gameplay())
            .ok_or(())?;
        let pass = self.root.try_issue_semantic_pass().map_err(|_| ())?;
        let mut context = (self, &mut *runtime, &permit, room, native, step, selected);
        pass.run_with_context(&mut context, move |holder, deadline, context| {
            Box::pin(async move {
                use crate::durability::spell_item_transaction::{
                    assert_spell_item_scope_in_transaction, begin_spell_owner_transaction,
                };
                let (owner, runtime, permit, room, native, step, selected) = context;
                let binding = runtime.binding();
                let mut tx = begin_spell_owner_transaction(holder, deadline).await?;
                let authority = assert_spell_item_scope_in_transaction(
                    &mut tx, permit, owner.root, owner.character, owner.holder,
                    binding.runtime_scope(), binding.scope_generation().get(),
                ).await.map_err(|_| DurabilityError::Unavailable)?;
                let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) = room.movement_cells().index()
                else { return Err(DurabilityError::Unavailable); };
                if !crate::durability::raid_runner::raid_effect_pending_in_transaction(
                    &mut tx, owner.holder, step, owner.world_id, owner.reset_epoch,
                    index.map().overlay.base().digest(),
                ).await? { return Err(DurabilityError::Unavailable); }
                // Identity comes from the currently locked firing, never a creature name
                // or a retained proposal. The existing owner already proved this effect's
                // world, reset, content, Channel generation and native action body above.
                let raid_key: String = sqlx::query_scalar("SELECT raid_key FROM game_world_raid_firings WHERE firing_id=encode($1,'hex')::uuid")
                    .bind(step.identity.firing_id.as_slice()).fetch_one(&mut *tx).await?;
                let grimvale = raid_key == "raid:edron.feroxa";
                if grimvale && !crate::durability::world_event::changes::callbacks::require(
                    &mut tx, owner.world_id, owner.reset_epoch,
                    "oteryn:quest-callback/grimvale-full-moon-entry",
                ).await? {
                    // Refusal is denial-only: revoke a retained local entry too.
                    let _ = project_grimvale_portal(index.map(), false);
                    return Err(DurabilityError::Unavailable);
                }
                let objects = owner.door.lock().await;
                let mut map = CurrentMonsterMap::new(&mut tx, &authority, runtime, room, &objects, native);
                let random = RaidRandom::new(seed, step.identity.firing_id);
                let stamp = runtime.issue_owner_work().map_err(|_| DurabilityError::Unavailable)?;
                let mut placement = PlacementContext {
                    random: &random, selected, bundle: index.map(),
                    remaining_reads: usize::from(MAX_CREATURES) * usize::from(RAID_PLACEMENT_ATTEMPTS),
                    stamp, firing: step.identity.firing_id,
                };
                let retained: BTreeSet<_> = runtime.raid_creatures(step.identity.firing_id)
                    .map_err(|_| DurabilityError::Unavailable)?.into_iter().map(|row| row.identity).collect();

                let CommittedRaidAction::Wave { creatures, .. } = &step.action else {
                    return Err(DurabilityError::InvalidStoredState);
                };
                let mut outcomes = Vec::with_capacity(creatures.len());
                for creature in creatures {
                    let identity = RaidSpawnIdentity {
                        firing_id: step.identity.firing_id, wave: creature.identity.wave,
                        entry: creature.identity.entry, index: creature.identity.index,
                    };
                    let policy = runtime.companion_policy(creature.creature.as_str())
                        .map_err(|_| DurabilityError::Unavailable)?;
                    let spec = NativeRaidSpawnSpec::qualified(
                        creature.creature.as_str(), &policy.definition_revision,
                        runtime.content_pin().server_artifact_digest(),
                        selected.boss_entries.contains(&(identity.wave, identity.entry)),
                    ).map_err(|_| DurabilityError::Unavailable)?;
                    // A retry returns the original success/failure before map
                    // lookup. A dead actor never becomes a second spawn.
                    let at = if retained.contains(&identity) { None }
                        else { placement.select(&mut map, runtime, creature).await? };
                    if !map.validate_transaction(runtime).await {
                        return Err(DurabilityError::Unavailable);
                    }
                    let receipt = runtime.realize_raid_spawn(stamp, identity, &spec, at.map(local), |owner, at| {
                        Some(map.view().matches(owner) && map.view().admits(at) && non_house(index.map(), at))
                    }).map_err(|_| DurabilityError::Unavailable)?;
                    outcomes.push(RaidPlacementOutcome {
                        identity: creature.identity,
                        placed: matches!(receipt.outcome, RaidSpawnOutcome::Placed(_)),
                    });
                }
                // Every live consumer finished while the physical Item lock
                // was held. This transaction only read current world facts.
                drop(map);
                if grimvale && final_wave_placed(selected, match &step.action { CommittedRaidAction::Wave { wave, .. } => *wave, _ => return Err(DurabilityError::InvalidStoredState) }, &outcomes)
                    && !project_grimvale_portal(index.map(), true)
                {
                    return Err(DurabilityError::Unavailable);
                }
                tx.rollback().await.map_err(|_| DurabilityError::Unavailable)?;
                Ok(outcomes)
            })
        }).await.map_err(|_| ())
    }
}

/// The source Lua occurrence opens after its final successful native wave (75 minutes).
/// Until-reset completion
/// retains this projection; revocation restores the original1949 native travel.
fn final_wave_placed(
    selected: &SelectedRaid,
    wave: u16,
    outcomes: &[RaidPlacementOutcome],
) -> bool {
    usize::from(wave).checked_add(1) == Some(selected.definition.waves.len())
        && !outcomes.is_empty()
        && outcomes.iter().all(|outcome| outcome.placed)
}

pub(super) fn project_grimvale_portal(bundle: &crate::map::boot::BundleMap, open: bool) -> bool {
    use crate::map::view::placement_key;
    let at = crate::map::overlay::TilePos {
        x: 33430,
        y: 31537,
        floor: -11,
    };
    if bundle.facts.house_tile(at) {
        return false;
    }
    let Some(tile) = bundle.overlay.base().tile(at.x, at.y, at.floor) else {
        return false;
    };
    // Native B3: ground11038, then the bound1949 source teleport at ordinal1.
    let ordinal = 1;
    let Some((&id, _)) = tile
        .ids()
        .iter()
        .zip(tile.depths())
        .filter(|(_, depth)| **depth == 0)
        .nth(usize::from(ordinal))
    else {
        return false;
    };
    let Some(key) = placement_key(at, ordinal) else {
        return false;
    };
    if bundle.facts.base_hidden(key) {
        return false;
    }
    let Some(before) = bundle.facts.base_entry(at, ordinal, id) else {
        return false;
    };
    if !matches!(before.appearance_id, 1949 | 22761) {
        return false;
    }
    let appearance = if open { 22761 } else { 1949 };
    if before.appearance_id == appearance {
        return true;
    }
    let Some(mut after) = bundle.facts.movement_item_facts(appearance) else {
        return false;
    };
    after.bound = before.bound;
    bundle.facts.apply_quest_callback(
        key,
        bundle.facts.object_revision(key),
        after,
        &[],
        &[],
        false,
        None,
    )
}

fn local(at: RaidPosition) -> MovementLocalPosition {
    MovementLocalPosition {
        x: at.x,
        y: at.y,
        floor: at.floor,
    }
}

fn non_house(bundle: &crate::map::boot::BundleMap, at: MovementLocalPosition) -> bool {
    let (Ok(x), Ok(y)) = (u16::try_from(at.x), u16::try_from(at.y)) else {
        return false;
    };
    let Some(floor) = at
        .floor
        .checked_neg()
        .and_then(|floor| i8::try_from(floor).ok())
    else {
        return false;
    };
    !bundle
        .facts
        .house_tile(crate::map::overlay::TilePos { x, y, floor })
}

struct PlacementContext<'a> {
    random: &'a RaidRandom,
    selected: &'a SelectedRaid,
    bundle: &'a crate::map::boot::BundleMap,
    remaining_reads: usize,
    stamp: oteryn_gameplay_foundation::foundation::RuntimeWorkStamp,
    firing: [u8; 16],
}

impl PlacementContext<'_> {
    async fn admits(
        &mut self,
        map: &mut CurrentMonsterMap<'_, '_>,
        runtime: &ChannelRuntimeV1,
        at: RaidPosition,
    ) -> Result<bool, DurabilityError> {
        self.remaining_reads = self
            .remaining_reads
            .checked_sub(1)
            .ok_or(DurabilityError::InvalidStoredState)?;
        let at = local(at);
        if !non_house(self.bundle, at) || !runtime.native_summon_cell_free(at) {
            return Ok(false);
        }
        Ok(map.read(runtime, at).await == Some(true))
    }

    async fn select(
        &mut self,
        map: &mut CurrentMonsterMap<'_, '_>,
        runtime: &mut ChannelRuntimeV1,
        plan: &RaidCreaturePlan,
    ) -> Result<Option<RaidPosition>, DurabilityError> {
        let area = *self
            .selected
            .areas
            .get(&plan.area_anchor)
            .ok_or(DurabilityError::InvalidStoredState)?;
        area.validate()
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let identity = plan.identity;
        let mode = self
            .selected
            .spawn_modes
            .get(&(identity.wave, identity.entry))
            .ok_or(DurabilityError::InvalidStoredState)?;
        match mode {
            CatalogueSpawnMode::Single => {
                if area.from != area.to {
                    return Err(DurabilityError::InvalidStoredState);
                }
                Ok(self
                    .admits(map, runtime, area.from)
                    .await?
                    .then_some(area.from))
            }
            CatalogueSpawnMode::Area => {
                for attempt in 0..RAID_PLACEMENT_ATTEMPTS {
                    let at = area
                        .candidate(
                            self.random,
                            identity.wave,
                            identity.entry,
                            identity.index,
                            attempt,
                        )
                        .map_err(|_| DurabilityError::InvalidStoredState)?;
                    if self.admits(map, runtime, at).await? {
                        return Ok(Some(at));
                    }
                }
                Ok(None)
            }
            CatalogueSpawnMode::SharedArea => {
                while let Some(attempt) = runtime
                    .spend_raid_shared_attempt(self.stamp, self.firing, identity.wave)
                    .map_err(|_| DurabilityError::Unavailable)?
                {
                    let at = area
                        .candidate_shared(
                            self.random,
                            identity.wave,
                            identity.entry,
                            identity.index,
                            attempt,
                        )
                        .map_err(|_| DurabilityError::InvalidStoredState)?;
                    if self.admits(map, runtime, at).await? {
                        return Ok(Some(at));
                    }
                }
                Ok(None)
            }
            CatalogueSpawnMode::Zone => {
                let count = (i64::from(area.to.x) - i64::from(area.from.x) + 1)
                    .checked_mul(i64::from(area.to.y) - i64::from(area.from.y) + 1)
                    .and_then(|n| {
                        n.checked_mul(i64::from(area.to.floor) - i64::from(area.from.floor) + 1)
                    })
                    .and_then(|n| usize::try_from(n).ok())
                    .ok_or(DurabilityError::InvalidStoredState)?;
                if count > self.remaining_reads {
                    return Err(DurabilityError::InvalidStoredState);
                }
                let mut allowed = Vec::with_capacity(count);
                // The donor exposes an unordered_set of positions. Canonical
                // z/y/x order preserves its uniform choice and makes replay stable.
                for floor in area.from.floor..=area.to.floor {
                    for y in area.from.y..=area.to.y {
                        for x in area.from.x..=area.to.x {
                            let at = RaidPosition { x, y, floor };
                            if self.admits(map, runtime, at).await? {
                                allowed.push(at);
                            }
                        }
                    }
                }
                self.random
                    .zone_position(identity.wave, identity.entry, identity.index, &allowed)
                    .map_err(|_| DurabilityError::InvalidStoredState)
            }
        }
    }
}
