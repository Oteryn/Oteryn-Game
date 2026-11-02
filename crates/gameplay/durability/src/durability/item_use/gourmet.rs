//! E12 gourmet USE: closed source classification and native one-unit consumption.
use super::*;
use super::super::db::{begin_semantic_transaction, commit_semantic_transaction};
use oteryn_gameplay_domain::domain::hireling::food::{
    GourmetBuff, GourmetConditions, GOURMET_BUFF_SECONDS, GOURMET_COOLDOWN_SECONDS,
};

pub const GOURMET_CAUSE: u32 = 471;
/// Only the action's nine registered source ids (`hireling_foods.lua:65-67`).
pub fn is_gourmet_key(key: &str) -> bool { gourmet_item_id(key).is_some() }
pub fn gourmet_item_id(key: &str) -> Option<u32> {
    match key {
        "oteryn:item.tibia.i29408" => Some(29408),
        "oteryn:item.tibia.i29409" => Some(29409),
        "oteryn:item.tibia.i29410" => Some(29410),
        "oteryn:item.tibia.i29411" => Some(29411),
        "oteryn:item.tibia.i29412" => Some(29412),
        "oteryn:item.tibia.i29413" => Some(29413),
        "oteryn:item.tibia.i29414" => Some(29414),
        "oteryn:item.tibia.i29415" => Some(29415),
        "oteryn:item.tibia.i29416" => Some(29416),
        _ => None,
    }
}

/// Native USE and the source-qualified persistent condition successor commit
/// together. The runtime owner supplies remaining ticks from its staged native
/// ConditionStore; this record never grants an arbitrary skill or duration.
#[derive(Debug, Clone)]
pub struct GourmetUseRequest {
    pub item: ItemUseRequest,
    pub wire_binding: [u8; 32],
    pub conditions: GourmetConditions,
}

fn checked_id(request: &GourmetUseRequest) -> Result<u32> {
    request.item.validate()?;
    if !matches!(request.item.kind, ItemUseKind::HirelingFood { .. })
        || request.item.source_definition.family != "Item"
        || request.item.flask != ItemUseFlaskPlan::None
        || !request.conditions.valid()
    { return Err(ItemUseError::InvalidInput); }
    let item_id=gourmet_item_id(&request.item.source_definition.production_key).ok_or(ItemUseError::InvalidInput)?;
    if !new_source_condition_matches(item_id,request.conditions) {return Err(ItemUseError::InvalidInput);}
    Ok(item_id)
}

fn new_source_condition_matches(item_id:u32,conditions:GourmetConditions)->bool {
    let full=Some(GourmetBuff{item_id,remaining_ms:GOURMET_BUFF_SECONDS*1000});
    match item_id {
        29408..=29411 | 29413=>conditions.attributes==full,
        29412=>conditions.haste==full,
        29414..=29416=>true,
        _=>false,
    }
}

fn stored_conditions(row: &sqlx::postgres::PgRow) -> std::result::Result<GourmetConditions, DurabilityError> {
    let attributes = row.try_get::<Option<i32>,_>("attributes_item")?.map(|id| {
        Ok::<_,DurabilityError>(GourmetBuff {
            item_id: u32::try_from(id).map_err(|_|DurabilityError::InvalidStoredState)?,
            remaining_ms: u32::try_from(row.try_get::<i32,_>("attributes_ms")?)
                .map_err(|_|DurabilityError::InvalidStoredState)?,
        })
    }).transpose()?;
    let haste_ms = u32::try_from(row.try_get::<i32,_>("haste_ms")?)
        .map_err(|_|DurabilityError::InvalidStoredState)?;
    let value = GourmetConditions { attributes,
        haste: (haste_ms > 0).then_some(GourmetBuff { item_id:29412, remaining_ms:haste_ms }) };
    if !value.valid() { return Err(DurabilityError::InvalidStoredState); }
    Ok(value)
}

/// A checkpoint may only reduce/clear a condition. One newly consumed source
/// may replace its own family; it cannot refresh the other family's ticks.
fn transition_allowed(before: GourmetConditions, after: GourmetConditions, source: Option<u32>) -> bool {
    let slot = |old: Option<GourmetBuff>, next: Option<GourmetBuff>| match next {
        None => true,
        Some(next) if source == Some(next.item_id) => next.valid(),
        Some(next) => old.is_some_and(|old| old.item_id == next.item_id && next.remaining_ms <= old.remaining_ms),
    };
    after.valid() && slot(before.attributes, after.attributes) && slot(before.haste, after.haste)
}

async fn snapshot(connection: &mut PgConnection, character: &[u8;16], locked: bool)
    -> std::result::Result<GourmetConditions, DurabilityError> {
    let sql = if locked {
        "SELECT attributes_item,attributes_ms,haste_ms FROM game_hireling_gourmet_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE"
    } else {
        "SELECT attributes_item,attributes_ms,haste_ms FROM game_hireling_gourmet_state WHERE character_id=encode($1,'hex')::uuid"
    };
    sqlx::query(sql).bind(character.as_slice()).fetch_optional(connection).await?
        .as_ref().map(stored_conditions).transpose().map(Option::unwrap_or_default)
}

async fn receipt(connection: &mut PgConnection, fence: &CurrentCharacterItemFence,
    command: u64, wire: [u8;32]) -> Result<bool> {
    let row = sqlx::query("SELECT wire_binding,character_id=encode($3,'hex')::uuid AS same_character \
        FROM game_hireling_gourmet_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
        .bind(fence.game_session_id.as_bytes().as_slice()).bind(command.to_string())
        .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(connection).await?;
    let Some(row)=row else{return Ok(false)};
    if !row.try_get::<bool,_>("same_character")? || row.try_get::<Vec<u8>,_>("wire_binding")?.as_slice()!=wire {
        return Err(ItemUseError::ConflictingCommand);
    }
    Ok(true)
}

impl DurabilityRoot {
    /// Receipt-first wire retry, including after the consumed instance retired.
    pub async fn read_gourmet_use_receipt(&self, authority:&ReconciledCharacterAuthority<'_,'_>,
        fence:CurrentCharacterItemFence, command:u64, wire:[u8;32]) -> Result<bool> {
        if command==0 {return Err(ItemUseError::InvalidInput)}
        let recovery=authority.record_for(self).map_err(|_|ItemUseError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            // Serialize a historical read behind a possibly ambiguous native
            // writer; this lock grants no new current-session write authority.
            sqlx::query("SELECT character_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let result=receipt(&mut tx,&fence,command,wire).await;
            commit_semantic_transaction(tx,deadline).await?;
            Ok(result)
        })).await?
    }

    pub async fn commit_gourmet_use(&self, authority:&ReconciledCharacterAuthority<'_,'_>,
        node:&NodeIncarnationProof, fence:CurrentCharacterItemFence, request:GourmetUseRequest)
        -> Result<ItemUseOutcome> {
        let item_id=checked_id(&request)?;
        let ItemUseKind::HirelingFood{origin_channel}=request.item.kind else{return Err(ItemUseError::InvalidInput)};
        let recovery=authority.record_for(self).map_err(|_|ItemUseError::AuthorityRejected)?;
        let node=node.clone();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=begin_type2_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            lock_admission_relations(&mut tx).await?;
            match receipt(&mut tx,&fence,request.item.command_id,request.wire_binding).await {
                Ok(true)=>{tx.commit(deadline).await?;return Ok(Ok(ItemUseOutcome::AlreadyCommitted{shape:ItemUseShape::Burn}))},
                Ok(false)=>{},Err(ItemUseError::Unavailable(e))=>return Err(e),Err(e)=>return Ok(Err(e)),
            }
            if !super::super::house_hireling::fence::assert_inventory_fence(&mut tx,&node,&fence,origin_channel).await? {
                return Ok(Err(ItemUseError::AuthorityRejected));
            }
            let revisions=sqlx::query("SELECT content_revision,ruleset_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
            if revisions.try_get::<String,_>("content_revision")?!=request.item.content_revision
                || revisions.try_get::<String,_>("ruleset_revision")?!=request.item.ruleset_revision {
                return Ok(Err(ItemUseError::StaleState));
            }
            let before=snapshot(&mut tx,fence.character_id.as_bytes(),true).await?;
            if !transition_allowed(before,request.conditions,Some(item_id)) {return Ok(Err(ItemUseError::StaleState));}
            let cooling:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_hireling_gourmet_state \
                WHERE character_id=encode($1,'hex')::uuid AND cooldown_until>public.game_daily_effect_now())")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
            if cooling {return Ok(Err(ItemUseError::Cooldown));}
            let tuple=tx.tuple();
            let outcome=match write(&mut tx,&fence,&request.item,tuple).await {
                Ok(ItemUseOutcome::Written{shape})=>ItemUseOutcome::Written{shape},
                Ok(_)=>return Err(DurabilityError::InvalidStoredState),
                Err(ItemUseError::Unavailable(e))=>return Err(e),Err(e)=>return Ok(Err(e)),
            };
            let attribute=request.conditions.attributes;
            let haste=request.conditions.haste.map_or(0,|b|b.remaining_ms);
            sqlx::query("INSERT INTO game_hireling_gourmet_receipts(transaction_id,character_id,game_session_id,command_id,wire_binding,food_id, \
                started_at,cooldown_until,attributes_item,attributes_ms,haste_ms) \
                SELECT encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric,$5,$6,at, \
                at+make_interval(secs=>$7),$8,$9,$10 FROM(SELECT public.game_daily_effect_now() at) instant")
                .bind(request.item.transaction_id.as_slice()).bind(fence.character_id.as_bytes().as_slice())
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.item.command_id.to_string())
                .bind(request.wire_binding.as_slice()).bind(item_id as i32).bind(f64::from(GOURMET_COOLDOWN_SECONDS))
                .bind(attribute.map(|b|b.item_id as i32)).bind(attribute.map_or(0,|b|b.remaining_ms) as i32).bind(haste as i32)
                .execute(&mut **tx).await?;
            let changed=sqlx::query("UPDATE game_hireling_gourmet_state s SET last_transaction_id=r.transaction_id, \
                cooldown_until=r.cooldown_until,attributes_item=r.attributes_item,attributes_ms=r.attributes_ms,haste_ms=r.haste_ms \
                FROM game_hireling_gourmet_receipts r WHERE r.transaction_id=encode($1,'hex')::uuid AND s.character_id=r.character_id")
                .bind(request.item.transaction_id.as_slice()).execute(&mut **tx).await?.rows_affected();
            if changed==0 {
                sqlx::query("INSERT INTO game_hireling_gourmet_state(character_id,last_transaction_id,cooldown_until,attributes_item,attributes_ms,haste_ms) \
                    SELECT character_id,transaction_id,cooldown_until,attributes_item,attributes_ms,haste_ms FROM game_hireling_gourmet_receipts \
                    WHERE transaction_id=encode($1,'hex')::uuid")
                    .bind(request.item.transaction_id.as_slice()).execute(&mut **tx).await?;
            }
            tx.commit(deadline).await?;
            Ok(Ok(outcome))
        })).await?
    }

    /// Admission data only. The caller proves the new actor and installs this
    /// snapshot once; reading it alone does not grant gameplay authority.
    pub async fn read_gourmet_conditions(&self,authority:&ReconciledCharacterAuthority<'_,'_>,
        character_id:crate::domain::CharacterId) -> Result<GourmetConditions> {
        let recovery=authority.record_for(self).map_err(|_|ItemUseError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            let saved=snapshot(&mut tx,character_id.as_bytes(),false).await?;
            commit_semantic_transaction(tx,deadline).await?;
            Ok(Ok(saved))
        })).await?
    }

    /// Checkpoint of the real native condition owner. It can only decrease or
    /// remove an existing source condition; creation requires a new native USE.
    pub async fn save_gourmet_conditions(&self,authority:&ReconciledCharacterAuthority<'_,'_>,
        node:&NodeIncarnationProof,fence:CurrentCharacterItemFence,origin_channel:[u8;16],
        conditions:GourmetConditions) -> Result<()> {
        if !conditions.valid(){return Err(ItemUseError::InvalidInput)}
        let recovery=authority.record_for(self).map_err(|_|ItemUseError::AuthorityRejected)?;
        let node=node.clone();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            lock_admission_relations(&mut tx).await?;
            // Persist only a monotone projection of this existing actor's
            // conditions. Revoked service ACL cannot trap terminal release;
            // actual new consumption still uses the full service fence above.
            let current = match fence.runtime_scope {
                RuntimeScopeRefV1::Instance { .. } => super::super::house_hireling::fence::assert_house_custody_fence(
                    &mut tx,&node,&fence,origin_channel).await?.is_some(),
                _ => super::super::house_hireling::fence::assert_inventory_fence(
                    &mut tx,&node,&fence,origin_channel).await?,
            };
            if !current { return Ok(Err(ItemUseError::AuthorityRejected)); }
            let before=snapshot(&mut tx,fence.character_id.as_bytes(),true).await?;
            if !transition_allowed(before,conditions,None){return Ok(Err(ItemUseError::StaleState));}
            if before!=conditions {
                sqlx::query("UPDATE game_hireling_gourmet_state SET attributes_item=$2,attributes_ms=$3,haste_ms=$4 \
                    WHERE character_id=encode($1,'hex')::uuid")
                    .bind(fence.character_id.as_bytes().as_slice()).bind(conditions.attributes.map(|b|b.item_id as i32))
                    .bind(conditions.attributes.map_or(0,|b|b.remaining_ms) as i32)
                    .bind(conditions.haste.map_or(0,|b|b.remaining_ms) as i32).execute(&mut *tx).await?;
            }
            commit_semantic_transaction(tx,deadline).await?;
            Ok(Ok(()))
        })).await?
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    #[test]
    fn only_registered_food_keys_can_reach_cause471() {
        for id in 29408..=29416 {assert_eq!(gourmet_item_id(&format!("oteryn:item.tibia.i{id}")),Some(id));}
        for bad in ["oteryn:item.tibia.i35173","oteryn:item.tibia.i029408","oteryn:item.tibia.i29417",""] {assert!(!is_gourmet_key(bad));}
    }
    #[test]
    fn checkpoint_never_grants_refreshes_or_changes_source_identity() {
        let before=GourmetConditions{attributes:Some(GourmetBuff{item_id:29408,remaining_ms:1000}),haste:None};
        assert!(transition_allowed(before,GourmetConditions::default(),None));
        assert!(transition_allowed(before,GourmetConditions{attributes:Some(GourmetBuff{remaining_ms:999,..before.attributes.unwrap()}),haste:None},None));
        assert!(!transition_allowed(before,GourmetConditions{attributes:Some(GourmetBuff{remaining_ms:1001,..before.attributes.unwrap()}),haste:None},None));
        let next=GourmetConditions{attributes:Some(GourmetBuff{item_id:29409,remaining_ms:3_600_000}),haste:None};
        assert!(!transition_allowed(before,next,None));
        assert!(!transition_allowed(before,next,Some(29408)));
        assert!(transition_allowed(before,next,Some(29409)));
    }
    #[test]
    fn a_new_buff_source_requires_its_complete_donor_duration() {
        let full=GourmetConditions{attributes:Some(GourmetBuff{item_id:29409,remaining_ms:3_600_000}),haste:None};
        assert!(new_source_condition_matches(29409,full));
        assert!(!new_source_condition_matches(29408,full));
        assert!(!new_source_condition_matches(29409,GourmetConditions::default()));
        assert!(!new_source_condition_matches(29409,GourmetConditions{attributes:Some(GourmetBuff{remaining_ms:3_599_999,..full.attributes.unwrap()}),haste:None}));
        assert!(new_source_condition_matches(29414,full));
        assert!(new_source_condition_matches(29412,GourmetConditions{haste:Some(GourmetBuff{item_id:29412,remaining_ms:3_600_000}),..full}));
    }
}
