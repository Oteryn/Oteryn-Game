//! AddItem consequence in the existing fenced native transfer transaction.
use super::*;
use crate::durability::consume_transform::plant_seed_after_move_in_transaction;
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;

impl DurabilityRoot {
    #[allow(
        clippy::too_many_arguments,
        reason = "independent lane, authority, node, session, command and content proofs"
    )]
    pub async fn commit_item_loop_materialized_seed_drop<C: ItemLoopContent>(
        &self,
        permit: &SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        content: &C,
        native: &NativeGameplayState,
        request: LoopMoveRequest,
    ) -> Result<Option<Result<LoopMoveReceipt, LoopMoveRefusal>>, DurabilityError> {
        if !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. }) {
            return Ok(None);
        }
        let LoopDestination::Ground(ground) = &request.destination else {
            return Ok(None);
        };
        let ground = ground.clone();
        permit.check_scope(fence.runtime_scope)?;
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let mut context = (permit, content, native);
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, context| Box::pin(async move {
            let (permit, content, native) = *context;
            let mut tx = crate::durability::db::begin_type2_transaction(holder, deadline).await?;
            let attempt: LoopResult<Option<Result<LoopMoveReceipt, LoopMoveRefusal>>> = async {
                assert_recovery_fence(&mut tx, &record).await?;
                let intent = intent_bytes(fence.character_id, &request)?;
                lock_loop_command(&mut tx, command).await?;
                if let Some(replay) = committed_loop_move(&mut tx, command, fence.character_id, &intent).await? {
                    return Ok(Some(replay));
                }
                assert_current_equipment_in_transaction(&mut tx, &record, &node, &fence, Some(command), content.loop_content_digest()).await?;
                let (world, channel) = scope_of(&fence).map_err(|_| LoopError::Refused(LoopMoveRefusal::Rejected))?;
                let Some((definition, _, _)) = lock_live_item(&mut tx, request.item, *world.as_bytes()).await? else { return Ok(None); };
                if !matches!(definition.production_key.as_str(), "oteryn:item.tibia.i647" | "oteryn:item.tibia.i13844") { return Ok(None); }
                // Selection is an actual Ground Item, not a caller-supplied pot UUID.
                let pots = sqlx::query("SELECT uuid_send(i.item_instance_id) AS item,to_jsonb(i) AS before_item,to_jsonb(g) AS location,i.definition_revision_ref,g.stack_ordinal FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id,world_id) WHERE i.world_id=encode($1,'hex')::uuid AND g.channel_id=encode($2,'hex')::uuid AND g.scope_kind=1 AND g.runtime_scope_ownership_generation=$3::text::numeric AND g.spatial_position=$4 AND g.map_revision=$5 AND g.content_revision=$6 AND g.stack_ordinal>0 AND i.lifecycle=1 AND i.quantity=1 AND i.definition_family='Item' AND i.definition_production_key='oteryn:item.tibia.i306' AND (g.native_room_placement_context=$7 OR game_item_loop_map_mint_source(i.item_instance_id) OR game_item_action_ground_source(i.item_instance_id)) ORDER BY i.item_instance_id LIMIT 2 FOR UPDATE OF i")
                    .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
                    .bind(fence.scope_ownership_generation.get().to_string()).bind(&ground.spatial_position)
                    .bind(&ground.map_revision).bind(&ground.content_revision).bind(&ground.placement_context)
                    .fetch_all(&mut **tx).await?;
                if pots.is_empty() { return Ok(None); }
                if pots.len() != 1 { return refuse(LoopMoveRefusal::NotSupported); }
                let pot = &pots[0];
                let receipt = apply_loop_move(&mut tx, permit, &record, &node, &fence, command, content,
                    &request, &intent, None, None, None).await?;
                // Prove the ordinary move first. Its AddItem consequence is fenced by the
                // same transaction; a later failure rolls the original move back too.
                sqlx::query("SET CONSTRAINTS ALL DEFERRED").execute(&mut **tx).await?;
                let seed = sqlx::query("SELECT uuid_send(i.item_instance_id) AS item,to_jsonb(i) AS before_item,to_jsonb(g) AS location,g.stack_ordinal FROM game_item_loop_move_lines l JOIN game_item_instances i USING(item_instance_id) JOIN game_item_ground_locations g USING(item_instance_id) WHERE l.transaction_id=encode($1,'hex')::uuid AND l.to_kind=3 AND g.world_id=encode($2,'hex')::uuid AND g.channel_id=encode($3,'hex')::uuid AND g.spatial_position=$4 FOR UPDATE OF i")
                    .bind(receipt.transaction_id.as_slice()).bind(world.as_bytes().as_slice())
                    .bind(channel.as_bytes().as_slice()).bind(&ground.spatial_position).fetch_one(&mut **tx).await?;
                let item = |row: &sqlx::postgres::PgRow| -> Result<[u8;16], DurabilityError> {
                    row.try_get::<Vec<u8>,_>("item")?.try_into().map_err(|_| DurabilityError::InvalidStoredState)
                };
                plant_seed_after_move_in_transaction(&mut tx, fence, command, intent, native,
                    item(pot)?, pot.try_get("before_item")?, pot.try_get("location")?,
                    item(&seed)?, seed.try_get("before_item")?, seed.try_get("location")?, receipt.transaction_id, &ground.spatial_position).await?;
                Ok(Some(Ok(receipt)))
            }.await;
            match attempt {
                Ok(Some(Ok(receipt))) => { tx.commit(deadline).await?; Ok(Some(Ok(receipt))) }
                Ok(other) => Ok(other),
                Err(error) => loop_error(error).map(|refusal| Some(Err(refusal))),
            }
        })).await
    }
}
