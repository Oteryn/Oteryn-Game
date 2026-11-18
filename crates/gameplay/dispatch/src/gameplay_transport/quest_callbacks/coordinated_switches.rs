//! The final authored lever starts the room deadline on the existing encounter scheduler.
use super::*;
use crate::content::{CollisionClass, LogicalCell};
use crate::map::{facts::QuestCallbackProjection, view::EntryFacts};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

fn lever(map: &BundleMap, position: Position, appearances: [u16; 2]) -> Option<(u64, EntryFacts)> {
    let at = tile(position);
    let base = map.overlay.base().tile(at.x, at.y, at.floor)?;
    let mut found = base
        .ids()
        .iter()
        .zip(base.depths())
        .filter(|(_, depth)| **depth == 0)
        .enumerate()
        .filter_map(|(ordinal, (id, _))| {
            let ordinal = u8::try_from(ordinal).ok()?;
            let key = placement_key(at, ordinal)?;
            let facts = map.facts.base_entry(at, ordinal, *id)?;
            (appearances.contains(&facts.appearance_id)
                && !facts.pickupable
                && !map.facts.base_hidden(key))
            .then_some((key, facts))
        });
    let selected = found.next()?;
    found.next().is_none().then_some(selected)
}

impl ComposedFreshAdmission<'_, '_, '_> {
    // Crystal 201189f8 actions_lord_of_elements_lever.lua:143-180. Each
    // station closes its own walls; any station can be the final activation.
    #[allow(
        clippy::too_many_arguments,
        reason = "one held Channel turn binds the source and roster"
    )]
    pub(super) fn activate_coordinated_switch(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        actor: ExactActorRef,
        session: GameSessionId,
        map: &BundleMap,
        callback: &Callback,
        source: (u64, u64),
        after: EntryFacts,
    ) -> UseDisposition {
        let (Some(window), Some(room), Some(appearances)) = (
            &callback.effects.timed_group_transfer,
            &callback.effects.encounter,
            callback.lever,
        ) else {
            return UseDisposition::Rejected;
        };
        let Some(selected) = window
            .switches
            .iter()
            .find(|switch| Some(switch.lever) == callback.selector.position)
        else {
            return UseDisposition::Rejected;
        };
        if runtime
            .read_actor_position(actor)
            .ok()
            .map(|position| position.position())
            != Some(local(selected.station))
            || runtime.bound_attacker_lease(actor, session).is_err()
            || states.is_dead(actor)
        {
            return UseDisposition::Rejected;
        }
        let Ok(census) = runtime.positioned_melee_census() else {
            return UseDisposition::Rejected;
        };
        let Some(wall) = map
            .facts
            .movement_item_facts(selected.wall_appearance)
            .filter(|wall| !wall.pickupable)
        else {
            return UseDisposition::Rejected;
        };
        let walls: Vec<_> = selected
            .wall_positions()
            .map(|position| (tile(position), wall))
            .collect();
        if walls.iter().any(|&(position, _)| {
            map.facts.house_tile(position)
                || map
                    .overlay
                    .base()
                    .tile(position.x, position.y, position.floor)
                    .is_none()
                || census
                    .iter()
                    .any(|(_, occupant, _)| native(occupant.position()) == Some(position))
        }) {
            return UseDisposition::Occupied;
        }
        let projection = QuestCallbackProjection::SwitchWalls { walls };
        let publish = || {
            map.facts.apply_quest_callback(
                source.0,
                source.1,
                after,
                &[],
                &[],
                false,
                Some(&projection),
            )
        };
        let mut switches = Vec::with_capacity(window.switches.len());
        for switch in &window.switches {
            let Some((key, before)) = lever(map, switch.lever, appearances) else {
                return UseDisposition::Rejected;
            };
            let revision = map.facts.object_revision(key);
            let reset_revision = if switch == selected {
                if (key, revision) != source || before.appearance_id != appearances[0] {
                    return UseDisposition::StaleState;
                }
                let Some(revision) = revision.checked_add(1) else {
                    return UseDisposition::Exhausted;
                };
                revision
            } else {
                if before.appearance_id != appearances[1]
                    || !map.facts.quest_switch_walls_current(key, revision)
                {
                    continue;
                }
                revision
            };
            let Some(restore) = map.facts.quest_callback_replacement(key, appearances[0]) else {
                return UseDisposition::Rejected;
            };
            switches.push((key, reset_revision, restore));
        }
        if switches.len() != window.switches.len() {
            return if publish() {
                UseDisposition::Committed
            } else {
                UseDisposition::Rejected
            };
        }

        let Some(n) = self.active_generation.and_then(|g| g.native_gameplay()) else {
            return UseDisposition::Rejected;
        };
        if n.install_companion_policies(runtime).is_err() {
            return UseDisposition::Rejected;
        }
        let mut monsters = Vec::new();
        for monster in &room.monsters {
            let at = local(monster.position);
            if self.movement_cells.index().lookup(
                self.movement_cells.scope(),
                LogicalCell {
                    x: at.x,
                    y: at.y,
                    z: i32::from(at.floor),
                },
            ) != Ok(CollisionClass::Walkable)
                || map.facts.quest_callback_blocked(
                    map.overlay.base(),
                    tile(monster.position),
                    false,
                )
                || census
                    .iter()
                    .any(|(_, snapshot, _)| snapshot.position() == at)
            {
                return UseDisposition::Occupied;
            }
            let Ok(policy) = runtime.companion_policy(&monster.creature) else {
                return UseDisposition::Rejected;
            };
            monsters.push((monster.creature.as_str(), policy.maximum_health, at));
        }
        let roster = window
            .switches
            .iter()
            .filter_map(|switch| {
                census.iter().find_map(|&(actor, position, session)| {
                    let session = session?;
                    (local(switch.station) == position.position()
                        && !states.is_dead(actor)
                        && runtime.bound_attacker_lease(actor, session).is_ok())
                    .then_some((actor, session))
                })
            })
            .collect();
        let Ok(mut pending) = self.states.quest_callbacks.timed.lock() else {
            return UseDisposition::Rejected;
        };
        if pending.len() >= super::timed_encounter::MAX_TIMED_ENCOUNTERS
            || pending.iter().any(|memo| memo.source_key() == source.0)
            || pending.try_reserve(1).is_err()
        {
            return UseDisposition::Exhausted;
        }
        let Some(mut memo) = super::timed_encounter::TimedEncounter::group(
            runtime,
            map,
            callback,
            source,
            self.owner_now().get(),
            roster,
        ) else {
            return UseDisposition::Exhausted;
        };
        let Ok(spawned) = runtime.commit_quest_creature_spawn_with_projection(&monsters, publish)
        else {
            return UseDisposition::Rejected;
        };
        memo.bind_switches(switches);
        memo.bind_spawns(spawned);
        pending.push(memo);
        states.owner_wake.notify_one();
        UseDisposition::Committed
    }
}
