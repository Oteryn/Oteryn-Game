//! An unspent, settled Store purchase authorizes one Daily Reward claim away
//! from its shrine. The ordinary Daily Reward transaction burns that purchase.
use sqlx::{Postgres, Transaction};

pub(in crate::durability) async fn available(
    tx: &mut Transaction<'_, Postgres>,
    character: &[u8; 16],
    world: &[u8; 16],
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT count(*) FROM (SELECT 1 FROM game_store_service_purchases p
         WHERE p.character_id=encode($1,'hex')::uuid AND p.world_id=encode($2,'hex')::uuid
           AND p.kind='INSTANT_REWARD_ACCESS' AND p.operation_id IS NOT NULL
           AND NOT EXISTS(SELECT 1 FROM game_store_reward_access_uses u WHERE u.purchase_id=p.purchase_id)
         LIMIT 90) available",
    )
    .bind(character.as_slice())
    .bind(world.as_slice())
    .fetch_one(&mut **tx)
    .await
}

pub(in crate::durability) async fn consume(
    tx: &mut Transaction<'_, Postgres>,
    character: &[u8; 16],
    world: &[u8; 16],
    claim: &[u8; 16],
) -> Result<bool, sqlx::Error> {
    let used = sqlx::query(
        "INSERT INTO game_store_reward_access_uses(purchase_id,claim_id)
         SELECT p.purchase_id,encode($3,'hex')::uuid FROM game_store_service_purchases p
         WHERE p.character_id=encode($1,'hex')::uuid AND p.world_id=encode($2,'hex')::uuid
           AND p.kind='INSTANT_REWARD_ACCESS' AND p.operation_id IS NOT NULL
           AND NOT EXISTS(SELECT 1 FROM game_store_reward_access_uses u WHERE u.purchase_id=p.purchase_id)
         ORDER BY p.purchase_id LIMIT 1 FOR UPDATE OF p
         ON CONFLICT DO NOTHING",
    )
    .bind(character.as_slice())
    .bind(world.as_slice())
    .bind(claim.as_slice())
    .execute(&mut **tx)
    .await?;
    Ok(used.rows_affected() == 1)
}
