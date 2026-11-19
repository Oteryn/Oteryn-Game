//! chsw / GAME-CHANNEL-01 §8-§11: the durable anti-hopping guard advances inside the fresh
//! admission receipt insert, against the production SQLx migration. The first admission sets the
//! baseline, the same Channel is not a switch, a different Channel is a switch even after a relog,
//! a refused or failed destination leaves the guard unchanged and a retry yields one outcome.
//! The online count of command 6251 is read from the same production schema.
use crate::bestiary_postgres_harness::{
    CHANNEL, CHARACTER, Harness, TestResult, WORLD, configured_admin, id, runtime,
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
         $6::uuid, $7, 1, 1, decode(repeat('ab', 16), 'hex'), 1, '{}', 1)",
    )
    .bind(format!("{seq:02x}"))
    .bind(hex(session))
    .bind(hex(id(7)))
    .bind(hex(id(CHARACTER)))
    .bind(hex(id(WORLD)))
    .bind(hex(id(channel)))
    .bind(i64::from(seq))
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

/// Command 6251's online count per current Channel, read as `oteryn_game_runtime`. With
/// `terminal` the fixture session is ended first, inside the same rolled-back transaction.
async fn load(pool: &sqlx::PgPool, terminal: bool) -> TestResult<Vec<(Vec<u8>, i64)>> {
    let mut tx = pool.begin().await?;
    if terminal {
        sqlx::query("SET LOCAL session_replication_role = replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=3")
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    let rows = sqlx::query_as(oteryn_gameplay_durability::durability::channel_switch::LOAD_SQL)
        .bind(id(WORLD).as_slice())
        .bind(64_i64)
        .fetch_all(&mut *tx)
        .await?;
    tx.rollback().await?;
    Ok(rows)
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

/// Explain the production capacity query with its real parameters on the fixture Channel.
async fn assert_capacity_uses_bounded_index_scan(pool: &sqlx::PgPool) -> TestResult {
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL enable_seqscan = off").execute(&mut *tx).await?;
    let body: String = sqlx::query_scalar(
        "SELECT prosrc FROM pg_proc WHERE oid='game_channel_full(uuid,uuid,uuid)'::regprocedure",
    ).fetch_one(&mut *tx).await?;
    let plan: Vec<String> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "EXPLAIN (COSTS OFF) SELECT ({body}) FROM (SELECT $1::uuid AS p_character, $2::uuid AS p_world, $3::uuid AS p_channel) args"
    )))
    .bind(hex(id(CHARACTER)))
    .bind(hex(id(WORLD)))
    .bind(hex(id(CHANNEL)))
    .fetch_all(&mut *tx)
    .await?;
    assert!(plan.iter().any(|line| line.contains("game_channel_live_sessions")), "{plan:?}");
    assert!(plan.iter().any(|line| line.contains("Limit")), "{plan:?}");
    tx.rollback().await?;
    Ok(())
}

fn channel_text(seed: u8) -> String {
    let h = hex(id(seed));
    format!(
        "{}-{}-{}-{}-{}",
        &h[0..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..32]
    )
}

fn code(result: Result<(), sqlx::Error>) -> Option<String> {
    result.err().and_then(|e| {
        e.as_database_error()
            .and_then(|e| e.code())
            .map(|c| c.into_owned())
    })
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
                    // Command 6251: the assigned Channel with its one current session; an ended
                    // session no longer counts, and the Channel stays listed at zero.
                    assert_eq!(load(pool, false).await?, vec![(id(CHANNEL).to_vec(), 1)]);
                    assert_eq!(load(pool, true).await?, vec![(id(CHANNEL).to_vec(), 0)]);
                    // One occupied player slot. The receipt guard runs even before a baseline exists.
                    sqlx::query("INSERT INTO game_channel_pvp_types(world_id,channel_id,pvp_type,player_capacity) VALUES($1::uuid,$2::uuid,1,1) ON CONFLICT(world_id,channel_id) DO UPDATE SET player_capacity=1")
                        .bind(hex(id(WORLD))).bind(hex(id(CHANNEL))).execute(pool).await?;
                    assert_eq!(preflight(pool,Some(CHANNEL)).await?.1.as_deref(), Some("CHANNEL_FULL"));
                    assert_capacity_uses_bounded_index_scan(pool).await?;
                    assert_eq!(code(admit(pool, 18, CHANNEL).await).as_deref(),Some("OTH02"));
                    assert_eq!(guard(pool).await?, None);
                    for account_type in [4_i16, 6] {
                        let mut staff = pool.begin().await?;
                        sqlx::query("SELECT game_chat_role_issue($1::uuid,0,4::smallint,$2,true,game_character_uuid_v7(),'capacity-regression')")
                            .bind(hex(id(CHARACTER))).bind(account_type).execute(&mut *staff).await?;
                        admit(&mut *staff,18,CHANNEL).await?;
                        staff.rollback().await?;
                    }
                    sqlx::query("UPDATE game_channel_pvp_types SET player_capacity=100 WHERE world_id=$1::uuid AND channel_id=$2::uuid")
                        .bind(hex(id(WORLD))).bind(hex(id(CHANNEL))).execute(pool).await?;
                    // The first admission sets the baseline and is not a switch.
                    admit(pool, 1, CHANNEL_A).await?;
                    assert_eq!(guard(pool).await?, Some((channel_text(CHANNEL_A), 0, false)));
                    // The same Channel again (a relog or reconnect) is not a switch.
                    admit(pool, 2, CHANNEL_A).await?;
                    assert_eq!(guard(pool).await?, Some((channel_text(CHANNEL_A), 0, false)));
                    // Upgrade from a database with receipts and no switch guard.
                    let mut upgrade = pool.begin().await?;
                    sqlx::raw_sql("DROP TRIGGER game_channel_switch_admit ON game_durability_fresh_admission_receipts;
                        DROP FUNCTION game_channel_switch_admit(), game_channel_switch_status(uuid,uuid), game_channel_switch_lock(uuid,uuid,uuid), game_channel_switch_cooldown_ms(), game_channel_switch_now_ms(), game_channel_full(uuid,uuid,uuid);
                        DROP FUNCTION game_channel_publish_policy(uuid,uuid,bytea,smallint,integer);
                        DROP INDEX game_channel_live_sessions;
                        DROP TABLE game_channel_switch_guards, game_channel_pvp_types;")
                        .execute(&mut *upgrade).await?;
                    admit(&mut *upgrade, 12, CHANNEL_B).await?;
                    sqlx::raw_sql(include_str!("../../../migrations/2858_channel_switch_guard.sql"))
                        .execute(&mut *upgrade).await?;
                    admit(&mut *upgrade, 13, CHANNEL_B).await?;
                    let switches: i64 = sqlx::query_scalar("SELECT switch_count FROM game_channel_switch_guards WHERE character_id=$1::uuid")
                        .bind(hex(id(CHARACTER))).fetch_one(&mut *upgrade).await?;
                    assert_eq!(switches, 0, "the latest receipt already entered B");
                    admit(&mut *upgrade, 14, CHANNEL_A).await?;
                    let switches: i64 = sqlx::query_scalar("SELECT switch_count FROM game_channel_switch_guards WHERE character_id=$1::uuid")
                        .bind(hex(id(CHARACTER))).fetch_one(&mut *upgrade).await?;
                    assert_eq!(switches, 1, "first post-upgrade switch");
                    upgrade.rollback().await?;
                    // A saved House can only be restored on its origin Channel.
                    let mut house = pool.begin().await?;
                    sqlx::query("INSERT INTO game_character_last_positions(character_id,world_id,tile_x,tile_y,tile_floor,coordinate_frame_digest,map_revision_digest,server_artifact_digest,character_lease_generation,write_sequence,house_key,origin_channel_id,origin_tile_x,origin_tile_y,origin_tile_floor) VALUES($1::uuid,$2::uuid,10,10,7,decode(repeat('00',32),'hex'),decode(repeat('00',32),'hex'),decode(repeat('00',32),'hex'),1,1,'oteryn:content.house.home',$3::uuid,10,10,7)")
                        .bind(hex(id(CHARACTER))).bind(hex(id(WORLD))).bind(hex(id(CHANNEL_A)))
                        .execute(&mut *house).await?;
                    assert_eq!(code(admit(&mut *house, 14, CHANNEL_B).await).as_deref(), Some("OTH02"));
                    house.rollback().await?;
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
                    for deadline in ["kill_block_until", "pz_block_until"] {
                        let mut tx = pool.begin().await?;
                        sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await?;
                        sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE game_character_pvp_state SET logout_block_until=now()-interval '1 second', {deadline}=now()+interval '15 minutes' WHERE character_id=$1::uuid")))
                            .bind(hex(id(CHARACTER))).execute(&mut *tx).await?;
                        sqlx::query("SET LOCAL session_replication_role = origin").execute(&mut *tx).await?;
                        assert_eq!(code(admit(&mut *tx, 12, CHANNEL_B).await).as_deref(), Some("OTH02"), "{deadline}");
                        tx.rollback().await?;
                    }
                    // Migration 2859: a held Party membership survives a refused admission and
                    // is taken back by the committed admission into the target Channel.
                    let mut party = pool.begin().await?;
                    sqlx::query("INSERT INTO game_parties(party_id,world_id,leader_character_id,revision,next_seq) VALUES($1::uuid,$2::uuid,$3::uuid,1,2)")
                        .bind(hex(id(CHANNEL_B + 10))).bind(hex(id(WORLD))).bind(hex(id(CHARACTER)))
                        .execute(&mut *party).await?;
                    sqlx::query("INSERT INTO game_party_members(character_id,party_id,seq,presence_state) VALUES($1::uuid,$2::uuid,1,3)")
                        .bind(hex(id(CHARACTER))).bind(hex(id(CHANNEL_B + 10)))
                        .execute(&mut *party).await?;
                    sqlx::query("INSERT INTO game_channel_switch_party_holds VALUES($1::uuid,game_channel_switch_now_ms()+game_channel_switch_party_hold_ms())")
                        .bind(hex(id(CHARACTER)))
                        .execute(&mut *party).await?;
                    party.commit().await?;
                    let held = || async {
                        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_channel_switch_party_holds")
                            .fetch_one(pool)
                            .await
                    };
                    assert_eq!(code(admit(pool, 10, CHANNEL_B).await).as_deref(), Some("OTH02"));
                    assert_eq!(held().await?, 1, "a refused admission keeps the hold");
                    let mut late = pool.begin().await?;
                    sqlx::query("UPDATE game_channel_switch_party_holds SET held_until_ms=1 WHERE character_id=$1::uuid")
                        .bind(hex(id(CHARACTER))).execute(&mut *late).await?;
                    admit(&mut *late, 15, CHANNEL_A).await?;
                    let member: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_party_members WHERE character_id=$1::uuid)")
                        .bind(hex(id(CHARACTER))).fetch_one(&mut *late).await?;
                    assert!(!member, "late admission expires membership before returning");
                    late.rollback().await?;
                    admit(pool, 11, CHANNEL_A).await?;
                    assert_eq!(held().await?, 0, "the admission takes the hold back");
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
