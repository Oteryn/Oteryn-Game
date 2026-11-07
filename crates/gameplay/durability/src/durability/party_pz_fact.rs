//! PARTY-XP-1 consumes the existing PvP owner deadline, including safe absence.
use super::{
    DurabilityError,
    spell_item_transaction::{SpellItemError, SpellItemScopeAuthority, check_scope_transaction},
};
use sqlx::{Postgres, Transaction};

/// The PvP deadline writer takes the same lock before insert/update. Unlike a row lock,
/// this also protects a Character whose PvP row does not yet exist.
pub(super) async fn lock_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    character: &[u8; 16],
) -> Result<(), DurabilityError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:party-xp-pz:' || encode($1,'hex'),0))")
        .bind(character.as_slice())
        .execute(&mut **tx)
        .await?;
    Ok(())
}
pub async fn leader_pz_blocked_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    scope: &SpellItemScopeAuthority,
    leader: &[u8; 16],
) -> Result<bool, SpellItemError> {
    check_scope_transaction(tx, scope).await?;
    // A kill can read leaders from several contributing parties. Refuse contention
    // rather than waiting in party order against the PvP writer's CharacterId order.
    let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock(hashtextextended('oteryn:party-xp-pz:' || encode($1,'hex'),0))")
        .bind(leader.as_slice()).fetch_one(&mut **tx).await?;
    if !acquired {
        return Err(SpellItemError::Rejected("XP PZ predecessor busy"));
    }
    Ok(sqlx::query_scalar("SELECT pz_block_until>clock_timestamp() FROM game_character_pvp_state WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid FOR SHARE")
        .bind(leader.as_slice()).bind(scope.world_bytes().as_slice()).fetch_optional(&mut **tx).await?.flatten().unwrap_or(false))
}
