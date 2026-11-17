//! Crystal 9f5a72c grimvale.lua grimvaleSpectators(): source Active2 at30m,
//! Active3 at45m. Geometry and player relocation remain physical owner effects.
use crate::durability::DurabilityError;
use sqlx::{Postgres, Transaction};

/// Caller holds the current firing and proves its World/reset and Channel owner.
/// Reconstruct an upgrading occurrence from that actual firing; never change its
/// chance or selection time. The World clock only advances, including on replay.
pub(super) async fn advance_active_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    firing: [u8; 16],
    now_ms: i64,
) -> Result<(), DurabilityError> {
    sqlx::query(
        "INSERT INTO game_world_feroxa_occurrences AS current(world_id,reset_epoch,phase,selected_at_ms)
         SELECT f.world_id,f.reset_epoch,
             (CASE WHEN $2-f.fired_at_ms>=2700000 THEN 3
                   WHEN $2-f.fired_at_ms>=1800000 THEN 2 ELSE 1 END)::smallint,
             f.fired_at_ms
         FROM game_world_raid_firings f
         WHERE f.firing_id=encode($1,'hex')::uuid AND f.raid_key=$3 AND f.state=1
         ON CONFLICT(world_id,reset_epoch) DO UPDATE SET
             phase=EXCLUDED.phase, selected_at_ms=EXCLUDED.selected_at_ms
         WHERE current.phase<EXCLUDED.phase
           AND (current.selected_at_ms IS NULL OR current.selected_at_ms=EXCLUDED.selected_at_ms)",
    )
    .bind(firing.as_slice())
    .bind(now_ms)
    .bind(super::super::raid_schedule::FEROXA_KEY)
    .execute(&mut **tx)
    .await?;
    Ok(())
}
