//! Existing audited carried-cash/bank fee owner as a Source-NPC child.
use super::super::super::item_fee_burn::{
    FeeBurnCause, FeeBurnError, FeeBurnOutcome, FeeBurnRequest, burn_fee_in_transaction,
};
use super::*;

pub(super) enum SourceFeeDisposition {
    Burned,
    /// The parent MUST leave the physical transaction uncommitted; earlier
    /// material costs or private observations cannot survive this refusal.
    InsufficientGold,
}

pub(super) async fn burn_source_fee(
    tx: &mut Type2Transaction<'_>,
    context: &NpcProgramContext<'_>,
    transition: &source::Transition,
    payment: &NpcSourcePaymentFacts,
) -> Result<SourceFeeDisposition, DurabilityError> {
    let npc = context
        .npc_key
        .strip_prefix("oteryn:npc.")
        .ok_or(DurabilityError::InvalidStoredState)?;
    if context.source_key != transition.source_key
        || context.fee_transaction_id != Some(payment.transaction)
        || payment.transaction == context.npc_transaction
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    let service = format!("oteryn:service.npc-cosmetic.{npc}.{}", transition.id);
    let price = super::source_fee_price(&service).ok_or(DurabilityError::InvalidStoredState)?;
    // The sealed source policy supplies the cash-only minimum. The mature
    // shared cash planner and SQL completion independently enforce it.
    if super::source_fee_cash_minimum(&service).is_none_or(|minimum| minimum > price) {
        return Err(DurabilityError::InvalidStoredState);
    }
    sqlx::query("SELECT game_npc_source_bind_fee(encode($1,'hex')::uuid)")
        .bind(payment.transaction.as_slice())
        .execute(&mut ***tx)
        .await?;
    let request = FeeBurnRequest {
        cause: FeeBurnCause::NpcCosmeticPurchase {
            service,
            occurrence: payment.occurrence,
        },
        fee_gold_units: price,
        transaction_id: payment.transaction,
        event_id: payment.event,
        occurred_at_unix_ms: payment.occurred_at_unix_ms,
        server_build_id: payment.server_build_id.clone(),
        change: payment.change.clone(),
    };
    match burn_fee_in_transaction(tx, &context.fence, &request).await {
        Ok(FeeBurnOutcome::Burned(committed))
            if committed.committed_character_revision.get() == context.next_revision =>
        {
            Ok(SourceFeeDisposition::Burned)
        }
        Ok(_) => Err(DurabilityError::InvalidStoredState),
        Err(FeeBurnError::InsufficientFunds) => Ok(SourceFeeDisposition::InsufficientGold),
        Err(FeeBurnError::Unavailable(error)) => Err(error),
        Err(_) => Err(DurabilityError::InvalidStoredState),
    }
}
