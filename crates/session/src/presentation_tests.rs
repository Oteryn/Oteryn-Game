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

#[test]
fn ambient_b24_negotiates_without_enabling_legacy_presentation() -> Result<(), BoxError> {
    use oteryn_protocol_oteryn::presentation::*;
    block_on(async {
        for selected in [false, true] {
            let (client, mut server) = tokio::io::duplex(64 * 1024);
            let peer = tokio::spawn(async move {
                let raw = if selected {
                    vec![(101, 0, 1, Vec::new())]
                } else {
                    vec![]
                };
                join_peer_with(
                    &mut server,
                    &[6, 288],
                    if selected { &[6, 288] } else { &[6] },
                    &[],
                    entity_snapshot(vec![]),
                    &raw,
                )
                .await?;
                read_frame(&mut server).await?;
                let events = vec![PresentationEvent {
                    decision_ordinal: 1,
                    emission_ordinal: 1,
                    kind: PresentationEventKind::AmbientSound { cue_id: 25 },
                }];
                let payload = encode_presentation_batch(&events).map_err(|e| format!("{e:?}"))?;
                write_frame(
                    &mut server,
                    &encode_state_delta(1, 41, 101, 0, 1, 1, &payload)?,
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
            let mut session = Session::admit(client, admission_with(&[6, 288])?).await?;
            let result = session.step(StepDirection::North).await;
            peer.await??;
            if selected {
                assert_eq!(result?.disposition, StepDisposition::Blocked);
                let events = session.take_events();
                assert!(
                    matches!(events.as_slice(), [SessionEvent::Presentation(delta)] if delta.new_revision == 1 && delta.value.len() == 1)
                );
                // Repeated/stale revisions cannot replay an audible event.
                let stale = encode_state_delta(1, 43, 101, 0, 1, 1, &[])?;
                assert!(
                    session
                        .apply_state_delta(43, decode_wire_envelope(&stale)?.payload())
                        .is_err()
                );
                let malformed = encode_state_delta(1, 43, 101, 1, 2, 1, &[255])?;
                assert!(
                    session
                        .apply_state_delta(43, decode_wire_envelope(&malformed)?.payload())
                        .is_err()
                );
                assert_eq!(
                    session
                        .gated_revisions
                        .iter()
                        .find(|(id, _)| *id == 101)
                        .map(|(_, rev)| *rev),
                    Some(1)
                );
                assert!(session.take_events().is_empty());
            } else {
                assert!(matches!(
                    result,
                    Err(SessionError::UnselectedDomain { domain_id: 101 })
                ));
            }
        }
        Ok(())
    })?
}

#[test]
fn nx5_item_effects_negotiate_without_casting_and_reject_other_payload_kinds()
-> Result<(), BoxError> {
    use oteryn_protocol_oteryn::presentation::*;
    block_on(async {
        for selected in [false, true] {
            let (client, mut server) = tokio::io::duplex(64 * 1024);
            let peer = tokio::spawn(async move {
                let raw = if selected {
                    vec![(STATE_DOMAIN_ITEM_EFFECTS, 0, 1, Vec::new())]
                } else {
                    vec![]
                };
                join_peer_with(
                    &mut server,
                    &[6, 350],
                    if selected { &[6, 350] } else { &[6] },
                    &[],
                    entity_snapshot(vec![]),
                    &raw,
                )
                .await?;
                read_frame(&mut server).await?;
                let events = [PresentationEvent {
                    decision_ordinal: 1,
                    emission_ordinal: 1,
                    kind: PresentationEventKind::MagicEffect {
                        position: at(0, 0),
                        effect_id: 29,
                        source: PresentationSource::Own,
                    },
                }];
                let payload = encode_item_effect_batch(&events).map_err(|e| format!("{e:?}"))?;
                write_frame(
                    &mut server,
                    &encode_state_delta(1, 41, STATE_DOMAIN_ITEM_EFFECTS, 0, 1, 1, &payload)?,
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
            let mut session = Session::admit(client, admission_with(&[6, 350])?).await?;
            assert!(!session.is_selected(CAPABILITY_PRESENTATION_V1));
            let result = session.step(StepDirection::North).await;
            peer.await??;
            if selected {
                assert_eq!(result?.disposition, StepDisposition::Blocked);
                let events = session.take_events();
                assert!(
                    matches!(events.as_slice(), [SessionEvent::ItemEffects(delta)] if delta.value.len()==1 && delta.new_revision==1)
                );
                assert_eq!(events[0].domain_id(), STATE_DOMAIN_ITEM_EFFECTS);
                let sound = PresentationEvent {
                    decision_ordinal: 1,
                    emission_ordinal: 1,
                    kind: PresentationEventKind::Sound {
                        position: at(0, 0),
                        cue_id: 1,
                        secondary_cue_id: 0,
                        source: PresentationSource::Own,
                    },
                };
                assert!(encode_item_effect_batch(&[sound]).is_err());
                let payload = encode_presentation_batch(&[sound]).map_err(|e| format!("{e:?}"))?;
                let frame =
                    encode_state_delta(1, 43, STATE_DOMAIN_ITEM_EFFECTS, 1, 2, 1, &payload)?;
                assert!(
                    session
                        .apply_state_delta(43, decode_wire_envelope(&frame)?.payload())
                        .is_err()
                );
                assert_eq!(
                    session
                        .gated_revisions
                        .iter()
                        .find(|(id, _)| *id == STATE_DOMAIN_ITEM_EFFECTS)
                        .map(|(_, revision)| *revision),
                    Some(1)
                );
                assert!(session.take_events().is_empty());
            } else {
                assert!(matches!(
                    result,
                    Err(SessionError::UnselectedDomain {
                        domain_id: STATE_DOMAIN_ITEM_EFFECTS
                    })
                ));
            }
        }
        Ok(())
    })?
}

#[test]
fn srx3_native_map_epoch_basis_requires_negotiated_capability() -> Result<(), String> {
    use oteryn_protocol_oteryn::presentation::*;
    let snapshots = vec![
        GatedSnapshot {
            domain_id: 19,
            revision: 0,
            snapshot_type: 2,
            payload: Vec::new(),
        },
        GatedSnapshot {
            domain_id: 20,
            revision: 0,
            snapshot_type: 1,
            payload: encode_actor_cooldowns(&ActorCooldowns {
                server_now_ms: 1,
                rtt_ms: None,
                entries: vec![],
            })
            .map_err(|error| format!("{error:?}"))?,
        },
    ];
    assert!(
        crate::presentation::validate_join(&[6, 23, 30, 385], &snapshots).is_ok(),
        "qualified native snapshot marker must install its explicit epoch basis"
    );
    assert!(crate::presentation::validate_join(&[6, 23, 30], &snapshots).is_err());
    assert!(crate::presentation::validate_join(&[6, 23, 385], &snapshots).is_err());
    assert!(crate::presentation::validate_join(&[6, 30, 385], &snapshots).is_err());
    let mut malformed = snapshots.clone();
    malformed[0].payload.push(0);
    assert!(crate::presentation::validate_join(&[6, 23, 30, 385], &malformed).is_err());
    let mut legacy = snapshots;
    legacy[0].snapshot_type = 1;
    assert!(crate::presentation::validate_join(&[6, 23], &legacy).is_ok());
    Ok(())
}
