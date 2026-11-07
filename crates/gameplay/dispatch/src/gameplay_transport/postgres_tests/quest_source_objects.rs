//! Actual source-world commands, including native ground replacement and occupancy.
use super::fixture::{Harness, debug, id, run};
use super::quest_world_callbacks::{local, wire};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1};
use crate::gameplay_transport::connection::{AdmittedSession, FirstEntryOutcome};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

#[test]
fn quest_source_objects_life_ring_real_use_opens_and_restores_native_water() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("life-ring-source-world", life_ring);
    let marker = if result.is_ok() {
        "POSTGRES_CASE_COMPLETED"
    } else {
        "POSTGRES_CASE_FAILED"
    };
    println!(
        "{marker} quest_source_objects_life_ring_real_use_opens_and_restores_native_water duration={:?}",
        started.elapsed()
    );
    result
}

#[test]
fn quest_source_objects_mintwallin_real_use_applies_all_twelve_edits() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("mintwallin-source-world", || {
        source_objects(true, false)
    });
    println!(
        "POSTGRES_CASE_{} quest_source_objects_mintwallin duration={:?}",
        if result.is_ok() {
            "COMPLETED"
        } else {
            "FAILED"
        },
        started.elapsed()
    );
    result
}

#[test]
fn quest_source_objects_hidden_threats_stepout_requires_both_once_claims() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result =
        super::kill_reward_e2e::scenario_thread::run("hidden-threats-source-world", || {
            source_objects(false, true)
        });
    println!(
        "POSTGRES_CASE_{} quest_source_objects_hidden_threats duration={:?}",
        if result.is_ok() {
            "COMPLETED"
        } else {
            "FAILED"
        },
        started.elapsed()
    );
    result
}

fn life_ring() -> TestResult {
    source_objects(false, false)
}

pub(super) fn hidden_npc_scenario(phase: u8) -> TestResult {
    source_objects_with_npc_start(false, true, Some(phase))
}

fn source_objects(mint: bool, hidden: bool) -> TestResult {
    source_objects_with_npc_start(mint, hidden, None)
}

fn source_objects_with_npc_start(mint: bool, hidden: bool, npc_start: Option<u8>) -> TestResult {
    let trigger = if hidden {
        Position {
            x: 33079,
            y: 32014,
            floor: -13,
        }
    } else if mint {
        Position {
            x: 32600,
            y: 32105,
            floor: -14,
        }
    } else {
        Position {
            x: 32413,
            y: 32230,
            floor: -10,
        }
    };
    let bridge: Vec<_> = if mint {
        (32593..32602)
            .map(|x| Position {
                x,
                y: 32104,
                floor: -14,
            })
            .collect()
    } else {
        [32231, 32232]
            .into_iter()
            .flat_map(|y| {
                [32410, 32411, 32412]
                    .into_iter()
                    .map(move |x| Position { x, y, floor: -10 })
            })
            .collect()
    };
    // This describes only the native test world. It is never injected as a callback.
    // The actual production registry must bind the no-AID lever and implement the source.
    let mut callback: Callback = serde_json::from_value(serde_json::json!({
        "key": "oteryn:quest-callback/life-ring-bridge", "quest": "oteryn:quest.life_ring_quest",
        "trigger": "use", "selector": {"position": trigger, "unique": null, "action": null, "appearance": 2772},
        "lever": [2772,2773], "reversible": true,
        "passages": bridge.iter().map(|position| serde_json::json!({"position":position,"appearance":4598,"replacement":5770})).collect::<Vec<_>>(),
        "teleport": null, "storage": [], "offerings": [], "return_on_failure": false,
        "failure_message": null, "sources": []
    }))?;
    let created = [
        Position {
            x: 32592,
            y: 32105,
            floor: -14,
        },
        Position {
            x: 32592,
            y: 32106,
            floor: -14,
        },
    ];
    if mint {
        callback.key = "oteryn:quest-callback/mintwallin-cyclops-lever".into();
        callback.quest = "oteryn:quest.mintwallin_cyclops_quest".into();
        callback.lever = None;
        for passage in &mut callback.passages {
            passage.appearance = 1271;
            passage.replacement = None;
        }
        callback.objects = Some(
            oteryn_gameplay_quests::quest::world_callbacks::ObjectEffects {
                consume_trigger: false,
                mutations: created
                    .iter()
                    .map(|position| {
                        oteryn_gameplay_quests::quest::world_callbacks::ObjectMutation {
                            position: *position,
                            remove: None,
                            create: Some(1270),
                        }
                    })
                    .collect(),
            },
        );
    }
    let destination = Position {
        x: 32409,
        y: 32231,
        floor: -10,
    };
    let hidden_tiles = [
        (33079, 32014),
        (33079, 32015),
        (33080, 32015),
        (33032, 32050),
        (33032, 32051),
    ]
    .map(|(x, y)| Position { x, y, floor: -13 });
    let chests = [(33080, 32014), (33031, 32050), (33078, 31980)]
        .map(|(x, y)| (Position { x, y, floor: -13 }, 2469));
    let extras = if mint {
        created.to_vec()
    } else {
        vec![destination]
    };
    let world = if hidden {
        callback.passages.clear();
        callback.selector.appearance = None;
        let mut extras: Vec<_> = hidden_tiles
            .iter()
            .flat_map(|p| {
                [
                    *p,
                    Position { x: p.x - 1, ..*p },
                    Position { x: p.x + 1, ..*p },
                ]
            })
            .chain(chests.iter().map(|(p, _)| Position { x: p.x - 1, ..*p }))
            .collect();
        if npc_start.is_some() {
            extras.extend((33100..33110).flat_map(|x| {
                [
                    Position {
                        x,
                        y: 32000,
                        floor: -13,
                    },
                    Position {
                        x,
                        y: 32001,
                        floor: -13,
                    },
                ]
            }));
        }
        super::quest_world_callbacks::world_with_chests(
            &callback, trigger, &extras, &chests, false,
        )?
    } else {
        let chest = if mint {
            Position {
                x: 32591,
                y: 32097,
                floor: -14,
            }
        } else {
            Position {
                x: 32443,
                y: 32238,
                floor: -11,
            }
        };
        let mut extras = extras;
        extras.push(Position {
            x: chest.x - 1,
            ..chest
        });
        super::quest_world_callbacks::world_with_chests(
            &callback,
            trigger,
            &extras,
            &[(chest, if mint { 2472 } else { 2469 })],
            !mint,
        )?
    };
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "quest_world_callbacks", false, false)
            .await?;
        let version: i32 =
            sqlx::query_scalar("SELECT current_setting('server_version_num')::INTEGER")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(version, 180006);
        println!("POSTGRES_CASE_STARTED quest_source_objects mint={mint} version={version}");
        prepare_current_session(&h).await?;
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
        let generation = controller.active().ok_or("generation")?;
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
                i32::from(trigger.x) - 1,
                i32::from(trigger.y),
                -i16::from(trigger.floor),
            ),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            if npc_start.is_some() { 32 } else { 8 },
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        let origin = local(Position {
            x: trigger.x,
            y: trigger.y + 1,
            ..trigger
        });
        runtime
            .initialize_pinned_test_position(deps.actor, origin)
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.movement_cells = &cells;
        owner.quest_catalogue = Some(std::sync::Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        admitted.item_fence = Some(fence.item_fence());
        sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice()).bind(activated.frame_binding().digest().as_slice()).execute(&h.pool).await?;
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
        let native = generation.native_gameplay().ok_or("native")?;
        let template = crate::durability::starter_grant::StarterTemplate {
            definition: crate::durability::item_mint::TypedDefinitionRef {
                family: "Item".into(),
                production_key: "oteryn:item.tibia.i2854".into(),
                revision_ref: "definition-r1".into(),
            },
            ..crate::durability::starter_grant::StarterTemplate::main()
        };
        h.root
            .grant_starter_items(
                &authority,
                &h.node,
                &fence.item_fence(),
                &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
                &template,
            )
            .await
            .map_err(debug)?;
        owner.admit_progression_session(&mut admitted).await;
        deps.initialize(&owner).await;
        owner.admit_quest_session(&admitted).await;
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
        if hidden {
            let mut dialogue = if let Some(phase) = npc_start {
                let dialogue =
                    super::hidden_threats_npcs::start(&mut owner, &mut admitted, &h, &authority)
                        .await?;
                if phase == 0 {
                    drop(owner);
                    drop(authority);
                    drop(seal);
                    return h.cleanup().await;
                }
                Some(dialogue)
            } else {
                None
            };
            hidden_steps(
                &mut owner,
                &mut admitted,
                &h,
                deps.actor,
                fence,
                &hidden_tiles,
                &chests,
            )
            .await?;
            if let Some(dialogue) = &mut dialogue {
                super::hidden_threats_npcs::rescues(
                    dialogue, &mut owner, &admitted, &h, &authority,
                )
                .await?;
                if npc_start == Some(2) {
                    super::hidden_threats_npcs::delivery(
                        dialogue, &mut owner, &admitted, &h, &authority,
                    )
                    .await?;
                }
            }
            drop(owner);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
        }
        let map = &world;
        let key =
            oteryn_world_bundle::bundle::placement_key(trigger.floor, trigger.x, trigger.y, 1)
                .ok_or("lever key")?;
        for p in &bridge {
            assert!(!world.enterable(crate::map::overlay::TilePos {
                x: p.x,
                y: p.y,
                floor: p.floor
            }));
        }
        if mint {
            for p in created {
                assert!(world.enterable(crate::map::overlay::TilePos {
                    x: p.x,
                    y: p.y,
                    floor: p.floor
                }));
            }
            assert_eq!(
                source_use(&mut owner, &mut admitted, 1, key, 0).await?,
                UseDisposition::Committed
            );
            assert!(map.facts().base_hidden(key), "source lever removed");
            for p in &bridge {
                assert!(
                    world.enterable(crate::map::overlay::TilePos {
                        x: p.x,
                        y: p.y,
                        floor: p.floor
                    }),
                    "all nine walls removed"
                );
            }
            for p in created {
                let pos = crate::map::overlay::TilePos {
                    x: p.x,
                    y: p.y,
                    floor: p.floor,
                };
                assert!(!world.enterable(pos), "new source walls block movement");
                let entries = map.facts().movement_items(pos);
                assert_eq!(entries.len(), 1);
                assert_eq!(entries[0].facts.appearance_id, 1270);
                assert!(!entries[0].facts.pickupable);
            }
            assert_ne!(
                source_use(&mut owner, &mut admitted, 2, key, 1).await?,
                UseDisposition::Committed
            );
            assert_eq!(map.facts().object_revision(key), 1);
            source_cache(&mut owner, &mut admitted, &h, deps.actor, true, 3).await?;
            drop(owner);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
        }
        let opened = source_use(&mut owner, &mut admitted, 1, key, 0).await?;
        assert_eq!(
            opened,
            UseDisposition::Committed,
            "native Life Ring source lever must execute"
        );
        assert_eq!(map.facts().object_revision(key), 1);
        for p in &bridge {
            assert!(
                world.enterable(crate::map::overlay::TilePos {
                    x: p.x,
                    y: p.y,
                    floor: p.floor
                }),
                "every native water ground becomes a traversable bridge"
            );
        }
        assert_eq!(
            source_use(&mut owner, &mut admitted, 2, key, 0).await?,
            UseDisposition::StaleState
        );
        assert_eq!(map.facts().object_revision(key), 1);
        let (guest, creature, outside) = {
            let mut runtime = owner.runtime.lock().await;
            let guest_session = crate::foundation::GameSessionId::decode(&id(84)).map_err(debug)?;
            let reservation = runtime
                .reserve_fresh_session(guest_session)
                .map_err(debug)?;
            let guest = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime
                .initialize_pinned_test_position(guest, local(bridge[0]))
                .map_err(debug)?;
            let creature = runtime
                .admit_encounter_actor("oteryn:creature.rat", 20, local(bridge[4]))
                .map_err(debug)?;
            let outside = runtime
                .admit_encounter_actor(
                    "oteryn:creature.rat",
                    20,
                    local(Position {
                        x: 32414,
                        y: 32231,
                        floor: -10,
                    }),
                )
                .map_err(debug)?;
            (guest, creature, outside)
        };
        {
            let mut runtime = owner.runtime.lock().await;
            let before = runtime.read_actor_position(creature).map_err(debug)?;
            let doors = crate::map::floor::BundleDoors(map.facts().doors());
            let proof = crate::map::floor::prepare_quest_callback_ejection(
                &runtime,
                owner.movement_cells,
                creature,
                None,
                before,
                local(destination),
                false,
                &doors,
            )
            .map_err(debug)?;
            assert!(
                runtime
                    .commit_quest_world_projection(vec![proof], || false)
                    .is_err()
            );
            assert_eq!(
                runtime.read_actor_position(creature).map_err(debug)?,
                before,
                "map rejection rolls back physical position and revision"
            );
            let first = crate::map::floor::prepare_quest_callback_ejection(
                &runtime,
                owner.movement_cells,
                creature,
                None,
                before,
                local(destination),
                false,
                &doors,
            )
            .map_err(debug)?;
            let stale_after_first = crate::map::floor::prepare_quest_callback_ejection(
                &runtime,
                owner.movement_cells,
                creature,
                None,
                before,
                local(destination),
                false,
                &doors,
            )
            .map_err(debug)?;
            let mut projected = false;
            assert!(
                runtime
                    .commit_quest_world_projection(vec![first, stale_after_first], || {
                        projected = true;
                        true
                    })
                    .is_err()
            );
            assert!(!projected, "stale later proof must never publish the map");
            assert_eq!(
                runtime.read_actor_position(creature).map_err(debug)?,
                before,
                "later proof failure rolls back earlier physical moves"
            );
            assert_eq!(map.facts().object_revision(key), 1);
        }
        // Match the existing source top-creature authority: an ambiguous
        // stack refuses the whole toggle, preserving every physical/map state.
        {
            let mut runtime = owner.runtime.lock().await;
            let expected = runtime.read_actor_position(creature).map_err(debug)?;
            let doors = crate::map::floor::BundleDoors(map.facts().doors());
            let proof = crate::map::floor::prepare_quest_callback_ejection(
                &runtime,
                owner.movement_cells,
                creature,
                None,
                expected,
                local(bridge[0]),
                false,
                &doors,
            )
            .map_err(debug)?;
            runtime.commit_source_step(proof).map_err(debug)?;
        }
        assert_eq!(
            source_use(&mut owner, &mut admitted, 3, key, 1).await?,
            UseDisposition::Occupied
        );
        assert_eq!(map.facts().object_revision(key), 1);
        {
            let mut runtime = owner.runtime.lock().await;
            assert_eq!(
                runtime
                    .read_actor_position(guest)
                    .map_err(debug)?
                    .position(),
                local(bridge[0])
            );
            let expected = runtime.read_actor_position(creature).map_err(debug)?;
            assert_eq!(expected.position(), local(bridge[0]));
            let doors = crate::map::floor::BundleDoors(map.facts().doors());
            let proof = crate::map::floor::prepare_quest_callback_ejection(
                &runtime,
                owner.movement_cells,
                creature,
                None,
                expected,
                local(bridge[4]),
                false,
                &doors,
            )
            .map_err(debug)?;
            runtime.commit_source_step(proof).map_err(debug)?;
        }
        let reply = wire(
            &owner,
            &mut admitted,
            4,
            oteryn_protocol_oteryn::world_spatial::COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            oteryn_protocol_oteryn::world_spatial::encode_step_intent(
                oteryn_protocol_oteryn::world_spatial::StepDirection::West,
            ),
        )
        .await?;
        assert_eq!(
            oteryn_protocol_oteryn::world_spatial::decode_step_result(&reply).map_err(debug)?,
            oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved
        );
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position(),
            local(Position {
                x: 32412,
                y: 32231,
                floor: -10
            })
        );
        assert_eq!(
            source_use(&mut owner, &mut admitted, 5, key, 1).await?,
            UseDisposition::Committed
        );
        assert_eq!(map.facts().object_revision(key), 2);
        {
            let runtime = owner.runtime.lock().await;
            for actor in [deps.actor, guest, creature] {
                assert_eq!(
                    runtime
                        .read_actor_position(actor)
                        .map_err(debug)?
                        .position(),
                    local(destination),
                    "closing ejects the caller and every other bridge occupant"
                );
            }
            assert_eq!(
                runtime
                    .read_actor_position(outside)
                    .map_err(debug)?
                    .position(),
                local(Position {
                    x: 32414,
                    y: 32231,
                    floor: -10
                }),
                "actors outside the six source cells stay put"
            );
        }
        for p in &bridge {
            assert!(
                !world.enterable(crate::map::overlay::TilePos {
                    x: p.x,
                    y: p.y,
                    floor: p.floor
                }),
                "closing restores the original native water collision"
            );
        }
        let receipts: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(
            receipts, 0,
            "a world-only lever cannot invent durable quest progress"
        );
        source_cache(&mut owner, &mut admitted, &h, deps.actor, false, 6).await?;
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn source_use(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: u64,
    key: u64,
    revision: u64,
) -> TestResult<UseDisposition> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let map = owner.bundle_map().ok_or("source map")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget {
        placement,
        expected_revision: revision,
    })
    .map_err(debug)?;
    let reply = wire(
        owner,
        admitted,
        command,
        oteryn_protocol_oteryn::world_object::COMMAND_TYPE_USE_INTENT,
        payload,
    )
    .await?;
    oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)
}

pub(super) async fn fixture_relocate(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    actor: crate::foundation::ExactActorRef,
    position: Position,
) -> TestResult {
    let map = owner.bundle_map().ok_or("map")?;
    let mut runtime = owner.runtime.lock().await;
    let expected = runtime.read_actor_position(actor).map_err(debug)?;
    let doors = crate::map::floor::BundleDoors(map.facts.doors());
    let proof = crate::map::floor::prepare_quest_callback_teleport(
        &runtime,
        owner.movement_cells,
        actor,
        admitted.game_session_id,
        expected,
        local(position),
        false,
        &doors,
    )
    .map_err(debug)?;
    runtime.commit_source_step(proof).map_err(debug)?;
    Ok(())
}

#[allow(
    clippy::too_many_arguments,
    reason = "one admitted source-world PG scenario"
)]
async fn hidden_steps(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    actor: crate::foundation::ExactActorRef,
    fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
    tiles: &[Position; 5],
    chests: &[(Position, u16); 3],
) -> TestResult {
    use crate::durability::quest_state::{
        QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
    };
    use crate::foundation::{CommandId, CommandRef};
    const TRACK: &str = "oteryn:quest-progress/quest/u11_50/hidden_threats/quest_line";
    let mut command = 0_u64;
    let mut seed_command = 900_u64;
    // NPC source transitions establish progress. No fixture SQL writes quest flags.
    for (index, tile) in tiles.iter().enumerate() {
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        for name in ["npc_1", "npc_3"] {
            seed_command += 1;
            let outcome = slot
                .commit_quest_transition(
                    owner.root,
                    owner.character,
                    owner.holder,
                    fence,
                    QuestTransitionRequest {
                        transition_key: format!(
                            "oteryn:quest-transition/quest/u11_50/hidden_threats/quest_line/{name}"
                        ),
                        cause: QuestCause::Command(CommandRef::new(
                            admitted.game_session_id,
                            CommandId::new(seed_command).map_err(debug)?,
                        )),
                    },
                    owner.quest_catalogue.clone().ok_or("catalogue")?,
                )
                .await
                .map_err(debug)?;
            assert!(matches!(outcome, QuestTransitionOutcome::Committed(_)));
        }
        drop(slot);
        if index == 0 {
            // Neither receipt, then only one receipt: leaving must preserve line2.
            for (count, (chest, _)) in chests.iter().take(2).enumerate() {
                fixture_relocate(owner, admitted, actor, *tile).await?;
                source_step(owner, admitted, &mut command).await?;
                let state = owner
                    .root
                    .read_character_quest_state(owner.character, fence.character_id)
                    .await
                    .map_err(debug)?;
                assert_eq!(
                    state.tracks().get(TRACK),
                    Some(&2),
                    "{count} fragments cannot advance"
                );
                let chest = *chest;
                fixture_relocate(
                    owner,
                    admitted,
                    actor,
                    Position {
                        x: chest.x - 1,
                        ..chest
                    },
                )
                .await?;
                let key =
                    oteryn_world_bundle::bundle::placement_key(chest.floor, chest.x, chest.y, 1)
                        .ok_or("chest key")?;
                command += 1;
                assert_eq!(
                    source_use(owner, admitted, command, key, 0).await?,
                    UseDisposition::Committed,
                    "native no-UID fragment chest"
                );
            }
        }
        if index == 0 {
            // Arrival is not StepOut. The ordinary step originates west of the source.
            fixture_relocate(
                owner,
                admitted,
                actor,
                Position {
                    x: tile.x - 1,
                    ..*tile
                },
            )
            .await?;
            source_step(owner, admitted, &mut command).await?;
            let state = owner
                .root
                .read_character_quest_state(owner.character, fence.character_id)
                .await
                .map_err(debug)?;
            assert_eq!(
                state.tracks().get(TRACK),
                Some(&2),
                "arrival cannot advance line2"
            );
        } else {
            fixture_relocate(owner, admitted, actor, *tile).await?;
        }
        if index == 0 {
            owner
                .states
                .quest_callbacks
                .interrupt_after_commit
                .store(true, std::sync::atomic::Ordering::Release);
        }
        source_step(owner, admitted, &mut command).await?;
        let state = owner
            .root
            .read_character_quest_state(owner.character, fence.character_id)
            .await
            .map_err(debug)?;
        assert_eq!(
            state.tracks().get(TRACK),
            Some(&3),
            "each of the five source departures advances after both claims"
        );
        if index == 0 {
            assert!(owner.has_pending_quest_callback(admitted.game_session_id));
            let map = owner.bundle_map().ok_or("map")?;
            let position = crate::map::overlay::TilePos {
                x: tile.x,
                y: tile.y,
                floor: tile.floor,
            };
            let key = oteryn_world_bundle::bundle::placement_key(tile.floor, tile.x, tile.y, 0)
                .ok_or("source ground")?;
            let base = map
                .overlay
                .base()
                .tile(tile.x, tile.y, tile.floor)
                .ok_or("source tile")?;
            let facts = map
                .facts
                .base_entry(position, 0, *base.ids().first().ok_or("source ground id")?)
                .ok_or("source facts")?;
            assert!(
                map.facts
                    .apply_quest_callback(key, 0, facts, &[], &[], false, None),
                "another completed owner event advances the shared source revision"
            );
            assert!(
                owner
                    .reconcile_quest_callbacks(admitted.game_session_id)
                    .await,
                "durable progress recovery must not be blocked by an unrelated shared map revision"
            );
            assert!(!owner.has_pending_quest_callback(admitted.game_session_id));
            assert_eq!(
                map.facts.object_revision(key),
                1,
                "recovery preserves the newer shared map state"
            );
            let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-transition/quest/u11_50/hidden_threats/quest_line/movement_1'").fetch_one(&h.pool).await?;
            assert_eq!(
                receipts, 1,
                "recovery reuses the exact committed progress receipt"
            );
        }
    }
    let claims: i64 = sqlx::query_scalar("SELECT count(*) FROM game_reward_claims WHERE claim_production_key LIKE 'oteryn:reward-claim.quest.u11_50.hidden_threats.rewards.key_fragment%'").fetch_one(&h.pool).await?;
    assert_eq!(claims, 2);
    let chest = chests[2].0;
    fixture_relocate(
        owner,
        admitted,
        actor,
        Position {
            x: chest.x - 1,
            ..chest
        },
    )
    .await?;
    let key = oteryn_world_bundle::bundle::placement_key(chest.floor, chest.x, chest.y, 1)
        .ok_or("file chest")?;
    command += 1;
    assert_eq!(
        source_use(owner, admitted, command, key, 0).await?,
        UseDisposition::Committed,
        "native metal-file chest"
    );
    let items: Vec<String> = sqlx::query_scalar(
        "SELECT i.definition_production_key FROM game_reward_claim_mint_receipts r JOIN game_item_instances i USING(item_instance_id) ORDER BY i.definition_production_key"
    ).fetch_all(&h.pool).await?;
    assert_eq!(
        items,
        [
            "oteryn:item.tibia.i27261",
            "oteryn:item.tibia.i27262",
            "oteryn:item.tibia.i27270"
        ]
    );
    Ok(())
}

async fn source_step(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: &mut u64,
) -> TestResult {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    *command += 1;
    let payload = oteryn_protocol_oteryn::world_spatial::encode_step_intent(
        oteryn_protocol_oteryn::world_spatial::StepDirection::East,
    );
    let reply = wire(
        owner,
        admitted,
        *command,
        oteryn_protocol_oteryn::world_spatial::COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        payload,
    )
    .await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_spatial::decode_step_result(&reply).map_err(debug)?,
        oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved
    );
    Ok(())
}

async fn source_cache(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    actor: crate::foundation::ExactActorRef,
    mint: bool,
    command: u64,
) -> TestResult {
    let chest = if mint {
        Position {
            x: 32591,
            y: 32097,
            floor: -14,
        }
    } else {
        Position {
            x: 32443,
            y: 32238,
            floor: -11,
        }
    };
    fixture_relocate(
        owner,
        admitted,
        actor,
        Position {
            x: chest.x - 1,
            ..chest
        },
    )
    .await?;
    let key = oteryn_world_bundle::bundle::placement_key(chest.floor, chest.x, chest.y, 1)
        .ok_or("cache key")?;
    assert_eq!(
        source_use(owner, admitted, command, key, 0).await?,
        UseDisposition::Committed,
        "native source cache"
    );
    let roots: Vec<(String,i64)> = sqlx::query_as(
        "SELECT i.definition_production_key,i.quantity::bigint FROM game_reward_claim_mint_receipts r JOIN game_item_instances i USING(item_instance_id)"
    ).fetch_all(&h.pool).await?;
    assert_eq!(
        roots,
        vec![(
            if mint {
                "oteryn:item.tibia.i3028"
            } else {
                "oteryn:item.tibia.i2853"
            }
            .to_owned(),
            1
        )]
    );
    if !mint {
        let children: Vec<(String,i64)> = sqlx::query_as(
            "SELECT i.definition_production_key,i.quantity::bigint FROM game_item_nested_entries n JOIN game_item_instances i USING(item_instance_id) JOIN game_reward_claim_mint_receipts r ON n.parent_item_instance_id=r.item_instance_id ORDER BY i.definition_production_key"
        ).fetch_all(&h.pool).await?;
        assert_eq!(
            children,
            vec![
                ("oteryn:item.tibia.i3052".to_owned(), 1),
                ("oteryn:item.tibia.i3085".to_owned(), 1)
            ]
        );
        use crate::content::ReferenceItemField::Known;
        let native = owner
            .active_generation
            .ok_or("generation")?
            .native_gameplay()
            .ok_or("native")?;
        let policy = native
            .item_policy("oteryn:item.tibia.i3085", "definition-r1")
            .ok_or("charge profile")?;
        assert!(
            matches!(&policy.record().semantics.charges, Known(charges) if charges.count == Known(200)),
            "definition charges must exactly match the donor's 200 before using the full-charge representation"
        );
        let states: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_item_timed_states t JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i3085'"
        ).fetch_one(&h.pool).await?;
        assert_eq!(
            states, 0,
            "a fresh full-charge item uses its exact definition value, with no partial override"
        );
    }
    Ok(())
}
