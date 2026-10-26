//! QUESTCB2: literal commands exercise the real callback, map, quest and item owners.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::{
    ChannelRuntimeV1, MessageType, MovementLocalPosition, decode_wire_envelope,
};
use crate::gameplay_transport::connection::{AdmittedSession, FirstEntryOutcome};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use std::collections::BTreeMap;

fn callback(name: &str) -> TestResult<Callback> {
    oteryn_gameplay_quests::quest::world_callbacks::load()?
        .into_iter()
        .find(|c| c.key.ends_with(name))
        .ok_or_else(|| "callback".into())
}
fn local(p: Position) -> MovementLocalPosition {
    MovementLocalPosition {
        x: i32::from(p.x),
        y: i32::from(p.y),
        floor: -i16::from(p.floor),
    }
}

async fn offering(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    offering: &oteryn_gameplay_quests::quest::world_callbacks::Offering,
    n: u64,
) -> TestResult {
    use crate::durability::{
        item_loop_move::{LoopDestination, LoopGround, LoopMoveRequest},
        item_mint::{
            GroundPlacement, ItemMintCause, ItemMintOutcome, ItemMintRequest, TypedDefinitionRef,
        },
        item_transfer::{ItemTransferDestination, ItemTransferRequest},
    };
    use crate::gameplay_transport::item_ref_admission::{NativeItemWeights, generation_item_facts};
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let definition = TypedDefinitionRef {
        family: "Item".into(),
        production_key: offering.item.clone(),
        revision_ref: "definition-r1".into(),
    };
    let facts = generation_item_facts(native, &definition).map_err(debug)?;
    let map = owner.bundle_map().ok_or("map")?;
    let mut candidate = owner
        .root
        .freeze_item_mint(
            owner.character,
            owner.holder,
            ItemMintRequest {
                cause: ItemMintCause::for_test(
                    owner.world_id,
                    owner.channel_id,
                    fence.scope_ownership_generation,
                    1,
                    n,
                    TypedDefinitionRef {
                        family: "LootTable".into(),
                        production_key: "fixture:quest-callbacks".into(),
                        revision_ref: "definition-r1".into(),
                    },
                    "fixture:quest-callbacks".into(),
                    0,
                ),
                item: definition,
                quantity: u32::from(offering.count),
                ground: GroundPlacement {
                    spatial_position:
                        crate::durability::map_item_mint_audit::map_item_spatial_position(
                            offering.position.x,
                            offering.position.y,
                            offering.position.floor,
                        ),
                    corpse_ref: id(3).to_vec(),
                    map_revision: crate::map::overlay::map_revision(map.overlay.base()),
                    content_revision: "content-1".into(),
                    native_room_placement_context: vec![1],
                },
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            },
        )
        .await
        .map_err(debug)?;
    let permit = owner.spell_lane_permit().await.ok_or("lane")?;
    let ItemMintOutcome::Committed(item) = owner
        .root
        .commit_item_mint(&permit, owner.character, owner.holder, &mut candidate)
        .await
        .map_err(debug)?
    else {
        return Err("mint".into());
    };
    let command = |offset| -> TestResult<_> {
        Ok(crate::foundation::CommandRef::new(
            fence.game_session_id,
            crate::foundation::CommandId::new(1000 + n * 10 + offset).map_err(debug)?,
        ))
    };
    let mut transfer = owner
        .root
        .freeze_item_transfer(
            owner.character,
            owner.holder,
            fence,
            ItemTransferRequest {
                weight_generation_digest: native.source_digest(),
                command: command(0)?,
                source_item_instance_id: item.item_instance_id,
                destination: ItemTransferDestination::MainBackpack,
                item: facts,
                backpack: Some(
                    generation_item_facts(
                        native,
                        &TypedDefinitionRef {
                            family: "Item".into(),
                            production_key: "oteryn:item.tibia.i2854".into(),
                            revision_ref: "definition-r1".into(),
                        },
                    )
                    .map_err(debug)?,
                ),
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            },
            &NativeItemWeights(native),
        )
        .await
        .map_err(debug)?;
    owner
        .root
        .commit_item_transfer(
            &permit,
            owner.character,
            owner.holder,
            fence,
            &mut transfer,
            &NativeItemWeights(native),
        )
        .await
        .map_err(debug)?;
    let (digest, frame) = {
        let r = owner.runtime.lock().await;
        (
            r.content_pin().server_artifact_digest(),
            r.content_pin().frame_binding_digest(),
        )
    };
    let revision = format!(
        "sha256:{}",
        digest
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    let moved = owner
        .root
        .commit_item_loop_move(
            &permit,
            owner.character,
            owner.holder,
            &fence,
            command(1)?,
            &NativeItemWeights(native),
            LoopMoveRequest {
                item: item.item_instance_id,
                count: None,
                pickup_cell: None,
                destination: LoopDestination::Ground(LoopGround {
                    spatial_position:
                        crate::durability::map_item_mint_audit::map_item_spatial_position(
                            offering.position.x,
                            offering.position.y,
                            offering.position.floor,
                        ),
                    map_revision: crate::map::overlay::map_revision(map.overlay.base()),
                    content_revision: revision,
                    placement_context: frame.to_vec(),
                }),
            },
        )
        .await
        .map_err(debug)?;
    assert!(moved.is_ok(), "drop: {moved:?}");
    Ok(())
}

fn world(callback: &Callback, trigger: Position) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::{bundle, sector};
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.truncate(1);
    let mut cells = BTreeMap::new();
    for x in trigger.x - 2..=trigger.x + 8 {
        for y in trigger.y - 1..=trigger.y + 1 {
            cells.insert((trigger.floor, x, y), Vec::new());
        }
    }
    for position in callback
        .passages
        .iter()
        .map(|p| p.position)
        .chain(callback.offerings.iter().map(|p| p.position))
        .chain(callback.teleport)
    {
        cells
            .entry((position.floor, position.x, position.y))
            .or_default();
    }
    let appearance = callback.selector.appearance.unwrap_or(100);
    data.manifest.palette.push(bundle::PaletteEntry {
        key: format!("oteryn:terrain.tibia.i{appearance}"),
        family: bundle::Family::Terrain,
        id: 1,
        terrain: Some(bundle::Terrain {
            kind: bundle::TerrainKind::Common,
            walkable: None,
            ground_speed: None,
        }),
    });
    cells
        .entry((trigger.floor, trigger.x, trigger.y))
        .or_default()
        .push(sector::Item {
            palette: 1,
            depth: 0,
            attrs: sector::Attrs {
                unique: callback.selector.unique,
                action: callback.selector.action,
                ..Default::default()
            },
        });
    for passage in &callback.passages {
        let appearance_key = format!("oteryn:terrain.tibia.i{}", passage.appearance);
        let index = if let Some(i) = data
            .manifest
            .palette
            .iter()
            .position(|p| p.key == appearance_key)
        {
            u32::try_from(i)?
        } else {
            let index = u32::try_from(data.manifest.palette.len())?;
            data.manifest.palette.push(bundle::PaletteEntry {
                key: appearance_key,
                family: bundle::Family::Terrain,
                id: index,
                terrain: Some(bundle::Terrain {
                    kind: bundle::TerrainKind::Wall,
                    walkable: None,
                    ground_speed: None,
                }),
            });
            index
        };
        if let Some(replacement) = passage.replacement {
            let replacement_key = format!("oteryn:terrain.tibia.i{replacement}");
            if !data
                .manifest
                .palette
                .iter()
                .any(|p| p.key == replacement_key)
            {
                let id = u32::try_from(data.manifest.palette.len())?;
                data.manifest.palette.push(bundle::PaletteEntry {
                    key: replacement_key,
                    family: bundle::Family::Terrain,
                    id,
                    terrain: Some(bundle::Terrain {
                        kind: bundle::TerrainKind::Ground,
                        walkable: Some(true),
                        ground_speed: Some(150),
                    }),
                });
            }
        }
        cells
            .entry((
                passage.position.floor,
                passage.position.x,
                passage.position.y,
            ))
            .or_default()
            .push(sector::Item {
                palette: index,
                depth: 0,
                attrs: Default::default(),
            });
    }
    let floors: std::collections::BTreeSet<_> = cells.keys().map(|(f, _, _)| *f).collect();
    data.manifest.world = bundle::Extent {
        min_x: cells.keys().map(|(_, x, _)| *x).min().ok_or("min")?,
        min_y: cells.keys().map(|(_, _, y)| *y).min().ok_or("min")?,
        max_x: cells.keys().map(|(_, x, _)| *x).max().ok_or("max")? + 1,
        max_y: cells.keys().map(|(_, _, y)| *y).max().ok_or("max")? + 1,
        floors: floors.into_iter().collect(),
    };
    let mut sectors: BTreeMap<(i8, u16, u16), Vec<sector::Tile>> = BTreeMap::new();
    for ((floor, x, y), mut items) in cells {
        items.insert(
            0,
            sector::Item {
                palette: 0,
                depth: 0,
                attrs: Default::default(),
            },
        );
        sectors
            .entry((floor, x / 32, y / 32))
            .or_default()
            .push(sector::Tile {
                x,
                y,
                flags: 0,
                house: 0,
                zones: vec![],
                items,
            });
    }
    let sectors = sectors
        .into_iter()
        .map(|((floor, sx, sy), mut tiles)| {
            tiles.sort_by_key(|t| (t.y, t.x));
            bundle::Sector {
                floor,
                sx,
                sy,
                tiles,
            }
        })
        .collect::<Vec<_>>();
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: trigger.x - 1,
                y: trigger.y,
                floor: trigger.floor,
            },
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |_| None,
    )
    .map_err(debug)
}

async fn wire(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    id: u64,
    kind: u32,
    payload: Vec<u8>,
) -> TestResult<Vec<u8>> {
    use crate::gameplay_transport::connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let frame = oteryn_protocol_oteryn::encode_client_command(
        1,
        &oteryn_protocol_oteryn::ClientCommandValue {
            command_id: id,
            command_type: kind,
            payload: &payload,
        },
    )
    .map_err(debug)?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    client
        .write_all(&u32::try_from(frame.len())?.to_be_bytes())
        .await?;
    client.write_all(&frame).await?;
    // Keep the transport alive while a paced command is buffered. Closing its write half
    // before the result correctly disconnects a real client before the queued step runs.
    let receiving = async move {
        let mut result = None;
        loop {
            let mut prefix = [0; 4];
            match client.read_exact(&mut prefix).await {
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(error) => return Err(error.into()),
            }
            let mut frame = vec![0; u32::from_be_bytes(prefix) as usize];
            client.read_exact(&mut frame).await?;
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            if envelope.message_type() == MessageType::CommandResult {
                let command = oteryn_protocol_oteryn::decode_command_result(envelope.payload())
                    .map_err(debug)?;
                assert_eq!(command.command_id, id);
                result = Some(command.payload.to_vec());
                client.shutdown().await?;
            }
        }
        result.ok_or_else(|| "no command result".into())
    };
    let reader = tokio::spawn(async move {
        let result: TestResult<Vec<u8>> = receiving.await;
        result.map_err(|error| error.to_string())
    });
    let end = serve_admitted(&mut server, *admitted, owner, IDLE_LIVENESS).await;
    drop(server);
    let result = reader.await?.map_err(Into::into);
    if let ConnectionEnd::AdmittedThenDisconnected(a) | ConnectionEnd::AdmittedThenClosed(a, _) =
        end
    {
        *admitted = a;
    }
    result
}

fn scenario(name: &str, trigger: Position, lever_fixture: bool) -> TestResult {
    let mut callback = callback(name)?;
    if lever_fixture {
        callback.trigger = oteryn_gameplay_quests::quest::world_callbacks::Trigger::Use;
        callback.selector.unique = Some(25012);
        callback.selector.appearance = Some(2772);
        callback.lever = Some([2772, 2773]);
    }
    let world = world(&callback, trigger)?;
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "quest_world_callbacks", false, false)
            .await?;
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
            2,
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        let origin = local(Position {
            x: trigger.x - 1,
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
        if lever_fixture {
            assert!(
                owner
                    .states
                    .quest_callbacks
                    .fixture
                    .set(vec![callback.clone()])
                    .is_ok()
            );
        }
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
        if !lever_fixture && !callback.offerings.is_empty() {
            for (n, altar) in callback.offerings.iter().take(3).enumerate() {
                offering(&owner, fence.item_fence(), altar, u64::try_from(n)? + 1).await?;
            }
            let east = oteryn_protocol_oteryn::world_spatial::encode_step_intent(
                oteryn_protocol_oteryn::world_spatial::StepDirection::East,
            );
            wire(&owner, &mut admitted, 1, 1, east.clone()).await?;
            assert_eq!(
                deps.runtime
                    .lock()
                    .await
                    .read_actor_position(deps.actor)
                    .map_err(debug)?
                    .position(),
                local(trigger)
            );
            let burned: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(burned, 0);
            offering(&owner, fence.item_fence(), &callback.offerings[3], 4).await?;
            wire(
                &owner,
                &mut admitted,
                2,
                1,
                oteryn_protocol_oteryn::world_spatial::encode_step_intent(
                    oteryn_protocol_oteryn::world_spatial::StepDirection::West,
                ),
            )
            .await?;
            wire(&owner, &mut admitted, 3, 1, east).await?;
            assert_eq!(
                deps.runtime
                    .lock()
                    .await
                    .read_actor_position(deps.actor)
                    .map_err(debug)?
                    .position(),
                local(callback.teleport.ok_or("teleport")?)
            );
            let burned: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(burned, 4);
            let copy = h
                .root
                .read_character_quest_state(&authority, fence.character_id)
                .await
                .map_err(debug)?;
            assert_eq!(copy.tracks().get(&callback.storage[0].track), Some(&1));
        } else if name == "liquid-black-shortcut" {
            // The source shortcut has no text. Exercise the generic authored refusal message
            // without changing that source binding in production content.
            let mut message_callback = callback.clone();
            message_callback.failure_message = Some("Finish the visitor mission first.".into());
            assert!(
                owner
                    .states
                    .quest_callbacks
                    .fixture
                    .set(vec![message_callback])
                    .is_ok()
            );
            owner
                .states
                .chat
                .admit(admitted.game_session_id, "Callback", owner.owner_now());
            let step = oteryn_protocol_oteryn::world_spatial::encode_step_intent(
                oteryn_protocol_oteryn::world_spatial::StepDirection::East,
            );
            wire(&owner, &mut admitted, 1, 1, step.clone()).await?;
            assert!(owner.states.chat.drain(admitted.game_session_id).iter().any(|line|
                matches!(line, oteryn_protocol_oteryn::chat::ChatLine::Private { text, .. } if text == "Finish the visitor mission first.")));
            assert_eq!(
                deps.runtime
                    .lock()
                    .await
                    .read_actor_position(deps.actor)
                    .map_err(debug)?
                    .position(),
                origin
            );
            let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
            for (n, key) in ["npc_1", "action_2", "action_1", "npc_2"]
                .into_iter()
                .enumerate()
            {
                use crate::durability::quest_state::{
                    QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
                };
                let request = QuestTransitionRequest {
                    transition_key: format!(
                        "oteryn:quest-transition/quest/u9_4/liquid_black_quest/visitor/{key}"
                    ),
                    cause: QuestCause::Command(crate::foundation::CommandRef::new(
                        admitted.game_session_id,
                        crate::foundation::CommandId::new(100 + u64::try_from(n)?)
                            .map_err(debug)?,
                    )),
                };
                assert!(matches!(
                    slot.commit_quest_transition(
                        owner.root,
                        owner.character,
                        owner.holder,
                        fence,
                        request,
                        owner.quest_catalogue.clone().ok_or("catalogue")?
                    )
                    .await
                    .map_err(debug)?,
                    QuestTransitionOutcome::Committed(_)
                ));
            }
            drop(slot);
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(2))
                .ok_or("clock")?;
            wire(&owner, &mut admitted, 2, 1, step).await?;
            assert_eq!(
                deps.runtime
                    .lock()
                    .await
                    .read_actor_position(deps.actor)
                    .map_err(debug)?
                    .position(),
                local(callback.teleport.ok_or("teleport")?)
            );
            let copy = h
                .root
                .read_character_quest_state(&authority, fence.character_id)
                .await
                .map_err(debug)?;
            assert_eq!(copy.tracks().get(&callback.storage[0].track), Some(&5));
        } else {
            let key =
                oteryn_world_bundle::bundle::placement_key(trigger.floor, trigger.x, trigger.y, 1)
                    .ok_or("key")?;
            let target = |revision| {
                let mut placement = world.base().digest().to_vec();
                placement.extend_from_slice(&key.to_be_bytes());
                WorldObjectTarget {
                    placement,
                    expected_revision: revision,
                }
            };
            if lever_fixture {
                for (n, altar) in callback.offerings.iter().take(3).enumerate() {
                    offering(&owner, fence.item_fence(), altar, u64::try_from(n)? + 1).await?;
                }
            }
            let result = wire(
                &owner,
                &mut admitted,
                1,
                2,
                oteryn_protocol_oteryn::world_object::encode_use_intent(&target(0))
                    .map_err(debug)?,
            )
            .await?;
            let expected = if lever_fixture {
                UseDisposition::Rejected
            } else {
                UseDisposition::Committed
            };
            assert_eq!(
                oteryn_protocol_oteryn::world_object::decode_use_result(&result).map_err(debug)?,
                expected
            );
            if lever_fixture {
                let untouched: i64 =
                    sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions")
                        .fetch_one(&h.pool)
                        .await?;
                assert_eq!(untouched, 0);
                let copy = h
                    .root
                    .read_character_quest_state(&authority, fence.character_id)
                    .await
                    .map_err(debug)?;
                assert!(!copy.tracks().contains_key(&callback.storage[0].track));
                offering(&owner, fence.item_fence(), &callback.offerings[3], 4).await?;
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(2))
                    .ok_or("clock")?;
                let context = {
                    let runtime = deps.runtime.lock().await;
                    crate::durability::quest_world_callback::QuestWorldContext {
                        map_revision: crate::map::overlay::map_revision(world.base()),
                        content_revision: format!(
                            "sha256:{}",
                            runtime
                                .content_pin()
                                .server_artifact_digest()
                                .iter()
                                .map(|b| format!("{b:02x}"))
                                .collect::<String>()
                        ),
                        placement_context: runtime.content_pin().frame_binding_digest().to_vec(),
                        reset_epoch: owner.reset_epoch,
                        actor_placement: deps.actor.placement_identity(),
                        movement_revision: runtime
                            .read_actor_position(deps.actor)
                            .map_err(debug)?
                            .revision(),
                    }
                };
                owner
                    .states
                    .quest_callbacks
                    .interrupt_after_commit
                    .store(true, std::sync::atomic::Ordering::Release);
                let result = wire(
                    &owner,
                    &mut admitted,
                    2,
                    2,
                    oteryn_protocol_oteryn::world_object::encode_use_intent(&target(0))
                        .map_err(debug)?,
                )
                .await;
                assert!(
                    result.is_err(),
                    "an unknown result must close without an acknowledgement"
                );
                assert!(owner.has_pending_quest_callback(admitted.game_session_id));
                assert_eq!(
                    deps.runtime
                        .lock()
                        .await
                        .read_actor_position(deps.actor)
                        .map_err(debug)?
                        .position(),
                    origin
                );
                let burned: i64 =
                    sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions")
                        .fetch_one(&h.pool)
                        .await?;
                assert_eq!(burned, 4);
                // Another session must not settle or project this disconnected player's
                // pending callback when its own command stream opens.
                let mut other = admitted;
                other.game_session_id =
                    crate::foundation::GameSessionId::decode(&id(51)).map_err(debug)?;
                other.continuity =
                    crate::gameplay_transport::connection::SessionContinuity::FRESH;
                other.item_fence = None;
                {
                    let mut runtime = deps.runtime.lock().await;
                    let reservation = runtime
                        .reserve_fresh_session(other.game_session_id)
                        .map_err(debug)?;
                    let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
                    runtime
                        .initialize_pinned_test_position(
                            actor,
                            local(Position {
                                x: trigger.x - 2,
                                ..trigger
                            }),
                        )
                        .map_err(debug)?;
                    other.runtime_actor = Some(actor);
                }
                assert!(wire(&owner, &mut other, 1, 999, Vec::new()).await?.is_empty());
                assert!(owner.has_pending_quest_callback(admitted.game_session_id));
                assert_eq!(
                    deps.runtime
                        .lock()
                        .await
                        .read_actor_position(deps.actor)
                        .map_err(debug)?
                        .position(),
                    origin
                );
                // The next real dispatch reconciles the original USE before allowing input.
                let replay = wire(
                    &owner,
                    &mut admitted,
                    2,
                    2,
                    oteryn_protocol_oteryn::world_object::encode_use_intent(&target(0))
                        .map_err(debug)?,
                )
                .await?;
                assert_eq!(
                    oteryn_protocol_oteryn::world_object::decode_use_result(&replay)
                        .map_err(debug)?,
                    UseDisposition::Committed
                );
                assert!(!owner.has_pending_quest_callback(admitted.game_session_id));
                use crate::durability::quest_state::{
                    QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
                };
                let request = QuestTransitionRequest {
                    transition_key: callback.key.clone(),
                    cause: QuestCause::Use(crate::foundation::CommandRef::new(
                        admitted.game_session_id,
                        crate::foundation::CommandId::new(2).map_err(debug)?,
                    )),
                };
                let catalogue = owner.quest_catalogue.clone().ok_or("catalogue")?;
                let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
                assert!(matches!(
                    slot.commit_world_callback(
                        owner.root,
                        owner.character,
                        owner.holder,
                        fence,
                        request.clone(),
                        catalogue.clone(),
                        context.clone()
                    )
                    .await
                    .map_err(debug)?,
                    QuestTransitionOutcome::AlreadyCommitted(_)
                ));
                // A generic quest caller cannot bypass the physical altar proof.
                assert!(matches!(slot.commit_quest_transition(owner.root, owner.character, owner.holder,
                    fence, request.clone(), catalogue.clone()).await,
                    Err(crate::durability::character_progression::CharacterProgressionError::InvalidInput)));
                let mut stale = fence;
                stale.character_lease_generation += 1;
                let fresh_request = QuestTransitionRequest {
                    cause: QuestCause::Use(crate::foundation::CommandRef::new(
                        admitted.game_session_id,
                        crate::foundation::CommandId::new(999).map_err(debug)?,
                    )),
                    ..request
                };
                assert!(matches!(slot.commit_world_callback(owner.root, owner.character, owner.holder,
                    stale, fresh_request, catalogue, context).await,
                    Err(crate::durability::character_progression::CharacterProgressionError::AuthorityRejected)));
                drop(slot);
                let (proofs, receipts, remaining): (i64, i64, i64) = sqlx::query_as(
                    "SELECT (SELECT count(*) FROM game_quest_callback_consumptions), \
                     (SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1), \
                     (SELECT count(*) FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key=$2)")
                    .bind(&callback.key).bind(&callback.offerings[0].item).fetch_one(&h.pool).await?;
                assert_eq!((proofs, receipts, remaining), (4, 1, 0));
                let copy = h
                    .root
                    .read_character_quest_state(&authority, fence.character_id)
                    .await
                    .map_err(debug)?;
                assert_eq!(copy.tracks().get(&callback.storage[0].track), Some(&1));
                assert_eq!(
                    deps.runtime
                        .lock()
                        .await
                        .read_actor_position(deps.actor)
                        .map_err(debug)?
                        .position(),
                    local(callback.teleport.ok_or("teleport")?)
                );
            }
            if !lever_fixture {
                for passage in &callback.passages {
                    assert!(world.enterable(crate::map::overlay::TilePos {
                        x: passage.position.x,
                        y: passage.position.y,
                        floor: passage.position.floor
                    }));
                }
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(2))
                    .ok_or("clock")?;
                let result = wire(
                    &owner,
                    &mut admitted,
                    2,
                    2,
                    oteryn_protocol_oteryn::world_object::encode_use_intent(&target(1))
                        .map_err(debug)?,
                )
                .await?;
                assert_eq!(
                    oteryn_protocol_oteryn::world_object::decode_use_result(&result)
                        .map_err(debug)?,
                    if callback.reversible {
                        UseDisposition::Committed
                    } else {
                        UseDisposition::NothingToUse
                    }
                );
                if callback.reversible {
                    for passage in &callback.passages {
                        assert!(!world.enterable(crate::map::overlay::TilePos {
                            x: passage.position.x,
                            y: passage.position.y,
                            floor: passage.position.floor,
                        }));
                    }
                }
            }
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

#[test]
fn quest_world_callbacks_real_use_opens_all_walls_and_toggle_restores_collision() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("quest-world-levers", || {
        scenario(
            "behemoth",
            Position {
                x: 33294,
                y: 31676,
                floor: -15,
            },
            false,
        )?;
        scenario(
            "giant-smithhammer",
            Position {
                x: 32779,
                y: 32230,
                floor: -8,
            },
            false,
        )?;
        scenario(
            "hellgate-bridge",
            Position {
                x: 32626,
                y: 31698,
                floor: -10,
            },
            false,
        )
    })
}

#[test]
fn quest_world_callbacks_real_step_checks_storage_and_teleports() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("quest-world-step", || {
        scenario(
            "liquid-black-shortcut",
            Position {
                x: 33476,
                y: 31314,
                floor: -7,
            },
            false,
        )
    })
}

#[test]
fn quest_world_callbacks_real_paradox_step_consumes_the_four_skulls() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("quest-world-paradox-step", || {
        scenario(
            "paradox-skulls",
            Position {
                x: 32564,
                y: 31958,
                floor: -1,
            },
            false,
        )
    })
}

#[test]
fn quest_world_callbacks_multi_item_lever_consumes_all_altars_and_grants_atomically() -> TestResult
{
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("quest-world-offerings", || {
        scenario(
            "paradox-skulls",
            Position {
                x: 32564,
                y: 31958,
                floor: -1,
            },
            true,
        )
    })
}

#[test]
fn quest_world_callbacks_source_binding_and_visible_collision_batch_are_atomic() -> TestResult {
    use crate::map::view::MapFacts;
    let c = callback("behemoth")?;
    let trigger = Position {
        x: 33294,
        y: 31676,
        floor: -15,
    };
    let w = world(&c, trigger)?;
    let key = oteryn_world_bundle::bundle::placement_key(trigger.floor, trigger.x, trigger.y, 1)
        .ok_or("key")?;
    assert!(
        w.facts()
            .quest_callback_matches(key, Some(30009), None, Some(2772))
    );
    assert!(
        !w.facts()
            .quest_callback_matches(key, Some(30010), None, Some(2772))
    );
    let pos = crate::map::overlay::TilePos {
        x: trigger.x,
        y: trigger.y,
        floor: trigger.floor,
    };
    let mut after = w.facts().base_entry(pos, 1, 1).ok_or("lever facts")?;
    after.appearance_id = 2773;
    let passages = c
        .passages
        .iter()
        .map(|p| {
            oteryn_world_bundle::bundle::placement_key(
                p.position.floor,
                p.position.x,
                p.position.y,
                1,
            )
            .ok_or("passage")
        })
        .collect::<Result<Vec<_>, _>>()?;
    assert!(w.facts().quest_callback_capacity(key, 0, passages.len()));
    for p in &c.passages {
        assert!(!w.enterable(crate::map::overlay::TilePos {
            x: p.position.x,
            y: p.position.y,
            floor: p.position.floor
        }));
    }
    assert!(
        w.facts()
            .apply_quest_callback(key, 0, after, &passages, &[], false)
    );
    let pinned = w.facts().pinned();
    for (&key, p) in passages.iter().zip(&c.passages) {
        assert!(pinned.base_hidden(key));
        assert!(w.enterable(crate::map::overlay::TilePos {
            x: p.position.x,
            y: p.position.y,
            floor: p.position.floor
        }));
    }
    assert!(
        !w.facts()
            .apply_quest_callback(key, 0, after, &passages, &[], true)
    );
    assert_eq!(w.facts().object_revision(key), 1);
    assert!(
        w.facts()
            .apply_quest_callback(key, 1, after, &passages, &[], true)
    );
    // A previously pinned view remains coherent while the current one closes.
    assert!(passages.iter().all(|k| pinned.base_hidden(*k)));
    assert!(passages.iter().all(|k| !w.facts().base_hidden(*k)));
    for p in &c.passages {
        assert!(!w.enterable(crate::map::overlay::TilePos {
            x: p.position.x,
            y: p.position.y,
            floor: p.position.floor
        }));
    }
    Ok(())
}
