//! Character-serialized eligibility reservations; no network or coin authority lives here.
use super::super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::super::character_progression::CurrentCharacterGameplayFence;
use super::super::db::{begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations};
use super::super::runtime_scope_assignment::NodeIncarnationProof;
use super::super::{DurabilityError, DurabilityRoot};
use super::codec::PurchaseRequest;
use super::hireling_model::{HirelingStoreAction, HirelingStorePurchase};
use super::purchase::record_request;
use super::storage::{PurchaseRequestState, StoreDurabilityError, account_matches,assert_store_fence};
use sqlx::{Postgres, Row, Transaction};
use super::super::item_transfer::ItemWeightLookup;

impl DurabilityRoot {
    /// Reserve a slot/unlock/target before Platform may debit. An uncertain request keeps its
    /// reservation until that same immutable id receives a terminal Platform answer.
    #[allow(clippy::too_many_arguments, reason = "native authority, immutable purchase, and admitted item facts are independently supplied")]
    pub async fn reserve_hireling_store_purchase(
        &self, authority: &ReconciledCharacterAuthority<'_, '_>, node: &NodeIncarnationProof,
        request: PurchaseRequest, action: HirelingStoreAction, fence: CurrentCharacterGameplayFence, origin_channel:[u8;16],
        weights: Option<&dyn ItemWeightLookup>,
    ) -> Result<Option<PurchaseRequestState>, StoreDurabilityError> {
        request.validate()?;
        action.validate()?;
        if request.character_id != *fence.character_id.as_bytes() { return Err(StoreDurabilityError::AuthorityRejected); }
        let recovery = authority.record_for(self).map_err(|_| StoreDurabilityError::AuthorityRejected)?;
        let node = node.clone();
        let mut weights=weights;
        self.try_issue_semantic_pass()?.run_with_context(&mut weights,move |holder, deadline, weights| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if !assert_store_fence(&mut tx, &fence, &node, Some(origin_channel)).await? || !account_matches(&mut tx, &fence, &request.account_id).await? {
                return Ok(Err(StoreDurabilityError::AuthorityRejected));
            }
            lock_character(&mut tx, request.character_id).await?;
            let state = match record_request(&mut tx, &request).await { Ok(v) => v, Err(e) => return Ok(Err(e)) };
            let bytes = action.bytes().map_err(|_| DurabilityError::InvalidStoredState)?;
            let prior = sqlx::query("SELECT action FROM game_hireling_store_reservations WHERE purchase_request_id=encode($1,'hex')::uuid FOR UPDATE")
                .bind(request.purchase_request_id.as_slice()).fetch_optional(&mut *tx).await?;
            if let Some(row) = prior {
                if row.try_get::<Vec<u8>,_>("action")? != bytes { return Ok(Err(StoreDurabilityError::ConflictingBinding)); }
            } else {
                if matches!(state, PurchaseRequestState::Answered(_)) { return Ok(Err(StoreDurabilityError::ConflictingBinding)); }
                if !eligible(&mut tx, request.character_id, &action, &fence, *weights).await? { return Ok(Ok(None)); }
                let target = action.target();
                sqlx::query("INSERT INTO game_hireling_store_reservations(purchase_request_id,character_id,action,kind,entitlement_id,hireling_id,reserved_name,state) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5,encode($6,'hex')::uuid,$7,1)")
                    .bind(request.purchase_request_id.as_slice()).bind(request.character_id.as_slice()).bind(bytes)
                    .bind(action.kind()).bind(action.entitlement()).bind(target.as_ref().map(|id| id.as_slice()))
                    .bind(action.name()).execute(&mut *tx).await?;
            }
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(Some(state)))
        })).await?
    }
}

pub(super) async fn lock_character(tx: &mut Transaction<'_,Postgres>, character: [u8;16]) -> Result<(),DurabilityError> {
    sqlx::query("SELECT character_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid AND lifecycle=1 FOR UPDATE")
        .bind(character.as_slice()).fetch_one(&mut **tx).await?;
    Ok(())
}

async fn eligible(tx: &mut Transaction<'_,Postgres>, character: [u8;16], action: &HirelingStoreAction, fence:&CurrentCharacterGameplayFence, weights:Option<&dyn ItemWeightLookup>) -> Result<bool,DurabilityError> {
    // Display names are not identities (Crystal schema.sql697-713); repeated names are allowed.
    match action {
        HirelingStoreAction::Hire { .. } => {
            let total: i64 = sqlx::query_scalar("SELECT (SELECT count(*) FROM game_house_hirelings WHERE owner_character_id=encode($1,'hex')::uuid)+(SELECT count(*) FROM game_hireling_store_reservations WHERE character_id=encode($1,'hex')::uuid AND state=1 AND kind=1)")
                .bind(character.as_slice()).fetch_one(&mut **tx).await?;
            if total>=10 { return Ok(false); } // Crystal init.lua:215,796-799,1903-1904.
            super::hireling_capacity::can_reserve_lamp(tx,fence,weights).await
        }
        HirelingStoreAction::Unlock { entitlement_id } => {
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_house_hirelings WHERE owner_character_id=encode($1,'hex')::uuid) AND NOT EXISTS(SELECT 1 FROM game_hireling_entitlements WHERE character_id=encode($1,'hex')::uuid AND entitlement_id=$2) AND NOT EXISTS(SELECT 1 FROM game_hireling_store_reservations WHERE character_id=encode($1,'hex')::uuid AND entitlement_id=$2 AND state=1)")
                .bind(character.as_slice()).bind(*entitlement_id as i16).fetch_one(&mut **tx).await.map_err(Into::into)
        }
        HirelingStoreAction::Rename { hireling_id, .. } | HirelingStoreAction::SexChange { hireling_id } => {
            let available: Option<bool> = sqlx::query_scalar("SELECT NOT active AND lamp_item_instance_id IS NOT NULL FROM game_house_hirelings WHERE hireling_id=encode($1,'hex')::uuid AND owner_character_id=encode($2,'hex')::uuid FOR UPDATE")
                .bind(hireling_id.as_slice()).bind(character.as_slice()).fetch_optional(&mut **tx).await?;
            if available != Some(true) { return Ok(false); }
            let busy: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_hireling_store_reservations WHERE hireling_id=encode($1,'hex')::uuid AND state=1)")
                .bind(hireling_id.as_slice()).fetch_one(&mut **tx).await?;
            Ok(!busy)
        }
    }
}

pub(super) async fn release_refused(tx: &mut Transaction<'_,Postgres>, request: &PurchaseRequest, answer: &[u8]) -> Result<(),StoreDurabilityError> {
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_hireling_store_reservations WHERE purchase_request_id=encode($1,'hex')::uuid AND state=1)")
        .bind(request.purchase_request_id.as_slice()).fetch_one(&mut **tx).await?;
    if !exists { return Ok(()); }
    let value: serde_json::Value = serde_json::from_slice(answer).map_err(|_| DurabilityError::InvalidStoredState)?;
    match value.get("result").and_then(serde_json::Value::as_str) {
        Some("ACCEPTED") => {}
        Some("STALE_QUOTE" | "OFFER_UNAVAILABLE" | "PURCHASE_DISABLED" | "INSUFFICIENT_BALANCE" | "ACCOUNT_RESTRICTED") => {
            sqlx::query("UPDATE game_hireling_store_reservations SET state=3 WHERE purchase_request_id=encode($1,'hex')::uuid AND state=1")
                .bind(request.purchase_request_id.as_slice()).execute(&mut **tx).await?;
        }
        _ => return Err(DurabilityError::InvalidStoredState.into()),
    }
    Ok(())
}

pub(super) async fn apply_effect(
    tx: &mut Transaction<'_,Postgres>, character: [u8;16], operation: [u8;16],
    lamp: Option<[u8;16]>, purchase: &HirelingStorePurchase,
) -> Result<(),StoreDurabilityError> {
    use super::super::house_hireling::{grant_store_hireling,grant_store_hireling_entitlement};
    lock_character(tx,character).await?;
    let row = sqlx::query("SELECT character_id=encode($2,'hex')::uuid AS owner,action,state FROM game_hireling_store_reservations WHERE purchase_request_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(purchase.purchase_request_id.as_slice()).bind(character.as_slice()).fetch_optional(&mut **tx).await?.ok_or(StoreDurabilityError::ConflictingBinding)?;
    if !row.try_get::<bool,_>("owner")? || row.try_get::<i16,_>("state")? != 1 || row.try_get::<Vec<u8>,_>("action")? != purchase.action.bytes()? {
        return Err(StoreDurabilityError::ConflictingOperation);
    }
    let mut target = purchase.action.target();
    let mut sex = None;
    match &purchase.action {
        HirelingStoreAction::Hire { name,sex: chosen } => {
            // The Inbox owner has minted in this same transaction; a failed final source
            // capacity check rolls back that mint and leaves the paid reservation retryable.
            let inbox:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_character_inbox_locations WHERE character_id=encode($1,'hex')::uuid")
                .bind(character.as_slice()).fetch_one(&mut **tx).await?;
            if inbox>20 {return Err(StoreDurabilityError::AuthorityRejected)}
            grant_store_hireling(tx,character,operation,lamp.ok_or(StoreDurabilityError::ConflictingOperation)?,name,*chosen).await?;
            target = Some(operation); sex = Some(i16::from(*chosen));
        }
        HirelingStoreAction::Unlock { entitlement_id } => {
            if lamp.is_some() { return Err(StoreDurabilityError::ConflictingOperation); }
            grant_store_hireling_entitlement(tx,character,*entitlement_id).await?;
        }
        HirelingStoreAction::Rename { hireling_id,name } => {
            let affected=sqlx::query("UPDATE game_house_hirelings SET name=$3 WHERE hireling_id=encode($1,'hex')::uuid AND owner_character_id=encode($2,'hex')::uuid AND NOT active AND lamp_item_instance_id IS NOT NULL")
                .bind(hireling_id.as_slice()).bind(character.as_slice()).bind(name).execute(&mut **tx).await?.rows_affected();
            if affected!=1 { return Err(StoreDurabilityError::AuthorityRejected); }
        }
        HirelingStoreAction::SexChange { hireling_id } => {
            // Crystal init.lua:2002-2014 resets the selected outfit to the new Citizen look.
            sex=sqlx::query_scalar("UPDATE game_house_hirelings SET sex=CASE sex WHEN 1 THEN 2 ELSE 1 END,look_type=CASE sex WHEN 1 THEN 1107 ELSE 1108 END WHERE hireling_id=encode($1,'hex')::uuid AND owner_character_id=encode($2,'hex')::uuid AND NOT active AND lamp_item_instance_id IS NOT NULL RETURNING sex")
                .bind(hireling_id.as_slice()).bind(character.as_slice()).fetch_optional(&mut **tx).await?;
            if sex.is_none() { return Err(StoreDurabilityError::AuthorityRejected); }
        }
    }
    sqlx::query("UPDATE game_hireling_store_reservations SET state=2,operation_id=encode($2,'hex')::uuid WHERE purchase_request_id=encode($1,'hex')::uuid AND state=1")
        .bind(purchase.purchase_request_id.as_slice()).bind(operation.as_slice()).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO game_store_hireling_effects(operation_id,purchase_request_id,character_id,kind,hireling_id,entitlement_id,name,sex) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,encode($5,'hex')::uuid,$6,$7,$8)")
        .bind(operation.as_slice()).bind(purchase.purchase_request_id.as_slice()).bind(character.as_slice())
        .bind(purchase.action.kind()).bind(target.as_ref().map(|id| id.as_slice())).bind(purchase.action.entitlement()).bind(purchase.action.name()).bind(sex).execute(&mut **tx).await?;
    Ok(())
}
