use super::TestResult;
use super::fixture::debug;
use super::item_look_fixtures::read_frame;
use oteryn_protocol_oteryn::{self as protocol, CommandStatus, MessageType};
use protocol::browse_field::{
    BrowseFieldIntent, COMMAND_TYPE_BROWSE_FIELD_INTENT, encode_browse_field_intent,
};
use protocol::container_tree::{
    ContainerViewOutcome, ContainerViews, STATE_DOMAIN_CONTAINER_VIEWS,
    decode_container_view_result, decode_container_views_for,
};
use protocol::item_view::{CharacterInventory, decode_character_inventory_with_equip_drop};
use tokio::io::{AsyncWriteExt, DuplexStream};

pub(super) async fn snapshot(client: &mut DuplexStream) -> TestResult<CharacterInventory> {
    let mut inventory = None;
    loop {
        let bytes = read_frame(client).await?;
        let envelope = protocol::decode_wire_envelope(&bytes).map_err(debug)?;
        if envelope.message_type() == MessageType::SnapshotChunk {
            for domain in protocol::decode_snapshot_chunk(envelope.payload())
                .map_err(debug)?
                .1
            {
                if domain.domain_id == 9 {
                    inventory = Some(
                        decode_character_inventory_with_equip_drop(domain.payload, true)
                            .map_err(debug)?,
                    );
                } else if domain.domain_id == STATE_DOMAIN_CONTAINER_VIEWS {
                    assert!(
                        decode_container_views_for(domain.payload, true)
                            .map_err(debug)?
                            .views
                            .is_empty()
                    );
                }
            }
        }
        if envelope.message_type() == MessageType::SnapshotCommit {
            return inventory.ok_or_else(|| "inventory snapshot absent".into());
        }
    }
}

pub(super) async fn send(
    client: &mut DuplexStream,
    command_id: u64,
    command_type: u32,
    payload: &[u8],
) -> TestResult {
    let frame = protocol::encode_client_command(
        1,
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

pub(super) async fn result(client: &mut DuplexStream, command_id: u64) -> TestResult<Vec<u8>> {
    loop {
        let bytes = read_frame(client).await?;
        let envelope = protocol::decode_wire_envelope(&bytes).map_err(debug)?;
        if envelope.message_type() == MessageType::CommandResult {
            let result = protocol::decode_command_result(envelope.payload()).map_err(debug)?;
            assert_eq!(result.command_id, command_id);
            assert_eq!(result.status, CommandStatus::Accepted);
            return Ok(result.payload.to_vec());
        }
    }
}

pub(super) async fn views(client: &mut DuplexStream) -> TestResult<ContainerViews> {
    loop {
        let bytes = read_frame(client).await?;
        let envelope = protocol::decode_wire_envelope(&bytes).map_err(debug)?;
        if envelope.message_type() == MessageType::StateDelta {
            let delta = protocol::decode_state_delta(envelope.payload()).map_err(debug)?;
            if delta.domain_id == STATE_DOMAIN_CONTAINER_VIEWS {
                return decode_container_views_for(delta.payload, true).map_err(debug);
            }
        }
    }
}

pub(super) async fn browse(
    client: &mut DuplexStream,
    command_id: u64,
    intent: BrowseFieldIntent,
) -> TestResult {
    send(
        client,
        command_id,
        COMMAND_TYPE_BROWSE_FIELD_INTENT,
        &encode_browse_field_intent(&intent).map_err(debug)?,
    )
    .await?;
    assert_eq!(
        decode_container_view_result(&result(client, command_id).await?).map_err(debug)?,
        ContainerViewOutcome::Opened
    );
    Ok(())
}
