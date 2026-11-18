//! Value-free monster plates derive occupancy from the current creature carrier.
use super::*;
use crate::map::facts::QuestCallbackProjection;
use crate::map::view::MapFacts;

/// Crystal 201189f8, ferumbras_ascension/movements_lovely_monsters.lua:14-28
/// and the_first_dragon/movements_heaven_blossom.lua:3-14.
pub(in crate::gameplay_transport) fn settle_monster_step(
    runtime: &mut ChannelRuntimeV1,
    map: &BundleMap,
    step: crate::map::facts::MonsterStep,
    entries: &[(u64, crate::map::view::EntryFacts)],
    registry: &[Callback],
) -> bool {
    let Some(position) = native(step.to) else {
        return false;
    };
    if map.overlay.runtime_scope() != runtime.binding().runtime_scope()
        || map.facts.house_tile(position)
        || !runtime
            .read_actor_position(step.actor)
            .is_ok_and(|at| at.position() == step.to)
    {
        return false;
    }
    let Some((effect, key, before)) = registry.iter().find_map(|callback| {
        let effect = callback.effects.monster_step.as_ref()?;
        if callback.selector.position.map(tile) != Some(position)
            || !runtime.matches_live_creature_identity(step.actor, effect.creature.as_bytes())
            || !world_requirements_match(map, callback)
        {
            return None;
        }
        let &(key, before) = entries
            .iter()
            .find(|(key, _)| !map.facts.base_hidden(*key) && matches(map, callback, *key))?;
        Some((effect, key, before))
    }) else {
        return false;
    };
    let Some(next) = effect.replace_with.as_deref() else {
        return runtime.remove_encounter_actor(step.actor).is_ok();
    };
    let Ok(policy) = runtime.companion_policy(next) else {
        return false;
    };
    let revision = map.facts.object_revision(key);
    runtime
        .commit_quest_creature_replacement(
            &[step.actor],
            &[(next, policy.maximum_health, step.to)],
            || {
                !effect.consume_source
                    || map.facts.apply_quest_callback(
                        key,
                        revision,
                        before,
                        &[],
                        &[],
                        false,
                        Some(&QuestCallbackProjection::Scene {
                            mutations: Vec::new(),
                            remove_self: true,
                        }),
                    )
            },
        )
        .is_ok()
}

pub(in crate::gameplay_transport) fn settle_monster_puzzles(
    runtime: &ChannelRuntimeV1,
    map: &BundleMap,
    registry: &[Callback],
) {
    // Refuse a partial census. Reconciliation also closes plates after death/despawn.
    let Ok(actors) = runtime.current_live_creature_census() else {
        return;
    };
    let occupied: std::collections::BTreeSet<_> = actors
        .into_iter()
        .filter_map(|actor| runtime.read_actor_position(actor).ok())
        .filter_map(|position| native(position.position()))
        .collect();
    for callback in registry
        .iter()
        .filter(|callback| callback.actor_kind == world_callbacks::ActorKind::Monster)
    {
        let Some(position) = callback.selector.position.map(tile) else {
            continue;
        };
        let close = callback.trigger == Trigger::StepOut;
        if close == occupied.contains(&position)
            || (!close && !world_requirements_match(map, callback))
        {
            continue;
        }
        let Some(base) = map
            .overlay
            .base()
            .tile(position.x, position.y, position.floor)
        else {
            continue;
        };
        for (ordinal, (id, _)) in base
            .ids()
            .iter()
            .zip(base.depths())
            .filter(|(_, depth)| **depth == 0)
            .enumerate()
        {
            let Ok(ordinal) = u8::try_from(ordinal) else {
                continue;
            };
            let Some(key) = placement_key(position, ordinal) else {
                continue;
            };
            if !matches(map, callback, key) || map.facts.house_tile(position) {
                continue;
            }
            let Some(before) = map.facts.base_entry(position, ordinal, *id) else {
                continue;
            };
            let Some(appearance) = callback.effects.transform else {
                continue;
            };
            if before.appearance_id == appearance {
                continue;
            }
            let Some(after) = map.facts.quest_callback_replacement(key, appearance) else {
                continue;
            };
            if before.pickupable || after.pickupable {
                continue;
            }
            let projection = if let Some(presence) = &callback.effects.item_presence {
                let pos = tile(presence.position);
                let Some(entry) = map.facts.movement_item_facts(presence.appearance) else {
                    continue;
                };
                if entry.pickupable
                    || !crate::map::floor::is_ladder_appearance(entry.appearance_id)
                    || map.facts.house_tile(pos)
                    || map.overlay.base().tile(pos.x, pos.y, pos.floor).is_none()
                {
                    continue;
                }
                Some(QuestCallbackProjection::ItemPresence {
                    position: pos,
                    entry,
                })
            } else {
                None
            };
            let revision = map.facts.object_revision(key);
            let _ = map.facts.apply_quest_callback(
                key,
                revision,
                after,
                &[],
                &[],
                close,
                projection.as_ref(),
            );
        }
    }
}

pub(in crate::gameplay_transport) fn settle_authored_monster_puzzles(
    runtime: &ChannelRuntimeV1,
    map: &BundleMap,
) {
    settle_monster_puzzles(runtime, map, callbacks());
}
