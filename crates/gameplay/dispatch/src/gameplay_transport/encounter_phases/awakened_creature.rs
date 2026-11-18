//! Crystal 201189f8 actions_mission10_a_message_of_freedom_sceptre.lua:16-19 and
//! creaturescripts_bosses_kill.lua:20-34 use the same physical statue for spawn and reset.
use super::*;
use crate::content::NativeEntryMovementCells;
use crate::map::view::EntryFacts;
use oteryn_gameplay_quests::quest::world_callbacks::EncounterMonster;

#[derive(Debug)]
struct Awakening {
    actor: ExactActorRef,
    scope: (
        crate::foundation::RuntimeScopeRefV1,
        crate::foundation::ScopeOwnershipGeneration,
    ),
    source: u64,
    active_revision: u64,
    dead: bool,
    original: EntryFacts,
    cells: NativeEntryMovementCells,
}

#[derive(Debug, Default)]
pub(in crate::gameplay_transport) struct AwakenedCreatures(Mutex<Vec<Awakening>>);

impl AwakenedCreatures {
    #[allow(
        clippy::too_many_arguments,
        reason = "the same turn owns the physical actor and its source revision"
    )]
    pub(in crate::gameplay_transport) fn publish(
        &self,
        runtime: &mut ChannelRuntimeV1,
        spawn: &EncounterMonster,
        cells: NativeEntryMovementCells,
        source: u64,
        revision: u64,
        original: EntryFacts,
        transform: impl FnOnce() -> bool,
    ) -> Result<(), CarrierError> {
        let mut active = self.0.lock().map_err(|_| CarrierError::PlanConflict)?;
        let scope = runtime.encounter_owner_scope();
        active.retain(|entry| entry.scope == scope);
        if active.len() >= MAX_ENCOUNTERS || active.iter().any(|entry| entry.source == source) {
            return Err(CarrierError::PlanConflict);
        }
        active
            .try_reserve(1)
            .map_err(|_| CarrierError::AllocationFailed)?;
        let active_revision = revision.checked_add(1).ok_or(CarrierError::PlanConflict)?;
        let at = MovementLocalPosition {
            x: i32::from(spawn.position.x),
            y: i32::from(spawn.position.y),
            floor: -i16::from(spawn.position.floor),
        };
        if !placement(&cells, runtime, at) || !runtime.native_summon_cell_free(at) {
            return Err(CarrierError::PlanConflict);
        }
        let health = runtime.companion_policy(&spawn.creature)?.maximum_health;
        let spawned = runtime.commit_quest_creature_spawn_with_projection(
            &[(spawn.creature.as_str(), health, at)],
            transform,
        )?;
        active.push(Awakening {
            actor: spawned[0],
            scope,
            source,
            active_revision,
            dead: false,
            original,
            cells,
        });
        Ok(())
    }

    pub(super) fn restore(&self, runtime: &ChannelRuntimeV1, actor: Option<ExactActorRef>) {
        let Ok(mut active) = self.0.lock() else {
            return;
        };
        let scope = runtime.encounter_owner_scope();
        active.retain_mut(|entry| {
            if entry.scope != scope {
                return false;
            }
            entry.dead |= actor == Some(entry.actor);
            if !entry.dead {
                return true;
            }
            let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
                entry.cells.index()
            else {
                return false;
            };
            let facts = &index.map().facts;
            facts.object_revision(entry.source) == entry.active_revision
                && !facts.apply_quest_callback(
                    entry.source,
                    entry.active_revision,
                    entry.original,
                    &[],
                    &[],
                    false,
                    None,
                )
        });
    }
}
