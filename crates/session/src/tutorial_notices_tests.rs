// Reuse the existing join peer and command transport instead of a second session harness.
#[test]
fn tutorial_notices_reach_typed_client_events_only_when_selected() -> Result<(), BoxError> {
    use oteryn_protocol_oteryn::tutorial_notices::{TutorialNotice, encode_tutorial_notice};
    block_on(async {
        for selected in [false, true] {
            let (client, mut server) = tokio::io::duplex(64 * 1024);
            let peer = tokio::spawn(async move {
                let raw = if selected {
                    vec![(23, 0, 1, vec![8, 1])]
                } else {
                    Vec::new()
                };
                join_peer_with(
                    &mut server,
                    &[31],
                    if selected { &[31] } else { &[] },
                    &[],
                    (SNAPSHOT_TYPE_WORLD_SPATIAL_V1, spatial(0)),
                    &raw,
                )
                .await?;
                read_frame(&mut server).await?;
                let notice = encode_tutorial_notice(&TutorialNotice::Hint(2))
                    .map_err(|error| format!("{error:?}"))?;
                write_frame(
                    &mut server,
                    &encode_state_delta(1, 41, 23, 0, 1, 1, &notice)?,
                )
                .await?;
                write_frame(
                    &mut server,
                    &encode_command_result(
                        1,
                        42,
                        7,
                        CommandStatus::Accepted,
                        &encode_step_result(StepDisposition::Blocked),
                    )?,
                )
                .await?;
                Ok::<(), BoxError>(())
            });
            let mut session = Session::admit(client, admission_with(&[31])?).await?;
            let step = session.step(StepDirection::North).await;
            peer.await??;
            if selected {
                assert_eq!(step?.disposition, StepDisposition::Blocked);
                assert!(
                    matches!(session.take_events().as_slice(), [SessionEvent::TutorialNotice(delta)] if delta.base_revision == 0 && delta.new_revision == 1 && delta.value == TutorialNotice::Hint(2))
                );
            } else {
                assert!(matches!(
                    step,
                    Err(SessionError::UnselectedDomain { domain_id: 23 })
                ));
            }
        }
        Ok(())
    })?
}
