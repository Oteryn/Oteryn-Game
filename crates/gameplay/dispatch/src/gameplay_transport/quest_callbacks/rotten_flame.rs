//! Actual static WorldObject selection and fixed source visual CAS, no Item alias.
use super::*;
use crate::durability::quest_state::rotten_flame::{
    FlameVisualRef, RottenFlameRead, RottenFlameUse,
};
use oteryn_gameplay_quests::quest::rotten_blood::flames;

fn selected(map: &BundleMap, key: u64) -> Option<(u16, MovementLocalPosition, u64, u64)> {
    let (at, ordinal) = crate::map::view::placement_of(key)?;
    if map.facts.base_hidden(key) {
        return None;
    }
    let tile = map.overlay.base().tile(at.x, at.y, at.floor)?;
    let id = tile
        .ids()
        .iter()
        .zip(tile.depths())
        .filter(|(_, d)| **d == 0)
        .nth(usize::from(ordinal))
        .map(|(id, _)| *id)?;
    let facts = map.facts.base_entry(at, ordinal, id)?;
    Some((
        facts.appearance_id,
        MovementLocalPosition {
            x: i32::from(at.x),
            y: i32::from(at.y),
            floor: -i16::from(at.floor),
        },
        key,
        map.facts.object_revision(key),
    ))
}
struct Current<'a> {
    map: &'a BundleMap,
    key: u64,
    revision: u64,
    context: &'a QuestWorldContext,
}
impl RottenFlameRead for Current<'_> {
    fn current(&self, runtime: &ChannelRuntimeV1) -> bool {
        map_revision(self.map) == self.context.map_revision
            && self.map.facts.object_revision(self.key) == self.revision
            && runtime.content_pin().frame_binding_digest().as_slice()
                == self.context.placement_context.as_slice()
    }
    fn selected(&self) -> Option<(u16, MovementLocalPosition, u64, u64)> {
        selected(self.map, self.key)
    }
    fn fixed_visuals(&self, source: &flames::FlameSource) -> Option<[Option<FlameVisualRef>; 2]> {
        let mut output = [None, None];
        for (role, (position, before, after)) in [
            (source.flame, source.inactive, source.active),
            (
                source.pillar,
                flames::PILLAR_INACTIVE,
                flames::PILLAR_ACTIVE,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let at = TilePos {
                x: position[0].try_into().ok()?,
                y: position[1].try_into().ok()?,
                floor: i8::try_from(position[2]).ok()?.checked_neg()?,
            };
            // The actual source capture/whole World must cover each fixed tile;
            // a missing capture region is not evidence of Lua's nil branch.
            let tile = self.map.overlay.base().tile(at.x, at.y, at.floor)?;
            for (ordinal, _) in tile
                .ids()
                .iter()
                .zip(tile.depths())
                .filter(|(_, d)| **d == 0)
                .enumerate()
            {
                let key = placement_key(at, u8::try_from(ordinal).ok()?)?;
                let Some((appearance, position, _, revision)) = selected(self.map, key) else {
                    continue;
                };
                if appearance == before {
                    self.map.facts.movement_item_facts(after)?;
                    output[role] = Some(FlameVisualRef {
                        key,
                        revision,
                        position,
                        before,
                        after,
                    });
                    break;
                }
            }
        }
        Some(output)
    }
}
fn map_revision(map: &BundleMap) -> String {
    crate::map::overlay::map_revision(map.overlay.base())
}

#[allow(
    clippy::too_many_arguments,
    reason = "source qualification independently fences owner, runtime, actor, command, world context, map and selected object key/revision"
)]
pub(super) fn qualify(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    cause: QuestCause,
    context: QuestWorldContext,
    map: &BundleMap,
    key: u64,
    revision: u64,
) -> Option<RottenFlameUse> {
    let QuestCause::Use(command) = cause else {
        return None;
    };
    let native = owner.active_generation?.native_gameplay()?;
    let reader = Current {
        map,
        key,
        revision,
        context: &context,
    };
    RottenFlameUse::from_current_owner(runtime, native, actor, command, context.clone(), &reader)
}
pub(super) fn publish(runtime: &ChannelRuntimeV1, map: &BundleMap, input: &RottenFlameUse) -> bool {
    if !input.current_scope(runtime) {
        return false;
    }
    // Existing Channel owner lock serializes these same fixed World mutations.
    // Exact post-revisions make an interrupted post-commit replay idempotent.
    for visual in input.visual_changes().iter().flatten() {
        let Some((appearance, _, _, revision)) = selected(map, visual.key) else {
            return false;
        };
        if revision == visual.revision + 1 && appearance == visual.after {
            continue;
        }
        if revision != visual.revision
            || appearance != visual.before
            || map.facts.movement_item_facts(visual.after).is_none()
        {
            return false;
        }
    }
    for visual in input.visual_changes().iter().flatten() {
        if map.facts.object_revision(visual.key) == visual.revision
            && !map.facts.transform_movement_item(visual.key, visual.after)
        {
            return false;
        }
    }
    true
}

struct ResetCurrent<'a> {
    map: &'a BundleMap,
    origin: crate::durability::quest_state::rotten_flame::BakPhaseOrigin,
}
impl crate::durability::quest_state::rotten_flame::phase::BakPublicResetRead for ResetCurrent<'_> {
    fn current(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.origin.current(runtime)
    }
    fn reset_slots(&self) -> Option<[Option<FlameVisualRef>; 8]> {
        let mut result = [None; 8];
        for (i, source) in flames::FLAMES.iter().enumerate() {
            for (role, (position, before, after)) in [
                (source.flame, source.active, source.inactive),
                (
                    source.pillar,
                    flames::PILLAR_ACTIVE,
                    flames::PILLAR_INACTIVE,
                ),
            ]
            .into_iter()
            .enumerate()
            {
                let at = TilePos {
                    x: position[0].try_into().ok()?,
                    y: position[1].try_into().ok()?,
                    floor: i8::try_from(position[2]).ok()?.checked_neg()?,
                };
                // All eight exact source tiles must be in the real origin view.
                // An absent active Item on a covered tile is the donor's nil branch.
                let tile = self.map.overlay.base().tile(at.x, at.y, at.floor)?;
                for (ordinal, _) in tile
                    .ids()
                    .iter()
                    .zip(tile.depths())
                    .filter(|(_, d)| **d == 0)
                    .enumerate()
                {
                    let key = placement_key(at, u8::try_from(ordinal).ok()?)?;
                    let Some((appearance, position, _, revision)) = selected(self.map, key) else {
                        continue;
                    };
                    if appearance == before {
                        if result[i * 2 + role].is_some() {
                            return None;
                        }
                        self.map.facts.movement_item_facts(after)?;
                        result[i * 2 + role] = Some(FlameVisualRef {
                            key,
                            revision,
                            position,
                            before,
                            after,
                        });
                    }
                }
            }
        }
        Some(result)
    }
    fn matches_after(&self, slots: &[Option<FlameVisualRef>; 8]) -> bool {
        for visual in slots.iter().flatten() {
            let Some((appearance, _, _, revision)) = selected(self.map, visual.key) else {
                return false;
            };
            if appearance != visual.after || revision != visual.revision + 1 {
                return false;
            }
        }
        // Recheck covered nil slots and exclude another active duplicate.
        self.reset_slots()
            .is_some_and(|current| current.iter().all(Option::is_none))
    }
}
/// Called while both canonical scope lanes and their ordered runtime owners are
/// held. The caller retains the returned opaque publication BEFORE its next await.
pub(super) fn prepare_public_reset(
    runtime: &ChannelRuntimeV1,
    map: &BundleMap,
    prepared: crate::durability::quest_state::rotten_flame::PreparedBakEssences,
) -> Option<crate::durability::quest_state::rotten_flame::phase::BakPhasePublication> {
    use crate::durability::quest_state::rotten_flame::phase::{BakPublicReset, BakPublicResetRead};
    let reader = ResetCurrent {
        map,
        origin: prepared.origin()?,
    };
    let reset = BakPublicReset::from_current_owner(runtime, prepared, &reader)?;
    let changes = reset.visual_changes();
    let count = changes.iter().flatten().count();
    if let Some(first) = changes.iter().flatten().next()
        && !map
            .facts
            .quest_callback_capacity(first.key, first.revision, count)
    {
        return None;
    }
    // Preflight every target before the first map mutation.
    for visual in changes.iter().flatten() {
        let (appearance, _, _, revision) = selected(map, visual.key)?;
        if appearance != visual.before || revision != visual.revision || revision == u64::MAX {
            return None;
        }
    }
    for visual in changes.iter().flatten() {
        if !map.facts.transform_movement_item(visual.key, visual.after) {
            return None;
        }
    }
    if !reader.current(runtime) {
        return None;
    }
    reset.published(runtime, &reader)
}
