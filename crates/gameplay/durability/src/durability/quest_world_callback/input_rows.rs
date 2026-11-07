//! Row mutation shared by the separately fenced world and NPC input owners.
use super::*;

pub(super) async fn apply_rows(
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
    if input.replacement.is_none() && input.quantity == i64::from(input.count) {
        let query = if input.kind == "ground" {
            "DELETE FROM game_item_ground_locations WHERE item_instance_id=$1::uuid"
        } else {
            location_queries(&input.kind)?.1
        };
        let removed = sqlx::query(query)
            .bind(&input.item)
            .execute(&mut **tx)
            .await?;
        if removed.rows_affected() != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    Ok(())
}
