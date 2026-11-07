//! Schedule metadata and RAIDS-GLOBAL checks using the real migrator/runtime grants.
//! These cases prove durable transitions; live map placement is qualified separately.
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
        eprintln!("RAIDS_GLOBAL POSTGRES_CASE_COMPLETED raid_schedule_postgres_metadata_cas_history_and_world_isolation route_less=true");
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

#[test]
fn raids_global_postgres_scheduler_snapshots_targets_and_fences_replay() -> TestResult {
    let Some(admin) = crate::bestiary_postgres_harness::configured_admin() else {
        eprintln!("RAIDS_GLOBAL POSTGRES_CASE_COMPLETED raids_global_postgres_scheduler_snapshots_targets_and_fences_replay route_less=true");
        return Ok(());
    };
    let result = std::thread::Builder::new().stack_size(64 << 20).spawn(move || {
        let run = || -> TestResult {
            runtime()?.block_on(async {
                let h = Harness::create(admin, "raids_global_schedule", false).await?;
                let result = exercise_live_schedule(&h).await;
                let cleanup = h.cleanup().await;
                result?;
                cleanup
            })
        };
        run().map_err(|error| error.to_string())
    })?.join().map_err(|_| "raid schedule case thread failed")?;
    result.map_err(Box::<dyn std::error::Error>::from)?;
    eprintln!("RAIDS_GLOBAL POSTGRES_CASE_COMPLETED raids_global_postgres_scheduler_snapshots_targets_and_fences_replay");
    Ok(())
}

async fn exercise_live_schedule(h: &Harness) -> TestResult {
    use crate::bestiary_postgres_harness::CHANNEL;
    use crate::durability::raid_schedule::{RaidScheduleCandidate, RaidSchedulerCheck};
    use crate::foundation::{ChannelId, ScopeOwnershipGeneration};
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
    let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
    let channel = ChannelId::decode(&id(CHANNEL)).map_err(debug)?;
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,'map-1',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp(0),'1970-01')")
        .bind(world.as_bytes().as_slice()).bind([7_u8;32].as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(world.as_bytes().as_slice()).execute(&h.pool).await?;
    let mut request = RaidSchedulerCheck {
        world, channel, generation: ScopeOwnershipGeneration::new(1).map_err(debug)?,
        reset_epoch: 1, now_ms: 0, bundle_digest: [7;32],
        candidates: vec![RaidScheduleCandidate {
            key: ProductionKey::new("raid:live-scheduler").map_err(debug)?, definition: definition()?,
            eligible: true, calendar: None, firing_id: id(80), global_margin_ms: 0, lua: None, calendar_window: None,
        }],
    };
    assert!(h.root.check_raid_schedules(&authority, &h.node, request.clone()).await?.firings.is_empty());
    request.now_ms = 60_000;
    let started = h.root.check_raid_schedules(&authority, &h.node, request.clone()).await?;
    assert_eq!(started.firings, vec![(id(80), ProductionKey::new("raid:live-scheduler").map_err(debug)?)]);
    assert!(h.root.check_raid_schedules(&authority, &h.node, request.clone()).await?.firings.is_empty());
    let target: (Vec<u8>, String) = sqlx::query_as("SELECT uuid_send(channel_id),owner_generation::text FROM game_world_raid_targets WHERE firing_id=encode($1,'hex')::uuid")
        .bind(id(80).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(target, (id(CHANNEL).to_vec(), "1".into()));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_world_raid_firings WHERE world_id=encode($1,'hex')::uuid")
        .bind(world.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(count, 1);
    assert_eq!(h.root.due_raid_firings(channel).await?.len(), 1);
    // The same public generation is insufficient for another unassigned channel.
    request.channel = ChannelId::decode(&id(98)).map_err(debug)?;
    assert!(h.root.check_raid_schedules(&authority, &h.node, request.clone()).await?.firings.is_empty());
    request.channel = channel;
    request.bundle_digest = [9;32];
    assert!(h.root.check_raid_schedules(&authority, &h.node, request.clone()).await?.firings.is_empty());
    // A reset owns a fresh startup margin exactly once for the World. Another
    // assigned Channel checking that epoch cannot postpone the deadline.
    request.bundle_digest = [7;32];
    request.reset_epoch = 2;
    request.now_ms = 300_000;
    request.candidates.first_mut().ok_or("candidate")?.global_margin_ms = 3 * 60 * 60 * 1_000;
    request.candidates.first_mut().ok_or("candidate")?.firing_id = id(82);
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,2,$2,'map-2',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp(300),'1970-01')")
        .bind(world.as_bytes().as_slice()).bind([7_u8;32].as_slice()).execute(&h.pool).await?;
    sqlx::query("UPDATE game_world_reset_active SET reset_epoch=2 WHERE world_id=encode($1,'hex')::uuid")
        .bind(world.as_bytes().as_slice()).execute(&h.pool).await?;
    assert!(h.root.check_raid_schedules(&authority, &h.node, request.clone()).await?.firings.is_empty());
    let clock: (String, i64) = sqlx::query_as("SELECT reset_epoch::text,last_end_at_ms FROM game_world_raid_scheduler_clocks")
        .fetch_one(&h.pool).await?;
    assert_eq!(clock, ("2".into(), 300_000));
    use crate::durability::runtime_scope_assignment::{AssignmentCommand, AssignmentOutcome, AssignmentRequest, ControlActor, OperationKey};
    let second = ChannelId::decode(&id(44)).map_err(debug)?;
    sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) SELECT session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,operation FROM unnest(ARRAY[1,2,3]::SMALLINT[]) AS operation")
        .bind(world.as_bytes().as_slice()).bind(second.as_bytes().as_slice()).execute(&h.pool).await?;
    let assigned = h.writer.submit(&AssignmentRequest {
        operation_key: OperationKey::from_bytes([83;32]),
        actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
        command: AssignmentCommand::Assign {
            scope: crate::foundation::RuntimeScopeRefV1::channel(world, second), target: h.node.fact(),
        },
    }).await.map_err(debug)?;
    assert!(matches!(assigned, AssignmentOutcome::Committed(_)));
    crate::bestiary_postgres_harness::publish_readiness_for_scope(
        &h.pool, &h.root, &h.node, 1, crate::foundation::RuntimeScopeRefV1::channel(world, second),
    ).await?;
    request.channel = second;
    request.now_ms = 360_000;
    assert!(h.root.check_raid_schedules(&authority, &h.node, request.clone()).await?.firings.is_empty());
    let second_clock: (String, i64) = sqlx::query_as("SELECT reset_epoch::text,last_end_at_ms FROM game_world_raid_scheduler_clocks")
        .fetch_one(&h.pool).await?;
    assert_eq!(second_clock, clock);
    request.now_ms = 300_000 + 3 * 60 * 60 * 1_000 - 1;
    assert!(h.root.check_raid_schedules(&authority, &h.node, request.clone()).await?.firings.is_empty());
    request.now_ms += 1;
    assert_eq!(h.root.check_raid_schedules(&authority, &h.node, request).await?.firings.len(), 1);
    Ok(())
}

#[derive(Clone, Copy)]
enum SourceScheduleCase { Lua, LuaInstance, Calendar }

async fn assign_activity_channel(h: &Harness, world: WorldId) -> TestResult<crate::foundation::ChannelId> {
    use crate::durability::runtime_scope_assignment::{AssignmentCommand, AssignmentOutcome, AssignmentRequest, ControlActor, OperationKey};
    let channel = crate::foundation::ChannelId::decode(&id(44)).map_err(debug)?;
    sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) SELECT session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,operation FROM unnest(ARRAY[1,2,3]::SMALLINT[]) AS operation")
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).execute(&h.pool).await?;
    let assigned = h.writer.submit(&AssignmentRequest {
        operation_key: OperationKey::from_bytes([84;32]),
        actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
        command: AssignmentCommand::Assign {
            scope: crate::foundation::RuntimeScopeRefV1::channel(world, channel), target: h.node.fact(),
        },
    }).await.map_err(debug)?;
    assert!(matches!(assigned, AssignmentOutcome::Committed(_)));
    crate::bestiary_postgres_harness::publish_readiness_for_scope(
        &h.pool, &h.root, &h.node, 1, crate::foundation::RuntimeScopeRefV1::channel(world, channel),
    ).await?;
    Ok(channel)
}

async fn publish_activity(
    h: &Harness,
    world: WorldId,
    channel: crate::foundation::ChannelId,
    now_ms: i64,
    players: Vec<(u8, i64)>,
) -> TestResult {
    use crate::durability::raid_schedule::{RaidActivityPlayer, RaidActivitySnapshot};
    use crate::foundation::{GameSessionId, ScopeOwnershipGeneration};
    let players = players.into_iter().map(|(session, active_until_ms)| {
        Ok(RaidActivityPlayer { session: GameSessionId::decode(&id(session)).map_err(debug)?, active_until_ms })
    }).collect::<TestResult<Vec<_>>>()?;
    let mut snapshot = RaidActivitySnapshot {
        world, scope: crate::foundation::RuntimeScopeRefV1::channel(world, channel), generation: ScopeOwnershipGeneration::new(1).map_err(debug)?,
        reset_epoch: 1, observed_at_ms: now_ms, players: Some(players),
    };
    assert!(h.root.publish_raid_activity_snapshot(&h.node, snapshot.clone()).await?);
    snapshot.generation = ScopeOwnershipGeneration::new(2).map_err(debug)?;
    assert!(!h.root.publish_raid_activity_snapshot(&h.node, snapshot.clone()).await?);
    snapshot.generation = ScopeOwnershipGeneration::new(1).map_err(debug)?;
    snapshot.reset_epoch = 2;
    assert!(!h.root.publish_raid_activity_snapshot(&h.node, snapshot).await?);
    Ok(())
}

#[test]
fn raids_global_postgres_lua_deferred_win_persists_and_replays() -> TestResult {
    source_schedule_case(SourceScheduleCase::Lua)
}

#[test]
fn raids_global_postgres_calendar_uses_reset_owner_window() -> TestResult {
    source_schedule_case(SourceScheduleCase::Calendar)
}

#[test]
fn raids_global_postgres_active_instance_players_use_current_scope_generation() -> TestResult {
    source_schedule_case(SourceScheduleCase::LuaInstance)
}

async fn activity_instance_reports(
    h: &Harness,
    world: WorldId,
    now_ms: i64,
) -> TestResult<(store::RaidActivitySnapshot, store::RaidActivitySnapshot)> {
    use crate::durability::runtime_scope_assignment::{AssignmentCommand, AssignmentOutcome, AssignmentRequest, ControlActor, OperationKey};
    use crate::foundation::{GameSessionId, RuntimeScopeRefV1, ScopeOwnershipGeneration};
    let active = RuntimeScopeRefV1::instance(world, id(88)).map_err(debug)?;
    let empty = RuntimeScopeRefV1::instance(world, id(89)).map_err(debug)?;
    for (tag, scope) in [(88, active), (89, empty)] {
        sqlx::query("INSERT INTO game_control_activity_scope_grants SELECT session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,operation FROM unnest(ARRAY[1,2,3]::SMALLINT[]) AS operation")
            .bind(world.as_bytes().as_slice()).bind(id(tag).as_slice()).execute(&h.pool).await?;
        let assigned = h.writer.submit(&AssignmentRequest {
            operation_key: OperationKey::from_bytes([tag;32]),
            actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
            command: AssignmentCommand::Assign { scope, target: h.node.fact() },
        }).await.map_err(debug)?;
        assert!(matches!(assigned, AssignmentOutcome::Committed(_)));
        crate::bestiary_postgres_harness::publish_readiness_for_scope(&h.pool, &h.root, &h.node, 1, scope).await?;
    }
    let mut active = store::RaidActivitySnapshot {
        world, scope: active, generation: ScopeOwnershipGeneration::new(1).map_err(debug)?,
        reset_epoch: 1, observed_at_ms: now_ms,
        players: Some(vec![store::RaidActivityPlayer {
            session: GameSessionId::decode(&id(91)).map_err(debug)?, active_until_ms: now_ms + 300_000,
        }]),
    };
    assert!(h.root.publish_raid_activity_snapshot(&h.node, active.clone()).await?);
    let predecessor = h.root.read_runtime_scope_predecessor(active.scope).await.map_err(debug)?.ok_or("Instance predecessor")?;
    let replacement = h.writer.submit(&AssignmentRequest {
        operation_key: OperationKey::from_bytes([90;32]),
        actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
        command: AssignmentCommand::Replace { scope: active.scope, predecessor, target: h.node.fact() },
    }).await.map_err(debug)?;
    let AssignmentOutcome::Committed(replacement) = replacement else { return Err("Instance replacement".into()); };
    let generation = replacement.assignment.ownership_generation;
    crate::bestiary_postgres_harness::publish_readiness_for_scope(&h.pool, &h.root, &h.node, generation, active.scope).await?;
    assert!(!h.root.publish_raid_activity_snapshot(&h.node, active.clone()).await?);
    active.generation = ScopeOwnershipGeneration::new(generation).map_err(debug)?;
    let empty = store::RaidActivitySnapshot {
        world, scope: empty, generation: ScopeOwnershipGeneration::new(1).map_err(debug)?,
        reset_epoch: 1, observed_at_ms: now_ms, players: Some(Vec::new()),
    };
    Ok((active, empty))
}

fn source_schedule_case(case: SourceScheduleCase) -> TestResult {
    let name = match case { SourceScheduleCase::Lua => "raids_global_lua", SourceScheduleCase::LuaInstance => "raids_global_lua_instance", SourceScheduleCase::Calendar => "raids_global_calendar" };
    let Some(admin) = crate::bestiary_postgres_harness::configured_admin() else {
        eprintln!("RAIDS_GLOBAL POSTGRES_CASE_COMPLETED {name} route_less=true");
        return Ok(());
    };
    let result = std::thread::Builder::new().stack_size(64 << 20).spawn(move || {
        let run = || -> TestResult {
            runtime()?.block_on(async {
                let h = Harness::create(admin, name, false).await?;
                let result = exercise_source_schedule(&h, case).await;
                let cleanup = h.cleanup().await;
                result?;
                cleanup
            })
        };
        run().map_err(|error| error.to_string())
    })?.join().map_err(|_| "source schedule case thread failed")?;
    result.map_err(Box::<dyn std::error::Error>::from)?;
    eprintln!("RAIDS_GLOBAL POSTGRES_CASE_COMPLETED {name}");
    Ok(())
}

async fn exercise_source_schedule(h: &Harness, case: SourceScheduleCase) -> TestResult {
    use crate::bestiary_postgres_harness::CHANNEL;
    use crate::durability::raid_schedule::{
        CalendarRaidScheduleCandidate, LuaRaidScheduleCandidate, RaidScheduleCandidate, RaidSchedulerCheck,
    };
    use crate::foundation::{ChannelId, ScopeOwnershipGeneration};
    use oteryn_gameplay_encounter_rules::raid::{LuaRaidPolicy, LuaRaidState, RaidCalendarWindow};
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
    let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
    let channel = ChannelId::decode(&id(CHANNEL)).map_err(debug)?;
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,'map-1',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp(0),'1970-01')")
        .bind(world.as_bytes().as_slice()).bind([7_u8;32].as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(world.as_bytes().as_slice()).execute(&h.pool).await?;
    let mut rule = definition()?;
    rule.policy = RaidPolicy::SourceScheduled;
    let mut request = RaidSchedulerCheck {
        world, channel, generation: ScopeOwnershipGeneration::new(1).map_err(debug)?,
        reset_epoch: 1, now_ms: 0, bundle_digest: [7;32],
        candidates: vec![RaidScheduleCandidate {
            key: ProductionKey::new("raid:source-scheduler").map_err(debug)?, definition: rule,
            eligible: true, calendar: None, firing_id: id(81), global_margin_ms: 0,
            lua: match case {
                SourceScheduleCase::Lua | SourceScheduleCase::LuaInstance => Some(LuaRaidScheduleCandidate {
                    // Synthetic certain win isolates persisted deferred-player gating.
                    policy: LuaRaidPolicy {
                        initial_chance: Some(100.0), target_chance_per_day: 100.0,
                        max_chance_per_check: 100.0, min_gap_ms: 0,
                        allowed_weekdays: Vec::new(), min_active_players: if matches!(case, SourceScheduleCase::LuaInstance) { 1 } else { 2 }, max_checks_per_day: None,
                    },
                }),
                SourceScheduleCase::Calendar => None,
            },
            calendar_window: match case {
                SourceScheduleCase::Calendar => Some(CalendarRaidScheduleCandidate {
                    window: RaidCalendarWindow { start_month: 1, start_day: 2, end_month: 1, end_day: 2 },
                    interval_ms: 120_000,
                }),
                SourceScheduleCase::Lua | SourceScheduleCase::LuaInstance => None,
            },
        }],
    };
    assert!(h.root.check_raid_schedules(&authority, &h.node, request.clone()).await?.firings.is_empty());
    request.now_ms = 60_000;
    if matches!(case, SourceScheduleCase::Lua | SourceScheduleCase::LuaInstance) {
        let deferred = h.root.check_raid_schedules(&authority, &h.node, request.clone()).await?;
        assert!(deferred.firings.is_empty() && deferred.retry_activity);
        let unchanged: (String, i64) = sqlx::query_as("SELECT check_ordinal::text,next_check_at_ms FROM game_world_raid_schedules")
            .fetch_one(&h.pool).await?;
        assert_eq!(unchanged, ("0".into(), 60_000), "missing source evidence cannot advance the check");
        publish_activity(h, world, channel, 60_000, Vec::new()).await?;
    }
    let checked = h.root.check_raid_schedules(&authority, &h.node, request.clone()).await?;
    assert!(checked.firings.is_empty() && !checked.retry_activity);
    let before: (String, i64) = sqlx::query_as("SELECT check_ordinal::text,next_check_at_ms FROM game_world_raid_schedules")
        .fetch_one(&h.pool).await?;
    assert_eq!(before, ("1".into(), 120_000));
    assert!(h.root.check_raid_schedules(&authority, &h.node, request.clone()).await?.firings.is_empty());
    let replay: (String, i64) = sqlx::query_as("SELECT check_ordinal::text,next_check_at_ms FROM game_world_raid_schedules")
        .fetch_one(&h.pool).await?;
    assert_eq!(replay, before);
    match case {
        SourceScheduleCase::Lua => {
            let raw: String = sqlx::query_scalar("SELECT state::text FROM game_world_raid_lua_states").fetch_one(&h.pool).await?;
            let state: LuaRaidState = serde_json::from_str(&raw)?;
            assert!(state.trigger_when_possible);
            assert_eq!(state.check_ordinal, 1);
            assert_eq!(state.checks_today, 1);
            // The first Channel has no active players. The World predicate
            // must include a second current owner, and may not guess its count.
            let second = assign_activity_channel(h, world).await?;
            request.now_ms = 120_000;
            publish_activity(h, world, channel, request.now_ms, Vec::new()).await?;
            let deferred = h.root.check_raid_schedules(&authority, &h.node, request.clone()).await?;
            assert!(deferred.firings.is_empty() && deferred.retry_activity);
            let unchanged: String = sqlx::query_scalar("SELECT state::text FROM game_world_raid_lua_states")
                .fetch_one(&h.pool).await?;
            assert_eq!(serde_json::from_str::<LuaRaidState>(&unchanged)?, state);
            publish_activity(h, world, second, request.now_ms,
                vec![(91, 120_001), (92, 420_000)]).await?;
            let unknown = store::RaidActivitySnapshot {
                world, scope: crate::foundation::RuntimeScopeRefV1::channel(world, channel), generation: request.generation, reset_epoch: 1,
                observed_at_ms: request.now_ms, players: None,
            };
            assert!(h.root.publish_raid_activity_snapshot(&h.node, unknown).await?);
            let unknown = h.root.check_raid_schedules(&authority, &h.node, request.clone()).await?;
            assert!(unknown.firings.is_empty() && unknown.retry_activity,
                "an incomplete source invalidates an earlier complete report in the same window");
            publish_activity(h, world, channel, request.now_ms, Vec::new()).await?;
            request.now_ms = 120_001;
            let boundary = h.root.check_raid_schedules(&authority, &h.node, request.clone()).await?;
            assert!(boundary.firings.is_empty() && !boundary.retry_activity,
                "an actor at exactly 300000ms idle is no longer active");
            request.now_ms = 180_001;
            // Only one fresh report in the new 60s window remains unknown.
            publish_activity(h, world, second, request.now_ms,
                vec![(91, 480_001), (92, 480_001)]).await?;
            let stale = h.root.check_raid_schedules(&authority, &h.node, request.clone()).await?;
            assert!(stale.firings.is_empty() && stale.retry_activity);
            publish_activity(h, world, channel, request.now_ms, Vec::new()).await?;
        }
        SourceScheduleCase::LuaInstance => {
            request.now_ms = 120_000;
            publish_activity(h, world, channel, request.now_ms, Vec::new()).await?;
            let (active, empty) = activity_instance_reports(h, world, request.now_ms).await?;
            let absent = h.root.check_raid_schedules(&authority, &h.node, request.clone()).await?;
            assert!(absent.firings.is_empty() && absent.retry_activity,
                "an empty but ready Instance needs a complete zero report");
            assert!(h.root.publish_raid_activity_snapshot(&h.node, empty).await?);
            let stale = h.root.check_raid_schedules(&authority, &h.node, request.clone()).await?;
            assert!(stale.firings.is_empty() && stale.retry_activity,
                "the prior Instance generation cannot supply current players");
            assert!(h.root.publish_raid_activity_snapshot(&h.node, active).await?);
        }
        // UTC epoch starts on January 1 at 01:00 Berlin; next UTC midnight
        // is January 2 at 01:00, inside the exact reset-owner civil window.
        SourceScheduleCase::Calendar => request.now_ms = 86_400_000,
    }
    let started = h.root.check_raid_schedules(&authority, &h.node, request.clone()).await?;
    assert_eq!(started.firings.len(), 1);
    assert_eq!(started.firings[0].0, id(81));
    assert!(h.root.check_raid_schedules(&authority, &h.node, request).await?.firings.is_empty());
    if matches!(case, SourceScheduleCase::Lua | SourceScheduleCase::LuaInstance) {
        let raw: String = sqlx::query_scalar("SELECT state::text FROM game_world_raid_lua_states").fetch_one(&h.pool).await?;
        let state: LuaRaidState = serde_json::from_str(&raw)?;
        assert!(!state.trigger_when_possible);
        assert_eq!(state.last_occurrence_ms, Some(if matches!(case, SourceScheduleCase::LuaInstance) { 120_000 } else { 180_000 }));
        assert_eq!(state.check_ordinal, if matches!(case, SourceScheduleCase::LuaInstance) { 2 } else { 3 });
        assert_eq!(state.checks_today, 1, "deferred win does not draw again");
    }
    let targets: i64 = sqlx::query_scalar("SELECT count(*) FROM game_world_raid_targets").fetch_one(&h.pool).await?;
    assert_eq!(targets, if matches!(case, SourceScheduleCase::Lua) { 2 } else { 1 });
    Ok(())
}
