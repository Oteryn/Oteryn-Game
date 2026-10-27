//! Real Session framing, capability gate, command sequencing and pushed-domain dispatch.
use super::*;
use crate::quest_log::*;

#[test]
fn quest_log_session_command_22_and_domain_16_are_gated() -> Result<(), BoxError> {
    block_on(async {
        for selected in [false, true] {
            let (client, mut server) = tokio::io::duplex(64 * 1024);
            let peer = tokio::spawn(async move {
                let raw = if selected {
                    vec![(16, 0, 1, vec![])]
                } else {
                    vec![]
                };
                join_peer_with(
                    &mut server,
                    &[16],
                    if selected { &[16] } else { &[] },
                    &[],
                    (1, spatial(0)),
                    &raw,
                )
                .await?;
                let command = read_frame(&mut server).await?;
                let command = decode_wire_envelope(&command)?.client_command(1)?;
                if selected {
                    assert_eq!(command.command_type, 22);
                    assert_eq!(command.payload, &[0x0a, 0]);
                    write_frame(
                        &mut server,
                        &encode_command_result(1, 41, 7, CommandStatus::Accepted, &[])?,
                    )
                    .await?;
                    // A domain-16 delta encountered while waiting for the next command result
                    // goes through the production pushed-state dispatcher.
                    read_frame(&mut server).await?;
                }
                write_frame(
                    &mut server,
                    &encode_state_delta(
                        1,
                        if selected { 42 } else { 41 },
                        16,
                        0,
                        1,
                        1,
                        &[0x0a, 0],
                    )?,
                )
                .await?;
                write_frame(
                    &mut server,
                    &encode_command_result(
                        1,
                        if selected { 43 } else { 42 },
                        if selected { 8 } else { 7 },
                        CommandStatus::Accepted,
                        &encode_step_result(StepDisposition::Blocked),
                    )?,
                )
                .await?;
                Ok::<(), BoxError>(())
            });
            let mut session = Session::admit(client, admission_with(&[16])?).await?;
            let result = session.gated_command(16, 22, &[0x0a, 0]).await;
            if selected {
                assert!(result?.payload.is_empty());
            } else {
                assert!(matches!(
                    result,
                    Err(SessionError::CapabilityNotSelected { capability: 16 })
                ));
            }
            let step = session.step(StepDirection::North).await;
            peer.await??;
            if selected {
                assert_eq!(step?.disposition, StepDisposition::Blocked);
                assert!(
                    matches!(session.take_events().as_slice(), [SessionEvent::QuestLog(delta)] if delta.new_revision == 1 && delta.value.view == Some(QuestLogView::List(vec![])))
                );
            } else {
                assert!(matches!(
                    step,
                    Err(SessionError::UnselectedDomain { domain_id: 16 })
                ));
            }
        }
        Ok(())
    })?
}

#[test]
fn quest_log_admission_rejects_missing_malformed_or_wrong_type_snapshot() -> Result<(), BoxError> {
    block_on(async {
        for raw in [
            vec![],
            vec![(16, 0, 1, vec![255])],
            vec![(16, 0, 2, vec![])],
        ] {
            let (client, mut server) = tokio::io::duplex(64 * 1024);
            let peer = tokio::spawn(async move {
                join_peer_with(&mut server, &[16], &[16], &[], (1, spatial(0)), &raw).await
            });
            let result = Session::admit(client, admission_with(&[16])?).await;
            let _ = peer.await?;
            assert!(result.is_err());
        }
        Ok(())
    })?
}
