//! M06Q: a production world placement checks its durable named quest storage over USE wire.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

#[test]
fn m06q_placed_quest_door_use_wire_requires_durable_storage() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(false, false, false, None, None).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "M06Q wire panic")??;
    Ok(())
}

#[test]
fn m06_close_quest_door_step_out_wire() -> TestResult {
    closing_case(false, false)
}

#[test]
fn m06_close_quest_door_step_out_moves_item_wire() -> TestResult {
    closing_case(true, false)
}

#[test]
fn m06_close_quest_door_step_out_moves_authored_item_wire() -> TestResult {
    closing_case(false, true)
}

fn closing_case(item: bool, authored: bool) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(true, item, authored, None, None).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "M06-CLOSE wire panic")??;
    Ok(())
}

#[derive(Clone, Copy, Debug)]
enum PlayerGroundCase {
    Move,
    Refused,
    Authored,
}

#[test]
fn m06_player_ground_move_commits_once_and_updates_observer() -> TestResult {
    player_ground_case(PlayerGroundCase::Move)
}

#[test]
fn m06_player_ground_move_far_or_missing_tile_writes_nothing() -> TestResult {
    player_ground_case(PlayerGroundCase::Refused)
}

#[test]
fn m06_player_ground_move_authored_item_materializes_once_and_replays() -> TestResult {
    player_ground_case(PlayerGroundCase::Authored)
}

fn player_ground_case(case: PlayerGroundCase) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(false, false, false, Some(case), None).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "M06 player ground move panic")??;
    Ok(())
}

fn scenario(
    closing: bool,
    with_item: bool,
    authored: bool,
    player_case: Option<PlayerGroundCase>,
    vampire_door: Option<u16>,
) -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "m06q_quest", vampire_door.is_none(), false)
                .await?;
        eprintln!("START M06Q real placed-door USE wire PostgreSQL");
        let started = std::time::Instant::now();
        super::reset();
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, pin) = if vampire_door.is_some() {
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?
        } else {
            crate::content::native_gameplay::test_support::activated_with_loot_tables(
                deps.admitted.world_id,
                None,
            )
        };
        deps.room = pin.qualified_room().clone();
        let active = controller.active().ok_or("active")?;
        let native = active.native_gameplay().ok_or("native")?;
        let index = native.item_index().ok_or("index")?;
        let door = if vampire_door.is_some() {
            "oteryn:item.tibia.i5102"
        } else {
            "oteryn:item.tibia.i5113"
        };
        let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
        let (bytes, mut pins) = crate::map::boot::tests::bundle_with_item_ids(
            (door, reference(door).ok_or("door")?.get() - 1),
            (
                "oteryn:item.tibia.i3031",
                reference("oteryn:item.tibia.i3031").ok_or("coin")?.get() - 1,
            ),
        );
        let mut data = oteryn_world_bundle::bundle::read(&bytes)?;
        if closing {
            // The shared map fixture puts a wall west of the doorway. This scenario needs
            // an exit tile so it can exercise a successful step through and off the door.
            data.sectors
                .iter_mut()
                .flat_map(|sector| &mut sector.tiles)
                .find(|tile| tile.x == 5)
                .ok_or("west exit")?
                .items
                .truncate(1);
        }
        if authored {
            // Moving off this door lands on a teleport. Cleanup still belongs to the
            // departed doorway, even though the player is no longer in pickup reach.
            data.sectors
                .iter_mut()
                .flat_map(|sector| &mut sector.tiles)
                .find(|tile| tile.x == 5)
                .and_then(|tile| tile.items.first_mut())
                .ok_or("teleport exit")?
                .attrs
                .teleport = Some((1, 0, (-7_i8).to_ne_bytes()[0]));
            let coin = data
                .sectors
                .iter()
                .flat_map(|sector| &sector.tiles)
                .find(|tile| tile.x == 7)
                .and_then(|tile| tile.items.last())
                .ok_or("authored coin")?
                .clone();
            data.sectors
                .iter_mut()
                .flat_map(|sector| &mut sector.tiles)
                .find(|tile| tile.x == 6)
                .ok_or("doorway")?
                .items
                .push(coin);
        }
        let entry = data
            .sectors
            .iter_mut()
            .flat_map(|s| &mut s.tiles)
            .find(|tile| tile.x == 6)
            .ok_or("door tile")?
            .items
            .get_mut(1)
            .ok_or("door entry")?;
        // Read the actual committed world placement, not a test-authored action id.
        let source_bytes = if vampire_door.is_some() {
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z14-x129-y123.b3"
            ))
            .as_slice()
        } else {
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z14-x125-y124.b3"
            ))
            .as_slice()
        };
        let region = oteryn_world_bundle_compiler::b3::decode_region(
            source_bytes,
            oteryn_world_bundle::bundle::TILE_LIMITS,
            &mut oteryn_world_bundle::sector::Budget {
                tiles: 65536,
                entries: 1_000_000,
            },
        )?;
        let (x, y, appearance, action) =
            vampire_door.map_or((32246, 31861, 5113, 10089), |x| (x, 31684, 5102, 1070));
        let placed = region
            .sectors
            .iter()
            .flat_map(|(_, _, tiles)| tiles)
            .find(|tile| tile.x == x && tile.y == y)
            .ok_or("placed door")?;
        assert_eq!(placed.house, 0);
        let palette: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/index.json"
        )))?;
        let source = placed
            .items
            .iter()
            .find(|item| {
                item.depth == 0
                    && palette["palette"][item.palette as usize]["source_item_id"].as_u64()
                        == Some(appearance)
            })
            .ok_or("source door")?;
        assert_eq!(source.attrs.action, Some(action));
        entry.attrs = source.attrs.clone();
        let bytes = oteryn_world_bundle_compiler::bundle::write_with_npcs(
            &data.manifest,
            &data.sectors,
            &data.spawns,
            &data.npcs,
        )?;
        pins.digest = oteryn_world_bundle::bundle::read(&bytes)?.digest;
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let map = crate::map::boot::boot(
            &bytes,
            &crate::map::boot::BootPins {
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
                use crate::content::{ReferenceItemField, ReferenceItemType};
                let policy = native.item_policy(key, "definition-r1")?.record();
                Some(crate::map::facts::ItemDefinition {
                    reference: reference(key)?,
                    solid: policy.attributes.blocks_movement,
                    blocks_projectile: policy.attributes.blocks_projectile.unwrap_or(true),
                    pickupable: matches!(&policy.semantics.physical, ReferenceItemField::Known(p) if p.pickupable == ReferenceItemField::Known(true)),
                    door: matches!(&policy.semantics.classification, ReferenceItemField::Known(c) if c.item_type == ReferenceItemField::Known(ReferenceItemType::Door)),
                })
            },
        )
        .map_err(debug)?;
        let cells = map
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        let frame = map.frame_binding_digest(deps.admitted.world_id);
        deps.combat_room = Some(
            deps.room
                .bundle_view(
                    cells.clone(),
                    frame,
                    map.map_revision_digest(),
                    crate::content::NativeEntryStart {
                        x: 7,
                        y: 0,
                        floor: 7,
                    },
                )
                .ok_or("bundle combat view")?,
        );
        let content_pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            frame,
            map.map_revision_digest(),
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
            content_pin,
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
                    x: 7,
                    y: 0,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = (&cells).into();
        deps.initialize(&owner).await;
        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(
            harness.root.clone(),
        )
        .current_session_at(deps.admitted.game_session_id)
        .await?
        .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("fence")?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(frame.as_slice()).execute(&harness.pool).await?;
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 15, 18])
                .ok_or("USE capabilities")?;
        let station_pos = crate::map::overlay::TilePos {
            x: 6,
            y: 0,
            floor: -7,
        };
        let mut placement = map.base().digest().to_vec();
        placement.extend_from_slice(
            &crate::map::view::placement_key(station_pos, 1)
                .ok_or("door placement")?
                .to_be_bytes(),
        );
        let target = WorldObjectTarget {
            placement,
            expected_revision: 0,
        };
        let payload =
            oteryn_protocol_oteryn::world_object::encode_use_intent(&target).map_err(debug)?;
        owner.admit_quest_session(&admitted).await;
        if vampire_door.is_some() {
            owner.admit_progression_session(&mut admitted).await;
            vampire_gate(&mut owner, admitted, &authority, &map, &payload).await?;
            drop(owner);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }
        let (next, result, _) =
            super::item_craft::exchange(&owner, admitted, 1, &payload, None).await?;
        assert_eq!(result, UseDisposition::Locked);
        assert!(!map.enterable(station_pos));
        assert_eq!(
            map.facts()
                .doors()
                .revision(crate::map::boot::tests::door_key()),
            0
        );
        // Fixture seed only: production admission reloads the durable track as runtime role.
        let mut tx = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) VALUES(encode($1,'hex')::uuid,'oteryn:quest-progress/quest/u7_2/the_queen_of_the_banshees/banshee_door','oteryn:quest.the_queen_of_the_banshees_quest',0,2)")
            .bind(id(41).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        owner.admit_quest_session(&next).await;
        // Advance the owner clock beyond the declared USE rate gate without sleeping.
        owner.clock_origin -= std::time::Duration::from_secs(1);
        let (next, result, _) =
            super::item_craft::exchange(&owner, next, 2, &payload, None).await?;
        assert_eq!(result, UseDisposition::Committed);
        assert!(map.enterable(station_pos));
        assert_eq!(
            map.facts()
                .doors()
                .revision(crate::map::boot::tests::door_key()),
            1
        );
        if let Some(case) = player_case {
            use crate::gameplay_transport::{
                connection::UseCommand,
                item_move::{LoopMove, LoopMoveRefusal, LoopMoveResult, LoopTarget},
                item_view::{ItemKey, ItemViewContinuity, SessionItemView},
                world_map::{MapUpdate, MapViewSource, SessionMapView},
            };
            use oteryn_protocol_oteryn::item_view::WorldTilePosition;
            let cell = |x| WorldTilePosition { x, y: 0, floor: -7 };
            let command = |command_id| UseCommand {
                game_session_id: next.game_session_id,
                command_id,
                item_fence: Some(fence),
            };
            let item = if matches!(case, PlayerGroundCase::Authored) {
                // The real authored coin is still visible on the actor's tile.
                ItemKey::MapBase {
                    bundle_digest: map.base().digest(),
                    placement_key: crate::map::view::placement_key(
                        crate::map::overlay::TilePos {
                            x: 7,
                            y: 0,
                            floor: -7,
                        },
                        1,
                    )
                    .ok_or("authored coin placement")?,
                    reset_epoch: owner.reset_epoch,
                }
            } else {
                // Reuse the door and item-look fixture's ordinary backpack -> Ground path.
                super::item_look::seed_main_backpack(&harness, &[("oteryn:item.tibia.i3031", 1)])
                    .await?;
                let backpack = owner
                    .root
                    .read_character_backpack(&authority, fence.character_id)
                    .await?
                    .ok_or("backpack")?;
                let item = ItemKey::Instance(
                    backpack
                        .entries
                        .first()
                        .ok_or("coin")?
                        .item
                        .item_instance_id,
                );
                assert_eq!(
                    owner
                        .move_loop_item(
                            deps.actor,
                            command(100),
                            LoopMove {
                                item,
                                count: None,
                                target: LoopTarget::Ground(cell(6)),
                            }
                        )
                        .await,
                    LoopMoveResult::Committed
                );
                item
            };
            let before = player_ground_state(&harness.pool).await?;
            match case {
                PlayerGroundCase::Move => {
                    let observer_at = oteryn_protocol_oteryn::world_spatial::ActorPosition {
                        x: 7,
                        y: 0,
                        floor: -7,
                    };
                    let mut observer = SessionMapView::default();
                    let mut observer_items =
                        SessionItemView::resume(ItemViewContinuity::default()).with_map_view();
                    let ground = crate::gameplay_transport::loop_move::loop_ground(
                        &owner,
                        map.base(),
                        Some(index),
                    )
                    .await
                    .ok_or("observer ground")?;
                    let facts = ground.facts(map.facts());
                    let source = MapViewSource {
                        overlay: ground.overlay(),
                        facts: &facts,
                        content_generation: native.source_digest(),
                        reset_epoch: 0,
                    };
                    let snapshot = observer
                        .snapshot(&mut observer_items, &source, observer_at)
                        .map_err(debug)?;
                    let header = oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(
                        &snapshot.payload,
                    )
                    .map_err(debug)?
                    .header;
                    assert_eq!(
                        owner
                            .move_loop_item(
                                deps.actor,
                                command(101),
                                LoopMove {
                                    item,
                                    count: None,
                                    target: LoopTarget::Ground(cell(7)),
                                }
                            )
                            .await,
                        LoopMoveResult::Committed
                    );
                    let ItemKey::Instance(item_id) = item else {
                        return Err("instance required".into());
                    };
                    let ground = owner
                        .root
                        .read_item_loop_ground(owner.world_id, owner.channel_id)
                        .await?;
                    let moved = ground
                        .iter()
                        .find(|entry| entry.item.item_instance_id == item_id)
                        .ok_or("moved instance")?;
                    let expected = [
                        7_i32.to_be_bytes().as_slice(),
                        0_i32.to_be_bytes().as_slice(),
                        (-7_i16).to_be_bytes().as_slice(),
                    ]
                    .concat();
                    assert_eq!(moved.item.ground.spatial_position, expected);
                    assert_eq!(moved.item.quantity, 1);
                    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_loop_move_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=101")
                        .bind(next.game_session_id.as_bytes().as_slice()).fetch_one(&harness.pool).await?;
                    assert_eq!(receipts, 1);
                    let lines: (i64, i64) = sqlx::query_as("SELECT count(*), COALESCE(sum(quantity_after),0)::bigint FROM game_item_loop_move_lines WHERE item_instance_id=encode($1,'hex')::uuid AND from_kind=3 AND to_kind=3")
                        .bind(item_id.as_slice()).fetch_one(&harness.pool).await?;
                    assert_eq!(lines, (1, 1));
                    let after = player_ground_state(&harness.pool).await?;
                    assert_eq!(before["drops"], serde_json::json!(1));
                    assert_eq!(
                        after["drops"], before["drops"],
                        "ITEM_LOOP_CHANNEL_DROPS_MAX counts roots, not moves"
                    );
                    let ground = crate::gameplay_transport::loop_move::loop_ground(
                        &owner,
                        map.base(),
                        Some(index),
                    )
                    .await
                    .ok_or("updated observer ground")?;
                    let facts = ground.facts(map.facts());
                    let source = MapViewSource {
                        overlay: ground.overlay(),
                        facts: &facts,
                        content_generation: native.source_digest(),
                        reset_epoch: 0,
                    };
                    let Some(MapUpdate::Delta(delta)) = observer
                        .update(&mut observer_items, &source, observer_at)
                        .map_err(debug)?
                    else {
                        return Err("observer must receive ground move delta".into());
                    };
                    let delta = oteryn_protocol_oteryn::world_map::decode_world_map_delta(
                        &header,
                        &delta.payload,
                    )
                    .map_err(debug)?;
                    let from = delta
                        .tiles
                        .iter()
                        .find(|tile| tile.position.x == 6 && tile.position.y == 0)
                        .ok_or("source delta")?;
                    assert!(!from.items.iter().any(|entry| entry.appearance_id == 3031));
                    let to = delta
                        .tiles
                        .iter()
                        .find(|tile| tile.position.x == 7 && tile.position.y == 0)
                        .ok_or("destination delta")?;
                    assert_eq!(
                        to.items
                            .iter()
                            .filter(|entry| entry.appearance_id == 3031)
                            .count(),
                        2,
                        "authored coin plus moved instance"
                    );
                }
                PlayerGroundCase::Refused => {
                    // x=1 is a real tile beyond reach; x=8 is in reach but absent from the bundle.
                    for (command_id, x, expected) in [
                        (101, 1, LoopMoveResult::TooFar),
                        (102, 8, LoopMoveResult::Refused(LoopMoveRefusal::Blocked)),
                    ] {
                        assert_eq!(
                            owner
                                .move_loop_item(
                                    deps.actor,
                                    command(command_id),
                                    LoopMove {
                                        item,
                                        count: None,
                                        target: LoopTarget::Ground(cell(x)),
                                    }
                                )
                                .await,
                            expected
                        );
                        assert_eq!(
                            player_ground_state(&harness.pool).await?,
                            before,
                            "refusal writes nothing"
                        );
                    }
                }
                PlayerGroundCase::Authored => {
                    use crate::gameplay_transport::item_move::{ItemMoveStep, item_move};
                    use oteryn_protocol_oteryn::item_view::{
                        ItemMoveDestination, ItemMoveIntent, ItemMoveOutcome, ItemMoveSelection,
                        encode_item_move_intent_for,
                    };

                    let mut observer = SessionMapView::default();
                    let mut items =
                        SessionItemView::resume(ItemViewContinuity::default()).with_map_view();
                    let Some(MapUpdate::Snapshot(snapshot)) = owner
                        .observe_world_map(deps.actor, &mut observer, &mut items)
                        .await
                        .ok_or("authored source map")?
                        .map_err(debug)?
                    else {
                        return Err("authored source snapshot".into());
                    };
                    let header = oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(
                        &snapshot.payload,
                    )
                    .map_err(debug)?
                    .header;
                    let selection = ItemMoveSelection {
                        equip_drop: false,
                        container_tree: false,
                        world_interaction: true,
                    };
                    let payload = encode_item_move_intent_for(
                        &ItemMoveIntent {
                            source: items.table().handle(&item).ok_or("authored coin handle")?,
                            destination: ItemMoveDestination::Ground(cell(6)),
                        },
                        selection,
                    )
                    .map_err(debug)?;
                    assert!(matches!(
                        item_move(
                            &owner,
                            deps.actor,
                            command(101),
                            &payload,
                            selection,
                            false,
                            &mut items,
                        )
                        .await,
                        ItemMoveStep::Result(ItemMoveOutcome::Moved, _)
                    ));
                    let ItemKey::MapBase { placement_key, .. } = item else {
                        return Err("authored source identity".into());
                    };
                    let (minted, definition, quantity): (Vec<u8>, String, i64) = sqlx::query_as(
                        "SELECT uuid_send(item_instance_id),definition_production_key,quantity \
                         FROM game_map_item_mint_receipts \
                         WHERE game_session_id=encode($1,'hex')::uuid AND command_id=101 \
                         AND base_bundle_digest=$2 AND placement_key=$3::text::numeric \
                         AND reset_epoch=$4::text::numeric",
                    )
                    .bind(next.game_session_id.as_bytes().as_slice())
                    .bind(map.base().digest().as_slice())
                    .bind(placement_key.to_string())
                    .bind(owner.reset_epoch.to_string())
                    .fetch_one(&harness.pool)
                    .await?;
                    let minted: [u8; 16] = minted.try_into().map_err(|_| "minted coin identity")?;
                    assert_eq!(
                        (definition.as_str(), quantity),
                        ("oteryn:item.tibia.i3031", 1)
                    );
                    let ground = owner
                        .root
                        .read_item_loop_ground(owner.world_id, owner.channel_id)
                        .await?;
                    let moved = ground
                        .iter()
                        .find(|entry| entry.item.item_instance_id == minted)
                        .ok_or("materialized coin custody")?;
                    assert_eq!(moved.item.quantity, 1);
                    assert_eq!(
                        moved.item.ground.spatial_position,
                        [
                            6_i32.to_be_bytes().as_slice(),
                            0_i32.to_be_bytes().as_slice(),
                            (-7_i16).to_be_bytes().as_slice(),
                        ]
                        .concat()
                    );
                    let custody: i64 = sqlx::query_scalar(
                        "SELECT (SELECT count(*) FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid) \
                         +(SELECT count(*) FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid) \
                         +(SELECT count(*) FROM game_item_nested_entries WHERE item_instance_id=encode($1,'hex')::uuid) \
                         +(SELECT count(*) FROM game_item_container_slots WHERE item_instance_id=encode($1,'hex')::uuid)",
                    )
                    .bind(minted.as_slice())
                    .fetch_one(&harness.pool)
                    .await?;
                    assert_eq!(
                        custody, 1,
                        "the minted coin must have exactly one custody owner"
                    );
                    let after = player_ground_state(&harness.pool).await?;
                    assert_eq!(before["mint_receipts"], serde_json::json!([]));
                    assert_eq!(
                        after["mint_receipts"]
                            .as_array()
                            .ok_or("MINT receipts")?
                            .len(),
                        1
                    );
                    assert_eq!(
                        after["receipts"].as_array().ok_or("move receipts")?.len(),
                        1
                    );
                    assert_eq!(
                        after["items"].as_array().ok_or("minted items")?.len(),
                        before["items"].as_array().ok_or("prior items")?.len() + 1,
                    );
                    let ground = crate::gameplay_transport::loop_move::loop_ground(
                        &owner,
                        map.base(),
                        Some(index),
                    )
                    .await
                    .ok_or("materialized source overlay")?;
                    assert!(
                        ground
                            .overlay()
                            .tile(crate::map::overlay::TilePos {
                                x: 7,
                                y: 0,
                                floor: -7,
                            })
                            .is_some_and(|tile| tile.is_hidden(1)),
                        "the authored source must stay hidden"
                    );
                    for (command_id, expected) in
                        [(101, ItemMoveOutcome::Moved), (102, ItemMoveOutcome::Stale)]
                    {
                        if command_id == 102 {
                            // Inventory replay does not retire a Map handle. Publish the actual
                            // changed map before testing reuse of its removed authored source.
                            let Some(MapUpdate::Delta(delta)) = owner
                                .observe_world_map(deps.actor, &mut observer, &mut items)
                                .await
                                .ok_or("materialized source map")?
                                .map_err(debug)?
                            else {
                                return Err("materialization must publish a map delta".into());
                            };
                            let delta = oteryn_protocol_oteryn::world_map::decode_world_map_delta(
                                &header,
                                &delta.payload,
                            )
                            .map_err(debug)?;
                            let source = delta
                                .tiles
                                .iter()
                                .find(|tile| tile.position.x == 7 && tile.position.y == 0)
                                .ok_or("removed authored source delta")?;
                            assert!(!source.items.iter().any(|entry| entry.appearance_id == 3031));
                            assert!(items.table().handle(&item).is_none());
                        }
                        assert!(matches!(
                            item_move(
                                &owner, deps.actor, command(command_id), &payload,
                                selection, false, &mut items,
                            ).await,
                            ItemMoveStep::Result(actual, _) if actual == expected
                        ));
                        assert_eq!(
                            player_ground_state(&harness.pool).await?,
                            after,
                            "replay and stale-source reuse must preserve exact custody and receipts"
                        );
                    }
                }
            }
            eprintln!(
                "M06-PLAYER-GROUND EXECUTED case={case:?} duration={:.3}s",
                started.elapsed().as_secs_f64()
            );
        }
        if closing {
            use crate::gameplay_transport::{
                connection::UseCommand,
                item_move::{LoopMove, LoopMoveResult, LoopTarget},
                item_view::ItemKey,
            };
            use oteryn_protocol_oteryn::world_spatial::StepDirection;
            let item = if with_item {
                super::item_look::seed_main_backpack(&harness, &[("oteryn:item.tibia.i3031", 1)])
                    .await?;
                let backpack = owner
                    .root
                    .read_character_backpack(&authority, fence.character_id)
                    .await?
                    .ok_or("backpack")?;
                let item = backpack
                    .entries
                    .first()
                    .ok_or("coin")?
                    .item
                    .item_instance_id;
                assert_eq!(
                    owner
                        .move_loop_item(
                            deps.actor,
                            UseCommand {
                                game_session_id: next.game_session_id,
                                command_id: 100,
                                item_fence: Some(fence)
                            },
                            LoopMove {
                                item: ItemKey::Instance(item),
                                count: None,
                                target: LoopTarget::Ground(
                                    oteryn_protocol_oteryn::item_view::WorldTilePosition {
                                        x: 6,
                                        y: 0,
                                        floor: -7
                                    }
                                )
                            }
                        )
                        .await,
                    LoopMoveResult::Committed
                );
                Some(item)
            } else {
                None
            };
            use crate::gameplay_transport::{
                item_view::{ItemViewContinuity, SessionItemView},
                world_map::{MapUpdate, MapViewSource, SessionMapView},
            };
            let observer_at = oteryn_protocol_oteryn::world_spatial::ActorPosition {
                x: 7,
                y: 0,
                floor: -7,
            };
            let mut observer = SessionMapView::default();
            let mut observer_items =
                SessionItemView::resume(ItemViewContinuity::default()).with_map_view();
            let ground =
                crate::gameplay_transport::loop_move::loop_ground(&owner, map.base(), Some(index))
                    .await
                    .ok_or("observer ground")?;
            let facts = ground.facts(map.facts());
            let source = MapViewSource {
                overlay: ground.overlay(),
                facts: &facts,
                content_generation: native.source_digest(),
                reset_epoch: 0,
            };
            let joined = observer
                .snapshot(&mut observer_items, &source, observer_at)
                .map_err(debug)?;
            let header =
                oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(&joined.payload)
                    .map_err(debug)?
                    .header;
            let next = step_exchange(&owner, next, 3, StepDirection::West).await?;
            assert!(map.enterable(station_pos), "occupied door remains open");
            let _next = step_exchange(&owner, next, 4, StepDirection::West).await?;
            if authored {
                assert_eq!(
                    owner
                        .runtime
                        .lock()
                        .await
                        .read_actor_position(deps.actor)
                        .map_err(debug)?
                        .position()
                        .x,
                    1,
                    "step-off teleport landed beyond pickup reach"
                );
            }
            assert!(!map.enterable(station_pos), "vacated quest door must close");
            assert_eq!(
                map.facts()
                    .doors()
                    .revision(crate::map::boot::tests::door_key()),
                2
            );
            let ground =
                crate::gameplay_transport::loop_move::loop_ground(&owner, map.base(), Some(index))
                    .await
                    .ok_or("updated observer ground")?;
            let facts = ground.facts(map.facts());
            let source = MapViewSource {
                overlay: ground.overlay(),
                facts: &facts,
                content_generation: native.source_digest(),
                reset_epoch: 0,
            };
            let Some(MapUpdate::Delta(delta)) = observer
                .update(&mut observer_items, &source, observer_at)
                .map_err(debug)?
            else {
                return Err("observer must receive closure delta".into());
            };
            let delta =
                oteryn_protocol_oteryn::world_map::decode_world_map_delta(&header, &delta.payload)
                    .map_err(debug)?;
            let doorway = delta
                .tiles
                .iter()
                .find(|tile| tile.position.x == 6 && tile.position.y == 0)
                .ok_or("observer doorway delta")?;
            assert!(doorway.items.iter().any(|entry| entry.appearance_id == 5113
                && matches!(
                    entry.origin,
                    oteryn_protocol_oteryn::world_map::MapOrigin::BaseOrdinal {
                        object_revision: 2,
                        ..
                    }
                )));
            if with_item || authored {
                assert!(
                    !doorway
                        .items
                        .iter()
                        .any(|entry| entry.appearance_id == 3031)
                );
                assert!(delta.tiles.iter().any(|tile| tile.position.x == 7
                    && tile.items.iter().any(|entry| entry.appearance_id == 3031)));
                let ground = owner
                    .root
                    .read_item_loop_ground(owner.world_id, owner.channel_id)
                    .await?;
                let moved = ground
                    .iter()
                    .find(|entry| {
                        item.map_or(
                            entry.item.definition.production_key == "oteryn:item.tibia.i3031",
                            |item| entry.item.item_instance_id == item,
                        )
                    })
                    .ok_or("moved coin")?;
                let item = moved.item.item_instance_id;
                let mut expected = Vec::new();
                expected.extend_from_slice(&7_i32.to_be_bytes());
                expected.extend_from_slice(&0_i32.to_be_bytes());
                expected.extend_from_slice(&(-7_i16).to_be_bytes());
                assert_eq!(moved.item.ground.spatial_position, expected);
                let relocation: (i64, i64) = sqlx::query_as("SELECT count(*), COALESCE(sum(quantity_after),0)::bigint FROM game_item_loop_move_lines WHERE item_instance_id=encode($1,'hex')::uuid AND from_kind=3 AND to_kind=3")
                    .bind(item.as_slice()).fetch_one(&harness.pool).await?;
                assert_eq!(
                    relocation,
                    (1, 1),
                    "one conserving Ground-to-Ground loop line"
                );
            }
            eprintln!(
                "M06-CLOSE EXECUTED item={with_item} authored={authored} duration={:.3}s",
                started.elapsed().as_secs_f64()
            );
        }
        eprintln!(
            "M06Q EXECUTED: placed action 10089, absent refused, durable value 0 opened; {:?}",
            started.elapsed()
        );
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

async fn step_exchange(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
    command_id: u64,
    direction: oteryn_protocol_oteryn::world_spatial::StepDirection,
) -> TestResult<crate::gameplay_transport::connection::AdmittedSession> {
    use crate::gameplay_transport::connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted};
    use tokio::io::AsyncWriteExt;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let client_task = async {
        loop {
            let raw = super::item_look::read_frame(&mut client).await?;
            if crate::foundation::decode_wire_envelope(&raw)?.message_type()
                == crate::foundation::MessageType::SnapshotCommit
            {
                break;
            }
        }
        let actual = oteryn_protocol_oteryn::world_spatial::encode_step_intent(direction);
        let frame = oteryn_protocol_oteryn::encode_client_command(
            1,
            &oteryn_protocol_oteryn::ClientCommandValue {
                command_id,
                command_type: 1,
                payload: &actual,
            },
        )?;
        client.write_u32(u32::try_from(frame.len())?).await?;
        client.write_all(&frame).await?;
        loop {
            let raw = super::item_look::read_frame(&mut client).await?;
            let envelope = crate::foundation::decode_wire_envelope(&raw)?;
            if envelope.message_type() == crate::foundation::MessageType::CommandResult {
                let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
                if result.command_id == command_id {
                    let disposition =
                        oteryn_protocol_oteryn::world_spatial::decode_step_result(result.payload)
                            .map_err(debug)?;
                    assert_eq!(
                        disposition,
                        oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved
                    );
                    client.shutdown().await?;
                    return Ok::<_, Box<dyn std::error::Error>>((disposition, actual));
                }
            }
        }
    };
    use std::future::Future;
    use std::task::Poll;
    let serving = async {
        let end = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let mut serving = std::pin::pin!(serving);
    let mut client_task = std::pin::pin!(client_task);
    let mut end = None;
    let mut result = None;
    tokio::time::timeout(
        std::time::Duration::from_secs(30),
        std::future::poll_fn(|context| {
            if end.is_none()
                && let Poll::Ready(value) = serving.as_mut().poll(context)
            {
                end = Some(value);
            }
            if result.is_none()
                && let Poll::Ready(value) = client_task.as_mut().poll(context)
            {
                result = Some(value);
            }
            if end.is_some() && result.is_some() {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        }),
    )
    .await?;
    let end = end.ok_or("server did not finish")?;
    let ConnectionEnd::AdmittedThenDisconnected(admitted) = end else {
        return Err(format!("{end:?}").into());
    };
    let (result, actual) = result.ok_or("client did not finish")??;
    let _ = (result, actual);
    Ok(admitted)
}

// Exact durable rows, not just counts: a refusal must not mutate custody or reserve a MINT.
async fn player_ground_state(pool: &sqlx::PgPool) -> Result<serde_json::Value, sqlx::Error> {
    sqlx::query_scalar("SELECT jsonb_build_object(\
        'drops',(SELECT count(*) FROM game_item_ground_locations WHERE game_item_loop_drop_ref(corpse_ref)),\
        'ground',(SELECT coalesce(jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r)::text),'[]'::jsonb) FROM game_item_ground_locations r),\
        'items',(SELECT coalesce(jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r)::text),'[]'::jsonb) FROM game_item_instances r),\
        'receipts',(SELECT coalesce(jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r)::text),'[]'::jsonb) FROM game_item_loop_move_receipts r),\
        'lines',(SELECT coalesce(jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r)::text),'[]'::jsonb) FROM game_item_loop_move_lines r),\
        'channels',(SELECT coalesce(jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r)::text),'[]'::jsonb) FROM game_item_loop_drop_channels r),\
        'mint_reservations',(SELECT coalesce(jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r)::text),'[]'::jsonb) FROM game_map_item_mint_reservations r),\
        'mint_receipts',(SELECT coalesce(jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r)::text),'[]'::jsonb) FROM game_map_item_mint_receipts r))")
        .fetch_one(pool).await
}

// Crystal door_level.lua:1070 and doors.lua:273: both source doors require level70.
#[test]
fn vampire_shield_placed_level_doors_require_seventy_over_wire() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            for x in [33190, 33195] {
                scenario(false, false, false, None, Some(x)).map_err(|e| e.to_string())?;
            }
            Ok::<(), String>(())
        })?
        .join()
        .map_err(|_| "Vampire Shield wire panic")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED vampire_shield_level_doors duration={:?}",
        started.elapsed()
    );
    Ok(())
}

async fn vampire_gate(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: crate::gameplay_transport::connection::AdmittedSession,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    map: &crate::map::boot::BundleWorld,
    payload: &[u8],
) -> TestResult {
    use crate::durability::character_progression::{
        ExperienceAwardRequest, ExperienceCommitOutcome, ExperienceRewardOccurrence,
    };
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            actor,
            admitted.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                    .map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    let formula = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|n| n.training_formula())
        .ok_or("formula")?;
    use crate::durability::character_build::{
        BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState, convert_vocation,
    };
    owner
        .revision_sequencer
        .acquire(fence.character_id)
        .await
        .commit_build(
            owner.root,
            authority,
            owner.holder,
            fence,
            BuildChangeRequest {
                occurrence: BuildOccurrence::from_bytes(id(90)).map_err(debug)?,
                cause: BuildCause::VocationChoice,
                before: DurableBuildState::default(),
                after: convert_vocation(formula, &DurableBuildState::default(), "monk")
                    .map_err(debug)?,
                pruned_stance: None,
            },
            formula,
            None,
        )
        .await
        .map_err(debug)?;
    let key = crate::map::boot::tests::door_key();
    for (command, level, expected) in [
        (1, 69, UseDisposition::Locked),
        (2, 70, UseDisposition::Committed),
    ] {
        let binding = owner
            .progression_binding(admitted.game_session_id)
            .ok_or("progression")?;
        let experience = binding
            .policy
            .thresholds
            .iter()
            .find(|t| t.level == level)
            .ok_or("threshold")?
            .minimum_experience
            .get();
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        let current = owner
            .root
            .read_character_progression(authority, fence.character_id)
            .await
            .map_err(debug)?
            .ok_or("progression state")?;
        let outcome = owner
            .revision_sequencer
            .acquire(fence.character_id)
            .await
            .commit_experience(
                owner.root,
                authority,
                owner.holder,
                fence,
                ExperienceAwardRequest {
                    occurrence: ExperienceRewardOccurrence::from_bytes(id(u8::try_from(level)?))
                        .map_err(debug)?,
                    amount: oteryn_simulation_determinism::ExactI64::new(
                        experience - current.total_experience.get(),
                    ),
                    context: binding.context.clone(),
                    policy_revision: binding.policy_revision.clone(),
                    reward_revision: binding.reward_revision.clone(),
                    policy: binding.policy.clone(),
                },
                None,
            )
            .await
            .map_err(debug)?;
        assert!(matches!(outcome, ExperienceCommitOutcome::Committed(_)));
        owner.states.level_refresh.mark(
            crate::gameplay_transport::level_up_live::leveled_character(&outcome),
        );
        Box::pin(owner.refresh_level_up_vitals(actor, admitted.game_session_id)).await;
        {
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            assert_eq!(
                states
                    .get(&runtime, actor, admitted.game_session_id)
                    .ok_or("live facts")?
                    .character_facts()
                    .level,
                level
            );
        }

        owner.clock_origin -= std::time::Duration::from_secs(1);
        let (next, result, _) =
            super::item_craft::exchange(owner, admitted, command, payload, None).await?;
        assert_eq!(result, expected, "source level{level}");
        assert_eq!(
            map.facts().doors().revision(key),
            if level == 69 { 0 } else { 1 }
        );
        admitted = next;
    }
    let (next, result, _) = super::item_craft::exchange(owner, admitted, 2, payload, None).await?;
    assert_eq!(result, UseDisposition::Committed);
    assert_eq!(map.facts().doors().revision(key), 1, "replay toggled twice");
    owner.clock_origin -= std::time::Duration::from_secs(1);
    let (_, result, _) = super::item_craft::exchange(owner, next, 3, payload, None).await?;
    assert_eq!(result, UseDisposition::StaleState);
    assert_eq!(map.facts().doors().revision(key), 1);
    Ok(())
}
