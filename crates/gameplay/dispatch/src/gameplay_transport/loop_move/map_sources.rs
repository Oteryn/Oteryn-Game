//! Immutable map container views; custody is decided by the durable source receipts.
use super::*;

pub(super) async fn observe(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    fence: Option<CurrentCharacterItemFence>,
    target: ItemKey,
) -> Option<ContainerObservation> {
    let fence = fence?;
    let _permit = admission.spell_lane_permit().await?;
    let sessions = super::super::FreshAdmissionStore::from_root(admission.root.clone());
    let current = sessions
        .current_session_at(fence.game_session_id)
        .await
        .ok()?
        .0;
    if current.session_state() != super::super::GameSessionState::Active
        || super::super::item_fence_of(current) != Some(fence)
    {
        return Some(ContainerObservation::Gone);
    }
    let ItemKey::MapBase {
        bundle_digest,
        placement_key,
        reset_epoch,
    } = target
    else {
        return None;
    };
    let source_key = |placement_key| ItemKey::MapBase {
        bundle_digest,
        placement_key,
        reset_epoch,
    };
    let (at, index) = {
        let mut runtime = admission.runtime.lock().await;
        if runtime
            .player_control_facts(actor, fence.game_session_id)
            .is_err()
        {
            return Some(ContainerObservation::Gone);
        }
        let index = pinned_index(&runtime, admission.active_generation)?;
        (
            super::super::bundle_map_position(&mut runtime, admission.movement_cells(), actor)?,
            index,
        )
    };
    let base = at.map.overlay.base();
    if base.digest() != bundle_digest || reset_epoch != admission.reset_epoch {
        return Some(ContainerObservation::Gone);
    }
    let (pos, ordinal) = view::placement_of(placement_key)?;
    if !in_reach(
        (at.at.x, at.at.y, at.at.floor),
        (i32::from(pos.x), i32::from(pos.y), i16::from(pos.floor)),
    ) {
        return Some(ContainerObservation::TooFar);
    }
    let ground = loop_ground(admission, base, Some(index)).await?;
    let mut ancestor = placement_key;
    loop {
        if ground.taken_sources.contains(&ancestor) {
            return Some(ContainerObservation::Gone);
        }
        let Some(node) = at.map.facts.captured_map_item(ancestor) else {
            break;
        };
        ancestor = node.parent;
    }
    let node = at.map.facts.captured_map_item(placement_key);
    let (facts, parent) = if let Some(node) = node {
        (node.facts, Some(source_key(node.parent)))
    } else {
        let tile = base.tile(pos.x, pos.y, pos.floor)?;
        let id = *tile
            .ids()
            .iter()
            .zip(tile.depths())
            .filter(|(_, d)| **d == 0)
            .nth(usize::from(ordinal))?
            .0;
        (at.map.facts.base_entry(pos, ordinal, id)?, None)
    };
    let MapDefinition::Item(reference) = facts.definition else {
        return None;
    };
    let definition = super::super::item_look::typed_definition(index, reference)?;
    let native = admission.active_generation?.native_gameplay()?;
    let children = at.map.facts.captured_map_children(placement_key)?;
    // A captured immovable source may lack physical container-capacity semantics
    // (e.g. a dead tree). Its read-only view exposes exactly the captured slots;
    // it never authorizes insertion or invents a movable container definition.
    let physical_capacity = generation_item_facts(native, &definition)
        .ok()
        .and_then(|facts| facts.container_capacity);
    let capacity = crate::map::facts::captured_source_capacity(
        physical_capacity,
        facts.pickupable,
        children.len(),
        MAX_CONTAINER_VIEW_ENTRIES,
    )?;
    let entries = children
        .into_iter()
        .filter(|child| !ground.taken_sources.contains(&child.placement_key))
        .map(|child| {
            let MapDefinition::Item(reference) = child.facts.definition else {
                return None;
            };
            Some(ViewItem {
                key: source_key(child.placement_key),
                item_definition_ref: reference,
                count: NonZeroU32::new(child.facts.count)?,
                sub_type: child.facts.sub_type,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    let current = sessions
        .current_session_at(fence.game_session_id)
        .await
        .ok()?
        .0;
    if current.session_state() != super::super::GameSessionState::Active
        || super::super::item_fence_of(current) != Some(fence)
        || admission
            .runtime
            .lock()
            .await
            .player_control_facts(actor, fence.game_session_id)
            .is_err()
    {
        return Some(ContainerObservation::Gone);
    }
    Some(ContainerObservation::Container {
        parent,
        capacity,
        entries,
    })
}

/// Nested source addresses never become renderable top-level overlay entries.
pub(super) fn is_top_level(base: &WorldBase, key: u64) -> bool {
    let Some((pos, ordinal)) = view::placement_of(key) else {
        return false;
    };
    base.tile(pos.x, pos.y, pos.floor).is_some_and(|tile| {
        tile.depths().iter().filter(|depth| **depth == 0).count() > usize::from(ordinal)
    })
}
