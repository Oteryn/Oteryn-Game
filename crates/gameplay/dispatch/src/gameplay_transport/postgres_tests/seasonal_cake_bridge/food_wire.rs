//! Full actual framed native Food USE transcript, including ordinary chat7 egress.
use super::*;
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, IDLE_LIVENESS, serve_admitted,
};
use oteryn_protocol_oteryn::world_object::{UseDisposition, UseTarget, encode_item_use_intent};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
pub(super) async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
    payload: &[u8],
    current_source: Option<std::num::NonZeroU32>,
) -> TestResult<(AdmittedSession, UseDisposition, Vec<u8>, Vec<Vec<u8>>)> {
    if owner.root.has_ready_demand() {
        owner.root.maintain_ready_once().await.map_err(debug)?;
    }
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let client_task = async {
        let mut source = None;
        let mut frames = Vec::new();
        loop {
            let raw = super::super::item_look::read_frame(&mut client).await?;
            frames.push(raw.clone());
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
            .map_err(debug)?;
            intent.target = UseTarget::Item(source.ok_or("current source handle")?);
            encode_item_use_intent(&intent).map_err(debug)?
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
        let mut disposition = None;
        let mut close_at = None;
        loop {
            // Keep the genuine session alive for two ordinary CHAT ticks after
            // the result; retain partial prefixes across the cancel-safe read.
            let mut prefix = [0_u8; 4];
            let mut used = 0;
            while used < prefix.len() {
                let read = if let Some(deadline) = close_at {
                    match tokio::time::timeout_at(deadline, client.read(&mut prefix[used..])).await
                    {
                        Ok(read) => read,
                        Err(_) => {
                            client.shutdown().await?;
                            close_at = None;
                            continue;
                        }
                    }
                } else {
                    client.read(&mut prefix[used..]).await
                };
                match read {
                    Ok(0) if used == 0 && disposition.is_some() => {
                        return Ok::<_, Box<dyn std::error::Error>>((
                            disposition.ok_or("actual Food USE result")?,
                            actual,
                            frames,
                        ));
                    }
                    Ok(0) => return Err("partial server frame or missing Food result".into()),
                    Ok(n) => used += n,
                    Err(error) => return Err(error.into()),
                }
            }
            let length = usize::try_from(u32::from_be_bytes(prefix))?;
            if length > 1 << 20 {
                return Err("oversized actual Food frame".into());
            }
            let mut raw = vec![0; length];
            client.read_exact(&mut raw).await?;
            frames.push(raw.clone());
            let envelope = crate::foundation::decode_wire_envelope(&raw)?;
            if envelope.message_type() == crate::foundation::MessageType::CommandResult {
                let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
                if result.command_id == command_id {
                    close_at = Some(
                        tokio::time::Instant::now()
                            + crate::gameplay_transport::chat_intent::CHAT_REFRESH * 2,
                    );
                    disposition = Some(
                        oteryn_protocol_oteryn::world_object::decode_use_result(result.payload)
                            .map_err(debug)?,
                    );
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
                && let Poll::Ready(value) = client_task.as_mut().poll(context)
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
    let (result, actual, frames) = result.ok_or("client did not finish")??;
    Ok((admitted, result, actual, frames))
}
