use tokio::io::AsyncWriteExt;
use super::fixture::debug;

use super::TestResult;

use oteryn_protocol_oteryn::CommandStatus;

use oteryn_protocol_oteryn::world_object::{
    UseTarget, encode_item_use_intent,
};

use std::num::NonZeroU32;

use std::time::Duration;

pub(super) async fn exchange(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: super::AdmittedSession,
    command_id: u64,
    payload: &[u8],
    current_source: Option<NonZeroU32>,
    current_target: Option<NonZeroU32>,
) -> TestResult<(super::AdmittedSession, CommandStatus, Vec<u8>, Vec<u8>)> {
    use crate::gameplay_transport::connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted};
    use oteryn_protocol_oteryn::world_object::MapUseTarget;
    use std::future::Future;
    use std::task::Poll;

    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let driving = async {
        let mut source = None;
        let mut target = None;
        loop {
            let raw = super::item_look_fixtures::read_frame(&mut client).await?;
            let envelope = crate::foundation::decode_wire_envelope(&raw)?;
            if envelope.message_type() == crate::foundation::MessageType::SnapshotChunk {
                let (_, domains) =
                    oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload())?;
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
                            .find(|entry| Some(entry.item_definition_ref) == current_source)
                            .map(|entry| entry.handle);
                        target = inventory
                            .entries
                            .iter()
                            .find(|entry| Some(entry.item_definition_ref) == current_target)
                            .map(|entry| entry.handle);
                    }
                }
            }
            if envelope.message_type() == crate::foundation::MessageType::SnapshotCommit {
                break;
            }
        }
        let actual = if current_source.is_some() || current_target.is_some() {
            let mut intent = oteryn_protocol_oteryn::world_object::decode_use_intent_target(
                admitted.continuity.selected_capabilities.as_slice(),
                payload,
            )
            .map_err(debug)?;
            if current_source.is_some() {
                intent.target = UseTarget::Item(source.ok_or("current source handle")?);
            }
            if current_target.is_some() {
                intent.use_with_map_item =
                    Some(MapUseTarget::Item(target.ok_or("current target handle")?));
            }
            encode_item_use_intent(&intent).map_err(debug)?
        } else {
            payload.to_vec()
        };
        let frame = oteryn_protocol_oteryn::encode_client_command(
            admitted.continuity.connection_generation,
            &oteryn_protocol_oteryn::ClientCommandValue {
                command_id,
                command_type: 2,
                payload: &actual,
            },
        )?;
        client.write_u32(u32::try_from(frame.len())?).await?;
        client.write_all(&frame).await?;
        client.shutdown().await?;
        loop {
            let raw = super::item_look_fixtures::read_frame(&mut client).await?;
            let envelope = crate::foundation::decode_wire_envelope(&raw)?;
            if envelope.message_type() == crate::foundation::MessageType::CommandResult {
                let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
                if result.command_id == command_id {
                    return Ok::<_, Box<dyn std::error::Error>>((
                        result.status,
                        result.payload.to_vec(),
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
    let mut serving = std::pin::pin!(serving);
    let mut driving = std::pin::pin!(driving);
    let mut end = None;
    let mut result = None;
    // This direct connection has no node task to replenish a retired DB holder.
    let mut maintenance = std::pin::pin!(super::item_craft_fixtures::maintain_ready(owner));
    tokio::time::timeout(
        Duration::from_secs(30),
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
                && let Poll::Ready(value) = driving.as_mut().poll(context)
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
    let end = end.ok_or("server did not finish")?;
    let ConnectionEnd::AdmittedThenDisconnected(admitted) = end else {
        return Err(format!("{end:?}").into());
    };
    let (status, result, actual) = result.ok_or("client did not finish")??;
    Ok((admitted, status, result, actual))
}
