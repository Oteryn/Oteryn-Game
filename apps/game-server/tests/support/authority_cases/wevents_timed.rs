//! WEVENTS: real PostgreSQL state transitions through the existing World owner.
use crate::bestiary_postgres_harness as fixture;
use crate::durability::world_event::changes::timed::{self, TimedActor, TimedRequest};
use oteryn_gameplay_world_change::timed::{EventKind, Input, MINUTE, Phase};

#[test]
fn wevents_postgres_timed_owner() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .name("wevents-pg".into())
        .stack_size(64 << 20)
        .spawn(move || {
            fixture::runtime()
                .map_err(fixture::debug)?
                .block_on(exercise(admin))
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "wevents thread failed")??;
    Ok(())
}

async fn exercise(admin: String) -> fixture::TestResult {
    let started = std::time::Instant::now();
    let h = fixture::Harness::create(admin, "wevents", false).await?;
    let result = run(&h).await;
    h.cleanup().await?;
    result?;
    eprintln!(
        "WEVENTS PG EXECUTED: timed owner, rollback, replay, calendar, jars, reset; duration={:?}",
        started.elapsed()
    );
    Ok(())
}

async fn run(h: &fixture::Harness) -> fixture::TestResult {
    let world = fixture::scope()?.world_id();
    sqlx::query("INSERT INTO game_durability_fresh_admission_receipts SELECT decode('01'||repeat('00',32),'hex'),game_session_id,account_id,character_id,world_id,runtime_scope_channel_id,character_lease_generation,scope_ownership_generation,1,current_transport_ref,1,'{}',1 FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid ON CONFLICT DO NOTHING")
        .bind(fixture::id(fixture::SESSION).as_slice()).execute(&h.pool).await?;
    sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=2,control_loss_epoch=NULL,original_grace_deadline=NULL,predecessor_generation=NULL,fresh_replay_key=decode('01'||repeat('00',32),'hex') WHERE game_session_id=encode($1,'hex')::uuid")
        .bind(fixture::id(fixture::SESSION).as_slice()).execute(&h.pool).await?;
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let recovery = authority.record_for(&h.root).map_err(fixture::debug)?;
    for epoch in 0..=1_i64 {
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,'2031-06-13T12:00:00Z','2031-06')")
            .bind(world.as_bytes().as_slice()).bind(epoch).bind([1u8;32].as_slice()).execute(&h.pool).await?;
    }
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,0)")
        .bind(world.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    let now: i64 = sqlx::query_scalar(
        "SELECT (extract(epoch FROM '2031-06-13T12:00:00Z'::timestamptz)*1000)::bigint",
    )
    .fetch_one(&h.pool)
    .await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    let a = timed::poll(&mut tx, world, 0, EventKind::FlowerMonth, now)
        .await?
        .ok_or("flower")?;
    assert!(a.flower_area().is_some());
    let b = timed::poll(&mut tx, world, 0, EventKind::FlowerMonth, now + 15 * MINUTE)
        .await?
        .ok_or("flower rotation")?;
    assert_eq!(
        b.flower_area(),
        Some((a.flower_area().ok_or("area")? + 1) % 4)
    );
    tx.rollback().await?;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_world_timed_events")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(count, 0);
    // Exercise the new lifecycle schema with the actual runtime role. These
    // are persistence checks, not title qualification or native effect ACKs.
    let lifecycle_started = std::time::Instant::now();
    let mut lifecycle_tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *lifecycle_tx)
        .await?;
    for kind in [
        EventKind::SpringIntoLife,
        EventKind::RiseOfDevovorga,
        EventKind::DevovorgasEssence,
        EventKind::Kingsday,
        EventKind::BankRobbery,
        EventKind::InsectoidInvasion,
    ] {
        let mut state = oteryn_gameplay_world_change::timed::EventState::for_kind(
            kind,
            now,
            now + 24 * 60 * MINUTE,
        )
        .map_err(fixture::debug)?;
        if kind == EventKind::InsectoidInvasion {
            assert!(
                !state
                    .advance(kind, now + 1, Input::Extinguish(1), 0)
                    .map_err(fixture::debug)?
            );
            assert!(
                state
                    .advance(kind, now + 2, Input::Fill(0), 0)
                    .map_err(fixture::debug)?
            );
            assert!(
                state
                    .advance(kind, now + 3, Input::Extinguish(1), 0)
                    .map_err(fixture::debug)?
            );
        }
        let encoded = serde_json::to_value(&state)?;
        sqlx::query("INSERT INTO game_world_timed_events VALUES(encode($1,'hex')::uuid,$2,$3,$4)")
            .bind(world.as_bytes().as_slice())
            .bind(kind.key())
            .bind(77_i64)
            .bind(&encoded)
            .execute(&mut *lifecycle_tx)
            .await?;
        let stored: serde_json::Value = sqlx::query_scalar("SELECT state FROM game_world_timed_events WHERE world_id=encode($1,'hex')::uuid AND event_key=$2 AND cycle_id=77")
            .bind(world.as_bytes().as_slice()).bind(kind.key()).fetch_one(&mut *lifecycle_tx).await?;
        let restored: oteryn_gameplay_world_change::timed::EventState =
            serde_json::from_value(stored)?;
        assert_eq!(restored, state);
        restored.lifecycle.validate().map_err(fixture::debug)?;
    }
    lifecycle_tx.rollback().await?;
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM game_world_timed_events WHERE cycle_id=77"
        )
        .fetch_one(&h.pool)
        .await?,
        0
    );
    eprintln!(
        "WEVENTS PG EXECUTED: six lifecycle kinds, shared water/fire, serialization, runtime role, rollback; duration={:?}",
        lifecycle_started.elapsed()
    );
    // Fresh session and stale epoch refusals reach the same production writer.
    let mut tx = h.pool.begin().await?;
    let mut stale = fixture::fence(1)?;
    stale.connection_generation =
        crate::foundation::ConnectionGeneration::new(2).map_err(fixture::debug)?;
    let base = TimedRequest {
        world,
        epoch: 1,
        kind: EventKind::MummysCurse,
        now_ms: now,
        operation: [1; 16],
        input: Input::Jar(50006),
    };
    assert_eq!(
        timed::interact(
            &mut tx,
            TimedActor {
                recovery: &recovery,
                node: &h.node,
                fence: stale
            },
            base
        )
        .await?,
        None
    );
    tx.rollback().await?;
    // Find native winning occurrences in memory; no test-only draw enters the owner.
    use oteryn_simulation_determinism::{
        DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
    };
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    hash.update(b"oteryn.world_event.timed.v1\0");
    hash.update(world.as_bytes());
    hash.update(EventKind::MummysCurse.key());
    hash.update(0_i64.to_be_bytes());
    let root = GameplayDecisionRoot::from_bytes(hash.finalize().into());
    let mut candidate = 1_u64;
    for aid in 50006..=50010 {
        let operation = loop {
            let mut op = [0; 16];
            op[8..].copy_from_slice(&candidate.to_be_bytes());
            candidate += 1;
            if deterministic_decision_u64(
                &root,
                DecisionOccurrenceId::from_bytes(op),
                "interaction",
                0,
            )
            .map_err(fixture::debug)?
                < u64::MAX / 10
            {
                break op;
            }
        };
        let request = TimedRequest {
            epoch: 0,
            operation,
            input: Input::Jar(aid),
            ..base
        };
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;
        assert_eq!(
            timed::interact(
                &mut tx,
                TimedActor {
                    recovery: &recovery,
                    node: &h.node,
                    fence: fixture::fence(1)?
                },
                request
            )
            .await?,
            Some(true)
        );
        assert_eq!(
            timed::interact(
                &mut tx,
                TimedActor {
                    recovery: &recovery,
                    node: &h.node,
                    fence: fixture::fence(1)?
                },
                request
            )
            .await?,
            Some(true)
        );
        let conflict = TimedRequest {
            input: Input::Jar(aid + 1),
            ..request
        };
        assert_eq!(
            timed::interact(
                &mut tx,
                TimedActor {
                    recovery: &recovery,
                    node: &h.node,
                    fence: fixture::fence(1)?
                },
                conflict
            )
            .await?,
            None
        );
        tx.commit().await?;
    }
    let mut tx = h.pool.begin().await?;
    let jars = timed::poll(&mut tx, world, 0, EventKind::MummysCurse, now)
        .await?
        .ok_or("jars")?;
    assert_eq!(jars.phase, Phase::Victory);
    assert_eq!(jars.boss_spawns, 1);
    assert_eq!(jars.mummy_walls(now + 276_999), (false, false));
    assert_eq!(jars.mummy_walls(now + 277_000), (false, true));
    assert_eq!(jars.mummy_walls(now + 20 * MINUTE), (true, true));
    assert!(
        timed::poll(&mut tx, world, 1, EventKind::MummysCurse, now)
            .await?
            .is_none()
    );
    assert_eq!(
        timed::poll(
            &mut tx,
            world,
            0,
            EventKind::FlowerMonth,
            now + 31 * 86_400_000
        )
        .await?
        .ok_or("expired season")?
        .phase,
        Phase::Failure
    );
    tx.commit().await?;
    sqlx::query(
        "UPDATE game_world_reset_active SET reset_epoch=1 WHERE world_id=encode($1,'hex')::uuid",
    )
    .bind(world.as_bytes().as_slice())
    .execute(&h.pool)
    .await?;
    let mut tx = h.pool.begin().await?;
    let reset = timed::poll(&mut tx, world, 1, EventKind::MummysCurse, now)
        .await?
        .ok_or("reset jars")?;
    assert_eq!(reset.objects, 0);
    assert_eq!(reset.boss_spawns, 0);
    tx.commit().await?;
    Ok(())
}
