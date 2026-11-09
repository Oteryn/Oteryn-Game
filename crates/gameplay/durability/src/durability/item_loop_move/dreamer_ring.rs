//! The literal UID2244 ADDITEM offering joins the existing fenced drop transaction.
use super::*;
use crate::durability::terrain_tool::TerrainScope;

#[derive(Debug, Clone)]
pub struct DreamerRingTarget {
    pub scope: TerrainScope,
    pub placement: Vec<u8>,
    pub expected_revision: i64,
    /// In source order: absent0 or present3636. At least one tree must be present.
    pub trees_before: [i32; 3],
}

impl DurabilityRoot {
    /// None delegates unrelated inputs to the ordinary move. The bool authorizes
    /// source presentation only for a fresh committed offering, never replay.
    #[allow(clippy::too_many_arguments, reason = "the existing drop owner retains its full fence and source map proof")]
    pub async fn commit_item_loop_dreamer_ring<C: ItemLoopContent>(
        &self,
        permit: &SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        content: &C,
        request: LoopMoveRequest,
        target: DreamerRingTarget,
    ) -> Result<Option<Result<(LoopMoveReceipt, bool), LoopMoveRefusal>>, DurabilityError> {
        permit.check_scope(fence.runtime_scope)?;
        let record=recovery.record_for(self).map_err(|_|DurabilityError::Unavailable)?;
        let node=node.clone();
        let mut context=(permit,content);
        self.try_issue_semantic_pass()?.run_with_context(&mut context,move|holder,deadline,context|Box::pin(async move{
            let (permit,content)=*context;
            let mut tx=crate::durability::db::begin_type2_transaction(holder,deadline).await?;
            let attempt:LoopResult<Option<Result<(LoopMoveReceipt,bool),LoopMoveRefusal>>>=async {
                assert_recovery_fence(&mut tx,&record).await?;
                let intent=intent_bytes(fence.character_id,&request)?;
                lock_loop_command(&mut tx,command).await?;
                if let Some(replay)=committed_loop_move(&mut tx,command,fence.character_id,&intent).await? {
                    return Ok(Some(replay.map(|receipt|(receipt,false))));
                }
                assert_current_equipment_in_transaction(&mut tx,&record,&node,&fence,Some(command),content.loop_content_digest()).await?;
                let (world,channel)=scope_of(&fence).map_err(|_|LoopError::Refused(LoopMoveRefusal::Rejected))?;
                let Some((definition,quantity,_))=lock_live_item(&mut tx,request.item,*world.as_bytes()).await? else {return refuse(LoopMoveRefusal::Stale);};
                if definition.production_key!="oteryn:item.tibia.i6299" {return Ok(None);}
                let LoopDestination::Ground(ground)=&request.destination else {return refuse(LoopMoveRefusal::Rejected);};
                if definition.family!="Item" || definition.revision_ref!="definition-r1" || quantity!=1
                    || target.scope.content_generation!=content.loop_content_digest() || target.scope.map_revision!=ground.map_revision
                    || target.placement.len()!=40 || target.expected_revision<0 || target.scope.reset_epoch<0
                    || !target.trees_before.contains(&3636) || target.trees_before.iter().any(|id|!matches!(id,0|3636)) {
                    return refuse(LoopMoveRefusal::Rejected);
                }
                let current:bool=sqlx::query_scalar("SELECT game_world_reset_map_epoch_current(encode($1,'hex')::uuid,$2::text::numeric)")
                    .bind(world.as_bytes().as_slice()).bind(target.scope.reset_epoch.to_string()).fetch_one(&mut **tx).await?;
                if !current {return refuse(LoopMoveRefusal::Stale);}
                let receipt=apply_loop_move(&mut tx,permit,&record,&node,&fence,command,content,&request,&intent,None,None,None,None).await?;
                let row=sqlx::query("SELECT to_jsonb(i) AS before_item,to_jsonb(g) AS before_ground FROM game_item_loop_move_lines l JOIN game_item_instances i USING(item_instance_id) JOIN game_item_ground_locations g USING(item_instance_id) WHERE l.transaction_id=encode($1,'hex')::uuid AND l.to_kind=3 AND i.item_instance_id=encode($2,'hex')::uuid AND g.spatial_position=$3 FOR UPDATE OF i")
                    .bind(receipt.transaction_id.as_slice()).bind(request.item.as_slice()).bind(&ground.spatial_position).fetch_one(&mut **tx).await?;
                let before:serde_json::Value=row.try_get("before_item")?;
                let before_ground:serde_json::Value=row.try_get("before_ground")?;
                let (transaction,event,occurred):(String,String,i64)=sqlx::query_as("SELECT game_character_uuid_v7()::text,game_character_uuid_v7()::text,floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
                    .fetch_one(&mut **tx).await?;
                let tuple=tx.tuple();
                let envelope=crate::durability::item_craft::audit::encode_map_harvest(fence,command,json!({
                    "source_unique":2244,"before":before,"ground":before_ground,"outputs":[],
                    "trees_before":target.trees_before,"tree_positions":[[32857,32231,11],[32857,32232,11],[32857,32233,11]],
                    "move_transaction":receipt.transaction_id,"intent":intent,"placement":target.placement,
                    "object_revision":target.expected_revision,"reset_epoch":target.scope.reset_epoch,"content_digest":content.loop_content_digest(),
                }),&transaction,&event,occurred,tuple.tuple())?;
                // The original move has already passed its unchanged guards. The offering's
                // final debit, custody removal and audit are checked together at commit.
                sqlx::query("SET CONSTRAINTS ALL DEFERRED").execute(&mut **tx).await?;
                sqlx::query("INSERT INTO game_dreamer_ring_offerings(transaction_id,event_id,move_transaction_id,game_session_id,command_id,character_id,world_id,channel_id,item_instance_id,before_item,before_ground,intent,placement,reset_epoch,object_revision,trees_before,map_revision,content_digest,occurred_at,envelope_sha256) VALUES($1::uuid,$2::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,encode($6,'hex')::uuid,encode($7,'hex')::uuid,encode($8,'hex')::uuid,encode($9,'hex')::uuid,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,sha256($20))")
                    .bind(&transaction).bind(&event).bind(receipt.transaction_id.as_slice()).bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string())
                    .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(request.item.as_slice()).bind(&before).bind(&before_ground)
                    .bind(&intent).bind(&target.placement).bind(target.scope.reset_epoch).bind(target.expected_revision).bind(target.trees_before.as_slice()).bind(&target.scope.map_revision)
                    .bind(content.loop_content_digest().as_slice()).bind(occurred).bind(&envelope).execute(&mut **tx).await?;
                sqlx::query("UPDATE game_item_instances SET quantity=0,lifecycle=2,last_transaction_id=$2::uuid WHERE item_instance_id=encode($1,'hex')::uuid")
                    .bind(request.item.as_slice()).bind(&transaction).execute(&mut **tx).await?;
                sqlx::query("DELETE FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid")
                    .bind(request.item.as_slice()).execute(&mut **tx).await?;
                sqlx::query("INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state) VALUES($1::uuid,$2::uuid,1,1,2,$3,$4,encode($5,'hex')::uuid,$6,$6+$7,$8,sha256($8),1)")
                    .bind(&event).bind(&transaction).bind(i64::from(tuple.schema_revision())).bind(tuple.retention_profile_id()).bind(request.item.as_slice()).bind(occurred)
                    .bind(crate::durability::item_mint_audit::AUDIT_RETENTION_P90D_MS).bind(&envelope).execute(&mut **tx).await?;
                Ok(Some(Ok((receipt,true))))
            }.await;
            match attempt {
                Ok(Some(Ok(receipt)))=>{tx.commit(deadline).await?;Ok(Some(Ok(receipt)))},
                Ok(other)=>Ok(other),Err(error)=>loop_error(error).map(|refusal|Some(Err(refusal))),
            }
        })).await
    }
}
