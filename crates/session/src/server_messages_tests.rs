#[test]
fn raids_global_server_messages_reach_typed_events_in_order_only_when_selected() -> Result<(), BoxError> {
    use oteryn_protocol_oteryn::zones_nologout::{ServerMessage, encode_server_message};
    assert!(CLIENT_SUPPORTED_CAPABILITIES.contains(&92));
    assert!(!CLIENT_SUPPORTED_CAPABILITIES.contains(&149));
    block_on(async {
        for selected in [false, true] {
            let (client, mut server) = tokio::io::duplex(64 * 1024);
            let peer = tokio::spawn(async move {
                let raw = if selected { vec![(57, 0, 1, vec![8, 1])] } else { Vec::new() };
                join_peer_with(&mut server, &[92], if selected { &[92] } else { &[] },
                    &[], (SNAPSHOT_TYPE_WORLD_SPATIAL_V1, spatial(0)), &raw).await?;
                read_frame(&mut server).await?;
                for (index, message) in [
                    ServerMessage::StatusText("Status".into()),
                    ServerMessage::Warning("A raid approaches.".into()),
                    ServerMessage::Advance("The creatures have arrived.".into()),
                ].into_iter().enumerate() {
                    let index = u64::try_from(index)?;
                    let payload = encode_server_message(&message).map_err(|error| format!("{error:?}"))?;
                    write_frame(&mut server, &encode_state_delta(1, 41 + index, 57, index, index + 1, 1, &payload)?).await?;
                }
                write_frame(&mut server, &encode_command_result(1, 44, 7, CommandStatus::Accepted,
                    &encode_step_result(StepDisposition::Blocked))?).await?;
                Ok::<(), BoxError>(())
            });
            let mut session = Session::admit(client, admission_with(&[92])?).await?;
            let step = session.step(StepDirection::North).await;
            peer.await??;
            if selected {
                assert_eq!(step?.disposition, StepDisposition::Blocked);
                let events = session.take_events();
                assert_eq!(events.len(), 3);
                assert!(matches!(&events[0], SessionEvent::ServerMessage(delta)
                    if delta.base_revision == 0 && delta.new_revision == 1
                    && delta.value == ServerMessage::StatusText("Status".into())));
                assert!(matches!(&events[1], SessionEvent::ServerMessage(delta)
                    if delta.value == ServerMessage::Warning("A raid approaches.".into())));
                assert!(matches!(&events[2], SessionEvent::ServerMessage(delta)
                    if delta.value == ServerMessage::Advance("The creatures have arrived.".into())));
                assert!(events.iter().all(|event| event.domain_id() == 57));
            } else {
                assert!(matches!(step, Err(SessionError::UnselectedDomain { domain_id: 57 })));
            }
        }
        Ok(())
    })?
}

#[test]
fn raids_global_server_message_join_accepts_only_empty_or_status_snapshot() -> Result<(), BoxError> {
    use oteryn_protocol_oteryn::zones_nologout::{ServerMessage, encode_server_message};
    let row = |snapshot_type, payload| GatedSnapshot { domain_id: 57, revision: 0, snapshot_type, payload };
    assert!(matches!(server_messages::validate_join(&[92], &[], 40), Err(SessionError::MissingDomain(57))));
    assert!(server_messages::validate_join(&[92], &[row(2, vec![8, 1])], 40).is_err());
    assert!(server_messages::validate_join(&[92], &[row(1, vec![8, 1])], 40)?.is_empty());
    let status = encode_server_message(&ServerMessage::StatusText("Welcome".into())).map_err(|error| format!("{error:?}"))?;
    let events = server_messages::validate_join(&[92], &[row(1, status)], 40)?;
    assert!(matches!(events.as_slice(), [SessionEvent::ServerMessage(delta)]
        if delta.server_sequence == 40 && delta.value == ServerMessage::StatusText("Welcome".into())));
    let warning = encode_server_message(&ServerMessage::Warning("Old raid".into())).map_err(|error| format!("{error:?}"))?;
    assert!(server_messages::validate_join(&[92], &[row(1, warning)], 40).is_err());
    Ok(())
}

#[test]
fn raids_global_server_message_decode_and_revision_failures_do_not_advance() -> Result<(), BoxError> {
    use oteryn_protocol_oteryn::zones_nologout::{ServerMessage, encode_server_message};
    block_on(async {
        let (client, mut server) = tokio::io::duplex(64 * 1024);
        let peer = tokio::spawn(async move {
            join_peer_with(&mut server, &[92], &[92], &[],
                (SNAPSHOT_TYPE_WORLD_SPATIAL_V1, spatial(0)), &[(57, 0, 1, vec![8, 1])]).await
        });
        let mut session = Session::admit(client, admission_with(&[92])?).await?;
        peer.await??;
        let valid = encode_server_message(&ServerMessage::Advance("Arrived".into())).map_err(|error| format!("{error:?}"))?;
        let payload = |base, new, kind, bytes: &[u8]| -> Result<Vec<u8>, BoxError> {
            let frame = encode_state_delta(1, 41, 57, base, new, kind, bytes)?;
            Ok(decode_wire_envelope(&frame)?.payload().to_vec())
        };
        assert!(session.apply_state_delta(41, &payload(0, 1, 1, &[8, 1, 26, 0])?).is_err());
        assert!(session.apply_state_delta(41, &payload(0, 1, 2, &valid)?).is_err());
        let event = session.apply_state_delta(41, &payload(0, 1, 1, &valid)?)?;
        assert!(matches!(event, SessionEvent::ServerMessage(delta) if delta.new_revision == 1));
        assert!(session.apply_state_delta(42, &payload(0, 1, 1, &valid)?).is_err());
        assert_eq!(session.gated_revisions.iter().find(|(id, _)| *id == 57).map(|(_, revision)| *revision), Some(1));
        Ok(())
    })?
}
