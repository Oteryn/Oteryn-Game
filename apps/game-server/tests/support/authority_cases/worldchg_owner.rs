//! WORLDCHG-Q: real migration, restart, reset, rollback and concurrent contributions.
use crate::bestiary_postgres_harness as fixture;
use crate::durability::world_event::changes::{self, ContributionWrite};
use oteryn_gameplay_world_change::owner::ContributionKind;

#[test]
fn worldchg_postgres_persistence_reset_and_atomic_contribution() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .name("worldchg-pg".into())
        .stack_size(64 << 20)
        .spawn(move || {
            fixture::runtime()
                .map_err(|e| e.to_string())?
                .block_on(exercise(admin))
                .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "worldchg thread failed")??;
    Ok(())
}

async fn exercise(admin: String) -> fixture::TestResult {
    let h = fixture::Harness::create(admin, "worldchg", false).await?;
    let result = run(&h).await;
    h.cleanup().await?;
    result
}

async fn transaction(
    pool: &sqlx::PgPool,
) -> Result<sqlx::Transaction<'_, sqlx::Postgres>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    Ok(tx)
}

async fn run(h: &fixture::Harness) -> fixture::TestResult {
    let world = fixture::scope()?.world_id();
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let recovery = authority.record_for(&h.root).map_err(fixture::debug)?;
    // Immutable committed calendar fixtures. Production decisions use these exact
    // epoch rows in the reset owner's transaction, never this test-only seed SQL.
    for epoch in 0..=1_i64 {
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,'2031-06-13T12:00:00Z','2031-06')")
            .bind(world.as_bytes().as_slice()).bind(epoch).bind([1u8;32].as_slice()).execute(&h.pool).await?;
    }
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,0)")
        .bind(world.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    let first = h.root.world_changes(world, 0).await?.ok_or("decision")?;
    assert!(first.flower_month && first.grimvale_full_moon);
    assert_eq!(h.root.world_changes(world, 0).await?, Some(first));
    assert!(h.root.world_changes(world, 1).await?.is_none());
    let missing_world =
        crate::foundation::WorldId::decode(&fixture::id(90)).map_err(fixture::debug)?;
    assert!(h.root.world_changes(missing_world, 0).await?.is_none());
    let mut tx = transaction(&h.pool).await?;
    assert_eq!(
        changes::contribute(
            &mut tx,
            &recovery,
            world,
            0,
            [99; 16],
            ContributionKind::Coal,
            7
        )
        .await?,
        ContributionWrite::Applied(7)
    );
    tx.rollback().await?;

    // Independent connections enter together; no task can read/update an unlocked
    // tally. Every new operation counts once, all retry receipts count zero times.
    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(9));
    let mut tasks = Vec::new();
    for operation in 1..=8_u8 {
        let pool = h.pool.clone();
        let recovery = recovery.clone();
        let barrier = barrier.clone();
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            let mut tx = transaction(&pool).await?;
            let result = changes::contribute(
                &mut tx,
                &recovery,
                world,
                0,
                [operation; 16],
                ContributionKind::Coal,
                3,
            )
            .await?;
            tx.commit().await?;
            Ok::<_, crate::durability::DurabilityError>(result)
        }));
    }
    barrier.wait().await;
    let mut observed = Vec::new();
    for task in tasks {
        observed.push(task.await??);
    }
    for total in (3..=24).step_by(3) {
        assert!(observed.contains(&ContributionWrite::Applied(total)));
    }
    let mut tx = transaction(&h.pool).await?;
    assert_eq!(
        changes::tally(&mut tx, world, 0, ContributionKind::Coal).await?,
        24
    );
    assert!(matches!(
        changes::contribute(
            &mut tx,
            &recovery,
            world,
            0,
            [1; 16],
            ContributionKind::Coal,
            3
        )
        .await?,
        ContributionWrite::Replayed(_)
    ));
    assert_eq!(
        changes::contribute(
            &mut tx,
            &recovery,
            world,
            0,
            [1; 16],
            ContributionKind::Medicine,
            3
        )
        .await?,
        ContributionWrite::Rejected
    );
    assert_eq!(
        changes::contribute(
            &mut tx,
            &recovery,
            world,
            0,
            [2; 16],
            ContributionKind::Coal,
            4
        )
        .await?,
        ContributionWrite::Rejected
    );
    assert_eq!(
        changes::contribute(
            &mut tx,
            &recovery,
            world,
            0,
            [10; 16],
            ContributionKind::Votes,
            0
        )
        .await?,
        ContributionWrite::Rejected
    );
    for kind in [ContributionKind::Medicine, ContributionKind::Votes] {
        let operation = if kind == ContributionKind::Medicine {
            [20; 16]
        } else {
            [21; 16]
        };
        assert_eq!(
            changes::contribute(&mut tx, &recovery, world, 0, operation, kind, 1).await?,
            ContributionWrite::Applied(1)
        );
    }
    tx.commit().await?;

    // This fixture performs only the pointer move; the real save owner composes
    // this with record() in its existing atomic finalization transaction.
    sqlx::query(
        "UPDATE game_world_reset_active SET reset_epoch=1 WHERE world_id=encode($1,'hex')::uuid",
    )
    .bind(world.as_bytes().as_slice())
    .execute(&h.pool)
    .await?;
    let next = h
        .root
        .world_changes(world, 1)
        .await?
        .ok_or("new decision")?;
    assert_eq!(next.reset_epoch, 1);
    let mut tx = transaction(&h.pool).await?;
    assert_eq!(
        changes::contribute(
            &mut tx,
            &recovery,
            world,
            0,
            [30; 16],
            ContributionKind::Coal,
            1
        )
        .await?,
        ContributionWrite::Rejected
    );
    for kind in [
        ContributionKind::Coal,
        ContributionKind::Medicine,
        ContributionKind::Votes,
    ] {
        assert_eq!(changes::tally(&mut tx, world, 1, kind).await?, 0);
    }
    assert_eq!(
        changes::tally(&mut tx, world, 0, ContributionKind::Coal).await?,
        24
    );
    assert_eq!(
        changes::contribute(
            &mut tx,
            &recovery,
            world,
            1,
            [1; 16],
            ContributionKind::Coal,
            1
        )
        .await?,
        ContributionWrite::Applied(1)
    );
    tx.commit().await?;
    // A fresh transaction reloads the complete current projection through its owner.
    let mut tx = transaction(&h.pool).await?;
    assert_eq!(changes::load(&mut tx, world, 1).await?, Some(next));
    let day: serde_json::Value = sqlx::query_scalar("SELECT decision FROM game_world_change_days WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=1")
        .bind(world.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
    // The immutable daily fields persist; current counter fields are a separate,
    // fenced projection and are intentionally omitted from the decision JSON.
    for field in [
        "counter_stages",
        "counter_generations",
        "counter_quest_occurrences",
        "counter_progress",
        "shared_used_sources_mask",
    ] {
        assert!(
            day.get(field).is_none(),
            "current projection {field} was persisted in the immutable daily decision"
        );
    }
    let mut immutable_next = next;
    immutable_next.counter_stages = [0; 8];
    immutable_next.counter_generations = [0; 8];
    immutable_next.counter_quest_occurrences = [0; 8];
    immutable_next.counter_progress = [[0; 3]; 8];
    immutable_next.shared_used_sources_mask = 0;
    assert_eq!(
        serde_json::from_value::<oteryn_gameplay_world_change::daily::DailyWorldChanges>(day)?,
        immutable_next
    );
    tx.rollback().await?;
    eprintln!(
        "WORLDCHG-Q PG EXECUTED: persistence, 8 concurrent writers, replay, rollback, 3 resource resets; PostgreSQL 18.6"
    );
    Ok(())
}
