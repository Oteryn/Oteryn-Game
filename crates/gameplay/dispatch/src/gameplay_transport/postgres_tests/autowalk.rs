//! USE-AUTOWALK-1: real session ingress, Movement, door USE and PostgreSQL map pickup.
use super::fixture::{Harness, debug, id, join_two, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS,
    serve_admitted,
};
use crate::gameplay_transport::{AutoWalkTarget, ComposedFreshAdmission};
use crate::map::boot::{BootPins, boot};
use crate::map::facts::ItemDefinition;
use crate::map::overlay::TilePos;
use oteryn_protocol_oteryn::item_view::{
    ItemMoveDestination, ItemMoveIntent, ItemMoveOutcome, ItemMoveSelection,
    encode_item_move_intent_for,
};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget, encode_use_intent};
use oteryn_protocol_oteryn::{
    ClientCommandValue, MessageType, decode_command_result, decode_snapshot_chunk,
    decode_state_delta, decode_wire_envelope, encode_client_command,
};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};

pub(super) const DOOR: &str = "oteryn:item.tibia.i1629";
pub(super) const COIN: &str = "oteryn:item.tibia.i3031";
const BAG: &str = "oteryn:item.tibia.i2854";
const MOVE_SELECTION: ItemMoveSelection = ItemMoveSelection {
    equip_drop: false,
    container_tree: false,
    world_interaction: true,
};

pub(super) fn pos(x: u16, y: u16) -> TilePos {
    TilePos { x, y, floor: -7 }
}
pub(super) fn at(x: i32, y: i32) -> MovementLocalPosition {
    MovementLocalPosition { x, y, floor: 7 }
}

pub(super) fn bundle(door: u32, coin: u32) -> TestResult<(Vec<u8>, crate::map::BundlePins)> {
    use oteryn_world_bundle::sector::{Attrs, Item, Tile};
    use oteryn_world_bundle::writer::{
        self as writer, BuildClass, Extent, Family, Identity, Manifest, PaletteEntry, Sector,
        Terrain, TerrainKind,
    };
    let manifest = Manifest {
        format: writer::FORMAT.into(),
        min_reader_version: writer::VERSION,
        projection_class: "server".into(),
        compiler_version: "engineering".into(),
        build_class: BuildClass::Production,
        identity: Identity {
            project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
            world_schema_version: "world-schema-1".into(),
            content_revision: "rev-1".into(),
            ..Identity::default()
        },
        world: Extent {
            min_x: 0,
            min_y: 0,
            max_x: 32,
            max_y: 32,
            floors: vec![-7],
        },
        palette: vec![
            PaletteEntry {
                key: "terrain:grass".into(),
                family: Family::Terrain,
                id: 0,
                terrain: Some(Terrain {
                    kind: TerrainKind::Ground,
                    walkable: Some(true),
                    ground_speed: Some(150),
                }),
            },
            PaletteEntry {
                key: "terrain:wall".into(),
                family: Family::Terrain,
                id: 1,
                terrain: Some(Terrain {
                    kind: TerrainKind::Wall,
                    walkable: None,
                    ground_speed: None,
                }),
            },
            PaletteEntry {
                key: DOOR.into(),
                family: Family::Item,
                id: door,
                terrain: None,
            },
            PaletteEntry {
                key: COIN.into(),
                family: Family::Item,
                id: coin,
                terrain: None,
            },
        ],
        draft_areas: vec![],
        skipped_provisional_keys: vec![],
        dropped_teleports: vec![],
        spawns: Default::default(),
        npcs: Default::default(),
    };
    let mut tiles = Vec::new();
    for y in 0..=6 {
        for x in 0..=14 {
            let mut palettes = vec![0];
            // One finite wall forces a detour; a second separates an unreachable room.
            if (x == 3 && y <= 2) || x == 9 {
                palettes.push(1);
            }
            if ((x == 6 || x == 12) && y == 1) || (x, y) == (1, 6) {
                palettes.push(2);
            }
            if (x, y) == (1, 5) || (x, y) == (7, 5) {
                palettes.push(3);
            }
            tiles.push(Tile {
                x,
                y,
                flags: 0,
                house: 0,
                zones: vec![],
                items: palettes
                    .into_iter()
                    .map(|palette| Item {
                        palette,
                        depth: 0,
                        attrs: Attrs::default(),
                    })
                    .collect(),
            });
        }
    }
    let bytes = writer::write(
        &manifest,
        &[Sector {
            floor: -7,
            sx: 0,
            sy: 0,
            tiles,
        }],
        &Default::default(),
    )
    .map_err(debug)?;
    let digest = writer::read(&bytes).map_err(debug)?.digest;
    Ok((
        bytes,
        crate::map::BundlePins {
            digest,
            project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
            world_schema_version: "world-schema-1".into(),
            content_revision: "rev-1".into(),
            production: false,
        },
    ))
}

#[test]
fn autowalk_real_session_use_move_cancel_and_stale_on_postgres() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "autowalk e2e panicked")?
        .map_err(Into::into)
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "autowalk", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, pin) = super::death_ml_loss_composed::production_generation_with_pin(
            deps.admitted.world_id,
            "content-1",
        )?;
        let active = controller.active().ok_or("active")?;
        deps.room = pin.qualified_room().clone();
        let native = active.native_gameplay().ok_or("native")?;
        let index = native.item_index().ok_or("item index")?;
        let reference = |key: &str| {
            index
                .definition_ref("Item", key, "definition-r1")
                .ok_or("item ref")
        };
        let (bytes, pins) = bundle(reference(DOOR)?.get() - 1, reference(COIN)?.get() - 1)?;
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let world = boot(
            &bytes,
            &BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: pins,
                start: pos(1, 1),
            },
            deps.admitted.world_id,
            deps.admitted.channel_id,
            |key| {
                Some(ItemDefinition {
                    reference: reference(key).ok()?,
                    solid: Some(key == DOOR),
                    blocks_projectile: key == DOOR,
                    pickupable: key == COIN,
                    door: key == DOOR,
                })
            },
        )
        .map_err(debug)?;
        assert!(
            world
                .facts()
                .doors()
                .contains(crate::map::view::placement_key(pos(6, 1), 1).ok_or("door key")?)
        );
        let cells = world
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        let channel_pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            active.identity().server_artifact_digest(),
            active.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            world.map_revision_digest(),
            (1, 1, 7),
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
            channel_pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(deps.actor, at(1, 1))
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = &cells;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?
                .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(active.identity().server_artifact_digest().as_slice())
            .bind(active.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        // Seed a real carried backpack; the map coin is minted only by command 9 below.
        seed_backpack(&h, &owner, fence).await?;
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 15, 18, 31])
                .ok_or("selection")?;
        assert_eq!(
            admitted.continuity.selected_capabilities.as_slice(),
            &[4, 6, 11, 15, 18, 31]
        );
        assert_eq!(
            cells.index().lookup(
                cells.scope(),
                crate::content::LogicalCell { x: 1, y: 2, z: 7 }
            ),
            Ok(crate::content::CollisionClass::Walkable)
        );
        {
            let mut r = owner.runtime.lock().await;
            assert!(
                r.player_control_facts(deps.actor, admitted.game_session_id)
                    .is_ok()
            );
            assert_eq!(
                r.borrow_movement_position()
                    .read(deps.actor)
                    .map_err(debug)?
                    .context(),
                r.pinned_movement_context()
            );
        }
        let open = owner
            .item_walk_route(
                deps.actor,
                admitted.game_session_id,
                AutoWalkTarget::Object(object(world.base().digest(), pos(1, 6), 0)?),
            )
            .await
            .map_err(|()| "open route: path search failed")?
            .ok_or("open route: target or actor unavailable")?;
        assert_eq!(
            open.steps.into_iter().collect::<Vec<_>>(),
            vec![oteryn_protocol_oteryn::world_spatial::StepDirection::South; 4]
        );
        let target = object(world.base().digest(), pos(6, 1), 0)?;
        let route = owner
            .item_walk_route(
                deps.actor,
                admitted.game_session_id,
                AutoWalkTarget::Object(target.clone()),
            )
            .await
            .map_err(|()| "detour route")?
            .ok_or("route")?;
        assert!(route.steps.len() > 4, "wall forces a detour");
        let mut walked = route.expected.position();
        for step in route.steps {
            walked = next(walked, step);
            assert!(!(walked.x == 3 && walked.y <= 2));
        }
        assert_eq!(crate::movement::path_search::chebyshev(walked, at(6, 1)), 1);
        assert!(
            owner
                .item_walk_route(
                    deps.actor,
                    admitted.game_session_id,
                    AutoWalkTarget::Object(object(world.base().digest(), pos(12, 1), 0)?)
                )
                .await
                .is_err()
        );
        let (mut server, mut client) = tokio::io::duplex(1 << 20);
        let client_flow = async {
            let result =
                client_scenario(&mut client, &owner, admitted, world.base().digest()).await;
            client.shutdown().await?;
            result
        };
        let server_flow = serve_admitted(&mut server, admitted, &owner, IDLE_LIVENESS);
        let (end, result) = join_two(server_flow, client_flow).await;
        result?;
        assert!(
            matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)),
            "{end:?}"
        );
        let bag = h
            .root
            .read_character_backpack(&authority, fence.character_id)
            .await?
            .ok_or("backpack")?;
        assert_eq!(
            bag.entries.len(),
            1,
            "far move committed exactly one map pickup"
        );
        assert_eq!(bag.entries[0].item.definition.production_key, COIN);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn seed_backpack(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
) -> TestResult {
    use crate::durability::item_mint::{
        GroundPlacement, ItemMintCause, ItemMintOutcome, ItemMintRequest, TypedDefinitionRef,
    };
    use crate::durability::item_transfer::{ItemTransferDestination, ItemTransferRequest};
    use crate::gameplay_transport::item_ref_admission::{NativeItemWeights, generation_item_facts};
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let index = native.item_index().ok_or("index")?;
    let definition = crate::gameplay_transport::item_look::typed_definition(
        index,
        index
            .definition_ref("Item", BAG, "definition-r1")
            .ok_or("bag")?,
    )
    .ok_or("definition")?;
    let facts = generation_item_facts(native, &definition).map_err(debug)?;
    let mut mint = h
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
                    1,
                    TypedDefinitionRef {
                        family: "LootTable".into(),
                        production_key: "fixture:autowalk".into(),
                        revision_ref: "definition-r1".into(),
                    },
                    "fixture:autowalk".into(),
                    0,
                ),
                item: definition,
                quantity: 1,
                ground: GroundPlacement {
                    spatial_position:
                        crate::durability::map_item_mint_audit::map_item_spatial_position(1, 1, -7),
                    corpse_ref: id(3).to_vec(),
                    map_revision: crate::map::overlay::map_revision(
                        owner.bundle_map().ok_or("map")?.overlay.base(),
                    ),
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
    let ItemMintOutcome::Committed(item) = h
        .root
        .commit_item_mint(&permit, owner.character, owner.holder, &mut mint)
        .await
        .map_err(debug)?
    else {
        return Err("mint".into());
    };
    let mut transfer = h
        .root
        .freeze_item_transfer(
            owner.character,
            owner.holder,
            fence,
            ItemTransferRequest {
                weight_generation_digest: native.source_digest(),
                command: crate::foundation::CommandRef::new(
                    fence.game_session_id,
                    crate::foundation::CommandId::new(900).map_err(debug)?,
                ),
                source_item_instance_id: item.item_instance_id,
                destination: ItemTransferDestination::ContainerSlot,
                item: facts,
                backpack: None,
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            },
            &NativeItemWeights(native),
        )
        .await
        .map_err(debug)?;
    h.root
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
    Ok(())
}

pub(super) fn object(digest: [u8; 32], pos: TilePos, revision: u64) -> TestResult<WorldObjectTarget> {
    let mut placement = digest.to_vec();
    placement.extend_from_slice(
        &crate::map::view::placement_key(pos, 1)
            .ok_or("placement")?
            .to_be_bytes(),
    );
    Ok(WorldObjectTarget {
        placement,
        expected_revision: revision,
    })
}
fn next(
    mut at: MovementLocalPosition,
    step: oteryn_protocol_oteryn::world_spatial::StepDirection,
) -> MovementLocalPosition {
    use oteryn_protocol_oteryn::world_spatial::StepDirection as D;
    match step {
        D::North => at.y -= 1,
        D::East => at.x += 1,
        D::South => at.y += 1,
        D::West => at.x -= 1,
    }
    at
}

pub(super) async fn send(client: &mut DuplexStream, id: u64, kind: u32, payload: &[u8]) -> TestResult {
    let frame = encode_client_command(
        1,
        &ClientCommandValue {
            command_id: id,
            command_type: kind,
            payload,
        },
    )?;
    client
        .write_all(&u32::try_from(frame.len())?.to_be_bytes())
        .await?;
    client.write_all(&frame).await?;
    Ok(())
}
async fn read(client: &mut DuplexStream) -> TestResult<Vec<u8>> {
    let len = tokio::time::timeout(Duration::from_secs(20), client.read_u32()).await??;
    let mut frame = vec![0; usize::try_from(len)?];
    client.read_exact(&mut frame).await?;
    Ok(frame)
}

async fn client_scenario(
    client: &mut DuplexStream,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    digest: [u8; 32],
) -> TestResult {
    let mut coin = None;
    let mut disappearing = None;
    loop {
        let frame = read(client).await?;
        let envelope = decode_wire_envelope(&frame)?;
        if envelope.message_type() == MessageType::SnapshotChunk {
            for domain in decode_snapshot_chunk(envelope.payload())?.1 {
                if domain.domain_id == 17 {
                    let map = oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(
                        domain.payload,
                    )
                    .map_err(debug)?;
                    disappearing = map
                        .tiles
                        .iter()
                        .find(|tile| (tile.position.x, tile.position.y) == (7, 5))
                        .and_then(|tile| {
                            tile.items.iter().find_map(|item| match item.origin {
                                oteryn_protocol_oteryn::world_map::MapOrigin::Handle(handle) => {
                                    Some(handle)
                                }
                                _ => None,
                            })
                        });
                    coin = map
                        .tiles
                        .iter()
                        .find(|tile| (tile.position.x, tile.position.y) == (1, 5))
                        .and_then(|tile| {
                            tile.items.iter().find_map(|item| match item.origin {
                                oteryn_protocol_oteryn::world_map::MapOrigin::Handle(handle) => {
                                    Some(handle)
                                }
                                _ => None,
                            })
                        });
                }
            }
        }
        if envelope.message_type() == MessageType::SnapshotCommit {
            break;
        }
    }
    // Far USE walks around a wall and toggles the real map door.
    send(
        client,
        1,
        2,
        &encode_use_intent(&object(digest, pos(6, 1), 0)?).map_err(debug)?,
    )
    .await?;
    let (used, steps) = result(client, 1).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&used).map_err(debug)?,
        UseDisposition::Committed
    );
    assert!(steps > 4);
    // Far MOVE reaches the source then performs the actual fenced map MINT/TRANSFER.
    let payload = encode_item_move_intent_for(
        &ItemMoveIntent {
            source: coin.ok_or("coin handle")?,
            destination: ItemMoveDestination::MainBackpack,
        },
        MOVE_SELECTION,
    )
    .map_err(debug)?;
    send(client, 2, 9, &payload).await?;
    let (moved, steps) = result(client, 2).await?;
    assert_eq!(
        oteryn_protocol_oteryn::item_view::decode_item_move_result_for(&moved, MOVE_SELECTION)
            .map_err(debug)?,
        ItemMoveOutcome::Moved
    );
    assert!(steps > 0);
    // A client step replaces the queue after its first committed step. It runs through the
    // ordinary one-step buffer after the pending USE receives its terminal refusal.
    send(
        client,
        3,
        2,
        &encode_use_intent(&object(digest, pos(6, 1), 1)?).map_err(debug)?,
    )
    .await?;
    loop {
        let frame = read(client).await?;
        let envelope = decode_wire_envelope(&frame)?;
        if envelope.message_type() == MessageType::StateDelta
            && decode_state_delta(envelope.payload())?.domain_id == 1
        {
            break;
        }
        if envelope.message_type() == MessageType::CommandResult {
            let result = decode_command_result(envelope.payload())?;
            return Err(format!(
                "expected a walk step before command {} result {:?}",
                result.command_id, result.payload
            )
            .into());
        }
    }
    send(
        client,
        4,
        1,
        &oteryn_protocol_oteryn::world_spatial::encode_step_intent(
            oteryn_protocol_oteryn::world_spatial::StepDirection::West,
        ),
    )
    .await?;
    let (cancelled, extra) = result(client, 3).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&cancelled).map_err(debug)?,
        UseDisposition::TooFar
    );
    assert_eq!(extra, 0, "no queued step after client input");
    let _ = result(client, 4).await?;
    loop {
        let frame = read(client).await?;
        let envelope = decode_wire_envelope(&frame)?;
        if envelope.message_type() == MessageType::StateDelta
            && decode_state_delta(envelope.payload())?.domain_id == 1
        {
            break;
        }
    }
    // An unreachable target refuses without moving.
    send(
        client,
        5,
        2,
        &encode_use_intent(&object(digest, pos(12, 1), 0)?).map_err(debug)?,
    )
    .await?;
    let (refused, steps) = result(client, 5).await?;
    assert_eq!(steps, 0);
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&refused).map_err(debug)?,
        UseDisposition::TooFar
    );
    loop {
        let frame = read(client).await?;
        let envelope = decode_wire_envelope(&frame)?;
        if envelope.message_type() == MessageType::StateDelta {
            let delta = decode_state_delta(envelope.payload())?;
            if delta.domain_id == 23 {
                assert_eq!(
                    oteryn_protocol_oteryn::tutorial_notices::decode_tutorial_notice(delta.payload)
                        .map_err(debug)?,
                    oteryn_protocol_oteryn::tutorial_notices::TutorialNotice::StatusText(
                        "There is no way.".into()
                    )
                );
                break;
            }
        }
    }
    // A refused world action retains its ordinary 200 ms cooldown.
    tokio::time::sleep(Duration::from_millis(200)).await;
    // Teleport/forced relocation invalidates the position revision before the next step.
    send(
        client,
        6,
        2,
        &encode_use_intent(&object(digest, pos(6, 1), 1)?).map_err(debug)?,
    )
    .await?;
    first_step(client).await?;
    let before_relocation = owner
        .runtime
        .lock()
        .await
        .borrow_movement_position()
        .read(admitted.runtime_actor.ok_or("actor")?)
        .map_err(debug)?
        .position();
    place(
        owner,
        admitted,
        MovementLocalPosition {
            y: before_relocation.y + 1,
            ..before_relocation
        },
    )
    .await?;
    place(owner, admitted, before_relocation).await?;
    let cancelled = result_at_rest(client, 6, before_relocation).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&cancelled).map_err(debug)?,
        UseDisposition::TooFar
    );
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .borrow_movement_position()
            .read(admitted.runtime_actor.ok_or("actor")?)
            .map_err(debug)?
            .position(),
        before_relocation
    );
    // An independent committed pickup removes the original source while this walk runs.
    // Its command is not retried against a replacement item; the existing route says STALE.
    let payload = encode_item_move_intent_for(
        &ItemMoveIntent {
            source: disappearing.ok_or("second coin handle")?,
            destination: ItemMoveDestination::MainBackpack,
        },
        MOVE_SELECTION,
    )
    .map_err(debug)?;
    send(client, 7, 9, &payload).await?;
    first_step(client).await?;
    send(client, 7, 9, &payload).await?; // same pending CommandId never starts a second action
    pick_origin(owner, admitted, pos(7, 5)).await?;
    let (stale, steps) = result(client, 7).await?;
    assert!(steps > 0, "a duplicate does not cancel the pending walk");
    assert_eq!(
        oteryn_protocol_oteryn::item_view::decode_item_move_result_for(&stale, MOVE_SELECTION)
            .map_err(debug)?,
        ItemMoveOutcome::Stale
    );
    // Death cancels a queue even when the position itself has not changed.
    send(
        client,
        8,
        2,
        &encode_use_intent(&object(digest, pos(1, 6), 0)?).map_err(debug)?,
    )
    .await?;
    first_step(client).await?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let cell = owner
        .runtime
        .lock()
        .await
        .borrow_movement_position()
        .read(actor)
        .map_err(debug)?
        .position();
    owner.states.spell_states.lock().await.deaths.push((
        actor,
        admitted.game_session_id,
        crate::gameplay_transport::actor_spell::PlayerDeath {
            occurrence: crate::durability::character_death::PlayerDeathOccurrence::from_bytes(id(
                99,
            ))
            .map_err(debug)?,
            cell,
        },
    ));
    let (dead, steps) = result(client, 8).await?;
    assert_eq!(steps, 0);
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&dead).map_err(debug)?,
        UseDisposition::TooFar
    );
    client.shutdown().await?;
    Ok(())
}

async fn first_step(client: &mut DuplexStream) -> TestResult {
    loop {
        let frame = read(client).await?;
        let envelope = decode_wire_envelope(&frame)?;
        if envelope.message_type() == MessageType::StateDelta
            && decode_state_delta(envelope.payload())?.domain_id == 1
        {
            return Ok(());
        }
        if envelope.message_type() == MessageType::CommandResult {
            let result = decode_command_result(envelope.payload())?;
            return Err(format!(
                "expected a walk step before command {} result {:?}",
                result.command_id, result.payload
            )
            .into());
        }
    }
}

async fn pick_origin(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    pos: TilePos,
) -> TestResult {
    use crate::durability::map_item_mint::{MapItemMintOutcome, MapItemMintRequest};
    use crate::gameplay_transport::item_ref_admission::generation_item_facts;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let index = native.item_index().ok_or("index")?;
    let definition = crate::gameplay_transport::item_look::typed_definition(
        index,
        index
            .definition_ref("Item", COIN, "definition-r1")
            .ok_or("coin")?,
    )
    .ok_or("definition")?;
    let item = generation_item_facts(native, &definition).map_err(debug)?;
    let fence = admitted.item_fence.ok_or("fence")?;
    let request = MapItemMintRequest {
        command: crate::foundation::CommandRef::new(
            admitted.game_session_id,
            crate::foundation::CommandId::new(901).map_err(debug)?,
        ),
        base_bundle_digest: owner.bundle_map().ok_or("map")?.overlay.base().digest(),
        placement_key: crate::map::view::placement_key(pos, 1).ok_or("key")?,
        reset_epoch: 0,
        item,
        quantity: 1,
        content_revision: format!(
            "sha256:{}",
            native
                .source_digest()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        ),
        ruleset_revision: crate::interaction_chest_use::entry_chest::RULESET_REVISION.into(),
        sim_revision: crate::interaction_chest_use::entry_chest::SIM_REVISION.into(),
    };
    let mut candidate = owner
        .root
        .freeze_map_item_mint(owner.character, owner.holder, fence, request)
        .await
        .map_err(debug)?;
    let permit = owner.spell_lane_permit().await.ok_or("lane")?;
    assert!(matches!(
        owner
            .root
            .commit_map_item_mint(
                &permit,
                owner.character,
                owner.holder,
                fence,
                &mut candidate
            )
            .await
            .map_err(debug)?,
        MapItemMintOutcome::Committed(_)
    ));
    owner.states.loop_ground.invalidate();
    Ok(())
}

async fn result_at_rest(
    client: &mut DuplexStream,
    id: u64,
    at: MovementLocalPosition,
) -> TestResult<Vec<u8>> {
    loop {
        let frame = read(client).await?;
        let envelope = decode_wire_envelope(&frame)?;
        if envelope.message_type() == MessageType::StateDelta {
            let delta = decode_state_delta(envelope.payload())?;
            if delta.domain_id == 1 {
                // The relocation's revision update is legitimate even after returning to the
                // same tile. Every observed position must stay at that tile after cancellation.
                let value = oteryn_protocol_oteryn::world_spatial_entities::decode_world_spatial_entities_delta_with_item_handles(delta.payload).map_err(debug)?;
                assert_eq!(
                    (
                        value.actor_position.x,
                        value.actor_position.y,
                        value.actor_position.floor
                    ),
                    (at.x, at.y, at.floor)
                );
            }
        }
        if envelope.message_type() == MessageType::CommandResult {
            let result = decode_command_result(envelope.payload())?;
            assert_eq!(result.command_id, id);
            return Ok(result.payload.to_vec());
        }
    }
}

pub(super) async fn result(client: &mut DuplexStream, id: u64) -> TestResult<(Vec<u8>, usize)> {
    let mut steps = 0;
    loop {
        let frame = read(client).await?;
        let envelope = decode_wire_envelope(&frame)?;
        if envelope.message_type() == MessageType::StateDelta
            && decode_state_delta(envelope.payload())?.domain_id == 1
        {
            steps += 1;
        }
        if envelope.message_type() == MessageType::CommandResult {
            let result = decode_command_result(envelope.payload())?;
            assert_eq!(result.command_id, id);
            return Ok((result.payload.to_vec(), steps));
        }
    }
}

async fn place(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    destination: MovementLocalPosition,
) -> TestResult {
    let mut runtime = owner.runtime.lock().await;
    let mut movement = runtime.borrow_movement_position();
    loop {
        let before = movement
            .read(admitted.runtime_actor.ok_or("actor")?)
            .map_err(debug)?;
        let mut at = before.position();
        if at == destination {
            return Ok(());
        }
        if at.x != destination.x {
            at.x += (destination.x - at.x).signum();
        } else {
            at.y += (destination.y - at.y).signum();
        }
        movement.commit_cardinal(before, at).map_err(debug)?;
    }
}
