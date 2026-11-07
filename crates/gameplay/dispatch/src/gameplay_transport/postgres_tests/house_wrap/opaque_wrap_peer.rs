//! Actual native402 opaque Wrap uses the handle from this exact House map.
use super::*;
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, ConnectionEnd, IDLE_LIVENESS, serve_admitted},
};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, MessageType, decode_command_result, decode_wire_envelope,
    encode_client_command,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
pub(super) async fn command(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: u64,
    kind: u32,
    body: Vec<u8>,
    observed: Option<std::num::NonZeroU32>,
    carried: bool,
) -> TestResult<(CommandStatus, Vec<u8>, Vec<Vec<u8>>, Vec<u8>)> {
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let initial = *admitted;
    let client_task = async {
        let mut frames: Vec<Vec<u8>> = Vec::new();
        let mut handle = None;
        let mut map_floor = None;
        loop {
            let raw = super::super::item_look::read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&raw).map_err(debug)?;
            if envelope.message_type() == MessageType::SnapshotChunk {
                let (_, domains) =
                    oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload())
                        .map_err(debug)?;
                for domain in domains {
                    if domain.domain_id
                        == oteryn_protocol_oteryn::item_view::STATE_DOMAIN_CHARACTER_INVENTORY
                    {
                        let inventory=oteryn_protocol_oteryn::item_view::decode_character_inventory_with_equip_drop(domain.payload,initial.continuity.selected_capabilities.contains(12)).map_err(debug)?;
                        handle = inventory
                            .entries
                            .iter()
                            .find(|entry| Some(entry.item_definition_ref) == observed)
                            .map(|entry| entry.handle)
                            .or(handle);
                    }
                    if domain.domain_id
                        == oteryn_protocol_oteryn::world_map::STATE_DOMAIN_WORLD_MAP_VIEW
                    {
                        let map = oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(
                            domain.payload,
                        )
                        .map_err(debug)?;
                        map_floor = Some(map.header.origin.floor);
                        handle = map
                            .tiles
                            .iter()
                            .flat_map(|tile| tile.items.iter())
                            .find_map(|item| match (item.definition, item.origin) {
                                (
                                    oteryn_protocol_oteryn::world_map::MapDefinition::Item(
                                        reference,
                                    ),
                                    oteryn_protocol_oteryn::world_map::MapOrigin::Handle(handle),
                                ) if Some(reference) == observed => Some(handle),
                                _ => None,
                            })
                            .or(handle);
                    }
                }
            }
            let committed = envelope.message_type() == MessageType::SnapshotCommit;
            frames.push(raw);
            if committed {
                break;
            }
        }
        let body = if let Some(observed_definition) = observed {
            let handle = handle.ok_or("actual House snapshot must issue item handle")?;
            if Some(kind) == oteryn_protocol_oteryn::item_look::ITEM_LOOK_COMMAND_ID {
                oteryn_protocol_oteryn::item_look::encode_item_look_intent_for(
                    oteryn_protocol_oteryn::item_look::ItemLookIntent {
                        target: oteryn_protocol_oteryn::item_look::ItemLookTarget::Item(handle),
                    },
                    oteryn_protocol_oteryn::item_look::ItemLookSelection { extended: true },
                )
                .map_err(debug)?
            } else if kind == 9 {
                use oteryn_protocol_oteryn::item_view::*;
                encode_item_move_intent_for(
                    &ItemMoveIntent {
                        source: handle,
                        destination: if body == [0] {
                            ItemMoveDestination::MainBackpack
                        } else {
                            ItemMoveDestination::Ground(
                                oteryn_protocol_oteryn::item_view::WorldTilePosition {
                                    x: POSITION[0],
                                    y: POSITION[1],
                                    floor: map_floor.ok_or("actual House map wire floor")?,
                                },
                            )
                        },
                    },
                    ItemMoveSelection {
                        world_interaction: true,
                        ..Default::default()
                    },
                )
                .map_err(debug)?
            } else if kind == 2 {
                oteryn_protocol_oteryn::world_object::encode_item_use_intent(
                    &oteryn_protocol_oteryn::world_object::UseIntent {
                        target: oteryn_protocol_oteryn::world_object::UseTarget::Item(handle),
                        use_with: None,
                        use_with_map_item: None,
                        use_with_position: None,
                    },
                )
                .map_err(debug)?
            } else {
                let request = oteryn_protocol_oteryn::house_wrap::OpaqueWrapRequest {
                    handle,
                    observed_definition,
                };
                if body == [5] {
                    request.encode_probe().to_vec()
                } else if carried {
                    request.encode_carried().to_vec()
                } else {
                    request.encode().to_vec()
                }
            }
        } else {
            body
        };
        let frame = encode_client_command(
            initial.continuity.connection_generation,
            &ClientCommandValue {
                command_id: command,
                command_type: kind,
                payload: &body,
            },
        )
        .map_err(debug)?;
        client.write_u32(u32::try_from(frame.len())?).await?;
        client.write_all(&frame).await?;
        client.shutdown().await?;
        let mut tail = Vec::new();
        client.read_to_end(&mut tail).await?;
        let mut cursor = 0;
        let mut result = None;
        while cursor < tail.len() {
            let length = u32::from_be_bytes(tail[cursor..cursor + 4].try_into()?) as usize;
            let frame = &tail[cursor + 4..cursor + 4 + length];
            let envelope = decode_wire_envelope(frame).map_err(debug)?;
            if envelope.message_type() == MessageType::CommandResult {
                let value = decode_command_result(envelope.payload()).map_err(debug)?;
                assert_eq!(value.command_id, command);
                result = Some((value.status, value.payload.to_vec()));
            }
            frames.push(frame.to_vec());
            cursor += 4 + length;
        }
        let (status, payload) = result.ok_or("actual Royal native command result")?;
        Ok::<_, Box<dyn std::error::Error>>((status, payload, frames, frame))
    };
    let server_task = async {
        let end = serve_admitted(&mut server, initial, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    use std::future::Future;
    use std::task::Poll;
    let mut serving = std::pin::pin!(server_task);
    let mut peer = std::pin::pin!(client_task);
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
                && let Poll::Ready(value) = peer.as_mut().poll(context)
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
    let end = end.ok_or("Royal native serve completion")?;
    let result = result.ok_or("Royal native peer completion")?;
    if let ConnectionEnd::AdmittedThenDisconnected(next)
    | ConnectionEnd::AdmittedThenClosed(next, _) = end
    {
        *admitted = next;
    } else {
        return Err(format!("Royal native peer ended {end:?}").into());
    }
    result
}
