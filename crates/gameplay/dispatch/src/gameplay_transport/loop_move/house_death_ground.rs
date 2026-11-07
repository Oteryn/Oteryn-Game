//! Existing command9, scoped to a real named-House death Ground root.
use super::*;
use crate::durability::consume_transform::HouseActionContext;

pub(super) async fn try_move(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    request: LoopMove,
) -> Option<LoopMoveResult> {
    let fence = command.item_fence?;
    let context = {
        let runtime = admission.runtime.lock().await;
        match HouseActionContext::from_native_owner(&runtime, actor, command.game_session_id, fence)
        {
            Ok(context) => context,
            Err(_) => return None,
        }
    };
    let refused = || LoopMoveResult::Refused(LoopMoveRefusal::Rejected);
    let Some(permit) = admission.spell_lane_permit().await else {
        return Some(LoopMoveResult::Unknown);
    };
    let ItemKey::Instance(item) = request.item else {
        return Some(refused());
    };
    if !admission
        .states
        .player_trades
        .physical(command.game_session_id, Some(item))
    {
        return Some(refused());
    }
    let native = {
        let runtime = admission.runtime.lock().await;
        let states = admission.states.spell_states.lock().await;
        let native = native_for_move(
            &runtime,
            admission.active_generation,
            states.is_dead(actor),
            states.has_pending_spell_commit(actor, command.game_session_id),
        );
        if !HouseActionContext::from_native_owner(&runtime, actor, command.game_session_id, fence)
            .is_ok_and(|current| {
                current.position() == context.position() && current.house() == context.house()
            })
        {
            return Some(refused());
        }
        native
    };
    let Some(native) = native else {
        return Some(refused());
    };
    let roots = match admission
        .root
        .read_house_death_ground(admission.holder, &context)
        .await
    {
        Ok(Some(roots)) => roots,
        Ok(None) => return Some(refused()),
        Err(_) => return Some(LoopMoveResult::Unknown),
    };
    let root = roots
        .into_iter()
        .find(|root| root.item.item_instance_id == item)?;
    let Ok(cell) =
        crate::durability::spell_items_abi::decode_ground_cell(&root.item.ground.spatial_position)
    else {
        return Some(refused());
    };
    let at = context.position();
    if cell.z != at[2] || cell.x.abs_diff(at[0]) > 1 || cell.y.abs_diff(at[1]) > 1 {
        return Some(LoopMoveResult::TooFar);
    }
    let destination = match request.target {
        LoopTarget::MainBackpack => LoopDestination::MainBackpack,
        LoopTarget::Container(ItemKey::Instance(container)) => {
            LoopDestination::Container(container)
        }
        _ => return Some(refused()),
    };
    {
        let runtime = admission.runtime.lock().await;
        if !HouseActionContext::from_native_owner(&runtime, actor, command.game_session_id, fence)
            .is_ok_and(|current| {
                current.position() == context.position() && current.house() == context.house()
            })
        {
            return Some(refused());
        }
    }
    let Ok(id) = CommandId::new(command.command_id) else {
        return Some(refused());
    };
    let result = admission
        .root
        .commit_house_death_ground_pickup(
            &permit,
            admission.character,
            admission.holder,
            &fence,
            CommandRef::new(command.game_session_id, id),
            &NativeItemWeights(native).with_wheel(admission.premium_coordinator),
            LoopMoveRequest {
                item,
                count: request.count,
                pickup_cell: Some(root.item.ground.spatial_position),
                destination,
            },
            context,
        )
        .await;
    Some(match result {
        Ok(Ok(_)) => LoopMoveResult::Committed,
        Ok(Err(refusal)) => LoopMoveResult::Refused(refusal),
        Err(_) => LoopMoveResult::Unknown,
    })
}
