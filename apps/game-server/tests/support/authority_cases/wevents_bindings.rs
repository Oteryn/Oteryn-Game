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
                    let catalogue: serde_json::Value = serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../content/encounters/world-changes/owner-catalogue.json")))?;
                    let spring_published = catalogue["entries"].as_array().ok_or("owner entries")?.iter().any(|entry| entry["key"] == "spring_into_life" && entry["status"] == "QUALIFIED");
                    let mut cases = vec!["chyllfroest", "flower", "mummy"];
                    if spring_published { cases.push("spring"); }
                    if catalogue["entries"].as_array().ok_or("owner entries")?.iter().any(|entry|entry["key"]=="kingsday" && entry["status"]=="QUALIFIED") { cases.push("kingsday"); }
                    for case in cases {
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
                            "mummy" => mummy(&h).await,
                            "spring" => spring(&h).await,
                            _ => kingsday(&h).await,
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
    // A server returning in another year must close the retained old cycle,
    // not merely create a fresh active row and leave history active forever.
    let years:Vec<i64>=sqlx::query_scalar("SELECT (extract(epoch FROM d)*1000)::bigint FROM unnest(ARRAY['2032-04-01T00:00Z','2033-04-01T00:00Z']::timestamptz[]) d").fetch_all(&h.pool).await?;
    for at in &years {
        timed::poll(&mut tx, world, 1, EventKind::Chyllfroest, *at)
            .await?
            .ok_or("later annual cycle")?;
    }
    let old_phase:String=sqlx::query_scalar("SELECT state->>'phase' FROM game_world_timed_events WHERE world_id=encode($1,'hex')::uuid AND event_key='chyllfroest' AND cycle_id=$2")
        .bind(world.as_bytes().as_slice()).bind(years[0]).fetch_one(&mut *tx).await?;
    if old_phase != "Failure" {
        tx.rollback().await?;
        return Err("expired annual cycle remained active".into());
    }
    tx.commit().await?;
    let actual = h
        .root
        .timed_world_event(world, 1, EventKind::Chyllfroest)
        .await?;
    h.root.set_world_event_clock_for_test(Some(years[1]))?;
    let observed = h
        .root
        .timed_world_event(world, 1, EventKind::Chyllfroest)
        .await?
        .ok_or("fixture calendar observation")?;
    assert_eq!(observed.start_ms, years[1]);
    assert_eq!(observed.phase, Phase::Active);
    h.root.set_world_event_clock_for_test(None)?;
    assert_eq!(
        h.root
            .timed_world_event(world, 1, EventKind::Chyllfroest)
            .await?,
        actual
    );
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
    let expired = oteryn_gameplay_world_change::timed::EventState::for_kind(
        EventKind::MummysCurse,
        now - 120_000,
        now - 60_000,
    )
    .map_err(fixture::debug)?;
    sqlx::query(
        "INSERT INTO game_world_timed_events VALUES(encode($1,'hex')::uuid,'mummys_curse',0,$2)",
    )
    .bind(world.as_bytes().as_slice())
    .bind(serde_json::to_value(expired)?)
    .execute(&h.pool)
    .await?;
    let mut old_cycle = h.pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:world-timed:'||encode($1,'hex')||':mummys_curse:0',0))")
        .bind(world.as_bytes().as_slice()).execute(&mut *old_cycle).await?;
    let next = command_for(world, 2, &mut cursor, true)?;
    let fence = fixture::fence(1)?;
    let use_current = h
        .root
        .use_mummy_jar(&authority, &h.node, fence, 2, next, 50007);
    let mut use_current = std::pin::pin!(use_current);
    let independent =
        tokio::time::timeout(std::time::Duration::from_secs(1), use_current.as_mut()).await;
    old_cycle.rollback().await?;
    let result = match independent {
        Ok(result) => result?,
        Err(_) => {
            let _ = use_current.await;
            return Err("current jar use waited on expired-cycle housekeeping".into());
        }
    };
    assert!(
        result.is_some(),
        "the current cycle remains independently usable"
    );
    Ok(())
}

async fn spring(h: &fixture::Harness) -> fixture::TestResult {
    // 2031-04-16 06:00 Europe/Berlin, retained public nests and authored timer.
    let at = 1_934_078_400_000_i64;
    seed(h, at)
        .await
        .map_err(|error| format!("Spring PostgreSQL line {}: {error:?}", line!()))?;
    let world = fixture::scope()?.world_id();
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let selected = candidate("raid:world-events.spring-nest-1", 91)?;
    let mut req = request(selected.clone(), at)?;
    assert!(
        h.root
            .check_raid_schedules(&authority, &h.node, req.clone())
            .await
            .map_err(|error| format!("Spring PostgreSQL line {}: {error:?}", line!()))?
            .firings
            .is_empty()
    );
    let mut tx = h
        .pool
        .begin()
        .await
        .map_err(|error| format!("Spring PostgreSQL line {}: {error:?}", line!()))?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await
        .map_err(|error| format!("Spring PostgreSQL line {}: {error:?}", line!()))?;
    let state = timed::poll(&mut tx, world, 1, EventKind::SpringIntoLife, at)
        .await
        .map_err(|error| format!("Spring PostgreSQL line {}: {error:?}", line!()))?
        .ok_or("spring selected")?;
    assert_eq!(state.phase, Phase::Active);
    tx.commit()
        .await
        .map_err(|error| format!("Spring PostgreSQL line {}: {error:?}", line!()))?;
    sqlx::query("INSERT INTO game_durability_fresh_admission_receipts SELECT decode('01'||repeat('00',32),'hex'),game_session_id,account_id,character_id,world_id,runtime_scope_channel_id,character_lease_generation,scope_ownership_generation,1,current_transport_ref,1,'{}',1 FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid ON CONFLICT DO NOTHING")
        .bind(fixture::id(fixture::SESSION).as_slice()).execute(&h.pool).await.map_err(|error| format!("Spring PostgreSQL line {}: {error:?}", line!()))?;
    sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=2,control_loss_epoch=NULL,original_grace_deadline=NULL,predecessor_generation=NULL,fresh_replay_key=decode('01'||repeat('00',32),'hex') WHERE game_session_id=encode($1,'hex')::uuid")
        .bind(fixture::id(fixture::SESSION).as_slice()).execute(&h.pool).await.map_err(|error| format!("Spring PostgreSQL line {}: {error:?}", line!()))?;
    let recovery = authority.record_for(&h.root).map_err(fixture::debug)?;
    for (op, input, expected) in [
        (101, Input::BeginObject(1), Some(true)),
        (102, Input::BeginObject(1), Some(false)),
        (103, Input::Reward, None),
    ] {
        let mut tx = h
            .pool
            .begin()
            .await
            .map_err(|error| format!("Spring PostgreSQL line {}: {error:?}", line!()))?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await
            .map_err(|error| format!("Spring PostgreSQL line {}: {error:?}", line!()))?;
        assert_eq!(
            timed::interact(
                &mut tx,
                TimedActor {
                    recovery: &recovery,
                    node: &h.node,
                    fence: fixture::fence(1)?
                },
                TimedRequest {
                    world,
                    epoch: 1,
                    kind: EventKind::SpringIntoLife,
                    now_ms: at,
                    operation: fixture::id(op),
                    input
                }
            )
            .await
            .map_err(|error| format!("Spring PostgreSQL line {}: {error:?}", line!()))?,
            expected
        );
        tx.commit()
            .await
            .map_err(|error| format!("Spring PostgreSQL line {}: {error:?}", line!()))?;
    }
    req.now_ms = at + 3_600_000 - 1;
    assert!(
        h.root
            .check_raid_schedules(&authority, &h.node, req.clone())
            .await
            .map_err(|error| format!("Spring PostgreSQL line {}: {error:?}", line!()))?
            .firings
            .is_empty()
    );
    req.now_ms += 1;
    assert_eq!(
        h.root
            .check_raid_schedules(&authority, &h.node, req.clone())
            .await
            .map_err(|error| format!("Spring PostgreSQL line {}: {error:?}", line!()))?
            .firings
            .len(),
        1
    );
    let wave = h
        .root
        .advance_raid_run(
            &h.node,
            fixture::id(91),
            req.channel,
            req.generation,
            selected.definition.clone(),
            req.now_ms,
        )
        .await
        .map_err(|error| format!("Spring PostgreSQL line {}: {error:?}", line!()))?
        .ok_or("hatch wave")?;
    let CommittedRaidAction::Wave { creatures, .. } = &wave.action else {
        return Err("hatch wave kind".into());
    };
    assert_eq!(creatures.len(), 1);
    assert!(creatures[0].timed_event);
    assert_eq!(
        creatures[0].creature.as_str(),
        "oteryn:creature.dragon_hatchling"
    );
    let outcomes = creatures
        .iter()
        .map(|c| RaidPlacementOutcome {
            identity: c.identity,
            placed: true,
        })
        .collect();
    h.root
        .acknowledge_raid_effect(&h.node, wave.identity, outcomes, req.now_ms)
        .await
        .map_err(|error| format!("Spring PostgreSQL line {}: {error:?}", line!()))?;
    let end = h
        .root
        .advance_raid_run(
            &h.node,
            fixture::id(91),
            req.channel,
            req.generation,
            selected.definition.clone(),
            req.now_ms,
        )
        .await?
        .ok_or("first hatch completion")?;
    h.root
        .acknowledge_raid_effect(&h.node, end.identity, vec![], req.now_ms)
        .await?;
    let mut tx = h
        .pool
        .begin()
        .await
        .map_err(|error| format!("Spring PostgreSQL line {}: {error:?}", line!()))?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await
        .map_err(|error| format!("Spring PostgreSQL line {}: {error:?}", line!()))?;
    assert_eq!(
        timed::interact(
            &mut tx,
            TimedActor {
                recovery: &recovery,
                node: &h.node,
                fence: fixture::fence(1)?
            },
            TimedRequest {
                world,
                epoch: 1,
                kind: EventKind::SpringIntoLife,
                now_ms: req.now_ms,
                operation: fixture::id(104),
                input: Input::Reward
            }
        )
        .await
        .map_err(|error| format!("Spring PostgreSQL line {}: {error:?}", line!()))?,
        Some(true)
    );
    tx.rollback()
        .await
        .map_err(|error| format!("Spring PostgreSQL line {}: {error:?}", line!()))?; // Preview cannot consume the later sequenced MINT claim.
    let mut tx = h
        .pool
        .begin()
        .await
        .map_err(|error| format!("Spring PostgreSQL line {}: {error:?}", line!()))?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await
        .map_err(|error| format!("Spring PostgreSQL line {}: {error:?}", line!()))?;
    assert_eq!(
        timed::interact(
            &mut tx,
            TimedActor {
                recovery: &recovery,
                node: &h.node,
                fence: fixture::fence(1)?
            },
            TimedRequest {
                world,
                epoch: 1,
                kind: EventKind::SpringIntoLife,
                now_ms: req.now_ms,
                operation: fixture::id(105),
                input: Input::BeginObject(2)
            }
        )
        .await
        .map_err(|error| format!("Spring PostgreSQL line {}: {error:?}", line!()))?,
        Some(true)
    );
    tx.commit()
        .await
        .map_err(|error| format!("Spring PostgreSQL line {}: {error:?}", line!()))?;
    let second = candidate("raid:world-events.spring-nest-2", 92)?;
    let failed = request(second.clone(), at + 7_200_000)?;
    assert_eq!(
        h.root
            .check_raid_schedules(&authority, &h.node, failed.clone())
            .await
            .map_err(|error| format!("Spring PostgreSQL line {}: {error:?}", line!()))?
            .firings
            .len(),
        1
    );
    settle_spring_wave(h, &failed, &second, 92, false)
        .await
        .map_err(|error| format!("Spring PostgreSQL line {}: {error:?}", line!()))?;
    let mut tx = h
        .pool
        .begin()
        .await
        .map_err(|error| format!("Spring PostgreSQL line {}: {error:?}", line!()))?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await
        .map_err(|error| format!("Spring PostgreSQL line {}: {error:?}", line!()))?;
    let state = timed::poll(&mut tx, world, 1, EventKind::SpringIntoLife, failed.now_ms)
        .await
        .map_err(|error| format!("Spring PostgreSQL line {}: {error:?}", line!()))?
        .ok_or("failed hatch state")?;
    assert!(!state.lifecycle.last_success_ms.contains_key(&2));
    assert_eq!(
        state.lifecycle.objects[&2].ready_at(),
        Some(at + 10_800_000)
    );
    tx.commit()
        .await
        .map_err(|error| format!("Spring PostgreSQL line {}: {error:?}", line!()))?;
    let retry = request(
        candidate("raid:world-events.spring-nest-2", 93)?,
        at + 10_800_000,
    )?;
    assert_eq!(
        h.root
            .check_raid_schedules(&authority, &h.node, retry.clone())
            .await
            .map_err(|error| format!("Spring PostgreSQL line {}: {error:?}", line!()))?
            .firings
            .len(),
        1
    );
    settle_spring_wave(h, &retry, &second, 93, true)
        .await
        .map_err(|error| format!("Spring PostgreSQL line {}: {error:?}", line!()))?;
    let retired: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_world_raid_retirements WHERE firing_id=encode($1,'hex')::uuid)")
        .bind(fixture::id(92).as_slice()).fetch_one(&h.pool).await?;
    assert!(retired);
    let terminal: i16 = sqlx::query_scalar(
        "SELECT state FROM game_world_raid_firings WHERE firing_id=encode($1,'hex')::uuid",
    )
    .bind(fixture::id(92).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(terminal, 2, "retirement preserves completed firing history");
    let newer = request(
        candidate("raid:world-events.spring-nest-1", 94)?,
        retry.now_ms,
    )?;
    assert_eq!(
        h.root
            .check_raid_schedules(&authority, &h.node, newer.clone())
            .await?
            .firings
            .len(),
        1
    );
    let lost = h
        .root
        .advance_raid_run(
            &h.node,
            fixture::id(91),
            req.channel,
            req.generation,
            selected.definition.clone(),
            retry.now_ms,
        )
        .await?
        .ok_or("retired hatch cleanup")?;
    assert!(matches!(lost.action, CommittedRaidAction::Lost));
    h.root
        .acknowledge_raid_effect(&h.node, lost.identity, vec![], retry.now_ms)
        .await?;
    let retained: i64 = sqlx::query_scalar("SELECT count(*) FROM game_world_raid_creatures WHERE firing_id=encode($1,'hex')::uuid AND state=1").bind(fixture::id(91).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        retained, 0,
        "retired hatch actors leave through LOST cleanup"
    );
    sqlx::query(
        "UPDATE game_world_reset_active SET reset_epoch=2 WHERE world_id=encode($1,'hex')::uuid",
    )
    .bind(world.as_bytes().as_slice())
    .execute(&h.pool)
    .await?;
    let mut reset = request(
        candidate("raid:world-events.spring-nest-1", 95)?,
        retry.now_ms + 1,
    )?;
    reset.reset_epoch = 2;
    assert!(
        h.root
            .check_raid_schedules(&authority, &h.node, reset.clone())
            .await?
            .firings
            .is_empty()
    );
    let mut tx = h.pool.begin().await?;
    let state = timed::poll(&mut tx, world, 2, EventKind::SpringIntoLife, reset.now_ms)
        .await?
        .ok_or("reset hatch state")?;
    assert_eq!(
        state.lifecycle.objects[&1].ready_at(),
        Some(reset.now_ms + 3_600_000)
    );
    assert_eq!(state.lifecycle.last_success_ms[&1], at + 3_600_000);
    tx.commit().await?;
    reset.now_ms += 3_600_000;
    reset.candidates = vec![candidate("raid:world-events.spring-nest-1", 96)?];
    // The reused raid schedule observes its normal one-minute reset cadence.
    assert!(
        h.root
            .check_raid_schedules(&authority, &h.node, reset.clone())
            .await?
            .firings
            .is_empty()
    );
    reset.now_ms += oteryn_gameplay_encounter_rules::raid::CHECK_INTERVAL_MS;
    assert_eq!(
        h.root
            .check_raid_schedules(&authority, &h.node, reset)
            .await?
            .firings
            .len(),
        1
    );

    Ok(())
}

async fn settle_spring_wave(
    h: &fixture::Harness,
    req: &RaidSchedulerCheck,
    selected: &RaidScheduleCandidate,
    id: u8,
    placed: bool,
) -> fixture::TestResult {
    let wave = h
        .root
        .advance_raid_run(
            &h.node,
            fixture::id(id),
            req.channel,
            req.generation,
            selected.definition.clone(),
            req.now_ms,
        )
        .await
        .map_err(|error| format!("Spring PostgreSQL line {}: {error:?}", line!()))?
        .ok_or("spring wave")?;
    let CommittedRaidAction::Wave { creatures, .. } = &wave.action else {
        return Err("spring wave kind".into());
    };
    let outcomes = creatures
        .iter()
        .map(|c| RaidPlacementOutcome {
            identity: c.identity,
            placed,
        })
        .collect();
    h.root
        .acknowledge_raid_effect(&h.node, wave.identity, outcomes, req.now_ms)
        .await
        .map_err(|error| format!("Spring PostgreSQL line {}: {error:?}", line!()))?;
    let end = h
        .root
        .advance_raid_run(
            &h.node,
            fixture::id(id),
            req.channel,
            req.generation,
            selected.definition.clone(),
            req.now_ms,
        )
        .await
        .map_err(|error| format!("Spring PostgreSQL line {}: {error:?}", line!()))?
        .ok_or("spring completion")?;
    h.root
        .acknowledge_raid_effect(&h.node, end.identity, vec![], req.now_ms)
        .await
        .map_err(|error| format!("Spring PostgreSQL line {}: {error:?}", line!()))?;
    Ok(())
}

async fn kingsday(h: &fixture::Harness) -> fixture::TestResult {
    let at = 1_934_078_400_000_i64;
    let world = fixture::scope()?.world_id();
    let epoch = (1..=4095)
        .find(|e| {
            crate::durability::world_event::changes::roll(world, *e, at)
                .is_ok_and(|d| d.timed.kingsday)
                && crate::durability::world_event::changes::roll(world, e + 1, at)
                    .is_ok_and(|d| !d.timed.kingsday)
        })
        .ok_or("selected celebration followed by inactive reset")?;
    for e in [epoch, epoch + 1] {
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2,$3,'kingsday-map',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp($4::double precision/1000),'2031-04')")
            .bind(world.as_bytes().as_slice()).bind(i64::try_from(e)?).bind([7u8;32].as_slice()).bind(at).execute(&h.pool).await?;
    }
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,$2)")
        .bind(world.as_bytes().as_slice())
        .bind(i64::try_from(epoch)?)
        .execute(&h.pool)
        .await?;
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let selected = candidate("raid:world-events.kingsday-arena", 97)?;
    let mut req = request(selected.clone(), at)?;
    req.reset_epoch = epoch;
    assert!(
        h.root
            .check_raid_schedules(&authority, &h.node, req.clone())
            .await?
            .firings
            .is_empty()
    );
    sqlx::query("INSERT INTO game_durability_fresh_admission_receipts SELECT decode('01'||repeat('00',32),'hex'),game_session_id,account_id,character_id,world_id,runtime_scope_channel_id,character_lease_generation,scope_ownership_generation,1,current_transport_ref,1,'{}',1 FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid ON CONFLICT DO NOTHING")
        .bind(fixture::id(fixture::SESSION).as_slice()).execute(&h.pool).await?;
    sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=2,control_loss_epoch=NULL,original_grace_deadline=NULL,predecessor_generation=NULL,fresh_replay_key=decode('01'||repeat('00',32),'hex') WHERE game_session_id=encode($1,'hex')::uuid")
        .bind(fixture::id(fixture::SESSION).as_slice()).execute(&h.pool).await?;
    let recovery = authority.record_for(&h.root).map_err(fixture::debug)?;
    for (id, input, expected) in [
        (111, Input::Reward, None),
        (112, Input::Attend, Some(true)),
        (113, Input::Attend, None),
    ] {
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
                TimedRequest {
                    world,
                    epoch,
                    kind: EventKind::Kingsday,
                    now_ms: at,
                    operation: fixture::id(id),
                    input
                }
            )
            .await?,
            expected
        );
        tx.commit().await?;
    }
    for (hour, firing) in [(1, 97), (2, 98)] {
        req.now_ms = at + hour * 3_600_000;
        req.candidates = vec![candidate("raid:world-events.kingsday-arena", firing)?];
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
        let wave = h
            .root
            .advance_raid_run(
                &h.node,
                fixture::id(firing),
                req.channel,
                req.generation,
                selected.definition.clone(),
                req.now_ms,
            )
            .await?
            .ok_or("hourly arena wave")?;
        let CommittedRaidAction::Wave { creatures, .. } = &wave.action else {
            return Err("arena wave kind".into());
        };
        assert_eq!(creatures.len(), 2);
        assert!(
            creatures
                .iter()
                .all(|c| c.timed_event && c.creature.as_str() == "oteryn:creature.orc_warlord")
        );
        h.root
            .acknowledge_raid_effect(
                &h.node,
                wave.identity,
                creatures
                    .iter()
                    .map(|c| RaidPlacementOutcome {
                        identity: c.identity,
                        placed: true,
                    })
                    .collect(),
                req.now_ms,
            )
            .await?;
        let end = h
            .root
            .advance_raid_run(
                &h.node,
                fixture::id(firing),
                req.channel,
                req.generation,
                selected.definition.clone(),
                req.now_ms,
            )
            .await?
            .ok_or("arena completion")?;
        h.root
            .acknowledge_raid_effect(&h.node, end.identity, vec![], req.now_ms)
            .await?;
    }
    let old = h
        .root
        .advance_raid_run(
            &h.node,
            fixture::id(97),
            req.channel,
            req.generation,
            selected.definition.clone(),
            req.now_ms,
        )
        .await?
        .ok_or("previous arena cleanup")?;
    assert!(matches!(old.action, CommittedRaidAction::Lost));
    h.root
        .acknowledge_raid_effect(&h.node, old.identity, vec![], req.now_ms)
        .await?;
    let mut tx = h.pool.begin().await?;
    let state = timed::poll(&mut tx, world, epoch, EventKind::Kingsday, req.now_ms)
        .await?
        .ok_or("celebration state")?;
    assert_eq!(state.phase, Phase::Victory);
    assert_eq!(state.wave, 2);
    tx.commit().await?;
    sqlx::query(
        "UPDATE game_world_reset_active SET reset_epoch=$2 WHERE world_id=encode($1,'hex')::uuid",
    )
    .bind(world.as_bytes().as_slice())
    .bind(i64::try_from(epoch + 1)?)
    .execute(&h.pool)
    .await?;
    req.reset_epoch = epoch + 1;
    req.candidates = vec![candidate("raid:world-events.kingsday-arena", 99)?];
    assert!(
        h.root
            .check_raid_schedules(&authority, &h.node, req.clone())
            .await?
            .firings
            .is_empty()
    );
    let mut tx = h.pool.begin().await?;
    assert!(
        timed::poll(&mut tx, world, epoch + 1, EventKind::Kingsday, req.now_ms)
            .await?
            .is_none()
    );
    tx.commit().await?;
    Ok(())
}
