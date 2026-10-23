//! Reopening Character authority verifies permanent promotion and the paid receipt source.
use crate::durability::DurabilityError;

pub(in crate::durability) async fn verify_purchase_integrity(
    connection: &mut sqlx::PgConnection,
) -> Result<(), DurabilityError> {
    let invalid = sqlx::query(
        "SELECT 1 FROM game_character_promotions p \
        LEFT JOIN game_character_npc_purchase_receipts r \
          ON r.character_id=p.character_id AND r.purchase_occurrence_id=p.purchase_occurrence_id \
        WHERE r.purchase_occurrence_id IS NULL OR r.cause_kind<>2 \
        UNION ALL SELECT 1 FROM game_character_npc_purchase_receipts r \
        LEFT JOIN game_item_fee_burns f ON f.transaction_id=r.fee_transaction_id \
        LEFT JOIN game_character_promotions p \
          ON p.character_id=r.character_id AND p.purchase_occurrence_id=r.purchase_occurrence_id \
        WHERE f.transaction_id IS NULL OR f.cause_kind<>r.cause_kind \
          OR f.character_id<>r.character_id OR f.cause_occurrence_id<>r.purchase_occurrence_id \
          OR f.purchase_service_key IS DISTINCT FROM r.service_key \
          OR f.committed_character_revision<>r.committed_character_revision \
          OR f.fee_gold_units<>r.fee_gold_units OR f.created_xact_id<>r.created_xact_id \
          OR (r.cause_kind=2 AND p.character_id IS NULL) LIMIT 1",
    )
    .fetch_optional(connection)
    .await?
    .is_some();
    if invalid {
        Err(DurabilityError::InvalidStoredState)
    } else {
        Ok(())
    }
}
