//! Value-free monster plates derive occupancy from the current creature carrier.
use super::*;
use crate::map::facts::QuestCallbackProjection;
use crate::map::view::MapFacts;

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
