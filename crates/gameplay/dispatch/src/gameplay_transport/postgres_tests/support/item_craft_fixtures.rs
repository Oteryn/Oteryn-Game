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
    exchange_with_map_item(owner, admitted, command_id, payload, current_source, source_offset, None).await
}

pub(super) async fn exchange_with_map_item(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
    payload: &[u8],
    current_source: Option<std::num::NonZeroU32>,
    source_offset: usize,
    map_item: Option<(i32, i32, i16, u16)>,
) -> TestResult<(AdmittedSession, UseDisposition, Vec<u8>)> {
    exchange_selected_operands(
        owner,
        admitted,
        command_id,
        payload,
        current_source,
        source_offset,
        map_item.map(|(x, y, floor, appearance)| SelectedTarget::OnMap(x, y, floor, appearance)),
    )
    .await
}

pub(super) async fn exchange_carried(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
    payload: &[u8],
    current_source: Option<std::num::NonZeroU32>,
    current_target: Option<std::num::NonZeroU32>,
) -> TestResult<(AdmittedSession, UseDisposition, Vec<u8>)> {
    exchange_selected_operands(
        owner,
        admitted,
        command_id,
        payload,
        current_source,
        0,
        current_target.map(SelectedTarget::Carried),
    )
    .await
}

#[derive(Clone, Copy)]
enum SelectedTarget {
    Carried(std::num::NonZeroU32),
    OnMap(i32, i32, i16, u16),
}

async fn exchange_selected_operands(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
    payload: &[u8],
    current_source: Option<std::num::NonZeroU32>,
    source_offset: usize,
    selected_target: Option<SelectedTarget>,
) -> TestResult<(AdmittedSession, UseDisposition, Vec<u8>)> {
    use oteryn_protocol_oteryn::world_map::{MapOrigin, STATE_DOMAIN_WORLD_MAP_VIEW, decode_world_map_snapshot};
    use oteryn_protocol_oteryn::world_object::MapUseTarget;
    if owner.root.has_ready_demand() {
        owner.root.maintain_ready_once().await.map_err(debug)?;
    }
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let client_task = async move {
        let mut source = None;
        let mut target = None;
        loop {
            let raw = super::item_look_fixtures::read_frame(&mut client).await?;
            let envelope = crate::foundation::decode_wire_envelope(&raw)?;
            if envelope.message_type() == crate::foundation::MessageType::SnapshotChunk {
                let (_, domains) = oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload())?;
                for domain in domains {
                    if domain.domain_id
                        == oteryn_protocol_oteryn::item_view::STATE_DOMAIN_CHARACTER_INVENTORY
                    {
                        let inventory =
                            oteryn_protocol_oteryn::item_view::decode_character_inventory(
                                domain.payload,
                            )
                            .map_err(debug)?;
                        source = inventory
                            .entries
                            .iter()
                            .filter(|entry| Some(entry.item_definition_ref) == current_source)
                            .nth(source_offset)
                            .map(|entry| entry.handle);
                        if let Some(SelectedTarget::Carried(definition)) = selected_target {
                            target = inventory
                                .entries
                                .iter()
                                .find(|entry| entry.item_definition_ref == definition)
                                .map(|entry| entry.handle);
                        }
                    }
                    if domain.domain_id == STATE_DOMAIN_WORLD_MAP_VIEW
                        && let Some(SelectedTarget::OnMap(x, y, floor, appearance)) = selected_target
                    {
                        let map = decode_world_map_snapshot(domain.payload).map_err(debug)?;
                        target = map.tiles.iter()
                            .find(|tile| tile.position.x == x && tile.position.y == y && tile.position.floor == floor)
                            .and_then(|tile| tile.items.iter().filter(|item| item.appearance_id == appearance).find_map(|item| match item.origin {
                                MapOrigin::Handle(handle) => Some(handle),
                                _ => None,
                            }));
                    }
                }
            }
            if envelope.message_type() == crate::foundation::MessageType::SnapshotCommit { break; }
        }
        let actual = if current_source.is_some() || selected_target.is_some() {
            let mut intent = oteryn_protocol_oteryn::world_object::decode_use_intent_target(admitted.continuity.selected_capabilities.as_slice(), payload).map_err(debug)?;
            if current_source.is_some() {
                intent.target = UseTarget::Item(source.ok_or("current source handle")?);
            }
            if selected_target.is_some() {
                intent.use_with_map_item = Some(MapUseTarget::Item(target.ok_or("current map item handle")?));
            }
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
    let (result, actual) = result.ok_or("client did not finish")??;
    let ConnectionEnd::AdmittedThenDisconnected(admitted) = end else { return Err(format!("{end:?}").into()); };
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
