//! Open a real nested container, then USE the displayed reward through the wire route.
use super::*;
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, IDLE_LIVENESS, serve_admitted,
};
use oteryn_protocol_oteryn::{self as protocol, container_tree, item_view};
use tokio::io::AsyncWriteExt;

pub(super) async fn exchange_nested(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
    intent: &UseIntent,
    container_definition: std::num::NonZeroU32,
    source_definition: std::num::NonZeroU32,
) -> TestResult<(AdmittedSession, UseDisposition, Vec<u8>)> {
    if owner.root.has_ready_demand() {
        owner.root.maintain_ready_once().await.map_err(debug)?;
    }
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let exercise = async {
        let mut container = None;
        loop {
            let raw = super::super::item_look::read_frame(&mut client).await?;
            let envelope = protocol::decode_wire_envelope(&raw).map_err(debug)?;
            if envelope.message_type() == protocol::MessageType::SnapshotChunk {
                for domain in protocol::decode_snapshot_chunk(envelope.payload())
                    .map_err(debug)?
                    .1
                {
                    if domain.domain_id == item_view::STATE_DOMAIN_CHARACTER_INVENTORY {
                        let inventory = item_view::decode_character_inventory_with_equip_drop(
                            domain.payload,
                            true,
                        )
                        .map_err(debug)?;
                        container = inventory
                            .entries
                            .iter()
                            .find(|entry| entry.item_definition_ref == container_definition)
                            .map(|entry| entry.handle);
                    }
                }
            }
            if envelope.message_type() == protocol::MessageType::SnapshotCommit {
                break;
            }
        }
        let container = container.ok_or("shown nested source bag")?;
        let open = container_tree::encode_container_view_intent(
            &container_tree::ContainerViewIntent::Open {
                handle: container,
                replace_view: None,
            },
        )
        .map_err(debug)?;
        send(
            &mut client,
            admitted.continuity.connection_generation,
            command_id - 1,
            container_tree::COMMAND_TYPE_CONTAINER_VIEW_INTENT,
            &open,
        )
        .await?;
        let mut opened = false;
        let mut source = None;
        while !opened || source.is_none() {
            let raw = super::super::item_look::read_frame(&mut client).await?;
            let envelope = protocol::decode_wire_envelope(&raw).map_err(debug)?;
            if envelope.message_type() == protocol::MessageType::CommandResult {
                let result = protocol::decode_command_result(envelope.payload()).map_err(debug)?;
                if result.command_id == command_id - 1 {
                    assert_eq!(
                        container_tree::decode_container_view_result(result.payload)
                            .map_err(debug)?,
                        container_tree::ContainerViewOutcome::Opened
                    );
                    opened = true;
                }
            } else if envelope.message_type() == protocol::MessageType::StateDelta {
                let delta = protocol::decode_state_delta(envelope.payload()).map_err(debug)?;
                if delta.domain_id == container_tree::STATE_DOMAIN_CONTAINER_VIEWS {
                    let views =
                        container_tree::decode_container_views(delta.payload).map_err(debug)?;
                    source = views
                        .views
                        .iter()
                        .find(|view| view.container_handle == container)
                        .and_then(|view| {
                            view.entries
                                .iter()
                                .find(|entry| entry.item_definition_ref == source_definition)
                        })
                        .map(|entry| entry.handle);
                }
            }
        }
        let mut intent = intent.clone();
        intent.target = UseTarget::Item(source.ok_or("shown nested source")?);
        let actual = encode_item_use_intent(&intent).map_err(debug)?;
        send(
            &mut client,
            admitted.continuity.connection_generation,
            command_id,
            2,
            &actual,
        )
        .await?;
        client.shutdown().await?;
        loop {
            let raw = super::super::item_look::read_frame(&mut client).await?;
            let envelope = protocol::decode_wire_envelope(&raw).map_err(debug)?;
            if envelope.message_type() == protocol::MessageType::CommandResult {
                let result = protocol::decode_command_result(envelope.payload()).map_err(debug)?;
                if result.command_id == command_id {
                    return Ok::<_, Box<dyn std::error::Error>>((
                        protocol::world_object::decode_use_result(result.payload).map_err(debug)?,
                        actual,
                    ));
                }
            }
        }
    };
    let serving = async {
        let end = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    use std::future::Future;
    use std::task::Poll;
    let mut serving = std::pin::pin!(serving);
    let mut exercise = std::pin::pin!(exercise);
    let mut end = None;
    let mut result = None;
    let mut maintenance = std::pin::pin!(super::super::item_craft::maintain_ready(owner));
    tokio::time::timeout(
        std::time::Duration::from_secs(30),
        std::future::poll_fn(|context| {
            if let Poll::Ready(result) = maintenance.as_mut().poll(context) {
                return Poll::Ready(result);
            }
            if end.is_none()
                && let Poll::Ready(value) = serving.as_mut().poll(context)
            {
                end = Some(value);
            }
            if result.is_none()
                && let Poll::Ready(value) = exercise.as_mut().poll(context)
            {
                result = Some(value);
            }
            if end.is_some() && result.is_some() {
                Poll::Ready(Ok(()))
            } else {
                Poll::Pending
            }
        }),
    )
    .await??;
    let end = end.ok_or("nested event server did not finish")?;
    let ConnectionEnd::AdmittedThenDisconnected(next) = end else {
        return Err(format!("nested event connection: {end:?}").into());
    };
    let (disposition, actual) = result.ok_or("nested event client did not finish")??;
    Ok((next, disposition, actual))
}

async fn send(
    client: &mut tokio::io::DuplexStream,
    generation: u64,
    command_id: u64,
    command_type: u32,
    payload: &[u8],
) -> TestResult {
    let frame = protocol::encode_client_command(
        generation,
        &protocol::ClientCommandValue {
            command_id,
            command_type,
            payload,
        },
    )
    .map_err(debug)?;
    client.write_u32(u32::try_from(frame.len())?).await?;
    client.write_all(&frame).await?;
    Ok(())
}
