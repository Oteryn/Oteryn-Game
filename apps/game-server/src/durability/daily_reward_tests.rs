use super::*;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Connection, Executor, PgConnection, PgPool};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// Repository PostgreSQL convention: without `OTERYN_TEST_POSTGRES_ADMIN_URL` a
/// case reports PRE-ROUTING / NONCANONICAL and returns. A routed run executes
/// the body on its own natively migrated database, dropped afterwards. This
/// module is compiled into every target that mounts `durability`, so the
/// registered PostgreSQL CI targets execute these cases.
fn postgres_case(tag: &str, case: impl AsyncFnOnce(PgPool) -> TestResult) -> TestResult {
    let Ok(admin_url) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        eprintln!("PRE-ROUTING / NONCANONICAL: OTERYN_TEST_POSTGRES_ADMIN_URL is not configured");
        return Ok(());
    };
    if !admin_url.starts_with("postgresql://oteryn_test_admin:")
        || !admin_url.ends_with("@127.0.0.1:5432/postgres")
    {
        return Err("unsafe PostgreSQL test admin URL".into());
    }
    let prefix = admin_url
        .strip_suffix("/postgres")
        .ok_or("invalid admin URL")?
        .to_owned();
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let name = format!("daily_{tag}_{}_{suffix}", std::process::id());
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let mut admin = PgConnection::connect(&admin_url).await?;
            admin
                .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                    "CREATE DATABASE {name}"
                ))))
                .await?;
            let result: TestResult = async {
                let pool = PgPoolOptions::new()
                    .max_connections(8)
                    .connect(&format!("{prefix}/{name}"))
                    .await?;
                let result = async {
                    sqlx::migrate!("./migrations").run(&pool).await?;
                    case(pool.clone()).await
                }
                .await;
                pool.close().await;
                result
            }
            .await;
            admin
                .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                    "DROP DATABASE {name} WITH (FORCE)"
                ))))
                .await?;
            admin.close().await?;
            result
        })
}
const BINDING: [u8; 32] = [5; 32];

async fn fixture(pool: &PgPool) -> TestResult<(ClaimKey, AccountId)> {
    let mut connection = pool.acquire().await?;
    let version: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(&mut *connection)
        .await?;
    assert_eq!(version, "170006", "PostgreSQL 17.6 is required");
    sqlx::raw_sql(DAILY_REWARD_SCHEMA_CANDIDATE)
        .execute(&mut *connection)
        .await?;
    // Only native root seeding bypasses its live-authority trigger, locally.
    // All candidate DDL, constraints, privileges, reads and locks run normally.
    let mut tx = connection.begin().await?;
    tx.execute("SET LOCAL session_replication_role=replica")
        .await?;
    let account:String=sqlx::query_scalar("INSERT INTO game_character_account_guards(account_id) VALUES(game_character_uuid_v7()) RETURNING account_id::text").fetch_one(&mut *tx).await?;
    let row=sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name) VALUES(game_character_uuid_v7(),$1::uuid,game_character_uuid_v7(),1,1,'p1','r1','c1','s1','Daily Hero') RETURNING character_id::text AS character_id,world_id::text AS world_id")
        .bind(&account).fetch_one(&mut *tx).await?;
    let key = ClaimKey {
        character_id: CharacterId::from_bytes(uuid_text(
            &row.try_get::<String, _>("character_id")?,
        )?)?,
        world_id: WorldId::from_bytes(uuid_text(&row.try_get::<String, _>("world_id")?)?)?,
        reset_epoch: u64::MAX,
    };
    tx.commit().await?;
    Ok((key, AccountId::from_bytes(uuid_text(&account)?)?))
}

async fn insert_claim(connection: &mut PgConnection, key: ClaimKey) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO game_character_daily_reward_claims(character_id,world_id,reset_epoch,transaction_id,request_binding,lane_day,streak_before,streak_after,jokers_spent,premium) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,game_character_uuid_v7(),$4,7,99,100,3,false)")
        .bind(key.character_id.as_bytes().as_slice()).bind(key.world_id.as_bytes().as_slice()).bind(key.reset_epoch.to_string()).bind(BINDING.as_slice()).execute(connection).await?;
    Ok(())
}

async fn insert_lines(connection: &mut PgConnection, key: ClaimKey) -> Result<(), sqlx::Error> {
    // These are inert storage rows, not admitted rewards or a sink/grant stub.
    sqlx::query("INSERT INTO game_character_daily_reward_lines(character_id,world_id,reset_epoch,line,kind,quantity,status) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,0,'PREY_WILDCARD',3,'PENDING'),(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,1,'XP_BOOST',10,'PENDING')")
        .bind(key.character_id.as_bytes().as_slice()).bind(key.world_id.as_bytes().as_slice()).bind(key.reset_epoch.to_string()).execute(connection).await?;
    Ok(())
}

fn sqlstate(error: &sqlx::Error) -> Option<String> {
    error
        .as_database_error()
        .and_then(|e| e.code())
        .map(|s| s.into_owned())
}

#[test]
fn daily_storage_replay_binding_reload_and_atomic_rollback() -> TestResult {
    postgres_case(
        "c1",
        daily_storage_replay_binding_reload_and_atomic_rollback_case,
    )
}

async fn daily_storage_replay_binding_reload_and_atomic_rollback_case(pool: PgPool) -> TestResult {
    let (key, account) = fixture(&pool).await?;
    let mut connection = pool.acquire().await?;
    let mut tx = connection.begin().await?;
    assert_eq!(
        read_reward_state(&mut tx, key.character_id, key.world_id).await?,
        None
    );
    assert_eq!(
        lock_joker_state(&mut tx, account, key.world_id).await?,
        None
    );
    assert_eq!(
        read_claim(&mut tx, key, &BINDING).await?,
        ClaimLookup::Missing
    );
    sqlx::query("INSERT INTO game_character_daily_rewards VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,100,$3::text::numeric)")
        .bind(key.character_id.as_bytes().as_slice()).bind(key.world_id.as_bytes().as_slice()).bind(key.reset_epoch.to_string()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_account_daily_reward_jokers VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,0,'2026-10-01',2)")
        .bind(account.as_bytes().as_slice()).bind(key.world_id.as_bytes().as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_account_daily_reward_joker_entries VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,game_character_uuid_v7(),'GRANT',3,3),(encode($1,'hex')::uuid,encode($2,'hex')::uuid,2,game_character_uuid_v7(),'SPEND',3,0)")
        .bind(account.as_bytes().as_slice()).bind(key.world_id.as_bytes().as_slice()).execute(&mut *tx).await?;
    insert_claim(&mut tx, key).await?;
    assert!(matches!(
        read_claim(&mut tx, key, &BINDING).await,
        Err(DurabilityError::InvalidStoredState)
    ));
    insert_lines(&mut tx, key).await?;
    let original = read_claim(&mut tx, key, &BINDING).await?;
    assert!(
        matches!(&original,ClaimLookup::Replay(receipt) if receipt.lines.len()==2 && !receipt.premium && receipt.jokers_spent==3)
    );
    tx.rollback().await?;
    let mut tx = connection.begin().await?;
    assert_eq!(
        read_claim(&mut tx, key, &BINDING).await?,
        ClaimLookup::Missing
    );
    assert_eq!(
        read_reward_state(&mut tx, key.character_id, key.world_id).await?,
        None
    );
    assert_eq!(
        lock_joker_state(&mut tx, account, key.world_id).await?,
        None
    );
    let entries: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_account_daily_reward_joker_entries")
            .fetch_one(&mut *tx)
            .await?;
    assert_eq!(entries, 0);
    insert_claim(&mut tx, key).await?;
    insert_lines(&mut tx, key).await?;
    let committed = read_claim(&mut tx, key, &BINDING).await?;
    tx.commit().await?;
    drop(connection);
    // A separate physical connection reload has no process/session state.
    let mut fresh = PgConnection::connect_with(&pool.connect_options()).await?;
    let mut tx = fresh.begin().await?;
    assert_eq!(read_claim(&mut tx, key, &BINDING).await?, committed);
    assert_eq!(
        read_claim(&mut tx, key, &[6; 32]).await?,
        ClaimLookup::BindingMismatch
    );
    let other_epoch = ClaimKey {
        reset_epoch: 0,
        ..key
    };
    assert_eq!(
        read_claim(&mut tx, other_epoch, &BINDING).await?,
        ClaimLookup::Missing
    );
    let other_world = WorldId::from_bytes(*account.as_bytes())?;
    assert_eq!(
        read_claim(
            &mut tx,
            ClaimKey {
                world_id: other_world,
                ..key
            },
            &BINDING
        )
        .await?,
        ClaimLookup::Missing
    );
    tx.rollback().await?;
    Ok(())
}

#[test]
fn daily_epoch_uniqueness_waits_then_rejects_without_second_receipt() -> TestResult {
    postgres_case(
        "c2",
        daily_epoch_uniqueness_waits_then_rejects_without_second_receipt_case,
    )
}

async fn daily_epoch_uniqueness_waits_then_rejects_without_second_receipt_case(
    pool: PgPool,
) -> TestResult {
    let (key, _) = fixture(&pool).await?;
    let mut first = pool.begin().await?;
    insert_claim(&mut first, key).await?;
    let mut second = pool.begin().await?;
    second.execute("SET LOCAL lock_timeout='50ms'").await?;
    let blocked = insert_claim(&mut second, key).await;
    assert!(matches!(blocked,Err(ref e) if sqlstate(e).as_deref()==Some("55P03")));
    second.rollback().await?;
    first.commit().await?;
    let mut second = pool.begin().await?;
    let duplicate = insert_claim(&mut second, key).await;
    assert!(matches!(duplicate,Err(ref e) if sqlstate(e).as_deref()==Some("23505")));
    second.rollback().await?;
    let mut tx = pool.begin().await?;
    let other_epoch = ClaimKey {
        reset_epoch: 0,
        ..key
    };
    insert_claim(&mut tx, other_epoch).await?;
    let other_world = ClaimKey {
        world_id: WorldId::from_bytes(*key.character_id.as_bytes())?,
        ..key
    };
    insert_claim(&mut tx, other_world).await?;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_daily_reward_claims")
        .fetch_one(&mut *tx)
        .await?;
    assert_eq!(count, 3);
    tx.rollback().await?;
    Ok(())
}

#[test]
fn daily_pending_line_runtime_lock_rollback_and_zero_credit_terminal() -> TestResult {
    postgres_case(
        "c3",
        daily_pending_line_runtime_lock_rollback_and_zero_credit_terminal_case,
    )
}

async fn daily_pending_line_runtime_lock_rollback_and_zero_credit_terminal_case(
    pool: PgPool,
) -> TestResult {
    let (key, _) = fixture(&pool).await?;
    let mut seed = pool.acquire().await?;
    insert_claim(&mut seed, key).await?;
    insert_lines(&mut seed, key).await?;
    let mut first = pool.begin().await?;
    first.execute("SET LOCAL ROLE oteryn_game_runtime").await?;
    let lines = lock_pending_lines(&mut first, key).await?;
    assert_eq!(lines.iter().map(|l| l.line).collect::<Vec<_>>(), vec![0, 1]);
    let mut second = pool.begin().await?;
    second.execute("SET LOCAL ROLE oteryn_game_runtime").await?;
    second.execute("SET LOCAL lock_timeout='50ms'").await?;
    assert!(matches!(
        lock_pending_lines(&mut second, key).await,
        Err(DurabilityError::Database(_))
    ));
    second.rollback().await?;
    sqlx::query("UPDATE game_character_daily_reward_lines SET status='DELIVERED',delivering_transaction_id=game_character_uuid_v7(),uncredited_excess=quantity WHERE line=0").execute(&mut *first).await?;
    first.rollback().await?;
    let mut second = pool.begin().await?;
    second.execute("SET LOCAL ROLE oteryn_game_runtime").await?;
    assert_eq!(lock_pending_lines(&mut second, key).await?.len(), 2);
    sqlx::query("UPDATE game_character_daily_reward_lines SET status='DELIVERED',delivering_transaction_id=game_character_uuid_v7(),uncredited_excess=quantity WHERE line=0").execute(&mut *second).await?;
    second.commit().await?;
    let mut fresh = pool.begin().await?;
    fresh.execute("SET LOCAL ROLE oteryn_game_runtime").await?;
    let remaining = lock_pending_lines(&mut fresh, key).await?;
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].part, RewardPart::XpBoost);
    match read_claim(&mut fresh, key, &BINDING).await? {
        ClaimLookup::Replay(receipt) => assert!(matches!(
            receipt.lines[0].delivery,
            DeliveryState::Delivered {
                uncredited_excess: 3,
                ..
            }
        )),
        other => return Err(format!("expected stored receipt, got {other:?}").into()),
    }
    let downgrade=sqlx::query("UPDATE game_character_daily_reward_lines SET status='PENDING',delivering_transaction_id=NULL,uncredited_excess=0 WHERE line=0").execute(&mut *fresh).await;
    assert!(matches!(downgrade,Err(ref e) if sqlstate(e).as_deref()==Some("23514")));
    fresh.rollback().await?;
    let mut replay = pool.begin().await?;
    let rewrite=sqlx::query("UPDATE game_character_daily_reward_lines SET delivering_transaction_id=game_character_uuid_v7() WHERE line=0").execute(&mut *replay).await;
    assert!(matches!(rewrite,Err(ref e) if sqlstate(e).as_deref()==Some("23514")));
    replay.rollback().await?;
    Ok(())
}

#[test]
fn daily_schema_bounds_and_runtime_immutable_columns() -> TestResult {
    postgres_case("c4", daily_schema_bounds_and_runtime_immutable_columns_case)
}

async fn daily_schema_bounds_and_runtime_immutable_columns_case(pool: PgPool) -> TestResult {
    let (key, account) = fixture(&pool).await?;
    let mut connection = pool.acquire().await?;
    insert_claim(&mut connection, key).await?;
    insert_lines(&mut connection, key).await?;
    for statement in [
        "UPDATE game_character_daily_reward_lines SET quantity=0 WHERE line=0",
        "UPDATE game_character_daily_reward_lines SET uncredited_excess=1 WHERE line=0",
        "UPDATE game_character_daily_reward_lines SET status='DELIVERED' WHERE line=0",
        "UPDATE game_character_daily_reward_lines SET status='DELIVERED',delivering_transaction_id=game_character_uuid_v7(),uncredited_excess=4 WHERE line=0",
        "UPDATE game_character_daily_reward_lines SET status='DELIVERED',delivering_transaction_id=game_character_uuid_v7(),uncredited_excess=1 WHERE line=1",
        "UPDATE game_character_daily_reward_claims SET request_binding='x'::bytea",
        "UPDATE game_character_daily_reward_claims SET reset_epoch=18446744073709551616",
    ] {
        let mut tx = connection.begin().await?;
        let result = sqlx::query(statement).execute(&mut *tx).await;
        assert!(
            matches!(result,Err(ref e) if sqlstate(e).as_deref()==Some("23514")),
            "{statement}"
        );
        tx.rollback().await?;
    }
    for statement in [
        "UPDATE game_character_daily_reward_claims SET premium=true",
        "DELETE FROM game_character_daily_reward_claims",
        "UPDATE game_character_daily_reward_lines SET quantity=2",
        "DELETE FROM game_character_daily_reward_lines",
        "UPDATE game_account_daily_reward_joker_entries SET amount=1",
        "DELETE FROM game_account_daily_reward_joker_entries",
    ] {
        let mut tx = connection.begin().await?;
        tx.execute("SET LOCAL ROLE oteryn_game_runtime").await?;
        let result = sqlx::query(statement).execute(&mut *tx).await;
        assert!(
            matches!(result,Err(ref e) if sqlstate(e).as_deref()==Some("42501")),
            "{statement}"
        );
        tx.rollback().await?;
    }
    // Storage accepts both declared maxima, including the full u64 epoch;
    // max+1 is refused, never wrapped into another claim/month.
    for (streak, lane) in [(100000, 7), (100001, 7), (100000, 8)] {
        let mut tx = connection.begin().await?;
        let result=sqlx::query("INSERT INTO game_character_daily_rewards VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,18446744073709551615)")
            .bind(key.character_id.as_bytes().as_slice()).bind(key.world_id.as_bytes().as_slice()).bind(lane as i16).bind(streak).execute(&mut *tx).await;
        assert_eq!(result.is_ok(), streak == 100000 && lane == 7);
        if result.is_ok() {
            assert_eq!(
                read_reward_state(&mut tx, key.character_id, key.world_id)
                    .await?
                    .map(|s| s.streak),
                Some(100000)
            );
        }
        tx.rollback().await?;
    }
    for jokers in [3_i16, 4] {
        let mut tx = connection.begin().await?;
        let result=sqlx::query("INSERT INTO game_account_daily_reward_jokers VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,'2026-10-01',18446744073709551615)")
            .bind(account.as_bytes().as_slice()).bind(key.world_id.as_bytes().as_slice()).bind(jokers).execute(&mut *tx).await;
        assert_eq!(result.is_ok(), jokers == 3);
        if result.is_ok() {
            assert_eq!(
                lock_joker_state(&mut tx, account, key.world_id)
                    .await?
                    .map(|s| (s.jokers, s.last_entry_id)),
                Some((3, u64::MAX))
            );
        }
        tx.rollback().await?;
    }
    Ok(())
}

async fn seed_joker_chain(
    pool: &PgPool,
    key: ClaimKey,
    account: AccountId,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("INSERT INTO game_account_daily_reward_jokers VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,3,'2026-10-01',1)")
        .bind(account.as_bytes().as_slice()).bind(key.world_id.as_bytes().as_slice()).execute(&mut *tx).await?;
    append_joker_entry(&mut tx, key, account, (1, "GRANT", 3, 3)).await?;
    tx.commit().await
}

async fn update_joker_head(
    connection: &mut PgConnection,
    key: ClaimKey,
    account: AccountId,
    balance: i16,
    entry: u64,
) -> Result<(), sqlx::Error> {
    let changed = sqlx::query("UPDATE game_account_daily_reward_jokers SET jokers=$3,last_entry_id=$4::text::numeric WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid")
        .bind(account.as_bytes().as_slice()).bind(key.world_id.as_bytes().as_slice())
        .bind(balance).bind(entry.to_string()).execute(connection).await?;
    assert_eq!(changed.rows_affected(), 1);
    Ok(())
}

async fn append_joker_entry(
    connection: &mut PgConnection,
    key: ClaimKey,
    account: AccountId,
    (entry, kind, amount, after): (u64, &str, i16, i16),
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO game_account_daily_reward_joker_entries VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,game_character_uuid_v7(),$4,$5,$6)")
        .bind(account.as_bytes().as_slice()).bind(key.world_id.as_bytes().as_slice())
        .bind(entry.to_string()).bind(kind).bind(amount).bind(after).execute(connection).await?;
    Ok(())
}

#[test]
fn daily_joker_ledger_final_state_multi_entry_and_rollback() -> TestResult {
    postgres_case(
        "c5",
        daily_joker_ledger_final_state_multi_entry_and_rollback_case,
    )
}

async fn daily_joker_ledger_final_state_multi_entry_and_rollback_case(pool: PgPool) -> TestResult {
    let (key, account) = fixture(&pool).await?;
    // Both orderings must see the final transaction, not an earlier trigger's NEW row.
    for (state_first, world) in [
        (true, key.world_id),
        (false, WorldId::from_bytes(*account.as_bytes())?),
    ] {
        let key = ClaimKey {
            world_id: world,
            ..key
        };
        let mut tx = pool.begin().await?;
        tx.execute("SET LOCAL ROLE oteryn_game_runtime").await?;
        sqlx::query("INSERT INTO game_account_daily_reward_jokers VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,0,NULL,0)")
            .bind(account.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let mut tx = pool.begin().await?;
        tx.execute("SET LOCAL ROLE oteryn_game_runtime").await?;
        if state_first {
            update_joker_head(&mut tx, key, account, 3, 1).await?;
            update_joker_head(&mut tx, key, account, 3, 3).await?;
        }
        // Physical INSERT order differs from ledger order. All rows must validate at COMMIT.
        sqlx::query("INSERT INTO game_account_daily_reward_joker_entries VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,3,game_character_uuid_v7(),'GRANT',1,3),(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,game_character_uuid_v7(),'GRANT',3,3),(encode($1,'hex')::uuid,encode($2,'hex')::uuid,2,game_character_uuid_v7(),'SPEND',1,2)")
            .bind(account.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).execute(&mut *tx).await?;
        if !state_first {
            update_joker_head(&mut tx, key, account, 3, 3).await?;
        }
        tx.commit().await?;
        let mut rollback = pool.begin().await?;
        rollback
            .execute("SET LOCAL ROLE oteryn_game_runtime")
            .await?;
        update_joker_head(&mut rollback, key, account, 2, 4).await?;
        append_joker_entry(&mut rollback, key, account, (4, "SPEND", 1, 2)).await?;
        rollback.rollback().await?;
        let mut fresh = pool.begin().await?;
        let stored = lock_joker_state(&mut fresh, account, world)
            .await?
            .ok_or("missing jokers")?;
        assert_eq!((stored.jokers, stored.last_entry_id), (3, 3));
        let entries: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_daily_reward_joker_entries WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid")
            .bind(account.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).fetch_one(&mut *fresh).await?;
        assert_eq!(entries, 3);
        fresh.rollback().await?;
    }
    Ok(())
}

#[test]
fn daily_joker_ledger_rejects_broken_head_chain_and_scope() -> TestResult {
    postgres_case(
        "c6",
        daily_joker_ledger_rejects_broken_head_chain_and_scope_case,
    )
}

async fn daily_joker_ledger_rejects_broken_head_chain_and_scope_case(pool: PgPool) -> TestResult {
    let (key, account) = fixture(&pool).await?;
    seed_joker_chain(&pool, key, account).await?;
    for (label, balance, cursor, entry, amount) in [
        ("balance differs from latest entry", 1, 2, 2, 1),
        ("cursor is not the latest entry", 2, 1, 2, 1),
        ("entry arithmetic is false", 2, 2, 2, 2),
        ("entry sequence has a gap", 2, 3, 3, 1),
        ("balance changes without an entry", 2, 2, 0, 1),
    ] {
        let mut tx = pool.begin().await?;
        tx.execute("SET LOCAL ROLE oteryn_game_runtime").await?;
        update_joker_head(&mut tx, key, account, balance, cursor).await?;
        if entry != 0 {
            append_joker_entry(&mut tx, key, account, (entry, "SPEND", amount, 2)).await?;
        }
        let result = tx.commit().await;
        assert!(
            matches!(result, Err(ref error) if sqlstate(error).as_deref() == Some("23514")),
            "{label}: {result:?}"
        );
        let mut fresh = pool.begin().await?;
        let stored = lock_joker_state(&mut fresh, account, key.world_id)
            .await?
            .ok_or("missing jokers")?;
        assert_eq!((stored.jokers, stored.last_entry_id), (3, 1));
        fresh.rollback().await?;
    }

    // A same-named temporary state must not substitute for the installed ledger's head.
    let mut shadowed = pool.begin().await?;
    update_joker_head(&mut shadowed, key, account, 1, 2).await?;
    append_joker_entry(&mut shadowed, key, account, (2, "SPEND", 1, 2)).await?;
    shadowed.execute("CREATE TEMP TABLE game_account_daily_reward_jokers (LIKE game_account_daily_reward_jokers INCLUDING ALL) ON COMMIT DROP").await?;
    sqlx::query("INSERT INTO game_account_daily_reward_jokers VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,2,'2026-10-01',2)")
        .bind(account.as_bytes().as_slice()).bind(key.world_id.as_bytes().as_slice()).execute(&mut *shadowed).await?;
    let result = shadowed.commit().await;
    assert!(matches!(result, Err(ref error) if sqlstate(error).as_deref() == Some("23514")));

    // An entry in either another World or another Account cannot supply the predecessor.
    for (other_account, world) in [
        (account, WorldId::from_bytes(*account.as_bytes())?),
        (
            AccountId::from_bytes(*key.character_id.as_bytes())?,
            key.world_id,
        ),
    ] {
        let other_key = ClaimKey {
            world_id: world,
            ..key
        };
        let mut tx = pool.begin().await?;
        sqlx::query("INSERT INTO game_account_daily_reward_jokers VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,0,NULL,0)")
            .bind(other_account.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let mut tx = pool.begin().await?;
        tx.execute("SET LOCAL ROLE oteryn_game_runtime").await?;
        update_joker_head(&mut tx, other_key, other_account, 2, 2).await?;
        append_joker_entry(&mut tx, other_key, other_account, (2, "SPEND", 1, 2)).await?;
        let result = tx.commit().await;
        assert!(matches!(result, Err(ref error) if sqlstate(error).as_deref() == Some("23514")));
        let mut fresh = pool.begin().await?;
        let stored = lock_joker_state(&mut fresh, other_account, world)
            .await?
            .ok_or("missing scope")?;
        assert_eq!((stored.jokers, stored.last_entry_id), (0, 0));
        fresh.rollback().await?;
    }
    Ok(())
}

#[test]
fn daily_joker_ledger_entries_cannot_be_rewritten_or_erased() -> TestResult {
    postgres_case(
        "c7",
        daily_joker_ledger_entries_cannot_be_rewritten_or_erased_case,
    )
}

async fn daily_joker_ledger_entries_cannot_be_rewritten_or_erased_case(pool: PgPool) -> TestResult {
    let (key, account) = fixture(&pool).await?;
    seed_joker_chain(&pool, key, account).await?;
    // The table owner can issue these statements: immutable history must also be a DB invariant.
    for statement in [
        "UPDATE game_account_daily_reward_joker_entries SET amount=1",
        "UPDATE game_account_daily_reward_joker_entries SET transaction_id=game_character_uuid_v7()",
        "DELETE FROM game_account_daily_reward_joker_entries",
        "TRUNCATE game_account_daily_reward_joker_entries",
    ] {
        let mut tx = pool.begin().await?;
        let result = sqlx::query(statement).execute(&mut *tx).await;
        assert!(
            matches!(result, Err(ref error) if sqlstate(error).as_deref() == Some("23514")),
            "{statement}: {result:?}"
        );
        tx.rollback().await?;
    }
    let mut fresh = pool.begin().await?;
    let stored = lock_joker_state(&mut fresh, account, key.world_id)
        .await?
        .ok_or("missing jokers")?;
    assert_eq!((stored.jokers, stored.last_entry_id), (3, 1));
    fresh.rollback().await?;
    Ok(())
}

#[test]
fn daily_joker_ledger_serializes_writers_and_rejects_stale_debit() -> TestResult {
    postgres_case(
        "c8",
        daily_joker_ledger_serializes_writers_and_rejects_stale_debit_case,
    )
}

async fn daily_joker_ledger_serializes_writers_and_rejects_stale_debit_case(
    pool: PgPool,
) -> TestResult {
    let (key, account) = fixture(&pool).await?;
    seed_joker_chain(&pool, key, account).await?;
    let mut first = pool.begin().await?;
    first.execute("SET LOCAL ROLE oteryn_game_runtime").await?;
    let before = lock_joker_state(&mut first, account, key.world_id)
        .await?
        .ok_or("missing jokers")?;
    assert_eq!(before.jokers, 3);
    let mut second = pool.begin().await?;
    second.execute("SET LOCAL ROLE oteryn_game_runtime").await?;
    second.execute("SET LOCAL lock_timeout='50ms'").await?;
    // Observe the real helper's row lock before DurabilityError intentionally
    // discards unregistered PostgreSQL states such as 55P03.
    let blocked = sqlx::query(
        "SELECT 1 FROM game_account_daily_reward_jokers \
         WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid FOR UPDATE",
    )
    .bind(account.as_bytes().as_slice())
    .bind(key.world_id.as_bytes().as_slice())
    .fetch_optional(&mut *second)
    .await;
    assert!(matches!(blocked, Err(ref error) if sqlstate(error).as_deref() == Some("55P03")));
    second.rollback().await?;
    update_joker_head(&mut first, key, account, 2, 2).await?;
    append_joker_entry(&mut first, key, account, (2, "SPEND", 1, 2)).await?;
    first.commit().await?;
    let mut second = pool.begin().await?;
    second.execute("SET LOCAL ROLE oteryn_game_runtime").await?;
    let current = lock_joker_state(&mut second, account, key.world_id)
        .await?
        .ok_or("missing jokers")?;
    assert_eq!((current.jokers, current.last_entry_id), (2, 2));
    // A stale debit planned from three jokers cannot consume the two left after the winner.
    update_joker_head(&mut second, key, account, 0, 3).await?;
    append_joker_entry(&mut second, key, account, (3, "SPEND", 3, 0)).await?;
    let result = second.commit().await;
    assert!(matches!(result, Err(ref error) if sqlstate(error).as_deref() == Some("23514")));
    let mut fresh = pool.begin().await?;
    let stored = lock_joker_state(&mut fresh, account, key.world_id)
        .await?
        .ok_or("missing jokers")?;
    assert_eq!((stored.jokers, stored.last_entry_id), (2, 2));
    fresh.rollback().await?;
    Ok(())
}
