//! Crystal 201189f8, ferumbras_ascension/creaturescripts_rift_invader_death.lua:1-10,23-45.
use super::{CarrierError, ChannelRuntimeV1, ExactActorRef, MovementLocalPosition};
use crate::content::NativeEntryMovementCells;
use crate::content::native_cell_lookup::NativeMovementCollisionIndex;
use crate::map::view::{MapFacts, placement_key};
use oteryn_gameplay_quests::quest::world_callbacks::{DeathCrystalCircuit, Position};
use oteryn_gameplay_world::map::boot::BundleCollisionIndexExt;

#[derive(Debug, Default)]
pub(super) struct Charges([u16; 8]);

impl Charges {
    pub(super) fn crystal(
        &self,
        plan: &DeathCrystalCircuit,
        creature: &str,
        at: MovementLocalPosition,
    ) -> Option<usize> {
        if creature != plan.defeated {
            return None;
        }
        let at = super::super::quest_callbacks::native(at)?;
        plan.crystals
            .iter()
            .enumerate()
            .position(|(index, crystal)| {
                self.0[index] < plan.charges_per_crystal
                    && crystal.area.contains(Position {
                        x: at.x,
                        y: at.y,
                        floor: at.floor,
                    })
            })
    }

    pub(super) fn count_death(
        &mut self,
        plan: &DeathCrystalCircuit,
        index: usize,
        runtime: &mut ChannelRuntimeV1,
        cells: &NativeEntryMovementCells,
    ) -> Result<Vec<ExactActorRef>, CarrierError> {
        let count = self.0[index] + 1;
        if count < plan.charges_per_crystal {
            self.0[index] = count;
            return Ok(Vec::new());
        }
        let NativeMovementCollisionIndex::Bundle(bundle) = cells.index() else {
            return Err(CarrierError::PlanConflict);
        };
        let map = bundle.map();
        let at = tile(plan.crystals[index].position);
        let base = map
            .overlay
            .base()
            .tile(at.x, at.y, at.floor)
            .ok_or(CarrierError::PlanConflict)?;
        let mut placements = base
            .ids()
            .iter()
            .zip(base.depths())
            .filter(|(_, depth)| **depth == 0)
            .enumerate()
            .filter_map(|(ordinal, (id, _))| {
                let ordinal = u8::try_from(ordinal).ok()?;
                let key = placement_key(at, ordinal)?;
                let facts = map.facts.base_entry(at, ordinal, *id)?;
                (facts.appearance_id == plan.dormant_appearance
                    && !facts.pickupable
                    && !map.facts.base_hidden(key))
                .then_some(key)
            });
        let Some(key) = placements.next() else {
            // The donor exits without credit when the dormant crystal has gone.
            return Ok(Vec::new());
        };
        if placements.next().is_some() {
            return Err(CarrierError::PlanConflict);
        }
        let revision = map.facts.object_revision(key);
        let after = map
            .facts
            .quest_callback_replacement(key, plan.charged_appearance)
            .filter(|facts| !facts.pickupable)
            .ok_or(CarrierError::PlanConflict)?;
        let project = || {
            map.facts
                .apply_quest_callback(key, revision, after, &[], &[], false, None)
        };
        let last = self.0[..plan.crystals.len()]
            .iter()
            .enumerate()
            .all(|(other, value)| other == index || *value == plan.charges_per_crystal);
        let actors = if last {
            let boss_at = super::super::quest_callbacks::local(plan.boss_position);
            let mut bosses = runtime
                .current_live_creature_census()?
                .into_iter()
                .filter(|actor| {
                    runtime.matches_live_creature_identity(*actor, plan.retire.as_bytes())
                        && runtime
                            .read_actor_position(*actor)
                            .is_ok_and(|at| at.position() == boss_at)
                });
            let boss = bosses.next().ok_or(CarrierError::CreatureTargetMismatch)?;
            if bosses.next().is_some() || !super::placement(cells, runtime, boss_at) {
                return Err(CarrierError::PlanConflict);
            }
            let health = runtime.companion_policy(&plan.spawn)?.maximum_health;
            let spawn = vec![(plan.spawn.as_str(), health, boss_at); plan.crystals.len()];
            runtime.commit_quest_creature_replacement(&[boss], &spawn, project)?
        } else {
            if !project() {
                return Err(CarrierError::PlanConflict);
            }
            Vec::new()
        };
        self.0[index] = count;
        if last && let Some(effect) = plan.charged_effect {
            for crystal in &plan.crystals {
                map.facts
                    .emit_movement_effect(tile(crystal.position), effect);
            }
        }
        Ok(actors)
    }
}

fn tile(position: Position) -> crate::map::overlay::TilePos {
    crate::map::overlay::TilePos {
        x: position.x,
        y: position.y,
        floor: position.floor,
    }
}
