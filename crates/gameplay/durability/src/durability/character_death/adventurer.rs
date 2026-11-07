//! PARTY-PVP-0 §10.2 / NIGHT-W1 decision 16: a permanent Open-PvP exemption.
use super::*;
use crate::durability::spell_field_policy::{
    FieldWorldType, read_world_field_policy_in_transaction,
};

pub(super) async fn applies(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    root: &DurabilityRoot,
    fence: CurrentCharacterGameplayFence,
    level: u32,
    pvp_death: bool,
) -> std::result::Result<bool, DurabilityError> {
    if level > 20 || !pvp_death {
        return Ok(false);
    }
    // The death writer already holds the current session/root fence and the
    // admission relation locks shared with live PvP harm. The immutable receipt
    // chain preserves first reaching 21 even after delevelling or a restart;
    // §10.4 needs no extra level-up write. Each EXISTS returns at most one bit,
    // with the receipt scans restricted by their Character/revision indexes.
    let excluded: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_character_pvp_state \
            WHERE character_id = encode($1,'hex')::uuid \
              AND (adventurer_forfeited OR (skull > 0 AND skull_until > clock_timestamp()))) \
         OR EXISTS(SELECT 1 FROM game_character_xp_receipts \
            WHERE character_id = encode($1,'hex')::uuid AND level_after >= 21) \
         OR EXISTS(SELECT 1 FROM game_character_death_receipts \
            WHERE character_id = encode($1,'hex')::uuid AND level_before >= 21)",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_one(&mut **tx)
    .await
    .map_err(DurabilityError::from)?;
    if excluded {
        return Ok(false);
    }
    let policy = read_world_field_policy_in_transaction(
        tx,
        root,
        fence.runtime_scope,
        fence.scope_ownership_generation.get(),
    )
    .await?;
    // Absence of a published Open policy never grants the exemption.
    Ok(policy.is_some_and(|policy| policy.bind(tx).mode() == FieldWorldType::Pvp))
}
