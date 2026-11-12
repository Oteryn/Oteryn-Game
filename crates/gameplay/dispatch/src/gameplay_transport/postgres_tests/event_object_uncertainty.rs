//! Real object USE must close without a result when its durable outcome is unresolved.
use super::TestResult;
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, IDLE_LIVENESS, serve_admitted,
};
use tokio::io::AsyncWriteExt;

pub(super) async fn exchange_uncertain(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    payload: &[u8],
) -> TestResult {
    exchange_uncertain_inner(owner, admitted, payload, None).await
}

pub(super) async fn exchange_item_uncertain(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    payload: &[u8],
    current_source: std::num::NonZeroU32,
) -> TestResult {
    exchange_uncertain_inner(owner, admitted, payload, Some(current_source)).await
}

async fn exchange_uncertain_inner(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    payload: &[u8],
    current_source: Option<std::num::NonZeroU32>,
) -> TestResult {
    let command_id = admitted.continuity.next_command_id;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let client_task = async {
        let mut source = None;
        loop {
            let raw = super::item_look::read_frame(&mut client).await?;
            let envelope = crate::foundation::decode_wire_envelope(&raw)?;
            if current_source.is_some()
                && envelope.message_type() == crate::foundation::MessageType::SnapshotChunk
            {
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
                            .map_err(super::fixture::debug)?;
                        source = inventory
                            .entries
                            .iter()
                            .find(|entry| Some(entry.item_definition_ref) == current_source)
                            .map(|entry| entry.handle);
                    }
                }
            }
            if envelope.message_type() == crate::foundation::MessageType::SnapshotCommit {
                break;
            }
        }
        let actual = if current_source.is_some() {
            let mut intent = oteryn_protocol_oteryn::world_object::decode_use_intent_target(
                admitted.continuity.selected_capabilities.as_slice(),
                payload,
            )
            .map_err(super::fixture::debug)?;
            intent.target = oteryn_protocol_oteryn::world_object::UseTarget::Item(
                source.ok_or("current source handle")?,
            );
            oteryn_protocol_oteryn::world_object::encode_item_use_intent(&intent)
                .map_err(super::fixture::debug)?
        } else {
            payload.to_vec()
        };
        let frame = oteryn_protocol_oteryn::encode_client_command(
            1,
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
            let raw = match super::item_look::read_frame(&mut client).await {
                Ok(raw) => raw,
                Err(error)
                    if error
                        .downcast_ref::<std::io::Error>()
                        .is_some_and(|error| error.kind() == std::io::ErrorKind::UnexpectedEof) =>
                {
                    return Ok::<_, Box<dyn std::error::Error>>(());
                }
                Err(error) => return Err(error),
            };
            let envelope = crate::foundation::decode_wire_envelope(&raw)?;
            if envelope.message_type() == crate::foundation::MessageType::CommandResult {
                let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
                if result.command_id == command_id {
                    return Err("unresolved event object USE sent a terminal command result".into());
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
    tokio::time::timeout(
        std::time::Duration::from_secs(30),
        std::future::poll_fn(|context| {
            if end.is_none()
                && let Poll::Ready(value) = serving.as_mut().poll(context)
            {
                end = Some(value);
            }
            if result.is_none()
                && let Poll::Ready(value) = client_task.as_mut().poll(context)
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
    let ConnectionEnd::AdmittedThenDisconnected(after) =
        end.ok_or("event object server did not finish")?
    else {
        return Err("event object failed before admission".into());
    };
    result.ok_or("event object client did not finish")??;
    // The existing ingress reserves its successor before dispatch. The absence of a
    // terminal CommandResult above is the uncertainty proof; retry reconstructs from the
    // saved initial admission and the retained durable command, never this reservation.
    assert_eq!(
        after.continuity.next_command_id,
        command_id.checked_add(1).ok_or("command exhausted")?
    );
    Ok(())
}
