//! Read the actual advertised map handle before sending literal command9.
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, debug};
use crate::foundation::{MessageType, decode_wire_envelope};
use crate::gameplay_transport::connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted};
use oteryn_protocol_oteryn::item_view::{
    ItemHandle, ItemMoveDestination, ItemMoveIntent, ItemMoveOutcome, decode_item_move_result,
    encode_item_move_intent,
};
use oteryn_protocol_oteryn::world_map::{
    MapDefinition, MapOrigin, STATE_DOMAIN_WORLD_MAP_VIEW, decode_world_map_snapshot,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn frame(client: &mut tokio::io::DuplexStream) -> TestResult<Vec<u8>> {
    let size = client.read_u32().await?;
    if size > 1 << 20 {
        return Err("unbounded test transport frame".into());
    }
    let mut bytes = vec![0; usize::try_from(size)?];
    client.read_exact(&mut bytes).await?;
    Ok(bytes)
}

pub(super) async fn pickup(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native Item policy")?;
    let reference = native
        .item_index()
        .ok_or("native Item index")?
        .definition_ref("Item", "oteryn:item.tibia.i3030", "definition-r1")
        .ok_or("ruby reference")?;
    let initial = *admitted;
    let first = initial.continuity.next_command_id;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
        let end = serve_admitted(&mut server, initial, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let receiving = async move {
        let mut body = Vec::new();
        let mut chunks = 0;
        let mut snapshot_id = None;
        loop {
            let bytes = frame(&mut client).await?;
            let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
            if envelope.message_type() == MessageType::SnapshotChunk {
                let id = oteryn_protocol_oteryn::decode_snapshot_id(envelope.payload())
                    .map_err(debug)?;
                assert!(snapshot_id.is_none_or(|prior| prior == id));
                snapshot_id = Some(id);
                let (index, data) =
                    oteryn_protocol_oteryn::decode_snapshot_chunk_framing(envelope.payload())
                        .map_err(debug)?;
                assert_eq!(index, chunks);
                chunks += 1;
                body.extend_from_slice(data);
            } else if envelope.message_type() == MessageType::SnapshotCommit {
                assert_eq!(
                    snapshot_id,
                    Some(
                        oteryn_protocol_oteryn::decode_snapshot_id(envelope.payload())
                            .map_err(debug)?
                    )
                );
                break;
            }
        }
        let domains = oteryn_protocol_oteryn::decode_snapshot_body(&body).map_err(debug)?;
        let domain = domains
            .iter()
            .find(|domain| domain.domain_id == STATE_DOMAIN_WORLD_MAP_VIEW)
            .ok_or("native map view domain")?;
        let map = decode_world_map_snapshot(domain.payload).map_err(debug)?;
        let tile = map
            .tiles
            .iter()
            .find(|tile| {
                tile.position.x == 32437 && tile.position.y == 32174 && tile.position.floor == -15
            })
            .ok_or("native ruby map tile")?;
        assert_eq!(
            tile.items.len(),
            3,
            "source ground, ruby and Fire Field remain layered"
        );
        let ruby = &tile.items[1];
        assert_eq!(ruby.definition, MapDefinition::Item(reference));
        assert_eq!(ruby.count, 1);
        let MapOrigin::Handle(handle) = ruby.origin else {
            return Err("ruby is not a physical pickup handle".into());
        };
        assert!(
            !matches!(tile.items[2].origin, MapOrigin::Handle(_)),
            "the source Fire Field is not loot"
        );
        let payload = encode_item_move_intent(&ItemMoveIntent {
            source: ItemHandle::new(handle.get()).ok_or("ruby handle")?,
            destination: ItemMoveDestination::MainBackpack,
        })
        .map_err(debug)?;
        for (id, expected) in [
            (first, ItemMoveOutcome::Moved),
            (first, ItemMoveOutcome::Moved),
            (first + 1, ItemMoveOutcome::Stale),
        ] {
            let bytes = oteryn_protocol_oteryn::encode_client_command(
                1,
                &oteryn_protocol_oteryn::ClientCommandValue {
                    command_id: id,
                    command_type: 9,
                    payload: &payload,
                },
            )
            .map_err(debug)?;
            client.write_u32(u32::try_from(bytes.len())?).await?;
            client.write_all(&bytes).await?;
            loop {
                let bytes = frame(&mut client).await?;
                let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
                if envelope.message_type() == MessageType::CommandResult {
                    let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())
                        .map_err(debug)?;
                    assert_eq!(result.command_id, id);
                    assert_eq!(
                        decode_item_move_result(result.payload).map_err(debug)?,
                        expected
                    );
                    break;
                }
            }
        }
        client.shutdown().await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    };
    let reader = tokio::spawn(async move { receiving.await.map_err(|error| error.to_string()) });
    let end = serving.await;
    let result: TestResult = reader.await?.map_err(Into::into);
    result?;
    match end {
        ConnectionEnd::AdmittedThenDisconnected(next) => {
            *admitted = next;
            Ok(())
        }
        other => Err(format!("source pickup transport end:{other:?}").into()),
    }
}
