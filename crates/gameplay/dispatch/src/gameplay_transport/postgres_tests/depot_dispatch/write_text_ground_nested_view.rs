//! External-tree view qualification, not populated-tree movement or TEXT activation.
use super::*;
use crate::gameplay_transport::container_view::ContainerObservation;
use crate::gameplay_transport::item_move::{LoopMove, LoopMoveResult, LoopTarget};
use oteryn_protocol_oteryn::container_tree::{
    COMMAND_TYPE_CONTAINER_VIEW_INTENT, ContainerViewIntent, ContainerViewOutcome, ContainerViews,
    decode_container_view_result, decode_container_views, encode_container_view_intent,
};
use oteryn_protocol_oteryn::item_view::ItemHandle;
use oteryn_protocol_oteryn::{MessageType, decode_snapshot_chunk, decode_wire_envelope};

pub(super) async fn exercise(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: crate::gameplay_transport::connection::AdmittedSession,
    pool: &sqlx::PgPool,
    root: [u8; 16],
    child: [u8; 16],
    definition: std::num::NonZeroU32,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let fence = admitted.item_fence.ok_or("fence")?;
    assert_eq!(
        owner
            .move_loop_item(
                actor,
                UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: 900,
                    item_fence: Some(fence),
                },
                LoopMove {
                    item: ItemKey::Instance(root),
                    count: None,
                    target: LoopTarget::Ground(
                        oteryn_protocol_oteryn::item_view::WorldTilePosition {
                            x: 7,
                            y: 0,
                            floor: -7,
                        }
                    ),
                }
            )
            .await,
        LoopMoveResult::Committed,
    );
    // The production loop still refuses populated drops. Only the external
    // topology is fixture data, seeded explicitly without claiming move parity.
    // The root retains its real production drop proof; neither a view nor TEXT
    // can mutate this fixture's custody through an alternate runtime writer.
    let original: serde_json::Value = sqlx::query_scalar(
        "SELECT to_jsonb(e) FROM game_item_container_entries e WHERE item_instance_id=encode($1,'hex')::uuid",
    ).bind(child.as_slice()).fetch_one(pool).await?;
    fixture(pool, "WITH e AS (DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid RETURNING *) INSERT INTO game_item_nested_entries SELECT item_instance_id,world_id,encode($2,'hex')::uuid,placement_ordinal,placed_transaction_id FROM e", &[child, root]).await?;
    let ground = owner
        .root
        .read_item_loop_ground(owner.world_id, owner.channel_id)
        .await?
        .into_iter()
        .find(|entry| entry.item.item_instance_id == root)
        .ok_or("Ground root")?
        .item
        .ground;
    let ground = crate::durability::item_loop_move::LoopGround {
        spatial_position: ground.spatial_position,
        map_revision: ground.map_revision,
        content_revision: ground.content_revision,
        placement_context: ground.native_room_placement_context,
    };
    assert_eq!(
        owner
            .root
            .read_item_loop_ground_container_root(owner.character, fence, child)
            .await?,
        Some(root)
    );
    assert!(
        matches!(owner.observe_container(actor, Some(fence), ItemKey::Instance(child)).await,
        Some(ContainerObservation::Container { parent: Some(ItemKey::Instance(parent)), entries, .. })
            if parent == root && entries.is_empty())
    );
    assert!(
        owner
            .root
            .read_item_loop_ground_container(owner.character, fence, child, child, ground.clone())
            .await?
            .is_none()
    );
    let mut wrong = ground.clone();
    wrong.placement_context[0] ^= 1;
    assert!(
        owner
            .root
            .read_item_loop_ground_container(owner.character, fence, child, root, wrong)
            .await?
            .is_none()
    );

    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 12, 14, 18])
            .ok_or("capabilities")?;
    dispatch_ground(owner, admitted, definition, Some((pool, root, child))).await?;

    // Each corrupt ancestry class fails closed; restore each fixture before the
    // next assertion. No production constraint or runtime test is disabled.
    for (mutation, restore) in [
        (
            "UPDATE game_item_instances SET world_id=game_character_uuid_v7() WHERE item_instance_id=encode($1,'hex')::uuid",
            "UPDATE game_item_instances SET world_id=(SELECT world_id FROM game_item_instances WHERE item_instance_id=encode($2,'hex')::uuid) WHERE item_instance_id=encode($1,'hex')::uuid",
        ),
        (
            "UPDATE game_item_instances SET lifecycle=2,quantity=0,last_transaction_id=minted_transaction_id WHERE item_instance_id=encode($1,'hex')::uuid",
            "UPDATE game_item_instances SET lifecycle=1,quantity=1,last_transaction_id=NULL WHERE item_instance_id=encode($1,'hex')::uuid",
        ),
        (
            "INSERT INTO game_item_nested_entries SELECT item_instance_id,world_id,encode($2,'hex')::uuid,1,game_character_uuid_v7() FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
            "DELETE FROM game_item_nested_entries WHERE item_instance_id=encode($1,'hex')::uuid",
        ),
    ] {
        // The cycle points root back into child; the other cases affect child.
        let cycle = mutation.starts_with("INSERT");
        let ids = if cycle { [root, child] } else { [child, root] };
        fixture(pool, mutation, &ids[..if cycle { 2 } else { 1 }]).await?;
        assert_eq!(
            owner
                .observe_container(actor, Some(fence), ItemKey::Instance(child))
                .await,
            Some(ContainerObservation::Gone)
        );
        fixture(
            pool,
            restore,
            &ids[..if restore.contains("$2") { 2 } else { 1 }],
        )
        .await?;
    }
    // Insert fixture spacers above the selected child, retaining the real
    // Ground root. Depth eight opens; the next edge must fail closed.
    for depth in 2..=9 {
        let spacer = id(180 + depth);
        fixture(pool, "INSERT INTO game_item_instances SELECT (jsonb_populate_record(NULL::game_item_instances,to_jsonb(i) || jsonb_build_object('item_instance_id',encode($1,'hex')::uuid,'minted_transaction_id',game_character_uuid_v7()))).* FROM game_item_instances i WHERE item_instance_id=encode($2,'hex')::uuid", &[spacer, child]).await?;
        fixture(pool, "INSERT INTO game_item_nested_entries SELECT encode($1,'hex')::uuid,world_id,parent_item_instance_id,1,game_character_uuid_v7() FROM game_item_nested_entries WHERE item_instance_id=encode($2,'hex')::uuid", &[spacer, child]).await?;
        fixture(pool, "UPDATE game_item_nested_entries SET parent_item_instance_id=encode($1,'hex')::uuid WHERE item_instance_id=encode($2,'hex')::uuid", &[spacer, child]).await?;
        if depth == 8 {
            assert!(
                matches!(owner.observe_container(actor, Some(fence), ItemKey::Instance(child)).await,
                Some(ContainerObservation::Container { parent: Some(ItemKey::Instance(parent)), .. }) if parent == spacer)
            );
        } else if depth == 9 {
            assert_eq!(
                owner
                    .root
                    .read_item_loop_ground_container_root(owner.character, fence, child)
                    .await?,
                None
            );
            assert_eq!(
                owner
                    .observe_container(actor, Some(fence), ItemKey::Instance(child))
                    .await,
                Some(ContainerObservation::Gone)
            );
        }
    }
    fixture(pool, "UPDATE game_item_nested_entries SET parent_item_instance_id=encode($1,'hex')::uuid WHERE item_instance_id=encode($2,'hex')::uuid", &[root, child]).await?;
    for depth in 2..=9 {
        let spacer = id(180 + depth);
        fixture(
            pool,
            "DELETE FROM game_item_nested_entries WHERE item_instance_id=encode($1,'hex')::uuid",
            &[spacer],
        )
        .await?;
        fixture(
            pool,
            "DELETE FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
            &[spacer],
        )
        .await?;
    }
    {
        let mut runtime = owner.runtime.lock().await;
        let mut movement = runtime.borrow_movement_position();
        for x in [6, 5] {
            let snapshot = movement.read(actor).map_err(debug)?;
            movement
                .commit_cardinal(snapshot, MovementLocalPosition { x, y: 0, floor: 7 })
                .map_err(debug)?;
        }
    }
    assert_eq!(
        owner
            .observe_container(actor, Some(fence), ItemKey::Instance(child))
            .await,
        Some(ContainerObservation::TooFar)
    );
    {
        let mut runtime = owner.runtime.lock().await;
        let mut movement = runtime.borrow_movement_position();
        for x in [6, 7] {
            let snapshot = movement.read(actor).map_err(debug)?;
            movement
                .commit_cardinal(snapshot, MovementLocalPosition { x, y: 0, floor: 7 })
                .map_err(debug)?;
        }
    }
    fixture(
        pool,
        "DELETE FROM game_item_nested_entries WHERE item_instance_id=encode($1,'hex')::uuid",
        &[child],
    )
    .await?;
    assert_eq!(
        owner
            .observe_container(actor, Some(fence), ItemKey::Instance(child))
            .await,
        Some(ContainerObservation::Gone)
    );
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_container_entries SELECT * FROM jsonb_populate_record(NULL::game_item_container_entries,$1)")
        .bind(sqlx::types::Json(original)).execute(&mut *tx).await?;
    tx.commit().await?;
    assert!(
        owner
            .root
            .read_item_loop_ground_container(owner.character, fence, child, root, ground)
            .await?
            .is_none()
    );
    assert!(matches!(
        owner
            .observe_container(actor, Some(fence), ItemKey::Instance(child))
            .await,
        Some(ContainerObservation::Container {
            parent: Some(_),
            ..
        })
    ));
    Ok(())
}

async fn fixture(pool: &sqlx::PgPool, sql: &'static str, ids: &[[u8; 16]]) -> TestResult {
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let mut query = sqlx::query(sql);
    for id in ids {
        query = query.bind(id.as_slice());
    }
    query.execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

fn open(command_id: u64, handle: ItemHandle) -> TestResult<Vec<u8>> {
    let payload = encode_container_view_intent(&ContainerViewIntent::Open {
        handle,
        replace_view: None,
    })
    .map_err(debug)?;
    let frame = oteryn_protocol_oteryn::encode_client_command(
        1,
        &oteryn_protocol_oteryn::ClientCommandValue {
            command_id,
            command_type: COMMAND_TYPE_CONTAINER_VIEW_INTENT,
            payload: &payload,
        },
    )?;
    let mut input = u32::try_from(frame.len())?.to_be_bytes().to_vec();
    input.extend(frame);
    Ok(input)
}

pub(super) async fn dispatch_ground(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
    definition: std::num::NonZeroU32,
    nested: Option<(&sqlx::PgPool, [u8; 16], [u8; 16])>,
) -> TestResult {
    use tokio::io::AsyncWriteExt;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = crate::gameplay_transport::connection::serve_admitted(
        &mut server,
        admitted,
        owner,
        crate::gameplay_transport::connection::IDLE_LIVENESS,
    );
    let exchange = async move {
        let mut root_handle = None;
        loop {
            let frame = read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&frame)?;
            if envelope.message_type() == MessageType::SnapshotChunk {
                for domain in decode_snapshot_chunk(envelope.payload())?.1 {
                    if domain.domain_id == 17 {
                        let map = oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(
                            domain.payload,
                        )
                        .map_err(debug)?;
                        root_handle =
                            map.tiles
                                .iter()
                                .flat_map(|tile| &tile.items)
                                .find_map(|entry| {
                                    if entry.definition
                                        == oteryn_protocol_oteryn::world_map::MapDefinition::Item(
                                            definition,
                                        )
                                        && let oteryn_protocol_oteryn::world_map::MapOrigin::Handle(
                                            handle,
                                        ) = entry.origin
                                    {
                                        Some(handle)
                                    } else {
                                        None
                                    }
                                });
                    }
                }
            }
            if envelope.message_type() == MessageType::SnapshotCommit {
                break;
            }
        }
        let root_handle = root_handle.ok_or("advertised Ground root handle")?;
        client.write_all(&open(1, root_handle)?).await?;
        let views = read_views(&mut client, 1, 1).await?;
        assert_eq!(views.views[0].container_handle, root_handle);
        assert_eq!(views.views[0].parent_handle, None);
        let selected_handle = if nested.is_some() {
            assert_eq!(views.views[0].entries.len(), 1);
            let child_handle = views.views[0].entries[0].handle;
            // The second command uses the handle just advertised by the first
            // command, in the same actual production connection.
            client.write_all(&open(2, child_handle)?).await?;
            let views = read_views(&mut client, 2, 2).await?;
            let child = views
                .views
                .iter()
                .find(|view| view.container_handle == child_handle)
                .ok_or("nested view")?;
            assert_eq!(child.parent_handle, Some(root_handle));
            assert!(child.entries.is_empty());
            child_handle
        } else {
            assert!(views.views[0].entries.is_empty());
            root_handle
        };
        // The connection retains its actual advertised handle and open views.
        // A historical successful open must not bypass current root reach.
        let next_command = if nested.is_some() { 3 } else { 2 };
        move_actor(owner, actor, &[6, 5]).await?;
        client
            .write_all(&open(next_command, selected_handle)?)
            .await?;
        read_outcome(&mut client, next_command, ContainerViewOutcome::TooFar).await?;
        move_actor(owner, actor, &[6, 7]).await?;
        client
            .write_all(&open(next_command + 1, selected_handle)?)
            .await?;
        let expected_views = if nested.is_some() { 2 } else { 1 };
        let reopened = read_views(&mut client, next_command + 1, expected_views).await?;
        assert!(
            reopened
                .views
                .iter()
                .any(|view| view.container_handle == selected_handle)
        );
        if let Some((pool, root, child)) = nested {
            // Explicitly corrupt only fixture topology after this exact live
            // connection has observed it. Reopening its retained handle must
            // revalidate ancestry, rather than trust the earlier view.
            fixture(pool, "INSERT INTO game_item_nested_entries SELECT item_instance_id,world_id,encode($2,'hex')::uuid,1,game_character_uuid_v7() FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid", &[root, child]).await?;
            client.write_all(&open(5, selected_handle)?).await?;
            read_outcome(&mut client, 5, ContainerViewOutcome::Stale).await?;
            fixture(pool, "DELETE FROM game_item_nested_entries WHERE item_instance_id=encode($1,'hex')::uuid", &[root]).await?;
            client.write_all(&open(6, selected_handle)?).await?;
            let restored = read_views(&mut client, 6, 2).await?;
            let child = restored
                .views
                .iter()
                .find(|view| view.container_handle == selected_handle)
                .ok_or("restored nested view")?;
            assert_eq!(child.parent_handle, Some(root_handle));
            assert!(child.entries.is_empty());
            for (owner_lock, command_id) in [(false, 7), (true, 9)] {
                revalidate_after_lock(
                    owner,
                    &mut client,
                    pool,
                    root,
                    selected_handle,
                    command_id,
                    owner_lock,
                )
                .await?;
                client
                    .write_all(&open(command_id + 1, selected_handle)?)
                    .await?;
                let restored = read_views(&mut client, command_id + 1, 2).await?;
                let child = restored
                    .views
                    .iter()
                    .find(|view| view.container_handle == selected_handle)
                    .ok_or("nested view after Ground proof restoration")?;
                assert_eq!(child.parent_handle, Some(root_handle));
                assert!(child.entries.is_empty());
            }
        }
        client.shutdown().await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    };
    let mut serving = std::pin::pin!(serving);
    let mut exchange = std::pin::pin!(exchange);
    let mut end = None;
    let mut result = None;
    tokio::time::timeout(
        std::time::Duration::from_secs(30),
        std::future::poll_fn(|cx| {
            use std::future::Future;
            use std::task::Poll;
            if end.is_none()
                && let Poll::Ready(value) = serving.as_mut().poll(cx)
            {
                end = Some(value);
            }
            if result.is_none()
                && let Poll::Ready(value) = exchange.as_mut().poll(cx)
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
    result.ok_or("Ground client did not finish")?
}

/// Observe a real command-21 read waiting on a physical lock, then change only
/// the root placement in privileged fixture data. An earlier open and a path
/// read before the wait must not authorize the now-stale Ground placement.
async fn revalidate_after_lock(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    client: &mut tokio::io::DuplexStream,
    pool: &sqlx::PgPool,
    root: [u8; 16],
    handle: ItemHandle,
    command_id: u64,
    owner_lock: bool,
) -> TestResult {
    use tokio::io::AsyncWriteExt;
    let mut blocker = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *blocker)
        .await?;
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *blocker)
        .await?;
    if owner_lock {
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended(encode($1,'hex') || encode($2,'hex'),33))")
            .bind(owner.world_id.as_bytes().as_slice())
            .bind(owner.channel_id.as_bytes().as_slice())
            .execute(&mut *blocker).await?;
    } else {
        sqlx::query("SELECT 1 FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid FOR UPDATE")
            .bind(root.as_slice()).execute(&mut *blocker).await?;
    }
    let original: Vec<u8> = sqlx::query_scalar("SELECT native_room_placement_context FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(root.as_slice()).fetch_one(&mut *blocker).await?;
    client.write_all(&open(command_id, handle)?).await?;
    // The outer exchange driver continues polling serve_admitted while this
    // future waits. Require an actual database waiter; elapsed time is no proof.
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let blocked: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid)))")
                .bind(pid).fetch_one(pool).await?;
            if blocked { return Ok::<(), sqlx::Error>(()); }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    }).await??;
    let mut changed = original.clone();
    *changed
        .first_mut()
        .ok_or("empty Ground placement context")? ^= 1;
    sqlx::query("UPDATE game_item_ground_locations SET native_room_placement_context=$2 WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(root.as_slice()).bind(changed).execute(&mut *blocker).await?;
    blocker.commit().await?;
    read_outcome(client, command_id, ContainerViewOutcome::Stale).await?;

    let mut restoring = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *restoring)
        .await?;
    sqlx::query("UPDATE game_item_ground_locations SET native_room_placement_context=$2 WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(root.as_slice()).bind(original).execute(&mut *restoring).await?;
    restoring.commit().await?;
    Ok(())
}

async fn read_frame(client: &mut tokio::io::DuplexStream) -> TestResult<Vec<u8>> {
    use tokio::io::AsyncReadExt;
    let length = usize::try_from(client.read_u32().await?)?;
    assert!(length <= 1 << 20);
    let mut frame = vec![0; length];
    client.read_exact(&mut frame).await?;
    Ok(frame)
}

async fn read_views(
    client: &mut tokio::io::DuplexStream,
    command_id: u64,
    expected: usize,
) -> TestResult<ContainerViews> {
    let mut results = 0;
    let mut latest = None;
    loop {
        let frame = read_frame(client).await?;
        let envelope = decode_wire_envelope(&frame)?;
        match envelope.message_type() {
            MessageType::CommandResult => {
                let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
                assert_eq!(result.command_id, command_id);
                assert_eq!(
                    result.status,
                    oteryn_protocol_oteryn::CommandStatus::Accepted
                );
                assert_eq!(
                    decode_container_view_result(result.payload).map_err(debug)?,
                    ContainerViewOutcome::Opened
                );
                results += 1;
            }
            MessageType::StateDelta => {
                let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())?;
                if delta.domain_id == 14 {
                    latest = Some(decode_container_views(delta.payload).map_err(debug)?);
                }
            }
            _ => {}
        }
        if results == 1
            && let Some(views) = latest.as_ref()
            && views.views.len() == expected
        {
            return latest.ok_or_else(|| "container view delta".into());
        }
    }
}

async fn move_actor(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    steps: &[i32],
) -> TestResult {
    let mut runtime = owner.runtime.lock().await;
    let mut movement = runtime.borrow_movement_position();
    for &x in steps {
        let snapshot = movement.read(actor).map_err(debug)?;
        movement
            .commit_cardinal(snapshot, MovementLocalPosition { x, y: 0, floor: 7 })
            .map_err(debug)?;
    }
    Ok(())
}

async fn read_outcome(
    client: &mut tokio::io::DuplexStream,
    command_id: u64,
    expected: ContainerViewOutcome,
) -> TestResult {
    loop {
        let frame = read_frame(client).await?;
        let envelope = decode_wire_envelope(&frame)?;
        if envelope.message_type() == MessageType::CommandResult {
            let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
            assert_eq!(result.command_id, command_id);
            assert_eq!(
                result.status,
                oteryn_protocol_oteryn::CommandStatus::Accepted
            );
            assert_eq!(
                decode_container_view_result(result.payload).map_err(debug)?,
                expected
            );
            return Ok(());
        }
    }
}
