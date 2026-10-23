//! Lifted from the existing PVP-1a ledger-cap fixture; one SQL scaffold for both owners.
use super::id;

pub(crate) async fn insert(
    pool: &sqlx::PgPool,
    victim: u8,
    contributor: u8,
    compact: bool,
    compact_kind: Option<i16>,
    age: &str,
) -> Result<sqlx::postgres::PgQueryResult, sqlx::Error> {
    let sql = format!(
        "INSERT INTO game_character_pvp_ledger (victim_character_id, contributor_character_id, \
            buckets, last_at, compact, compact_kind) \
         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, '[]', \
            clock_timestamp() - interval '{age}', $3, CASE WHEN $3 THEN $4 END)"
    );
    sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(id(victim).as_slice())
        .bind(id(contributor).as_slice())
        .bind(compact)
        .bind(compact_kind)
        .execute(pool)
        .await
}
