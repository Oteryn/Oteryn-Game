//! Carried NPC debits cannot use the ground-mutation spell permit.
use super::*;

pub(super) async fn apply(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    _permit: &crate::durability::quest_state::npc_program::NpcItemPermit,
    input: &Input,
    transaction: &str,
) -> Result<(), DurabilityError> {
    // Resolve the carried location before any write; ground is not admitted.
    let (_, delete) = location_queries(&input.kind)?;
    if input.replacement.is_some() {
        return Err(DurabilityError::InvalidStoredState);
    }
    let changed = sqlx::query(
        "UPDATE game_item_instances SET quantity=quantity-$2, \
         lifecycle=CASE WHEN quantity=$2 THEN 2 ELSE 1 END, \
         last_transaction_id=$3::uuid WHERE item_instance_id=$1::uuid AND lifecycle=1 AND quantity=$4",
    )
    .bind(&input.item).bind(i64::from(input.count)).bind(transaction)
    .bind(input.quantity).execute(&mut **tx).await?;
    if changed.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    if input.quantity == i64::from(input.count) {
        let removed = sqlx::query(delete)
            .bind(&input.item)
            .execute(&mut **tx)
            .await?;
        if removed.rows_affected() != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    Ok(())
}
