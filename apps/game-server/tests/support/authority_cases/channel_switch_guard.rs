//! chsw / GAME-CHANNEL-01 §8-§11: the durable anti-hopping guard advances inside the fresh
//! admission receipt insert, against the production SQLx migration. The first admission sets the
//! baseline, the same Channel is not a switch, a different Channel is a switch even after a relog,
//! a refused or failed destination leaves the guard unchanged and a retry yields one outcome.
use crate::bestiary_postgres_harness::{
    CHARACTER, Harness, TestResult, WORLD, configured_admin, id, runtime,
};

const CHANNEL_A: u8 = 61;
const CHANNEL_B: u8 = 62;

fn hex(bytes: [u8; 16]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// One fresh admission receipt of the fixture Character into `channel`, as admission `seq`.
async fn admit<'e>(
    executor: impl sqlx::PgExecutor<'e>,
    seq: u8,
    channel: u8,
) -> Result<(), sqlx::Error> {
    let mut session = id(100 + seq);
    session[1] = 0xc5;
    sqlx::query(
        "INSERT INTO game_durability_fresh_admission_receipts VALUES(\
         '\\x01'::bytea || decode(repeat($1, 32), 'hex'), $2::uuid, $3::uuid, $4::uuid, $5::uuid, \
         $6::uuid, 1, 1, 1, decode(repeat('ab', 16), 'hex'), 1, '{}', 1)",
    )
    .bind(format!("{seq:02x}"))
    .bind(hex(session))
    .bind(hex(id(7)))
    .bind(hex(id(CHARACTER)))
    .bind(hex(id(WORLD)))
    .bind(hex(id(channel)))
    .execute(executor)
    .await
    .map(|_| ())
}

/// The remembered Channel seed, the switch count and whether a cooldown runs.
async fn guard(pool: &sqlx::PgPool) -> TestResult<Option<(String, i64, bool)>> {
    Ok(sqlx::query_as(
        "SELECT g.last_channel_id::text, g.switch_count, s.cooldown_left_ms > 0 \
         FROM game_channel_switch_guards g, game_channel_switch_status($1::uuid, $2::uuid) s \
         WHERE g.character_id=$1::uuid AND g.world_id=$2::uuid",
    )
    .bind(hex(id(CHARACTER)))
    .bind(hex(id(WORLD)))
    .fetch_optional(pool)
    .await?)
}

/// Seeds the PvP state as the death fixture does: its writer guards only run inside a contributor
/// death transaction, which this case does not exercise (`pvp_death_postgres_cases.rs`).
async fn pvp_fixture(pool: &sqlx::PgPool, sql: &'static str) -> TestResult {
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(sql)
        .bind(hex(id(CHARACTER)))
        .bind(hex(id(WORLD)))
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}


/// The durable preflight read as `oteryn_game_runtime`: cooldown left, lock, destination exists.
async fn preflight(
    pool: &sqlx::PgPool,
    target: Option<u8>,
) -> TestResult<(i64, Option<String>, bool)> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    let row: (i64, Option<String>, bool) =
        sqlx::query_as(oteryn_gameplay_durability::durability::channel_switch::PREFLIGHT_SQL)
            .bind(id(CHARACTER).as_slice())
            .bind(id(WORLD).as_slice())
            .bind(target.map(id).as_ref().map(|t| t.as_slice()))
            .fetch_one(&mut *tx)
            .await?;
    tx.rollback().await?;
    Ok(row)
}

fn channel_text(seed: u8) -> String {
    let h = hex(id(seed));
    format!("{}-{}-{}-{}-{}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32])
}

fn code(result: Result<(), sqlx::Error>) -> Option<String> {
    result
        .err()
        .and_then(|e| e.as_database_error().and_then(|e| e.code()).map(|c| c.into_owned()))
}

#[test]
fn channel_switch_guard_classifies_admissions_and_enforces_the_cooldown() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            let run = async {
                let h = Harness::create(admin, "channel_switch_guard", true).await?;
                let result: TestResult = async {
                    let pool = &h.pool;
                    // The first admission sets the baseline and is not a switch.
                    admit(pool, 1, CHANNEL_A).await?;
                    assert_eq!(guard(pool).await?, Some((channel_text(CHANNEL_A), 0, false)));
                    // The same Channel again (a relog or reconnect) is not a switch.
                    admit(pool, 2, CHANNEL_A).await?;
                    assert_eq!(guard(pool).await?, Some((channel_text(CHANNEL_A), 0, false)));
                    // A failed destination (the admission rolls back) leaves the guard unchanged.
                    let mut tx = pool.begin().await?;
                    admit(&mut *tx, 3, CHANNEL_B).await?;
                    tx.rollback().await?;
                    assert_eq!(guard(pool).await?, Some((channel_text(CHANNEL_A), 0, false)));
                    // A fresh admission into another Channel is a switch and starts the cooldown.
                    admit(pool, 4, CHANNEL_B).await?;
                    assert_eq!(guard(pool).await?, Some((channel_text(CHANNEL_B), 1, true)));
                    // A crash and retry of the same admission yields the one outcome it had.
                    assert_eq!(code(admit(pool, 4, CHANNEL_B).await).as_deref(), Some("23505"));
                    assert_eq!(guard(pool).await?, Some((channel_text(CHANNEL_B), 1, true)));
                    // Back to the remembered Channel is not a switch; relog elsewhere is refused
                    // while the cooldown runs, and the refusal uses no cooldown.
                    admit(pool, 5, CHANNEL_B).await?;
                    assert_eq!(code(admit(pool, 6, CHANNEL_A).await).as_deref(), Some("OTH01"));
                    assert_eq!(guard(pool).await?, Some((channel_text(CHANNEL_B), 1, true)));
                    // Five minutes later by the server clock the switch is allowed again.
                    sqlx::query("UPDATE game_channel_switch_guards SET switched_at=switched_at-300000")
                        .execute(pool)
                        .await?;
                    assert_eq!(guard(pool).await?, Some((channel_text(CHANNEL_B), 1, false)));
                    // A red skull cannot escape into a Channel that is not Open PvP.
                    pvp_fixture(
                        pool,
                        "INSERT INTO game_character_pvp_state(character_id,world_id,skull,skull_until) \
                         VALUES($1::uuid,$2::uuid,1,clock_timestamp()+interval '1 day')",
                    )
                    .await?;
                    assert_eq!(code(admit(pool, 7, CHANNEL_A).await).as_deref(), Some("OTH02"));
                    sqlx::query("INSERT INTO game_channel_pvp_types VALUES($1::uuid,$2::uuid,2)")
                        .bind(hex(id(WORLD)))
                        .bind(hex(id(CHANNEL_A)))
                        .execute(pool)
                        .await?;
                    // The preflight the server runs as the GameNode runtime role reads the lock.
                    assert_eq!(preflight(pool, Some(CHANNEL_B)).await?, (0, Some("SKULL".into()), false));
                    admit(pool, 8, CHANNEL_A).await?;
                    assert_eq!(guard(pool).await?, Some((channel_text(CHANNEL_A), 2, true)));
                    // An open in-fight logout block is a combat lock.
                    pvp_fixture(
                        pool,
                        "UPDATE game_character_pvp_state SET skull=0,skull_until=NULL,\
                         logout_block_until=clock_timestamp()+interval '1 minute' \
                         WHERE character_id=$1::uuid AND world_id=$2::uuid",
                    )
                    .await?;
                    sqlx::query("UPDATE game_channel_switch_guards SET switched_at=switched_at-300000")
                        .execute(pool)
                        .await?;
                    assert_eq!(code(admit(pool, 9, CHANNEL_B).await).as_deref(), Some("OTH02"));
                    assert_eq!(preflight(pool, None).await?, (0, Some("COMBAT".into()), true));
                    assert_eq!(guard(pool).await?, Some((channel_text(CHANNEL_A), 2, false)));
                    Ok(())
                }
                .await;
                h.cleanup().await?;
                result
            };
            runtime()
                .map_err(|e| e.to_string())?
                .block_on(run)
                .map_err(|e| e.to_string())
        })
        .map_err(|e| e.to_string())?
        .join()
        .map_err(|_| "channel switch guard thread failed")??;
    Ok(())
}
