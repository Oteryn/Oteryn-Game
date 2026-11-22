//! Source-bound AddItem offerings join the existing fenced drop transaction.
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

#[derive(Clone, Debug)]
pub struct CoalBasinTarget {
    pub scope: TerrainScope,
    pub placement: Vec<u8>,
    pub expected_revision: i64,
    pub uid: u16,
    pub recipient: Option<CoalBasinRecipient>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CoalBasinRecipient {
    pub session: [u8; 16],
    pub actor: [u8; 16],
    pub actor_generation: u64,
    pub movement_revision: u64,
}

enum OfferingTarget {
    Dreamer(DreamerRingTarget),
    CoalBasin(CoalBasinTarget),
}

impl DurabilityRoot {
    /// None delegates unrelated inputs to the ordinary move. The bool authorizes
    /// source presentation only for a fresh committed offering, never replay; a lost
    /// acknowledgement is published by the dispatch replay through
    /// [`DurabilityRoot::read_dreamer_ring_offering`].
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
        self.commit_item_loop_offering(
            permit, recovery, node, fence, command, content, request, OfferingTarget::Dreamer(target),
        ).await
    }

    #[allow(clippy::too_many_arguments, reason = "the existing drop owner retains its full fence and exact basin target")]
    pub async fn commit_item_loop_coal_basin<C: ItemLoopContent>(
        &self,
        permit: &SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        content: &C,
        request: LoopMoveRequest,
        target: CoalBasinTarget,
    ) -> Result<Option<Result<(LoopMoveReceipt, bool), LoopMoveRefusal>>, DurabilityError> {
        self.commit_item_loop_offering(
            permit, recovery, node, fence, command, content, request, OfferingTarget::CoalBasin(target),
        ).await
    }

    #[allow(clippy::too_many_arguments, reason = "one item-loop transaction owns the move and the exact source offering")]
    async fn commit_item_loop_offering<C: ItemLoopContent>(
        &self,
        permit: &SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        content: &C,
        request: LoopMoveRequest,
        target: OfferingTarget,
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
                use oteryn_gameplay_quests::quest::carried_step::{COAL_BASINS, SCARAB_COIN};
                let (scope, placement, revision, trees, basin, recipient, key) = match &target {
                    OfferingTarget::Dreamer(target) => {
                        if quantity != 1 || !target.trees_before.contains(&3636)
                            || target.trees_before.iter().any(|id| !matches!(id, 0 | 3636)) {
                            // Unrelated drops are still owned by the normal item loop.
                            if definition.production_key != "oteryn:item.tibia.i6299" { return Ok(None); }
                            return refuse(LoopMoveRefusal::Rejected);
                        }
                        (&target.scope,&target.placement,target.expected_revision,Some(target.trees_before.as_slice()),None,None,"oteryn:item.tibia.i6299")
                    }
                    OfferingTarget::CoalBasin(target) => {
                        if !COAL_BASINS.iter().any(|basin| basin.unique == target.uid) { return refuse(LoopMoveRefusal::Rejected); }
                        let recipient = target.recipient.as_ref().map(serde_json::to_value).transpose().map_err(|_| DurabilityError::InvalidStoredState)?;
                        (&target.scope,&target.placement,target.expected_revision,None,Some(i32::from(target.uid)),recipient,SCARAB_COIN)
                    }
                };
                if definition.production_key!=key {return Ok(None);}
                let LoopDestination::Ground(ground)=&request.destination else {return refuse(LoopMoveRefusal::Rejected);};
                if definition.family!="Item" || definition.revision_ref!="definition-r1"
                    || scope.content_generation!=content.loop_content_digest() || scope.map_revision!=ground.map_revision
                    || placement.len()!=40 || revision<0 || scope.reset_epoch<0 {
                    return refuse(LoopMoveRefusal::Rejected);
                }
                let current:bool=sqlx::query_scalar("SELECT game_world_reset_map_epoch_current(encode($1,'hex')::uuid,$2::text::numeric)")
                    .bind(world.as_bytes().as_slice()).bind(scope.reset_epoch.to_string()).fetch_one(&mut **tx).await?;
                if !current {return refuse(LoopMoveRefusal::Stale);}
                let receipt=apply_loop_move(&mut tx,permit,&record,&node,&fence,command,content,&request,&intent,None,None,None,None).await?;
                let row=sqlx::query("SELECT to_jsonb(i) AS before_item,to_jsonb(g) AS before_ground,i.item_instance_id::text AS item FROM game_item_loop_move_lines l JOIN game_item_instances i USING(item_instance_id) JOIN game_item_ground_locations g USING(item_instance_id) WHERE l.transaction_id=encode($1,'hex')::uuid AND l.to_kind=3 AND g.spatial_position=$2 FOR UPDATE OF i")
                    .bind(receipt.transaction_id.as_slice()).bind(&ground.spatial_position).fetch_one(&mut **tx).await?;
                let item: String = row.try_get("item")?;
                let before:serde_json::Value=row.try_get("before_item")?;
                let before_ground:serde_json::Value=row.try_get("before_ground")?;
                let (transaction,event,occurred):(String,String,i64)=sqlx::query_as("SELECT game_character_uuid_v7()::text,game_character_uuid_v7()::text,floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
                    .fetch_one(&mut **tx).await?;
                let tuple=tx.tuple();
                let mut proof=json!({
                    "source_unique":basin.unwrap_or(2244),"before":before,"ground":before_ground,"outputs":[],
                    "trees_before":trees,"tree_positions":trees.map(|_| [[32857,32231,11],[32857,32232,11],[32857,32233,11]]),
                    "move_transaction":receipt.transaction_id,"intent":intent,"placement":placement,
                    "object_revision":revision,"reset_epoch":scope.reset_epoch,"content_digest":content.loop_content_digest(),
                });
                if basin.is_some() {
                    if let Some(object) = proof.as_object_mut() {
                        object.remove("trees_before");
                        object.remove("tree_positions");
                    }
                    proof["basin_recipient"] = json!(recipient);
                }
                let envelope=crate::durability::item_craft::audit::encode_map_harvest(fence,command,proof,&transaction,&event,occurred,tuple.tuple())?;
                // The original move has already passed its unchanged guards. The offering's
                // final debit, custody removal and audit are checked together at commit.
                sqlx::query("SET CONSTRAINTS ALL DEFERRED").execute(&mut **tx).await?;
                sqlx::query("INSERT INTO game_dreamer_ring_offerings(transaction_id,event_id,move_transaction_id,game_session_id,command_id,character_id,world_id,channel_id,item_instance_id,before_item,before_ground,intent,placement,reset_epoch,object_revision,trees_before,map_revision,content_digest,occurred_at,envelope_sha256,basin_uid,basin_recipient) VALUES($1::uuid,$2::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,encode($6,'hex')::uuid,encode($7,'hex')::uuid,encode($8,'hex')::uuid,$9::uuid,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,sha256($20),$21,$22)")
                    .bind(&transaction).bind(&event).bind(receipt.transaction_id.as_slice()).bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string())
                    .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(&item).bind(&before).bind(&before_ground)
                    .bind(&intent).bind(&placement).bind(scope.reset_epoch).bind(revision).bind(trees).bind(&scope.map_revision)
                    .bind(content.loop_content_digest().as_slice()).bind(occurred).bind(&envelope).bind(basin).bind(&recipient).execute(&mut **tx).await?;
                sqlx::query("UPDATE game_item_instances SET quantity=0,lifecycle=2,last_transaction_id=$2::uuid WHERE item_instance_id=$1::uuid")
                    .bind(&item).bind(&transaction).execute(&mut **tx).await?;
                sqlx::query("DELETE FROM game_item_ground_locations WHERE item_instance_id=$1::uuid")
                    .bind(&item).execute(&mut **tx).await?;
                sqlx::query("INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state) VALUES($1::uuid,$2::uuid,1,1,2,$3,$4,$5::uuid,$6,$6+$7,$8,sha256($8),1)")
                    .bind(&event).bind(&transaction).bind(i64::from(tuple.schema_revision())).bind(tuple.retention_profile_id()).bind(&item).bind(occurred)
                    .bind(crate::durability::item_mint_audit::AUDIT_RETENTION_P90D_MS).bind(&envelope).execute(&mut **tx).await?;
                Ok(Some(Ok((receipt,true))))
            }.await;
            match attempt {
                Ok(Some(Ok(receipt)))=>{tx.commit(deadline).await?;Ok(Some(Ok(receipt)))},
                Ok(other)=>Ok(other),Err(error)=>loop_error(error).map(|refusal|Some(Err(refusal))),
            }
        })).await
    }

    /// The durable target of the offering committed by one command, if any. Dispatch
    /// republishes it after a lost acknowledgement; a published source no longer matches it.
    pub async fn read_dreamer_ring_offering(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        character_id: CharacterId,
        command: CommandRef,
    ) -> Result<Option<DreamerRingTarget>, DurabilityError> {
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx =
                        crate::durability::db::begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &record).await?;
                    let row = sqlx::query(
                        "SELECT placement,reset_epoch,object_revision,trees_before,map_revision,content_digest \
                           FROM game_dreamer_ring_offerings WHERE game_session_id = encode($1,'hex')::uuid \
                            AND command_id = $2::text::numeric(20,0) AND character_id = encode($3,'hex')::uuid AND basin_uid IS NULL",
                    )
                    .bind(command.game_session_id().as_bytes().as_slice())
                    .bind(command.command_id().get().to_string())
                    .bind(character_id.as_bytes().as_slice())
                    .fetch_optional(&mut *tx)
                    .await?;
                    let target = match row {
                        None => None,
                        Some(row) => {
                            let trees: Vec<i32> = row.try_get("trees_before")?;
                            let content: Vec<u8> = row.try_get("content_digest")?;
                            Some(DreamerRingTarget {
                                scope: TerrainScope {
                                    map_revision: row.try_get("map_revision")?,
                                    content_generation: content
                                        .try_into()
                                        .map_err(|_| DurabilityError::InvalidStoredState)?,
                                    reset_epoch: row.try_get("reset_epoch")?,
                                },
                                placement: row.try_get("placement")?,
                                expected_revision: row.try_get("object_revision")?,
                                trees_before: trees
                                    .try_into()
                                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                            })
                        }
                    };
                    crate::durability::db::commit_semantic_transaction(tx, deadline).await?;
                    Ok(target)
                })
            })
            .await
    }
    pub async fn read_coal_basin_offering(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        scope: TerrainScope,
    ) -> Result<Option<(u16, [u8; 16], Option<CoalBasinRecipient>)>, DurabilityError> {
        let record = recovery.record_for(self).map_err(|_| DurabilityError::Unavailable)?;
        let (world, channel) = scope_of(&fence).map_err(|_| DurabilityError::CommitRejected)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = super::super::db::begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &record).await?;
            let row = sqlx::query_as::<_, (i32, String, Option<sqlx::types::Json<CoalBasinRecipient>>)>(
                "SELECT o.basin_uid,o.transaction_id::text,o.basin_recipient \
                 FROM game_dreamer_ring_offerings o JOIN game_item_loop_move_receipts r ON r.transaction_id=o.move_transaction_id \
                 WHERE o.game_session_id=encode($1,'hex')::uuid AND o.command_id=$2::text::numeric \
                   AND o.character_id=encode($3,'hex')::uuid AND o.world_id=encode($4,'hex')::uuid \
                   AND o.channel_id=encode($5,'hex')::uuid AND r.scope_generation=$6::text::numeric \
                   AND o.map_revision=$7 AND o.content_digest=$8 AND o.reset_epoch=$9 AND o.basin_uid IS NOT NULL \
                   AND game_world_reset_map_epoch_current(o.world_id,o.reset_epoch::numeric)",
            ).bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string())
             .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
             .bind(fence.scope_ownership_generation.get().to_string()).bind(&scope.map_revision)
             .bind(scope.content_generation.as_slice()).bind(scope.reset_epoch).fetch_optional(&mut *tx).await?;
            super::super::db::commit_semantic_transaction(tx, deadline).await?;
            row.map(|(uid, transaction, who)| {
                Ok((u16::try_from(uid).map_err(|_| DurabilityError::InvalidStoredState)?,
                    super::super::item_mint::uuid_text(&transaction)?, who.map(|who| who.0)))
            }).transpose()
        })).await
    }

}
