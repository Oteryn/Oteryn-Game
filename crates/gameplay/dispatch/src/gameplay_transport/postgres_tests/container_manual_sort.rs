//! CONTAINER-MANUAL-SORT-1 (capability 510): a slotted command-9 container move through the
//! actual connection router and the PostgreSQL item loop owner.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, item_look, prepare_current_session};
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::FirstEntryOutcome;
use oteryn_protocol_oteryn::{
    self as protocol,
    item_view::{
        ItemHandle, ItemMoveDestination, ItemMoveIntent, ItemMoveOutcome, ItemMoveSelection,
    },
};
use tokio::io::AsyncWriteExt;

const SORT: ItemMoveSelection = ItemMoveSelection {
    equip_drop: true,
    container_tree: true,
    world_interaction: false,
    manual_sort: true,
};

#[test]
fn container_manual_sort_postgres_places_moves_at_the_slot_and_keeps_merges() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("container-manual-sort", scenario)
}
fn scenario() -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "container_manual_sort", true, false)
                .await?;
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

        // Main backpack 70: 71 gold x3, 72 bag, 73 i3577 x7 (newest first: 73, 72, 71).
        item_look::seed_main_backpack(
            &harness,
            &[
                ("oteryn:item.tibia.i3031", 3),
                ("oteryn:item.tibia.i2854", 1),
                ("oteryn:item.tibia.i3577", 7),
            ],
        )
        .await?;
        // Bag 72: 74 an empty bag, 75 gold x2 (newest first: 75, 74).
        {
            let mut tx = harness.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            for (seed, key, quantity) in [
                (74, "oteryn:item.tibia.i2854", 1),
                (75, "oteryn:item.tibia.i3031", 2),
            ] {
                sqlx::raw_sql(sqlx::AssertSqlSafe(
                    crate::durability::item_fee_burn::test_support::item(
                        seed,
                        key,
                        "definition-r1",
                        quantity,
                    ),
                ))
                .execute(&mut *tx)
                .await?;
                sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,encode($1,'hex')::uuid)").bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(id(72).as_slice()).bind(i64::from(seed-73)).execute(&mut *tx).await?;
            }
            tx.commit().await?;
        }
        assert_eq!(
            order(&harness.pool, "game_item_container_entries", 70).await?,
            vec![id(73), id(72), id(71)]
        );
        assert_eq!(
            order(&harness.pool, "game_item_nested_entries", 72).await?,
            vec![id(75), id(74)]
        );
        let mut admitted = deps.admitted;
        admitted.item_fence = Some(fence);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[4, 6, 12, 14, 510],
        )
        .ok_or("capabilities")?;
        assert!(admitted.continuity.selected_capabilities.contains(510));
        let (mut server, mut client) = tokio::io::duplex(1 << 20);
        let serve = async {
            crate::gameplay_transport::connection::serve_admitted(
                &mut server,
                admitted,
                &owner,
                crate::gameplay_transport::connection::IDLE_LIVENESS,
            )
            .await
        };
        let exercise = async {
            // The domain-9 snapshot names the main backpack and its entries by handle.
            let (mut backpack, mut entries) = (None, Vec::new());
            loop {
                let frame = item_look::read_frame(&mut client).await?;
                let e = protocol::decode_wire_envelope(&frame).map_err(debug)?;
                if e.message_type() == protocol::MessageType::SnapshotChunk {
                    let (_, domains) =
                        protocol::decode_snapshot_chunk(e.payload()).map_err(debug)?;
                    for d in domains {
                        if d.domain_id == 9 {
                            let inventory =
                                protocol::item_view::decode_character_inventory(d.payload)
                                    .map_err(debug)?;
                            backpack = inventory.main_backpack.map(|b| b.handle);
                            entries = inventory
                                .entries
                                .iter()
                                .map(|e| (e.count.get(), e.handle))
                                .collect();
                        }
                    }
                }
                if e.message_type() == protocol::MessageType::SnapshotCommit {
                    break;
                }
            }
            let backpack = backpack.ok_or("main backpack")?;
            let by_count = |count: u32| -> TestResult<ItemHandle> {
                Ok(entries
                    .iter()
                    .find(|(c, _)| *c == count)
                    .ok_or("entry")?
                    .1)
            };
            let (gold, bag, meat) = (by_count(3)?, by_count(1)?, by_count(7)?);
            let slotted = |source, container, slot| {
                protocol::item_view::encode_item_move_intent_for(
                    &ItemMoveIntent {
                        source,
                        destination: ItemMoveDestination::Container(container, Some(slot)),
                    },
                    SORT,
                )
                .map_err(debug)
            };

            // Within the same container: the moved entry takes slot 2.
            assert_eq!(
                moved(&mut client, 1, &slotted(meat, backpack, 2)?).await?,
                ItemMoveOutcome::Moved
            );
            assert_eq!(
                order(&harness.pool, "game_item_container_entries", 70).await?,
                vec![id(72), id(71), id(73)]
            );
            // A slot past the last entry is last.
            assert_eq!(
                moved(&mut client, 2, &slotted(bag, backpack, 99)?).await?,
                ItemMoveOutcome::Moved
            );
            assert_eq!(
                order(&harness.pool, "game_item_container_entries", 70).await?,
                vec![id(71), id(73), id(72)]
            );
            // Into a nested container at slot 1.
            let third = slotted(meat, bag, 1)?;
            assert_eq!(moved(&mut client, 3, &third).await?, ItemMoveOutcome::Moved);
            assert_eq!(
                order(&harness.pool, "game_item_nested_entries", 72).await?,
                vec![id(75), id(73), id(74)]
            );
            assert_eq!(
                order(&harness.pool, "game_item_container_entries", 70).await?,
                vec![id(71), id(72)]
            );
            // The session retains the latest outcome: replaying it moves nothing again.
            assert_eq!(
                send(&mut client, 3, &third).await?.0,
                protocol::CommandStatus::DuplicateReplay
            );
            assert_eq!(
                order(&harness.pool, "game_item_nested_entries", 72).await?,
                vec![id(75), id(73), id(74)]
            );
            // Gold merges into the bag's gold stack as before; no entry moves.
            assert_eq!(
                moved(&mut client, 4, &slotted(gold, bag, 2)?).await?,
                ItemMoveOutcome::Moved
            );
            assert_eq!(
                order(&harness.pool, "game_item_nested_entries", 72).await?,
                vec![id(75), id(73), id(74)]
            );
            let gold_left: i64 = sqlx::query_scalar(
                "SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
            )
            .bind(id(75).as_slice())
            .fetch_one(&harness.pool)
            .await?;
            assert_eq!(gold_left, 5);
            let receipts: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_item_container_sort_receipts")
                    .fetch_one(&harness.pool)
                    .await?;
            assert_eq!(receipts, 3, "one ordinal receipt per slotted placement");
            client.shutdown().await?;
            Ok::<_, Box<dyn std::error::Error>>(())
        };
        tokio::time::timeout(std::time::Duration::from_secs(45), drive(serve, exercise))
            .await??;
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
async fn moved(
    client: &mut tokio::io::DuplexStream,
    id: u64,
    payload: &[u8],
) -> TestResult<ItemMoveOutcome> {
    let (status, result) = send(client, id, payload).await?;
    assert_eq!(status, protocol::CommandStatus::Accepted);
    protocol::item_view::decode_item_move_result_for(&result, SORT).map_err(debug)
}
async fn send(
    client: &mut tokio::io::DuplexStream,
    id: u64,
    payload: &[u8],
) -> TestResult<(protocol::CommandStatus, Vec<u8>)> {
    let frame = protocol::encode_client_command(
        1,
        &protocol::ClientCommandValue {
            command_id: id,
            command_type: protocol::item_view::COMMAND_TYPE_ITEM_MOVE_INTENT,
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
            return Ok((r.status, r.payload.to_vec()));
        }
    }
}
async fn order(pool: &sqlx::PgPool, table: &str, parent: u8) -> TestResult<Vec<[u8; 16]>> {
    let sql = format!(
        "SELECT item_instance_id::text FROM {table} WHERE parent_item_instance_id=encode($1,'hex')::uuid ORDER BY placement_ordinal DESC"
    );
    let ids: Vec<String> = sqlx::query_scalar(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(id(parent).as_slice())
        .fetch_all(pool)
        .await?;
    ids.iter()
        .map(|s| {
            let h = s.replace('-', "");
            let mut value = [0; 16];
            for (i, byte) in value.iter_mut().enumerate() {
                *byte = u8::from_str_radix(&h[i * 2..i * 2 + 2], 16)?;
            }
            Ok(value)
        })
        .collect()
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
                Err("connection ended before the manual sort exercise".into())
            });
        }
        Poll::Pending
    })
    .await
}
