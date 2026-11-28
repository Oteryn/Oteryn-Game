use super::super::item_look_fixtures::read_frame;
use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub(super) async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    payloads: &[Vec<u8>],
) -> TestResult<(Vec<UseDisposition>, AdmittedSession)> {
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    for (command_id, payload) in (admitted.continuity.next_command_id..).zip(payloads) {
        let frame = oteryn_protocol_oteryn::encode_client_command(
            1,
            &oteryn_protocol_oteryn::ClientCommandValue {
                command_id,
                command_type: 2,
                payload,
            },
        )?;
        client
            .write_all(&u32::try_from(frame.len())?.to_be_bytes())
            .await?;
        client.write_all(&frame).await?;
    }
    client.shutdown().await?;
    let end = crate::gameplay_transport::connection::serve_admitted(
        &mut server,
        admitted,
        &WireOwner::new(owner, false),
        crate::gameplay_transport::connection::IDLE_LIVENESS,
    )
    .await;
    let crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenDisconnected(retained) =
        end
    else {
        return Err(format!("{end:?}").into());
    };
    drop(server);
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    let mut cursor = 0;
    let mut results = Vec::new();
    while cursor < output.len() {
        let length = u32::from_be_bytes(output[cursor..cursor + 4].try_into()?) as usize;
        let frame =
            crate::foundation::decode_wire_envelope(&output[cursor + 4..cursor + 4 + length])?;
        if frame.message_type() == crate::foundation::MessageType::CommandResult {
            let result = oteryn_protocol_oteryn::decode_command_result(frame.payload())?;
            results.push(
                oteryn_protocol_oteryn::world_object::decode_use_result(result.payload)
                    .map_err(debug)?,
            );
        }
        cursor += 4 + length;
    }
    Ok((results, retained))
}

pub(super) async fn projection_retry(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    actor: ExactActorRef,
    session: GameSessionId,
    index: &ItemDefinitionIndex,
) -> TestResult {
    use crate::foundation::{MessageType, decode_wire_envelope};
    use crate::gameplay_transport::capabilities::{
        BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
    };
    use crate::gameplay_transport::connection::{IDLE_LIVENESS, serve_admitted};
    use oteryn_protocol_oteryn::item_view::{
        STATE_DOMAIN_CHARACTER_INVENTORY, decode_character_inventory,
    };
    use oteryn_protocol_oteryn::world_object::{
        COMMAND_TYPE_USE_INTENT, MapUseTarget, UseTarget, decode_use_result, encode_item_use_intent,
    };
    use oteryn_protocol_oteryn::{
        ClientCommandValue, CommandStatus, decode_command_result, decode_snapshot_chunk,
        decode_state_delta, encode_client_command,
    };

    let wire = WireOwner::new(owner, true);
    let admitted = AdmittedSession {
        game_session_id: session,
        world_id: owner.world_id,
        channel_id: owner.channel_id,
        runtime_actor: Some(actor),
        first_entry: FirstEntryOutcome::Positioned,
        controller: None,
        continuity: SessionContinuity {
            selected_capabilities: SelectedCapabilities::select(
                BUNDLE_WORLD_OFFERED_CAPABILITIES,
                &[4, 11, 15],
            )
            .ok_or("item USE capabilities")?,
            ..SessionContinuity::FRESH
        },
        item_fence: Some(item_fence()?),
    };
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
        let result = serve_admitted(&mut server, admitted, &wire, IDLE_LIVENESS).await;
        drop(server);
        result
    };
    let client_task = async {
        let mut inventory = None;
        let mut revision = None;
        loop {
            let frame = read_frame(&mut client).await?;
            let frame = decode_wire_envelope(&frame)?;
            match frame.message_type() {
                MessageType::SnapshotChunk => {
                    let (_, domains) = decode_snapshot_chunk(frame.payload())?;
                    for domain in domains {
                        if domain.domain_id == STATE_DOMAIN_CHARACTER_INVENTORY {
                            inventory =
                                Some(decode_character_inventory(domain.payload).map_err(debug)?);
                            revision = Some(domain.revision);
                        }
                    }
                }
                MessageType::SnapshotCommit => break,
                _ => {}
            }
        }
        let inventory = inventory.ok_or("initial inventory")?;
        let find = |key| -> TestResult<_> {
            let definition = index
                .revision(key)
                .and_then(|revision| index.definition_ref("Item", key, revision))
                .ok_or("pinned item definition")?;
            Ok(inventory
                .entries
                .iter()
                .find(|item| item.item_definition_ref == definition)
                .ok_or("carried fixture item")?
                .handle)
        };
        let pick = find(oteryn_gameplay_items::mining::PICK)?;
        let stone = find(oteryn_gameplay_items::mining::SHINY_STONE)?;
        let payload = encode_item_use_intent(&UseIntent {
            target: UseTarget::Item(pick),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Item(stone)),
            use_with_position: None,
        })
        .map_err(debug)?;
        let frame = encode_client_command(
            admitted.continuity.connection_generation,
            &ClientCommandValue {
                command_id: admitted.continuity.next_command_id,
                command_type: COMMAND_TYPE_USE_INTENT,
                payload: &payload,
            },
        )?;
        client.write_u32(u32::try_from(frame.len())?).await?;
        client.write_all(&frame).await?;
        let mut committed_sequence = None;
        loop {
            let frame = read_frame(&mut client).await?;
            let frame = decode_wire_envelope(&frame)?;
            match frame.message_type() {
                MessageType::CommandResult => {
                    let result = decode_command_result(frame.payload())?;
                    assert_eq!(result.status, CommandStatus::Accepted);
                    assert_eq!(
                        decode_use_result(result.payload).map_err(debug)?,
                        UseDisposition::Committed
                    );
                    assert!(committed_sequence.replace(frame.server_sequence()).is_none());
                }
                MessageType::StateDelta => {
                    let delta = decode_state_delta(frame.payload())?;
                    if delta.domain_id != STATE_DOMAIN_CHARACTER_INVENTORY {
                        continue;
                    }
                    assert_eq!(Some(delta.base_revision), revision);
                    assert!(
                        frame.server_sequence()
                            > committed_sequence.ok_or("committed reply precedes inventory")?
                    );
                    let updated = decode_character_inventory(delta.payload).map_err(debug)?;
                    assert!(updated.entries.iter().any(|item| item.handle == pick));
                    assert!(updated.entries.iter().all(|item| item.handle != stone));
                    assert_eq!(
                        updated.entries.iter().filter(|item| {
                            inventory.entries.iter().all(|prior| prior.handle != item.handle)
                        }).count(),
                        1,
                        "one reward handle is published",
                    );
                    break;
                }
                _ => {}
            }
        }
        client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let (_, client_result) = tokio::time::timeout(IDLE_LIVENESS.interval, async {
        tokio::join!(serving, client_task)
    })
    .await?;
    client_result?;
    assert_eq!(wire.rejected_projections.get(), 1);
    assert_eq!(harness.count("game_pick_refinements").await?, 1);
    Ok(())
}
