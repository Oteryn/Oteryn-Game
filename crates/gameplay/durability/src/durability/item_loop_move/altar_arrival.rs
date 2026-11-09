//! M49: retire the exact arriving corpse and retain a pending boss in the move's transaction.
use super::*;
use crate::durability::item_mint_audit::Type2Transaction;
use crate::durability::spell_item_transaction::{
    assert_spell_item_authority_with_recovery, prepare_moved_arrival_corpse_in_transaction,
};
use crate::durability::spell_items_abi::{QualifiedItemDefinition, SpellGroundTarget};
use oteryn_gameplay_quests::quest::elemental_spheres;
use sha2::{Digest, Sha256};

/// Frozen map observation, not permission to mutate custody or install an actor.
#[derive(Clone, Debug)]
pub struct AltarArrivalTarget {
    pub rule: String,
    pub readset: Vec<(u64, u64)>,
    pub reset_epoch: i64,
}
impl AltarArrivalTarget {
    pub fn new(rule: &str, readset: Vec<(u64, u64)>, reset_epoch: i64) -> Self {
        Self {
            rule: rule.into(),
            readset,
            reset_epoch,
        }
    }
    fn valid(&self) -> bool {
        self.reset_epoch >= 0
            && self.readset.len() == 8
            && self
                .readset
                .iter()
                .all(|(key, rev)| *key != 0 && *rev != u64::MAX)
            && self
                .readset
                .iter()
                .map(|(key, _)| *key)
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                == 8
            && elemental_spheres::rules()
                .iter()
                .any(|rule| rule.key == self.rule)
    }
}
/// A committed pending realization. It never grants current runtime authority.
#[derive(Clone, Debug)]
pub struct PendingAltarBoss {
    pub transaction_id: [u8; 16],
    pub rule: String,
}
#[derive(Debug)]
pub struct AltarArrivalReceipt {
    pub moved: LoopMoveReceipt,
    pub summon: Option<PendingAltarBoss>,
}

impl DurabilityRoot {
    #[allow(
        clippy::too_many_arguments,
        reason = "the ordinary move and its source-qualified consequence share one owner fence"
    )]
    pub async fn commit_item_loop_altar_arrival<C: ItemLoopContent>(
        &self,
        permit: &SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        content: &C,
        request: LoopMoveRequest,
        qualified: &QualifiedItemDefinition,
        target: AltarArrivalTarget,
    ) -> Result<Result<AltarArrivalReceipt, LoopMoveRefusal>, DurabilityError> {
        permit.check_scope(fence.runtime_scope)?;
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let root = self.clone();
        let mut context = (permit, content, qualified);
        self.try_issue_semantic_pass()?.run_with_context(&mut context,move |holder,deadline,context|Box::pin(async move {
            let (permit,content,qualified)=*context;
            let mut tx=crate::durability::db::begin_type2_transaction(holder,deadline).await?;
            let attempt:LoopResult<AltarArrivalReceipt>=async {
                assert_recovery_fence(&mut tx,&record).await?;
                let intent=intent_bytes(fence.character_id,&request)?;
                lock_loop_command(&mut tx,command).await?;
                if let Some(replay)=committed_loop_move(&mut tx,command,fence.character_id,&intent).await? {
                    return replay.map(|moved|AltarArrivalReceipt{moved,summon:None}).map_err(LoopError::Refused);
                }
                if !target.valid() {return refuse(LoopMoveRefusal::Rejected);}
                let LoopDestination::Ground(ground)=&request.destination else {return refuse(LoopMoveRefusal::Rejected);};
                let Some(rule)=elemental_spheres::rules().iter().find(|rule|rule.key==target.rule) else {return refuse(LoopMoveRefusal::Rejected);};
                if qualified.definition.production_key!=rule.corpse || qualified.content_generation_digest!=content.loop_content_digest() {
                    return refuse(LoopMoveRefusal::Rejected);
                }
                let moved=apply_loop_move(&mut tx,permit,&record,&node,&fence,command,content,&request,&intent,None,None,None,None).await?;
                let (world,channel)=scope_of(&fence).map_err(|_|LoopError::Refused(LoopMoveRefusal::Rejected))?;
                let current:bool=sqlx::query_scalar("SELECT game_world_reset_map_epoch_current(encode($1,'hex')::uuid,$2::numeric)")
                    .bind(world.as_bytes().as_slice()).bind(target.reset_epoch).fetch_one(&mut **tx).await?;
                if !current {return refuse(LoopMoveRefusal::Stale);}
                // Every writer already owns the Channel item lock (key 33). This
                // serializes the four bounded latch keys before checking absence.
                let busy:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_elemental_boss_latches WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND rule=$3 AND reset_epoch=$4)")
                    .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(&target.rule).bind(target.reset_epoch).fetch_one(&mut **tx).await?;
                if busy {return Ok(AltarArrivalReceipt{moved,summon:None});}
                let authority=assert_spell_item_authority_with_recovery(&mut tx,permit,&root,&record,&node,&fence,command,content.loop_content_digest()).await.map_err(spell_error)?;
                let revision:String=sqlx::query_scalar("SELECT state_revision::text FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                    .bind(request.item.as_slice()).fetch_one(&mut **tx).await?;
                let revision=revision.parse().map_err(|_|DurabilityError::InvalidStoredState)?;
                let at=SpellGroundTarget {spatial_position:ground.spatial_position.clone(),map_revision:ground.map_revision.clone(),content_revision:ground.content_revision.clone(),placement_context:ground.placement_context.clone()};
                let corpse=prepare_moved_arrival_corpse_in_transaction(&mut tx,&authority,&at,qualified,(request.item,revision),&moved).await.map_err(spell_error)?;
                let mut inputs=Vec::with_capacity(corpse.contents.len()+1);
                inputs.push(snapshot(&mut tx,corpse.item_instance_id,"ground").await?);
                for child in &corpse.contents {
                    inputs.push(snapshot(&mut tx,child.item_instance_id,if child.corpse_entry {"corpse"} else {"container"}).await?);
                }
                let (transaction,event,occurred):(String,String,i64)=sqlx::query_as("SELECT game_character_uuid_v7()::text,game_character_uuid_v7()::text,floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
                    .fetch_one(&mut **tx).await?;
                let inputs_intent=serde_json::to_vec(&inputs).map_err(|_|DurabilityError::InvalidStoredState)?;
                let hash:[u8;32]=Sha256::digest(&inputs_intent).into();
                let tuple=tx.tuple();
                let proof=json!({"rule":target.rule,"move":moved.transaction_id,"corpse":corpse.item_instance_id,"inputs_sha256":hash,"readset":target.readset,"reset_epoch":target.reset_epoch,"content_digest":content.loop_content_digest()});
                let envelope=crate::durability::item_craft::audit::encode_map_harvest(fence,command,proof,&transaction,&event,occurred,tuple.tuple())?;
                sqlx::query("SET CONSTRAINTS ALL DEFERRED").execute(&mut **tx).await?;
                sqlx::query("INSERT INTO game_elemental_altar_receipts(transaction_id,event_id,move_transaction_id,rule,reset_epoch,readset,inputs,inputs_intent,inputs_sha256,occurred_at,envelope_sha256) VALUES($1::uuid,$2::uuid,encode($3,'hex')::uuid,$4,$5,$6,$7,$8,$9,$10,sha256($11))")
                    .bind(&transaction).bind(&event).bind(moved.transaction_id.as_slice()).bind(&target.rule).bind(target.reset_epoch).bind(json!(target.readset)).bind(json!(inputs)).bind(&inputs_intent).bind(hash.as_slice()).bind(occurred).bind(&envelope).execute(&mut **tx).await?;
                sqlx::query("INSERT INTO game_elemental_boss_latches(world_id,channel_id,rule,reset_epoch,arrival_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5::uuid)")
                    .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(&target.rule).bind(target.reset_epoch).bind(&transaction).execute(&mut **tx).await?;
                // Children are reserved deepest first; the root is retired last.
                for input in inputs.iter().skip(1).chain(inputs.first()) {
                    let item=input["item"]["item_instance_id"].as_str().ok_or(DurabilityError::InvalidStoredState)?;
                    sqlx::query("UPDATE game_item_instances SET quantity=0,lifecycle=2,last_transaction_id=$2::uuid WHERE item_instance_id=$1::uuid")
                        .bind(item).bind(&transaction).execute(&mut **tx).await?;
                    let query=match input["kind"].as_str() {
                        Some("ground")=>"DELETE FROM game_item_ground_locations WHERE item_instance_id=$1::uuid",
                        Some("corpse")=>"DELETE FROM game_item_corpse_container_entries WHERE item_instance_id=$1::uuid",
                        Some("container")=>"DELETE FROM game_item_container_entries WHERE item_instance_id=$1::uuid",
                        _=>return Err(DurabilityError::InvalidStoredState.into()),
                    };
                    sqlx::query(query).bind(item).execute(&mut **tx).await?;
                }
                sqlx::query("INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state) VALUES($1::uuid,$2::uuid,1,1,2,$3,$4,encode($5,'hex')::uuid,$6,$6+$7,$8,sha256($8),1)")
                    .bind(&event).bind(&transaction).bind(i64::from(tuple.schema_revision())).bind(tuple.retention_profile_id()).bind(corpse.item_instance_id.as_slice()).bind(occurred).bind(crate::durability::item_mint_audit::AUDIT_RETENTION_P90D_MS).bind(&envelope).execute(&mut **tx).await?;
                Ok(AltarArrivalReceipt{moved,summon:Some(PendingAltarBoss{transaction_id:uuid_text(&transaction)?,rule:target.rule})})
            }.await;
            match attempt {
                Ok(receipt)=>{tx.commit(deadline).await?;Ok(Ok(receipt))},
                Err(error)=>loop_error(error).map(Err),
            }
        })).await
    }
}
async fn snapshot(
    tx: &mut Type2Transaction<'_>,
    id: [u8; 16],
    kind: &str,
) -> LoopResult<serde_json::Value> {
    let query = match kind {
        "ground" => {
            "SELECT jsonb_build_object('item',to_jsonb(i),'custody',to_jsonb(c),'kind','ground') FROM game_item_instances i JOIN game_item_ground_locations c USING(item_instance_id) WHERE i.item_instance_id=encode($1,'hex')::uuid"
        }
        "corpse" => {
            "SELECT jsonb_build_object('item',to_jsonb(i),'custody',to_jsonb(c),'kind','corpse') FROM game_item_instances i JOIN game_item_corpse_container_entries c USING(item_instance_id) WHERE i.item_instance_id=encode($1,'hex')::uuid"
        }
        "container" => {
            "SELECT jsonb_build_object('item',to_jsonb(i),'custody',to_jsonb(c),'kind','container') FROM game_item_instances i JOIN game_item_container_entries c USING(item_instance_id) WHERE i.item_instance_id=encode($1,'hex')::uuid"
        }
        _ => return Err(DurabilityError::InvalidStoredState.into()),
    };
    Ok(sqlx::query_scalar(query)
        .bind(id.as_slice())
        .fetch_one(&mut ***tx)
        .await?)
}

fn spell_error(error: SpellItemError) -> LoopError {
    match error {
        SpellItemError::Rejected(_) | SpellItemError::FieldCapabilityRefused => {
            LoopError::Refused(LoopMoveRefusal::Rejected)
        }
        SpellItemError::Database(error) => LoopError::Database(error),
        SpellItemError::Durability(error) => LoopError::Durability(error),
        SpellItemError::Training(_) => LoopError::Durability(DurabilityError::Unavailable),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn altar_target_requires_eight_unique_current_operands_and_a_known_rule() {
        let mut target = AltarArrivalTarget::new(
            elemental_spheres::rules()[0].key,
            (1..=8).map(|key| (key, 0)).collect(),
            0,
        );
        assert!(target.valid());
        target.readset[7] = target.readset[0];
        assert!(!target.valid());
        target.readset[7] = (8, u64::MAX);
        assert!(!target.valid());
        target.readset[7] = (8, 0);
        target.rule = "unbound".into();
        assert!(!target.valid());
    }
}
