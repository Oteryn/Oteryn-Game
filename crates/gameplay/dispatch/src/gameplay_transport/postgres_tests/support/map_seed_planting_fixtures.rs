use tokio::io::AsyncWriteExt;
use super::fixture::debug;

use super::TestResult;

use crate::gameplay_transport::ComposedFreshAdmission;

use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, IDLE_LIVENESS,
    serve_admitted,
};

use oteryn_protocol_oteryn::item_view::*;

// Direct connections have no node maintenance task. Service demand from a
// bounded pass that retired its holder before the next fixture pass.
pub(super) async fn maintain_ready(owner: &ComposedFreshAdmission<'_, '_, '_>) -> TestResult {
    if owner.root.has_ready_demand() {
        owner.root.maintain_ready_once().await.map_err(debug)?;
    }
    Ok(())
}

// Preserve the existing wire probe controls while varying only its target tile.
#[allow(clippy::too_many_arguments)]
pub(super) async fn drop_exchange_at(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
    replay: Option<Vec<u8>>,
    seed: std::num::NonZeroU32,
    count: u8,
    expect_disconnect: bool,
    destination: WorldTilePosition,
) -> TestResult<(AdmittedSession, Option<ItemMoveOutcome>, Vec<u8>)> {
    maintain_ready(owner).await?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let client_task = async {
        let mut source = None;
        loop {
            let raw = super::item_look_fixtures::read_frame(&mut client).await?;
            let envelope = crate::foundation::decode_wire_envelope(&raw)?;
            if envelope.message_type() == crate::foundation::MessageType::SnapshotChunk {
                let (_, domains) =
                    oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload())?;
                for domain in domains {
                    if domain.domain_id == STATE_DOMAIN_CHARACTER_INVENTORY {
                        let inventory =
                            decode_character_inventory_with_equip_drop(domain.payload, true)
                                .map_err(debug)?;
                        source = inventory
                            .entries
                            .iter()
                            .find(|entry| entry.item_definition_ref == seed)
                            .map(|entry| entry.handle);
                    }
                }
            }
            if envelope.message_type() == crate::foundation::MessageType::SnapshotCommit {
                break;
            }
        }
        let actual = if let Some(replay) = replay {
            replay
        } else {
            let intent = ItemMoveIntent {
                source: source.ok_or("current seed handle")?,
                destination: ItemMoveDestination::Ground(destination),
            };
            let mut payload = encode_item_move_intent_for(
                &intent,
                ItemMoveSelection {
                    equip_drop: true,
                    container_tree: true,
                    ..ItemMoveSelection::default()
                },
            )
            .map_err(debug)?;
            payload.extend([0x30, count]);
            payload
        };
        let frame = oteryn_protocol_oteryn::encode_client_command(
            1,
            &oteryn_protocol_oteryn::ClientCommandValue {
                command_id,
                command_type: COMMAND_TYPE_ITEM_MOVE_INTENT,
                payload: &actual,
            },
        )?;
        client.write_u32(u32::try_from(frame.len())?).await?;
        client.write_all(&frame).await?;
        client.shutdown().await?;
        loop {
            let raw = match super::item_look_fixtures::read_frame(&mut client).await {
                Ok(raw) => raw,
                Err(error)
                    if expect_disconnect
                        && error.downcast_ref::<std::io::Error>().is_some_and(|error| {
                            error.kind() == std::io::ErrorKind::UnexpectedEof
                        }) =>
                {
                    return Ok::<_, Box<dyn std::error::Error>>((None, actual));
                }
                Err(error) => return Err(error),
            };
            let envelope = crate::foundation::decode_wire_envelope(&raw)?;
            if envelope.message_type() == crate::foundation::MessageType::CommandResult {
                let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
                if result.command_id == command_id {
                    let outcome = decode_item_move_result_with_equip_drop(result.payload, true)
                        .map_err(debug)?;
                    return Ok::<_, Box<dyn std::error::Error>>((Some(outcome), actual));
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
    maintain_ready(owner).await?;
    let ConnectionEnd::AdmittedThenDisconnected(admitted) =
        end.ok_or("drop server did not finish")?
    else {
        return Err("drop server ended before admission".into());
    };
    let (result, actual) = result.ok_or("drop client did not finish")??;
    Ok((admitted, result, actual))
}
