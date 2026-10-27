//! Container sorting through the actual connection router and PostgreSQL item owner.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, item_look, prepare_current_session};
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use oteryn_protocol_oteryn::{
    self as protocol,
    container_sort::{Request, SortKey},
};
use tokio::io::AsyncWriteExt;

#[test]
fn container_sort_postgres_dispatch_replay_stale_generation_and_conservation() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("container-sort", scenario)
}
fn scenario() -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "container_sort", true, false).await?;
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

        item_look::seed_main_backpack(
            &harness,
            &[
                ("oteryn:item.tibia.i3031", 3),
                ("oteryn:item.tibia.i2854", 1),
                ("oteryn:item.tibia.i3577", 7),
            ],
        )
        .await?;
        {
            let mut tx = harness.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            for (seed, key, quantity) in [
                (74, "oteryn:item.tibia.i3031", 2),
                (75, "oteryn:item.tibia.i3577", 6),
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
        let before = snapshot(&harness.pool).await?;
        let mut admitted = deps.admitted;
        admitted.item_fence = Some(fence);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[4, 6, 12, 14, 146],
        )
        .ok_or("capabilities")?;
        assert!(admitted.continuity.selected_capabilities.contains(146));
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
            let mut bag = None;
            loop {
                let frame = item_look::read_frame(&mut client).await?;
                let e = protocol::decode_wire_envelope(&frame).map_err(debug)?;
                if e.message_type() == protocol::MessageType::SnapshotChunk {
                    let (_, domains) =
                        protocol::decode_snapshot_chunk(e.payload()).map_err(debug)?;
                    for d in domains {
                        if d.domain_id == 9 {
                            bag = protocol::item_view::decode_character_inventory(d.payload)
                                .map_err(debug)?
                                .main_backpack
                                .map(|b| b.handle);
                        }
                    }
                }
                if e.message_type() == protocol::MessageType::SnapshotCommit {
                    break;
                }
            }
            let request = Request {
                view: 0,
                key: SortKey::StackSize,
                descending: false,
                recursive: true,
            };
            // A closed target cannot authorize sorting.
            assert_eq!(
                send(&mut client, 1, 168, &request.encode()).await?.0,
                protocol::CommandStatus::Rejected
            );
            let open = protocol::container_tree::encode_container_view_intent(
                &protocol::container_tree::ContainerViewIntent::Open {
                    handle: bag.ok_or("bag")?,
                    replace_view: None,
                },
            )
            .map_err(debug)?;
            assert_eq!(
                send(&mut client, 2, 21, &open).await?.0,
                protocol::CommandStatus::Accepted
            );
            assert_eq!(
                send(&mut client, 3, 168, &request.encode()).await?,
                (protocol::CommandStatus::Accepted, vec![1])
            );
            loop {
                let frame = item_look::read_frame(&mut client).await?;
                let envelope = protocol::decode_wire_envelope(&frame).map_err(debug)?;
                if envelope.message_type() == protocol::MessageType::StateDelta {
                    let delta = protocol::decode_state_delta(envelope.payload()).map_err(debug)?;
                    if delta.domain_id == 14 {
                        let views = protocol::container_tree::decode_container_views(delta.payload)
                            .map_err(debug)?;
                        assert_eq!(
                            views.views[0]
                                .entries
                                .iter()
                                .map(|e| e.count.get())
                                .collect::<Vec<_>>(),
                            vec![1, 3, 7]
                        );
                        break;
                    }
                }
            }
            let initial_order = order(&harness.pool, "game_item_container_entries", 70).await?;
            assert_eq!(initial_order, vec![id(72), id(71), id(73)]);
            assert_eq!(order_nested(&harness.pool).await?, vec![id(74), id(75)]);
            assert_eq!(
                snapshot(&harness.pool).await?,
                before,
                "identities, quantities, revisions and parents are conserved"
            );
            assert_eq!(
                send(&mut client, 3, 168, &request.encode()).await?,
                (protocol::CommandStatus::DuplicateReplay, vec![1])
            );
            assert_eq!(
                owner.container_sort(admitted, 3, id(70), request).await,
                Some(true),
                "durable same-request replay"
            );
            assert_eq!(
                owner
                    .container_sort(
                        admitted,
                        3,
                        id(70),
                        Request {
                            descending: true,
                            ..request
                        }
                    )
                    .await,
                Some(false),
                "conflicting replay refuses"
            );
            assert_eq!(
                owner.container_sort_replay(admitted, 3, request).await,
                Some(Some(true)),
                "receipt replay does not need a surviving open view"
            );
            let receipts: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_item_container_sort_receipts")
                    .fetch_one(&harness.pool)
                    .await?;
            assert_eq!(receipts, 1);
            // Every key and direction reaches the real router; closed nested bags
            // participate only when recursive is set.
            let mut command = 4;
            for key in [
                SortKey::Name,
                SortKey::Weight,
                SortKey::Expiry,
                SortKey::StackSize,
            ] {
                for descending in [false, true] {
                    let r = Request {
                        key,
                        descending,
                        recursive: false,
                        ..request
                    };
                    assert_eq!(
                        send(&mut client, command, 168, &r.encode()).await?,
                        (protocol::CommandStatus::Accepted, vec![1])
                    );
                    let leaves = if descending && key != SortKey::Expiry {
                        [id(73), id(71)]
                    } else {
                        [id(71), id(73)]
                    };
                    assert_eq!(
                        order(&harness.pool, "game_item_container_entries", 70).await?,
                        vec![id(72), leaves[0], leaves[1]],
                        "{key:?} descending={descending}"
                    );
                    command += 1;
                }
            }
            assert_eq!(snapshot(&harness.pool).await?, before);
            assert_eq!(
                order_nested(&harness.pool).await?,
                vec![id(74), id(75)],
                "nonrecursive sort preserves nested order"
            );
            client.shutdown().await?;
            Ok::<_, Box<dyn std::error::Error>>(())
        };
        let end = tokio::time::timeout(std::time::Duration::from_secs(45), drive(serve, exercise))
            .await?;
        let end = end?;
        let crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenDisconnected(
            mut plain,
        ) = end
        else {
            return Err("sort connection end".into());
        };
        plain.continuity.selected_capabilities = SelectedCapabilities::select(
            crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[4, 6, 12, 14],
        )
        .ok_or("plain capabilities")?;
        let (mut server, mut client) = tokio::io::duplex(1 << 20);
        let serve = async {
            crate::gameplay_transport::connection::serve_admitted(
                &mut server,
                plain,
                &owner,
                crate::gameplay_transport::connection::IDLE_LIVENESS,
            )
            .await
        };
        let exercise = async {
            let r = Request {
                view: 0,
                key: SortKey::Name,
                descending: false,
                recursive: false,
            };
            assert_eq!(
                send(
                    &mut client,
                    plain.continuity.next_command_id,
                    168,
                    &r.encode()
                )
                .await?,
                (protocol::CommandStatus::Rejected, Vec::new())
            );
            client.shutdown().await?;
            Ok::<_, Box<dyn std::error::Error>>(())
        };
        tokio::time::timeout(std::time::Duration::from_secs(45), drive(serve, exercise)).await??;
        // An ordinal UPDATE without a same-transaction sort receipt is refused
        // even though the runtime now has the narrow column privilege.
        let mut tx = harness.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;
        let error=sqlx::query("UPDATE game_item_container_entries SET placement_ordinal=999 WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).execute(&mut *tx).await;
        let error = match error {
            Ok(_) => return Err("unguarded ordinal update accepted".into()),
            Err(error) => error,
        };
        assert_eq!(
            error.as_database_error().and_then(|e| e.code()).as_deref(),
            Some("23514")
        );
        tx.rollback().await?;
        let request = Request {
            view: 0,
            key: SortKey::Name,
            descending: false,
            recursive: true,
        };
        let command = plain.continuity.next_command_id + 1;
        // Supersede only the session generation; otherwise keep the current
        // Character, lease, scope, Content and node facts intact.
        sqlx::query("UPDATE game_durability_reconnect_sessions SET current_generation=current_generation+1 WHERE game_session_id=encode($1,'hex')::uuid").bind(admitted.game_session_id.as_bytes().as_slice()).execute(&harness.pool).await?;
        let refused = owner
            .container_sort(admitted, command, id(70), request)
            .await;
        assert_eq!(refused, Some(false));
        assert_eq!(
            owner
                .container_sort_replay(
                    admitted,
                    3,
                    Request {
                        key: SortKey::StackSize,
                        ..request
                    }
                )
                .await,
            Some(Some(true)),
            "historical replay never reacquires stale mutation authority"
        );
        assert_eq!(snapshot(&harness.pool).await?, before);
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
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
            return Ok((r.status, r.payload.to_vec()));
        }
    }
}
async fn snapshot(pool: &sqlx::PgPool) -> TestResult<Vec<(String, i64, String, String)>> {
    Ok(sqlx::query_as("SELECT i.item_instance_id::text,i.quantity,i.state_revision::text,COALESCE(e.parent_item_instance_id::text,'') FROM game_item_instances i LEFT JOIN (SELECT item_instance_id,parent_item_instance_id FROM game_item_container_entries UNION ALL SELECT item_instance_id,parent_item_instance_id FROM game_item_nested_entries) e USING(item_instance_id) ORDER BY i.item_instance_id").fetch_all(pool).await?)
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
async fn order_nested(pool: &sqlx::PgPool) -> TestResult<Vec<[u8; 16]>> {
    order(pool, "game_item_nested_entries", 72).await
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
                Err("connection ended before sort exercise".into())
            });
        }
        Poll::Pending
    })
    .await
}
