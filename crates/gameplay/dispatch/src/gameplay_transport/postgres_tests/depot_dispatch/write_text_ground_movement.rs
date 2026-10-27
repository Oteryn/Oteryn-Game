//! A real step retires external selections while preserving carried views.
use super::write_text_ground_nested_view::{open, read_frame, read_outcome, read_views};
use super::*;
use oteryn_protocol_oteryn::container_tree::{ContainerViewOutcome, decode_container_views};
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};
use oteryn_protocol_oteryn::{MessageType, decode_snapshot_chunk, decode_wire_envelope};
use tokio::io::AsyncWriteExt;

pub(super) async fn dispatch(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
    definition: std::num::NonZeroU32,
) -> TestResult {
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = crate::gameplay_transport::connection::serve_admitted(
        &mut server,
        admitted,
        owner,
        crate::gameplay_transport::connection::IDLE_LIVENESS,
    );
    let exchange = async {
        let mut root = None;
        let mut carried = None;
        loop {
            let frame = read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&frame)?;
            if envelope.message_type() == MessageType::SnapshotChunk {
                for domain in decode_snapshot_chunk(envelope.payload())?.1 {
                    if domain.domain_id == 9 {
                        carried = oteryn_protocol_oteryn::item_view::decode_character_inventory_with_equip_drop(domain.payload, true)
                            .map_err(debug)?.main_backpack.map(|item| item.handle);
                    }
                    if domain.domain_id == 17 {
                        let map = oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(
                            domain.payload,
                        )
                        .map_err(debug)?;
                        root = map
                            .tiles
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
        let root = root.ok_or("advertised Ground root")?;
        let carried = carried.ok_or("advertised carried backpack")?;
        client.write_all(&open(1, carried)?).await?;
        read_views(&mut client, 1, 1).await?;
        client.write_all(&open(2, root)?).await?;
        let views = read_views(&mut client, 2, 2).await?;
        let child = views
            .views
            .iter()
            .find(|view| view.container_handle == root)
            .and_then(|view| view.entries.first())
            .ok_or("advertised nested child")?
            .handle;
        client.write_all(&open(3, child)?).await?;
        let before = read_views(&mut client, 3, 3).await?;
        let carried_before = before
            .views
            .iter()
            .find(|view| view.container_handle == carried)
            .ok_or("carried view")?
            .clone();
        // x=8 is within one tile. No container delta should precede the
        // second step's result: the first step leaves all three views intact.
        step(&mut client, 4, StepDirection::East).await?;
        read_step(&mut client, 4).await?;
        step(&mut client, 5, StepDirection::East).await?;
        read_step(&mut client, 5).await?;
        loop {
            let frame = read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&frame)?;
            if envelope.message_type() == MessageType::StateDelta {
                let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())?;
                if delta.domain_id == 14 {
                    let after = decode_container_views(delta.payload).map_err(debug)?;
                    assert_eq!(after.views, vec![carried_before]);
                    break;
                }
            }
        }
        // The nested child left every view, so the previously advertised
        // handle must be stale even after the actor returns to reach.
        client.write_all(&open(6, child)?).await?;
        read_outcome(&mut client, 6, ContainerViewOutcome::Stale).await?;
        step(&mut client, 7, StepDirection::West).await?;
        read_step(&mut client, 7).await?;
        client.write_all(&open(8, root)?).await?;
        let reopened = read_views(&mut client, 8, 2).await?;
        let fresh_child = reopened
            .views
            .iter()
            .find(|view| view.container_handle == root)
            .and_then(|view| view.entries.first())
            .ok_or("fresh child handle")?
            .handle;
        assert!(
            fresh_child.get() > child.get(),
            "retired handles are never reused"
        );
        client.write_all(&open(9, child)?).await?;
        read_outcome(&mut client, 9, ContainerViewOutcome::Stale).await?;
        client.write_all(&open(10, fresh_child)?).await?;
        let restored = read_views(&mut client, 10, 3).await?;
        assert_eq!(
            restored
                .views
                .iter()
                .find(|view| view.container_handle == fresh_child)
                .ok_or("restored child")?
                .parent_handle,
            Some(root)
        );
        step(&mut client, 11, StepDirection::West).await?;
        read_step(&mut client, 11).await?;
        client.write_all(&open(12, fresh_child)?).await?;
        read_views(&mut client, 12, 3).await?;
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
    result.ok_or("movement exchange")??;
    assert!(matches!(
        end,
        Some(crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenDisconnected(_))
    ));
    Ok(())
}

async fn step(
    client: &mut tokio::io::DuplexStream,
    command_id: u64,
    direction: StepDirection,
) -> TestResult {
    let payload = oteryn_protocol_oteryn::world_spatial::encode_step_intent(direction);
    let frame = oteryn_protocol_oteryn::encode_client_command(
        1,
        &oteryn_protocol_oteryn::ClientCommandValue {
            command_id,
            command_type: 1,
            payload: &payload,
        },
    )?;
    client
        .write_all(&u32::try_from(frame.len())?.to_be_bytes())
        .await?;
    client.write_all(&frame).await?;
    Ok(())
}

async fn read_step(client: &mut tokio::io::DuplexStream, command_id: u64) -> TestResult {
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
                oteryn_protocol_oteryn::world_spatial::decode_step_result(result.payload)
                    .map_err(debug)?,
                StepDisposition::Moved
            );
            return Ok(());
        }
        if envelope.message_type() == MessageType::StateDelta {
            assert_ne!(
                oteryn_protocol_oteryn::decode_state_delta(envelope.payload())?.domain_id,
                14,
                "unchanged in-reach views emit no delta; a closing delta follows its step result"
            );
        }
    }
}
