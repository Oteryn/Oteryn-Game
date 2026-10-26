//! A write-ahead refresh samples the database clock after a conflicting row lock.
use crate::bestiary_postgres_harness::{
    CHARACTER, Harness, TestResult, configured_admin, debug, fence, id, runtime,
};
use crate::domain::CharacterId;
use crate::durability::pvp_state::{PvpDeadlineKinds, PvpWriteAhead};

#[test]
fn pvp_state_refresh_samples_after_conflicting_row_lock() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async {
        let h = Harness::create(admin, "pvp_refresh_lock", true).await?;
        let outcome = refresh_after_lock(&h).await;
        h.cleanup().await?;
        outcome
    })
}

async fn refresh_after_lock(h: &Harness) -> TestResult {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let target = |logout_block, pz_block, white_skull| -> TestResult<_> {
        Ok(vec![PvpWriteAhead {
            character_id: CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?,
            kinds: PvpDeadlineKinds {
                logout_block,
                pz_block,
                white_skull,
            },
        }])
    };
    h.root
        .write_ahead_pvp_deadlines(&authority, &h.node, fence(1)?, target(true, true, true)?)
        .await
        .map_err(debug)?;
    let deadlines = "SELECT (extract(epoch FROM logout_block_until)*1000000)::bigint, \
        (extract(epoch FROM pz_block_until)*1000000)::bigint, \
        (extract(epoch FROM white_skull_until)*1000000)::bigint, revision::bigint \
        FROM game_character_pvp_state WHERE character_id=encode($1,'hex')::uuid";
    let before: (i64, i64, i64, i64) = sqlx::query_as(deadlines)
        .bind(id(CHARACTER).as_slice())
        .fetch_one(&h.pool)
        .await?;

    let mut blocker = h.pool.begin().await?;
    let blocker_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *blocker)
        .await?;
    sqlx::query("SELECT 1 FROM game_character_pvp_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(id(CHARACTER).as_slice()).execute(&mut *blocker).await?;

    let refresh = h.root.write_ahead_pvp_deadlines(
        &authority,
        &h.node,
        fence(1)?,
        target(true, false, false)?,
    );
    let pool = h.pool.clone();
    let release = tokio::spawn(async move {
        // Observe the real upsert waiting on this exact blocker before delaying it.
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let waiting: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM pg_stat_activity a \
                     WHERE a.datname=current_database() AND a.pid<>pg_backend_pid() \
                     AND $1=ANY(pg_blocking_pids(a.pid)) \
                     AND a.query LIKE 'INSERT INTO game_character_pvp_state AS s%')",
                )
                .bind(blocker_pid)
                .fetch_one(&pool)
                .await?;
                if waiting {
                    return Ok::<(), sqlx::Error>(());
                }
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }
        })
        .await
        .map_err(|_| sqlx::Error::Protocol("PvP refresh did not reach the row lock".into()))??;
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        let released_at: i64 =
            sqlx::query_scalar("SELECT (extract(epoch FROM clock_timestamp())*1000000)::bigint")
                .fetch_one(&mut *blocker)
                .await?;
        blocker.commit().await?;
        Ok::<_, sqlx::Error>(released_at)
    });
    let written = refresh.await;
    let released_at = release.await??;
    written.map_err(debug)?;
    let after: (i64, i64, i64, i64) = sqlx::query_as(deadlines)
        .bind(id(CHARACTER).as_slice())
        .fetch_one(&h.pool)
        .await?;
    assert!(
        after.0 >= released_at + 70_000_000,
        "refresh must preserve the full write-ahead after the row-lock wait"
    );
    assert!(after.0 > before.0);
    assert_eq!((after.1, after.2), (before.1, before.2));
    assert_eq!(after.3, before.3 + 1);
    Ok(())
}
