//! Counted command 9 -> real composed dispatch -> existing fenced loop receipts -> reload.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS,
    serve_admitted,
};
use oteryn_protocol_oteryn::item_view::*;
use tokio::io::AsyncWriteExt;

#[test]
fn stack_quantity_dispatch_split_merge_drop_replay_and_atomic_refusals() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "stack quantity e2e panic")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "stack_quantity", true, false).await?;
        super::reset();
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(
                deps.admitted.world_id,
                None,
            );
        let active = controller.active().ok_or("active")?;
        let native = active.native_gameplay().ok_or("native")?;
        let index = native.item_index().ok_or("index")?;
        let coin = "oteryn:item.tibia.i3031";
        let bag = "oteryn:item.tibia.i2854";
        let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
        let (bytes, pins) = crate::map::boot::tests::bundle_with_item_ids(
            (bag, reference(bag).ok_or("bag")?.get() - 1),
            (coin, reference(coin).ok_or("oven")?.get() - 1),
        );
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let map = crate::map::boot::boot(
            &bytes,
            &crate::map::boot::BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: pins,
                start: crate::map::overlay::TilePos {
                    x: 6,
                    y: 0,
                    floor: -7,
                },
            },
            deps.admitted.world_id,
            deps.admitted.channel_id,
            |key| {
                let policy = native.item_policy(key, "definition-r1")?.record();
                let crate::content::ReferenceItemField::Known(physical) =
                    &policy.semantics.physical
                else {
                    return None;
                };
                Some(crate::map::facts::ItemDefinition {
                    reference: reference(key)?,
                    solid: policy.attributes.blocks_movement,
                    blocks_projectile: policy.attributes.blocks_projectile.unwrap_or(true),
                    pickupable: physical.pickupable
                        == crate::content::ReferenceItemField::Known(true),
                    door: false,
                })
            },
        )
        .map_err(debug)?;
        let cells = map
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        let content_pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            pin.qualified_room().map_revision_digest(),
            (6, 0, 7),
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
                    x: 6,
                    y: 0,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = &cells;
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
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&harness.pool).await?;
        super::item_look::seed_main_backpack(&harness, &[(coin, 100), (bag, 1)]).await?;
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[4, 6, 12, 14, 18, CAPABILITY_ITEM_STACK_COUNT_V1],
        )
        .ok_or("count capability selectable")?;
        // Split 30 into the visible carried bag. Source keeps its identity and 70 units.
        let (next, result, payload) = exchange(&owner, admitted, 1, None, 30, true, true).await?;
        assert_eq!(result, ItemMoveOutcome::Moved);
        assert_eq!(quantity(&harness, id(71)).await?, 70);
        let nested: (Vec<u8>, i64) = sqlx::query_as("SELECT uuid_send(i.item_instance_id), i.quantity FROM game_item_nested_entries n JOIN game_item_instances i USING(item_instance_id) WHERE n.parent_item_instance_id=encode($1,'hex')::uuid")
            .bind(id(72).as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!(nested.1, 30);
        // Move 10 back through a real container-view handle: merge into the main stack.
        let nested_id: [u8; 16] = nested.0.try_into().map_err(|_| "nested identity")?;
        let (next, result, _) = exchange(&owner, next, 3, None, 10, false, true).await?;
        assert_eq!(result, ItemMoveOutcome::Moved);
        assert_eq!(quantity(&harness, id(71)).await?, 80);
        assert_eq!(quantity(&harness, nested_id).await?, 20);
        // Split inside the same container twice: the second split must not auto-merge
        // into the sibling that the first split created.
        let (next, result, _) = exchange(&owner, next, 4, None, 20, false, false).await?;
        assert_eq!(result, ItemMoveOutcome::Moved);
        let (next, result, _) = exchange(&owner, next, 5, None, 10, false, false).await?;
        assert_eq!(result, ItemMoveOutcome::Moved);
        assert_eq!(quantity(&harness, id(71)).await?, 50);
        let quantities: Vec<i64> = sqlx::query_scalar("SELECT i.quantity FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key=$1 ORDER BY i.quantity")
            .bind(coin).fetch_all(&harness.pool).await?;
        assert_eq!(quantities, [10, 20, 50]);
        let (next, result, _) = exchange(&owner, next, 6, None, 10, true, false).await?;
        assert_eq!(result, ItemMoveOutcome::Moved);
        assert_eq!(quantity(&harness, id(71)).await?, 40);
        let dropped: i64 = sqlx::query_scalar("SELECT i.quantity FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id)").fetch_one(&harness.pool).await?;
        assert_eq!(dropped, 10);
        // Decode failures and an over-source count never write a receipt.
        let mut next = next;
        for (command, count) in [(7, 0), (8, 101), (9, 41)] {
            let (end, result, _) =
                exchange(&owner, next, command, None, count, false, false).await?;
            assert_eq!(result, ItemMoveOutcome::Rejected);
            next = end;
        }
        assert_eq!(quantity(&harness, id(71)).await?, 40);
        let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_loop_move_receipts")
            .fetch_one(&harness.pool)
            .await?;
        assert_eq!(receipts, 5);
        // Fill the remaining 16 slots as fixture state; a counted in-place split then
        // fails atomically, with no source shrink, fresh item, or receipt left behind.
        fill_backpack(&harness).await?;
        let (next, result, _) = exchange(&owner, next, 10, None, 1, false, false).await?;
        assert_eq!(result, ItemMoveOutcome::NoRoom);
        assert_eq!(quantity(&harness, id(71)).await?, 40);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_instances")
                .fetch_one(&harness.pool)
                .await?,
            23
        );
        // Without negotiated count support the same field is rejected by real dispatch.
        let mut legacy = next;
        legacy.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 12, 14, 18])
                .ok_or("legacy selection")?;
        let (_, result, _) = exchange(&owner, legacy, 11, None, 1, false, false).await?;
        assert_eq!(result, ItemMoveOutcome::Rejected);
        // Restart the composed owner and replay the original command before resolving its
        // now-stale source handle: no second split and no additional durable receipt.
        drop(owner);
        let mut restarted = deps.owner(&harness, &authority);
        restarted.active_generation = Some(active);
        restarted.movement_cells = &cells;
        deps.initialize(&restarted).await;
        let mut replay_admitted = admitted;
        replay_admitted.continuity.item_view.handle_counter = 1000;
        let (_, result, _) = exchange(
            &restarted,
            replay_admitted,
            1,
            Some(payload),
            30,
            true,
            true,
        )
        .await?;
        assert_eq!(result, ItemMoveOutcome::Moved);
        assert_eq!(quantity(&harness, id(71)).await?, 40);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_loop_move_receipts")
                .fetch_one(&harness.pool)
                .await?,
            receipts
        );
        drop(restarted);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

async fn quantity(harness: &Harness, item: [u8; 16]) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(item.as_slice())
    .fetch_one(&harness.pool)
    .await?)
}

#[allow(clippy::too_many_arguments)]
async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
    replay: Option<Vec<u8>>,
    count: u8,
    to: bool,
    nested: bool,
) -> TestResult<(AdmittedSession, ItemMoveOutcome, Vec<u8>)> {
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let client_task = async {
        let mut source = None;
        let mut bag = None;
        loop {
            let raw = super::item_look::read_frame(&mut client).await?;
            let envelope = crate::foundation::decode_wire_envelope(&raw)?;
            if envelope.message_type() == crate::foundation::MessageType::SnapshotChunk {
                let (_, domains) =
                    oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload())?;
                for domain in domains {
                    if domain.domain_id == STATE_DOMAIN_CHARACTER_INVENTORY {
                        let inventory =
                            decode_character_inventory_with_equip_drop(domain.payload, true)
                                .map_err(debug)?;
                        source = inventory
                            .entries
                            .iter()
                            .max_by_key(|entry| entry.count)
                            .map(|entry| entry.handle);
                        bag = inventory
                            .entries
                            .iter()
                            .find(|entry| entry.count.get() == 1)
                            .map(|entry| entry.handle);
                    }
                }
            }
            if envelope.message_type() == crate::foundation::MessageType::SnapshotCommit {
                break;
            }
        }
        let actual = if let Some(payload) = replay {
            payload
        } else {
            if nested && !to {
                use oteryn_protocol_oteryn::container_tree::*;
                let open = encode_container_view_intent(&ContainerViewIntent::Open {
                    handle: bag.ok_or("visible carried bag")?,
                    replace_view: None,
                })
                .map_err(debug)?;
                send(
                    &mut client,
                    command_id - 1,
                    COMMAND_TYPE_CONTAINER_VIEW_INTENT,
                    &open,
                )
                .await?;
                loop {
                    let raw = super::item_look::read_frame(&mut client).await?;
                    let envelope = crate::foundation::decode_wire_envelope(&raw)?;
                    if envelope.message_type() == crate::foundation::MessageType::StateDelta {
                        let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())?;
                        if delta.domain_id == STATE_DOMAIN_CONTAINER_VIEWS {
                            let views = decode_container_views(delta.payload).map_err(debug)?;
                            source = views
                                .views
                                .iter()
                                .flat_map(|view| &view.entries)
                                .next()
                                .map(|entry| entry.handle);
                            break;
                        }
                    }
                }
            }
            let destination = match (to, nested) {
                (true, true) => ItemMoveDestination::Container(bag.ok_or("carried bag")?),
                (true, false) => ItemMoveDestination::Ground(WorldTilePosition {
                    x: 6,
                    y: 0,
                    floor: -7,
                }),
                (false, _) => ItemMoveDestination::MainBackpack,
            };
            let intent = ItemMoveIntent {
                source: source.ok_or("current source")?,
                destination,
            };
            let mut payload = encode_item_move_intent_for(
                &intent,
                ItemMoveSelection {
                    equip_drop: true,
                    container_tree: true,
                    ..ItemMoveSelection::default()
                },
            )
            .map_err(debug)?;
            payload.extend([0x30, count]);
            payload
        };
        send(
            &mut client,
            command_id,
            COMMAND_TYPE_ITEM_MOVE_INTENT,
            &actual,
        )
        .await?;
        client.shutdown().await?;
        loop {
            let raw = super::item_look::read_frame(&mut client).await?;
            let envelope = crate::foundation::decode_wire_envelope(&raw)?;
            if envelope.message_type() == crate::foundation::MessageType::CommandResult {
                let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
                if result.command_id == command_id {
                    let disposition = decode_item_move_result_with_equip_drop(result.payload, true)
                        .map_err(debug)?;
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
    let ConnectionEnd::AdmittedThenDisconnected(admitted) = end.ok_or("server did not finish")?
    else {
        return Err("server ended before admission".into());
    };
    let (result, actual) = result.ok_or("client did not finish")??;
    Ok((admitted, result, actual))
}

async fn send(
    client: &mut tokio::io::DuplexStream,
    command_id: u64,
    command_type: u32,
    payload: &[u8],
) -> TestResult {
    let frame = oteryn_protocol_oteryn::encode_client_command(
        1,
        &oteryn_protocol_oteryn::ClientCommandValue {
            command_id,
            command_type,
            payload,
        },
    )?;
    client.write_u32(u32::try_from(frame.len())?).await?;
    client.write_all(&frame).await?;
    Ok(())
}

/// Only fixture construction bypasses triggers, as in `seed_main_backpack`.
async fn fill_backpack(harness: &Harness) -> TestResult {
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    for seed in 100..116 {
        sqlx::query("INSERT INTO game_item_instances(item_instance_id, world_id, definition_family, definition_production_key, definition_revision_ref, quantity, lifecycle, minted_transaction_id) VALUES(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 'Item', 'oteryn:item.tibia.i2854', 'definition-r1', 1, 1, encode($3,'hex')::uuid)")
            .bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(id(69).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_container_entries(character_id, world_id, parent_item_instance_id, item_instance_id, placement_ordinal, placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,encode($6,'hex')::uuid)")
            .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(id(70).as_slice()).bind(id(seed).as_slice()).bind(i64::from(seed)+1000).bind(id(69).as_slice()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}
