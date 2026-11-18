//! The shipped Ghulosh bookstand starts the encounter through native USE.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority, UseCommand,
};
use crate::map::view::EntryFacts;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use std::sync::Arc;

const START: MovementLocalPosition = MovementLocalPosition {
    x: 32748,
    y: 32773,
    floor: 10,
};
const ENTRY: MovementLocalPosition = MovementLocalPosition {
    x: 32757,
    y: 32727,
    floor: 10,
};
const BOSS: &str = "oteryn:creature.ghulosh";
const DEATHGAZE: &str = "oteryn:creature.ghulosh_deathgaze";
const BOOK: &str = "oteryn:creature.the_book_of_death";
const EXIT: MovementLocalPosition = MovementLocalPosition {
    x: 32664,
    y: 32711,
    floor: 13,
};
const DEATH: &str = "oteryn:creature.concentrated_death";

#[test]
fn seclib2_ghulosh_party_use_transforms_respawns_book_and_exits_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_ghulosh(&admin, false)
}

#[test]
fn seclib2_ghulosh_party_timeout_and_cooldown_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_ghulosh(&admin, true)
}

fn run_ghulosh(admin: &str, expire: bool) -> TestResult {
    if admin.is_empty() {
        return Err("PostgreSQL test URL is empty".into());
    }
    super::kill_reward_e2e::scenario_thread::run("seclib2-ghulosh", move || {
        run(async |admin| {
            reset();
            let h =
                Harness::create_with_admission_seed(admin, "seclib2_ghulosh", true, false).await?;
            prepare_current_session(&h).await?;
            let mut deps = Dependencies::new()?;
            let (controller, activated) =
                super::death_ml_loss_composed::production_generation_with_pin(
                    deps.admitted.world_id,
                    "content-1",
                )?;
            let generation = controller.active().ok_or("active generation")?;
            let native = generation.native_gameplay().ok_or("native gameplay")?;
            use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_scene};
            let world = native_scene(
                native,
                vec![
                    NativeCrop {
                        bytes: include_bytes!(concat!(
                            env!("CARGO_MANIFEST_DIR"),
                            "/../../../content/world/placements/region-z10-x127-y128.b3"
                        )),
                        region: (10, 127, 128),
                        bounds: (32746, 32772, 32752, 32774),
                        start: (32748, 32773),
                        expected: vec![ExpectedTile {
                            position: (32746, 32773),
                            ids: vec![28285, 28889, 28888],
                            counted_entry: None,
                        }],
                    },
                    NativeCrop {
                        bytes: include_bytes!(concat!(
                            env!("CARGO_MANIFEST_DIR"),
                            "/../../../content/world/placements/region-z10-x127-y127.b3"
                        )),
                        region: (10, 127, 127),
                        bounds: (32748, 32713, 32764, 32730),
                        start: (32757, 32727),
                        expected: vec![ExpectedTile {
                            position: (32755, 32729),
                            ids: vec![28285, 1949],
                            counted_entry: None,
                        }],
                    },
                    NativeCrop {
                        bytes: include_bytes!(concat!(
                            env!("CARGO_MANIFEST_DIR"),
                            "/../../../content/world/placements/region-z13-x127-y127.b3"
                        )),
                        region: (13, 127, 127),
                        bounds: (32660, 32711, 32666, 32713),
                        start: (32664, 32711),
                        expected: vec![ExpectedTile {
                            position: (32664, 32711),
                            ids: vec![11421],
                            counted_entry: None,
                        }],
                    },
                ],
                &[],
            )?;
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
                        x: START.x,
                        y: START.y,
                        floor: START.floor,
                    },
                )
                .ok_or("bundle combat view")?;
            let pin = ChannelContentPin::from_activation(
                deps.admitted.world_id,
                activated.activation_sequence(),
                generation.identity().server_artifact_digest(),
                generation.identity().client_artifact_digest(),
                frame,
                world.map_revision_digest(),
                (START.x, START.y, START.floor),
            );
            let world_id = deps.admitted.world_id;
            sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,'map-1',$3,$4,$5,'TIBIA_SERVER_SAVE_BERLIN_06',1,clock_timestamp(),to_char((clock_timestamp() AT TIME ZONE 'Europe/Berlin')-interval '6 hours','YYYY-MM')) ON CONFLICT DO NOTHING")
                .bind(world_id.as_bytes().as_slice())
                .bind(world.map_revision_digest().as_slice())
                .bind(generation.identity().server_artifact_digest().as_slice())
                .bind(generation.identity().client_artifact_digest().as_slice())
                .bind(frame.as_slice()).execute(&h.pool).await?;
            sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1) ON CONFLICT DO NOTHING")
                .bind(world_id.as_bytes().as_slice()).execute(&h.pool).await?;
            let mut runtime = ChannelRuntimeV1::from_committed_assignment(
                deps.admitted.world_id,
                deps.admitted.channel_id,
                crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
                1,
                1,
                1,
                "runtime-scope-assignment:1",
                8,
                pin,
            )
            .map_err(debug)?;
            let session = deps.admitted.game_session_id;
            let slot = runtime.reserve_fresh_session(session).map_err(debug)?;
            deps.actor = runtime.commit_fresh_session(slot).map_err(debug)?;
            runtime
                .initialize_pinned_test_position(deps.actor, START)
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
                .current_quest_fence(session)
                .await
                .map_err(debug)?
                .ok_or("quest fence")?;
            admitted.item_fence = Some(fence.item_fence());
            sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
                .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
            sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
                .bind(id(42).as_slice()).bind(id(43).as_slice())
                .bind(generation.identity().server_artifact_digest().as_slice())
                .bind(generation.identity().client_artifact_digest().as_slice())
                .bind(frame.as_slice()).execute(&h.pool).await?;
            deps.runtime
                .lock()
                .await
                .bind_attacker_lease(
                    deps.actor,
                    session,
                    crate::foundation::CharacterLease::new(
                        crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                            .map_err(debug)?,
                        fence.character_lease_generation,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
            owner.admit_progression_session(&mut admitted).await;
            h.seed_pvp_target().await?;
            super::appearance_change_postgres::observer_visibility_tests::admit(&h).await?;
            let peer_session = crate::foundation::GameSessionId::decode(&id(110)).map_err(debug)?;
            let peer_fence = owner
                .current_quest_fence(peer_session)
                .await
                .map_err(debug)?
                .ok_or("peer quest fence")?;
            let peer = {
                let mut runtime = deps.runtime.lock().await;
                let slot = runtime.reserve_fresh_session(peer_session).map_err(debug)?;
                let peer = runtime.commit_fresh_session(slot).map_err(debug)?;
                runtime
                    .initialize_pinned_test_position(
                        peer,
                        MovementLocalPosition {
                            x: START.x + 1,
                            ..START
                        },
                    )
                    .map_err(debug)?;
                runtime
                    .bind_attacker_lease(
                        peer,
                        peer_session,
                        crate::foundation::CharacterLease::new(
                            crate::foundation::CharacterId::decode(
                                peer_fence.character_id.as_bytes(),
                            )
                            .map_err(debug)?,
                            peer_fence.character_lease_generation,
                        )
                        .map_err(debug)?,
                    )
                    .map_err(debug)?;
                let mut spells = owner.states.spell_states.lock().await;
                for (actor, session) in [(deps.actor, session), (peer, peer_session)] {
                    assert!(
                        spells
                            .initialize(
                                &runtime,
                                actor,
                                session,
                                crate::spell::cast::CharacterCastFacts {
                                    level: 250,
                                    ..crate::gameplay_transport::actor_spell::tests::FACTS
                                },
                                (0, 0),
                                owner.owner_now()
                            )
                            .is_some()
                    );
                }
                peer
            };
            let mut peer_admission = super::AdmittedSession {
                game_session_id: peer_session,
                runtime_actor: Some(peer),
                first_entry: FirstEntryOutcome::Positioned,
                controller: None,
                continuity: crate::gameplay_transport::connection::SessionContinuity::FRESH,
                item_fence: Some(peer_fence.item_fence()),
                ..admitted
            };
            owner.admit_progression_session(&mut peer_admission).await;
            owner.admit_quest_session(&peer_admission).await;
            owner.admit_quest_session(&admitted).await;

            assert!(
                owner
                    .refresh_source_party_presence(deps.actor, session)
                    .await
            );
            assert!(
                owner
                    .refresh_source_party_presence(peer, peer_session)
                    .await
            );
            let placement = crate::map::view::placement_key(
                crate::map::overlay::TilePos {
                    x: 32746,
                    y: 32773,
                    floor: -10,
                },
                2,
            )
            .ok_or("bookstand placement")?;
            let admitted_after: i64 = sqlx::query_scalar(
                "SELECT floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint",
            )
            .fetch_one(&h.pool)
            .await?;
            let outcome = owner
                .use_object(
                    deps.actor,
                    UseCommand {
                        game_session_id: session,
                        command_id: 1,
                        item_fence: admitted.item_fence,
                    },
                    WorldObjectTarget {
                        placement: world
                            .base()
                            .digest()
                            .into_iter()
                            .chain(placement.to_be_bytes())
                            .collect(),
                        expected_revision: 0,
                    },
                )
                .await;
            assert_eq!(outcome.disposition, UseDisposition::Committed);
            let cooldowns: Vec<(String, i64)> = sqlx::query_as(
                "SELECT admission::text, available_at FROM game_character_boss_cooldowns
                 WHERE boss_key=$1 AND character_id IN (encode($2,'hex')::uuid,encode($3,'hex')::uuid)",
            ).bind(BOSS).bind(fence.character_id.as_bytes().as_slice())
                .bind(peer_fence.character_id.as_bytes().as_slice()).fetch_all(&h.pool).await?;
            assert_eq!(cooldowns.len(), 2);
            assert_eq!(cooldowns[0], cooldowns[1]);
            assert!(cooldowns[0].1 >= admitted_after + 72_000_000);
            for actor in [deps.actor, peer] {
                assert_eq!(
                    owner
                        .runtime
                        .lock()
                        .await
                        .read_actor_position(actor)
                        .map_err(debug)?
                        .position(),
                    ENTRY
                );
            }
            {
                let mut runtime = owner.runtime.lock().await;
                let spells = owner.states.spell_states.lock().await;
                let boss = exact(&runtime, BOSS)?;
                let snapshot = runtime.companion_snapshot(boss).map_err(debug)?;
                let hp = snapshot.maximum_health * 3 / 4;
                runtime.set_test_creature_health(boss, hp).map_err(debug)?;
                crate::gameplay_transport::encounter_phases::tick(&mut runtime, &spells, 1000);
                assert_eq!(exact(&runtime, DEATHGAZE)?, boss);
                assert_eq!(runtime.companion_snapshot(boss).map_err(debug)?.health, hp);
            }
            kill(&owner, admitted, BOOK, 100, 2000).await?;
            kill(&owner, admitted, DEATH, 101, 3000).await?;
            {
                let mut runtime = owner.runtime.lock().await;
                let spells = owner.states.spell_states.lock().await;
                for now in [6999, 7000, 9000, 11000, 13000, 14999] {
                    crate::gameplay_transport::encounter_phases::tick(&mut runtime, &spells, now);
                    assert!(exact(&runtime, BOOK).is_err(), "book respawned at {now}");
                }
                crate::gameplay_transport::encounter_phases::tick(&mut runtime, &spells, 15000);
                exact(&runtime, BOOK)?;
            }
            if expire {
                owner.clock_origin -= std::time::Duration::from_secs(1190);
                owner.drain_quest_encounter_deadlines().await;
                {
                    let runtime = owner.runtime.lock().await;
                    for actor in [deps.actor, peer] {
                        assert_eq!(
                            runtime
                                .read_actor_position(actor)
                                .map_err(debug)?
                                .position(),
                            ENTRY
                        );
                    }
                    exact(&runtime, BOOK)?;
                }
                owner.clock_origin -= std::time::Duration::from_secs(11);
                owner.drain_quest_encounter_deadlines().await;
                let map = owner.bundle_map().ok_or("map")?;
                {
                    let mut runtime = owner.runtime.lock().await;
                    for actor in [deps.actor, peer] {
                        assert_eq!(
                            runtime
                                .read_actor_position(actor)
                                .map_err(debug)?
                                .position(),
                            MovementLocalPosition {
                                x: 32660,
                                y: 32713,
                                floor: 13
                            }
                        );
                    }
                    for creature in [BOSS, DEATHGAZE, BOOK, DEATH, "oteryn:creature.bone_jaw"] {
                        assert!(
                            exact(&runtime, creature).is_err(),
                            "{creature} survived the room deadline"
                        );
                    }
                    let closed = crate::map::floor::BundleDoors(&map.facts);
                    for (actor, session, x) in [
                        (deps.actor, session, START.x),
                        (peer, peer_session, START.x + 1),
                    ] {
                        let at = runtime.read_actor_position(actor).map_err(debug)?;
                        let proof = crate::map::floor::prepare_quest_callback_teleport(
                            &runtime,
                            &owner.movement_cells,
                            actor,
                            session,
                            at,
                            MovementLocalPosition { x, ..START },
                            false,
                            &closed,
                        )
                        .map_err(debug)?;
                        runtime.commit_source_step(proof).map_err(debug)?;
                    }
                }
                // Only the peer is still cooling down, so the eligible initiator
                // must remain outside and must not receive a new cooldown.
                let changed = sqlx::query(
                    "UPDATE game_character_boss_cooldowns SET available_at=0
                     WHERE character_id=encode($1,'hex')::uuid AND boss_key=$2",
                )
                .bind(fence.character_id.as_bytes().as_slice())
                .bind(BOSS)
                .execute(&h.pool)
                .await?;
                assert_eq!(changed.rows_affected(), 1);
                let outcome = owner
                    .use_object(
                        deps.actor,
                        UseCommand {
                            game_session_id: session,
                            command_id: 2,
                            item_fence: admitted.item_fence,
                        },
                        WorldObjectTarget {
                            placement: world
                                .base()
                                .digest()
                                .into_iter()
                                .chain(placement.to_be_bytes())
                                .collect(),
                            expected_revision: map.facts.object_revision(placement),
                        },
                    )
                    .await;
                assert_eq!(outcome.disposition, UseDisposition::RequirementNotMet);
                let runtime = owner.runtime.lock().await;
                for (actor, x) in [(deps.actor, START.x), (peer, START.x + 1)] {
                    assert_eq!(
                        runtime
                            .read_actor_position(actor)
                            .map_err(debug)?
                            .position(),
                        MovementLocalPosition { x, ..START }
                    );
                }
                assert!(exact(&runtime, BOSS).is_err());
                let remaining: Vec<i64> = sqlx::query_scalar(
                    "SELECT available_at FROM game_character_boss_cooldowns WHERE boss_key=$1 ORDER BY available_at",
                ).bind(BOSS).fetch_all(&h.pool).await?;
                assert_eq!(remaining, [0, cooldowns[0].1]);
            } else {
                for (actor, session) in [(deps.actor, session), (peer, peer_session)] {
                    {
                        let mut runtime = owner.runtime.lock().await;
                        let at = runtime.read_actor_position(actor).map_err(debug)?;
                        let map = owner.bundle_map().ok_or("map")?;
                        let closed = crate::map::floor::BundleDoors(&map.facts);
                        let proof = crate::map::floor::prepare_quest_callback_teleport(
                            &runtime,
                            &owner.movement_cells,
                            actor,
                            session,
                            at,
                            MovementLocalPosition {
                                x: 32755,
                                y: 32728,
                                floor: 10,
                            },
                            false,
                            &closed,
                        )
                        .map_err(debug)?;
                        runtime.commit_source_step(proof).map_err(debug)?;
                    }
                    let (step, _) = owner
                        .ordinary_paced_step(
                            actor,
                            session,
                            oteryn_protocol_oteryn::world_spatial::StepDirection::South,
                            None,
                            None,
                        )
                        .await;
                    assert_eq!(
                        step.disposition,
                        oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved
                    );
                    assert_eq!(
                        owner
                            .runtime
                            .lock()
                            .await
                            .read_actor_position(actor)
                            .map_err(debug)?
                            .position(),
                        EXIT
                    );
                    if actor == deps.actor {
                        let (step, _) = owner
                            .ordinary_paced_step(
                                actor,
                                session,
                                oteryn_protocol_oteryn::world_spatial::StepDirection::East,
                                None,
                                None,
                            )
                            .await;
                        assert_eq!(
                            step.disposition,
                            oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved
                        );
                    }
                }
            }
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}

fn exact(
    runtime: &ChannelRuntimeV1,
    creature: &str,
) -> TestResult<crate::foundation::ExactActorRef> {
    runtime
        .current_live_creature_census()
        .map_err(debug)?
        .into_iter()
        .find(|actor| runtime.matches_live_creature_identity(*actor, creature.as_bytes()))
        .ok_or_else(|| format!("expected {creature}").into())
}

async fn kill(
    owner: &super::ComposedFreshAdmission<'_, '_, '_>,
    admitted: super::AdmittedSession,
    creature: &str,
    command: u64,
    now_ms: u64,
) -> TestResult {
    use crate::foundation::{CommandId, CommandRef, OwnerDamageCommand};
    let queue = owner.states.attack.lock().await.kills().clone();
    let mut runtime = owner.runtime.lock().await;
    let spells = owner.states.spell_states.lock().await;
    let actor = admitted.runtime_actor.ok_or("player")?;
    let target = exact(&runtime, creature)?;
    let lease = runtime
        .bound_attacker_lease(actor, admitted.game_session_id)
        .map_err(debug)?;
    let health = runtime.companion_snapshot(target).map_err(debug)?.health;
    runtime
        .borrow_exact_actor_commit_at(now_ms)
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
                occurrence: b"seclib2-ghulosh",
                binding: b"seclib2-library-death",
                damage: health,
            },
        )
        .map_err(debug)?;
    crate::gameplay_transport::kill_reward::record_creature_kill(
        &mut runtime,
        crate::gameplay_transport::kill_reward::KillSink {
            prey: Some(&spells),
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
        now_ms,
        None,
    );
    Ok(())
}
