//! Crystal AddItem seed callback joins the existing move, terrain and value-audit owners.
use super::*;
use crate::durability::terrain_tool::{MapMutation, TerrainScope};
use sha2::{Digest, Sha256};

/// The existing fenced read checks this command's actual seed source.
#[derive(Debug, Clone, Copy)]
pub struct SeedDropProbe {
    pub command: CommandRef,
    pub item: [u8; 16],
}

#[derive(Debug, Clone)]
pub struct SeedDropTarget {
    pub scope: TerrainScope,
    pub placement: Vec<u8>,
    pub expected_revision: i64,
    pub ordinary_plant_admitted: bool,
    pub ice_plant_admitted: bool,
}

impl DurabilityRoot {
    /// Read the actual fenced source before reserving the source AddItem cue. Ordinary
    /// drops onto a flower pot must not depend on an unrelated presentation reservation.
    /// The writer below repeats this check together with the full custody/CAS proof.
    pub async fn item_loop_is_seed<C: ItemLoopContent>(
        &self,
        permit: &SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        content: &C,
        request: SeedDropProbe,
    ) -> Result<bool, DurabilityError> {
        let SeedDropProbe { command, item } = request;
        permit.check_scope(fence.runtime_scope)?;
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let mut context = content;
        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, content| {
                Box::pin(async move {
                    let mut tx =
                        crate::durability::db::begin_type2_transaction(holder, deadline).await?;
                    let attempt: LoopResult<bool> = async {
                        assert_recovery_fence(&mut tx, &record).await?;
                        assert_current_equipment_in_transaction(
                            &mut tx,
                            &record,
                            &node,
                            &fence,
                            Some(command),
                            content.loop_content_digest(),
                        )
                        .await?;
                        let (world, _) = scope_of(&fence)
                            .map_err(|_| LoopError::Refused(LoopMoveRefusal::Rejected))?;
                        Ok(lock_live_item(&mut tx, item, *world.as_bytes())
                            .await?
                            .is_some_and(|(definition, _, _)| {
                                matches!(
                                    definition.production_key.as_str(),
                                    "oteryn:item.tibia.i647" | "oteryn:item.tibia.i13844"
                                )
                            }))
                    }
                    .await;
                    match attempt {
                        Ok(seed) => {
                            tx.commit(deadline).await?;
                            Ok(seed)
                        }
                        Err(error) => loop_error(error).map(|_| false),
                    }
                })
            })
            .await
    }

    /// None means the locked source is not a seed; the caller uses the ordinary move.
    /// An exact existing move receipt is returned before current map/custody is inspected.
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_item_loop_seed_drop<C: ItemLoopContent>(
        &self,
        permit: &SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        content: &C,
        request: LoopMoveRequest,
        target: SeedDropTarget,
    ) -> Result<Option<Result<LoopMoveReceipt, LoopMoveRefusal>>, DurabilityError> {
        permit.check_scope(fence.runtime_scope)?;
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let mut context = (permit, content);
        self.try_issue_semantic_pass()?.run_with_context(&mut context,move|holder,deadline,context|Box::pin(async move{
            let(permit,content)=*context;
            let mut tx=crate::durability::db::begin_type2_transaction(holder,deadline).await?;
            let attempt:LoopResult<Option<Result<LoopMoveReceipt,LoopMoveRefusal>>>=async{
                assert_recovery_fence(&mut tx,&record).await?;
                let intent=intent_bytes(fence.character_id,&request)?;
                lock_loop_command(&mut tx,command).await?;
                if let Some(replay)=committed_loop_move(&mut tx,command,fence.character_id,&intent).await?{
                    return Ok(Some(replay));
                }
                assert_current_equipment_in_transaction(&mut tx,&record,&node,&fence,Some(command),content.loop_content_digest()).await?;
                let(world,channel)=scope_of(&fence).map_err(|_|LoopError::Refused(LoopMoveRefusal::Rejected))?;
                let Some((definition,_,_))=lock_live_item(&mut tx,request.item,*world.as_bytes()).await?else{return refuse(LoopMoveRefusal::Stale);};
                let(after,admitted)=match definition.production_key.as_str(){
                    "oteryn:item.tibia.i647"=>(324_u32,target.ordinary_plant_admitted),
                    "oteryn:item.tibia.i13844"=>(14031,target.ice_plant_admitted),_=>return Ok(None),
                };
                if !admitted{return refuse(LoopMoveRefusal::NotSupported);}
                let LoopDestination::Ground(ground)=&request.destination else{return refuse(LoopMoveRefusal::Rejected);};
                if target.scope.content_generation!=content.loop_content_digest()||target.scope.map_revision!=ground.map_revision
                    ||target.placement.len()!=40||target.expected_revision<0||target.scope.reset_epoch<0{
                    return refuse(LoopMoveRefusal::Rejected);
                }
                // Existing owner proves the exact drop/split, including nested/Ground source
                // custody and every original deferred move constraint, before this consequence.
                let receipt=apply_loop_move(&mut tx,permit,&record,&node,&fence,command,content,&request,&intent,None,None,None,None).await?;
                let planted=sqlx::query("SELECT to_jsonb(i) AS before_item,to_jsonb(g) AS before_ground,uuid_send(i.item_instance_id) AS item,i.quantity FROM game_item_loop_move_lines l JOIN game_item_instances i USING(item_instance_id) JOIN game_item_ground_locations g USING(item_instance_id) WHERE l.transaction_id=encode($1,'hex')::uuid AND l.to_kind=3 AND g.world_id=encode($2,'hex')::uuid AND g.channel_id=encode($3,'hex')::uuid AND g.spatial_position=$4 FOR UPDATE OF i")
                    .bind(receipt.transaction_id.as_slice()).bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(&ground.spatial_position)
                    .fetch_one(&mut **tx).await?;
                let before:serde_json::Value=planted.try_get("before_item")?;
                let before_ground:serde_json::Value=planted.try_get("before_ground")?;
                let item:Vec<u8>=planted.try_get("item")?;
                let quantity:i64=planted.try_get("quantity")?;
                let mutation=MapMutation{placement:target.placement.clone(),expected_revision:target.expected_revision,
                    before:306,after,decay:oteryn_gameplay_items::event_planting::planting_decays(after as u16)};
                let map_intent:[u8;32]=Sha256::digest(&intent).into();
                // The prior move is already proven. New seed/CAS/audit proofs remain deferred
                // until their complete final state exists; no original guard is disabled.
                sqlx::query("SET CONSTRAINTS ALL DEFERRED").execute(&mut **tx).await?;
                if !crate::durability::terrain_tool::apply_map_in_transaction(&mut tx,fence,command,&map_intent,&target.scope,&mutation).await?{
                    return refuse(LoopMoveRefusal::Stale);
                }
                let(transaction,event,occurred):(String,String,i64)=sqlx::query_as("SELECT game_character_uuid_v7()::text,game_character_uuid_v7()::text,floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
                    .fetch_one(&mut **tx).await?;
                let tuple=tx.tuple();
                let envelope=crate::durability::item_craft::audit::encode_map_harvest(fence,command,json!({
                    "recipe":"oteryn:map-seed-drop.crystal","before":before,"ground":before_ground,
                    "move_transaction":receipt.transaction_id,"intent":intent,"map_intent":map_intent,
                    "placement":target.placement,"reset_epoch":target.scope.reset_epoch,"before_item_id":306,"after_item_id":after,
                    "quantity_after":quantity-1,"content_digest":content.loop_content_digest(),
                }),&transaction,&event,occurred,tuple.tuple())?;
                sqlx::query("INSERT INTO game_map_seed_drop_records(transaction_id,event_id,move_transaction_id,game_session_id,command_id,character_id,world_id,channel_id,item_instance_id,before_item,before_ground,intent,placement,reset_epoch,object_revision,after_item_id,map_revision,occurred_at,envelope_sha256) VALUES($1::uuid,$2::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,encode($6,'hex')::uuid,encode($7,'hex')::uuid,encode($8,'hex')::uuid,encode($9,'hex')::uuid,$10,$11,$12,$13,$14,$15,$16,$17,$18,sha256($19))")
                    .bind(&transaction).bind(&event).bind(receipt.transaction_id.as_slice()).bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string())
                    .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(&item).bind(&before).bind(&before_ground)
                    .bind(&intent).bind(&target.placement).bind(target.scope.reset_epoch).bind(target.expected_revision).bind(i64::from(after)).bind(&target.scope.map_revision)
                    .bind(occurred).bind(&envelope).execute(&mut **tx).await?;
                sqlx::query("UPDATE game_item_instances SET quantity=quantity-1,lifecycle=CASE WHEN quantity=1 THEN 2 ELSE 1 END,last_transaction_id=$2::uuid WHERE item_instance_id=encode($1,'hex')::uuid")
                    .bind(&item).bind(&transaction).execute(&mut **tx).await?;
                if quantity==1{sqlx::query("DELETE FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid").bind(&item).execute(&mut **tx).await?;}
                sqlx::query("INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state) VALUES($1::uuid,$2::uuid,1,1,2,$3,$4,encode($5,'hex')::uuid,$6,$6+$7,$8,sha256($8),1)")
                    .bind(&event).bind(&transaction).bind(i64::from(tuple.schema_revision())).bind(tuple.retention_profile_id()).bind(&item).bind(occurred)
                    .bind(crate::durability::item_mint_audit::AUDIT_RETENTION_P90D_MS).bind(&envelope).execute(&mut **tx).await?;
                Ok(Some(Ok(receipt)))
            }.await;
            match attempt{
                Ok(Some(Ok(receipt)))=>{tx.commit(deadline).await?;Ok(Some(Ok(receipt)))},
                Ok(other)=>Ok(other),Err(error)=>loop_error(error).map(|refusal|Some(Err(refusal))),
            }
        })).await
    }
}
