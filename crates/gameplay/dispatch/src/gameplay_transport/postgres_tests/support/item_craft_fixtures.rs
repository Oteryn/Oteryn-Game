use tokio::io::AsyncWriteExt;
use super::fixture::debug;

use super::TestResult;

use crate::gameplay_transport::ComposedFreshAdmission;

use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, IDLE_LIVENESS,
    serve_admitted,
};

use oteryn_protocol_oteryn::world_object::{
    UseDisposition, UseTarget, encode_item_use_intent,
};

pub(super) async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
    payload: &[u8],
    current_source: Option<std::num::NonZeroU32>,
) -> TestResult<(AdmittedSession, UseDisposition, Vec<u8>)> {
    exchange_selected(owner, admitted, command_id, payload, current_source, 0).await
}

pub(super) async fn exchange_selected(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
    payload: &[u8],
    current_source: Option<std::num::NonZeroU32>,
    source_offset: usize,
) -> TestResult<(AdmittedSession, UseDisposition, Vec<u8>)> {
    if owner.root.has_ready_demand() {
        owner.root.maintain_ready_once().await.map_err(debug)?;
    }
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let client_task = async {
        let mut source = None;
        loop {
            let raw = super::item_look_fixtures::read_frame(&mut client).await?;
            let envelope = crate::foundation::decode_wire_envelope(&raw)?;
            if envelope.message_type() == crate::foundation::MessageType::SnapshotChunk {
                let (_, domains) = oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload())?;
                for domain in domains {
                    if domain.domain_id == oteryn_protocol_oteryn::item_view::STATE_DOMAIN_CHARACTER_INVENTORY {
                        let inventory = oteryn_protocol_oteryn::item_view::decode_character_inventory(domain.payload).map_err(debug)?;
                        source = inventory.entries.iter().filter(|entry| Some(entry.item_definition_ref) == current_source).nth(source_offset).map(|entry| entry.handle);
                    }
                }
            }
            if envelope.message_type() == crate::foundation::MessageType::SnapshotCommit { break; }
        }
        let actual = if current_source.is_some() {
            let mut intent = oteryn_protocol_oteryn::world_object::decode_use_intent_target(admitted.continuity.selected_capabilities.as_slice(), payload).map_err(debug)?;
            intent.target = UseTarget::Item(source.ok_or("current source handle")?);
            encode_item_use_intent(&intent).map_err(debug)?
        } else { payload.to_vec() };
        let frame = oteryn_protocol_oteryn::encode_client_command(1, &oteryn_protocol_oteryn::ClientCommandValue { command_id, command_type: 2, payload: &actual })?;
        client.write_u32(u32::try_from(frame.len())?).await?;
        client.write_all(&frame).await?;
        client.shutdown().await?;
        loop {
            let raw = super::item_look_fixtures::read_frame(&mut client).await?;
            let envelope = crate::foundation::decode_wire_envelope(&raw)?;
            if envelope.message_type() == crate::foundation::MessageType::CommandResult {
                let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
                if result.command_id == command_id {
                    let disposition = oteryn_protocol_oteryn::world_object::decode_use_result(result.payload).map_err(debug)?;
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
    let mut maintenance = std::pin::pin!(maintain_ready(owner));
    tokio::time::timeout(std::time::Duration::from_secs(30), std::future::poll_fn(|context| {
        if let Poll::Ready(result) = maintenance.as_mut().poll(context) {
            return Poll::Ready(result);
        }
        if end.is_none() && let Poll::Ready(value) = serving.as_mut().poll(context) {
            end = Some(value);
        }
        if result.is_none() && let Poll::Ready(value) = client_task.as_mut().poll(context) {
            result = Some(value);
        }
        if end.is_some() && result.is_some() { Poll::Ready(Ok(())) } else { Poll::Pending }
    })).await??;
    let end = end.ok_or("server did not finish")?;
    let ConnectionEnd::AdmittedThenDisconnected(admitted) = end else { return Err(format!("{end:?}").into()); };
    let (result, actual) = result.ok_or("client did not finish")??;
    Ok((admitted, result, actual))
}

/// Stand in for the node maintenance driver while a fixture serves a connection.
pub(super) async fn maintain_ready(owner: &ComposedFreshAdmission<'_, '_, '_>) -> TestResult {
    loop {
        if owner.root.has_ready_demand() {
            owner.root.maintain_ready_once().await.map_err(debug)?;
        }
        tokio::task::yield_now().await;
    }
}
