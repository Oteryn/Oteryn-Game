//! Drive quick-loot commands with the handles published by a real served connection.
use super::{TestResult, fixture::debug};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, IDLE_LIVENESS, serve_admitted,
};
use oteryn_protocol_oteryn::{
    self as protocol, CommandStatus, item_view, quick_loot::*, world_spatial_entities,
};
use tokio::io::{AsyncWriteExt, DuplexStream};

#[derive(Default)]
pub(super) struct ShownHandles {
    pub(super) bag: Option<[u8; 16]>,
    pub(super) corpse: Option<[u8; 16]>,
}

async fn read_handles(
    client: &mut DuplexStream,
    corpse_identity: [u8; 16],
    selected: SelectedCapabilities,
) -> TestResult<ShownHandles> {
    use world_spatial_entities::EntityDetail;
    let mut shown = ShownHandles::default();
    loop {
        let frame = super::item_look_fixtures::read_frame(client).await?;
        let envelope = protocol::decode_wire_envelope(&frame).map_err(debug)?;
        match envelope.message_type() {
            protocol::MessageType::SnapshotCommit => return Ok(shown),
            protocol::MessageType::SnapshotChunk => {}
            _ => continue,
        }
        for domain in protocol::decode_snapshot_chunk(envelope.payload())
            .map_err(debug)?
            .1
        {
            if domain.domain_id == item_view::STATE_DOMAIN_CHARACTER_INVENTORY {
                let inventory = item_view::decode_character_inventory_with_appearances(
                    domain.payload,
                    selected.contains(item_view::CAPABILITY_ITEM_EQUIP_DROP_V1),
                    selected.contains(item_view::CAPABILITY_ITEM_APPEARANCE_V1),
                )
                .map_err(debug)?;
                shown.bag = inventory
                    .entries
                    .first()
                    .map(|item| encode_target_handle(item.handle));
            } else if domain.domain_id
                == protocol::world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY
                && domain.snapshot_type
                    == world_spatial_entities::SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2
            {
                let (snapshot, _) =
                    world_spatial_entities::decode_world_spatial_entities_snapshot_with_fields(
                        domain.payload,
                        world_spatial_entities::EntityFields::selected(selected.as_slice()),
                    )
                    .map_err(debug)?;
                shown.corpse = snapshot.entities.iter().find_map(|entry| {
                    if entry.entity.identity != corpse_identity {
                        return None;
                    }
                    match entry.detail {
                        EntityDetail::Object { item_handle, .. }
                        | EntityDetail::ObjectWithAppearance { item_handle, .. } => {
                            item_handle.map(encode_target_handle)
                        }
                        _ => None,
                    }
                });
            }
        }
    }
}

pub(super) async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    corpse_identity: [u8; 16],
    requests: impl FnOnce(&ShownHandles) -> TestResult<Vec<QuickLootRequest>>,
) -> TestResult<(ShownHandles, Vec<(CommandStatus, QuickLootResult)>)> {
    let current = *admitted;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async move {
        match serve_admitted(&mut server, current, owner, IDLE_LIVENESS).await {
            ConnectionEnd::AdmittedThenDisconnected(next) => Ok(next),
            end => Err(format!("quick-loot wire session ended: {end:?}").into()),
        }
    };
    let exercise = async move {
        let shown = read_handles(
            &mut client,
            corpse_identity,
            current.continuity.selected_capabilities,
        )
        .await?;
        let mut replies = Vec::new();
        for (offset, request) in requests(&shown)?.into_iter().enumerate() {
            let command_id = current.continuity.next_command_id + u64::try_from(offset)?;
            let payload = encode_quick_loot_request(&request).map_err(debug)?;
            let frame = protocol::encode_client_command(
                current.continuity.connection_generation,
                &protocol::ClientCommandValue {
                    command_id,
                    command_type: COMMAND_TYPE_QUICK_LOOT,
                    payload: &payload,
                },
            )
            .map_err(debug)?;
            client.write_u32(u32::try_from(frame.len())?).await?;
            client.write_all(&frame).await?;
            loop {
                let frame = super::item_look_fixtures::read_frame(&mut client).await?;
                let envelope = protocol::decode_wire_envelope(&frame).map_err(debug)?;
                if envelope.message_type() == protocol::MessageType::CommandResult {
                    let reply =
                        protocol::decode_command_result(envelope.payload()).map_err(debug)?;
                    assert_eq!(reply.command_id, command_id);
                    replies.push((
                        reply.status,
                        decode_quick_loot_result(reply.payload).map_err(debug)?,
                    ));
                    break;
                }
            }
        }
        client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>((shown, replies))
    };
    let (next, result) = tokio::time::timeout(std::time::Duration::from_secs(45), async {
        tokio::try_join!(serving, exercise)
    })
    .await??;
    *admitted = next;
    Ok(result)
}
