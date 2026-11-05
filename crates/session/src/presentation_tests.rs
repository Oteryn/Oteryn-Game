#[test]
fn presentation_audio_negotiates_and_validates_revisions_before_delivery() -> Result<(), BoxError> {
    use oteryn_protocol_oteryn::presentation::*;
    block_on(async {
        for selected in [false, true] {
            let (client, mut server) = tokio::io::duplex(64 * 1024);
            let peer = tokio::spawn(async move {
                let raw = if selected {
                    vec![
                        (19, 0, 1, encode_presentation_snapshot()),
                        (
                            20,
                            0,
                            1,
                            encode_actor_cooldowns(&ActorCooldowns {
                                server_now_ms: 1,
                                rtt_ms: None,
                                entries: vec![],
                            })
                            .map_err(|e| format!("{e:?}"))?,
                        ),
                    ]
                } else {
                    vec![]
                };
                join_peer_with(
                    &mut server,
                    &[6, 23],
                    if selected { &[6, 23] } else { &[6] },
                    &[],
                    entity_snapshot(vec![]),
                    &raw,
                )
                .await?;
                read_frame(&mut server).await?;
                let events = vec![PresentationEvent {
                    decision_ordinal: 1,
                    emission_ordinal: 1,
                    kind: PresentationEventKind::Sound {
                        position: at(0, 0),
                        cue_id: 41,
                        secondary_cue_id: 42,
                        source: PresentationSource::Own,
                    },
                }];
                let payload = encode_presentation_batch(&events).map_err(|e| format!("{e:?}"))?;
                write_frame(
                    &mut server,
                    &encode_state_delta(1, 41, 19, 0, 1, 1, &payload)?,
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
            let mut session = Session::admit(client, admission_with(&[6, 23])?).await?;
            let result = session.step(StepDirection::North).await;
            peer.await??;
            if selected {
                assert_eq!(result?.disposition, StepDisposition::Blocked);
                let events = session.take_events();
                assert!(
                    matches!(events.as_slice(), [SessionEvent::Presentation(delta)] if delta.new_revision == 1 && delta.value.len() == 1)
                );
                // Repeated/stale revisions cannot replay an audible event.
                let stale = encode_state_delta(1, 43, 19, 0, 1, 1, &[])?;
                assert!(
                    session
                        .apply_state_delta(43, decode_wire_envelope(&stale)?.payload())
                        .is_err()
                );
                let malformed = encode_state_delta(1, 43, 19, 1, 2, 1, &[255])?;
                assert!(
                    session
                        .apply_state_delta(43, decode_wire_envelope(&malformed)?.payload())
                        .is_err()
                );
                assert_eq!(
                    session
                        .gated_revisions
                        .iter()
                        .find(|(id, _)| *id == 19)
                        .map(|(_, rev)| *rev),
                    Some(1)
                );
                assert!(session.take_events().is_empty());
            } else {
                assert!(matches!(
                    result,
                    Err(SessionError::UnselectedDomain { domain_id: 19 })
                ));
            }
        }
        Ok(())
    })?
}
