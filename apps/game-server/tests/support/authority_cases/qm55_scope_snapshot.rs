//! A scoped admission snapshot must retain its World save while reading history.
use crate::bestiary_postgres_harness as f;

#[test]
fn qm55_scoped_admission_holds_world_save_until_history_is_read() -> f::TestResult {
    let Some(admin) = f::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            f::runtime()
                .map_err(f::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h = f::Harness::create(admin, "qm55_scope_snapshot", true).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "M55_PG_EXECUTED scope_snapshot duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "M55 scope snapshot worker failed")?
        .map_err(Into::into)
}

async fn exercise(h: &f::Harness) -> f::TestResult {
    let world = f::scope()?.world_id();
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,'map-1',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,'2031-06-13T12:00:00Z','2031-06')")
        .bind(world.as_bytes().as_slice()).bind([7_u8;32].as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(world.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    let seal = h.recovery.seal_current().map_err(f::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(f::debug)?;
    let character = f::fence(1)?.character_id;
    // Pause the real canonical reader at its history query, after its root/epoch
    // read. This table lock does not lock the World save row itself.
    let mut history_pause = h.pool.begin().await?;
    sqlx::query("LOCK TABLE ONLY game_character_counter_quest_scopes IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *history_pause)
        .await?;
    let read = h.root.read_character_quest_state(&authority, character);
    let pool = h.pool.clone();
    let probe = tokio::spawn(async move {
        let waiting = tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock' AND query LIKE 'SELECT DISTINCT ON%game_current_counter_quest_progress%')")
                    .fetch_one(&pool).await?;
                if waiting { return Ok::<_, sqlx::Error>(()); }
                tokio::time::sleep(std::time::Duration::from_millis(2)).await;
            }
        }).await;
        let save_lock = if matches!(waiting, Ok(Ok(()))) {
            Some(sqlx::query("SELECT reset_epoch FROM game_world_reset_active WHERE world_id=encode($1,'hex')::uuid FOR UPDATE NOWAIT")
                .bind(world.as_bytes().as_slice()).fetch_one(&pool).await)
        } else {
            None
        };
        // Always release the pause before joining the reader or asserting.
        history_pause.rollback().await?;
        Ok::<_, sqlx::Error>(save_lock)
    });
    let snapshot = read.await;
    let save_lock = probe.await?;
    snapshot?;
    let lock = save_lock?.ok_or("canonical reader did not reach the history query")?;
    assert!(
        matches!(lock, Err(sqlx::Error::Database(ref error)) if error.code().as_deref() == Some("55P03")),
        "a World save must not overtake an admission snapshot between its epoch and history reads"
    );
    // The reader released its save fence at commit; the normal reset owner can
    // obtain the row immediately afterward.
    sqlx::query("SELECT reset_epoch FROM game_world_reset_active WHERE world_id=encode($1,'hex')::uuid FOR UPDATE NOWAIT")
        .bind(world.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    Ok(())
}
