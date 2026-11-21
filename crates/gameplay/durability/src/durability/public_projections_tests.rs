use crate::durability::DurabilityError;
use tokio::sync::oneshot;

struct ReadBarrier {
    read: oneshot::Sender<()>,
    resume: oneshot::Receiver<()>,
}
std::thread_local! {
    static READ_BARRIER: std::cell::RefCell<Option<ReadBarrier>> = const { std::cell::RefCell::new(None) };
}

// Schedule the real publisher between its first data read and decoding. A current-thread
// runtime keeps the barrier local to this case, including when the lib suite runs in parallel.
pub(super) async fn pause_after_highscore_read() -> Result<(), DurabilityError> {
    let barrier = READ_BARRIER.with_borrow_mut(Option::take);
    if let Some(barrier) = barrier {
        barrier
            .read
            .send(())
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        tokio::time::timeout(std::time::Duration::from_secs(10), barrier.resume)
            .await
            .map_err(|_| DurabilityError::InvalidStoredState)?
            .map_err(|_| DurabilityError::InvalidStoredState)?;
    }
    Ok(())
}

use crate::bestiary_postgres_harness as harness;
use harness::{Harness, TestResult, WORLD, debug, id};

async fn snapshot(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, n: i64) -> TestResult {
    let snapshot: i64 = sqlx::query_scalar("INSERT INTO game_world_highscore_snapshots(world_id,category,computed_at_ms,source_cut,row_count) VALUES(encode($1,'hex')::uuid,1,$2,'',0) RETURNING snapshot_id")
        .bind(id(WORLD).as_slice()).bind(n * 1000).fetch_one(&mut **tx).await?;
    sqlx::query("INSERT INTO game_world_highscore_heads(world_id,category,current_snapshot_id) VALUES(encode($1,'hex')::uuid,1,$2) ON CONFLICT(world_id,category) DO UPDATE SET previous_snapshot_id=game_world_highscore_heads.current_snapshot_id,current_snapshot_id=EXCLUDED.current_snapshot_id")
        .bind(id(WORLD).as_slice()).bind(snapshot).execute(&mut **tx).await?;
    Ok(())
}

#[test]
fn highscores_revision_and_contents_share_snapshot_during_refresh() -> TestResult {
    let Some(admin) = harness::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> std::result::Result<(), String> {
            let result: TestResult = harness::runtime().map_err(debug)?.block_on(async move {
                let h = Harness::create(admin, "highscores_snapshot", true).await?;
                let result = race(&h).await.map_err(debug);
                h.cleanup().await?;
                result.map_err(Into::into)
            });
            result.map_err(debug)
        })?
        .join()
        .map_err(|_| "highscores thread failed")?
        .map_err(Into::into)
}

async fn race(h: &Harness) -> TestResult {
    let mut initial = h.pool.begin().await?;
    snapshot(&mut initial, 1).await?;
    initial.commit().await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let before = h
        .root
        .next_public_highscores(&authority, &[])
        .await
        .map_err(debug)?
        .ok_or("missing first batch")?;
    let mut refresh = h.pool.begin().await?;
    let (read, ready) = oneshot::channel();
    let (resume, resumed) = oneshot::channel();
    READ_BARRIER.with_borrow_mut(|slot| {
        assert!(slot.is_none());
        *slot = Some(ReadBarrier {
            read,
            resume: resumed,
        });
    });
    let reader = h.root.next_public_highscores(&authority, &[]);
    let refresh = async move {
        tokio::time::timeout(std::time::Duration::from_secs(10), ready).await??;
        snapshot(&mut refresh, 2).await?;
        refresh.commit().await?;
        resume
            .send(())
            .map_err(|_| "publisher dropped read barrier")?;
        TestResult::Ok(())
    };
    let (read, refreshed) = tokio::task::LocalSet::new()
        .run_until(async move {
            let refreshing = tokio::task::spawn_local(refresh);
            let read = reader.await;
            TestResult::Ok((read, refreshing.await?))
        })
        .await?;
    refreshed?;
    let batch = read.map_err(debug)?.ok_or("missing raced batch")?;
    assert!(batch.lists.iter().all(Vec::is_empty));
    let replay = h
        .root
        .next_public_highscores(&authority, &[])
        .await
        .map_err(debug)?
        .ok_or("missing replay")?;
    assert_eq!(replay.projection_revision, before.projection_revision + 1);
    assert_eq!(replay.computed_at, 2);
    assert!(
        batch == before || batch == replay,
        "a read must contain one complete revision: {batch:?}"
    );
    assert!(
        h.root
            .next_public_highscores(&authority, &[batch.subject()])
            .await
            .map_err(debug)?
            .is_none()
    );
    Ok(())
}
