//! Exact named-House pickup of one ordinary-death Ground root.
use super::*;
use crate::durability::consume_transform::HouseActionContext;

pub(super) async fn assert_house_death_pickup<C: ItemLoopContent>(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    command: CommandRef,
    content: &C,
    request: &LoopMoveRequest,
    house: &HouseActionContext,
) -> LoopResult<()> {
    if house.item_fence() != *fence
        || house.house().runtime_scope().ok() != Some(fence.runtime_scope)
        || command.game_session_id() != fence.game_session_id
        || request.pickup_cell.is_none()
        || matches!(request.destination, LoopDestination::Ground(_))
    {
        return refuse(LoopMoveRefusal::Rejected);
    }
    if super::super::character_progression::assert_house_death_fence(
        tx,
        fence,
        None,
        node,
        house.house(),
    )
    .await?
    .is_err()
    {
        return refuse(LoopMoveRefusal::Rejected);
    }
    // The native caller proves content against the actual owner pin before
    // constructing HouseActionContext; CharacterRoot interpretation tags are
    // deliberately not hashes of this native catalogue.
    if content.loop_content_digest() != house.server_artifact_digest() {
        return refuse(LoopMoveRefusal::Rejected);
    }
    let at = house.position();
    let exists:bool=sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_item_ground_locations g \
         JOIN game_item_instances i USING(item_instance_id,world_id) \
         JOIN game_player_death_item_records r ON r.item_instance_id=i.item_instance_id \
           AND r.operation=1 AND g.corpse_ref=convert_to('OTERYN_ITEM_LOOP_DROP/v1:','UTF8')||uuid_send(r.transaction_id) \
                       AND g.native_room_placement_context=uuid_send(r.death_occurrence_id) \
         JOIN game_character_death_receipts d USING(death_occurrence_id) \
         WHERE i.item_instance_id=encode($1,'hex')::uuid AND i.lifecycle=1 \
           AND g.world_id=encode($2,'hex')::uuid AND g.scope_kind=2 AND g.channel_id IS NULL \
           AND g.instance_id=encode($3,'hex')::uuid AND g.spatial_position=$4 \
           AND r.world_id=g.world_id AND r.scope_kind=2 AND r.instance_id=g.instance_id \
           AND d.death_world_id=g.world_id AND d.death_scope_kind=2 AND d.death_instance_id=g.instance_id \
           AND d.death_channel_id IS NULL AND d.death_spatial_position=g.spatial_position)")
        .bind(request.item.as_slice()).bind(house.house().world_id().as_bytes().as_slice())
        .bind(house.house().instance_id().as_slice()).bind(request.pickup_cell.as_deref())
        .fetch_one(&mut **tx).await?;
    let cell = request
        .pickup_cell
        .as_ref()
        .and_then(|bytes| super::super::spell_items_abi::decode_ground_cell(bytes).ok());
    if !exists
        || cell.is_none_or(|cell| {
            cell.z != at[2] || cell.x.abs_diff(at[0]) > 1 || cell.y.abs_diff(at[1]) > 1
        })
    {
        return refuse(LoopMoveRefusal::Rejected);
    }
    Ok(())
}

impl DurabilityRoot {
    /// Picks up one receipt-proven House death root under the current House item fence.
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_house_death_ground_pickup<C: ItemLoopContent>(
        &self,
        permit: &SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: &CurrentCharacterItemFence,
        command: CommandRef,
        content: &C,
        request: LoopMoveRequest,
        house: HouseActionContext,
    ) -> std::result::Result<std::result::Result<LoopMoveReceipt, LoopMoveRefusal>, DurabilityError>
    {
        // The retained native lane serializes the owner; the closed House proof below
        // grants authority. Never claim this origin lane owns the House scope.
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let fence = *fence;
        let mut context = (permit, content);
        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, context| {
                Box::pin(async move {
                    let (permit, content) = *context;
                    let mut tx =
                        super::super::db::begin_semantic_transaction(holder, deadline).await?;
                    let attempt: LoopResult<std::result::Result<LoopMoveReceipt, LoopMoveRefusal>> =
                        async {
                            let intent = intent_bytes(fence.character_id, &request)?;
                            lock_loop_command(&mut tx, command).await?;
                            if let Some(replay) =
                                committed_loop_move(&mut tx, command, fence.character_id, &intent)
                                    .await?
                            {
                                return Ok(replay);
                            }
                            apply_loop_move(
                                &mut tx,
                                permit,
                                &record,
                                &node,
                                &fence,
                                command,
                                content,
                                &request,
                                &intent,
                                None,
                                None,
                                None,
                                Some(&house),
                            )
                            .await
                            .map(Ok)
                        }
                        .await;
                    match attempt {
                        Ok(Ok(receipt)) => {
                            super::super::db::commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(receipt))
                        }
                        Ok(Err(refusal)) => Ok(Err(refusal)),
                        Err(error) => loop_error(error).map(Err),
                    }
                })
            })
            .await
    }
}
