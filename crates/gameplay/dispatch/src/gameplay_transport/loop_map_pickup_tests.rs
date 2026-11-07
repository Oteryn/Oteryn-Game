//! MAP-PICKUP-1 through the production command-9 dispatch, composed authority and PostgreSQL.
//! Reuses the existing fresh-session Harness, Dependencies and activated production generation.

use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::item_mint::{
    GroundPlacement, ItemMintCause, ItemMintOutcome, ItemMintRequest,
};
use crate::durability::item_transfer::{ItemTransferDestination, ItemTransferRequest};
use crate::foundation::{ChannelRuntimeV1, CommandId, CommandRef, MovementLocalPosition};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::item_move::{ItemMoveStep, item_move};
use crate::gameplay_transport::item_ref_admission::{NativeItemWeights, generation_item_facts};
use crate::gameplay_transport::item_view::{ItemKey, ItemViewContinuity, SessionItemView};
use crate::gameplay_transport::world_map::{MapUpdate, SessionMapView};
use crate::map::boot::{BootPins, boot, tests as bundle_fixture};
use crate::map::facts::ItemDefinition;
use oteryn_protocol_oteryn::item_view::{
    ItemMoveDestination, ItemMoveIntent, ItemMoveOutcome, ItemMoveSelection,
    encode_item_move_intent_for,
};

const SELECTION: ItemMoveSelection = ItemMoveSelection {
    equip_drop: true,
    container_tree: true,
    world_interaction: true,
};

#[test]
fn map_pickup_dispatch_mints_moves_rehides_resends_and_replays_after_a_crash() -> TestResult {
    scenario(true, false, 0)
}

#[test]
fn map_pickup_dispatch_refusals_write_nothing_and_preserve_the_handle() -> TestResult {
    scenario(false, true, 0)
}

#[test]
fn map_pickup_dispatch_to_main_backpack_resends_its_tile() -> TestResult {
    scenario(false, false, 0)
}

#[test]
fn map_key_pickup_silver_source_binding_real_command9() -> TestResult {
    scenario_with_key(false, false, 0, Some((2969, 3007)))
}

#[test]
fn map_key_pickup_copper_binding_survives_mint_transfer_crash() -> TestResult {
    scenario_with_key(true, false, 0, Some((2970, 3008)))
}

#[test]
fn map_key_pickup_golden_full_bag_preserves_shared_stock() -> TestResult {
    scenario_with_key(false, true, 0, Some((2972, 3002)))
}

fn scenario(crash: bool, full: bool, epoch: u64) -> TestResult {
    scenario_with_key(crash, full, epoch, None)
}

fn scenario_with_key(
    crash: bool,
    full: bool,
    epoch: u64,
    source_key: Option<(u16, u16)>,
) -> TestResult {
    scenario_at(crash, full, epoch, source_key, None)
}

#[test]
fn map_native_key_pickup_draconia_3003_crash_replay() -> TestResult {
    native_key(3003, 32790, 31593, -7, true)
}
#[test]
fn map_native_key_pickup_draconia_3004() -> TestResult {
    native_key(3004, 32807, 31576, -6, false)
}
#[test]
fn map_native_key_pickup_draconia_3005() -> TestResult {
    native_key(3005, 32792, 31590, -6, false)
}
#[test]
fn map_native_key_pickup_draconia_3006() -> TestResult {
    native_key(3006, 32801, 31579, -6, false)
}
#[test]
fn map_native_key_pickup_draconia_3007() -> TestResult {
    native_key(3007, 32813, 31576, -5, false)
}

fn native_key(number: u16, x: u16, y: u16, floor: i8, crash: bool) -> TestResult {
    scenario_at(
        crash,
        false,
        0,
        Some((2969, number)),
        Some(crate::map::overlay::TilePos { x, y, floor }),
    )
}

fn scenario_at(
    crash: bool,
    full: bool,
    epoch: u64,
    source_key: Option<(u16, u16)>,
    native_position: Option<crate::map::overlay::TilePos>,
) -> TestResult {
    let source_at = native_position.unwrap_or(crate::map::overlay::TilePos {
        x: 7,
        y: 0,
        floor: -7,
    });
    let source_local = MovementLocalPosition {
        x: i32::from(source_at.x),
        y: i32::from(source_at.y),
        floor: -i16::from(source_at.floor),
    };
    // These three full database bootstraps share the test cluster. Serialize this fixture's
    // setup/teardown so concurrent migrations cannot exhaust the owner's fixed DB deadlines.
    static DATABASE_BOOT: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _database_boot = DATABASE_BOOT
        .lock()
        .map_err(|_| "map pickup test gate poisoned")?;
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "map_pickup_dispatch", true, false).await?;
        let started = std::time::Instant::now();
        if let Some((appearance, number)) = source_key {
            let version: String = sqlx::query_scalar("SHOW server_version")
                .fetch_one(&harness.pool)
                .await?;
            eprintln!(
                "QUESTPC-MAP-KEY-WIRE-PG version={version} appearance={appearance} key={number} crash={crash} full={full} native={native_position:?}"
            );
        }
        if epoch != 0 {
            let database: String = sqlx::query_scalar("SELECT current_database()")
                .fetch_one(&harness.pool)
                .await?;
            eprintln!("WORLD-RESET-MAP-PG: {database}, epoch={epoch}, full={full}");
        }
        super::reset();
        let world_id = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        assert_eq!(harness.root.active_world_reset_epoch(world_id).await?, None);
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activated_pin) =
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("active generation")?;
        let native = generation.native_gameplay().ok_or("native")?;
        let index = native.item_index().ok_or("item index")?;
        let backpack_definition = crate::gameplay_transport::item_look::typed_definition(
            index,
            index
                .definition_ref("Item", "oteryn:item.tibia.i2854", "definition-r1")
                .ok_or("backpack ref")?,
        )
        .ok_or("backpack definition")?;
        let source_item = source_key.map_or_else(
            || "oteryn:item.tibia.i3031".to_owned(),
            |(appearance, _)| format!("oteryn:item.tibia.i{appearance}"),
        );
        let coin_definition = crate::gameplay_transport::item_look::typed_definition(
            index,
            index
                .definition_ref("Item", &source_item, "definition-r1")
                .ok_or("coin ref")?,
        )
        .ok_or("coin definition")?;
        let backpack_facts = generation_item_facts(native, &backpack_definition).map_err(debug)?;
        let box_id = index
            .definition_ref(
                "Item",
                &backpack_definition.production_key,
                &backpack_definition.revision_ref,
            )
            .ok_or("box id")?
            .get()
            - 1;
        let coin_id = index
            .definition_ref(
                "Item",
                &coin_definition.production_key,
                &coin_definition.revision_ref,
            )
            .ok_or("coin id")?
            .get()
            - 1;
        let (mut bytes, mut bundle_pins) = bundle_fixture::bundle_with_item_ids(
            (&backpack_definition.production_key, box_id),
            (&coin_definition.production_key, coin_id),
        );
        if let Some((_, number)) = source_key {
            let mut bundle = oteryn_world_bundle::bundle::read(&bytes)?;
            let tile = bundle
                .sectors
                .iter_mut()
                .flat_map(|sector| &mut sector.tiles)
                .find(|tile| tile.x == 7 && tile.y == 0)
                .ok_or("key fixture tile")?;
            let item = tile
                .items
                .iter_mut()
                .filter(|item| item.depth == 0)
                .nth(1)
                .ok_or("key fixture item")?;
            if native_position.is_none() {
                item.attrs.action = Some(number);
            }
            item.attrs.count = None;
            if native_position.is_some() {
                // Only the controlled fixture is translated. Production selectors remain the
                // exact pinned startup positions; no synthetic aliases are introduced.
                let mut sectors = std::collections::BTreeMap::new();
                for sector in bundle.sectors {
                    for mut tile in sector.tiles {
                        tile.x += source_at.x - 7;
                        tile.y += source_at.y;
                        sectors
                            .entry((source_at.floor, tile.x / 32, tile.y / 32))
                            .or_insert_with(Vec::new)
                            .push(tile);
                    }
                }
                bundle.sectors = sectors
                    .into_iter()
                    .map(
                        |((floor, sx, sy), tiles)| oteryn_world_bundle::bundle::Sector {
                            floor,
                            sx,
                            sy,
                            tiles,
                        },
                    )
                    .collect();
                bundle.manifest.world.min_x = source_at.x - 7;
                bundle.manifest.world.max_x = source_at.x + 25;
                bundle.manifest.world.min_y = source_at.y;
                bundle.manifest.world.max_y = source_at.y + 32;
                bundle.manifest.world.floors = vec![source_at.floor];
            }
            bytes = oteryn_world_bundle_compiler::bundle::write_with_npcs(
                &bundle.manifest,
                &bundle.sectors,
                &bundle.spawns,
                &bundle.npcs,
            )?;
            bundle_pins.digest = oteryn_world_bundle::bundle::read(&bytes)?.digest;
        }
        let base = crate::map::load(&bytes, &bundle_pins).map_err(debug)?;
        let world = boot(
            &bytes,
            &BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: bundle_pins,
                start: source_at,
            },
            deps.admitted.world_id,
            deps.admitted.channel_id,
            |key| {
                Some(ItemDefinition {
                    reference: index.definition_ref("Item", key, "definition-r1")?,
                    solid: Some(key == backpack_definition.production_key),
                    blocks_projectile: false,
                    pickupable: key == coin_definition.production_key,
                    door: false,
                })
            },
        )
        .map_err(debug)?;
        let cells = world
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        if epoch != 0 {
            // An already activated pointer models boot after step 4. This is
            // map-consumer qualification, not evidence for the activation writer.
            sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,$7,$4,$5,$6,'TIBIA_SERVER_SAVE_BERLIN_06',1,clock_timestamp(),'2026-10')")
                .bind(world_id.as_bytes().as_slice()).bind(epoch.to_string()).bind(world.base().digest().as_slice()).bind(generation.identity().server_artifact_digest().as_slice()).bind(generation.identity().client_artifact_digest().as_slice()).bind(activated_pin.frame_binding().digest().as_slice()).bind(crate::map::overlay::map_revision(world.base())).execute(&harness.pool).await?;
            sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,$2::text::numeric)")
                .bind(world_id.as_bytes().as_slice()).bind(epoch.to_string()).execute(&harness.pool).await?;
        }
        let boot_epoch = harness
            .root
            .active_world_reset_epoch(world_id)
            .await?
            .unwrap_or(0);
        assert_eq!(boot_epoch, epoch);
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated_pin.frame_binding().digest(),
            world.map_revision_digest(),
            (source_local.x, source_local.y, source_local.floor),
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
        runtime
            .initialize_pinned_test_position(deps.actor, source_local)
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&harness, &authority);
        let drain =
            crate::gameplay_transport::gameplay_drain::GameplayDrain::at_reset_epoch(boot_epoch);
        owner.reset_epoch = drain.epoch();
        owner.active_generation = Some(generation);
        owner.movement_cells = &cells;
        let session = deps.admitted.game_session_id;
        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(
            harness.root.clone(),
        )
        .current_session_at(session)
        .await?
        .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
        let command_ref = |n| -> TestResult<CommandRef> {
            Ok(CommandRef::new(session, CommandId::new(n).map_err(debug)?))
        };
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice()).bind(activated_pin.frame_binding().digest().as_slice()).execute(&harness.pool).await?;
        // Seed only the carried target, using the existing MINT/TRANSFER owners. The behavior
        // under test starts below at the real dispatch, with a map handle issued by domain 17.
        let mut carried = None;
        for ordinal in 0..=(if full { 20 } else { 1 }) {
            let mut seed = harness
                .root
                .freeze_item_mint(
                    &authority,
                    &harness.node,
                    ItemMintRequest {
                        cause: ItemMintCause::for_test(
                            deps.admitted.world_id,
                            deps.admitted.channel_id,
                            fence.scope_ownership_generation,
                            ordinal + 1,
                            1,
                            crate::durability::item_mint::TypedDefinitionRef {
                                family: "LootTable".into(),
                                production_key: "fixture:loot.map-target".into(),
                                revision_ref: "definition-r1".into(),
                            },
                            "fixture:purpose.map-target".into(),
                            0,
                        ),
                        item: backpack_definition.clone(),
                        quantity: 1,
                        ground: GroundPlacement {
                            spatial_position:
                                crate::durability::map_item_mint_audit::map_item_spatial_position(
                                    source_at.x,
                                    source_at.y,
                                    source_at.floor,
                                ),
                            corpse_ref: id(3).to_vec(),
                            map_revision: world.map_revision().into(),
                            content_revision: "content-1".into(),
                            native_room_placement_context: world.base().digest().to_vec(),
                        },
                        content_revision: "content-1".into(),
                        ruleset_revision: "ruleset-1".into(),
                        sim_revision: "sim-1".into(),
                    },
                )
                .await
                .map_err(debug)?;
            let permit = owner.spell_lane_permit().await.ok_or("lane")?;
            let backpack = match harness
                .root
                .commit_item_mint(&permit, &authority, &harness.node, &mut seed)
                .await
                .map_err(debug)?
            {
                ItemMintOutcome::Committed(item) => item.item_instance_id,
                other => return Err(debug(other)),
            };
            let mut transfer = harness
                .root
                .freeze_item_transfer(
                    &authority,
                    &harness.node,
                    fence,
                    ItemTransferRequest {
                        weight_generation_digest: native.source_digest(),
                        command: command_ref(100 + u64::from(ordinal))?,
                        source_item_instance_id: backpack,
                        destination: if ordinal == 0 {
                            ItemTransferDestination::ContainerSlot
                        } else {
                            ItemTransferDestination::MainBackpack
                        },
                        item: backpack_facts.clone(),
                        backpack: (ordinal != 0).then(|| backpack_facts.clone()),
                        content_revision: "content-1".into(),
                        ruleset_revision: "ruleset-1".into(),
                        sim_revision: "sim-1".into(),
                    },
                    &NativeItemWeights(native),
                )
                .await
                .map_err(debug)?;
            harness
                .root
                .commit_item_transfer(
                    &permit,
                    &authority,
                    &harness.node,
                    fence,
                    &mut transfer,
                    &NativeItemWeights(native),
                )
                .await
                .map_err(debug)?;
            drop(permit);
            if ordinal == 1 {
                carried = Some(backpack);
            }
        }
        let mut items = SessionItemView::resume(ItemViewContinuity::default())
            .with_equip_drop()
            .with_container_tree()
            .with_map_view();
        items
            .snapshot(
                owner
                    .observe_character_inventory(deps.actor, session)
                    .await
                    .ok_or("inventory")?,
                None,
            )
            .map_err(debug)?;
        let mut map = SessionMapView::default();
        let Some(MapUpdate::Snapshot(snapshot)) = owner
            .observe_world_map(deps.actor, &mut map, &mut items)
            .await
            .ok_or("map")?
            .map_err(debug)?
        else {
            return Err("map snapshot".into());
        };
        let snapshot =
            oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(&snapshot.payload)
                .map_err(debug)?;
        assert_eq!(snapshot.header.reset_epoch, epoch);
        let key = ItemKey::MapBase {
            bundle_digest: world.base().digest(),
            placement_key: crate::map::view::placement_key(source_at, 1).ok_or("key")?,
            reset_epoch: epoch,
        };
        let handle = items.table().handle(&key).ok_or("map handle")?;
        if epoch != 0 {
            assert!(matches!(
                crate::gameplay_transport::item_look::look_item(
                    &owner,
                    deps.actor,
                    session,
                    Some(fence),
                    &items,
                    handle,
                )
                .await,
                oteryn_protocol_oteryn::item_look::ItemLookResult::Text(_)
            ));
        }
        let payload = encode_item_move_intent_for(
            &ItemMoveIntent {
                source: handle,
                destination: if crash {
                    ItemMoveDestination::Container(
                        items
                            .table()
                            .handle(&ItemKey::Instance(carried.ok_or("carried bag")?))
                            .ok_or("carried handle")?,
                    )
                } else {
                    ItemMoveDestination::MainBackpack
                },
            },
            SELECTION,
        )
        .map_err(debug)?;
        let command = UseCommand {
            game_session_id: session,
            command_id: 20,
            item_fence: Some(fence),
        };
        let before = footprint(&harness.pool).await?;
        if full {
            // A full real backpack refuses before MINT; reach and stale identities likewise
            // preserve both the durable footprint and the issued source handle.
            for (x, assert_far) in [(source_local.x - 6, true), (source_local.x, false)] {
                let mut runtime = owner.runtime.lock().await;
                let mut movement = runtime.borrow_movement_position();
                loop {
                    let snapshot = movement.read(deps.actor).map_err(debug)?;
                    let at = snapshot.position();
                    if at.x == x {
                        break;
                    }
                    movement
                        .commit_cardinal(
                            snapshot,
                            MovementLocalPosition {
                                x: at.x + (x - at.x).signum(),
                                ..at
                            },
                        )
                        .map_err(debug)?;
                }
                drop(runtime);
                if assert_far {
                    assert_eq!(
                        item_move(
                            &owner, deps.actor, command, &payload, SELECTION, false, &mut items
                        )
                        .await,
                        ItemMoveStep::Result(ItemMoveOutcome::TooFar, vec![])
                    );
                    assert_eq!(footprint(&harness.pool).await?, before);
                    assert_eq!(items.resolve(handle), Some(key));
                }
            }
            assert_eq!(
                item_move(
                    &owner, deps.actor, command, &payload, SELECTION, false, &mut items
                )
                .await,
                ItemMoveStep::Result(ItemMoveOutcome::NoRoom, vec![])
            );
            assert_eq!(footprint(&harness.pool).await?, before);
            assert_eq!(items.resolve(handle), Some(key));
            for stale in [
                ItemKey::MapBase {
                    bundle_digest: [9; 32],
                    placement_key: match key {
                        ItemKey::MapBase { placement_key, .. } => placement_key,
                        _ => return Err("key".into()),
                    },
                    reset_epoch: epoch,
                },
                ItemKey::MapBase {
                    bundle_digest: world.base().digest(),
                    placement_key: match key {
                        ItemKey::MapBase { placement_key, .. } => placement_key,
                        _ => return Err("key".into()),
                    },
                    reset_epoch: epoch + 1,
                },
            ] {
                items.map_view(&[stale], |_| Ok(())).map_err(debug)?;
                let handle = items.table().handle(&stale).ok_or("stale handle")?;
                let payload = encode_item_move_intent_for(
                    &ItemMoveIntent {
                        source: handle,
                        destination: ItemMoveDestination::MainBackpack,
                    },
                    SELECTION,
                )
                .map_err(debug)?;
                assert_eq!(
                    item_move(
                        &owner, deps.actor, command, &payload, SELECTION, false, &mut items
                    )
                    .await,
                    ItemMoveStep::Result(ItemMoveOutcome::Stale, vec![])
                );
                assert_eq!(footprint(&harness.pool).await?, before);
                if epoch != 0 {
                    assert_eq!(
                        crate::gameplay_transport::item_look::look_item(
                            &owner,
                            deps.actor,
                            session,
                            Some(fence),
                            &items,
                            handle,
                        )
                        .await,
                        oteryn_protocol_oteryn::item_look::ItemLookResult::Refused(
                            oteryn_protocol_oteryn::item_look::ItemLookRefusal::NotAvailable
                        )
                    );
                }
                assert_eq!(items.resolve(handle), Some(stale));
            }
        } else {
            if crash {
                // Inject the process boundary after MINT by refusing the subsequent move receipt.
                sqlx::raw_sql("CREATE FUNCTION test_map_pickup_crash() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'map pickup crash boundary'; END $$; CREATE TRIGGER test_map_pickup_crash BEFORE INSERT ON game_item_loop_move_receipts FOR EACH ROW EXECUTE FUNCTION test_map_pickup_crash()")
                .execute(&harness.pool).await?;
                assert_eq!(
                    item_move(
                        &owner, deps.actor, command, &payload, SELECTION, false, &mut items
                    )
                    .await,
                    ItemMoveStep::Disconnect
                );
                assert_eq!(mint_and_move_counts(&harness.pool).await?, (1, 0, 1));
                // Rebuild the cache, just as after process loss: the base stays hidden and the one
                // minted instance is on Ground. Retain the request's session handle for its resend.
                std::sync::Arc::get_mut(&mut owner.states)
                    .expect("W02: isolated fixture state")
                    .loop_ground = Default::default();
                let ground = crate::gameplay_transport::loop_move::loop_ground(
                    &owner,
                    world.base(),
                    Some(index),
                )
                .await
                .ok_or("rebuilt ground")?;
                let (pos, ordinal) = crate::map::view::placement_of(match key {
                    ItemKey::MapBase { placement_key, .. } => placement_key,
                    _ => return Err("key".into()),
                })
                .ok_or("placement")?;
                assert!(
                    ground
                        .overlay()
                        .tile(pos)
                        .ok_or("hidden tile")?
                        .is_hidden(ordinal)
                );
                sqlx::raw_sql("DROP TRIGGER test_map_pickup_crash ON game_item_loop_move_receipts; DROP FUNCTION test_map_pickup_crash()") .execute(&harness.pool).await?;
            }
            assert!(matches!(
                item_move(
                    &owner, deps.actor, command, &payload, SELECTION, false, &mut items
                )
                .await,
                ItemMoveStep::Result(ItemMoveOutcome::Moved, _)
            ));
            assert_eq!(mint_and_move_counts(&harness.pool).await?, (1, 1, 0));
            let moved = footprint(&harness.pool).await?;
            assert!(matches!(
                item_move(
                    &owner, deps.actor, command, &payload, SELECTION, false, &mut items
                )
                .await,
                ItemMoveStep::Result(ItemMoveOutcome::Moved, _)
            ));
            assert_eq!(footprint(&harness.pool).await?, moved);
            let second = UseCommand {
                command_id: 21,
                ..command
            };
            assert_eq!(
                item_move(
                    &owner, deps.actor, second, &payload, SELECTION, false, &mut items
                )
                .await,
                ItemMoveStep::Result(ItemMoveOutcome::Stale, vec![])
            );
            assert_eq!(footprint(&harness.pool).await?, moved);
            let Some(MapUpdate::Delta(delta)) = owner
                .observe_world_map(deps.actor, &mut map, &mut items)
                .await
                .ok_or("resend")?
                .map_err(debug)?
            else {
                return Err("tile delta".into());
            };
            let delta = oteryn_protocol_oteryn::world_map::decode_world_map_delta(
                &snapshot.header,
                &delta.payload,
            )
            .map_err(debug)?;
            let tile = delta
                .tiles
                .iter()
                .find(|tile| {
                    tile.position.x == i32::from(source_at.x)
                        && tile.position.y == i32::from(source_at.y)
                        && tile.position.floor == i16::from(source_at.floor)
                })
                .ok_or("origin tile resent")?;
            assert_eq!(
                tile.items.len(),
                1,
                "only the ground remains on the taken origin"
            );
            assert_eq!(items.resolve(handle), None);
        }
        if let Some((_, number)) = source_key {
            let bindings: Vec<i32> =
                sqlx::query_scalar("SELECT key_number FROM game_item_keys ORDER BY key_number")
                    .fetch_all(&harness.pool)
                    .await?;
            assert_eq!(
                bindings,
                if full {
                    Vec::new()
                } else {
                    vec![i32::from(number)]
                }
            );
            eprintln!(
                "QUESTPC-MAP-KEY-WIRE-PG GREEN key={number} crash={crash} full={full} native={native_position:?} duration={:?}",
                started.elapsed()
            );
        }
        // Both durable readers fail closed. No boot-overlay fallback may re-show an origin.
        for relation in ["game_map_item_mint_receipts", "game_item_ground_locations"] {
            owner.states.loop_ground.invalidate();
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "ALTER TABLE {relation} RENAME TO test_unavailable_map_source"
            )))
            .execute(&harness.pool)
            .await?;
            assert!(
                owner
                    .observe_world_map(deps.actor, &mut map, &mut items)
                    .await
                    .is_none()
            );
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "ALTER TABLE test_unavailable_map_source RENAME TO {relation}"
            )))
            .execute(&harness.pool)
            .await?;
        }
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

async fn mint_and_move_counts(pool: &sqlx::PgPool) -> TestResult<(i64, i64, i64)> {
    let counts: (i64, i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_map_item_mint_receipts), (SELECT count(*) FROM game_item_loop_move_receipts), (SELECT count(*) FROM game_item_ground_locations WHERE corpse_ref = convert_to('map-item-materialization-v1','UTF8'))").fetch_one(pool).await?;
    let custody: i64 = sqlx::query_scalar("SELECT (SELECT count(*) FROM game_map_item_mint_receipts m JOIN game_item_ground_locations g USING(item_instance_id)) + (SELECT count(*) FROM game_map_item_mint_receipts m JOIN game_item_container_entries e USING(item_instance_id)) + (SELECT count(*) FROM game_map_item_mint_receipts m JOIN game_item_nested_entries n USING(item_instance_id)) + (SELECT count(*) FROM game_map_item_mint_receipts m JOIN game_item_container_slots s USING(item_instance_id))").fetch_one(pool).await?;
    assert_eq!(
        custody, counts.0,
        "each minted item has exactly one custody location"
    );
    Ok(counts)
}

async fn footprint(pool: &sqlx::PgPool) -> TestResult<Vec<i64>> {
    // Include reservations and charged work, so a refused request cannot hide a partial write.
    let row: (i64, i64, i64, i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_item_instances), (SELECT count(*) FROM game_map_item_mint_reservations), (SELECT COALESCE(sum(work_units_used),0)::bigint FROM game_map_item_mint_reservations), (SELECT count(*) FROM game_map_item_mint_receipts), (SELECT count(*) FROM game_item_loop_move_receipts)").fetch_one(pool).await?;
    Ok(vec![row.0, row.1, row.2, row.3, row.4])
}

#[path = "world_reset_map_pickup_postgres_tests.rs"]
mod world_reset;
