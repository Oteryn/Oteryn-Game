//! Exact actors and the selected USE target share one Channel owner and deadline.
use super::*;
use crate::foundation::{RuntimeScopeRefV1, ScopeOwnershipGeneration};
use crate::map::view::EntryFacts;

const MAX_ACTIVE_DISTRACTIONS: usize = 64;

pub(super) struct PreparedDistraction<'callback> {
    rule: &'callback world_callbacks::Distraction,
    monsters: Vec<(&'callback str, i64, MovementLocalPosition)>,
}

pub(super) struct DistractionVisit {
    pub(super) due: u64,
    map: [u8; 32],
    scope: (RuntimeScopeRefV1, ScopeOwnershipGeneration),
    key: u64,
    revision: u64,
    source: TilePos,
    restore: EntryFacts,
    radius: u8,
    landing: Position,
    creatures: Vec<ExactActorRef>,
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn prepare_distraction<'callback>(
        &self,
        runtime: &ChannelRuntimeV1,
        map: &BundleMap,
        callback: &'callback Callback,
        key: u64,
        origin: TilePos,
    ) -> Result<Option<PreparedDistraction<'callback>>, UseDisposition> {
        let Some(rule) = &callback.effects.distraction else {
            return Ok(None);
        };
        if map.facts.base_hidden(key)
            || map
                .overlay
                .base()
                .tile(rule.eject_to.x, rule.eject_to.y, rule.eject_to.floor)
                .is_none()
            || (origin.floor == rule.eject_to.floor
                && origin
                    .x
                    .abs_diff(rule.eject_to.x)
                    .max(origin.y.abs_diff(rule.eject_to.y))
                    <= u16::from(rule.radius))
        {
            return Err(UseDisposition::Rejected);
        }
        let mut pending = self
            .states
            .quest_callbacks
            .distractions
            .lock()
            .map_err(|_| UseDisposition::Rejected)?;
        if pending.len() >= MAX_ACTIVE_DISTRACTIONS
            || pending.iter().any(|visit| visit.key == key)
            || pending.try_reserve(1).is_err()
        {
            return Err(UseDisposition::Exhausted);
        }
        let mut monsters = Vec::new();
        for spawn in &rule.spawns {
            let Some(y) = origin.y.checked_add_signed(spawn.offset_y) else {
                return Err(UseDisposition::Rejected);
            };
            let position = Position {
                x: origin.x,
                y,
                floor: origin.floor,
            };
            let policy = runtime
                .companion_policy(&spawn.creature)
                .map_err(|_| UseDisposition::Rejected)?;
            if map
                .overlay
                .base()
                .tile(position.x, position.y, position.floor)
                .is_none()
            {
                return Err(UseDisposition::Rejected);
            }
            monsters.extend(std::iter::repeat_n(
                (
                    spawn.creature.as_str(),
                    policy.maximum_health,
                    local(position),
                ),
                usize::from(spawn.count),
            ));
        }
        Ok(Some(PreparedDistraction { rule, monsters }))
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the source CAS and spawned actors use the same owner turn"
    )]
    pub(super) fn publish_distraction(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &ChannelSpellStates,
        map: &BundleMap,
        key: u64,
        revision: u64,
        restore: EntryFacts,
        source: TilePos,
        prepared: PreparedDistraction<'_>,
    ) -> bool {
        let Some(due) = self
            .owner_now()
            .get()
            .checked_add(u64::from(prepared.rule.duration_ms) * 1_000)
        else {
            return false;
        };
        let Some(revision_after) = revision.checked_add(1) else {
            return false;
        };
        let Ok(mut pending) = self.states.quest_callbacks.distractions.lock() else {
            return false;
        };
        let hide = || {
            map.facts
                .apply_quest_callback(key, revision, restore, &[key], &[], false, None)
        };
        let creatures =
            match runtime.commit_quest_creature_spawn_with_projection(&prepared.monsters, hide) {
                Ok(creatures) => creatures,
                Err(_) if hide() => Vec::new(),
                Err(_) => return false,
            };
        pending.push(DistractionVisit {
            due,
            map: map.overlay.base().digest(),
            scope: runtime.encounter_owner_scope(),
            key,
            revision: revision_after,
            source,
            restore,
            radius: prepared.rule.radius,
            landing: prepared.rule.eject_to,
            creatures,
        });
        states.owner_wake.notify_one();
        true
    }

    pub(super) fn drain_distractions(&self, runtime: &mut ChannelRuntimeV1, map: &BundleMap) {
        let Ok(mut states) = self.states.spell_states.try_lock() else {
            return;
        };
        let Ok(mut pending) = self.states.quest_callbacks.distractions.lock() else {
            return;
        };
        let now = self.owner_now().get();
        pending.retain(|visit| {
            if visit.scope != runtime.encounter_owner_scope()
                || visit.map != map.overlay.base().digest()
            {
                return false;
            }
            if visit.due > now {
                return true;
            }
            let Ok(census) = runtime.positioned_melee_census() else {
                return true;
            };
            for (actor, expected, session) in census {
                let Some(session) = session else { continue };
                let Some(position) = native(expected.position()) else {
                    continue;
                };
                if position.floor != visit.source.floor
                    || position
                        .x
                        .abs_diff(visit.source.x)
                        .max(position.y.abs_diff(visit.source.y))
                        > u16::from(visit.radius)
                    || states.is_dead(actor)
                    || !states.spell_idle(runtime, actor, session)
                {
                    continue;
                }
                let doors = crate::map::floor::BundleDoors(&map.facts);
                let Ok(proof) = crate::map::floor::prepare_quest_callback_teleport(
                    runtime,
                    self.movement_cells(),
                    actor,
                    session,
                    expected,
                    local(visit.landing),
                    false,
                    &doors,
                ) else {
                    continue;
                };
                let Ok(landed) = runtime.commit_source_step(proof) else {
                    continue;
                };
                states.stage_special_movement_step(
                    runtime,
                    map,
                    actor,
                    session,
                    now,
                    Some(expected.position()),
                    landed.position(),
                    None,
                );
                states.stage_underwater_step(
                    runtime,
                    actor,
                    session,
                    now,
                    underwater(map, expected.position()),
                    underwater(map, landed.position()),
                );
            }
            for actor in &visit.creatures {
                if runtime.contains_live_creature(*actor) {
                    if let Ok(position) = runtime.read_actor_position(*actor)
                        && let Some(position) = native(position.position())
                    {
                        map.facts.emit_movement_effect(position, 11);
                    }
                    if runtime.remove_encounter_actor(*actor).is_err() {
                        return true;
                    }
                }
            }
            // CAS prevents an old distraction from resurrecting a replaced target.
            map.facts.object_revision(visit.key) == visit.revision
                && !map.facts.apply_quest_callback(
                    visit.key,
                    visit.revision,
                    visit.restore,
                    &[visit.key],
                    &[],
                    true,
                    None,
                )
        });
    }
}
