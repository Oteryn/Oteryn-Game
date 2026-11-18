//! Scheduler and literal USE coverage with bindings authored by the fixture.
use super::fixture::{Harness, debug, id, run};
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition, NodeId,
};
use oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin;
use oteryn_gameplay_quests::quest::world_callbacks::{
    Callback, Encounter, EncounterMonster, EncounterPlayer, Position, Trigger,
    periodic_bosses::{PeriodicBoss, WeightedBoss},
    reactive_waves::{Chance, EarthHit},
    waves::{Wave, WaveRun, WaveSpawn},
};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use std::time::Duration;

const TRIGGER: Position = Position {
    x: 33294,
    y: 31677,
    floor: -15,
};
const fn row(offset: u16) -> Position {
    Position {
        x: TRIGGER.x + offset,
        ..TRIGGER
    }
}
const START: Position = Position {
    x: TRIGGER.x - 1,
    ..TRIGGER
};
const PEER_START: Position = Position {
    y: START.y - 1,
    ..START
};
const LANDING: Position = row(2);
const PEER_LANDING: Position = Position {
    y: LANDING.y + 1,
    ..LANDING
};
const FIRST: Position = row(3);
const SECOND: Position = row(4);
const EXIT: Position = row(6);
const RAT: &str = "oteryn:creature.rat";
const CAVE_RAT: &str = "oteryn:creature.cave_rat";
const CREDIT: &str = "oteryn:quest-transition/wave-egress/fixture";
const TRACK: &str = "oteryn:quest-progress/fixture/wave-survived";

#[test]
fn periodic_boss_waits_for_interval_and_cooldown_and_keeps_one_live_actor() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("quest-periodic-boss", move || {
        scenario(admin, Case::Periodic)
    })
}

#[test]
fn quest_use_admits_two_staged_players_then_ejects_and_credits_both_survivors() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("quest-timed-waves", move || {
        scenario(admin, Case::Waves)
    })
}

#[test]
fn quest_step_earth_hit_updates_current_vitals_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("quest-earth-step", move || {
        scenario(admin, Case::EarthStep)
    })
}

#[derive(Clone, Copy, Debug)]
enum Case {
    Periodic,
    Waves,
    EarthStep,
}

fn scenario(admin: String, case: Case) -> TestResult {
    run(async move |_| {
        let started = std::time::Instant::now();
        let mut callback = super::authored_world_rewards::placeholder(TRIGGER)?;
        callback.key = "oteryn:quest-callback/fixture/quest-spawn".into();
        callback.storage.clear();
        callback.requires_world_admission = false;
        callback.effects.encounter = Some(Encounter {
            minimum_level: 0,
            room_min: LANDING,
            room_max: Position {
                x: SECOND.x,
                ..PEER_LANDING
            },
            players: vec![
                EncounterPlayer {
                    from: START,
                    to: LANDING,
                },
                EncounterPlayer {
                    from: PEER_START,
                    to: PEER_LANDING,
                },
            ],
            monsters: Vec::new(),
            timing: None,
            phase: None,
            // Crystal the_new_frontier/action_arena.lua:39-82. The PG fixture
            // uses shorter durations and two waves to exercise both deadlines.
            wave_run: Some(WaveRun {
                timeout_ms: 10_000,
                timeout_exit: EXIT,
                timeout_credit: Some(CREDIT.into()),
                victory: None,
                eject_room: false,
                stay_at_stations: false,
                hatch: None,
                allow_unstaged_activator: false,
                member_guard: Some(
                    oteryn_gameplay_quests::quest::reward_callbacks::TrackRequirement {
                        track: TRACK.into(),
                        minimum: 0,
                        maximum: 0,
                    },
                ),
                waves: [(1_000, FIRST, RAT), (3_000, SECOND, CAVE_RAT)]
                    .into_iter()
                    .map(|(at_ms, position, creature)| Wave {
                        at_ms,
                        spawn: WaveSpawn::Fixed {
                            monsters: vec![EncounterMonster {
                                position,
                                creature: creature.into(),
                            }],
                        },
                    })
                    .collect(),
            }),
        });
        if matches!(case, Case::EarthStep) {
            // Crystal movements_area_damage.lua:3-16. The fixture forces a
            // fixed roll so it can verify the real HP owner deterministically.
            callback.trigger = Trigger::StepIn;
            callback.selector.position = None;
            callback.selector.unique = None;
            callback.effects.encounter = None;
            callback.effects.earth_hit = Some(EarthHit {
                chance: Chance { wins: 1, rolls: 1 },
                minimum: 35,
                maximum: 35,
                effect: 51,
            });
        }
        // Use the real typed callback decode in addition to the composed PG fixture.
        let callback: Callback = serde_json::from_value(serde_json::to_value(callback)?)?;
        let world = super::quest_world_callbacks::world_with_cells(
            &callback,
            TRIGGER,
            &[FIRST, SECOND, EXIT, PEER_START, PEER_LANDING],
        )?;
        let h =
            Harness::create_with_admission_seed(admin, "quest_spawn_waves", false, false).await?;
        let admission = prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activated) =
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("content generation")?;
        let cells = world
            .movement_cells(activated.qualified_room().movement_cells())
            .map_err(debug)?;
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
            world.map_revision_digest(),
            (
                i32::from(START.x),
                i32::from(START.y),
                -i16::from(START.floor),
            ),
        );
        super::boss_activity::issue_content_and_reset(&h, &pin).await?;
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            16,
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        deps.admitted.runtime_actor = Some(deps.actor);
        runtime
            .initialize_pinned_test_position(deps.actor, local(START))
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.movement_cells = (&cells).into();
        if matches!(case, Case::Waves) {
            owner.quest_catalogue = Some(std::sync::Arc::new(credit_catalogue(&callback)?));
        } else if matches!(case, Case::EarthStep) {
            assert!(
                owner
                    .states
                    .quest_callbacks
                    .fixture
                    .set(vec![callback])
                    .is_ok()
            );
        } else {
            // Crystal globalevents_pits_of_inferno_bosses.lua:12-14,23. This
            // fixture uses a nonzero cooldown to prove the shared owner gate.
            let plan = PeriodicBoss {
                key: "oteryn:periodic-boss/fixture".into(),
                interval_ms: 1_000,
                cooldown_ms: 4_000,
                choices: [(FIRST, RAT, 1), (SECOND, CAVE_RAT, 3)]
                    .into_iter()
                    .map(|(position, creature, weight)| WeightedBoss {
                        weight,
                        spawn: EncounterMonster {
                            creature: creature.into(),
                            position,
                        },
                    })
                    .collect(),
            };
            assert!(
                owner
                    .states
                    .quest_callbacks
                    .periodic_boss_fixture
                    .set(vec![plan])
                    .is_ok()
            );
        }
        let mut admitted = deps.admitted;
        super::srx3_activity_source_fixture::prepare_player_with_operation(
            &h,
            &owner,
            &mut admitted,
            100,
            "knight",
            admission.operation(),
        )
        .await?;
        match case {
            Case::Waves => wave_scenario(&h, &owner, &mut admitted, deps.actor).await?,
            Case::Periodic => periodic_scenario(&owner).await?,
            Case::EarthStep => earth_step_scenario(&owner, &mut admitted, deps.actor).await?,
        }
        eprintln!(
            "POSTGRES_CASE_COMPLETED quest_spawn case={case:?} duration={:?}",
            started.elapsed()
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

fn credit_catalogue(
    callback: &Callback,
) -> TestResult<oteryn_gameplay_quests::quest::QuestStateCatalogue> {
    use oteryn_gameplay_quests::quest::{
        QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack,
        QuestTransition, authored_bindings::Bindings,
    };
    let base = QuestStateCatalogue::new(
        "content-1",
        vec![QuestTrack {
            key: TRACK.into(),
            quest: callback.quest.clone(),
            initial: 0,
            min: 0,
            max: 1,
        }],
        vec![QuestTransition {
            key: CREDIT.into(),
            quest: callback.quest.clone(),
            completes: false,
            experience: None,
            world_change: None,
            effects: vec![QuestEffect {
                track: TRACK.into(),
                from: QuestComparison::Between(0, 0),
                effect: QuestEffectKind::Set(1),
            }],
        }],
    )
    .map_err(debug)?;
    let document = serde_json::json!({"schema":1,"quests":[{
        "quest": callback.quest,
        "recipe_sha256":"2222222222222222222222222222222222222222222222222222222222222222",
        "sources":["Crystal 201189f8 the_new_frontier/action_arena.lua:39-82 and children_of_the_revolution/movements_click.lua:29-43"],
        "tracks":[{"key":TRACK,"initial":0,"minimum":0,"maximum":1}],
        "stages":[{"key":"waves","completes":false,"trigger":{"kind":"map_use","callback":callback}}]
    }]});
    let bindings = Bindings::parse(&serde_json::to_vec(&document)?)?;
    Ok(bindings.attach(&base)?)
}

async fn periodic_scenario(owner: &ComposedFreshAdmission<'_, '_, '_>) -> TestResult {
    tokio::time::pause();
    owner.drain_quest_encounter_deadlines().await;
    advance(owner, 999).await;
    assert!(creatures(owner).await?.is_empty());
    advance(owner, 1).await;
    let actors = creatures(owner).await?;
    let [(actor, at)] = actors.as_slice() else {
        return Err("the interval should produce exactly one boss".into());
    };
    assert!([local(FIRST), local(SECOND)].contains(at));
    advance(owner, 1_000).await;
    assert_eq!(
        creatures(owner).await?,
        actors,
        "no replacement while the boss lives"
    );
    owner
        .runtime
        .lock()
        .await
        .remove_encounter_actor(*actor)
        .map_err(debug)?;
    advance(owner, 1_000).await;
    assert!(
        creatures(owner).await?.is_empty(),
        "the next interval is inside cooldown"
    );
    advance(owner, 2_000).await;
    let respawn = creatures(owner).await?;
    assert_eq!(respawn.len(), 1);
    assert_ne!(respawn[0].0, *actor);
    advance(owner, 5_000).await;
    assert_eq!(
        creatures(owner).await?,
        respawn,
        "live identity holds after cooldown too"
    );
    tokio::time::resume();
    Ok(())
}

async fn wave_scenario(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    player: ExactActorRef,
) -> TestResult {
    assert_eq!(
        use_room(owner, admitted, 1).await?,
        UseDisposition::Rejected
    );
    assert_eq!(position(owner, player).await?, local(START));
    let peer = stage_peer(h, owner, admitted).await?;
    let peer_actor = peer.runtime_actor.ok_or("staged peer actor")?;
    let blocker = {
        let mut runtime = owner.runtime.lock().await;
        let session = GameSessionId::decode(&id(58)).map_err(debug)?;
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(actor, local(SECOND))
            .map_err(debug)?;
        actor
    };
    assert_eq!(
        use_room(owner, admitted, 2).await?,
        UseDisposition::Occupied
    );
    assert_eq!(position(owner, player).await?, local(START));
    assert_eq!(position(owner, peer_actor).await?, local(PEER_START));
    owner
        .runtime
        .lock()
        .await
        .remove_test_actor(blocker)
        .map_err(debug)?;

    let (release, released) = std::sync::mpsc::channel::<()>();
    let (ready, started) = tokio::sync::oneshot::channel();
    let blocking = tokio::task::spawn_blocking(move || {
        let _ = ready.send(());
        let _ = released.recv_timeout(Duration::from_secs(45));
    });
    started.await?;
    tokio::time::pause();
    let result = use_room(owner, admitted, 3).await;
    drop(release);
    blocking.await?;
    assert_eq!(result?, UseDisposition::Committed);
    assert_eq!(position(owner, player).await?, local(LANDING));
    assert_eq!(position(owner, peer_actor).await?, local(PEER_LANDING));
    advance(owner, 999).await;
    assert!(creatures(owner).await?.is_empty());
    advance(owner, 1).await;
    let first = creatures(owner).await?;
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].1, local(FIRST));
    advance(owner, 2_000).await;
    let both = creatures(owner).await?;
    assert_eq!(both.len(), 2);
    assert!(both.iter().any(|(_, position)| *position == local(SECOND)));
    advance(owner, 7_000).await;
    assert_eq!(position(owner, player).await?, local(EXIT));
    assert_eq!(position(owner, peer_actor).await?, local(EXIT));
    assert!(creatures(owner).await?.is_empty());
    tokio::time::resume();

    let primary = admitted
        .item_fence
        .ok_or("primary item fence")?
        .character_id;
    let partner = peer.item_fence.ok_or("peer item fence")?.character_id;
    assert_ne!(primary, partner);
    owner.drain_quest_encounter_deadlines().await;
    owner.drain_quest_encounter_deadlines().await;
    let read_credit = async |character| {
        owner
            .root
            .read_character_quest_state(owner.character, character)
            .await
            .map_err(debug)
    };
    let primary_after = read_credit(primary).await?;
    let partner_after = read_credit(partner).await?;
    assert_eq!(primary_after.tracks().get(TRACK), Some(&1));
    assert_eq!(partner_after.tracks().get(TRACK), Some(&1));
    owner.drain_quest_encounter_deadlines().await;
    assert_eq!(read_credit(primary).await?, primary_after);
    assert_eq!(read_credit(partner).await?, partner_after);
    Ok(())
}

async fn stage_peer(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    leader: &AdmittedSession,
) -> TestResult<AdmittedSession> {
    use crate::gameplay_transport::connection::{
        ControllerBinding, FirstEntryOutcome, FreshAdmissionAuthority, SessionContinuity,
    };
    use crate::world_party_runtime_postgres_cases::{
        FreshPlayerOwners, committed_fresh_player_with_operation,
    };
    use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};

    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let (current, operation) = committed_fresh_player_with_operation(
        &FreshPlayerOwners {
            root: h.root.clone(),
            node: h.node.clone(),
            recovery: &h.recovery,
        },
        60,
        63,
        171,
        now,
    )
    .await
    .map_err(debug)?;
    let fence = crate::gameplay_transport::item_fence_of(current).ok_or("peer item fence")?;
    let actor = {
        let mut runtime = owner.runtime.lock().await;
        let reservation = runtime
            .reserve_fresh_session(fence.game_session_id)
            .map_err(debug)?;
        runtime.commit_fresh_session(reservation).map_err(debug)?
    };
    let mut peer = AdmittedSession {
        game_session_id: fence.game_session_id,
        world_id: leader.world_id,
        channel_id: leader.channel_id,
        runtime_actor: Some(actor),
        first_entry: FirstEntryOutcome::Positioned,
        controller: Some(ControllerBinding {
            transport: current.current_transport().ok_or("peer transport")?,
            account_id: id(63),
        }),
        continuity: SessionContinuity::FRESH,
        item_fence: Some(fence),
    };
    super::srx3_activity_source_fixture::prepare_player_with_operation(
        h, owner, &mut peer, 100, "knight", &operation,
    )
    .await?;
    let (step, _) = owner
        .paced_step(actor, peer.game_session_id, StepDirection::North)
        .await;
    assert_eq!(step.disposition, StepDisposition::Moved);
    assert_eq!(position(owner, actor).await?, local(PEER_START));
    Ok(peer)
}

async fn earth_step_scenario(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    player: ExactActorRef,
) -> TestResult {
    use oteryn_protocol_oteryn::world_spatial::{
        StepDirection, StepDisposition, decode_step_result, encode_step_intent,
    };
    let health = async |session| -> TestResult<_> {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        Ok(states
            .get(&runtime, player, session)
            .ok_or("vitals")?
            .vitals()
            .health)
    };
    let before = health(admitted.game_session_id).await?;
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        1,
        1,
        encode_step_intent(StepDirection::East),
    )
    .await?;
    assert_eq!(
        decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    assert_eq!(position(owner, player).await?, local(TRIGGER));
    assert_eq!(before - health(admitted.game_session_id).await?, 35);
    owner.drain_quest_encounter_deadlines().await;
    assert_eq!(before - health(admitted.game_session_id).await?, 35);
    Ok(())
}

async fn use_room(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    id: u64,
) -> TestResult<UseDisposition> {
    use oteryn_protocol_oteryn::world_object::{decode_use_result, encode_use_intent};
    let key = oteryn_world_bundle::bundle::placement_key(TRIGGER.floor, TRIGGER.x, TRIGGER.y, 1)
        .ok_or("fixture USE placement")?;
    let map = owner.bundle_map().ok_or("fixture map")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = encode_use_intent(&WorldObjectTarget {
        placement,
        expected_revision: 0,
    })
    .map_err(debug)?;
    let reply = super::quest_world_callbacks::wire(owner, admitted, id, 2, payload).await?;
    decode_use_result(&reply).map_err(debug)
}

async fn advance(owner: &ComposedFreshAdmission<'_, '_, '_>, millis: u64) {
    tokio::time::advance(Duration::from_millis(millis)).await;
    owner.drain_quest_encounter_deadlines().await;
}

async fn creatures(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
) -> TestResult<Vec<(ExactActorRef, MovementLocalPosition)>> {
    let runtime = owner.runtime.lock().await;
    let mut actors = Vec::new();
    for actor in runtime.current_live_creature_census().map_err(debug)? {
        actors.push((
            actor,
            runtime
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
        ));
    }
    Ok(actors)
}

async fn position(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
) -> TestResult<MovementLocalPosition> {
    Ok(owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position())
}

fn local(position: Position) -> MovementLocalPosition {
    MovementLocalPosition {
        x: i32::from(position.x),
        y: i32::from(position.y),
        floor: -i16::from(position.floor),
    }
}
