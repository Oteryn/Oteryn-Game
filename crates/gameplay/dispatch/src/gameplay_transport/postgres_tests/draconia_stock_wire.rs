//! Native base-ordinal container USE and child pickup through literal framed commands.
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult,
    fixture::{Harness, debug},
};
use crate::gameplay_transport::{
    capabilities::SelectedCapabilities,
    connection::{ConnectionEnd, FreshAdmissionAuthority, IDLE_LIVENESS, serve_admitted},
};
use oteryn_protocol_oteryn::{
    self as protocol, container_tree, item_view, world_map, world_object,
};
use tokio::io::AsyncWriteExt;

#[test]
fn draconia_stock_wire_native_container_opens() -> TestResult {
    super::draconia_container_stock::scenario("wire-open")
}
#[test]
fn draconia_stock_wire_native_key_withdraws() -> TestResult {
    super::draconia_container_stock::scenario("wire-pickup")
}
#[test]
fn draconia_stock_wire_replay_and_stale_child_never_duplicate() -> TestResult {
    super::draconia_container_stock::scenario("wire-replay")
}
async fn send(
    client: &mut tokio::io::DuplexStream,
    generation: u64,
    id: u64,
    kind: u32,
    payload: &[u8],
) -> TestResult {
    let frame = protocol::encode_client_command(
        generation,
        &protocol::ClientCommandValue {
            command_id: id,
            command_type: kind,
            payload,
        },
    )
    .map_err(debug)?;
    client.write_u32(u32::try_from(frame.len())?).await?;
    client.write_all(&frame).await?;
    Ok(())
}
async fn move_result(
    client: &mut tokio::io::DuplexStream,
    id: u64,
    expected_status: protocol::CommandStatus,
) -> TestResult<item_view::ItemMoveOutcome> {
    loop {
        let raw = super::item_look::read_frame(client).await?;
        let frame = protocol::decode_wire_envelope(&raw).map_err(debug)?;
        if frame.message_type() == protocol::MessageType::CommandResult {
            let result = protocol::decode_command_result(frame.payload()).map_err(debug)?;
            if result.command_id == id {
                assert_eq!(result.status, expected_status);
                return item_view::decode_item_move_result(result.payload).map_err(debug);
            }
        }
    }
}
async fn empty_pane(client: &mut tokio::io::DuplexStream) -> TestResult {
    loop {
        let raw = super::item_look::read_frame(client).await?;
        let frame = protocol::decode_wire_envelope(&raw).map_err(debug)?;
        if frame.message_type() == protocol::MessageType::StateDelta {
            let delta = protocol::decode_state_delta(frame.payload()).map_err(debug)?;
            if delta.domain_id == container_tree::STATE_DOMAIN_CONTAINER_VIEWS {
                let views = container_tree::decode_container_views(delta.payload).map_err(debug)?;
                assert_eq!(
                    views.views.len(),
                    1,
                    "parent pane stays open after withdrawal"
                );
                assert!(
                    views.views[0].entries.is_empty(),
                    "consumed stock leaves the shared pane"
                );
                return Ok(());
            }
        }
    }
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 12, 14, 15, 18])
            .ok_or("stock wire capabilities")?;
    let session = *admitted;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
        let end = serve_admitted(&mut server, session, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let exercise = async {
        let mut target = None;
        loop {
            let raw = super::item_look::read_frame(&mut client).await?;
            let frame = protocol::decode_wire_envelope(&raw).map_err(debug)?;
            if frame.message_type() == protocol::MessageType::SnapshotChunk {
                for domain in protocol::decode_snapshot_chunk(frame.payload())
                    .map_err(debug)?
                    .1
                {
                    if domain.domain_id == world_map::STATE_DOMAIN_WORLD_MAP_VIEW {
                        let map =
                            world_map::decode_world_map_snapshot(domain.payload).map_err(debug)?;
                        for tile in &map.tiles {
                            if (tile.position.x, tile.position.y, tile.position.floor)
                                != (32794, 31572, -7)
                            {
                                continue;
                            }
                            for item in &tile.items {
                                if item.appearance_id != 5972 {
                                    continue;
                                }
                                let world_map::MapOrigin::BaseOrdinal {
                                    ordinal,
                                    object_revision,
                                } = item.origin
                                else {
                                    return Err(
                                        "native immovable skeleton must be base ordinal".into()
                                    );
                                };
                                assert_eq!(ordinal, 1);
                                let mut placement = map.header.bundle_digest.to_vec();
                                placement.extend_from_slice(
                                    &crate::map::view::placement_key(
                                        crate::map::overlay::TilePos {
                                            x: 32794,
                                            y: 31572,
                                            floor: -7,
                                        },
                                        ordinal,
                                    )
                                    .ok_or("source placement")?
                                    .to_be_bytes(),
                                );
                                target = Some(world_object::WorldObjectTarget {
                                    placement,
                                    expected_revision: object_revision,
                                });
                            }
                        }
                    }
                }
            }
            if frame.message_type() == protocol::MessageType::SnapshotCommit {
                break;
            }
        }
        let target = target.ok_or("native skeleton in map snapshot")?;
        let generation = session.continuity.connection_generation;
        send(
            &mut client,
            generation,
            1,
            2,
            &world_object::encode_use_intent(&target).map_err(debug)?,
        )
        .await?;
        let mut opened = false;
        let mut child = None;
        while !opened || child.is_none() {
            let raw = super::item_look::read_frame(&mut client).await?;
            let frame = protocol::decode_wire_envelope(&raw).map_err(debug)?;
            if frame.message_type() == protocol::MessageType::CommandResult {
                let result = protocol::decode_command_result(frame.payload()).map_err(debug)?;
                if result.command_id == 1 {
                    assert_eq!(
                        world_object::decode_use_result(result.payload).map_err(debug)?,
                        world_object::UseDisposition::Committed,
                        "native base-ordinal USE must open source container"
                    );
                    opened = true;
                }
            } else if frame.message_type() == protocol::MessageType::StateDelta {
                let delta = protocol::decode_state_delta(frame.payload()).map_err(debug)?;
                if delta.domain_id == container_tree::STATE_DOMAIN_CONTAINER_VIEWS {
                    let views =
                        container_tree::decode_container_views(delta.payload).map_err(debug)?;
                    let view = views.views.first().ok_or("source container pane")?;
                    assert_eq!(view.capacity, 10);
                    assert_eq!(view.entries.len(), 1);
                    let entry = &view.entries[0];
                    let native = owner
                        .active_generation
                        .ok_or("generation")?
                        .native_gameplay()
                        .ok_or("native")?;
                    assert_eq!(
                        native
                            .item_index()
                            .ok_or("index")?
                            .definition_at(entry.item_definition_ref)
                            .ok_or("definition")?
                            .0,
                        "oteryn:item.tibia.i2969"
                    );
                    child = Some(entry.handle);
                }
            }
        }
        if !name.ends_with("open") {
            let payload = item_view::encode_item_move_intent(&item_view::ItemMoveIntent {
                source: child.ok_or("child handle")?,
                destination: item_view::ItemMoveDestination::MainBackpack,
            })
            .map_err(debug)?;
            send(&mut client, generation, 2, 9, &payload).await?;
            assert_eq!(
                move_result(&mut client, 2, protocol::CommandStatus::Accepted).await?,
                item_view::ItemMoveOutcome::Moved
            );
            empty_pane(&mut client).await?;
            if name.ends_with("replay") {
                send(&mut client, generation, 2, 9, &payload).await?;
                assert_eq!(
                    move_result(&mut client, 2, protocol::CommandStatus::DuplicateReplay).await?,
                    item_view::ItemMoveOutcome::Moved
                );
                send(&mut client, generation, 3, 9, &payload).await?;
                assert_eq!(
                    move_result(&mut client, 3, protocol::CommandStatus::Accepted).await?,
                    item_view::ItemMoveOutcome::Stale
                );
            }
        }
        client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    use std::future::Future;
    use std::task::Poll;
    let mut serving = std::pin::pin!(serving);
    let mut exercise = std::pin::pin!(exercise);
    let mut maintenance = std::pin::pin!(super::item_craft::maintain_ready(owner));
    let mut end = None;
    let mut completed = false;
    tokio::time::timeout(
        std::time::Duration::from_secs(40),
        std::future::poll_fn(|context| {
            if let Poll::Ready(result) = maintenance.as_mut().poll(context) {
                return Poll::Ready(result);
            }
            if end.is_none()
                && let Poll::Ready(value) = serving.as_mut().poll(context)
            {
                end = Some(value);
            }
            if !completed && let Poll::Ready(result) = exercise.as_mut().poll(context) {
                if let Err(error) = result {
                    return Poll::Ready(Err(error));
                }
                completed = true;
            }
            if completed && end.is_some() {
                Poll::Ready(Ok(()))
            } else {
                Poll::Pending
            }
        }),
    )
    .await??;
    let ConnectionEnd::AdmittedThenDisconnected(next) =
        end.ok_or("stock wire server did not finish")?
    else {
        return Err("stock wire connection ended unexpectedly".into());
    };
    *admitted = next;
    let keys: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_keys WHERE key_number=3001")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(keys, if name.ends_with("open") { 0 } else { 1 });
    eprintln!("POSTGRES_STOCK_WIRE literal_frames=true keys={keys}");
    Ok(())
}
