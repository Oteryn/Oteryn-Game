//! First WEVENTS native bindings: real World admission, current sessions and existing raid runner.
use crate::bestiary_postgres_harness as fixture;
use crate::durability::raid_runner::{CommittedRaidAction, RaidPlacementOutcome};
use crate::durability::raid_schedule::{
    CalendarRaidScheduleCandidate, RaidScheduleCandidate, RaidSchedulerCheck,
};
use crate::durability::world_event::changes::timed::{
    self, EventKind, Input, TimedActor, TimedRequest,
};
use oteryn_gameplay_content::content::ProductionKey;
use oteryn_gameplay_encounter_rules::raid::{CatalogueSchedule, RaidCatalogue};
use oteryn_gameplay_world_change::timed::Phase;

#[test]
fn wevents_postgres_first_title_bindings() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .name("wevents-bindings".into())
        .stack_size(64 << 20)
        .spawn(move || {
            fixture::runtime()
                .map_err(fixture::debug)?
                .block_on(async {
                    for case in ["chyllfroest", "flower", "mummy"] {
                        let started = std::time::Instant::now();
                        let h = fixture::Harness::create(
                            admin.clone(),
                            &format!("wevents_{case}"),
                            false,
                        )
                        .await?;
                        let result = match case {
                            "chyllfroest" => chyllfroest(&h).await,
                            "flower" => flower(&h).await,
                            _ => mummy(&h).await,
                        };
                        h.cleanup().await?;
                        result?;
                        eprintln!(
                            "WEVENTS PG EXECUTED: {case} native owner binding and timed provenance; duration={:?}",
                            started.elapsed()
                        );
                    }
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "WEVENTS binding thread")??;
    Ok(())
}

async fn seed(h: &fixture::Harness, at: i64) -> fixture::TestResult {
    let world = fixture::scope()?.world_id();
    for epoch in [1_i64, 2] {
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2,$3,'wevents-map',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp($4::double precision/1000),to_char(to_timestamp($4::double precision/1000) AT TIME ZONE 'Europe/Berlin','YYYY-MM'))")
            .bind(world.as_bytes().as_slice()).bind(epoch).bind([7u8;32].as_slice()).bind(at).execute(&h.pool).await?;
    }
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(world.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    Ok(())
}

fn candidate(key: &str, firing: u8) -> fixture::TestResult<RaidScheduleCandidate> {
    let catalogue = RaidCatalogue::embedded().map_err(fixture::debug)?;
    let key = ProductionKey::new(key).map_err(fixture::debug)?;
    let raid = catalogue.get(&key).ok_or("event raid")?;
    let selected = raid
        .select([7; 32], fixture::id(firing))
        .map_err(fixture::debug)?;
    let CatalogueSchedule::Calendar {
        window: Some(window),
        interval_ms: Some(interval_ms),
        repeat_persistent,
        start_immediately,
    } = raid.schedule
    else {
        return Err("event calendar".into());
    };
    Ok(RaidScheduleCandidate {
        key,
        definition: selected.definition,
        eligible: true,
        calendar: None,
        firing_id: fixture::id(firing),
        global_margin_ms: 0,
        lua: None,
        calendar_window: Some(CalendarRaidScheduleCandidate {
            window: window.window(),
            interval_ms,
            repeat_persistent,
            start_immediately,
        }),
    })
}

fn request(candidate: RaidScheduleCandidate, at: i64) -> fixture::TestResult<RaidSchedulerCheck> {
    Ok(RaidSchedulerCheck {
        world: fixture::scope()?.world_id(),
        channel: crate::foundation::ChannelId::decode(&fixture::id(fixture::CHANNEL))
            .map_err(fixture::debug)?,
        generation: crate::foundation::ScopeOwnershipGeneration::new(1).map_err(fixture::debug)?,
        reset_epoch: 1,
        now_ms: at,
        bundle_digest: [7; 32],
        candidates: vec![candidate],
    })
}

async fn chyllfroest(h: &fixture::Harness) -> fixture::TestResult {
    let world = fixture::scope()?.world_id();
    let dates:Vec<i64>=sqlx::query_scalar("SELECT (extract(epoch FROM d)*1000)::bigint FROM unnest(ARRAY['2031-01-01T00:00Z','2031-03-31T23:59:59Z','2031-04-01T00:00Z','2031-05-01T23:59:59Z','2031-05-02T00:00Z']::timestamptz[]) d").fetch_all(&h.pool).await?;
    seed(h, dates[0]).await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    assert!(
        timed::poll(&mut tx, world, 1, EventKind::Chyllfroest, dates[1])
            .await?
            .is_none()
    );
    for at in [dates[2], dates[3]] {
        assert_eq!(
            timed::poll(&mut tx, world, 1, EventKind::Chyllfroest, at)
                .await?
                .ok_or("open bridge")?
                .phase,
            Phase::Active
        );
        assert!(timed::seasonal_open(EventKind::Chyllfroest, at));
    }
    assert_eq!(
        timed::poll(&mut tx, world, 1, EventKind::Chyllfroest, dates[4])
            .await?
            .ok_or("closed bridge")?
            .phase,
        Phase::Failure
    );
    assert!(!timed::seasonal_open(EventKind::Chyllfroest, dates[4]));
    assert!(
        timed::poll(&mut tx, world, 2, EventKind::Chyllfroest, dates[3])
            .await?
            .is_none()
    );
    tx.commit().await?;
    Ok(())
}

async fn flower(h: &fixture::Harness) -> fixture::TestResult {
    let at = 1_939_118_400_000_i64;
    seed(h, at).await?;
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let selected = candidate("oteryn:raid.raid.dryad_invasion", 88)?;
    let req = request(selected.clone(), at)?;
    assert_eq!(
        h.root
            .check_raid_schedules(&authority, &h.node, req.clone())
            .await?
            .firings
            .len(),
        1
    );
    assert!(
        h.root
            .check_raid_schedules(&authority, &h.node, req.clone())
            .await?
            .firings
            .is_empty()
    );
    for index in 0..4 {
        let now = at + i64::from(index) * 900_000;
        let notice = h
            .root
            .advance_raid_run(
                &h.node,
                fixture::id(88),
                req.channel,
                req.generation,
                selected.definition.clone(),
                now,
            )
            .await?
            .ok_or("flower notice")?;
        assert!(matches!(
            notice.action,
            CommittedRaidAction::Announcement { .. }
        ));
        h.root
            .acknowledge_raid_effect(&h.node, notice.identity, vec![], now)
            .await?;
        let wave = h
            .root
            .advance_raid_run(
                &h.node,
                fixture::id(88),
                req.channel,
                req.generation,
                selected.definition.clone(),
                now,
            )
            .await?
            .ok_or("flower wave")?;
        let CommittedRaidAction::Wave { creatures, .. } = &wave.action else {
            return Err("flower spawn action".into());
        };
        assert_eq!(creatures.len(), 25);
        assert!(creatures.iter().all(|c|c.identity.wave==index && c.creature.as_str()=="oteryn:creature.dryad"));
        let outcomes = creatures
            .iter()
            .map(|c| RaidPlacementOutcome {
                identity: c.identity,
                placed: false,
            })
            .collect();
        h.root
            .acknowledge_raid_effect(&h.node, wave.identity, outcomes, now)
            .await?;
    }
    let end = h
        .root
        .advance_raid_run(
            &h.node,
            fixture::id(88),
            req.channel,
            req.generation,
            selected.definition.clone(),
            at + 2_700_000,
        )
        .await?
        .ok_or("flower end")?;
    assert_eq!(end.action, CommittedRaidAction::Complete { cleanup: false });
    h.root
        .acknowledge_raid_effect(&h.node, end.identity, vec![], at + 2_700_000)
        .await?;
    let again = request(
        candidate("oteryn:raid.raid.dryad_invasion", 89)?,
        at + 3_600_000,
    )?;
    assert_eq!(
        h.root
            .check_raid_schedules(&authority, &h.node, again)
            .await?
            .firings
            .len(),
        1,
        "donor cycle repeats within the same epoch"
    );
    // A stale reset cannot admit a new cycle, even with the source calendar definition.
    sqlx::query(
        "UPDATE game_world_reset_active SET reset_epoch=2 WHERE world_id=encode($1,'hex')::uuid",
    )
    .bind(req.world.as_bytes().as_slice())
    .execute(&h.pool)
    .await?;
    assert!(
        h.root
            .check_raid_schedules(
                &authority,
                &h.node,
                request(
                    candidate("oteryn:raid.raid.dryad_invasion", 90)?,
                    at + 7_200_000
                )?
            )
            .await?
            .firings
            .is_empty()
    );
    Ok(())
}

fn command_for(
    world: crate::foundation::WorldId,
    epoch: i64,
    cursor: &mut u64,
    winning: bool,
) -> fixture::TestResult<crate::foundation::CommandRef> {
    use oteryn_simulation_determinism::{
        DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
    };
    use sha2::{Digest, Sha256};
    let session = crate::foundation::GameSessionId::decode(&fixture::id(fixture::SESSION))
        .map_err(fixture::debug)?;
    let mut root = Sha256::new();
    root.update(b"oteryn.world_event.timed.v1\0");
    root.update(world.as_bytes());
    root.update(EventKind::MummysCurse.key());
    root.update(epoch.to_be_bytes());
    let root = GameplayDecisionRoot::from_bytes(root.finalize().into());
    for _ in 0..10_000 {
        *cursor += 1;
        let mut hash = Sha256::new();
        hash.update(b"oteryn:wevents:use:v1\0");
        hash.update(session.as_bytes());
        hash.update(cursor.to_be_bytes());
        let mut operation = [0; 16];
        operation.copy_from_slice(&hash.finalize()[..16]);
        let draw = deterministic_decision_u64(
            &root,
            DecisionOccurrenceId::from_bytes(operation),
            "interaction",
            0,
        )
        .map_err(fixture::debug)?;
        if (draw < u64::MAX / 10) == winning {
            return Ok(crate::foundation::CommandRef::new(
                session,
                crate::foundation::CommandId::new(*cursor).map_err(fixture::debug)?,
            ));
        }
    }
    Err("event draw search bound".into())
}

async fn mummy(h: &fixture::Harness) -> fixture::TestResult {
    let world = fixture::scope()?.world_id();
    let now: i64 =
        sqlx::query_scalar("SELECT (extract(epoch FROM statement_timestamp())*1000)::bigint")
            .fetch_one(&h.pool)
            .await?;
    seed(h, now).await?;
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let recovery = authority.record_for(&h.root).map_err(fixture::debug)?;
    let mut cursor = 0;
    let first = command_for(world, 1, &mut cursor, true)?;
    assert!(
        h.root
            .use_mummy_jar(&authority, &h.node, fixture::fence(1)?, 1, first, 50006)
            .await?
            .is_none(),
        "reconnectable is not an active controller"
    );
    sqlx::query("INSERT INTO game_durability_fresh_admission_receipts SELECT decode('01'||repeat('00',32),'hex'),game_session_id,account_id,character_id,world_id,runtime_scope_channel_id,character_lease_generation,scope_ownership_generation,1,current_transport_ref,1,'{}',1 FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid ON CONFLICT DO NOTHING")
        .bind(fixture::id(fixture::SESSION).as_slice()).execute(&h.pool).await?;
    sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=2,control_loss_epoch=NULL,original_grace_deadline=NULL,predecessor_generation=NULL,fresh_replay_key=decode('01'||repeat('00',32),'hex') WHERE game_session_id=encode($1,'hex')::uuid")
        .bind(fixture::id(fixture::SESSION).as_slice()).execute(&h.pool).await?;
    // Reconciliation waits for an in-flight first write, including its invisible
    // state INSERT. It cannot acknowledge "absent" while that writer can commit.
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    hash.update(b"oteryn:wevents:use:v1\0");
    hash.update(first.game_session_id().as_bytes());
    hash.update(first.command_id().get().to_be_bytes());
    let mut operation = [0; 16];
    operation.copy_from_slice(&hash.finalize()[..16]);
    let mut pending = h.pool.begin().await?;
    assert_eq!(
        timed::interact(
            &mut pending,
            TimedActor {
                recovery: &recovery,
                node: &h.node,
                fence: fixture::fence(1)?,
            },
            TimedRequest {
                world,
                epoch: 1,
                kind: EventKind::MummysCurse,
                now_ms: now,
                operation,
                input: Input::Jar(50006),
            }
        )
        .await?,
        Some(true)
    );
    let reconciliation = h.root.reconcile_mummy_use(
        &authority,
        fixture::fence(1)?.character_id,
        world,
        1,
        first,
        50006,
    );
    let mut reconciliation = std::pin::pin!(reconciliation);
    assert!(
        tokio::time::timeout(
            std::time::Duration::from_millis(30),
            reconciliation.as_mut()
        )
        .await
        .is_err(),
        "in-flight commit must settle first"
    );
    pending.commit().await?;
    assert_eq!(reconciliation.await?, Some(true));
    let before = request(candidate("raid:world-events.horestis", 90)?, now)?;
    assert!(
        h.root
            .check_raid_schedules(&authority, &h.node, before.clone())
            .await?
            .firings
            .is_empty()
    );
    for aid in 50006..=50010 {
        let command = if aid == 50006 {
            first
        } else {
            command_for(world, 1, &mut cursor, true)?
        };
        let result = h
            .root
            .use_mummy_jar(&authority, &h.node, fixture::fence(1)?, 1, command, aid)
            .await?
            .ok_or("native jar USE")?;
        assert!(result.0);
        assert_eq!(
            h.root
                .reconcile_mummy_use(
                    &authority,
                    fixture::fence(1)?.character_id,
                    world,
                    1,
                    command,
                    aid
                )
                .await?,
            Some(true)
        );
        assert!(
            h.root
                .use_mummy_jar(&authority, &h.node, fixture::fence(1)?, 1, command, aid)
                .await?
                .ok_or("jar replay")?
                .0
        );
    }
    let state = h
        .root
        .timed_world_event(world, 1, EventKind::MummysCurse)
        .await?
        .ok_or("tomb snapshot")?;
    assert_eq!(state.objects, 5);
    assert_eq!(state.boss_spawns, 1);
    assert_eq!(
        h.root
            .check_raid_schedules(&authority, &h.node, before.clone())
            .await?
            .firings
            .len(),
        1
    );
    assert!(
        h.root
            .check_raid_schedules(&authority, &h.node, before.clone())
            .await?
            .firings
            .is_empty()
    );
    let sealed: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_world_timed_raid_origins WHERE firing_id=encode($1,'hex')::uuid",
    )
    .bind(fixture::id(90).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        sealed, 1,
        "the admitted firing needs immutable event provenance"
    );
    let step = h
        .root
        .advance_raid_run(
            &h.node,
            fixture::id(90),
            before.channel,
            before.generation,
            before.candidates[0].definition.clone(),
            now,
        )
        .await?
        .ok_or("Horestis wave")?;
    let CommittedRaidAction::Wave { creatures, .. } = step.action else {
        return Err("Horestis spawn".into());
    };
    assert_eq!(creatures.len(), 1);
    assert_eq!(creatures[0].creature.as_str(), "oteryn:creature.horestis");
    assert!(creatures[0].timed_event);
    use crate::durability::world_event::changes::timed::{NativeKillOutcome, NativeKillRequest};
    let source = oteryn_gameplay_entities::carrier::RaidSpawnIdentity {
        firing_id: fixture::id(90),
        wave: creatures[0].identity.wave,
        entry: creatures[0].identity.entry,
        index: creatures[0].identity.index,
    };
    let mut occurrence = Vec::new();
    occurrence.extend_from_slice(world.as_bytes());
    occurrence.extend_from_slice(before.channel.as_bytes());
    occurrence.extend_from_slice(&before.generation.get().to_be_bytes());
    occurrence.extend_from_slice(&7_u64.to_be_bytes());
    occurrence.extend_from_slice(&1_u32.to_be_bytes());
    let event = NativeKillRequest {
        source,
        occurrence,
        occurred_at_ms: now,
        participants: vec![fixture::fence(1)?.character_id],
    };
    assert_eq!(
        h.root
            .credit_timed_world_kill(
                &authority,
                &h.node,
                fixture::scope()?,
                before.generation,
                1,
                event.clone()
            )
            .await?,
        NativeKillOutcome::Pending,
        "a native death waits for the independently fenced placement ACK"
    );
    h.root
        .acknowledge_raid_effect(
            &h.node,
            step.identity,
            vec![RaidPlacementOutcome {
                identity: creatures[0].identity,
                placed: true,
            }],
            now,
        )
        .await?;
    assert_eq!(
        h.root
            .credit_timed_world_kill(
                &authority,
                &h.node,
                fixture::scope()?,
                before.generation,
                1,
                event.clone()
            )
            .await?,
        NativeKillOutcome::Applied
    );
    assert_eq!(
        h.root
            .credit_timed_world_kill(
                &authority,
                &h.node,
                fixture::scope()?,
                before.generation,
                1,
                event.clone()
            )
            .await?,
        NativeKillOutcome::AlreadyApplied
    );
    let mut forged = event.clone();
    forged.occurrence[51] = 2;
    assert_eq!(
        h.root
            .credit_timed_world_kill(
                &authority,
                &h.node,
                fixture::scope()?,
                before.generation,
                1,
                forged
            )
            .await?,
        NativeKillOutcome::Refused,
        "one actor cannot contribute a second death"
    );
    let mut empty = event.clone();
    empty.participants.clear();
    assert_eq!(
        h.root
            .credit_timed_world_kill(
                &authority,
                &h.node,
                fixture::scope()?,
                before.generation,
                1,
                empty
            )
            .await?,
        NativeKillOutcome::Refused
    );
    let credited:i64=sqlx::query_scalar("SELECT credited FROM game_world_timed_participants WHERE world_id=encode($1,'hex')::uuid AND event_key='mummys_curse' AND cycle_id=1 AND character_id=encode($2,'hex')::uuid")
        .bind(world.as_bytes().as_slice()).bind(fixture::fence(1)?.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        credited, 6,
        "five real jar admissions plus one native-source credit"
    );
    // Fresh reset, real failed draw, out-of-order attempts and cooldown refusal.
    sqlx::query(
        "UPDATE game_world_reset_active SET reset_epoch=2 WHERE world_id=encode($1,'hex')::uuid",
    )
    .bind(world.as_bytes().as_slice())
    .execute(&h.pool)
    .await?;
    assert_eq!(
        h.root
            .credit_timed_world_kill(
                &authority,
                &h.node,
                fixture::scope()?,
                before.generation,
                1,
                event
            )
            .await?,
        NativeKillOutcome::Refused,
        "reset invalidates old native provenance"
    );
    let failed = command_for(world, 2, &mut cursor, false)?;
    let mut hash = sha2::Sha256::new();
    hash.update(b"oteryn:wevents:use:v1\0");
    hash.update(failed.game_session_id().as_bytes());
    hash.update(failed.command_id().get().to_be_bytes());
    let mut operation = [0; 16];
    operation.copy_from_slice(&hash.finalize()[..16]);
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    let req = TimedRequest {
        world,
        epoch: 2,
        kind: EventKind::MummysCurse,
        now_ms: now,
        operation,
        input: Input::Jar(50006),
    };
    assert_eq!(
        timed::interact(
            &mut tx,
            TimedActor {
                recovery: &recovery,
                node: &h.node,
                fence: fixture::fence(1)?
            },
            req
        )
        .await?,
        Some(false)
    );
    let wrong = TimedRequest {
        operation: [9; 16],
        input: Input::Jar(50007),
        ..req
    };
    assert_eq!(
        timed::interact(
            &mut tx,
            TimedActor {
                recovery: &recovery,
                node: &h.node,
                fence: fixture::fence(1)?
            },
            wrong
        )
        .await?,
        Some(false)
    );
    let retry = TimedRequest {
        operation: [10; 16],
        ..req
    };
    assert_eq!(
        timed::interact(
            &mut tx,
            TimedActor {
                recovery: &recovery,
                node: &h.node,
                fence: fixture::fence(1)?
            },
            retry
        )
        .await?,
        None
    );
    tx.commit().await?;
    Ok(())
}
