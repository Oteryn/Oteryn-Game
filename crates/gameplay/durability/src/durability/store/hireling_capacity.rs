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
    // Crystal items.xml StoreInbox23396 has20 slots. Outstanding lamp reservations consume
    // eligibility too, so two accepted Hireling requests cannot both reserve its last slot.
    let occupied:i64=sqlx::query_scalar(
        "SELECT (SELECT count(*) FROM game_item_character_inbox_locations WHERE character_id=encode($1,'hex')::uuid) \
         +(SELECT count(*) FROM game_hireling_store_reservations WHERE character_id=encode($1,'hex')::uuid AND state=1 AND kind=1)",
    ).bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    if occupied >= 20 {
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

impl super::super::DurabilityRoot {
    /// Validate a fresh Store item's capacity and Inbox room under the current buyer fence.
    #[allow(clippy::too_many_arguments, reason = "the buyer, session and active item definition are independent authorities")]
    pub async fn store_item_fits_before_debit(
        &self,
        authority: &super::super::character_authority::ReconciledCharacterAuthority<'_, '_>,
        node: &super::super::runtime_scope_assignment::NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        account: [u8; 16],
        origin: [u8; 16],
        definition: TypedDefinitionRef,
        weights: &dyn ItemWeightLookup,
    ) -> Result<bool, super::StoreDurabilityError> {
        use super::super::character_authority::assert_recovery_fence;
        use super::super::db::{begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations};
        use super::storage::{StoreDurabilityError, account_matches, assert_store_fence};

        let recovery = authority.record_for(self).map_err(|_| StoreDurabilityError::AuthorityRejected)?;
        let node = node.clone();
        let mut weights = weights;
        self.try_issue_semantic_pass()?.run_with_context(&mut weights, move |holder, deadline, weights| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if !assert_store_fence(&mut tx, &fence, &node, Some(origin)).await?
                || !account_matches(&mut tx, &fence, &account).await?
            {
                return Ok(Err(StoreDurabilityError::AuthorityRejected));
            }
            super::hireling_reservation::lock_character(&mut tx, *fence.character_id.as_bytes()).await?;
            let fits = can_receive_item(&mut tx, &fence, *weights, &definition).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(fits))
        })).await?
    }
}
