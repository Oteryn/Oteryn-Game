//! Real callback admission, all Cults native phases, and the existing durable quest writer.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::foundation::{
    ChannelRuntimeV1, CommandId, CommandRef, ExactActorRef, MovementLocalPosition,
    OwnerDamageCommand,
};
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority, UseCommand,
};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};

use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use std::sync::Arc;

const SCAR_BOSS: &str = "oteryn:creature.alptramun";
const SCAR_COUNT: &str = "oteryn:quest-progress/dcourts2/dream-scar/count";
const SCAR_GATE: &str = "oteryn:quest-progress/dcourts2/dream-scar/entry";
const SCAR_TIMER: &str = "oteryn:quest-progress/dcourts2/dream-scar/alptramun-timer";

const COUNT: &str = "oteryn:quest-progress/quest/u11_40/cults_of_tibia/humans/mission";

fn callback() -> TestResult<Callback> {
    let mut callback = oteryn_gameplay_quests::quest::world_callbacks::load()?
        .into_iter()
        .find(|c| c.key == "oteryn:quest-callback/engenc2/cults-essence")
        .ok_or("production Cults callback")?;
    // Translate the actual source roster into the fixture's qualified map frame.
    callback.selector.position = Some(Position {
        x: 100,
        y: 100,
        floor: -7,
    });
    callback.selector.appearance = Some(2772);
    callback.lever = Some([2772, 2773]);
    let encounter = callback
        .effects
        .encounter
        .as_mut()
        .ok_or("Cults encounter")?;
    for (i, player) in encounter.players.iter_mut().enumerate() {
        player.from = Position {
            x: 100,
            y: 100 + u16::try_from(i)?,
            floor: -7,
        };
        player.to = Position {
            x: 116,
            y: 117,
            floor: -7,
        };
    }
    for monster in &mut encounter.monsters {
        monster.position = Position {
            x: monster.position.x + 116 - 33098,
            y: monster.position.y + 116 - 31920,
            floor: -7,
        };
    }
    encounter.room_min = Position {
        x: 106,
        y: 106,
        floor: -7,
    };
    encounter.room_max = Position {
        x: 126,
        y: 126,
        floor: -7,
    };
    Ok(callback)
}

#[test]
fn engenc2_cults_callback_native_phases_and_settlement_postgres() -> TestResult {
    run_encounter(EncounterCase::Cults { reenter: false })
}

#[test]
fn qmrevfix_cults_second_lever_after_victory_postgres() -> TestResult {
    run_encounter(EncounterCase::Cults { reenter: true })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EncounterCase {
    Cults { reenter: bool },
    DreamScar,
    Faceless,
}

#[test]
fn dcourts2_faceless_steps_open_boss_damage_gate_postgres() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_encounter(EncounterCase::Faceless)
}

#[test]
fn dcourts2_dream_scar_uses_startup_roster_on_lever_use_postgres() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_encounter(EncounterCase::DreamScar)
}

fn run_encounter(case: EncounterCase) -> TestResult {
    let dream_scar = case == EncounterCase::DreamScar;
    let faceless = case == EncounterCase::Faceless;
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let began = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("engenc2-cults", move || {
        run(async |admin| {
            reset();
            let h =
                Harness::create_with_admission_seed(admin, "engenc2_cults", true, false).await?;
            prepare_current_session(&h).await?;
            let mut callback = callback()?;
            if dream_scar {
                callback.key = "oteryn:quest-callback/dcourts2/dream-scar".into();
                callback.quest = "oteryn:quest.the_dream_courts_quest".into();
                callback.reversible = false;
                callback.lever = None;
                let encounter = callback.effects.encounter.as_mut().ok_or("encounter")?;
                let position = encounter
                    .monsters
                    .first()
                    .ok_or("monster position")?
                    .position;
                encounter.phase = None;
                encounter.minimum_level = 250;
                encounter.weekday_monsters = Some(
                    [
                        "alptramun",
                        "izcandar_the_banished",
                        "malofur_mangrinder",
                        "maxxenius",
                        "izcandar_the_banished",
                        "plagueroot",
                        "maxxenius",
                    ]
                    .map(|boss| {
                        vec![
                            oteryn_gameplay_quests::quest::world_callbacks::EncounterMonster {
                                position,
                                creature: format!("oteryn:creature.{boss}"),
                            },
                        ]
                    }),
                );
                encounter.monsters.clear();
            }
            if faceless {
                callback.reversible = false;
                let encounter = callback.effects.encounter.as_mut().ok_or("encounter")?;
                encounter.phase = Some(
                    oteryn_gameplay_quests::quest::world_callbacks::EncounterPhase::FacelessBane,
                );
                encounter.minimum_level = 250;
                encounter.monsters.truncate(1);
                encounter.monsters[0].creature = "oteryn:creature.faceless_bane".into();
            }
            let trigger = Position {
                x: 100,
                y: 100,
                floor: -7,
            };
            let world = if faceless {
                let tiles: Vec<_> = (117..=123)
                    .map(|x| {
                        (
                            Position {
                                x,
                                y: 117,
                                floor: -7,
                            },
                            23108,
                        )
                    })
                    .collect();
                super::quest_world_callbacks::world_with_action_tiles(&callback, trigger, &tiles)?
            } else {
                super::quest_world_callbacks::world(&callback, trigger)?
            };
            let mut deps = Dependencies::new()?;
            let (controller, activated) =
                super::death_ml_loss_composed::production_generation_with_pin(
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
                        x: 100,
                        y: 100,
                        floor: 7,
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
                (100, 100, 7),
            );
            let mut runtime = ChannelRuntimeV1::from_committed_assignment(
                deps.admitted.world_id,
                deps.admitted.channel_id,
                crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
                1,
                1,
                1,
                "runtime-scope-assignment:1",
                64,
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
                        x: 100,
                        y: 100,
                        floor: 7,
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
            owner.quest_catalogue = Some(Arc::new(if dream_scar {
                dream_scar_catalogue(&callback)?
            } else {
                oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                    .map_err(debug)?
                    .catalogue()
                    .clone()
            }));
            assert!(
                owner
                    .states
                    .quest_callbacks
                    .fixture
                    .set(vec![callback.clone()])
                    .is_ok()
            );
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
            let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
            h.root
                .initialize_admission_equipment(
                    &authority,
                    &h.node,
                    &fence.item_fence(),
                    generation.identity().server_artifact_digest(),
                    fresh.operation(),
                )
                .await
                .map_err(debug)?;
            h.root
                .grant_starter_items(
                    &authority,
                    &h.node,
                    &fence.item_fence(),
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
            // Replacing the fixture runtime does not transfer the admitted Character lease.
            owner
                .runtime
                .lock()
                .await
                .bind_attacker_lease(
                    deps.actor,
                    admitted.game_session_id,
                    crate::foundation::CharacterLease::new(
                        crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                            .map_err(debug)?,
                        fence.character_lease_generation,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
            owner.admit_progression_session(&mut admitted).await;
            if dream_scar || faceless {
                let runtime = owner.runtime.lock().await;
                let mut states = owner.states.spell_states.lock().await;
                assert!(
                    states
                        .initialize(
                            &runtime,
                            deps.actor,
                            admitted.game_session_id,
                            crate::spell::cast::CharacterCastFacts {
                                level: 250,
                                ..crate::gameplay_transport::actor_spell::tests::FACTS
                            },
                            (0, 0),
                            owner.owner_now(),
                        )
                        .is_some()
                );
            } else {
                deps.initialize(&owner).await;
            }
            owner.admit_quest_session(&admitted).await;
            assert!(
                owner
                    .refresh_source_party_presence(deps.actor, admitted.game_session_id)
                    .await
            );
            {
                let runtime = owner.runtime.lock().await;
                runtime
                    .bound_attacker_lease(deps.actor, admitted.game_session_id)
                    .map_err(debug)?;
                for monster in &callback
                    .effects
                    .encounter
                    .as_ref()
                    .ok_or("encounter")?
                    .monsters
                {
                    runtime
                        .companion_policy(&monster.creature)
                        .map_err(|e| format!("production creature {}: {e:?}", monster.creature))?;
                }
                for role in
                    oteryn_gameplay_encounters::bosses::phase_controller::cults::plan().roles
                {
                    runtime.companion_policy(role.creature).map_err(|e| {
                        format!("production phase creature {}: {e:?}", role.creature)
                    })?;
                }
            }
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
            let key =
                oteryn_world_bundle::bundle::placement_key(-7, 100, 100, 1).ok_or("lever key")?;
            let mut placement = world.base().digest().to_vec();
            placement.extend_from_slice(&key.to_be_bytes());
            if dream_scar {
                // Noon on Monday, deliberately away from the day boundary.
                super::HOST_NOW.set(Some(4 * 86_400 + 43_200));
            }
            let outcome = owner
                .use_object(
                    deps.actor,
                    UseCommand {
                        game_session_id: admitted.game_session_id,
                        command_id: 1,
                        item_fence: admitted.item_fence,
                    },
                    WorldObjectTarget {
                        placement: placement.clone(),
                        expected_revision: 0,
                    },
                )
                .await;
            if dream_scar {
                super::HOST_NOW.set(None);
            }
            assert_eq!(
                outcome.disposition,
                UseDisposition::Committed,
                "phase admission"
            );
            if dream_scar {
                let boss = {
                    let runtime = owner.runtime.lock().await;
                    let spawned = runtime.current_live_creature_census().map_err(debug)?;
                    let [boss] = spawned.as_slice() else {
                        return Err("Dream Scar spawns one boss".into());
                    };
                    assert!(runtime.matches_live_creature_identity(*boss, SCAR_BOSS.as_bytes()));
                    *boss
                };
                kill(&owner, &admitted, boss, SCAR_BOSS, 100).await?;
                assert_eq!(
                    owner.drain_kill_rewards(admitted.game_session_id).await,
                    crate::gameplay_transport::kill_reward::DrainOutcome::Drained
                );
                let values: Vec<(String, i64)> = sqlx::query_as(
                    "SELECT track_key, value FROM game_character_quest_tracks WHERE track_key=ANY($1) ORDER BY track_key LIMIT 3",
                ).bind([SCAR_COUNT, SCAR_TIMER].as_slice()).fetch_all(&h.pool).await?;
                assert_eq!(values.len(), 2);
                assert_eq!(values[1], (SCAR_COUNT.into(), 0));
                assert_eq!(values[0].0, SCAR_TIMER);
                let now = super::super::unix_seconds().ok_or("host clock")?;
                assert!((now - 60..=now).contains(&values[0].1));
                {
                    let mut runtime = owner.runtime.lock().await;
                    let expected = runtime.read_actor_position(deps.actor).map_err(debug)?;
                    let closed = crate::map::floor::BundleDoors(world.facts());
                    let proof = crate::map::floor::prepare_quest_callback_teleport(
                        &runtime,
                        &cells,
                        deps.actor,
                        admitted.game_session_id,
                        expected,
                        MovementLocalPosition {
                            x: 100,
                            y: 100,
                            floor: 7,
                        },
                        false,
                        &closed,
                    )
                    .map_err(debug)?;
                    runtime
                        .commit_source_steps_and_project(vec![proof], || true)
                        .map_err(debug)?;
                }
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(1))
                    .ok_or("reuse clock")?;
                let outcome = owner
                    .use_object(
                        deps.actor,
                        UseCommand {
                            game_session_id: admitted.game_session_id,
                            command_id: 101,
                            item_fence: admitted.item_fence,
                        },
                        WorldObjectTarget {
                            placement: placement.clone(),
                            expected_revision: crate::map::view::MapFacts::object_revision(
                                world.facts(),
                                key,
                            ),
                        },
                    )
                    .await;
                assert_eq!(
                    outcome.disposition,
                    UseDisposition::NotPossible,
                    "Alptramun cooldown"
                );
                eprintln!(
                    "POSTGRES_CASE_COMPLETED dcourts2_dream_scar_uses_startup_roster_on_lever_use_postgres duration={:?}",
                    began.elapsed(),
                );
                drop(owner);
                drop(authority);
                drop(seal);
                return h.cleanup().await;
            }
            if faceless {
                faceless_steps(&mut owner, &admitted).await?;
                drop(owner);
                drop(authority);
                drop(seal);
                return h.cleanup().await;
            }
            assert_eq!(
                owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .encounter_phases
                    .active
                    .lock()
                    .map_err(|_| "phases")?
                    .len(),
                1
            );
            let plan = oteryn_gameplay_encounters::bosses::phase_controller::cults::plan();
            let mut command = 100;
            for i in (0..5).flat_map(|i| [i, i + 5]).chain([10]) {
                assert_eq!(
                    value(&h).await?,
                    0,
                    "no credit before the actual Essence death"
                );
                command += 1;
                let target = {
                    let runtime = owner.runtime.lock().await;
                    runtime
                        .positioned_melee_census()
                        .map_err(debug)?
                        .into_iter()
                        .find(|(actor, _, _)| {
                            runtime.matches_live_creature_identity(
                                *actor,
                                plan.roles[i].creature.as_bytes(),
                            )
                        })
                        .map(|(actor, _, _)| actor)
                        .ok_or("expected source phase actor")?
                };
                let decorations = if i == 10 {
                    let runtime = owner.runtime.lock().await;
                    let decorations: Vec<_> = runtime
                        .positioned_melee_census()
                        .map_err(debug)?
                        .into_iter()
                        .filter(|(actor, _, _)| {
                            runtime.matches_live_creature_identity(
                                *actor,
                                b"oteryn:creature.destroyed_pillar",
                            )
                        })
                        .map(|(actor, _, _)| actor)
                        .collect();
                    assert_eq!(decorations.len(), 5, "all five pillars became decorations");
                    decorations
                } else {
                    Vec::new()
                };
                kill(&owner, &admitted, target, plan.roles[i].creature, command).await?;
                if i == 10 {
                    let runtime = owner.runtime.lock().await;
                    for decoration in decorations {
                        assert!(
                            !runtime.contains_live_creature(decoration),
                            "victory retires the exact pillar decoration"
                        );
                    }
                    assert!(
                        runtime
                            .positioned_melee_census()
                            .map_err(debug)?
                            .iter()
                            .all(|(actor, _, _)| !runtime.matches_live_creature_identity(
                                *actor,
                                b"oteryn:creature.destroyed_pillar",
                            )),
                        "victory leaves no live pillar decorations"
                    );
                }
                if i < 10 {
                    let runtime = owner.runtime.lock().await;
                    let essence = runtime
                        .positioned_melee_census()
                        .map_err(debug)?
                        .iter()
                        .filter(|(actor, _, _)| {
                            runtime.matches_live_creature_identity(
                                *actor,
                                plan.roles[10].creature.as_bytes(),
                            )
                        })
                        .count();
                    assert_eq!(
                        essence,
                        usize::from(i == 9),
                        "Essence joins only after all five guardians"
                    );
                }
            }
            assert_eq!(
                owner.drain_kill_rewards(admitted.game_session_id).await,
                crate::gameplay_transport::kill_reward::DrainOutcome::Drained
            );
            assert_eq!(value(&h).await?, 2, "source Humans.Mission reaches two");
            assert_eq!(
                owner.drain_kill_rewards(admitted.game_session_id).await,
                crate::gameplay_transport::kill_reward::DrainOutcome::Drained
            );
            assert_eq!(
                value(&h).await?,
                2,
                "replayed deaths never duplicate settlement"
            );
            let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-transition/native-kills/cults-essence'")
            .fetch_one(&h.pool).await?;
            assert_eq!(receipts, 1);
            if case == (EncounterCase::Cults { reenter: true }) {
                {
                    let mut runtime = owner.runtime.lock().await;
                    let expected = runtime.read_actor_position(deps.actor).map_err(debug)?;
                    let closed = crate::map::floor::BundleDoors(world.facts());
                    let proof = crate::map::floor::prepare_quest_callback_teleport(
                        &runtime,
                        &cells,
                        deps.actor,
                        admitted.game_session_id,
                        expected,
                        MovementLocalPosition {
                            x: 100,
                            y: 100,
                            floor: 7,
                        },
                        false,
                        &closed,
                    )
                    .map_err(debug)?;
                    runtime
                        .commit_source_steps_and_project(vec![proof], || true)
                        .map_err(debug)?;
                }
                assert!(
                    owner
                        .refresh_source_party_presence(deps.actor, admitted.game_session_id)
                        .await
                );
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(1))
                    .ok_or("lever retry clock")?;
                let revision = crate::map::view::MapFacts::object_revision(world.facts(), key);
                let reset = owner
                    .use_object(
                        deps.actor,
                        UseCommand {
                            game_session_id: admitted.game_session_id,
                            command_id: 2,
                            item_fence: admitted.item_fence,
                        },
                        WorldObjectTarget {
                            placement: placement.clone(),
                            expected_revision: revision,
                        },
                    )
                    .await;
                assert_eq!(
                    reset.disposition,
                    UseDisposition::Committed,
                    "reset reversible source lever"
                );
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(1))
                    .ok_or("lever retry clock")?;
                let revision = crate::map::view::MapFacts::object_revision(world.facts(), key);
                let second = owner
                    .use_object(
                        deps.actor,
                        UseCommand {
                            game_session_id: admitted.game_session_id,
                            command_id: 3,
                            item_fence: admitted.item_fence,
                        },
                        WorldObjectTarget {
                            placement,
                            expected_revision: revision,
                        },
                    )
                    .await;
                assert_eq!(
                    second.disposition,
                    UseDisposition::Committed,
                    "terminal victory releases the same Cults lever key"
                );
            }
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })?;
    let elapsed = began.elapsed().as_secs_f64();
    assert!(elapsed > 0.05);
    eprintln!("ENGENC2_CULTS_PG_EXECUTED elapsed_s={elapsed:.6}");
    Ok(())
}

fn dream_scar_catalogue(
    callback: &Callback,
) -> TestResult<oteryn_gameplay_quests::quest::QuestStateCatalogue> {
    use oteryn_gameplay_quests::quest::{
        QuestStateCatalogue,
        authored_bindings::{Bindings, BoundQuest, Stage, Track, Trigger, kills::KillCounter},
    };
    use sha2::Digest;
    let hash = sha2::Sha256::digest(serde_json::to_vec(callback)?);
    let bindings = Bindings {
        schema: 1,
        quests: vec![BoundQuest {
            quest: callback.quest.clone(),
            recipe_sha256: format!("{hash:x}"),
            occurrence: None,
            admission: None,
            tracks: vec![
                Track {
                    key: SCAR_COUNT.into(),
                    initial: -1,
                    minimum: -1,
                    maximum: 6,
                },
                Track {
                    key: SCAR_GATE.into(),
                    initial: 1,
                    minimum: 0,
                    maximum: 1,
                },
                Track {
                    key: SCAR_TIMER.into(),
                    initial: 0,
                    minimum: 0,
                    maximum: i64::MAX,
                },
            ],
            stages: vec![
                Stage {
                    key: "lever".into(),
                    completes: false,
                    trigger: Trigger::MapUse {
                        callback: Box::new(callback.clone()),
                    },
                },
                Stage {
                    key: "alptramun".into(),
                    completes: false,
                    trigger: Trigger::Kill {
                        counter: Box::new(KillCounter {
                            key: "oteryn:quest-transition/native-kills/dcourts2-alptramun".into(),
                            creature: SCAR_BOSS.into(),
                            track: SCAR_COUNT.into(),
                            gate: SCAR_GATE.into(),
                            gate_value: 1,
                            minimum: -1,
                            maximum: 6,
                            advance_gate_to: None,
                            death_area: None,
                            world_change: None,
                            encounter_source: Some(callback.key.clone()),
                            cooldown_track: Some(SCAR_TIMER.into()),
                        }),
                    },
                },
            ],
            sources: vec!["Crystal 201189f8 creaturescripts_dream_courts_death.lua:88".into()],
        }],
    };
    Ok(bindings.attach(&QuestStateCatalogue::empty("content-1").ok_or("quest catalogue")?)?)
}

async fn faceless_steps(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &crate::gameplay_transport::connection::AdmittedSession,
) -> TestResult {
    use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};
    let player = admitted.runtime_actor.ok_or("player")?;
    let session = admitted.game_session_id;
    let (boss, max) = {
        let mut runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let boss = runtime
            .current_live_creature_census()
            .map_err(debug)?
            .into_iter()
            .find(|actor| {
                runtime.matches_live_creature_identity(*actor, b"oteryn:creature.faceless_bane")
            })
            .ok_or("Faceless Bane")?;
        let max = runtime
            .live_creature_health(boss)
            .ok_or("Faceless health")?;
        faceless_hit(
            &mut runtime,
            boss,
            player,
            session,
            20,
            i64::try_from(max - max / 5)?,
        )
        .map_err(debug)?;
        crate::gameplay_transport::encounter_phases::tick(&mut runtime, &states);
        assert_eq!(runtime.live_creature_health(boss), Some(max));
        assert!(faceless_hit(&mut runtime, boss, player, session, 21, 1).is_err());
        (boss, max)
    };

    for _ in 0..7 {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_millis(500))
            .ok_or("movement clock")?;
        let (step, _) = owner.paced_step(player, session, StepDirection::East).await;
        assert_eq!(step.disposition, StepDisposition::Moved);
    }

    let mut runtime = owner.runtime.lock().await;
    faceless_hit(&mut runtime, boss, player, session, 22, 1).map_err(debug)?;
    assert_eq!(runtime.live_creature_health(boss), Some(max - 1));
    Ok(())
}

fn faceless_hit(
    runtime: &mut ChannelRuntimeV1,
    boss: ExactActorRef,
    player: ExactActorRef,
    session: crate::foundation::GameSessionId,
    command: u64,
    damage: i64,
) -> Result<(), crate::foundation::CarrierError> {
    runtime
        .borrow_exact_actor_commit_at(0)
        .commit_damage_for_bound_attacker(
            boss,
            player,
            CommandRef::new(
                session,
                CommandId::new(command)
                    .map_err(|_| crate::foundation::CarrierError::PlanConflict)?,
            ),
            0,
            OwnerDamageCommand {
                target: b"oteryn:creature.faceless_bane",
                occurrence: b"dcourts2",
                binding: b"faceless-bane-damage",
                damage,
            },
        )
        .map(|_| ())
}

async fn value(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE track_key=$1),0)",
    )
    .bind(COUNT)
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
                occurrence: b"engenc2",
                binding: b"engenc2-phase-death",
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
