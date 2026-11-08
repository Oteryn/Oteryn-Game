//! Quantity/absence is checked on the same locked rows used by consumption.
use super::DurabilityError;
use oteryn_gameplay_quests::quest::reward_callbacks::InventoryRequirement;
use sqlx::{Row, postgres::PgRow};
use std::collections::BTreeMap;

pub(super) fn holds(
    rows: &[PgRow],
    requirements: &[InventoryRequirement],
) -> Result<bool, DurabilityError> {
    if requirements.is_empty() {
        return Ok(true);
    }
    let mut quantities = BTreeMap::<String, u64>::new();
    for row in rows {
        let quantity = u64::try_from(row.try_get::<i64, _>("quantity")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let total = quantities
            .entry(row.try_get("definition_production_key")?)
            .or_default();
        *total = total
            .checked_add(quantity)
            .ok_or(DurabilityError::InvalidStoredState)?;
    }
    Ok(requirements
        .iter()
        .all(|requirement| requirement.holds(&quantities)))
}
