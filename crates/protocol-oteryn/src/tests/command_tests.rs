fn client_command_frame(generation: u64, command_id: u64, command_type: u64) -> Vec<u8> {
    let mut body = Vec::new();
    push_scalar(&mut body, 1, command_id);
    push_scalar(&mut body, 2, command_type);
    push_bytes(&mut body, 4, &[0x08, 0x02]);
    let mut frame = Vec::new();
    push_scalar(&mut frame, 1, MessageType::ClientCommand as u64);
    push_scalar(&mut frame, 2, generation);
    push_bytes(&mut frame, 4, &body);
    frame
}

#[test]
fn client_command_decodes_identity_type_and_payload() -> Result<(), FoundationProtocolError> {
    let frame = client_command_frame(3, 1, 1);
    let command = decode_wire_envelope(&frame)?.client_command(3)?;
    assert_eq!(
        command,
        ClientCommandView {
            command_id: 1,
            command_type: 1,
            payload: &[0x08, 0x02],
        }
    );
    assert_eq!(
        decode_wire_envelope(&frame)?.client_command(4),
        Err(FoundationProtocolError::StaleConnectionGeneration)
    );
    for (id, command_type) in [(0, 1), (1, 0), (1, u64::from(u32::MAX) + 1)] {
        let frame = client_command_frame(3, id, command_type);
        assert_eq!(
            decode_wire_envelope(&frame).and_then(|view| view.client_command(3)),
            Err(FoundationProtocolError::MalformedEnvelope),
            "{id} {command_type}"
        );
    }
    Ok(())
}

/// `encode_client_command` round-trips through the same server-side ingress
/// (`decode_wire_envelope` + `WireEnvelopeView::client_command`) the shipped server uses for
/// every real `ClientCommand`, and matches bytes transcribed independently from
/// `foundation.proto` (envelope 1/2/4, command 1/2/4).
#[test]
fn client_command_encoder_round_trips_through_server_ingress_decode()
-> Result<(), FoundationProtocolError> {
    const STEP_EAST: &[u8] = &[8, 7, 16, 3, 34, 8, 8, 5, 16, 1, 34, 2, 8, 2];
    let wire = encode_client_command(
        3,
        &ClientCommandValue {
            command_id: 5,
            command_type: 1,
            payload: &[0x08, 0x02],
        },
    )?;
    assert_eq!(wire, STEP_EAST);
    let envelope = decode_wire_envelope(&wire)?;
    assert_eq!(envelope.message_type(), MessageType::ClientCommand);
    assert_eq!(envelope.validate(Direction::ClientToServer, true), Ok(()));
    assert_eq!(envelope.server_sequence(), 0);
    assert_eq!(
        envelope.client_command(3)?,
        ClientCommandView {
            command_id: 5,
            command_type: 1,
            payload: &[0x08, 0x02],
        }
    );
    // A multi-byte command id and type survive the round trip.
    let wire = encode_client_command(
        u64::MAX,
        &ClientCommandValue {
            command_id: u64::MAX,
            command_type: u32::MAX,
            payload: &[],
        },
    )?;
    let view = decode_wire_envelope(&wire)?.client_command(u64::MAX)?;
    assert_eq!(
        (view.command_id, view.command_type, view.payload),
        (u64::MAX, u32::MAX, &[][..])
    );
    Ok(())
}

#[test]
fn client_command_encoder_refuses_what_ingress_would_refuse() {
    let value = ClientCommandValue {
        command_id: 1,
        command_type: 1,
        payload: &[],
    };
    assert_eq!(
        encode_client_command(0, &value),
        Err(FoundationProtocolError::MalformedEnvelope)
    );
    assert_eq!(
        encode_client_command(
            1,
            &ClientCommandValue {
                command_id: 0,
                ..value
            }
        ),
        Err(FoundationProtocolError::MalformedEnvelope)
    );
    assert_eq!(
        encode_client_command(
            1,
            &ClientCommandValue {
                command_type: 0,
                ..value
            }
        ),
        Err(FoundationProtocolError::MalformedEnvelope)
    );
    let at_limit = vec![0_u8; MAX_COMMAND_PAYLOAD_BYTES];
    let over_limit = vec![0_u8; MAX_COMMAND_PAYLOAD_BYTES + 1];
    assert!(
        encode_client_command(
            1,
            &ClientCommandValue {
                payload: &at_limit,
                ..value
            }
        )
        .is_ok()
    );
    assert_eq!(
        encode_client_command(
            1,
            &ClientCommandValue {
                payload: &over_limit,
                ..value
            }
        ),
        Err(FoundationProtocolError::PayloadLimitExceeded)
    );
}

/// `encode_liveness_ack` matches the explicit-zero-diagnostic vector the ingress test above
/// transcribes, and `decode_liveness_probe` reads back what the existing server-side
/// `encode_liveness_probe` emits.
#[test]
fn liveness_ack_encoder_and_probe_decoder_round_trip_through_existing_codecs()
-> Result<(), FoundationProtocolError> {
    let ack = encode_liveness_ack(7, 150, 0)?;
    assert_eq!(
        ack,
        [
            0x08, 0x06, 0x10, 0x07, 0x22, 0x05, 0x08, 0x96, 0x01, 0x10, 0x00
        ]
    );
    assert_eq!(
        decode_wire_envelope(&ack)?.liveness_ack(7)?,
        LivenessAckView {
            probe_id: 150,
            last_applied_server_sequence: Some(0),
        }
    );
    let ack = encode_liveness_ack(7, 1, 300)?;
    assert_eq!(
        decode_wire_envelope(&ack)?.liveness_ack(7)?,
        LivenessAckView {
            probe_id: 1,
            last_applied_server_sequence: Some(300),
        }
    );

    let probe = encode_liveness_probe(7, 150)?;
    let envelope = decode_wire_envelope(&probe)?;
    assert_eq!(envelope.validate(Direction::ServerToClient, true), Ok(()));
    assert_eq!(decode_liveness_probe(envelope.payload())?, 150);

    assert_eq!(
        encode_liveness_ack(0, 1, 0),
        Err(FoundationProtocolError::MalformedEnvelope)
    );
    assert_eq!(
        encode_liveness_ack(1, 0, 0),
        Err(FoundationProtocolError::MalformedEnvelope)
    );
    // Absent, zero, wrong-wire-type and repeated probe ids all fail closed.
    for payload in [
        &[][..],
        &[0x08, 0x00],
        &[0x0a, 0x01, 0x01],
        &[0x08, 0x01, 0x08, 0x02],
    ] {
        assert_eq!(
            decode_liveness_probe(payload),
            Err(FoundationProtocolError::MalformedEnvelope),
            "{payload:?}"
        );
    }
    Ok(())
}

/// `decode_command_result` round-trips through the existing server-side
/// `encode_command_result`, including the sequenced envelope the client validates first.
#[test]
fn command_result_decoder_round_trips_through_existing_server_encoder()
-> Result<(), FoundationProtocolError> {
    for (status, payload) in [
        (CommandStatus::Accepted, &[0x08, 0x01][..]),
        (CommandStatus::Rejected, &[][..]),
    ] {
        let wire = encode_command_result(4, 9, 300, status, payload)?;
        let envelope = decode_wire_envelope(&wire)?;
        assert_eq!(envelope.message_type(), MessageType::CommandResult);
        assert_eq!(envelope.validate(Direction::ServerToClient, true), Ok(()));
        assert_eq!(envelope.connection_generation(), 4);
        assert_eq!(envelope.server_sequence(), 9);
        assert_eq!(
            decode_command_result(envelope.payload())?,
            CommandResultView {
                command_id: 300,
                status,
                error_code: 0,
                payload,
            }
        );
    }
    Ok(())
}

/// An operation-terminal command error: `REJECTED`, the registered code in `error_code` and no
/// payload, in hand-computed canonical bytes. Other dispositions are refused.
#[test]
fn command_error_result_carries_only_an_operation_terminal_code()
-> Result<(), FoundationProtocolError> {
    let wire =
        encode_command_error_result(4, 9, 300, FoundationProtocolError::PayloadLimitExceeded)?;
    // CommandResult (8), generation 4, sequence 9, body: command 300 = ac 02, status 2,
    // error_code 1009 = f1 07.
    assert_eq!(
        wire,
        [
            0x08, 0x08, 0x10, 0x04, 0x18, 0x09, 0x22, 0x08, 0x08, 0xac, 0x02, 0x10, 0x02, 0x18,
            0xf1, 0x07
        ]
    );
    let envelope = decode_wire_envelope(&wire)?;
    assert_eq!(envelope.validate(Direction::ServerToClient, true), Ok(()));
    assert_eq!(
        decode_command_result(envelope.payload())?,
        CommandResultView {
            command_id: 300,
            status: CommandStatus::Rejected,
            error_code: 1009,
            payload: &[],
        }
    );
    // ACCOUNT_DATA_INTEGRITY 1050 = 9a 08: the code and nothing else (no payload, no text).
    let wire =
        encode_command_error_result(4, 9, 300, FoundationProtocolError::AccountDataIntegrity)?;
    assert_eq!(
        wire,
        [
            0x08, 0x08, 0x10, 0x04, 0x18, 0x09, 0x22, 0x08, 0x08, 0xac, 0x02, 0x10, 0x02, 0x18,
            0x9a, 0x08
        ]
    );
    assert_eq!(
        decode_command_result(decode_wire_envelope(&wire)?.payload())?,
        CommandResultView {
            command_id: 300,
            status: CommandStatus::Rejected,
            error_code: 1050,
            payload: &[],
        }
    );
    for error in [
        FoundationProtocolError::MalformedFrame,
        FoundationProtocolError::UnknownMessageType,
        FoundationProtocolError::CommandSequenceGap,
    ] {
        assert_eq!(
            encode_command_error_result(4, 9, 300, error),
            Err(FoundationProtocolError::MalformedEnvelope)
        );
    }
    assert_eq!(
        encode_command_error_result(0, 9, 300, FoundationProtocolError::PayloadLimitExceeded),
        Err(FoundationProtocolError::MalformedEnvelope)
    );
    Ok(())
}

/// The registered duplicate statuses (FND-02 §13.2) round-trip through the existing server
/// encoder, carrying the replayed payload (or none) and the sequenced envelope.
#[test]
fn command_result_decoder_represents_the_duplicate_statuses() -> Result<(), FoundationProtocolError>
{
    for (status, wire_status, payload) in [
        (CommandStatus::DuplicateReplay, 3, &[0x08, 0x02][..]),
        (CommandStatus::DuplicateOutcomeExpired, 4, &[][..]),
    ] {
        assert_eq!(status as u32, wire_status);
        let wire = encode_command_result(4, 12, 6, status, payload)?;
        let envelope = decode_wire_envelope(&wire)?;
        assert_eq!(envelope.validate(Direction::ServerToClient, true), Ok(()));
        assert_eq!(envelope.server_sequence(), 12);
        assert_eq!(
            decode_command_result(envelope.payload())?,
            CommandResultView {
                command_id: 6,
                status,
                error_code: 0,
                payload,
            }
        );
    }
    Ok(())
}

#[test]
fn command_result_decoder_fails_closed_on_unusable_results() {
    // An unregistered status (5), unspecified or absent status, an absent or zero command
    // id, a duplicated singular field, a wrong-wire-type payload and an error code above
    // u32 all fail closed.
    for payload in [
        &[0x08, 0x01, 0x10, 0x05][..],
        &[0x08, 0x01, 0x10, 0x00],
        &[0x08, 0x01],
        &[0x10, 0x01],
        &[0x08, 0x00, 0x10, 0x01],
        &[0x08, 0x01, 0x08, 0x02, 0x10, 0x01],
        &[0x08, 0x01, 0x10, 0x01, 0x28, 0x01],
        &[0x08, 0x01, 0x10, 0x01, 0x18, 0xff, 0xff, 0xff, 0xff, 0x1f],
    ] {
        assert_eq!(
            decode_command_result(payload),
            Err(FoundationProtocolError::MalformedEnvelope),
            "{payload:?}"
        );
    }
    // Optional error code and unknown fields decode; a bad authoritative_revision list is
    // refused by the shared ingress validation.
    assert_eq!(
        decode_command_result(&[0x08, 0x01, 0x10, 0x02, 0x18, 0x07, 0x78, 0x01]),
        Ok(CommandResultView {
            command_id: 1,
            status: CommandStatus::Rejected,
            error_code: 7,
            payload: &[],
        })
    );
    assert_eq!(
        decode_command_result(&[0x08, 0x01, 0x10, 0x01, 0x22, 0x02, 0x10, 0x01]),
        Err(FoundationProtocolError::StateRevisionMismatch)
    );
}
