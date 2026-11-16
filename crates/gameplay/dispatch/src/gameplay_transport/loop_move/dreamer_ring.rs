//! Source UID2244 is selected from the actual composed destination of an item drop.
use super::*;
use crate::durability::item_loop_move::DreamerRingTarget;
use crate::durability::terrain_tool::TerrainScope;
use crate::map::boot::BundleMap;
use crate::map::view::EntryFacts;

pub(super) struct Prepared {
    pub(super) target: DreamerRingTarget,
    source: u64,
    revision: u64,
    facts: EntryFacts,
    trees: Vec<(u64, TilePos)>,
}

pub(super) fn target(
    map: &BundleMap,
    to: WorldTilePosition,
    content: [u8; 32],
    epoch: u64,
) -> Option<Prepared> {
    let pos = TilePos {
        x: to.x.try_into().ok()?,
        y: to.y.try_into().ok()?,
        floor: to.floor.try_into().ok()?,
    };
    let pinned = map.facts.pinned();
    if pinned.house_tile(pos) {
        return None;
    }
    let tile = view::compose_tile(&map.overlay, &pinned, pos).ok()??;
    let selected = tile
        .entries
        .iter()
        .filter_map(|entry| {
            let view::MapEntryKey::Base(key) = entry.source.key() else {
                return None;
            };
            map.facts
                .quest_callback_matches(key, Some(2244), None, None)
                .then_some((key, entry.facts))
        })
        .collect::<Vec<_>>();
    let [(source, facts)] = selected.as_slice() else {
        return None;
    };
    let revision = map.facts.object_revision(*source);
    let mut trees = Vec::new();
    let mut trees_before = [0; 3];
    for (index, position) in oteryn_gameplay_quests::quest::dreamer_death_ring::TREES
        .into_iter()
        .enumerate()
    {
        let pos = TilePos {
            x: position.x,
            y: position.y,
            floor: position.floor,
        };
        if pinned.house_tile(pos) {
            return None;
        }
        let tile = map.overlay.base().tile(pos.x, pos.y, pos.floor)?;
        let candidates = tile
            .ids()
            .iter()
            .zip(tile.depths())
            .filter(|(_, depth)| **depth == 0)
            .enumerate()
            .filter_map(|(ordinal, (id, _))| {
                let ordinal = u8::try_from(ordinal).ok()?;
                let key =
                    oteryn_world_bundle::bundle::placement_key(pos.floor, pos.x, pos.y, ordinal)?;
                let facts = map.facts.base_entry(pos, ordinal, *id)?;
                (facts.appearance_id == 3636).then_some(key)
            })
            .collect::<Vec<_>>();
        let [key] = candidates.as_slice() else {
            return None;
        };
        if !pinned.base_hidden(*key) {
            trees_before[index] = 3636;
            trees.push((*key, pos));
        }
    }
    if trees.is_empty()
        || !map
            .facts
            .quest_callback_capacity(*source, revision, trees.len())
    {
        return None;
    }
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&source.to_be_bytes());
    Some(Prepared {
        source: *source,
        revision,
        facts: *facts,
        trees,
        target: DreamerRingTarget {
            scope: TerrainScope {
                map_revision: map_revision(map.overlay.base()),
                content_generation: content,
                reset_epoch: epoch.try_into().ok()?,
            },
            placement,
            expected_revision: revision.try_into().ok()?,
            trees_before,
        },
    })
}

pub(super) fn publish(map: &BundleMap, prepared: &Prepared) -> bool {
    let keys = prepared
        .trees
        .iter()
        .map(|(key, _)| *key)
        .collect::<Vec<_>>();
    if !map.facts.apply_quest_callback(
        prepared.source,
        prepared.revision,
        prepared.facts,
        &keys,
        &[],
        false,
        None,
    ) {
        return false;
    }
    for (_, position) in &prepared.trees {
        map.facts.emit_movement_effect(*position, 13);
    }
    true
}

/// Publishes a committed offering whose acknowledgement was lost. Only the exact durable
/// target composed again from the unchanged source publishes; a published source hides its
/// trees and composes none, so a later replay presents nothing. False is an unknown result.
pub(super) async fn reconcile(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    character: crate::domain::CharacterId,
    command: CommandRef,
) -> bool {
    let Ok(offering) = admission
        .root
        .read_dreamer_ring_offering(admission.character, character, command)
        .await
    else {
        return false;
    };
    let Some(offering) = offering else {
        return true;
    };
    let content = admission
        .runtime
        .lock()
        .await
        .content_pin()
        .server_artifact_digest();
    let Some(map) = admission.bundle_map() else {
        return false;
    };
    let Some((pos, _)) = offering
        .placement
        .get(32..)
        .and_then(|key| <[u8; 8]>::try_from(key).ok())
        .and_then(|key| view::placement_of(u64::from_be_bytes(key)))
    else {
        return false;
    };
    let to = WorldTilePosition {
        x: i32::from(pos.x),
        y: i32::from(pos.y),
        floor: i16::from(pos.floor),
    };
    let Some(prepared) = target(map, to, content, admission.reset_epoch) else {
        return true;
    };
    let current = &prepared.target;
    if current.placement != offering.placement
        || current.expected_revision != offering.expected_revision
        || current.trees_before != offering.trees_before
        || current.scope.reset_epoch != offering.scope.reset_epoch
        || current.scope.map_revision != offering.scope.map_revision
        || current.scope.content_generation != offering.scope.content_generation
    {
        return true;
    }
    publish(map, &prepared)
}

#[cfg(test)]
static INTERRUPT_AFTER_COMMIT: std::sync::Mutex<Option<(crate::foundation::GameSessionId, u64)>> =
    std::sync::Mutex::new(None);

#[cfg(test)]
impl ComposedFreshAdmission<'_, '_, '_> {
    /// Drop only the acknowledgement of this test's exact committed offering, before
    /// publication. Production reaches the same state on an actual unavailable result.
    pub(in crate::gameplay_transport) fn fixture_interrupt_dreamer_ring_commit(
        &self,
        session: crate::foundation::GameSessionId,
        command: u64,
    ) -> Result<(), String> {
        let mut expected = INTERRUPT_AFTER_COMMIT.lock().map_err(|_| "fault lock")?;
        if command == 0 || expected.is_some() {
            return Err("fault already armed or invalid command".into());
        }
        *expected = Some((session, command));
        Ok(())
    }
}

#[cfg(test)]
pub(super) fn interrupted_after_commit(
    session: crate::foundation::GameSessionId,
    command: u64,
) -> bool {
    let Ok(mut expected) = INTERRUPT_AFTER_COMMIT.lock() else {
        return false;
    };
    if *expected != Some((session, command)) {
        return false;
    }
    *expected = None;
    true
}
