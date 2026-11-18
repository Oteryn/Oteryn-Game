//! Crystal actions_oasis_lever_door.lua:26-34 creates a portable item until its ground timer.
use super::*;
use crate::durability::{
    db::begin_type2_transaction,
    item_craft::audit::encode_map_harvest,
};
use serde_json::json;

impl DurabilityRoot {
    #[allow(clippy::too_many_arguments)]
    pub async fn claim_temporary_quest_pickup<C: ItemLoopContent>(
        &self,
        permit: &SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        content: &C,
        occurrence: [u8; 32],
        definition: TypedDefinitionRef,
    ) -> std::result::Result<std::result::Result<(), LoopMoveRefusal>, DurabilityError> {
        permit.check_scope(fence.runtime_scope)?;
        let record = recovery.record_for(self).map_err(|_| DurabilityError::CommitRejected)?;
        let node = node.clone();
        let mut context = content;
        self.try_issue_semantic_pass()?.run_with_context(&mut context,move |holder,deadline,content| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&record).await?;
            lock_loop_command(&mut tx,command).await?;
            let attempt: LoopResult<()> = async {
                assert_current_equipment_in_transaction(
                    &mut tx,&record,&node,&fence,Some(command),content.loop_content_digest(),
                ).await?;
                let (world,_) = scope_of(&fence).map_err(|_|LoopError::Refused(LoopMoveRefusal::Rejected))?;
                let facts = content.loop_facts(&definition).filter(|f| f.container_capacity.is_none())
                    .ok_or(LoopError::Refused(LoopMoveRefusal::NotSupported))?;
                let weight = facts.weight.ok_or(LoopError::Refused(LoopMoveRefusal::Rejected))?;
                let bag = sqlx::query(
                    "SELECT uuid_send(i.item_instance_id) AS id,i.definition_family,i.definition_production_key,
                            i.definition_revision_ref
                     FROM game_item_container_slots b JOIN game_item_instances i USING(item_instance_id)
                     WHERE b.character_id=encode($1,'hex')::uuid AND b.world_id=encode($2,'hex')::uuid
                       AND i.world_id=b.world_id AND i.lifecycle=1 FOR UPDATE OF i",
                )
                .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice())
                .fetch_optional(&mut **tx).await?.ok_or(LoopError::Refused(LoopMoveRefusal::NoBackpack))?;
                let parent:Vec<u8> = bag.try_get("id")?;
                let bag_definition = TypedDefinitionRef {
                    family:bag.try_get("definition_family")?,
                    production_key:bag.try_get("definition_production_key")?,
                    revision_ref:bag.try_get("definition_revision_ref")?,
                };
                let capacity = content.loop_facts(&bag_definition).and_then(|f|f.container_capacity)
                    .ok_or(LoopError::Refused(LoopMoveRefusal::NotSupported))?;
                let (occupied,last):(i64,String) = sqlx::query_as(
                    "SELECT count(*),coalesce(max(placement_ordinal),0)::text FROM game_item_container_entries
                     WHERE parent_item_instance_id=encode($1,'hex')::uuid",
                ).bind(&parent).fetch_one(&mut **tx).await?;
                let weight_before = carried_weight(&mut tx, fence.character_id,*world.as_bytes(),*content).await?;
                let weight_limit = read_derived_transfer_capacity_centi_oz(&mut tx,fence.character_id,*content).await?;
                let tree:i64 = sqlx::query_scalar(
                    "WITH RECURSIVE t(item_instance_id,depth) AS (
                       SELECT item_instance_id,1 FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid
                       UNION ALL SELECT n.item_instance_id,t.depth+1 FROM t JOIN game_item_nested_entries n
                           ON n.parent_item_instance_id=t.item_instance_id WHERE t.depth<8)
                     SELECT count(*) FROM (SELECT 1 FROM t LIMIT 501) bounded",
                ).bind(&parent).fetch_one(&mut **tx).await?;
                if occupied >= i64::from(capacity) || tree >= 500 {
                    return refuse(LoopMoveRefusal::NoRoom);
                }
                if weight_before.checked_add(u64::from(weight)).is_none_or(|total|total>weight_limit) {
                    return refuse(LoopMoveRefusal::Rejected);
                }
                let ordinal = last.parse::<u64>().ok().and_then(|v|v.checked_add(1))
                    .ok_or(LoopError::Refused(LoopMoveRefusal::NoRoom))?;
                let (transaction,event,item,occurred):(String,String,String,i64) = sqlx::query_as(
                    "SELECT game_character_uuid_v7()::text,game_character_uuid_v7()::text,
                            game_character_uuid_v7()::text,floor(extract(epoch FROM clock_timestamp())*1000)::bigint",
                ).fetch_one(&mut **tx).await?;
                let tuple=tx.tuple();
                let envelope=encode_map_harvest(fence, command, json!({
                    "recipe":"oteryn:quest-temporary-pickup",
                    "occurrence":occurrence,"outputs":[{"item":item,"key":definition.production_key,
                      "revision":definition.revision_ref,"count":1,"parent":parent,"ordinal":ordinal}],
                }), &transaction,&event,occurred,tuple.tuple())?;
                let saved = sqlx::query(
                    "INSERT INTO game_quest_temporary_pickups
                       (occurrence,transaction_id,event_id,item_instance_id,world_id,character_id,game_session_id,
                        command_id,parent_item_instance_id,placement_ordinal,definition_key,definition_revision,envelope_sha256,occurred_at)
                     VALUES ($1,$2::uuid,$3::uuid,$4::uuid,encode($5,'hex')::uuid,encode($6,'hex')::uuid,
                             encode($7,'hex')::uuid,$8::text::numeric,encode($9,'hex')::uuid,$10::text::numeric,$11,$12,sha256($13),$14)
                     ON CONFLICT DO NOTHING",
                ).bind(occurrence.as_slice()).bind(&transaction).bind(&event).bind(&item)
                    .bind(world.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice())
                    .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string())
                    .bind(&parent).bind(ordinal.to_string()).bind(&definition.production_key)
                    .bind(&definition.revision_ref).bind(&envelope).bind(occurred).execute(&mut **tx).await?;
                if saved.rows_affected()!=1 {
                    return refuse(LoopMoveRefusal::Stale);
                }
                sqlx::query(
                    "INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,
                         definition_revision_ref,quantity,lifecycle,minted_transaction_id)
                     VALUES($1::uuid,encode($2,'hex')::uuid,'Item',$3,$4,1,1,$5::uuid)",
                ).bind(&item).bind(world.as_bytes().as_slice()).bind(&definition.production_key)
                    .bind(&definition.revision_ref).bind(&transaction).execute(&mut **tx).await?;
                sqlx::query(
                    "INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,
                         placement_ordinal,placed_transaction_id)
                     VALUES($1::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,$6::uuid)",
                ).bind(&item).bind(world.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice())
                    .bind(&parent).bind(ordinal.to_string()).bind(&transaction).execute(&mut **tx).await?;
                sqlx::query(
                    "INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,
                         event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,
                         envelope,envelope_sha256,publication_state)
                     VALUES($1::uuid,$2::uuid,1,1,2,$3,$4,$5::uuid,$6,$6+$7,$8,sha256($8),1)",
                ).bind(&event).bind(&transaction).bind(i64::from(tuple.schema_revision())).bind(tuple.retention_profile_id())
                    .bind(&item).bind(occurred).bind(crate::durability::item_mint_audit::AUDIT_RETENTION_P90D_MS)
                    .bind(&envelope).execute(&mut **tx).await?;
                Ok(())
            }.await;
            match attempt {
                Ok(()) => {tx.commit(deadline).await?;Ok(Ok(()))},
                Err(error) => loop_error(error).map(Err),
            }
        })).await
    }
}
