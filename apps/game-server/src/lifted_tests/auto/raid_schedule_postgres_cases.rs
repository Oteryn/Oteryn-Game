//! EVENTS-RAID-1 metadata store, using the real migrator/runtime SQL grants.
//! No raid runtime is admitted by this test: its integration owners are absent.
use crate::bestiary_postgres_harness::{Harness, TestResult, WORLD, debug, id, runtime};
use crate::character_recovery_fence::CharacterRecoveryFenceV1;
use crate::content::ProductionKey;
use crate::durability::raid_schedule::{self as store, RaidWrite};
use crate::foundation::WorldId;
use oteryn_gameplay_encounter_rules::raid::{
    Announcement, AnnouncementKind, FiringState, RaidAction, RaidDefinition, RaidPolicy,
    RaidSchedule, RaidTransition, Wave, WaveEntry,
};
use sqlx::{Postgres, Transaction};

fn definition() -> TestResult<RaidDefinition> {
    Ok(RaidDefinition {
        policy: RaidPolicy::Random {
            mean_interval_s: 60,
            min_gap_s: 0,
        },
        despawn_after_ms: Some(1_000),
        announcements: vec![Announcement {
            delay_ms: 1_000,
            text: "A raid approaches.".into(),
            kind: AnnouncementKind::Warning,
        }],
        waves: vec![Wave {
            delay_ms: 2_000,
            entries: vec![WaveEntry {
                creature: ProductionKey::new("creature:rat").map_err(debug)?,
                count: 1,
                area_anchor: ProductionKey::new("area:raid").map_err(debug)?,
            }],
        }],
    })
}

fn check(now_ms: i64, seed: u8) -> RaidAction {
    RaidAction::Check {
        now_ms,
        eligible: true,
        calendar: None,
        firing_id: id(seed),
        bundle_digest: [7; 32],
    }
}

async fn transaction(h: &Harness) -> TestResult<Transaction<'_, Postgres>> {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    Ok(tx)
}

async fn apply(
    h: &Harness,
    recovery: &CharacterRecoveryFenceV1,
    change: &RaidTransition,
) -> TestResult<RaidWrite> {
    let mut tx = transaction(h).await?;
    let result = store::compare_and_set(&mut tx, recovery, change).await?;
    tx.commit().await?;
    Ok(result)
}

async fn competing_write(
    pool: sqlx::PgPool,
    recovery: CharacterRecoveryFenceV1,
    change: RaidTransition,
) -> Result<RaidWrite, store::RaidStoreError> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    let result = store::compare_and_set(&mut tx, &recovery, &change).await?;
    tx.commit().await?;
    Ok(result)
}

#[test]
fn raid_schedule_postgres_metadata_cas_history_and_world_isolation() -> TestResult {
    let Some(admin) = crate::bestiary_postgres_harness::configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async {
        let h = Harness::create(admin, "raid_schedule", false).await?;
        let result = exercise(&h).await;
        let cleanup = h.cleanup().await;
        result?;
        cleanup
    })
}

async fn exercise(h: &Harness) -> TestResult {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let recovery = authority.record_for(&h.root).map_err(debug)?;
    let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
    let other_world = WorldId::decode(&id(90)).map_err(debug)?;
    let key = ProductionKey::new("raid:badger").map_err(debug)?;
    let d = definition()?;

    for target in [world, other_world] {
        let mut tx = transaction(h).await?;
        assert_eq!(
            store::initialize(&mut tx, &recovery, target, &key, 1, 0).await?,
            RaidWrite::Applied
        );
        assert_eq!(
            store::initialize(&mut tx, &recovery, target, &key, 1, 0).await?,
            RaidWrite::Replayed
        );
        tx.commit().await?;
    }
    let initial = RaidSchedule::initial(1, 0)?;
    let first = initial.plan(world, &key, &d, check(60_000, 10))?;
    // Rollback must roll back the firing and the schedule in the same TX.
    let mut tx = transaction(h).await?;
    assert_eq!(
        store::compare_and_set(&mut tx, &recovery, &first).await?,
        RaidWrite::Applied
    );
    tx.rollback().await?;
    let mut tx = transaction(h).await?;
    assert_eq!(
        store::load(&mut tx, world, &key).await?,
        Some(initial.clone())
    );
    tx.commit().await?;

    // Two writers with different candidate IDs, the same whole expected state.
    let loser = initial.plan(world, &key, &d, check(60_000, 11))?;
    let a = tokio::spawn(competing_write(
        h.pool.clone(),
        recovery.clone(),
        first.clone(),
    ));
    let b = tokio::spawn(competing_write(
        h.pool.clone(),
        recovery.clone(),
        loser.clone(),
    ));
    let outcomes = [a.await??, b.await??];
    assert_eq!(
        outcomes
            .iter()
            .filter(|o| **o == RaidWrite::Applied)
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|o| **o == RaidWrite::CompareFailed)
            .count(),
        1
    );
    let winner = if outcomes[0] == RaidWrite::Applied {
        &first
    } else {
        &loser
    };
    assert_eq!(apply(h, &recovery, winner).await?, RaidWrite::Replayed);
    let mut tx = transaction(h).await?;
    assert_eq!(
        store::load(&mut tx, world, &key).await?,
        Some(winner.after().clone())
    );
    assert_eq!(
        store::load(&mut tx, other_world, &key).await?,
        Some(initial)
    );
    tx.commit().await?;

    let mut current = winner.after().clone();
    for ordinal in 1..=11_u8 {
        if ordinal > 1 {
            let fire = current.plan(
                world,
                &key,
                &d,
                check(i64::from(ordinal) * 60_000, 20 + ordinal),
            )?;
            assert_eq!(apply(h, &recovery, &fire).await?, RaidWrite::Applied);
            current = fire.after().clone();
        }
        let close = current.plan(
            world,
            &key,
            &d,
            RaidAction::Close {
                now_ms: i64::from(ordinal) * 60_000 + 3_000,
            },
        )?;
        assert_eq!(apply(h, &recovery, &close).await?, RaidWrite::Applied);
        current = close.after().clone();
        let mut tx = transaction(h).await?;
        let reloaded = store::load(&mut tx, world, &key)
            .await?
            .ok_or("missing schedule")?;
        assert_eq!(reloaded, current);
        assert_eq!(
            reloaded.latest_firing.as_ref().map(|f| f.ordinal),
            Some(u64::from(ordinal))
        );
        tx.commit().await?;
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_world_raid_firings WHERE world_id=encode($1,'hex')::uuid AND raid_key=$2")
        .bind(world.as_bytes().as_slice()).bind(key.as_str()).fetch_one(&h.pool).await?;
    assert_eq!(count, 11);
    assert_eq!(apply(h, &recovery, winner).await?, RaidWrite::CompareFailed);

    // Full expected snapshot guard, even with the correct revision/ordinal.
    let mut wrong = current.clone();
    wrong
        .latest_firing
        .as_mut()
        .ok_or("missing firing")?
        .bundle_digest = [99; 32];
    let stale = wrong.plan(world, &key, &d, check(720_000, 60))?;
    assert_eq!(apply(h, &recovery, &stale).await?, RaidWrite::CompareFailed);

    // Every written column is granted. Immutable facts and deletes are refused.
    let mut tx = transaction(h).await?;
    assert!(
        sqlx::query("UPDATE game_world_raid_firings SET fired_at_ms=fired_at_ms+1")
            .execute(&mut *tx)
            .await
            .is_err()
    );
    tx.rollback().await?;
    let mut tx = transaction(h).await?;
    assert!(
        sqlx::query("DELETE FROM game_world_raid_schedules")
            .execute(&mut *tx)
            .await
            .is_err()
    );
    tx.rollback().await?;

    // Persistent same-epoch protection after close, and reset cancellation.
    let mut persistent = d.clone();
    persistent.despawn_after_ms = None;
    let persistent_key = ProductionKey::new("raid:persistent").map_err(debug)?;
    let mut tx = transaction(h).await?;
    assert_eq!(
        store::initialize(&mut tx, &recovery, world, &persistent_key, 1, 0).await?,
        RaidWrite::Applied
    );
    tx.commit().await?;
    let fire = RaidSchedule::initial(1, 0)?.plan(
        world,
        &persistent_key,
        &persistent,
        check(60_000, 70),
    )?;
    assert_eq!(apply(h, &recovery, &fire).await?, RaidWrite::Applied);
    let close = fire.after().plan(
        world,
        &persistent_key,
        &persistent,
        RaidAction::Close { now_ms: 62_000 },
    )?;
    assert_eq!(apply(h, &recovery, &close).await?, RaidWrite::Applied);
    let blocked = close
        .after()
        .plan(world, &persistent_key, &persistent, check(120_000, 71))?;
    assert_eq!(blocked.after().next_firing_ordinal, 2);
    assert_eq!(apply(h, &recovery, &blocked).await?, RaidWrite::Applied);
    let reset = blocked.after().plan(
        world,
        &persistent_key,
        &persistent,
        RaidAction::Reset {
            reset_epoch: 2,
            now_ms: 120_000,
        },
    )?;
    assert_eq!(apply(h, &recovery, &reset).await?, RaidWrite::Applied);
    let second = reset
        .after()
        .plan(world, &persistent_key, &persistent, check(180_000, 72))?;
    assert_eq!(apply(h, &recovery, &second).await?, RaidWrite::Applied);
    let reset = second.after().plan(
        world,
        &persistent_key,
        &persistent,
        RaidAction::Reset {
            reset_epoch: 3,
            now_ms: 180_000,
        },
    )?;
    assert_eq!(
        reset.after().latest_firing.as_ref().map(|f| f.state),
        Some(FiringState::Cancelled)
    );
    assert_eq!(apply(h, &recovery, &reset).await?, RaidWrite::Applied);

    let mut stale_fence = recovery.clone();
    stale_fence.recovery_generation += 1;
    let mut tx = transaction(h).await?;
    assert!(
        store::compare_and_set(&mut tx, &stale_fence, &reset)
            .await
            .is_err()
    );
    tx.rollback().await?;

    // Stored projection corruption fails closed on decode, never silently resets.
    sqlx::query("UPDATE game_world_raid_schedules SET next_firing_ordinal=99 WHERE world_id=encode($1,'hex')::uuid AND raid_key=$2")
        .bind(world.as_bytes().as_slice()).bind(key.as_str()).execute(&h.pool).await?;
    let mut tx = transaction(h).await?;
    assert!(store::load(&mut tx, world, &key).await.is_err());
    tx.rollback().await?;
    Ok(())
}
