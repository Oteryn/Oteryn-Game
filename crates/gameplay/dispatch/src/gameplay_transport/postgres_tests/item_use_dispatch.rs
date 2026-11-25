//! Direct USE of a fresh map handle and a carried container through command 2.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, item_look_fixtures, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, IDLE_LIVENESS, serve_admitted};
use crate::map::boot::{BootPins, boot, tests as bundle_fixture};
use crate::map::facts::ItemDefinition;
use oteryn_protocol_oteryn::{self as protocol, world_object::UseDisposition};
use tokio::io::AsyncWriteExt;

#[test]
fn direct_use_opens_fresh_map_rack() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("use-map-rack", || {
        scenario("oteryn:item.tibia.i10210", Some(25))
    })
}

#[test]
fn direct_use_opens_carried_backpack() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("use-carried-bag", || {
        scenario("oteryn:item.tibia.i10326", Some(20))
    })
}

#[test]
fn direct_use_consumes_admitted_crocodile_steak() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("use-crocodile-steak-mode=seeded", || {
        scenario("oteryn:item.tibia.i10219", None)
    })
}

fn scenario(key: &'static str, expected_capacity: Option<u8>) -> TestResult {
    let map = expected_capacity == Some(25);
    run(async move |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "item_use_dispatch", true, false).await?;
        let result: TestResult = async {
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut deps = Dependencies::new()?;
            let (controller, activated_pin) =
                super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                    deps.admitted.world_id,
                    "content-1",
                )?;
            let generation = controller.active().ok_or("active generation")?;
            let native = generation.native_gameplay().ok_or("native")?;
            let index = native.item_index().ok_or("native Item index")?;
            let container_key = "oteryn:item.tibia.i10210";
            let leaf_key = "oteryn:item.tibia.i3003";
            let reference = |key| {
                index.definition_ref("Item", key, "definition-r1").ok_or("fixture Item reference")
            };
            let (bytes, mut pins) = bundle_fixture::bundle_with_item_ids(
                (container_key, reference(container_key)?.get() - 1),
                (leaf_key, reference(leaf_key)?.get() - 1),
            );
            let mut bundle = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
            let rack_tile = bundle.sectors[0].tiles.iter_mut().find(|tile| tile.x == 6).ok_or("rack tile")?;
            rack_tile.items.push(oteryn_world_bundle::sector::Item {
                palette: 4,
                depth: 1,
                attrs: Default::default(),
            });
            let bytes = oteryn_world_bundle::writer::write(&bundle.manifest, &bundle.sectors, &bundle.spawns).map_err(debug)?;
            pins.digest = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?.digest;
            let base = crate::map::load(&bytes, &pins).map_err(debug)?;
            let world = boot(
                &bytes,
                &BootPins {
                    map_revision: crate::map::overlay::map_revision(&base),
                    bundle: pins,
                    start: crate::map::overlay::TilePos {
                        x: 7,
                        y: 0,
                        floor: -7,
                    },
                },
                deps.admitted.world_id,
                deps.admitted.channel_id,
                |key| {
                    Some(ItemDefinition {
                        reference: index.definition_ref("Item", key, "definition-r1")?,
                        solid: Some(key == container_key),
                        blocks_projectile: false,
                        pickupable: key == leaf_key,
                        door: false,
                    })
                },
            )
            .map_err(debug)?;
            let cells = world
                .movement_cells(deps.room.movement_cells())
                .map_err(debug)?;
            let pin = ChannelContentPin::from_activation(
                deps.admitted.world_id,
                1,
                generation.identity().server_artifact_digest(),
                generation.identity().client_artifact_digest(),
                activated_pin.frame_binding().digest(),
                world.map_revision_digest(),
                (7, 0, 7),
            );
            let mut runtime = ChannelRuntimeV1::from_committed_assignment(
                deps.admitted.world_id,
                deps.admitted.channel_id,
                crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
                1,
                1,
                1,
                "runtime-scope-assignment:1",
                2,
                pin,
            )
            .map_err(debug)?;
            let reservation = runtime
                .reserve_fresh_session(deps.admitted.game_session_id)
                .map_err(debug)?;
            deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
            deps.admitted.runtime_actor = Some(deps.actor);
            runtime
                .initialize_pinned_test_position(
                    deps.actor,
                    MovementLocalPosition {
                        x: 7,
                        y: 0,
                        floor: 7,
                    },
                )
                .map_err(debug)?;
            deps.runtime = tokio::sync::Mutex::new(runtime);
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(generation);
            owner.movement_cells = (&cells).into();
            deps.initialize(&owner).await;
            let session = deps.admitted.game_session_id;
            let current =
                crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                    .current_session_at(session)
                    .await?
                    .0;
            let fence = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice()).bind(generation.identity().client_artifact_digest().as_slice()).bind(activated_pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;

            let definition = oteryn_gameplay_domain::domain::item_definition::TypedDefinitionRef {
                family: "Item".into(),
                production_key: key.into(),
                revision_ref: "definition-r1".into(),
            };
            if !map {
                let facts = oteryn_gameplay_items::item_ref_admission::generation_item_facts(native, &definition).map_err(debug)?;
                assert_ne!(facts.stack, oteryn_gameplay_domain::domain::item_definition::ItemStackClass::Unknown);
            }
            item_look_fixtures::seed_main_backpack(&h, &[(key, 1)]).await?;
            let mut admitted = deps.admitted;
            admitted.item_fence = Some(fence);
            admitted.first_entry = FirstEntryOutcome::Positioned;
            admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
                &[4, 6, 12, 14, 15, 18, 22, 49, 445, 446],
            ).ok_or("capabilities")?;
            let target = reference(key)?;
            // mode=seeded gives the actor one steak and leaves its food condition empty.
            let steak_quantity_and_custody = || async {
                sqlx::query_as::<_, (i64, bool)>(
                    "SELECT item.quantity, EXISTS (
                        SELECT 1 FROM game_item_container_entries AS entry
                        WHERE entry.item_instance_id = item.item_instance_id
                          AND entry.character_id = encode($1, 'hex')::uuid
                    )
                    FROM game_item_instances AS item
                    WHERE item.definition_production_key = $2",
                )
                .bind(id(41).to_vec())
                .bind(key)
                .fetch_one(&h.pool)
                .await
            };
            use oteryn_gameplay_domain::domain::condition::ConflictKey;
            let food_regeneration = async || -> TestResult<Option<i32>> {
                let runtime = deps.runtime.lock().await;
                let states = owner.states.spell_states.lock().await;
                let state = states.get(&runtime, deps.actor, session).ok_or("actor spell state")?;
                let Some(condition) = state
                    .owned_conditions()
                    .active_at(ConflictKey::FoodRegeneration, owner.owner_now().get())
                else {
                    return Ok(None);
                };
                let duration = condition.remaining_duration_ms(condition.started_at())
                    .ok_or("food condition duration")?;
                Ok(Some(duration))
            };
            if expected_capacity.is_none() {
                assert_eq!(steak_quantity_and_custody().await?, (1, true), "seeded steak missing");
                assert_eq!(food_regeneration().await?, None, "actor must start unfed");
            }
            let (mut server, mut client) = tokio::io::duplex(1 << 20);
            let serve = serve_admitted(&mut server, admitted, &owner, IDLE_LIVENESS);
            let exercise = async {
                let mut handle = None;
                loop {
                    let frame = item_look_fixtures::read_frame(&mut client).await?;
                    let envelope = protocol::decode_wire_envelope(&frame).map_err(debug)?;
                    if envelope.message_type() == protocol::MessageType::SnapshotCommit {
                        break;
                    }
                    if envelope.message_type() != protocol::MessageType::SnapshotChunk {
                        continue;
                    }
                    let (_, domains) = protocol::decode_snapshot_chunk(envelope.payload()).map_err(debug)?;
                    for domain in domains {
                        if map && domain.domain_id == protocol::world_map::STATE_DOMAIN_WORLD_MAP_VIEW {
                            let snapshot = protocol::world_map::decode_world_map_snapshot(domain.payload).map_err(debug)?;
                            handle = snapshot.tiles.iter().flat_map(|tile| &tile.items).find_map(|item| {
                                match item.origin {
                                    protocol::world_map::MapOrigin::Handle(handle) if item.definition == protocol::world_map::MapDefinition::Item(target) => Some(handle),
                                    _ => None,
                                }
                            }).or(handle);
                        } else if !map && domain.domain_id == 9 {
                            let inventory = protocol::item_view::decode_character_inventory(domain.payload).map_err(debug)?;
                            handle = inventory.entries.iter().find(|item| item.item_definition_ref == target).map(|item| item.handle).or(handle);
                        }
                    }
                }
                let handle = handle.ok_or("join snapshot must publish the item handle")?;
                let payload = protocol::world_object::encode_use_item_intent(handle);
                let frame = protocol::encode_client_command(1, &protocol::ClientCommandValue {
                    command_id: 1,
                    command_type: 2,
                    payload: &payload,
                }).map_err(debug)?;
                client.write_u32(u32::try_from(frame.len())?).await?;
                client.write_all(&frame).await?;
                let mut accepted = false;
                let mut capacity = None;
                while !accepted || (expected_capacity.is_some() && capacity.is_none()) {
                    let frame = item_look_fixtures::read_frame(&mut client).await?;
                    let envelope = protocol::decode_wire_envelope(&frame).map_err(debug)?;
                    match envelope.message_type() {
                        protocol::MessageType::CommandResult => {
                            let reply = protocol::decode_command_result(envelope.payload()).map_err(debug)?;
                            assert_eq!(reply.command_id, 1);
                            let disposition = protocol::world_object::decode_use_result(reply.payload).map_err(debug)?;
                            assert_eq!((reply.status, disposition), (protocol::CommandStatus::Accepted, UseDisposition::Committed));
                            accepted = true;
                        }
                        protocol::MessageType::StateDelta => {
                            let delta = protocol::decode_state_delta(envelope.payload()).map_err(debug)?;
                            if delta.domain_id == 14 {
                                let views = protocol::container_tree::decode_container_views_with_house_capacity(delta.payload, false, true).map_err(debug)?;
                                capacity = views.views.iter().find(|view| view.container_handle == handle).map(|view| view.capacity);
                            }
                        }
                        _ => {}
                    }
                }
                assert_eq!(capacity, expected_capacity);
                if expected_capacity.is_none() {
                    assert_eq!(steak_quantity_and_custody().await?, (0, false), "steak must be consumed in PostgreSQL");
                    // Crystal 201189f8 foods.lua:88,150–154 adds 10 × 12 seconds and removes one steak.
                    assert_eq!(food_regeneration().await?, Some(120_000));
                }
                client.shutdown().await?;
                Ok::<_, Box<dyn std::error::Error>>(())
            };
            tokio::time::timeout(std::time::Duration::from_secs(45), async {
                tokio::select! {
                    _ = serve => Err("connection closed before USE completed".into()),
                    result = exercise => result,
                }
            }).await?
        }.await;
        h.cleanup().await?;
        result
    })
}
