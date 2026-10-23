//! Minimum PREY-0/D118 durable hunting state consumed by creature XP.
//! Slot absence is PREY-0 §6.5's uninitialized slot, never a synthetic active bonus.
use super::{DurabilityError, character_progression::CurrentCharacterGameplayFence};
use sqlx::{Postgres, Row, Transaction};

#[derive(Debug, Clone, Copy)]
pub struct HuntingXpModifiers {
    pub prey_slot: Option<i16>,
    pub prey_epoch: Option<i64>,
    pub prey_grade: Option<i16>,
    pub prey_percent: i16,
    pub stamina_minutes: i32,
}

/// Caller owns the fenced Character root lock before reading its typed hunting children.
pub(super) async fn read_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    fence: &CurrentCharacterGameplayFence,
    creature: &str,
    premium: bool,
) -> Result<HuntingXpModifiers, DurabilityError> {
    let stamina: Option<i32> = sqlx::query_scalar(
        "SELECT stamina_minutes FROM game_character_build_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    // OFFLINE-0 §3 defines this absent-build-row seed. Reading it never creates a build
    // row or invents a hunting tick; build/stamina writers own that lifecycle.
    let mut result = HuntingXpModifiers {
        prey_slot: None,
        prey_epoch: None,
        prey_grade: None,
        prey_percent: 0,
        stamina_minutes: stamina.unwrap_or(2520),
    };
    let rows = sqlx::query(
        "SELECT p.slot,p.slot_epoch,p.grade,p.bonus_percent FROM game_character_prey_slots p \
         JOIN game_character_roots r USING(character_id) \
         WHERE p.character_id=encode($1,'hex')::uuid AND p.creature_key=$2 \
         AND p.state='ACTIVE' AND p.bonus_type='XP' AND p.hunting_s_left>0 \
         AND (p.slot=1 OR (p.slot=2 AND $3) OR EXISTS(SELECT 1 FROM game_account_prey_unlocks u JOIN game_character_roots claim ON claim.character_id=u.claiming_character_id \
         WHERE claim.account_id=u.account_id AND claim.world_id=u.world_id AND u.account_id=r.account_id AND u.world_id=r.world_id AND u.unlock_kind='PREY_SLOT_PERMANENT' AND u.target=p.slot)) \
         ORDER BY p.slot LIMIT 3",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(creature)
    .bind(premium)
    .fetch_all(&mut **tx)
    .await?;
    if rows.len() > 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    if let Some(row) = rows.first() {
        result.prey_slot = Some(row.try_get("slot")?);
        result.prey_epoch = Some(row.try_get("slot_epoch")?);
        result.prey_grade = Some(row.try_get("grade")?);
        result.prey_percent = row.try_get("bonus_percent")?;
    }
    Ok(result)
}
