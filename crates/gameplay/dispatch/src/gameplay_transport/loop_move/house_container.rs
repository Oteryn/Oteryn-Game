//! The existing carried-container reader projected under a current named-House actor.
//! Command 21 and container USE keep the shared view/handle owner; this child grants no
//! Channel Ground or HouseInterior container custody through a carried observation.
use super::super::ComposedFreshAdmission;
use super::super::container_view::ContainerObservation;
use super::super::item_ref_admission::{definition_ref, generation_item_facts, pinned_index};
use super::super::item_view::{ItemKey, ViewItem};
use crate::durability::consume_transform::HouseActionContext;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::{ExactActorRef, RuntimeScopeRefV1};
use oteryn_protocol_oteryn::container_tree::MAX_CONTAINER_VIEW_ENTRIES;
use std::num::NonZeroU32;

pub(super) async fn observe(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    fence: Option<CurrentCharacterItemFence>,
    target: ItemKey,
) -> Option<ContainerObservation> {
    let fence = fence?;
    if !matches!(fence.runtime_scope, RuntimeScopeRefV1::Instance { .. }) {
        return None;
    }
    let ItemKey::Instance(container) = target else {
        return Some(ContainerObservation::Gone);
    };
    let _lane = admission.spell_lane_permit().await?;
    let (context, index) = {
        let runtime = admission.runtime.lock().await;
        let context = match HouseActionContext::from_native_owner(
            &runtime,
            actor,
            fence.game_session_id,
            fence,
        ) {
            Ok(context) => context,
            Err(_) => return Some(ContainerObservation::Gone),
        };
        (
            context,
            pinned_index(&runtime, admission.active_generation)?,
        )
    };
    if !admission
        .root
        .house_action_current(admission.holder, &context)
        .await
        .ok()?
    {
        return Some(ContainerObservation::Gone);
    }
    // The item owner already proves the bounded same-Character carried graph.
    // A missing carried container must never fall through to a Channel Ground read.
    let read = admission
        .root
        .read_item_loop_container(admission.character, fence.character_id, container)
        .await
        .ok()?;
    let Some(read) = read else {
        return Some(ContainerObservation::Gone);
    };
    if !admission
        .root
        .house_action_current(admission.holder, &context)
        .await
        .ok()?
    {
        return Some(ContainerObservation::Gone);
    }
    {
        let runtime = admission.runtime.lock().await;
        if HouseActionContext::from_native_owner(&runtime, actor, fence.game_session_id, fence)
            .is_err()
            || pinned_index(&runtime, admission.active_generation).is_none()
        {
            return Some(ContainerObservation::Gone);
        }
    }
    let native = admission.active_generation?.native_gameplay()?;
    let capacity = generation_item_facts(native, &read.container.definition)
        .ok()
        .and_then(|facts| facts.container_capacity)
        .and_then(|capacity| u8::try_from(capacity).ok())
        .filter(|capacity| (1..=MAX_CONTAINER_VIEW_ENTRIES).contains(&usize::from(*capacity)));
    let Some(capacity) = capacity else {
        return Some(ContainerObservation::NotAContainer);
    };
    let entries = read
        .entries
        .iter()
        .take(usize::from(capacity))
        .map(|entry| {
            Some(ViewItem {
                key: ItemKey::Instance(entry.item.item_instance_id),
                item_definition_ref: definition_ref(index, &entry.item.definition)?,
                count: NonZeroU32::new(entry.item.quantity)?,
                sub_type: read
                    .fluids
                    .get(&entry.item.item_instance_id)
                    .copied()
                    .map_or(0, u32::from),
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(ContainerObservation::Container {
        parent: read.parent.map(ItemKey::Instance),
        capacity,
        entries,
    })
}
