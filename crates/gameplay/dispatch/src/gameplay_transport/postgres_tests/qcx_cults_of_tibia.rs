use super::classic_quests::{install_positions, progress, talk};
use super::fixture::{Harness, debug, id, run};
use super::quest_world_callbacks::{local, wire, world};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, CharacterId, CharacterLease, NodeId};
use crate::gameplay_transport::SeamRooms;
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::map::overlay::TilePos;
use oteryn_gameplay_quests::quest::world_callbacks::{Position, load};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
    encode_step_intent,
};

#[test]
fn cult_teleports_publish_the_source_effect_at_the_landing() -> TestResult {
    run_cases(&[
        (
            "carlin_exit",
            Some(Position {
                x: 32403,
                y: 31813,
                floor: -8,
            }),
        ),
        (
            "essence_exit",
            Some(Position {
                x: 32349,
                y: 31668,
                floor: -10,
            }),
        ),
        (
            "orc_entry",
            Some(Position {
                x: 33182,
                y: 31837,
                floor: -15,
            }),
        ),
        (
            "orc_exit",
            Some(Position {
                x: 33168,
                y: 31930,
                floor: -12,
            }),
        ),
    ])
}

#[test]
fn gerimor_is_reachable_at_the_production_placement() -> TestResult {
    run_cases(&[("gerimor", None)])
}

fn gerimor_position() -> TestResult<Position> {
    let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../"));
    let index: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/world/npc-placements/index.json"),
    )?)?;
    let expected = Position {
        x: 33450,
        y: 32246,
        floor: -7,
    };
    for shard in index["shards"].as_array().ok_or("placement shards")? {
        let path = shard.as_str().ok_or("placement shard path")?;
        let document: serde_json::Value = serde_json::from_slice(&std::fs::read(root.join(path))?)?;
        for record in document["records"].as_array().ok_or("placement records")? {
            let row = &record["declaration"];
            if row["npc"] == "oteryn:npc.gerimor" {
                assert_eq!(
                    row["cell"],
                    serde_json::json!({
                        "x": expected.x, "y": expected.y, "floor": -expected.floor
                    })
                );
                return Ok(expected);
            }
        }
    }
    Err("missing production Gerimor placement".into())
}

fn run_cases(cases: &'static [(&'static str, Option<Position>)]) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("qcx-cults", move || {
        run(async |admin| {
            let started = std::time::Instant::now();
            let callbacks = load()?;
            let mut mismatches = Vec::new();
            for &(name, destination) in cases {
                let case_started = std::time::Instant::now();
                // The Carlin callback supplies a simple walkable room fixture for the NPC
                // case. No movement through its source selector occurs in that case.
                let fixture = if destination.is_some() {
                    name
                } else {
                    "carlin_exit"
                };
                let key = format!("oteryn:quest-callback/cults_of_tibia/{fixture}");
                let callback = callbacks.iter().find(|entry| entry.key == key).ok_or(key)?;
                reset();
                let h = Harness::create_with_admission_seed(
                    admin.clone(),
                    &format!("qcx_cults_{name}"),
                    true,
                    false,
                )
                .await?;
                prepare_current_session(&h).await?;
                let trigger = if destination.is_some() {
                    callback.selector.position.ok_or("source trigger")?
                } else {
                    match gerimor_position() {
                        Ok(position) => position,
                        Err(error) => {
                            h.cleanup().await?;
                            eprintln!(
                                "QCX_CULTS_POSTGRES_CASE {name} duration={:?}",
                                case_started.elapsed()
                            );
                            return Err(error);
                        }
                    }
                };
                let origin = local(Position {
                    x: trigger.x - 1,
                    ..trigger
                });
                let world = world(callback, trigger)?;
                let mut deps = Dependencies::new()?;
                let (controller, activated) =
                    super::death_ml_loss_composed::production_generation_with_pin(
                        deps.admitted.world_id,
                        "content-1",
                    )?;
                let generation = controller.active().ok_or("active generation")?;
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
                            x: origin.x,
                            y: origin.y,
                            floor: origin.floor,
                        },
                    )
                    .ok_or("bundle room")?;
                let pin = ChannelContentPin::from_activation(
                    deps.admitted.world_id,
                    1,
                    generation.identity().server_artifact_digest(),
                    generation.identity().client_artifact_digest(),
                    frame,
                    world.map_revision_digest(),
                    (origin.x, origin.y, origin.floor),
                );
                let mut runtime = ChannelRuntimeV1::from_committed_assignment(
                    deps.admitted.world_id,
                    deps.admitted.channel_id,
                    NodeId::decode(&id(1)).map_err(debug)?,
                    1,
                    1,
                    1,
                    "runtime-scope-assignment:1",
                    8,
                    pin,
                )
                .map_err(debug)?;
                let reservation = runtime
                    .reserve_fresh_session(deps.admitted.game_session_id)
                    .map_err(debug)?;
                deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
                runtime
                    .initialize_pinned_test_position(deps.actor, origin)
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
                owner.rooms = SeamRooms::new(None, Some(&room));
                let mut admitted = deps.admitted;
                admitted.runtime_actor = Some(deps.actor);
                admitted.first_entry = FirstEntryOutcome::Positioned;
                let fence = owner
                    .current_quest_fence(admitted.game_session_id)
                    .await
                    .map_err(|()| "quest fence")?
                    .ok_or("quest fence")?;
                owner
                    .runtime
                    .lock()
                    .await
                    .bind_attacker_lease(
                        deps.actor,
                        admitted.game_session_id,
                        CharacterLease::new(
                            CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                            fence.character_lease_generation,
                        )
                        .map_err(debug)?,
                    )
                    .map_err(debug)?;
                admitted.item_fence = Some(fence.item_fence());
                deps.initialize(&owner).await;
                if let Some(destination) = destination {
                    let reply = wire(
                        &owner,
                        &mut admitted,
                        1,
                        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
                        encode_step_intent(StepDirection::East),
                    )
                    .await?;
                    assert_eq!(
                        decode_step_result(&reply).map_err(debug)?,
                        StepDisposition::Moved
                    );
                    assert_eq!(
                        owner
                            .runtime
                            .lock()
                            .await
                            .read_actor_position(deps.actor)
                            .map_err(debug)?
                            .position(),
                        local(destination),
                        "{name}"
                    );
                    let map = owner.bundle_map().ok_or("callback map")?;
                    let effects = map.facts.movement_effects_since(0);
                    let observed: Vec<_> = effects
                        .iter()
                        .map(|&(_, at, effect)| (at, effect))
                        .collect();
                    if observed
                        != [(
                            TilePos {
                                x: destination.x,
                                y: destination.y,
                                floor: destination.floor,
                            },
                            11,
                        )]
                    {
                        mismatches.push((name, observed));
                    }
                } else {
                    owner.quest_catalogue = Some(std::sync::Arc::new(
                        oteryn_gameplay_quests::quest::loader::load_embedded_quest_state(
                            "content-1",
                        )
                        .map_err(debug)?
                        .catalogue()
                        .clone(),
                    ));
                    let [npc] =
                        install_positions(&owner, &admitted, ["oteryn:npc.gerimor"], [trigger])
                            .await?;
                    talk(&mut owner, &admitted, npc, 32000, "hi").await?;
                    talk(&mut owner, &admitted, npc, 32001, "mission").await?;
                    let offer = talk(&mut owner, &admitted, npc, 32002, "minotaurs").await?;
                    assert!(offer.iter().any(|line| line.contains("animal-like cult")));
                    talk(&mut owner, &admitted, npc, 32003, "yes").await?;
                    assert_eq!(
                        progress(
                            &owner,
                            &admitted,
                            &authority,
                            "oteryn:quest-progress/quest/u11_40/cults_of_tibia/minotaurs/mission",
                        )
                        .await?,
                        1
                    );
                }
                drop(owner);
                drop(authority);
                drop(seal);
                h.cleanup().await?;
                eprintln!(
                    "QCX_CULTS_POSTGRES_CASE {name} duration={:?}",
                    case_started.elapsed()
                );
            }
            eprintln!(
                "QCX_CULTS_POSTGRES_COMPLETED duration={:?}",
                started.elapsed()
            );
            assert!(
                mismatches.is_empty(),
                "unexpected teleport effects: {mismatches:?}"
            );
            Ok(())
        })
    })
}
