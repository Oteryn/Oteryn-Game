// Source fixture before the lethal gift, on the same actual transport.
pub(super) struct HouseLampPrelude<'a> {
    pub(super) harness: &'a Harness,
    pub(super) item: [u8; 16],
    pub(super) definition: std::num::NonZeroU32,
}
// Test-only live command preceding the genuine lethal gift; no EOF/re-admission.
async fn use_lamp_before_death(
    client: &mut tokio::io::DuplexStream,
    admitted: AdmittedSession,
    gift_command: u64,
    lamp: oteryn_protocol_oteryn::item_view::ItemHandle,
    gift_payload: &[u8],
) -> TestResult {
    let command = gift_command
        .checked_sub(1)
        .ok_or("actual lamp prelude command")?;
    let mut intent = oteryn_protocol_oteryn::world_object::decode_use_intent_target(
        admitted.continuity.selected_capabilities.as_slice(),
        gift_payload,
    )
    .map_err(debug)?;
    intent.target = UseTarget::Item(lamp);
    assert!(
        intent.use_with.is_none()
            && intent.use_with_map_item.is_none()
            && intent.use_with_position.is_none()
    );
    let payload = encode_item_use_intent(&intent).map_err(debug)?;
    let frame = encode_client_command(
        admitted.continuity.connection_generation,
        &ClientCommandValue {
            command_id: command,
            command_type: 2,
            payload: &payload,
        },
    )
    .map_err(debug)?;
    client.write_u32(u32::try_from(frame.len())?).await?;
    client.write_all(&frame).await?;
    let (mut committed, mut refreshed) = (false, false);
    while !committed || !refreshed {
        let raw = super::super::item_look::read_frame(client).await?;
        let envelope = decode_wire_envelope(&raw).map_err(debug)?;
        if envelope.message_type() == MessageType::CommandResult {
            let result = decode_command_result(envelope.payload()).map_err(debug)?;
            assert_eq!(result.command_id, command);
            assert_eq!(result.status, CommandStatus::Accepted);
            assert_eq!(
                oteryn_protocol_oteryn::world_object::decode_use_result(result.payload)
                    .map_err(debug)?,
                UseDisposition::Committed
            );
            committed = true;
        }
        if envelope.message_type() == MessageType::StateDelta {
            let delta =
                oteryn_protocol_oteryn::decode_state_delta(envelope.payload()).map_err(debug)?;
            refreshed |= delta.domain_id
                == oteryn_protocol_oteryn::item_view::STATE_DOMAIN_CHARACTER_INVENTORY;
        }
    }
    Ok(())
}
