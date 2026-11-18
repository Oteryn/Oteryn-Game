//! A pillar is restored only after the actor it spawned has a projected death.
use super::*;
use crate::map::boot::BundleMap;
use crate::map::view::{EntryFacts, MapFacts};
use oteryn_gameplay_quests::quest::world_callbacks::EncounterMonster;
use oteryn_protocol_oteryn::world_object::UseDisposition;

#[derive(Debug)]
pub(super) struct Incarnation {
    key: u64,
    revision: u64,
    actor: ExactActorRef,
    dead: bool,
    scope: (
        crate::foundation::RuntimeScopeRefV1,
        crate::foundation::ScopeOwnershipGeneration,
    ),
    cells: crate::content::NativeEntryMovementCells,
}

impl Registry {
    #[allow(
        clippy::too_many_arguments,
        reason = "one exact pillar CAS owns the native spawn"
    )]
    pub(in crate::gameplay_transport) fn incarnation(
        &self,
        runtime: &mut ChannelRuntimeV1,
        map: &BundleMap,
        cells: &crate::content::NativeEntryMovementCells,
        monster: &EncounterMonster,
        key: u64,
        revision: u64,
        after: EntryFacts,
        preflight: bool,
    ) -> UseDisposition {
        let Ok(mut rooms) = self.incarnations.lock() else {
            return UseDisposition::Rejected;
        };
        let scope = runtime.encounter_owner_scope();
        rooms.retain(|room| room.scope == scope);
        if rooms.len() >= 4 || rooms.iter().any(|room| room.key == key) {
            return UseDisposition::Occupied;
        }
        let at = MovementLocalPosition {
            x: i32::from(monster.position.x),
            y: i32::from(monster.position.y),
            floor: -i16::from(monster.position.floor),
        };
        let Ok(policy) = runtime.companion_policy(&monster.creature) else {
            return UseDisposition::Rejected;
        };
        if !placement(cells, runtime, at)
            || map.facts.quest_callback_replacement(key, 11427).is_none()
            || runtime.positioned_melee_census().map_or(true, |census| {
                census
                    .iter()
                    .any(|(_, position, _)| position.position() == at)
            })
        {
            return UseDisposition::Occupied;
        }
        let Some(next_revision) = revision.checked_add(1) else {
            return UseDisposition::Exhausted;
        };
        if preflight {
            return UseDisposition::Committed;
        }
        let Ok(actors) = runtime.commit_quest_creature_spawn_with_projection(
            &[(&monster.creature, policy.maximum_health, at)],
            || {
                map.facts
                    .apply_quest_callback(key, revision, after, &[], &[], false, None)
            },
        ) else {
            return UseDisposition::Rejected;
        };
        rooms.push(Incarnation {
            key,
            revision: next_revision,
            actor: actors[0],
            dead: false,
            scope,
            cells: cells.clone(),
        });
        UseDisposition::Committed
    }

    pub(super) fn incarnation_death(&self, actor: ExactActorRef) {
        if let Ok(mut rooms) = self.incarnations.lock()
            && let Some(room) = rooms.iter_mut().find(|room| room.actor == actor)
        {
            room.dead = true;
        }
    }

    pub(in crate::gameplay_transport) fn reset_incarnations(&self, runtime: &ChannelRuntimeV1) {
        let Ok(mut rooms) = self.incarnations.lock() else {
            return;
        };
        rooms.retain(|room| {
            if room.scope != runtime.encounter_owner_scope()
                || room.cells.scope().generation_digest
                    != runtime.content_pin().server_artifact_digest()
            {
                return false;
            }
            if !room.dead {
                return true;
            }
            let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
                room.cells.index()
            else {
                return false;
            };
            let facts = &index.map().facts;
            facts.object_revision(room.key) == room.revision
                && !facts
                    .quest_callback_replacement(room.key, 11427)
                    .is_some_and(|before| {
                        facts.apply_quest_callback(
                            room.key,
                            room.revision,
                            before,
                            &[],
                            &[],
                            true,
                            None,
                        )
                    })
        });
    }
}
