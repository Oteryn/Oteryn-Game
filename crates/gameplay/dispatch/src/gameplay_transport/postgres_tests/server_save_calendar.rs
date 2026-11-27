//! Exercise the actual Task Board period and Daily Reward publication on the migrated schema.
use super::fixture::{Harness, id};
use super::TestResult;

#[test]
fn server_save_daily_and_weekly_boundaries_follow_berlin_dst_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run("server-save-calendar", move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async move {
                let harness = Harness::create_with_admission_seed(
                    admin, "server_save_calendar", false, false,
                )
                .await?;
                boundaries(&harness.pool).await
            })
    })
}

async fn boundaries(pool: &sqlx::PgPool) -> TestResult {
    for (date, utc_hour, old_monday, monday) in [
        ("2026-01-05", "09", "2025-12-29", "2026-01-05"),
        ("2026-07-06", "08", "2026-06-29", "2026-07-06"),
        ("2026-03-29", "08", "2026-03-23", "2026-03-23"),
        ("2026-03-30", "08", "2026-03-23", "2026-03-30"),
        ("2026-10-25", "09", "2026-10-19", "2026-10-19"),
        ("2026-10-26", "09", "2026-10-19", "2026-10-26"),
        ("2026-11-23", "09", "2026-11-16", "2026-11-23"),
    ] {
        let mut tx = pool.begin().await?;
        sqlx::query("SET LOCAL TIME ZONE 'Pacific/Honolulu'")
            .execute(&mut *tx)
            .await?;
        let world = id(77);
        let (before, at): (String, String) = sqlx::query_as(
            "SELECT (t-interval '1 minute')::text,t::text FROM (SELECT $1::text::timestamptz AS t) v",
        )
        .bind(format!("{date} {utc_hour}:00:00+00"))
        .fetch_one(&mut *tx)
        .await?;
        insert_epoch(&mut tx, world, 0, &before, false).await?;
        sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,0)")
            .bind(world.as_slice())
            .execute(&mut *tx)
            .await?;
        assert_periods(&mut tx, world, date, old_monday, true).await?;

        insert_epoch(&mut tx, world, 1, &at, false).await?;
        sqlx::query("UPDATE game_world_reset_active SET reset_epoch=1 WHERE world_id=encode($1,'hex')::uuid")
            .bind(world.as_slice()).execute(&mut *tx).await?;
        assert_periods(&mut tx, world, date, monday, false).await?;

        let (exact, before_ms, at_ms, before_weekday, at_weekday): (bool, i64, i64, i16, i16) =
            sqlx::query_as(
                "SELECT game_server_save_at($1::text::date)=at_time,
                        (extract(epoch FROM before_time)*1000)::bigint,
                        (extract(epoch FROM at_time)*1000)::bigint,
                        extract(isodow FROM game_server_save_day(before_time))::smallint,
                        extract(isodow FROM game_server_save_day(at_time))::smallint
                 FROM (SELECT $2::text::timestamptz AS before_time,$3::text::timestamptz AS at_time) v",
            )
            .bind(date)
            .bind(&before)
            .bind(&at)
            .fetch_one(&mut *tx)
            .await?;
        assert!(exact, "{date} must save at {utc_hour}:00 UTC");
        use oteryn_gameplay_entities::rashid_schedule::server_save_weekday_index;
        assert_eq!(server_save_weekday_index(before_ms), usize::try_from(before_weekday - 1)?);
        assert_eq!(server_save_weekday_index(at_ms), usize::try_from(at_weekday - 1)?);
        tx.rollback().await?;
    }

    let dst_lengths: (i64, i64) = sqlx::query_as(
        "SELECT extract(epoch FROM game_server_save_at(DATE '2026-03-29')
                                       -game_server_save_at(DATE '2026-03-28'))::bigint,
                extract(epoch FROM game_server_save_at(DATE '2026-10-25')
                                       -game_server_save_at(DATE '2026-10-24'))::bigint",
    )
    .fetch_one(pool).await?;
    assert_eq!(dst_lengths, (23 * 3600, 25 * 3600));

    let mut tx = pool.begin().await?;
    let world = id(77);
    // Simulate an epoch already published under 0562 before migration 3328.
    sqlx::query("ALTER TABLE game_world_reset_epochs DISABLE TRIGGER world_reset_server_save_policy")
        .execute(&mut *tx).await?;
    insert_epoch(&mut tx, world, 0, "2026-11-23 06:00 Europe/Berlin", true).await?;
    sqlx::query("ALTER TABLE game_world_reset_epochs ENABLE TRIGGER world_reset_server_save_policy")
        .execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,0)")
        .bind(world.as_slice()).execute(&mut *tx).await?;

    for (epoch, instant, expected_legacy) in [
        (0, "", true),
        (1, "2026-11-23 08:00 Europe/Berlin", true),
        (2, "2026-11-23 10:00 Europe/Berlin", false),
    ] {
        if epoch != 0 {
            insert_epoch(&mut tx, world, epoch, instant, false).await?;
            sqlx::query("UPDATE game_world_reset_active SET reset_epoch=$2::text::numeric WHERE world_id=encode($1,'hex')::uuid")
                .bind(world.as_slice()).bind(epoch.to_string()).execute(&mut *tx).await?;
        }
        assert_periods(&mut tx, world, "2026-11-23", "2026-11-23", false).await?;
        let legacy: bool = sqlx::query_scalar(
            "SELECT legacy_six_am FROM game_world_reset_history WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric",
        )
        .bind(world.as_slice()).bind((epoch + 1).to_string()).fetch_one(&mut *tx).await?;
        assert_eq!(legacy, expected_legacy);
    }
    let skipped_double_save: bool = sqlx::query_scalar(
        "SELECT game_server_save_next(TIMESTAMPTZ '2026-11-23 08:00 Europe/Berlin', DATE '2026-11-23')
                = TIMESTAMPTZ '2026-11-24 10:00 Europe/Berlin'",
    )
    .fetch_one(&mut *tx).await?;
    assert!(skipped_double_save);
    tx.rollback().await?;
    eprintln!("P04CAL PG EXECUTED: real Board and Daily Reward boundaries and legacy publication");
    Ok(())
}

async fn insert_epoch(
    tx: &mut sqlx::PgConnection,
    world: [u8; 16],
    epoch: i32,
    instant: &str,
    legacy: bool,
) -> TestResult {
    sqlx::query(
        "INSERT INTO game_world_reset_epochs
          (world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,
           frame_digest,calendar,calendar_revision,activated_at,reset_month,legacy_six_am)
         VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,'map-1',$3,$3,$3,
           'TIBIA_SERVER_SAVE_BERLIN_06',1,$4::text::timestamptz,'2026-11',$5)",
    )
    .bind(world.as_slice())
    .bind(epoch.to_string())
    .bind([9_u8; 32].as_slice())
    .bind(instant)
    .bind(legacy)
    .execute(tx)
    .await?;
    Ok(())
}

async fn assert_periods(
    tx: &mut sqlx::PgConnection,
    world: [u8; 16],
    date: &str,
    monday: &str,
    before: bool,
) -> TestResult {
    let (weekly, expected_weekly, daily, expected_daily, history_day): (i64, i64, i32, i32, i64) =
        sqlx::query_as(
            "SELECT game_task_board_period(a.world_id),
                    ($3::text::date-DATE '1970-01-01')::bigint,
                    game_task_board_reset_day(a.world_id)-DATE '1970-01-01',
                    $2::text::date-DATE '1970-01-01'-$4::boolean::integer,
                    h.reset_day
             FROM game_world_reset_active a
             JOIN game_world_reset_history h ON (h.world_id,h.reset_epoch)=(a.world_id,a.reset_epoch+1)
             WHERE a.world_id=encode($1,'hex')::uuid",
        )
        .bind(world.as_slice())
        .bind(date)
        .bind(monday)
        .bind(before)
        .fetch_one(tx)
        .await?;
    assert_eq!(weekly, expected_weekly, "{date} before={before}");
    assert_eq!(daily, expected_daily, "{date} before={before}");
    assert_eq!(history_day, i64::from(expected_daily), "{date} before={before}");
    Ok(())
}
