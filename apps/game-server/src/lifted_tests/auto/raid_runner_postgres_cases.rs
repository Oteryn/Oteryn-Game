//! Durable raid transitions through the production writer and real PostgreSQL grants.
use crate::bestiary_postgres_harness::{Harness, TestResult, configured_admin, runtime};

#[test]
fn raid_runner_postgres_cursor_receipts_and_privileges() -> TestResult {
    let Some(admin) = configured_admin() else {
        eprintln!("RAIDS_GLOBAL POSTGRES_CASE_COMPLETED raid_runner_postgres_cursor_receipts_and_privileges route_less=true");
        return Ok(());
    };
    runtime()?.block_on(async {
        let h = Harness::create(admin, "raid_runner", false).await?;
        let result = exercise(&h).await;
        let cleanup = h.cleanup().await;
        result?;
        cleanup
    })
}

#[derive(Clone, Copy)]
enum GlobalCase { Wave, Announcement, Cleanup, Lost, Badger, Boss, PersistentBoss, LateChannel, FailedSingle, BossBeforeWave }

impl GlobalCase {
    fn name(self) -> &'static str {
        match self {
            Self::Wave => "raids_global_wave", Self::Announcement => "raids_global_announcement",
            Self::Cleanup => "raids_global_cleanup", Self::Lost => "raids_global_lost",
            Self::Badger => "raids_global_badger", Self::Boss => "raids_global_boss",
            Self::PersistentBoss => "raids_global_persistent_boss",
            Self::LateChannel => "raids_global_late_channel",
            Self::FailedSingle => "raids_global_failed_single",
            Self::BossBeforeWave => "raids_global_boss_before_wave",
        }
    }
}

fn global_case(case: GlobalCase) -> TestResult {
    let Some(admin) = configured_admin() else {
        eprintln!("RAIDS_GLOBAL POSTGRES_CASE_COMPLETED {} route_less=true", case.name());
        return Ok(());
    };
    let result = std::thread::Builder::new().stack_size(64 << 20).spawn(move || {
        let run = || -> TestResult {
            runtime()?.block_on(async {
                let h = Harness::create(admin, case.name(), false).await?;
                let result = global_exercise(&h, case).await;
                let cleanup = h.cleanup().await;
                result?;
                cleanup
            })
        };
        run().map_err(|error| error.to_string())
    })?.join().map_err(|_| "raid PostgreSQL case thread failed")?;
    result.map_err(Box::<dyn std::error::Error>::from)?;
    eprintln!("RAIDS_GLOBAL POSTGRES_CASE_COMPLETED {}", case.name());
    Ok(())
}

#[test]
fn raids_global_postgres_wave_placement_replay_identity() -> TestResult { global_case(GlobalCase::Wave) }
#[test]
fn raids_global_postgres_broadcast_delivered_once() -> TestResult { global_case(GlobalCase::Announcement) }
#[test]
fn raids_global_postgres_end_and_cleanup() -> TestResult { global_case(GlobalCase::Cleanup) }
#[test]
fn raids_global_postgres_new_generation_is_lost() -> TestResult { global_case(GlobalCase::Lost) }
#[test]
fn raids_global_postgres_badger_full_durable_run() -> TestResult { global_case(GlobalCase::Badger) }
#[test]
fn raids_global_postgres_boss_death_full_durable_run() -> TestResult { global_case(GlobalCase::Boss) }
#[test]
fn raids_global_postgres_persistent_boss_cleans_after_schedule_done() -> TestResult { global_case(GlobalCase::PersistentBoss) }
#[test]
fn raids_global_postgres_late_channel_does_not_join() -> TestResult { global_case(GlobalCase::LateChannel) }
#[test]
fn raids_global_postgres_failed_single_aborts_later_events() -> TestResult { global_case(GlobalCase::FailedSingle) }
#[test]
fn raids_global_postgres_dead_boss_stops_new_wave_after_pending_ack() -> TestResult { global_case(GlobalCase::BossBeforeWave) }

use crate::bestiary_postgres_harness::{CHANNEL, WORLD, debug, id, register, scope};
use crate::durability::raid_runner::{CommittedRaidAction, CommittedRaidStep, RaidPlacementOutcome};
use crate::durability::raid_schedule::RaidWrite;
use crate::durability::runtime_scope_assignment::{AssignmentCommand, AssignmentOutcome, AssignmentRequest, ControlActor, OperationKey};
use crate::foundation::{ChannelId, RuntimeScopeRefV1, ScopeOwnershipGeneration, WorldId};
use oteryn_gameplay_content::content::ProductionKey;
use oteryn_gameplay_encounter_rules::raid::{Announcement, AnnouncementKind, RaidDefinition, RaidPolicy, Wave, WaveEntry};

fn global_definition(case: GlobalCase) -> TestResult<RaidDefinition> {
    let badger = matches!(case, GlobalCase::Badger);
    let mut definition = RaidDefinition {
        // Synthetic timing for transition qualification, except the donor Badger
        // run: Crystal 201189f abdendriel/badgers.xml:3-4 requests 100 at 1000 ms.
        policy: RaidPolicy::Random { mean_interval_s: if badger { 1440 * 60 } else { 60 }, min_gap_s: 0 },
        despawn_after_ms: if badger { None } else { Some(20) },
        announcements: Vec::new(),
        waves: vec![Wave {
            delay_ms: if badger { 1_000 } else { 10 },
            entries: vec![WaveEntry {
                creature: ProductionKey::new(if badger { "creature:badger" } else { "creature:rat" }).map_err(debug)?,
                count: if badger { 100 } else { 2 },
                area_anchor: ProductionKey::new("area:raid-test").map_err(debug)?,
            }],
        }],
    };
    if matches!(case, GlobalCase::Announcement | GlobalCase::Lost | GlobalCase::Boss | GlobalCase::PersistentBoss) {
        definition.announcements.push(Announcement {
            delay_ms: if matches!(case, GlobalCase::Lost) { 20 } else { 0 },
            text: "A raid approaches.".into(), kind: AnnouncementKind::Warning,
        });
    }
    if matches!(case, GlobalCase::Boss | GlobalCase::PersistentBoss | GlobalCase::BossBeforeWave) {
        definition.despawn_after_ms = if matches!(case, GlobalCase::PersistentBoss) { None } else { Some(60_000) };
        definition.waves[0].entries.push(WaveEntry {
            creature: ProductionKey::new("creature:ferumbras").map_err(debug)?, count: 1,
            area_anchor: ProductionKey::new("area:raid-test").map_err(debug)?,
        });
    }
    if matches!(case, GlobalCase::BossBeforeWave) {
        let mut later = definition.waves[0].clone();
        later.delay_ms = 40;
        later.entries.truncate(1);
        definition.waves.push(later);
    }
    if matches!(case, GlobalCase::FailedSingle) {
        definition.waves[0].entries[0].count = 1;
        for delay_ms in [20, 40] {
            let mut wave = definition.waves[0].clone();
            wave.delay_ms = delay_ms;
            definition.waves.push(wave);
        }
        definition.announcements.push(Announcement {
            delay_ms: 30, text: "This later event must not execute.".into(),
            kind: AnnouncementKind::Advance,
        });
    }
    Ok(definition)
}

async fn global_seed(h: &Harness, definition: &RaidDefinition, firing: [u8; 16], key: &str) -> TestResult {
    let deadline = i64::from(definition.waves.iter().map(|wave| wave.delay_ms).max().ok_or("missing wave")?)
        + i64::from(definition.despawn_after_ms.unwrap_or(0));
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,'map-1',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp(0),'1970-01') ON CONFLICT DO NOTHING")
        .bind(id(WORLD).as_slice()).bind([7_u8;32].as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1) ON CONFLICT DO NOTHING")
        .bind(id(WORLD).as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_raid_schedules VALUES(encode($1,'hex')::uuid,$2,2,1,1,2,60000,0,NULL,false)")
        .bind(id(WORLD).as_slice()).bind(key).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_raid_firings VALUES(encode($1,'hex')::uuid,$2,1,encode($3,'hex')::uuid,1,0,$4,$5,1)")
        .bind(id(WORLD).as_slice()).bind(key).bind(firing.as_slice()).bind(deadline).bind([7_u8;32].as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_raid_targets VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1)")
        .bind(firing.as_slice()).bind(id(CHANNEL).as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_raid_scheduler_clocks VALUES(encode($1,'hex')::uuid,1,0) ON CONFLICT DO NOTHING")
        .bind(id(WORLD).as_slice()).execute(&h.pool).await?;
    Ok(())
}

async fn global_next(h: &Harness, firing: [u8;16], definition: &RaidDefinition, now: i64) -> TestResult<CommittedRaidStep> {
    h.root.advance_raid_run(&h.node, firing, ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
        ScopeOwnershipGeneration::new(1).map_err(debug)?, definition.clone(), now).await?
        .ok_or_else(|| "missing committed raid step".into())
}

fn successful_placements(step: &CommittedRaidStep) -> TestResult<Vec<RaidPlacementOutcome>> {
    let CommittedRaidAction::Wave { creatures, .. } = &step.action else { return Err("expected wave".into()); };
    Ok(creatures.iter().map(|creature| RaidPlacementOutcome { identity: creature.identity, placed: true }).collect())
}

async fn assert_live_effect_guard(h: &Harness, step: &CommittedRaidStep, pending: bool) -> TestResult {
    use crate::durability::raid_runner::raid_effect_pending_in_transaction;
    let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut *tx).await?;
    assert_eq!(raid_effect_pending_in_transaction(&mut tx, &h.node, step, world, 1, [7;32]).await?, pending);
    if pending {
        let mut changed = step.clone();
        match &mut changed.action {
            CommittedRaidAction::Announcement { text, .. } => text.push('!'),
            CommittedRaidAction::Wave { creatures, .. } => {
                creatures.first_mut().ok_or("wave creature")?.creature = ProductionKey::new("creature:changed-body").map_err(debug)?;
            }
            _ => return Err("guard requires a live effect".into()),
        }
        assert!(raid_effect_pending_in_transaction(&mut tx, &h.node, &changed, world, 1, [7;32]).await.is_err());
        assert!(raid_effect_pending_in_transaction(&mut tx, &h.node, step, world, 1, [9;32]).await.is_err());
        assert!(raid_effect_pending_in_transaction(&mut tx, &h.node, step, world, 2, [7;32]).await.is_err());
        changed = step.clone();
        changed.identity.generation = ScopeOwnershipGeneration::new(2).map_err(debug)?;
        assert!(raid_effect_pending_in_transaction(&mut tx, &h.node, &changed, world, 1, [7;32]).await.is_err());
    }
    tx.rollback().await?;
    Ok(())
}

async fn global_exercise(h: &Harness, case: GlobalCase) -> TestResult {
    let definition = global_definition(case)?;
    let firing = id(61);
    let channel = ChannelId::decode(&id(CHANNEL)).map_err(debug)?;
    let generation = ScopeOwnershipGeneration::new(1).map_err(debug)?;
    global_seed(h, &definition, firing, if matches!(case, GlobalCase::Badger) { "raid:badger" } else { "raid:global-test" }).await?;
    if matches!(case, GlobalCase::FailedSingle) {
        return failed_single_exercise(h, firing, definition).await;
    }
    if matches!(case, GlobalCase::BossBeforeWave) {
        return boss_before_wave_exercise(h, firing, definition).await;
    }
    assert_eq!(h.root.due_raid_firings(channel).await?.len(), 1);
    let first_at = if matches!(case, GlobalCase::Announcement | GlobalCase::Boss | GlobalCase::PersistentBoss) { 0 }
        else if matches!(case, GlobalCase::Badger) { 1_000 } else { 10 };
    let first = global_next(h, firing, &definition, first_at).await?;
    let replay = global_next(h, firing, &definition, first_at).await?;
    assert_eq!(first.identity, replay.identity);
    assert_eq!(first.action, replay.action);
    assert!(!first.replayed);
    assert!(replay.replayed);
    assert_live_effect_guard(h, &first, true).await?;
    if matches!(case, GlobalCase::Announcement) {
        assert!(matches!(first.action, CommittedRaidAction::Announcement { .. }));
        assert_eq!(h.root.acknowledge_raid_effect(&h.node, first.identity, Vec::new(), 0).await?, RaidWrite::Applied);
        assert_eq!(h.root.acknowledge_raid_effect(&h.node, first.identity, Vec::new(), 0).await?, RaidWrite::Replayed);
        assert_live_effect_guard(h, &first, false).await?;
        assert!(h.root.advance_raid_run(&h.node, firing, channel, generation, definition, 0).await?.is_none());
        let delivered: i64 = sqlx::query_scalar("SELECT count(*) FROM game_world_raid_effects WHERE action_kind=1 AND state=2").fetch_one(&h.pool).await?;
        assert_eq!(delivered, 1);
        return Ok(());
    }
    let wave = if matches!(case, GlobalCase::Boss | GlobalCase::PersistentBoss) {
        h.root.acknowledge_raid_effect(&h.node, first.identity, Vec::new(), 0).await?;
        global_next(h, firing, &definition, 10).await?
    } else { first };
    let mut outcomes = successful_placements(&wave)?;
    if matches!(case, GlobalCase::Wave) {
        outcomes.get_mut(1).ok_or("second placement")?.placed = false;
    }
    assert_eq!(h.root.acknowledge_raid_effect(&h.node, wave.identity, outcomes.clone(), first_at.max(10)).await?, RaidWrite::Applied);
    assert_eq!(h.root.acknowledge_raid_effect(&h.node, wave.identity, outcomes.clone(), first_at.max(10)).await?, RaidWrite::Replayed);
    assert_live_effect_guard(h, &wave, false).await?;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_world_raid_creature_placements").fetch_one(&h.pool).await?;
    assert_eq!(usize::try_from(count)?, outcomes.len());
    match case {
        GlobalCase::Wave => {
            assert_eq!(h.root.query_raid_schedule(channel).await?.active_creatures, 1);
            outcomes.get_mut(1).ok_or("second placement")?.placed = true;
            assert!(h.root.acknowledge_raid_effect(&h.node, wave.identity, outcomes, 10).await.is_err());
            assert_eq!(h.root.query_raid_schedule(channel).await?.active_creatures, 1);
        }
        GlobalCase::Cleanup | GlobalCase::Badger => {
            let end_at = if matches!(case, GlobalCase::Badger) { 1_000 } else { 30 };
            let end = global_next(h, firing, &definition, end_at).await?;
            assert_eq!(end.action, CommittedRaidAction::Complete { cleanup: !matches!(case, GlobalCase::Badger) });
            assert_eq!(h.root.acknowledge_raid_effect(&h.node, end.identity, Vec::new(), end_at).await?, RaidWrite::Applied);
            let state = h.root.query_raid_schedule(channel).await?;
            assert!(!state.running);
            assert_eq!(state.active_creatures, if matches!(case, GlobalCase::Badger) { 100 } else { 0 });
            if matches!(case, GlobalCase::Badger) {
                assert_eq!(h.root.due_raid_firings(channel).await?.len(), 1);
                let deaths = outcomes.iter().map(|outcome| outcome.identity).collect();
                assert_eq!(h.root.record_raid_creature_deaths(&h.node, end.identity, deaths).await?, RaidWrite::Applied);
            }
            assert!(h.root.due_raid_firings(channel).await?.is_empty());
            assert!(h.root.advance_raid_run(&h.node, firing, channel, generation, definition, end_at + 1).await?.is_none());
        }
        GlobalCase::Lost => {
            let pending_announcement = global_next(h, firing, &definition, 20).await?;
            assert!(matches!(pending_announcement.action, CommittedRaidAction::Announcement { .. }));
            let replacement = register(&h.root, 2).await?;
            let predecessor = h.root.read_runtime_scope_predecessor(scope()?).await.map_err(debug)?.ok_or("scope predecessor")?;
            let outcome = h.writer.submit(&AssignmentRequest {
                operation_key: OperationKey::from_bytes([81;32]),
                actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
                command: AssignmentCommand::Replace { scope: scope()?, predecessor, target: replacement.fact() },
            }).await.map_err(debug)?;
            let AssignmentOutcome::Committed(assignment) = outcome else { return Err("scope replacement failed".into()); };
            let new_generation = ScopeOwnershipGeneration::new(assignment.assignment.ownership_generation).map_err(debug)?;
            crate::bestiary_postgres_harness::publish_readiness_for_scope(
                &h.pool, &h.root, &replacement, new_generation.get(), scope()?,
            ).await?;
            assert!(h.root.advance_raid_run(&h.node, firing, channel, generation, definition.clone(), 30).await?.is_none());
            assert_eq!(h.root.acknowledge_raid_effect(&h.node, pending_announcement.identity, Vec::new(), 30).await?, RaidWrite::CompareFailed);
            // Removed catalogue content is not needed to retire an obsolete generation.
            let lost = h.root.claim_lost_raid_run(&replacement, firing, channel, new_generation, 30).await?.ok_or("LOST cleanup")?;
            assert_eq!(lost.action, CommittedRaidAction::Lost);
            assert_eq!(h.root.acknowledge_raid_effect(&replacement, lost.identity, Vec::new(), 30).await?, RaidWrite::Applied);
            assert!(h.root.advance_raid_run(&replacement, firing, channel, new_generation, definition, 31).await?.is_none());
            let state: i16 = sqlx::query_scalar("SELECT state FROM game_world_raid_channel_runs").fetch_one(&h.pool).await?;
            assert_eq!(state, 3);
            let live: i64 = sqlx::query_scalar("SELECT count(*) FROM game_world_raid_creatures WHERE state=1").fetch_one(&h.pool).await?;
            assert_eq!(live, 0);
            let discarded: i64 = sqlx::query_scalar("SELECT count(*) FROM game_world_raid_effects WHERE action_kind=1 AND state=3").fetch_one(&h.pool).await?;
            assert_eq!(discarded, 1);
        }
        GlobalCase::Boss | GlobalCase::PersistentBoss => {
            let boss = outcomes.last().ok_or("boss placement")?.identity;
            let persistent_cursor = if matches!(case, GlobalCase::PersistentBoss) {
                let scheduled_end = global_next(h, firing, &definition, 10).await?;
                assert_eq!(scheduled_end.action, CommittedRaidAction::Complete { cleanup: false });
                h.root.acknowledge_raid_effect(&h.node, scheduled_end.identity, Vec::new(), 10).await?;
                assert!(!h.root.query_raid_schedule(channel).await?.running);
                assert_eq!(h.root.due_raid_firings(channel).await?.len(), 1);
                Some(sqlx::query_scalar::<_, i32>("SELECT committed_steps FROM game_world_raid_channel_runs").fetch_one(&h.pool).await?)
            } else { None };
            assert!(h.root.finish_raid_after_boss_death(&h.node, wave.identity, vec![boss], 11).await?.is_none());
            assert_eq!(h.root.record_raid_creature_deaths(&h.node, wave.identity, vec![boss]).await?, RaidWrite::Applied);
            assert_eq!(h.root.record_raid_creature_deaths(&h.node, wave.identity, vec![boss]).await?, RaidWrite::Replayed);
            let end = h.root.finish_raid_after_boss_death(&h.node, wave.identity, vec![boss], 12).await?.ok_or("boss cleanup")?;
            assert_eq!(end.action, CommittedRaidAction::Complete { cleanup: true });
            let replay = h.root.finish_raid_after_boss_death(&h.node, wave.identity, vec![boss], 12).await?.ok_or("boss cleanup replay")?;
            assert_eq!(replay.identity, end.identity);
            assert!(replay.replayed);
            assert_eq!(h.root.acknowledge_raid_effect(&h.node, end.identity, Vec::new(), 12).await?, RaidWrite::Applied);
            assert!(h.root.finish_raid_after_boss_death(&h.node, wave.identity, vec![boss], 13).await?.is_none());
            if let Some(cursor) = persistent_cursor {
                let after: i32 = sqlx::query_scalar("SELECT committed_steps FROM game_world_raid_channel_runs").fetch_one(&h.pool).await?;
                assert_eq!(after, cursor, "terminal schedule history remains immutable");
            }
            assert!(!h.root.query_raid_schedule(channel).await?.running);
            assert_eq!(h.root.query_raid_schedule(channel).await?.active_creatures, 0);
            assert!(h.root.due_raid_firings(channel).await?.is_empty());
        }
        GlobalCase::LateChannel => {
            let late = ChannelId::decode(&id(44)).map_err(debug)?;
            let late_scope = RuntimeScopeRefV1::channel(WorldId::decode(&id(WORLD)).map_err(debug)?, late);
            sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) SELECT session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,operation FROM unnest(ARRAY[1,2,3]::SMALLINT[]) AS operation")
                .bind(id(WORLD).as_slice()).bind(late.as_bytes().as_slice()).execute(&h.pool).await?;
            let outcome = h.writer.submit(&AssignmentRequest {
                operation_key: OperationKey::from_bytes([82;32]),
                actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
                command: AssignmentCommand::Assign { scope: late_scope, target: h.node.fact() },
            }).await.map_err(debug)?;
            assert!(matches!(outcome, AssignmentOutcome::Committed(_)));
            crate::bestiary_postgres_harness::publish_readiness_for_scope(
                &h.pool, &h.root, &h.node, 1, late_scope,
            ).await?;
            assert!(h.root.due_raid_firings(late).await?.is_empty());
            assert!(h.root.advance_raid_run(&h.node, firing, late, generation, definition.clone(), 11).await?.is_none());
            assert!(h.root.advance_raid_run(&h.node, firing, channel, ScopeOwnershipGeneration::new(2).map_err(debug)?, definition, 11).await?.is_none());
            let runs: i64 = sqlx::query_scalar("SELECT count(*) FROM game_world_raid_channel_runs").fetch_one(&h.pool).await?;
            assert_eq!(runs, 1);
        }
        GlobalCase::Announcement | GlobalCase::FailedSingle | GlobalCase::BossBeforeWave => return Err("special case returned too late".into()),
    }
    Ok(())
}

async fn boss_before_wave_exercise(h: &Harness, firing: [u8; 16], definition: RaidDefinition) -> TestResult {
    let channel = ChannelId::decode(&id(CHANNEL)).map_err(debug)?;
    let generation = ScopeOwnershipGeneration::new(1).map_err(debug)?;
    assert!(h.root.pending_raid_effect(&h.node, firing, channel, generation).await?.is_none());
    let runs: i64 = sqlx::query_scalar("SELECT count(*) FROM game_world_raid_channel_runs")
        .fetch_one(&h.pool).await?;
    assert_eq!(runs, 0, "a pending read does not create a run");
    let wave = global_next(h, firing, &definition, 10).await?;
    let mut outcomes = successful_placements(&wave)?;
    let boss = outcomes.last().ok_or("boss placement")?.identity;
    let failed = outcomes.first().ok_or("ordinary placement")?.identity;
    outcomes[0].placed = false;
    let pending = h.root.pending_raid_effect(&h.node, firing, channel, generation).await?.ok_or("pending wave")?;
    assert_eq!(pending.identity, wave.identity);
    assert!(pending.replayed);
    assert!(h.root.pending_raid_effect(&h.node, firing, channel, ScopeOwnershipGeneration::new(2).map_err(debug)?).await?.is_none());
    assert_eq!(h.root.record_raid_creature_deaths(&h.node, wave.identity, vec![boss]).await?, RaidWrite::CompareFailed,
        "death before placement ACK is not a replay and cannot enter the observer's cache");
    h.root.acknowledge_raid_effect(&h.node, wave.identity, outcomes, 10).await?;
    assert!(h.root.pending_raid_effect(&h.node, firing, channel, generation).await?.is_none());
    assert_eq!(h.root.record_raid_creature_deaths(&h.node, wave.identity, vec![boss, failed]).await?, RaidWrite::CompareFailed);
    let live: i64 = sqlx::query_scalar("SELECT count(*) FROM game_world_raid_creatures WHERE state=1")
        .fetch_one(&h.pool).await?;
    assert_eq!(live, 2, "one failed placement rejects the whole death batch before mutation");
    assert_eq!(h.root.record_raid_creature_deaths(&h.node, wave.identity, vec![boss]).await?, RaidWrite::Applied);
    assert_eq!(h.root.record_raid_creature_deaths(&h.node, wave.identity, vec![boss]).await?, RaidWrite::Replayed);
    // The consumer was delayed beyond the next wave's deadline. Reading pending
    // effects still creates nothing; the acknowledged boss death ends it first.
    let end = h.root.finish_raid_after_boss_death(&h.node, wave.identity, vec![boss], 100).await?.ok_or("boss end")?;
    assert_eq!(end.action, CommittedRaidAction::Complete { cleanup: true });
    assert_eq!(h.root.pending_raid_effect(&h.node, firing, channel, generation).await?.ok_or("pending end")?.identity, end.identity);
    h.root.acknowledge_raid_effect(&h.node, end.identity, Vec::new(), 100).await?;
    assert!(h.root.advance_raid_run(&h.node, firing, channel, generation, definition, 100).await?.is_none());
    let later: i64 = sqlx::query_scalar("SELECT count(*) FROM game_world_raid_creatures WHERE wave=1")
        .fetch_one(&h.pool).await?;
    assert_eq!(later, 0);
    Ok(())
}

async fn failed_single_exercise(h: &Harness, firing: [u8; 16], definition: RaidDefinition) -> TestResult {
    // Crystal 201189f raids.cpp:391-400 returns false for a failed singlespawn;
    // executeRaidEvent:244-258 resets the sequence without removing earlier actors.
    let channel = ChannelId::decode(&id(CHANNEL)).map_err(debug)?;
    let generation = ScopeOwnershipGeneration::new(1).map_err(debug)?;
    let earlier = global_next(h, firing, &definition, 10).await?;
    let placed = successful_placements(&earlier)?;
    h.root.acknowledge_raid_effect(&h.node, earlier.identity, placed.clone(), 10).await?;
    let failed_wave = global_next(h, firing, &definition, 20).await?;
    let mut outcomes = successful_placements(&failed_wave)?;
    let failed = outcomes.first().ok_or("single placement")?.identity;
    outcomes[0].placed = false;
    // A proposed failure, a successful older placement, and a stale ownership
    // generation cannot close the run. Each changes one authority/evidence fact.
    assert!(h.root.finish_raid_after_failed_single(&h.node, failed_wave.identity, vec![failed], 20).await?.is_none());
    h.root.acknowledge_raid_effect(&h.node, failed_wave.identity, outcomes.clone(), 20).await?;
    assert_eq!(h.root.acknowledge_raid_effect(&h.node, failed_wave.identity, outcomes, 20).await?, RaidWrite::Replayed);
    assert!(h.root.finish_raid_after_failed_single(&h.node, failed_wave.identity, vec![placed[0].identity], 20).await?.is_none());
    let mut wrong_generation = failed_wave.identity;
    wrong_generation.generation = ScopeOwnershipGeneration::new(2).map_err(debug)?;
    assert!(h.root.finish_raid_after_failed_single(&h.node, wrong_generation, vec![failed], 20).await?.is_none());
    assert!(h.root.finish_raid_after_failed_single(&h.node, failed_wave.identity, vec![failed], 19).await.is_err());
    let end = h.root.finish_raid_after_failed_single(&h.node, failed_wave.identity, vec![failed], 21).await?.ok_or("failed-single completion")?;
    assert_eq!(end.action, CommittedRaidAction::Complete { cleanup: false });
    assert_eq!(end.identity.ordinal, failed_wave.identity.ordinal + 1);
    let replay = h.root.finish_raid_after_failed_single(&h.node, failed_wave.identity, vec![failed], 21).await?.ok_or("completion replay")?;
    assert!(replay.replayed);
    assert_eq!(replay.identity, end.identity);
    // A cancellation after commit cannot let the ordinary cursor advance past
    // this pending terminal effect on retry.
    let pending = global_next(h, firing, &definition, 100).await?;
    assert_eq!(pending.identity, end.identity);
    assert_eq!(pending.action, end.action);
    h.root.acknowledge_raid_effect(&h.node, end.identity, Vec::new(), 21).await?;
    let acknowledged = h.root.finish_raid_after_failed_single(&h.node, failed_wave.identity, vec![failed], 22).await?.ok_or("acknowledged abort replay")?;
    assert!(acknowledged.replayed);
    assert_eq!(acknowledged.identity, end.identity);
    assert_eq!(acknowledged.action, end.action);
    assert!(h.root.advance_raid_run(&h.node, firing, channel, generation, definition, 100).await?.is_none());
    let state = h.root.query_raid_schedule(channel).await?;
    assert!(!state.running);
    assert_eq!(state.active_creatures, 1, "earlier successfully placed actors survive");
    let later_waves: i64 = sqlx::query_scalar("SELECT count(*) FROM game_world_raid_creatures WHERE wave>=2").fetch_one(&h.pool).await?;
    let later_announcements: i64 = sqlx::query_scalar("SELECT count(*) FROM game_world_raid_effects WHERE action_kind=1").fetch_one(&h.pool).await?;
    let terminal_receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_world_raid_effects WHERE action_kind=3").fetch_one(&h.pool).await?;
    assert_eq!((later_waves, later_announcements, terminal_receipts), (0, 0, 1));
    let replacement = register(&h.root, 2).await?;
    let predecessor = h.root.read_runtime_scope_predecessor(scope()?).await.map_err(debug)?.ok_or("scope predecessor")?;
    let outcome = h.writer.submit(&AssignmentRequest {
        operation_key: OperationKey::from_bytes([86;32]),
        actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
        command: AssignmentCommand::Replace { scope: scope()?, predecessor, target: replacement.fact() },
    }).await.map_err(debug)?;
    let AssignmentOutcome::Committed(assignment) = outcome else { return Err("scope replacement failed".into()); };
    let mut current_identity = failed_wave.identity;
    current_identity.generation = ScopeOwnershipGeneration::new(assignment.assignment.ownership_generation).map_err(debug)?;
    assert!(h.root.finish_raid_after_failed_single(&h.node, failed_wave.identity, vec![failed], 23).await?.is_none());
    assert!(h.root.finish_raid_after_failed_single(&replacement, current_identity, vec![failed], 23).await?.is_none());
    let receipts_after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_world_raid_effects WHERE action_kind=3").fetch_one(&h.pool).await?;
    assert_eq!(receipts_after, 1, "new generations never acquire old failure authority");
    Ok(())
}

async fn exercise(h: &Harness) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_world_raid_schedules VALUES ('00000000-0000-7000-8000-000000000001', 'raid:runner', 1, 1, 0, 2, 60000, 0, NULL, false)")
        .execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_world_raid_firings VALUES ('00000000-0000-7000-8000-000000000001', 'raid:runner', 1, '00000000-0000-7000-8000-000000000002', 1, 0, 1000, decode(repeat('00',32),'hex'), 1)")
        .execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_world_raid_channel_runs (firing_id,channel_id,owner_generation,updated_at_ms) VALUES ('00000000-0000-7000-8000-000000000002','00000000-0000-7000-8000-000000000003',1,0)")
        .execute(&mut *tx).await?;
    tx.commit().await?;

    // Rolling back an action transaction loses neither its position nor the
    // ability to retry the same receipt key. No in-memory spawn is simulated.
    for commit in [false, true] {
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;
        sqlx::query("SELECT committed_steps FROM game_world_raid_channel_runs WHERE firing_id='00000000-0000-7000-8000-000000000002' AND channel_id='00000000-0000-7000-8000-000000000003' FOR UPDATE")
            .fetch_one(&mut *tx).await?;
        sqlx::query("INSERT INTO game_world_raid_step_receipts VALUES ('00000000-0000-7000-8000-000000000002','00000000-0000-7000-8000-000000000003',0,1,10)")
            .execute(&mut *tx).await?;
        sqlx::query("UPDATE game_world_raid_channel_runs SET committed_steps=1,updated_at_ms=10 WHERE firing_id='00000000-0000-7000-8000-000000000002' AND channel_id='00000000-0000-7000-8000-000000000003'")
            .execute(&mut *tx).await?;
        if commit {
            tx.commit().await?;
        } else {
            tx.rollback().await?;
        }
        let position: i32 = sqlx::query_scalar("SELECT committed_steps FROM game_world_raid_channel_runs WHERE firing_id='00000000-0000-7000-8000-000000000002' AND channel_id='00000000-0000-7000-8000-000000000003'")
            .fetch_one(&h.pool).await?;
        assert_eq!(position, i32::from(commit));
    }

    for (statement, code) in [
        (
            "UPDATE game_world_raid_channel_runs SET owner_generation=2",
            "42501",
        ),
        (
            "UPDATE game_world_raid_channel_runs SET committed_steps=0",
            "23514",
        ),
        (
            "UPDATE game_world_raid_channel_runs SET updated_at_ms=9",
            "23514",
        ),
        ("UPDATE game_world_raid_channel_runs SET state=4", "23514"),
        ("DELETE FROM game_world_raid_channel_runs", "42501"),
        (
            "UPDATE game_world_raid_step_receipts SET action_kind=2",
            "42501",
        ),
        ("DELETE FROM game_world_raid_step_receipts", "42501"),
        (
            "INSERT INTO game_world_raid_step_receipts VALUES ('00000000-0000-7000-8000-000000000002','00000000-0000-7000-8000-000000000003',0,1,10)",
            "23505",
        ),
        (
            "INSERT INTO game_world_raid_step_receipts VALUES ('00000000-0000-7000-8000-000000000002','00000000-0000-7000-8000-000000000004',0,1,10)",
            "23503",
        ),
        (
            "INSERT INTO game_world_raid_step_receipts VALUES ('00000000-0000-7000-8000-000000000002','00000000-0000-7000-8000-000000000003',1,4,10)",
            "23514",
        ),
    ] {
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;
        let error = sqlx::query(sqlx::AssertSqlSafe(statement))
            .execute(&mut *tx)
            .await
            .err()
            .ok_or("invalid runtime mutation must fail")?;
        assert_eq!(
            error.as_database_error().and_then(|e| e.code()).as_deref(),
            Some(code),
            "{statement}"
        );
        tx.rollback().await?;
    }

    // A new generation loses the old run; it cannot take over and replay it.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_world_raid_channel_runs SET state=3,updated_at_ms=20 WHERE firing_id='00000000-0000-7000-8000-000000000002' AND channel_id='00000000-0000-7000-8000-000000000003'")
        .execute(&mut *tx).await?;
    tx.commit().await?;
    for statement in [
        "UPDATE game_world_raid_channel_runs SET state=1",
        "UPDATE game_world_raid_channel_runs SET state=2",
        "UPDATE game_world_raid_channel_runs SET committed_steps=2",
    ] {
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;
        let error = sqlx::query(sqlx::AssertSqlSafe(statement))
            .execute(&mut *tx)
            .await
            .err()
            .ok_or("terminal run cannot resume")?;
        assert_eq!(
            error.as_database_error().and_then(|e| e.code()).as_deref(),
            Some("23514")
        );
        tx.rollback().await?;
    }
    Ok(())
}
