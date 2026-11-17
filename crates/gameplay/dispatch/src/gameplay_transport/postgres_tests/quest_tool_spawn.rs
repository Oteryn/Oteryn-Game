//! Real item-on-map USE and durable progression for a local creature spawn.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_gameplay_world::map::view::MapFacts;
use oteryn_protocol_oteryn::world_object::{
    COMMAND_TYPE_USE_INTENT, MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget,
    decode_use_result, encode_item_use_intent,
};
use std::sync::Arc;

const CREATURE: &str = "oteryn:creature.squidgy_slime";
const TRACK: &str = "oteryn:quest-progress/quest/u8_7/spirit_hunters/tomb_use";
const TOOL: &str = "oteryn:item.tibia.i4049";
const GRAVE: Position = Position {
    x: 33210,
    y: 31670,
    floor: -7,
};

#[test]
fn item_on_map_use_commits_progress_and_spawns_once_without_relocation_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run("quest-tool-spawn", || {
        run(async |admin| {
            reset();
            let h =
                Harness::create_with_admission_seed(admin, "quest_tool_spawn", true, false).await?;
            prepare_current_session(&h).await?;
            let mut deps = Dependencies::new()?;
            let (controller, activated) =
                super::death_ml_loss_composed::production_generation_with_pin(
                    deps.admitted.world_id,
                    "content-1",
                )?;
            let generation = controller.active().ok_or("active generation")?;
            let native = generation.native_gameplay().ok_or("native profiles")?;
            let start = (33211_u16, 31670_u16);
            let world = super::l6_world_pickups::native_crop(
                native,
                super::l6_world_pickups::NativeCrop {
                    bytes: include_bytes!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/../../../content/world/placements/region-z07-x129-y123.b3"
                    )),
                    region: (7, 129, 123),
                    bounds: (33210, 31670, 33214, 31671),
                    start,
                    expected: vec![super::l6_world_pickups::ExpectedTile {
                        position: (GRAVE.x, GRAVE.y),
                        ids: vec![992, 1979],
                        counted_entry: None,
                    }],
                },
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
                        x: i32::from(start.0),
                        y: i32::from(start.1),
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
                (i32::from(start.0), i32::from(start.1), 7),
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
                        x: i32::from(start.0),
                        y: i32::from(start.1),
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
            let loaded =
                oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                    .map_err(debug)?;
            owner.quest_catalogue = Some(Arc::new(loaded.catalogue().clone()));
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
            deps.initialize(&owner).await;
            owner.admit_quest_session(&admitted).await;

            admitted.continuity.selected_capabilities =
                crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                    owner.offered_capabilities(),
                    &[4, 6, 11, 15, 18],
                )
                .ok_or("USE capabilities")?;
            super::quest_world_callbacks::input_item(
                &owner,
                fence.item_fence(),
                &Offering {
                    position: GRAVE,
                    item: TOOL.into(),
                    count: 1,
                },
                951,
                true,
            )
            .await?;
            let payload = {
                let map = owner.bundle_map().ok_or("grave map")?;
                let key =
                    oteryn_world_bundle::bundle::placement_key(GRAVE.floor, GRAVE.x, GRAVE.y, 1)
                        .ok_or("grave placement")?;
                let mut placement = map.overlay.base().digest().to_vec();
                placement.extend_from_slice(&key.to_be_bytes());
                encode_item_use_intent(&UseIntent {
                    target: UseTarget::ItemByDefinition(
                        native
                            .item_index()
                            .and_then(|index| index.definition_ref("Item", TOOL, "definition-r1"))
                            .ok_or("tool definition")?,
                    ),
                    use_with: None,
                    use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
                        placement,
                        expected_revision: map.facts.object_revision(key),
                    })),
                    use_with_position: None,
                })
                .map_err(debug)?
            };
            let command = admitted.continuity.next_command_id;
            for attempt in ["fresh USE", "replayed USE"] {
                let reply = super::quest_world_callbacks::wire(
                    &owner,
                    &mut admitted,
                    command,
                    COMMAND_TYPE_USE_INTENT,
                    payload.clone(),
                )
                .await?;
                assert_eq!(
                    decode_use_result(&reply).map_err(debug)?,
                    UseDisposition::Committed,
                    "{attempt}"
                );
            }
            let state = h
                .root
                .read_character_quest_state(&authority, fence.character_id)
                .await
                .map_err(debug)?;
            assert_eq!(state.tracks().get(TRACK), Some(&1));
            {
                let runtime = owner.runtime.lock().await;
                let actors = runtime.positioned_actor_census().map_err(debug)?;
                assert_eq!(actors.len(), 2, "replay must not spawn another creature");
                assert!(actors.iter().any(|(actor, position, _)| {
                    *actor == deps.actor
                        && position.position()
                            == MovementLocalPosition {
                                x: i32::from(start.0),
                                y: i32::from(start.1),
                                floor: 7,
                            }
                }));
                assert!(actors.iter().any(|(actor, position, session)| {
                    session.is_none()
                        && runtime.matches_live_creature_identity(*actor, CREATURE.as_bytes())
                        && position.position()
                            == MovementLocalPosition {
                                x: i32::from(start.0) + 1,
                                y: i32::from(start.1),
                                floor: 7,
                            }
                }));
            }
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(2))
                .ok_or("pace second USE")?;
            let reply = super::quest_world_callbacks::wire(
                &owner,
                &mut admitted,
                command + 1,
                COMMAND_TYPE_USE_INTENT,
                payload,
            )
            .await?;
            assert_eq!(
                decode_use_result(&reply).map_err(debug)?,
                UseDisposition::Committed
            );
            let state = h
                .root
                .read_character_quest_state(&authority, fence.character_id)
                .await
                .map_err(debug)?;
            assert_eq!(state.tracks().get(TRACK), Some(&2));
            {
                let runtime = owner.runtime.lock().await;
                let actors = runtime.positioned_actor_census().map_err(debug)?;
                let east = MovementLocalPosition {
                    x: i32::from(start.0) + 1,
                    y: i32::from(start.1),
                    floor: 7,
                };
                assert_eq!(
                    actors
                        .iter()
                        .filter(|(actor, position, _)| {
                            let position = position.position();
                            runtime.matches_live_creature_identity(*actor, CREATURE.as_bytes())
                                && position.floor == east.floor
                                && position.x.abs_diff(east.x) <= 1
                                && position.y.abs_diff(east.y) <= 1
                                && position != east
                        })
                        .count(),
                    1,
                    "occupied east tile must use a neighbouring cell"
                );
            }
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}
