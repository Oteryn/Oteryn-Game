//! M21: map-source containers retain shared custody, reset identity and key provenance.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::gameplay_transport::container_view::ContainerObservation;
use crate::gameplay_transport::item_view::ItemKey;
use crate::map::boot::{BootPins, boot, tests as bundle_fixture};
use crate::map::facts::ItemDefinition;

#[test]
fn qmech_m21_postgres_map_container_source_view() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_STARTED M21 map container source view");
    let result = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| source_view(false).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "M21 source-view thread panicked".to_owned())
        .flatten();
    eprintln!(
        "POSTGRES_CASE_COMPLETED M21 map container source view duration={:.6}s success={}",
        started.elapsed().as_secs_f64(),
        result.is_ok()
    );
    result?;
    Ok(())
}

fn source_view(nested: bool) -> TestResult {
    run(async move |admin| {
        let h = Harness::create_with_admission_seed(admin, "qmech_m21_source_view", true, false)
            .await?;
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
                super::death_ml_loss_composed::production_generation_with_pin(
                    deps.admitted.world_id,
                    "content-1",
                )?;
            let generation = controller.active().ok_or("active generation")?;
            let native = generation.native_gameplay().ok_or("native")?;
            let index = native.item_index().ok_or("native Item index")?;
            let container_key = "oteryn:item.tibia.i2854";
            let leaf_key = if nested {
                "oteryn:item.tibia.i2970"
            } else {
                "oteryn:item.tibia.i3031"
            };
            let reference = |key| {
                index
                    .definition_ref("Item", key, "definition-r1")
                    .ok_or("fixture Item reference")
            };
            let container = reference(container_key)?;
            let leaf = reference(leaf_key)?;
            let (mut bytes, mut pins) = bundle_fixture::bundle_with_item_ids(
                (container_key, container.get() - 1),
                (leaf_key, leaf.get() - 1),
            );
            if nested {
                let mut bundle = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
                let tile = bundle.sectors[0]
                    .tiles
                    .iter_mut()
                    .find(|t| t.x == 6)
                    .ok_or("source tile")?;
                tile.items.push(oteryn_world_bundle::sector::Item {
                    palette: 3,
                    depth: 1,
                    attrs: Default::default(),
                });
                tile.items.push(oteryn_world_bundle::sector::Item {
                    palette: 4,
                    depth: 2,
                    attrs: oteryn_world_bundle::sector::Attrs {
                        action: Some(3301),
                        ..Default::default()
                    },
                });
                bytes = oteryn_world_bundle::writer::write(
                    &bundle.manifest,
                    &bundle.sectors,
                    &bundle.spawns,
                )
                .map_err(debug)?;
                pins.digest = oteryn_world_bundle::bundle::read(&bytes)
                    .map_err(debug)?
                    .digest;
            }
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
            let session = deps.admitted.game_session_id;
            let current =
                crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                    .current_session_at(session)
                    .await?
                    .0;
            let fence = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
            if nested {
                deps.initialize(&owner).await;
                sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
                    .bind(id(41).as_slice()).execute(&h.pool).await?;
                sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
                    .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
                sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
                    .bind(id(42).as_slice()).bind(id(43).as_slice())
                    .bind(generation.identity().server_artifact_digest().as_slice())
                    .bind(generation.identity().client_artifact_digest().as_slice())
                    .bind(activated_pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
                super::item_look::seed_main_backpack(&h, &[]).await?;
            }
            let key = ItemKey::MapBase {
                bundle_digest: world.base().digest(),
                placement_key: crate::map::view::placement_key(
                    crate::map::overlay::TilePos {
                        x: 6,
                        y: 0,
                        floor: -7,
                    },
                    1,
                )
                .ok_or("source placement")?,
                reset_epoch: 0,
            };
            let observed = owner.observe_container(deps.actor, Some(fence), key).await;
            match observed {
                Some(ContainerObservation::Container {
                    parent: None,
                    capacity: 20,
                    entries,
                }) if entries.len() == usize::from(nested) => {}
                other => {
                    return Err(format!(
                        "M21: qualified empty map container must open; observed {other:?}"
                    )
                    .into());
                }
            }
            if nested {
                use crate::gameplay_transport::item_move::{LoopMove, LoopMoveResult, LoopTarget};
                let source_key = |ordinal| ItemKey::MapBase {
                    bundle_digest: world.base().digest(),
                    placement_key: crate::map::view::placement_key(
                        crate::map::overlay::TilePos {
                            x: 6,
                            y: 0,
                            floor: -7,
                        },
                        ordinal,
                    )
                    .expect("source address"),
                    reset_epoch: 0,
                };
                let inner = source_key(2);
                match owner
                    .observe_container(deps.actor, Some(fence), inner)
                    .await
                {
                    Some(ContainerObservation::Container {
                        parent: Some(parent),
                        entries,
                        ..
                    }) => {
                        assert_eq!(parent, key);
                        assert_eq!(entries.len(), 1);
                        assert_eq!(entries[0].key, source_key(3));
                    }
                    other => return Err(format!("nested source must open: {other:?}").into()),
                }
                let result = owner
                    .move_loop_item(
                        deps.actor,
                        crate::gameplay_transport::connection::UseCommand {
                            game_session_id: session,
                            command_id: 700,
                            item_fence: Some(fence),
                        },
                        LoopMove {
                            item: source_key(3),
                            count: None,
                            target: LoopTarget::MainBackpack,
                        },
                    )
                    .await;
                assert_eq!(result, LoopMoveResult::Committed);
                let number: i32 = sqlx::query_scalar(
                    "SELECT key_number FROM game_item_keys WHERE key_number=3301",
                )
                .fetch_one(&h.pool)
                .await?;
                assert_eq!(number, 3301);
                let stale = owner.move_loop_item(deps.actor,
                    crate::gameplay_transport::connection::UseCommand { game_session_id: session, command_id: 701, item_fence: Some(fence) },
                    LoopMove { item: source_key(3), count: None, target: LoopTarget::MainBackpack }).await;
                assert_eq!(stale, LoopMoveResult::Refused(crate::gameplay_transport::item_move::LoopMoveRefusal::Stale));
                let stale_epoch = ItemKey::MapBase {
                    bundle_digest: world.base().digest(),
                    placement_key: crate::map::view::placement_key(crate::map::overlay::TilePos { x: 6, y: 0, floor: -7 }, 1).ok_or("root key")?,
                    reset_epoch: 99,
                };
                assert!(matches!(owner.observe_container(deps.actor, Some(fence), stale_epoch).await, Some(ContainerObservation::Gone)));

                match owner
                    .observe_container(deps.actor, Some(fence), inner)
                    .await
                {
                    Some(ContainerObservation::Container { entries, .. }) => {
                        assert!(entries.is_empty())
                    }
                    other => return Err(format!("taken source view: {other:?}").into()),
                }
            }
            let receipts: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_map_item_mint_receipts")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(
                receipts,
                i64::from(nested),
                "opening the captured source does not mint or transfer custody"
            );
            Ok(())
        }
        .await;
        h.cleanup().await?;
        result
    })
}

#[test]
fn qmech_m21_postgres_nested_key_pickup() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_STARTED M21 nested key pickup");
    let result = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| source_view(true).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "M21 nested key thread panicked".to_owned())
        .flatten();
    eprintln!(
        "POSTGRES_CASE_COMPLETED M21 nested key pickup duration={:.6}s success={}",
        started.elapsed().as_secs_f64(),
        result.is_ok()
    );
    result?;
    Ok(())
}
