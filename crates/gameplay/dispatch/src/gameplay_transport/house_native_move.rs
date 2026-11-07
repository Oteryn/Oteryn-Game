//! Native command9 delegates plain House custody to the mature House writer.
use super::{
    ComposedFreshAdmission,
    connection::UseCommand,
    item_move::{LoopMoveRefusal, LoopMoveResult},
    item_view::ItemKey,
};
use crate::{
    durability::{
        consume_transform::HouseActionContext,
        house_interior::{
            HouseItemAction, HouseItemOutcome, NativeHouseMoveContext, NativeHouseRecovery,
            NativeHouseTarget,
        },
    },
    foundation::{CommandId, CommandRef, ExactActorRef},
};
use oteryn_protocol_oteryn::item_view::ItemMoveDestination;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn native_house_item_move(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        payload: &[u8],
        source: Option<ItemKey>,
        destination: ItemMoveDestination,
        count: Option<u32>,
    ) -> Option<LoopMoveResult> {
        let fence = command.item_fence?;
        let context = {
            let runtime = self.runtime.lock().await;
            HouseActionContext::from_native_owner(&runtime, actor, command.game_session_id, fence)
                .ok()?
        };
        let reject = || LoopMoveResult::Refused(LoopMoveRefusal::Rejected);
        if count.is_some() {
            return Some(LoopMoveResult::Refused(LoopMoveRefusal::NotSupported));
        }
        let Some(native) = self
            .active_generation
            .and_then(|generation| generation.native_gameplay())
        else {
            return Some(reject());
        };
        let Some(_permit) = self.spell_lane_permit().await else {
            return Some(LoopMoveResult::Unknown);
        };
        let Ok(id) = CommandId::new(command.command_id) else {
            return Some(reject());
        };
        let input = NativeHouseMoveContext {
            fence,
            origin_channel: *self.channel_id.as_bytes(),
            house: context.house().clone(),
            command: CommandRef::new(command.game_session_id, id),
        };
        let recovered = self
            .root
            .recover_native_house_item(self.character, self.holder, input.clone(), payload)
            .await;
        let candidate = match recovered {
            Ok(NativeHouseRecovery::Committed(_)) => return Some(LoopMoveResult::Committed),
            Ok(NativeHouseRecovery::Pending(candidate)) => *candidate,
            Err(error) => {
                return Some(
                    if matches!(
                        error,
                        crate::durability::house_interior::HouseWriterError::Durability(_)
                    ) {
                        LoopMoveResult::Unknown
                    } else {
                        reject()
                    },
                );
            }
            Ok(NativeHouseRecovery::Absent) => {
                let Some(ItemKey::Instance(item)) = source else {
                    return None;
                };
                if !self
                    .states
                    .player_trades
                    .physical(command.game_session_id, Some(item))
                {
                    return Some(reject());
                }
                let target = match destination {
                    ItemMoveDestination::MainBackpack => NativeHouseTarget::Backpack,
                    ItemMoveDestination::Ground(position) => {
                        let Some(floor) = i32::from(position.floor).checked_neg() else {
                            return Some(reject());
                        };
                        NativeHouseTarget::Position([position.x, position.y, floor])
                    }
                    _ => return None,
                };
                let prepared = match self
                    .root
                    .prepare_native_house_item(
                        self.character,
                        self.holder,
                        input.clone(),
                        item,
                        target,
                        native,
                    )
                    .await
                {
                    Ok(Some(prepared)) => prepared,
                    Ok(None) => return None,
                    Err(_) => return Some(reject()),
                };
                if let Some(at) = prepared.source_position
                    && !near(context.position(), at)
                {
                    return Some(LoopMoveResult::TooFar);
                }
                if !self
                    .native_house_destination(&context, &prepared.request.action)
                    .await
                {
                    return Some(LoopMoveResult::Refused(LoopMoveRefusal::Blocked));
                }
                match self
                    .root
                    .freeze_native_house_item(
                        self.character,
                        self.holder,
                        fence,
                        prepared.request,
                        payload,
                    )
                    .await
                {
                    Ok(candidate) => candidate,
                    Err(_) => return Some(LoopMoveResult::Unknown),
                }
            }
        };
        // A pending reservation retains intent, never position or current source authority.
        let retained = candidate.native_request();
        if !self
            .states
            .player_trades
            .physical(command.game_session_id, Some(retained.item_instance_id))
        {
            return Some(reject());
        }
        {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            if states.is_dead(actor)
                || states.has_pending_spell_commit(actor, command.game_session_id)
                || runtime
                    .assert_house_actor_spell_unreserved(actor, command.game_session_id)
                    .is_err()
            {
                return Some(reject());
            }
        }

        let target = match &retained.action {
            HouseItemAction::Take { .. } => NativeHouseTarget::Backpack,
            HouseItemAction::Place { position, .. }
            | HouseItemAction::TileMove { position, .. } => {
                let Some(at) = HouseActionContext::decode_spatial_position(position) else {
                    return Some(reject());
                };
                NativeHouseTarget::Position(at)
            }
        };
        match self
            .root
            .prepare_native_house_item(
                self.character,
                self.holder,
                input,
                retained.item_instance_id,
                target,
                native,
            )
            .await
        {
            Ok(Some(current)) => {
                if let Some(at) = current.source_position
                    && !near(context.position(), at)
                {
                    return Some(LoopMoveResult::TooFar);
                }
            }
            Ok(None) => return Some(reject()),
            Err(_) => return Some(LoopMoveResult::Unknown),
        }
        if !self
            .native_house_destination(&context, &candidate.native_request().action)
            .await
        {
            return Some(LoopMoveResult::Refused(LoopMoveRefusal::Blocked));
        }
        let current = {
            let runtime = self.runtime.lock().await;
            HouseActionContext::from_native_owner(&runtime, actor, command.game_session_id, fence)
                .is_ok_and(|current| {
                    current.house() == context.house() && current.position() == context.position()
                })
        };
        if !current {
            return Some(reject());
        }
        Some(
            match self
                .root
                .commit_house_item_with_native(
                    self.character,
                    self.holder,
                    fence,
                    &candidate,
                    native,
                )
                .await
            {
                Ok(HouseItemOutcome::Committed(_) | HouseItemOutcome::AlreadyCommitted(_)) => {
                    LoopMoveResult::Committed
                }
                // This typed content refusal is returned before the Type2 COMMIT:
                // the transaction rolled back, so it is a known not-applied answer.
                // A database/commit uncertainty remains Unknown and reconciles.
                Err(crate::durability::house_interior::HouseWriterError::ContentChanged) => {
                    LoopMoveResult::Refused(LoopMoveRefusal::Rejected)
                }
                Err(_) => LoopMoveResult::Unknown,
            },
        )
    }
    async fn native_house_destination(
        &self,
        context: &HouseActionContext,
        action: &HouseItemAction,
    ) -> bool {
        let position = match action {
            HouseItemAction::Place { position, .. }
            | HouseItemAction::TileMove { position, .. } => position,
            _ => return true,
        };
        let Some(at) = HouseActionContext::decode_spatial_position(position) else {
            return false;
        };
        if !near(context.position(), at) {
            return false;
        }
        let cell = oteryn_gameplay_content::content::LogicalCell {
            x: at[0],
            y: at[1],
            z: at[2],
        };
        if self
            .movement_cells
            .index()
            .lookup(self.movement_cells.scope(), cell)
            != Ok(oteryn_gameplay_content::content::CollisionClass::Walkable)
        {
            return false;
        }
        !self.door.lock().await.blocking_cells().contains(&cell)
    }
}
fn near(actor: [i32; 3], target: [i32; 3]) -> bool {
    actor[2] == target[2] && actor[0].abs_diff(target[0]) <= 1 && actor[1].abs_diff(target[1]) <= 1
}
