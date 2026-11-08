//! Named-House containers retain the existing container handles and nested item owner.
//! A carried read grants no House custody; current House roots have their own fenced read.
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
    house_capacity_v2: bool,
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
    // Preserve carried custody; only the actual same-House owner may supply a
    // separate bounded House graph. Neither path falls through to Channel Ground.
    let carried = admission
        .root
        .read_item_loop_container(admission.character, fence.character_id, container)
        .await
        .ok()?;
    let (read, is_house) = match carried {
        Some(read) => (Some(read), false),
        None => (
            admission
                .root
                .read_house_item_loop_container_with_capacity(
                    admission.character,
                    admission.holder,
                    &context,
                    container,
                    house_capacity_v2,
                )
                .await
                .ok()?,
            true,
        ),
    };
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
        if !HouseActionContext::from_native_owner(&runtime, actor, fence.game_session_id, fence)
            .is_ok_and(|current| current.position() == context.position())
            || pinned_index(&runtime, admission.active_generation).is_none()
        {
            return Some(ContainerObservation::Gone);
        }
    }
    let native = admission.active_generation?.native_gameplay()?;
    let limit = if is_house && house_capacity_v2 {
        oteryn_protocol_oteryn::container_tree::MAX_HOUSE_CONTAINER_VIEW_ENTRIES
    } else {
        MAX_CONTAINER_VIEW_ENTRIES
    };
    let capacity = generation_item_facts(native, &read.container.definition)
        .ok()
        .and_then(|facts| facts.container_capacity)
        .and_then(|capacity| u8::try_from(capacity).ok())
        .filter(|capacity| (1..=limit).contains(&usize::from(*capacity)));
    let Some(capacity) = capacity else {
        return Some(ContainerObservation::NotAContainer);
    };
    if read.entries.len() > usize::from(capacity) {
        return Some(ContainerObservation::Gone);
    }
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

pub(super) async fn move_item(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: super::super::connection::UseCommand,
    request: super::super::item_move::LoopMove,
    permit: &crate::durability::spell_owner_commit::SpellLanePermit,
    house_capacity_v2: bool,
) -> super::super::item_move::LoopMoveResult {
    use super::super::item_move::{LoopMoveRefusal, LoopMoveResult, LoopTarget};
    use crate::durability::item_loop_move::{LoopDestination, LoopMoveRequest};
    let refused = LoopMoveResult::Refused;
    let Some(fence) = command.item_fence else {
        return refused(LoopMoveRefusal::Rejected);
    };
    let ItemKey::Instance(item) = request.item else {
        return refused(LoopMoveRefusal::NotSupported);
    };
    let (context, native) = {
        let runtime = admission.runtime.lock().await;
        let states = admission.states.spell_states.lock().await;
        if !runtime
            .house_player_control_facts(actor, command.game_session_id)
            .is_ok_and(|facts| facts.control_loss.is_none())
        {
            return refused(LoopMoveRefusal::Rejected);
        }
        let Some(native) = super::super::equipment_move::native_for_move(
            &runtime,
            admission.active_generation,
            states.is_dead(actor),
            states.has_pending_spell_commit(actor, command.game_session_id),
        ) else {
            return refused(LoopMoveRefusal::Rejected);
        };
        let Ok(context) =
            HouseActionContext::from_native_owner(&runtime, actor, command.game_session_id, fence)
        else {
            return refused(LoopMoveRefusal::Rejected);
        };
        (context, native)
    };
    let destination = match request.target {
        LoopTarget::MainBackpack => LoopDestination::MainBackpack,
        LoopTarget::Container(ItemKey::Instance(parent)) => LoopDestination::Container(parent),
        _ => return refused(LoopMoveRefusal::NotSupported),
    };
    let Ok(id) = crate::foundation::CommandId::new(command.command_id) else {
        return refused(LoopMoveRefusal::Rejected);
    };
    let result = admission
        .root
        .commit_house_item_loop_move_with_capacity(
            permit,
            admission.character,
            admission.holder,
            &context,
            crate::foundation::CommandRef::new(command.game_session_id, id),
            &super::super::item_ref_admission::NativeItemWeights(native)
                .with_wheel(admission.premium_coordinator),
            LoopMoveRequest {
                item,
                count: request.count,
                pickup_cell: None,
                destination,
            },
            house_capacity_v2,
        )
        .await;
    if matches!(result, Ok(Ok(_))) {
        admission
            .states
            .attack
            .lock()
            .await
            .forget_weapon(actor, command.game_session_id);
    }
    match result {
        Ok(Ok(_)) => LoopMoveResult::Committed,
        Ok(Err(error)) => refused(error),
        Err(_) => LoopMoveResult::Unknown,
    }
}
