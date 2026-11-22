//! Crystal's Store boost ladder and hunting timer. The World reset history owns the
//! purchase day, and only the authoritative creature XP transaction advances the timer.
use super::store::DeliveryIdentity;
use sqlx::{Postgres, Transaction};

// Crystal gamestore.lua XP Boost description and data/events/scripts/player.lua.
const BOOST_MINUTES: i16 = 60;
pub(super) const MIN_STAMINA_MINUTES: i32 = 14 * 60;

pub(super) async fn available_tier(
    tx: &mut Transaction<'_, Postgres>,
    character: &[u8; 16],
    world: &[u8; 16],
) -> Result<Option<(String, i16)>, sqlx::Error> {
    sqlx::query_as(
        "WITH latest AS (
             SELECT reset_epoch FROM game_world_reset_history
             WHERE world_id=encode($2,'hex')::uuid ORDER BY reset_epoch DESC LIMIT 1
         )
         SELECT latest.reset_epoch::text, priced.tier::smallint FROM latest
         CROSS JOIN LATERAL (SELECT 1+count(*) AS tier FROM game_store_service_purchases
             WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid
               AND kind='XP_BOOST' AND reset_epoch=latest.reset_epoch AND operation_id IS NOT NULL) priced
         WHERE priced.tier<=5 AND NOT EXISTS (SELECT 1 FROM game_store_hunting_xp_boosts
             WHERE character_id=encode($1,'hex')::uuid AND remaining_minutes>0)
         AND NOT EXISTS (SELECT 1 FROM game_character_store_xp_boosts
             WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid
               AND expires_at>public.game_daily_effect_now())",
    )
    .bind(character.as_slice())
    .bind(world.as_slice())
    .fetch_optional(&mut **tx)
    .await
}

pub(super) async fn grant(
    tx: &mut Transaction<'_, Postgres>,
    identity: &DeliveryIdentity,
    operation_id: [u8; 16],
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO game_store_hunting_xp_boosts
         (operation_id,character_id,world_id,remaining_minutes,next_minute_at)
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,
                $4,public.game_daily_effect_now()+interval '1 minute')",
    )
    .bind(operation_id.as_slice())
    .bind(identity.character_id.as_slice())
    .bind(identity.world_id.as_slice())
    .bind(BOOST_MINUTES)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Called after replay reconciliation, under the fenced Character lock. One
/// activity spends at most two minutes and advances the same durable deadline.
pub(super) async fn advance(
    tx: &mut Transaction<'_, Postgres>,
    character: &[u8; 16],
    world: &[u8; 16],
    stamina_minutes: i32,
) -> Result<(), sqlx::Error> {
    if stamina_minutes < MIN_STAMINA_MINUTES {
        return Ok(());
    }
    sqlx::query(
        "WITH instant AS MATERIALIZED (SELECT public.game_daily_effect_now() AS at)
         UPDATE game_store_hunting_xp_boosts b
         SET remaining_minutes=greatest(0,b.remaining_minutes-CASE WHEN instant.at>b.next_minute_at+interval '1 minute' THEN 2 ELSE 1 END),
             next_minute_at=instant.at+CASE WHEN instant.at>b.next_minute_at+interval '1 minute' THEN interval '2 minutes' ELSE interval '1 minute' END
         FROM instant WHERE b.character_id=encode($1,'hex')::uuid AND b.world_id=encode($2,'hex')::uuid
             AND b.remaining_minutes>0 AND instant.at>b.next_minute_at",
    )
    .bind(character.as_slice())
    .bind(world.as_slice())
    .execute(&mut **tx)
    .await?;
    Ok(())
}
