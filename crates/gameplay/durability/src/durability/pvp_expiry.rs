//! Bounded World expiry, taking every candidate state lock before any ledger lock.
use super::pvp_state::{PVP_CLEANUP_BATCH, PvpCleanup};
use super::spell_item_transaction::{
    SpellItemError, SpellItemScopeAuthority, check_scope_transaction,
};

pub async fn cleanup_pvp_rows_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    authority: &SpellItemScopeAuthority,
) -> Result<PvpCleanup, SpellItemError> {
    check_scope_transaction(tx, authority).await?;
    let world = authority.world_bytes();
    // Materialize both bounded candidate sets before acquiring locks. Restrict all
    // subsequent state/ledger writes to this set so none can acquire a new victim
    // lock after a ledger lock. Other Channels use the same CharacterId ordering.
    let candidates: Vec<String> = sqlx::query_scalar(
        "WITH expired_skulls AS MATERIALIZED (\
           SELECT character_id FROM game_character_pvp_state \
            WHERE world_id = encode($1,'hex')::uuid AND skull > 0 AND skull_until <= clock_timestamp() \
            ORDER BY skull_until, character_id LIMIT $2), \
         expired_ledger AS MATERIALIZED (\
           SELECT l.victim_character_id FROM game_character_pvp_ledger l \
           JOIN game_character_pvp_state s ON s.character_id = l.victim_character_id \
           WHERE s.world_id = encode($1,'hex')::uuid \
             AND l.last_at < clock_timestamp() - interval '5 minutes' \
           ORDER BY l.last_at, l.victim_character_id, l.contributor_character_id LIMIT $2) \
         SELECT s.character_id::text FROM game_character_pvp_state s \
          WHERE s.character_id IN (SELECT character_id FROM expired_skulls \
                                  UNION SELECT victim_character_id FROM expired_ledger) \
          ORDER BY s.character_id FOR UPDATE OF s",
    ).bind(world.as_slice()).bind(PVP_CLEANUP_BATCH).fetch_all(&mut **tx).await?;
    let skulls = sqlx::query(
        "UPDATE game_character_pvp_state SET skull = 0, skull_until = NULL, revision = revision + 1 \
         WHERE character_id IN (SELECT character_id FROM game_character_pvp_state \
           WHERE character_id::text = ANY($1) AND skull > 0 AND skull_until <= clock_timestamp() \
           ORDER BY skull_until, character_id LIMIT $2)",
    ).bind(&candidates).bind(PVP_CLEANUP_BATCH).execute(&mut **tx).await?.rows_affected();
    let points = sqlx::query(
        "DELETE FROM game_character_unjustified_points WHERE (character_id, death_occurrence_id) IN \
           (SELECT p.character_id, p.death_occurrence_id FROM game_character_unjustified_points p \
              JOIN game_character_roots r ON r.character_id = p.character_id \
             WHERE r.world_id = encode($1,'hex')::uuid \
               AND p.committed_at < clock_timestamp() - interval '45 days' \
             ORDER BY p.committed_at, p.character_id LIMIT $2 FOR UPDATE OF p)",
    ).bind(world.as_slice()).bind(PVP_CLEANUP_BATCH).execute(&mut **tx).await?.rows_affected();
    let marks = sqlx::query(
        "DELETE FROM game_character_revenge_marks WHERE (victim_character_id, killer_character_id, \
            death_occurrence_id) IN \
           (SELECT m.victim_character_id, m.killer_character_id, m.death_occurrence_id \
              FROM game_character_revenge_marks m \
              JOIN game_character_roots r ON r.character_id = m.victim_character_id \
             WHERE r.world_id = encode($1,'hex')::uuid AND m.expires_at <= clock_timestamp() \
             ORDER BY m.expires_at, m.victim_character_id LIMIT $2 FOR UPDATE OF m)",
    )
    .bind(world.as_slice())
    .bind(PVP_CLEANUP_BATCH)
    .execute(&mut **tx)
    .await?
    .rows_affected();
    let ledger_rows = sqlx::query(
        "DELETE FROM game_character_pvp_ledger WHERE (victim_character_id, contributor_character_id) IN \
           (SELECT victim_character_id, contributor_character_id FROM game_character_pvp_ledger \
             WHERE victim_character_id::text = ANY($1) \
               AND last_at < clock_timestamp() - interval '5 minutes' \
             ORDER BY last_at, victim_character_id, contributor_character_id LIMIT $2 FOR UPDATE)",
    ).bind(&candidates).bind(PVP_CLEANUP_BATCH).execute(&mut **tx).await?.rows_affected();
    Ok(PvpCleanup {
        points,
        marks,
        ledger_rows,
        skulls,
    })
}
