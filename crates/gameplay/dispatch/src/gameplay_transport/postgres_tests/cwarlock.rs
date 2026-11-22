//! CWARLOCK: real production lever USE, native lethal queue, shared encounter tick, PG receipts.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::foundation::{
    ChannelRuntimeV1, CommandId, CommandRef, ExactActorRef, MovementLocalPosition,
    OwnerDamageCommand,
};
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority, UseCommand,
};
use oteryn_gameplay_encounters::bosses::phase_controller::court_warlock::{
    self as phases, WARLOCK,
};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::world_callbacks::{Position, court_warlock as source};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use std::sync::Arc;

#[test]
fn cwarlock_lever_admission_requires_wall_and_staging_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario(admin, 0)
}

#[test]
fn cwarlock_cooldown_refuses_second_real_lever_use_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario(admin, 1)
}

#[test]
fn cwarlock_waves_spawn_in_donor_order_and_time_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario(admin, 2)
}

#[test]
fn cwarlock_boss_death_teleports_out_after_sixty_seconds_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario(admin, 3)
}

#[test]
fn cwarlock_last_wave_sets_completion_storage_once_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario(admin, 4)
}

#[test]
fn cwarlock_stack_two_player_entry_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario(admin, 5)
}

#[test]
fn cwarlock_stack_two_player_exit_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario(admin, 6)
}

#[test]
fn cwarlock_stack_solo_exit_occupied_landing_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario(admin, 7)
}

#[test]
fn cwarlock_successor_keeps_paid_cooldown_before_publication_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario(admin, 8)
}

#[test]
fn cwarlock_successor_keeps_paid_cooldown_after_publication_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario(admin, 9)
}

fn scenario(_admin: String, mode: u8) -> TestResult {
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("cwarlock", move || {
        run(async |admin| {
            reset();
            let h = Harness::create_with_admission_seed(admin, "cwarlock", false, false).await?;
            // Keep the independent native source registered for genuine peer bootstrap.
            let _source = super::house_transfer_fixtures::RecoverySource::start(&h).await?;
            let fresh = prepare_current_session(&h).await?;
            let callback = oteryn_gameplay_quests::quest::world_callbacks::load()?
                .into_iter()
                .find(|c| c.key == source::KEY)
                .ok_or("production Court Warlock callback")?;
            let trigger = callback.selector.position.ok_or("lever position")?;
            let arena: Vec<_> = (33830..=33847)
                .flat_map(|x| (32133..=32150).map(move |y| Position { x, y, floor: -2 }))
                .chain([source::EXIT])
                .collect();
            let world = super::quest_world_callbacks_fixtures::world_with_cells(&callback, trigger, &arena)?;
            let mut deps = Dependencies::new()?;
            let (controller, activated) =
                super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                    deps.admitted.world_id,
                    "content-1",
                )?;
            let generation = controller.active().ok_or("active generation")?;
            let native = generation.native_gameplay().ok_or("native profiles")?;
            let cells = world
                .movement_cells(activated.qualified_room().movement_cells())
                .map_err(debug)?;
            let frame = world.frame_binding_digest(deps.admitted.world_id);
            let room = activated
                .qualified_room()
                .bundle_view(
                    cells.clone(),
                    frame,
                    world.map_revision_digest(),
                    crate::content::NativeEntryStart {
                        x: 33842,
                        y: 32146,
                        floor: 3,
                    },
                )
                .ok_or("bundle room")?;
            let pin = crate::content::channel_content_pin::ChannelContentPin::from_activation(
                deps.admitted.world_id,
                1,
                generation.identity().server_artifact_digest(),
                generation.identity().client_artifact_digest(),
                frame,
                world.map_revision_digest(),
                (33842, 32146, 3),
            );
            let mut runtime = ChannelRuntimeV1::from_committed_assignment(
                deps.admitted.world_id,
                deps.admitted.channel_id,
                crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
                1,
                1,
                1,
                "runtime-scope-assignment:1",
                256,
                pin,
            )
            .map_err(debug)?;
            let reservation = runtime
                .reserve_fresh_session(deps.admitted.game_session_id)
                .map_err(debug)?;
            deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime
                .initialize_pinned_test_position(
                    deps.actor,
                    MovementLocalPosition {
                        x: 33842,
                        y: 32146,
                        floor: 3,
                    },
                )
                .map_err(debug)?;
            native
                .install_companion_policies(&mut runtime)
                .map_err(debug)?;
            deps.runtime = tokio::sync::Mutex::new(runtime);
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(generation);
            owner.movement_cells = (&cells).into();
            owner.rooms = crate::gameplay_transport::SeamRooms::new(Some(&room), Some(&room));
            owner.quest_catalogue = Some(Arc::new(
                oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                    .map_err(debug)?
                    .catalogue()
                    .clone(),
            ));
            let mut admitted = deps.admitted;
            admitted.runtime_actor = Some(deps.actor);
            admitted.first_entry = FirstEntryOutcome::Positioned;
            let fence = owner
                .current_quest_fence(admitted.game_session_id)
                .await
                .map_err(|()| "fence")?
                .ok_or("fence")?;
            admitted.item_fence = Some(fence.item_fence());
            sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
            sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice()).bind(frame.as_slice()).execute(&h.pool).await?;
            // Bind native progression at root revision one, before starter or build writes.
            // The generic historical seed carries incompatible policy/evidence revisions.
            Box::pin(
                super::srx3_activity_source_fixture::prepare_player_with_operation(
                    &h,
                    &owner,
                    &mut admitted,
                    50,
                    "monk",
                    fresh.operation(),
                ),
            )
            .await?;
            assert!(
                owner
                    .progression_binding(admitted.game_session_id)
                    .is_some()
            );
            h.root
                .grant_starter_items(
                    &authority,
                    &h.node,
                    &admitted.item_fence.ok_or("native starter fence")?,
                    &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
                    &crate::durability::starter_grant::StarterTemplate {
                        definition: crate::durability::item_mint::TypedDefinitionRef {
                            family: "Item".into(),
                            production_key: "oteryn:item.tibia.i2854".into(),
                            revision_ref: "definition-r1".into(),
                        },
                        ..crate::durability::starter_grant::StarterTemplate::main()
                    },
                )
                .await
                .map_err(debug)?;
            owner.admit_quest_session(&admitted).await;
            assert!(
                owner
                    .refresh_source_party_presence(deps.actor, admitted.game_session_id)
                    .await
            );
            assert!(
                owner
                    .states
                    .quest_sessions
                    .lock()
                    .map_err(|_| "quest sessions")?
                    .get(&admitted.game_session_id)
                    .and_then(|s| s.copy.as_ref())
                    .and_then(|copy| copy.character_revision())
                    .is_some(),
                "admitted quest snapshot"
            );
            let key = oteryn_world_bundle::bundle::placement_key(-3, 33843, 32147, 1)
                .ok_or("lever key")?;
            let mut placement = world.base().digest().to_vec();
            placement.extend_from_slice(&key.to_be_bytes());
            // A real use without Marie-Denise's WALL stage cannot spend a cooldown.
            assert_eq!(
                use_lever(&owner, &admitted, &placement, 0, 1).await,
                UseDisposition::Rejected
            );
            let spent: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_character_boss_cooldowns")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(spent, 0);
            super::qengine_outfit_stories_fixtures::seed_quest_prerequisites(
                &h,
                &owner,
                &admitted,
                &[(source::TRACK.into(), 11)],
                0xc1,
            )
            .await?;
            owner
                .refresh_quest_session(admitted.game_session_id, false)
                .await;
            // Adjacent to the lever is insufficient: the user must stand on
            // one of the donor's five source staging squares.
            if mode == 0 {
                move_to(
                    &owner,
                    &admitted,
                    &cells,
                    world.facts(),
                    MovementLocalPosition {
                        x: 33843,
                        y: 32146,
                        floor: 3,
                    },
                )
                .await?;
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(1))
                    .ok_or("clock")?;
                assert_eq!(
                    use_lever(&owner, &admitted, &placement, 0, 3).await,
                    UseDisposition::Rejected
                );
                move_to(
                    &owner,
                    &admitted,
                    &cells,
                    world.facts(),
                    MovementLocalPosition {
                        x: 33842,
                        y: 32146,
                        floor: 3,
                    },
                )
                .await?;
            }
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(1))
                .ok_or("clock")?;
            // BossLever removes stale room monsters before starting a new run.
            {
                let mut runtime = owner.runtime.lock().await;
                let health = runtime
                    .companion_policy(phases::RAUBRITTERS[0])
                    .map_err(debug)?
                    .maximum_health;
                runtime
                    .commit_quest_creature_spawn(&[(
                        phases::RAUBRITTERS[0],
                        health,
                        MovementLocalPosition {
                            x: 33830,
                            y: 32133,
                            floor: 2,
                        },
                    )])
                    .map_err(debug)?;
            }
            let handoff = matches!(mode, 8 | 9);
            let mut peer = if mode == 5 || handoff {
                Some(
                    Box::pin(prepare_peer(
                        &h,
                        &owner,
                        &admitted,
                        MovementLocalPosition {
                            x: 33842,
                            y: 32145,
                            floor: 3,
                        },
                    ))
                    .await?,
                )
            } else {
                None
            };
            use std::sync::atomic::Ordering;
            let interrupted = mode == 8;
            if interrupted {
                owner.states.quest_callbacks.interrupt_source_boss_before_projection
                    .store(true, Ordering::Release);
            }
            let before = owner.owner_now().get();
            let outcome = use_lever(&owner, &admitted, &placement, 0, 4).await;
            let began = owner.owner_now().get();
            if interrupted {
                assert_eq!(outcome, UseDisposition::Rejected);
                assert!(!owner.states.quest_callbacks.interrupt_source_boss_before_projection
                    .load(Ordering::Acquire));
                let runtime = owner.runtime.lock().await;
                let peer = peer.as_ref().ok_or("staged peer")?.runtime_actor.ok_or("peer actor")?;
                for (actor, y) in [(deps.actor, 32146), (peer, 32145)] {
                    let at = runtime.read_actor_position(actor).map_err(debug)?.position();
                    assert_eq!((at.x, at.y, at.floor), (33842, y, 3));
                }
                assert_eq!(runtime.visible_entities().creatures.len(), 1, "staging did not remove the stale monster");
                assert!(owner.states.spell_states.lock().await.encounter_phases.active
                    .lock().map_err(|_| "phases")?.is_empty());
                assert_eq!(crate::map::view::MapFacts::object_revision(world.facts(), key), 0);
            } else {
                assert_eq!(outcome, UseDisposition::Committed);
                let runtime = owner.runtime.lock().await;
                let at = runtime
                    .read_actor_position(deps.actor)
                    .map_err(debug)?
                    .position();
                assert_eq!((at.x, at.y, at.floor), (33838, 32141, 2));
                assert_eq!(
                    runtime.visible_entities().creatures.len(),
                    0,
                    "initial three-second delay"
                );
            }
            if handoff {
                if !interrupted {
                    let runtime = owner.runtime.lock().await;
                    let peer = peer.as_ref().ok_or("published peer")?.runtime_actor.ok_or("peer actor")?;
                    let at = runtime.read_actor_position(peer).map_err(debug)?.position();
                    assert_eq!((at.x, at.y, at.floor), (33838, 32141, 2));
                    assert_eq!(owner.states.spell_states.lock().await.encounter_phases.active
                        .lock().map_err(|_| "phases")?.len(), 1);
                    assert_eq!(crate::map::view::MapFacts::object_revision(world.facts(), key), 1);
                }
                let mut party = Vec::new();
                for member in [&admitted, peer.as_ref().ok_or("handoff peer")?] {
                    let fence = owner.current_quest_fence(member.game_session_id)
                        .await.map_err(|()| "handoff party fence")?.ok_or("missing member fence")?;
                    party.push(fence.item_fence());
                }
                super::source_boss_handoff_fixtures::assert_paid_handoff(
                    &h, &deps, &owner, crate::durability::boss_encounter::SourceBossLever::CourtWarlock, party,
                ).await?;
                drop(owner);
                drop(authority);
                drop(seal);
                return h.cleanup().await;
            }
            let cooldown: i64 = sqlx::query_scalar("SELECT available_at-floor(extract(epoch FROM clock_timestamp())*1000)::bigint FROM game_character_boss_cooldowns WHERE boss_key=$1")
                .bind(source::BOSS).fetch_one(&h.pool).await?;
            assert!((71_990_000..=72_000_000).contains(&cooldown));
            if mode == 1 {
                move_to(
                    &owner,
                    &admitted,
                    &cells,
                    world.facts(),
                    MovementLocalPosition {
                        x: 33842,
                        y: 32146,
                        floor: 3,
                    },
                )
                .await?;
                tick(&owner, began).await;
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(1))
                    .ok_or("clock")?;
                let revision = crate::map::view::MapFacts::object_revision(world.facts(), key);
                assert!(
                    owner
                        .states
                        .spell_states
                        .lock()
                        .await
                        .encounter_phases
                        .active
                        .lock()
                        .map_err(|_| "phases")?
                        .is_empty(),
                    "the prior room is released before the cooldown check"
                );
                assert_eq!(
                    use_lever(&owner, &admitted, &placement, revision, 5).await,
                    UseDisposition::Rejected
                );
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(1))
                    .ok_or("clock")?;
                assert_eq!(
                    use_lever(&owner, &admitted, &placement, revision, 4).await,
                    UseDisposition::Rejected,
                    "an old command number with the later source revision cannot replay admission"
                );
            } else if mode >= 5 {
                if mode == 5 {
                    let runtime = owner.runtime.lock().await;
                    let at = runtime
                        .read_actor_position(
                            peer.as_ref()
                                .ok_or("entry peer")?
                                .runtime_actor
                                .ok_or("peer actor")?,
                        )
                        .map_err(debug)?
                        .position();
                    assert_eq!(
                        (at.x, at.y, at.floor),
                        (33838, 32141, 2),
                        "both participants land on the shared centre"
                    );
                    drop(runtime);
                    tick(&owner, began + 3_000_000).await;
                    assert_eq!(
                        owner
                            .runtime
                            .lock()
                            .await
                            .visible_entities()
                            .creatures
                            .len(),
                        7,
                        "party entry starts the first wave"
                    );
                } else {
                    let destination = if mode == 6 {
                        MovementLocalPosition {
                            x: 33839,
                            y: 32141,
                            floor: 2,
                        }
                    } else {
                        MovementLocalPosition {
                            x: i32::from(source::EXIT.x),
                            y: i32::from(source::EXIT.y),
                            floor: -i16::from(source::EXIT.floor),
                        }
                    };
                    peer = Some(Box::pin(prepare_peer(&h, &owner, &admitted, destination)).await?);
                    tick(&owner, began + 1_800_000_000).await;
                    let runtime = owner.runtime.lock().await;
                    for actor in [
                        deps.actor,
                        peer.as_ref()
                            .ok_or("exit peer")?
                            .runtime_actor
                            .ok_or("peer actor")?,
                    ] {
                        let at = runtime
                            .read_actor_position(actor)
                            .map_err(debug)?
                            .position();
                        assert_eq!(
                            (at.x, at.y, at.floor),
                            (
                                i32::from(source::EXIT.x),
                                i32::from(source::EXIT.y),
                                -i16::from(source::EXIT.floor)
                            ),
                            "all players reach the shared exit, including an existing bystander"
                        );
                    }
                    drop(runtime);
                    assert!(
                        owner
                            .states
                            .spell_states
                            .lock()
                            .await
                            .encounter_phases
                            .active
                            .lock()
                            .map_err(|_| "phases")?
                            .is_empty(),
                        "ejection retires the phase and releases the lever"
                    );
                }
            } else if mode >= 2 {
                tick(&owner, before + 2_999_999).await;
                assert_eq!(
                    owner
                        .runtime
                        .lock()
                        .await
                        .visible_entities()
                        .creatures
                        .len(),
                    0,
                    "no early first wave"
                );
                let mut command = 100;
                let waves = phases::waves();
                assert_eq!(waves.len(), 22, "21 Raubritter waves then Falcons");
                for (i, wave) in waves.iter().enumerate() {
                    tick(&owner, began + wave.second * 1_000_000).await;
                    let creatures = {
                        let runtime = owner.runtime.lock().await;
                        let creatures = runtime.visible_entities().creatures;
                        assert_eq!(
                            creatures.len(),
                            wave.count,
                            "source wave {i} at {} seconds",
                            wave.second
                        );
                        creatures
                            .into_iter()
                            .map(|c| {
                                let name = runtime
                                    .companion_snapshot(c.actor)
                                    .map_err(debug)?
                                    .state
                                    .policy
                                    .definition_key
                                    .clone();
                                assert!(
                                    wave.creatures.contains(&name.as_str()),
                                    "source wave {i}: {name}"
                                );
                                Ok((c.actor, name))
                            })
                            .collect::<TestResult<Vec<_>>>()?
                    };
                    assert_eq!(value(&h).await?, 11, "minions never complete the quest");
                    for (actor, name) in creatures {
                        command += 1;
                        kill(&owner, &admitted, actor, &name, command).await?;
                    }
                    assert_eq!(
                        owner.drain_kill_rewards(admitted.game_session_id).await,
                        crate::gameplay_transport::kill_reward::DrainOutcome::Drained
                    );
                }
                tick(&owner, began + 319_000_000).await;
                let boss = {
                    let runtime = owner.runtime.lock().await;
                    let creatures = runtime.visible_entities().creatures;
                    assert_eq!(creatures.len(), 1);
                    let boss = creatures[0].actor;
                    let state = runtime.companion_snapshot(boss).map_err(debug)?;
                    assert_eq!(state.state.policy.definition_key, WARLOCK);
                    assert_eq!(state.health, state.state.policy.maximum_health * 40 / 100);
                    boss
                };
                kill(&owner, &admitted, boss, WARLOCK, 999).await?;
                assert_eq!(
                    owner.drain_kill_rewards(admitted.game_session_id).await,
                    crate::gameplay_transport::kill_reward::DrainOutcome::Drained
                );
                assert_eq!(value(&h).await?, 12);
                tick(&owner, began + 320_000_000).await;
                tick(&owner, began + 379_999_999).await;
                assert_eq!(
                    owner
                        .runtime
                        .lock()
                        .await
                        .read_actor_position(deps.actor)
                        .map_err(debug)?
                        .position()
                        .floor,
                    2,
                    "full after-kill grace"
                );
                tick(&owner, began + 380_000_000).await;
                let at = owner
                    .runtime
                    .lock()
                    .await
                    .read_actor_position(deps.actor)
                    .map_err(debug)?
                    .position();
                assert_eq!(
                    (at.x, at.y, at.floor),
                    (33842, 32141, 3),
                    "source death exit"
                );
                assert_eq!(
                    owner.drain_kill_rewards(admitted.game_session_id).await,
                    crate::gameplay_transport::kill_reward::DrainOutcome::Drained
                );
                assert_eq!(value(&h).await?, 12);
                let receipts:i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-transition/order-of-the-stag/court-warlock-room-death'").fetch_one(&h.pool).await?;
                assert_eq!(
                    receipts, 1,
                    "duplicate lethal publication and tick never repeat storage"
                );
            }
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })?;
    assert!(started.elapsed().as_secs_f64() > 0.01);
    eprintln!(
        "CWARLOCK_PG_EXECUTED mode={mode} elapsed={:?}",
        started.elapsed()
    );
    Ok(())
}

async fn use_lever(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &crate::gameplay_transport::connection::AdmittedSession,
    placement: &[u8],
    revision: u64,
    command: u64,
) -> UseDisposition {
    let Some(actor) = admitted.runtime_actor else {
        return UseDisposition::Rejected;
    };
    owner
        .use_object(
            actor,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: command,
                item_fence: admitted.item_fence,
            },
            WorldObjectTarget {
                placement: placement.to_vec(),
                expected_revision: revision,
            },
        )
        .await
        .disposition
}
async fn tick(owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>, now: u64) {
    let mut runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    crate::gameplay_transport::encounter_phases::tick(&mut runtime, &states, now);
}
async fn move_to(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &crate::gameplay_transport::connection::AdmittedSession,
    cells: &crate::content::NativeEntryMovementCells,
    facts: &crate::map::facts::BundleFacts,
    to: MovementLocalPosition,
) -> TestResult {
    let mut runtime = owner.runtime.lock().await;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let expected = runtime.read_actor_position(actor).map_err(debug)?;
    let closed = crate::map::floor::BundleDoors(facts);
    let proof = crate::map::floor::prepare_quest_callback_teleport(
        &runtime,
        cells,
        actor,
        admitted.game_session_id,
        expected,
        to,
        false,
        &closed,
    )
    .map_err(debug)?;
    runtime
        .commit_quest_relocation(vec![proof], || true)
        .map_err(debug)?;
    Ok(())
}
async fn prepare_peer(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    primary: &crate::gameplay_transport::connection::AdmittedSession,
    position: MovementLocalPosition,
) -> TestResult<crate::gameplay_transport::connection::AdmittedSession> {
    use crate::gameplay_transport::connection::{
        AdmittedSession, ControllerBinding, SessionContinuity,
    };
    let now = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
        .fetch_one(&h.pool)
        .await?;
    let (current, operation) =
        crate::world_party_runtime_postgres_cases::committed_fresh_player_from_source_owned(
            &crate::world_party_runtime_postgres_cases::FreshPlayerOwners {
                root: h.root.clone(),
                node: h.node.clone(),
                recovery: &h.recovery,
            },
            60,
            63,
            171,
            now,
            Some("platform-test"),
        )
        .await
        .map_err(|error| format!("Court Warlock peer admission: {error}"))?;
    let fence = crate::gameplay_transport::item_fence_of(current).ok_or("peer fence")?;
    let actor = {
        let mut runtime = owner.runtime.lock().await;
        let reservation = runtime
            .reserve_fresh_session(fence.game_session_id)
            .map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .bind_attacker_lease(
                actor,
                fence.game_session_id,
                current.current_character_lease(),
            )
            .map_err(debug)?;
        runtime
            .initialize_pinned_test_position(actor, position)
            .map_err(debug)?;
        actor
    };
    let mut continuity = SessionContinuity::FRESH;
    continuity.selected_capabilities = primary.continuity.selected_capabilities;
    let mut peer = AdmittedSession {
        game_session_id: fence.game_session_id,
        world_id: primary.world_id,
        channel_id: primary.channel_id,
        runtime_actor: Some(actor),
        first_entry: FirstEntryOutcome::Positioned,
        controller: Some(ControllerBinding {
            transport: current.current_transport().ok_or("peer transport")?,
            account_id: id(63),
        }),
        continuity,
        item_fence: Some(fence),
    };
    Box::pin(
        super::srx3_activity_source_fixture::prepare_player_with_operation(
            h, owner, &mut peer, 50, "monk", &operation,
        ),
    )
    .await?;
    assert!(owner.progression_binding(peer.game_session_id).is_some());
    owner.admit_quest_session(&peer).await;
    super::qengine_outfit_stories_fixtures::seed_quest_prerequisites(
        h,
        owner,
        &peer,
        &[(source::TRACK.into(), 11)],
        0xc2,
    )
    .await?;
    owner
        .refresh_quest_session(peer.game_session_id, false)
        .await;
    assert!(
        owner
            .refresh_source_party_presence(actor, peer.game_session_id)
            .await
    );
    Ok(peer)
}

async fn value(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE track_key=$1),-1)",
    )
    .bind(source::TRACK)
    .fetch_one(&h.pool)
    .await?)
}

async fn kill(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &crate::gameplay_transport::connection::AdmittedSession,
    target: ExactActorRef,
    creature: &str,
    command: u64,
) -> TestResult {
    let queue = owner.states.attack.lock().await.kills().clone();
    let mut runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let lease = runtime
        .bound_attacker_lease(actor, admitted.game_session_id)
        .map_err(debug)?;
    let health = runtime.companion_snapshot(target).map_err(debug)?.health;
    let observed = owner.owner_now();
    let now = observed.get() / 1000;
    runtime
        .borrow_exact_actor_commit_at(now)
        .commit_damage_for_bound_attacker(
            target,
            actor,
            CommandRef::new(
                admitted.game_session_id,
                CommandId::new(command).map_err(debug)?,
            ),
            0,
            OwnerDamageCommand {
                target: creature.as_bytes(),
                occurrence: b"cwarlock",
                binding: b"cwarlock-wave-death",
                damage: health,
            },
        )
        .map_err(debug)?;
    for _ in 0..2 {
        crate::gameplay_transport::kill_reward::record_creature_kill(
            &mut runtime,
            crate::gameplay_transport::kill_reward::KillSink {
                prey: Some(&states),
                activity: &[],
                queue: &queue,
                rewards: owner.reward_table(),
                charms: None,
            },
            target,
            crate::combat::CapturedRewardPrincipal {
                character: lease.character_id(),
                lease_generation: lease.generation(),
                session: admitted.game_session_id,
                actor,
            },
            now,
            Some(observed),
        );
    }
    Ok(())
}

#[test]
fn cwarlock_donor_wave_clock_uses_twenty_two_and_forty_five_second_branches() {
    let waves = phases::waves();
    let seconds: Vec<_> = waves.iter().map(|w| w.second).collect();
    assert_eq!(
        seconds,
        [
            3, 25, 47, 48, 69, 91, 93, 113, 135, 138, 157, 179, 183, 201, 223, 228, 245, 267, 273,
            289, 311, 318
        ]
    );
    assert!(
        waves[..21]
            .iter()
            .all(|w| w.count == 7 && w.creatures == phases::RAUBRITTERS)
    );
    assert_eq!(waves[21].count, 22);
    assert_eq!(waves[21].creatures, phases::FALCONS);
}
