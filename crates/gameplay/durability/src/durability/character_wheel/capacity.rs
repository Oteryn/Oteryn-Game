//! Wheel capacity shares the item's already-fenced Character transaction.
use super::*;

/// Caller holds the current Character root and proved live Premium for its account.
/// Crystal 201189f player.cpp:5401-5408 converts Wheel capacity to hundredths of an ounce.
pub(in crate::durability) async fn read_capacity_bonus_in_transaction(
    tx: &mut Transaction<'_>,
    character: CharacterId,
    vocation: &str,
    level: u32,
) -> std::result::Result<u64, DurabilityError> {
    if level <= WHEEL_LEVEL_OFFSET
        || !super::super::character_promotion::read_durable_promotion(tx, character).await?
    {
        return Ok(0);
    }
    let data = oteryn_gameplay_progression::wheel_gem_data::WheelGemData::embedded()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let ruleset = WheelRuleset::from_catalogue(data.revision(), data.wheel())
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let stored = read_wheel_state(tx, character, false).await?;
    let allocation = allocation_from_stored(tx, stored, &ruleset).await?;
    let mut effects = oteryn_gameplay_progression::wheel_effects::derive_runtime_effects(
        data.wheel(), &allocation, vocation, level, true, true,
    ).ok_or(DurabilityError::InvalidStoredState)?;
    if allocation.current
        && let Some(atelier) = super::super::character_atelier::load_in_transaction(tx, character).await?
    {
        effects.contribute_gems(&atelier, &data, &allocation)
            .ok_or(DurabilityError::InvalidStoredState)?;
    }
    u64::from(effects.capacity).checked_mul(100)
        .ok_or(DurabilityError::InvalidStoredState)
}
