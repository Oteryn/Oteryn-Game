//! Carried-only custody mutation; ground retirement stays behind SpellLanePermit.
use super::{DurabilityError, Input};

pub(super) async fn update(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    input: &Input,
    transaction: &str,
) -> Result<(), DurabilityError> {
    let changed = sqlx::query(
        "UPDATE game_item_instances SET \
         quantity=CASE WHEN $4::text IS NULL THEN quantity-$2 ELSE quantity END, \
         lifecycle=CASE WHEN $4::text IS NULL AND quantity=$2 THEN 2 ELSE 1 END, \
         definition_production_key=coalesce($4,definition_production_key), \
         last_transaction_id=$3::uuid WHERE item_instance_id=$1::uuid AND lifecycle=1 AND quantity=$5",
    )
    .bind(&input.item).bind(i64::from(input.count)).bind(transaction)
    .bind(&input.replacement).bind(input.quantity)
    .execute(&mut **tx).await?;
    if changed.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(())
}

pub(super) async fn apply(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    input: &Input,
    transaction: &str,
) -> Result<(), DurabilityError> {
    // Validate the custody class before any mutation. This selector rejects Ground.
    let retirement = super::location_queries(&input.kind)?.1;
    update(tx, input, transaction).await?;
    if input.replacement.is_none() && input.quantity == i64::from(input.count) {
        let removed = sqlx::query(retirement)
            .bind(&input.item)
            .execute(&mut **tx)
            .await?;
        if removed.rows_affected() != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    Ok(())
}
