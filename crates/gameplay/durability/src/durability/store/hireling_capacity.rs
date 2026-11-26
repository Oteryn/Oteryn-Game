//! Crystal hireling.lua:562-575: both capacity and StoreInbox room precede payment.
use super::super::DurabilityError;
use super::super::character_progression::CurrentCharacterGameplayFence;
use super::super::item_loop_move::{LoopError, carried_weight};
use super::super::item_mint::TypedDefinitionRef;
use super::super::item_transfer::{ItemWeightLookup, read_derived_transfer_capacity_centi_oz};
use sqlx::{Postgres, Transaction};

pub(super) async fn can_reserve_lamp(
    tx: &mut Transaction<'_, Postgres>,
    fence: &CurrentCharacterGameplayFence,
    weights: Option<&dyn ItemWeightLookup>,
) -> Result<bool, DurabilityError> {
    let Some(weights) = weights else {
        return Ok(false);
    };
    let lamp = TypedDefinitionRef {
        family: "Item".into(),
        production_key: "oteryn:item.tibia.i29432".into(),
        revision_ref: "definition-r1".into(),
    };
    can_receive_item(tx, fence, weights, &lamp).await
}

async fn can_receive_item(
    tx: &mut Transaction<'_, Postgres>,
    fence: &CurrentCharacterGameplayFence,
    weights: &dyn ItemWeightLookup,
    definition: &TypedDefinitionRef,
) -> Result<bool, DurabilityError> {
    if inbox_occupancy(tx, fence.character_id.as_bytes()).await? >= 20 {
        return Ok(false);
    }
    let Ok(unit) = weights.weight_of(definition) else {
        return Ok(false);
    };
    let carried = match carried_weight(
        tx,
        fence.character_id,
        *fence.runtime_scope.world_id().as_bytes(),
        weights,
    )
    .await
    {
        Ok(value) => value,
        Err(LoopError::Durability(error)) => return Err(error),
        Err(LoopError::Database(error)) => return Err(error.into()),
        Err(_) => return Ok(false),
    };
    let capacity = read_derived_transfer_capacity_centi_oz(tx, fence.character_id, weights).await?;
    Ok(carried
        .checked_add(u64::from(unit))
        .is_some_and(|total| total <= capacity))
}

/// Entries and pending paid Store reservations consume the same 20 slots.
pub(super) async fn inbox_occupancy(
    tx: &mut Transaction<'_, Postgres>,
    character: &[u8; 16],
) -> Result<i64, DurabilityError> {
    Ok(sqlx::query_scalar(
        "SELECT (SELECT count(*) FROM game_item_character_inbox_locations WHERE character_id=encode($1,'hex')::uuid)
         +(SELECT count(*) FROM game_hireling_store_reservations WHERE character_id=encode($1,'hex')::uuid AND state=1 AND kind=1)
         +(SELECT count(*) FROM game_store_item_reservations r JOIN store_purchase_requests p USING(purchase_request_id)
           WHERE r.character_id=encode($1,'hex')::uuid
             AND NOT EXISTS(SELECT 1 FROM game_store_restore_conflicts conflict
                 WHERE conflict.operation_id=(convert_from(p.answer,'UTF8')::jsonb -> 'operation_ids' ->> 0)::uuid))",
    ).bind(character.as_slice()).fetch_one(&mut **tx).await?)
}

impl super::super::DurabilityRoot {
    /// Reserve one Inbox slot together with the exact Platform request journal.
    #[allow(clippy::too_many_arguments, reason = "the buyer, session and active item definition are independent authorities")]
    pub async fn reserve_store_item_purchase(
        &self,
        authority: &super::super::character_authority::ReconciledCharacterAuthority<'_, '_>,
        node: &super::super::runtime_scope_assignment::NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: super::PurchaseRequest,
        definition: TypedDefinitionRef,
        weights: &dyn ItemWeightLookup,
    ) -> Result<Option<super::PurchaseRequestState>, super::StoreDurabilityError> {
        use super::super::character_authority::assert_recovery_fence;
        use super::super::db::{begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations};
        use super::storage::{PurchaseRequestState, StoreDurabilityError, account_matches, assert_store_fence};

        request.validate()?;
        if request.character_id != *fence.character_id.as_bytes() {
            return Err(StoreDurabilityError::AuthorityRejected);
        }
        let recovery = authority.record_for(self).map_err(|_| StoreDurabilityError::AuthorityRejected)?;
        let node = node.clone();
        let mut weights = weights;
        self.try_issue_semantic_pass()?.run_with_context(&mut weights, move |holder, deadline, weights| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            // The answer writer holds this lock before its history FK locks the buyer.
            super::purchase::lock_request(&mut tx, &request).await?;
            if !assert_store_fence(&mut tx, &fence, &node, None).await?
                || !account_matches(&mut tx, &fence, &request.account_id).await?
            {
                return Ok(Err(StoreDurabilityError::AuthorityRejected));
            }
            super::hireling_reservation::lock_character(&mut tx, request.character_id).await?;
            let state = match super::purchase::read_request(&mut tx, &request).await {
                Ok(state) => state,
                Err(error) => return Ok(Err(error)),
            };
            if let Some(state @ PurchaseRequestState::Answered(_)) = state {
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(Ok(Some(state)));
            }
            let reserved: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM game_store_item_reservations
                 WHERE purchase_request_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid)",
            ).bind(request.purchase_request_id.as_slice()).bind(request.character_id.as_slice())
                .fetch_one(&mut *tx).await?;
            if !reserved {
                // A legacy Sent journal can precede the first Platform call. Replaying
                // it can debit, so require a slot and capacity before making that call.
                if !can_receive_item(&mut tx, &fence, *weights, &definition).await? {
                    return Ok(Ok(None));
                }
                if state.is_none() && let Err(error) = super::purchase::insert_request(&mut tx, &request).await {
                    return Ok(Err(error));
                }
                sqlx::query(
                    "INSERT INTO game_store_item_reservations(purchase_request_id,character_id)
                     VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid)",
                ).bind(request.purchase_request_id.as_slice()).bind(request.character_id.as_slice())
                    .execute(&mut *tx).await?;
            }
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(Some(PurchaseRequestState::Sent)))
        })).await?
    }
}
