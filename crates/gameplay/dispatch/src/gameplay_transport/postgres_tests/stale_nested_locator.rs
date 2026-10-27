//! F32: cached presentation positions must never redirect a durable nested-bag move.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, item_look, prepare_current_session};
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::FirstEntryOutcome;
use oteryn_protocol_oteryn::{self as protocol, container_tree::ContainerViewIntent};
use protocol::item_view::{ItemHandle, ItemMoveDestination, ItemMoveIntent, ItemMoveSelection};
use tokio::io::AsyncWriteExt;

#[derive(Clone, Copy, Debug)]
enum Mutation {
    Merge,
    Remove,
    Sort,
    Reopen,
    FullViewShrink,
}

#[test]
fn f32_parent_stack_merge() -> TestResult {
    scenario(|| exercise(Mutation::Merge))
}
#[test]
fn f32_parent_sibling_removal() -> TestResult {
    scenario(|| exercise(Mutation::Remove))
}
#[test]
fn f32_parent_reorder_sort() -> TestResult {
    scenario(|| exercise(Mutation::Sort))
}
#[test]
fn f32_nested_close_reopen() -> TestResult {
    scenario(|| exercise(Mutation::Reopen))
}
#[test]
fn f32_full_view_boundary_shrink() -> TestResult {
    scenario(|| exercise(Mutation::FullViewShrink))
}

fn scenario(exercise: fn() -> TestResult) -> TestResult {
    // Match the existing live-container fixture's stack requirement.
    super::kill_reward_e2e::scenario_thread::run("f32-locator", exercise)
}
fn exercise(mutation: Mutation) -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "f32_locator", true, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) =
            super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
        let generation = controller.active().ok_or("active")?;
        let room = pin.qualified_room().clone();
        let frame_digest = pin.frame_binding().digest();
        let deps = Dependencies::with_content(
            pin.into_channel_pin(),
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        deps.runtime
            .lock()
            .await
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: 7,
                    y: 0,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(generation);
        deps.initialize(&owner).await;
        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(
            harness.root.clone(),
        )
        .current_session_at(deps.admitted.game_session_id)
        .await?
        .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice()).bind(generation.identity().client_artifact_digest().as_slice()).bind(frame_digest.as_slice()).execute(&harness.pool).await?;

        // A = 70, B = 72, shadow = 71. Entries are shown by descending ordinal.
        // The boundary variant fills the twenty-slot native bag; sibling removal moves the shadow
        // into B's old index. Only fixture seeding bypasses the durable writer.
        let mut entries = vec![
            ("oteryn:item.tibia.i2854", 1),
            ("oteryn:item.tibia.i2854", 1),
            ("oteryn:item.tibia.i3031", 7),
            ("oteryn:item.tibia.i3031", 3),
            ("oteryn:item.tibia.i3577", 7),
        ];
        if matches!(mutation, Mutation::FullViewShrink) {
            entries.resize(
                protocol::container_tree::MAX_CONTAINER_VIEW_ENTRIES,
                ("oteryn:item.tibia.i2854", 1),
            );
        }
        item_look::seed_main_backpack(&harness, &entries).await?;
        {
            let mut tx = harness.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            for (seed, parent) in [(100_u8, 72_u8), (101, 71)] {
                sqlx::raw_sql(sqlx::AssertSqlSafe(
                    crate::durability::item_fee_burn::test_support::item(
                        seed,
                        "oteryn:item.tibia.i2854",
                        "definition-r1",
                        1,
                    ),
                ))
                .execute(&mut *tx)
                .await?;
                sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,encode($1,'hex')::uuid)")
                    .bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(id(parent).as_slice()).execute(&mut *tx).await?;
            }
            tx.commit().await?;
        }
        let before = conservation(&harness.pool).await?;
        let mut admitted = deps.admitted;
        admitted.item_fence = Some(fence);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[4, 6, 12, 14, 146],
        )
        .ok_or("capabilities")?;
        let (mut server, mut client) = tokio::io::duplex(1 << 20);
        let serve = crate::gameplay_transport::connection::serve_admitted(
            &mut server,
            admitted,
            &owner,
            crate::gameplay_transport::connection::IDLE_LIVENESS,
        );
        let exercise = async {
            let mut inventory = None;
            loop {
                let frame = item_look::read_frame(&mut client).await?;
                let envelope = protocol::decode_wire_envelope(&frame).map_err(debug)?;
                if envelope.message_type() == protocol::MessageType::SnapshotChunk {
                    let (_, domains) =
                        protocol::decode_snapshot_chunk(envelope.payload()).map_err(debug)?;
                    for domain in domains {
                        if domain.domain_id == 9 {
                            inventory = Some(
                                protocol::item_view::decode_character_inventory(domain.payload)
                                    .map_err(debug)?,
                            );
                        }
                    }
                }
                if envelope.message_type() == protocol::MessageType::SnapshotCommit {
                    break;
                }
            }
            let inventory = inventory.ok_or("inventory snapshot")?;
            let main = inventory.main_backpack.ok_or("main backpack")?.handle;
            open(&mut client, 1, main).await?;
            let initial = views(&mut client).await?;
            let parent = &initial.views[0];
            assert_eq!(parent.entries.len(), entries.len());
            assert_eq!(
                usize::from(parent.capacity),
                protocol::container_tree::MAX_CONTAINER_VIEW_ENTRIES
            );
            let old_index = entries.len() - 2;
            let bag = parent.entries[old_index].handle;
            let shadow = parent.entries[entries.len() - 1].handle;
            let sibling = parent.entries[entries.len() - 3].handle;
            let source = parent.entries[entries.len() - 5].handle;
            open(&mut client, 2, bag).await?;
            let opened = views(&mut client).await?;
            let child = opened.views[1].entries[0].handle;

            if matches!(mutation, Mutation::Sort) {
                let request = protocol::container_sort::Request {
                    view: 0,
                    key: protocol::container_sort::SortKey::StackSize,
                    descending: false,
                    recursive: false,
                };
                assert_eq!(
                    send(&mut client, 3, 168, &request.encode()).await?,
                    (protocol::CommandStatus::Accepted, vec![1])
                );
            } else {
                let destination = if matches!(mutation, Mutation::Merge) {
                    ItemMoveDestination::MainBackpack
                } else {
                    ItemMoveDestination::Container(shadow)
                };
                move_item(
                    &mut client,
                    3,
                    sibling,
                    destination,
                    protocol::item_view::ItemMoveOutcome::Moved,
                )
                .await?;
            }
            let changed = views(&mut client).await?;
            let parent = &changed.views[0];
            assert_ne!(
                parent.entries[old_index].handle, bag,
                "parent position really changed"
            );
            assert!(parent.entries.iter().any(|entry| entry.handle == bag));
            assert_eq!(changed.views[1].container_handle, bag);
            if !matches!(mutation, Mutation::Sort) {
                assert_eq!(parent.entries.len(), entries.len() - 1);
                assert_eq!(
                    parent.entries[old_index].handle, shadow,
                    "old slot now names another bag"
                );
            }
            if matches!(mutation, Mutation::Merge) {
                // Fully merged source is retired: its old handle cannot move the survivor.
                let merged = conservation(&harness.pool).await?;
                move_item(
                    &mut client,
                    4,
                    sibling,
                    ItemMoveDestination::Container(bag),
                    protocol::item_view::ItemMoveOutcome::Stale,
                )
                .await?;
                assert_eq!(conservation(&harness.pool).await?, merged);
            }
            if matches!(mutation, Mutation::Reopen) {
                let close = protocol::container_tree::encode_container_view_intent(
                    &ContainerViewIntent::Close { view_id: 1 },
                )
                .map_err(debug)?;
                assert_eq!(
                    send(&mut client, 4, 21, &close).await?.0,
                    protocol::CommandStatus::Accepted
                );
                views(&mut client).await?;
                open(&mut client, 5, shadow).await?;
                let replacement = views(&mut client).await?;
                assert_eq!(replacement.views[1].view_id, 1);
                assert_eq!(replacement.views[1].container_handle, shadow);
                let closed = conservation(&harness.pool).await?;
                move_item(
                    &mut client,
                    6,
                    child,
                    ItemMoveDestination::MainBackpack,
                    protocol::item_view::ItemMoveOutcome::Stale,
                )
                .await?;
                assert_eq!(conservation(&harness.pool).await?, closed);
                open(&mut client, 7, bag).await?;
                let reopened = views(&mut client).await?;
                let fresh = reopened
                    .views
                    .iter()
                    .find(|view| view.container_handle == bag)
                    .ok_or("reopened B")?;
                assert_ne!(
                    fresh.entries[0].handle, child,
                    "closed entry handle is never reused"
                );
            }
            // The request was prepared from the old parent projection. It carries B's
            // identity handle, never the now-stale (view 0, old_index) presentation locator.
            let command = match mutation {
                Mutation::Merge => 5,
                Mutation::Reopen => 8,
                Mutation::Remove | Mutation::Sort | Mutation::FullViewShrink => 4,
            };
            move_item(
                &mut client,
                command,
                source,
                ItemMoveDestination::Container(bag),
                protocol::item_view::ItemMoveOutcome::Moved,
            )
            .await?;
            let final_views = views(&mut client).await?;
            let bag_view = final_views
                .views
                .iter()
                .find(|view| view.container_handle == bag)
                .ok_or("B view")?;
            assert!(bag_view.entries.iter().any(|entry| entry.handle == source));
            let parent: String = sqlx::query_scalar("SELECT parent_item_instance_id::text FROM game_item_nested_entries WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(75).as_slice()).fetch_one(&harness.pool).await?;
            assert_eq!(parent.replace('-', ""), hex(id(72)));
            let after = conservation(&harness.pool).await?;
            assert_eq!(before.0, after.0, "live quantities by definition conserved");
            let mut expected_ids = before.1;
            if matches!(mutation, Mutation::Merge) {
                expected_ids.retain(|item| item != &hex(id(73)));
            }
            assert_eq!(
                expected_ids, after.1,
                "exact live identity set, only merge source retires"
            );
            client.shutdown().await?;
            Ok::<_, Box<dyn std::error::Error>>(())
        };
        tokio::time::timeout(std::time::Duration::from_secs(45), drive(serve, exercise)).await??;
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

fn hex(id: [u8; 16]) -> String {
    id.iter().map(|byte| format!("{byte:02x}")).collect()
}

// This isolated fixture has at most twenty-three seeded instances. Assert every live instance
// has exactly one custody row, as well as totals and identities (a stranded item fails).
async fn conservation(pool: &sqlx::PgPool) -> TestResult<(Vec<(String, i64)>, Vec<String>)> {
    let totals = sqlx::query_as("SELECT definition_production_key,sum(quantity)::bigint FROM game_item_instances WHERE lifecycle=1 GROUP BY definition_production_key ORDER BY definition_production_key").fetch_all(pool).await?;
    let rows: Vec<(String, i64)> = sqlx::query_as("SELECT replace(i.item_instance_id::text,'-',''),count(c.item_instance_id) FROM game_item_instances i LEFT JOIN (SELECT item_instance_id FROM game_item_container_slots UNION ALL SELECT item_instance_id FROM game_item_container_entries UNION ALL SELECT item_instance_id FROM game_item_nested_entries) c USING(item_instance_id) WHERE i.lifecycle=1 GROUP BY i.item_instance_id ORDER BY i.item_instance_id").fetch_all(pool).await?;
    for (item, count) in &rows {
        assert_eq!(*count, 1, "live item {item} has exactly one custody");
    }
    Ok((totals, rows.into_iter().map(|(item, _)| item).collect()))
}

async fn open(
    client: &mut tokio::io::DuplexStream,
    command: u64,
    handle: ItemHandle,
) -> TestResult {
    let payload =
        protocol::container_tree::encode_container_view_intent(&ContainerViewIntent::Open {
            handle,
            replace_view: None,
        })
        .map_err(debug)?;
    assert_eq!(
        send(client, command, 21, &payload).await?,
        (
            protocol::CommandStatus::Accepted,
            protocol::container_tree::encode_container_view_result(
                protocol::container_tree::ContainerViewOutcome::Opened
            )
        )
    );
    Ok(())
}
async fn move_item(
    client: &mut tokio::io::DuplexStream,
    command: u64,
    source: ItemHandle,
    destination: ItemMoveDestination,
    expected: protocol::item_view::ItemMoveOutcome,
) -> TestResult {
    let selection = ItemMoveSelection {
        equip_drop: true,
        container_tree: true,
        world_interaction: false,
    };
    let payload = protocol::item_view::encode_item_move_intent_for(
        &ItemMoveIntent {
            source,
            destination,
        },
        selection,
    )
    .map_err(debug)?;
    let (status, payload) = send(client, command, 9, &payload).await?;
    assert_eq!(status, protocol::CommandStatus::Accepted);
    assert_eq!(
        protocol::item_view::decode_item_move_result_for(&payload, selection).map_err(debug)?,
        expected
    );
    Ok(())
}
async fn views(
    client: &mut tokio::io::DuplexStream,
) -> TestResult<protocol::container_tree::ContainerViews> {
    loop {
        let frame = item_look::read_frame(client).await?;
        let envelope = protocol::decode_wire_envelope(&frame).map_err(debug)?;
        if envelope.message_type() == protocol::MessageType::StateDelta {
            let delta = protocol::decode_state_delta(envelope.payload()).map_err(debug)?;
            if delta.domain_id == 14 {
                return protocol::container_tree::decode_container_views(delta.payload)
                    .map_err(debug);
            }
        }
    }
}
async fn send(
    client: &mut tokio::io::DuplexStream,
    id: u64,
    kind: u32,
    payload: &[u8],
) -> TestResult<(protocol::CommandStatus, Vec<u8>)> {
    let frame = protocol::encode_client_command(
        1,
        &protocol::ClientCommandValue {
            command_id: id,
            command_type: kind,
            payload,
        },
    )
    .map_err(debug)?;
    client.write_u32(u32::try_from(frame.len())?).await?;
    client.write_all(&frame).await?;
    loop {
        let frame = item_look::read_frame(client).await?;
        let e = protocol::decode_wire_envelope(&frame).map_err(debug)?;
        if e.message_type() == protocol::MessageType::CommandResult {
            let r = protocol::decode_command_result(e.payload()).map_err(debug)?;
            assert_eq!(r.command_id, id);
            return Ok((r.status, r.payload.to_vec()));
        }
    }
}
async fn drive<A, B>(
    serve: A,
    exercise: B,
) -> TestResult<crate::gameplay_transport::connection::ConnectionEnd>
where
    A: std::future::Future<Output = crate::gameplay_transport::connection::ConnectionEnd>,
    B: std::future::Future<Output = TestResult>,
{
    use std::task::Poll;
    let mut serve = std::pin::pin!(serve);
    let mut exercise = std::pin::pin!(exercise);
    let mut result = None;
    std::future::poll_fn(|cx| {
        if result.is_none()
            && let Poll::Ready(value) = exercise.as_mut().poll(cx)
        {
            if let Err(error) = value {
                return Poll::Ready(Err(error));
            }
            result = Some(());
        }
        if let Poll::Ready(end) = serve.as_mut().poll(cx) {
            return Poll::Ready(if result.is_some() {
                Ok(end)
            } else {
                Err("connection ended before F32 exercise".into())
            });
        }
        Poll::Pending
    })
    .await
}
