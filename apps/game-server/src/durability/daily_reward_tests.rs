use super::*;
use sqlx::{Connection, Executor, PgConnection, PgPool};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
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

#[sqlx::test(migrations = "./migrations")]
async fn daily_storage_replay_binding_reload_and_atomic_rollback(pool: PgPool) -> TestResult {
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

#[sqlx::test(migrations = "./migrations")]
async fn daily_epoch_uniqueness_waits_then_rejects_without_second_receipt(
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

#[sqlx::test(migrations = "./migrations")]
async fn daily_pending_line_runtime_lock_rollback_and_zero_credit_terminal(
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

#[sqlx::test(migrations = "./migrations")]
async fn daily_schema_bounds_and_runtime_immutable_columns(pool: PgPool) -> TestResult {
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
