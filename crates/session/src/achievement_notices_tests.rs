// Reuse the existing join peer and command transport instead of a second session harness.
#[test]
fn achievement_notices_reach_typed_client_events_only_when_selected() -> Result<(), BoxError> {
    use oteryn_protocol_oteryn::achievement_notices::{
        AchievementEarned, AchievementWatermark, encode_achievement_earned,
    };
    achievement_block_on(async {
        for selected in [false, true] {
            let (client, mut server) = tokio::io::duplex(64 * 1024);
            let peer = tokio::spawn(async move {
                let raw = if selected {
                    vec![(13, 0, 1, vec![])]
                } else {
                    Vec::new()
                };
                join_peer_with(
                    &mut server,
                    &[8],
                    if selected { &[8] } else { &[] },
                    &[],
                    (SNAPSHOT_TYPE_WORLD_SPATIAL_V1, spatial(0)),
                    &raw,
                )
                .await?;
                read_frame(&mut server).await?;
                let notice = encode_achievement_earned(&AchievementEarned {
                    key: "oteryn:achievement/first".into(),
                    name: "First Steps".into(),
                    watermark: AchievementWatermark {
                        fact_count: 1,
                        total_points: 5,
                    },
                })
                .map_err(|error| format!("{error:?}"))?;
                write_frame(
                    &mut server,
                    &encode_state_delta(1, 41, 13, 0, 1, 1, &notice)?,
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
            let mut session = Session::admit(client, admission_with(&[8])?).await?;
            let step = session.step(StepDirection::North).await;
            peer.await??;
            if selected {
                assert_eq!(step?.disposition, StepDisposition::Blocked);
                assert!(
                    matches!(session.take_events().as_slice(), [SessionEvent::AchievementEarned(delta)] if delta.base_revision == 0 && delta.new_revision == 1 && delta.value.name == "First Steps" && delta.value.watermark.total_points == 5)
                );
            } else {
                assert!(matches!(
                    step,
                    Err(SessionError::UnselectedDomain { domain_id: 13 })
                ));
            }
        }
        Ok(())
    })?
}

#[test]
fn achievement_notices_validate_snapshots_and_reject_bad_deltas_without_advancing()
-> Result<(), BoxError> {
    use achievement_notices::*;
    let selected = [CAPABILITY_ACHIEVEMENT_NOTICES_V1];
    assert!(achievement_notices_transport::validate_join(&selected, &[]).is_err());
    for (snapshot_type, payload) in [(2, vec![]), (1, vec![0])] {
        assert!(
            achievement_notices_transport::validate_join(
                &selected,
                &[GatedSnapshot {
                    domain_id: 13,
                    revision: 0,
                    snapshot_type,
                    payload,
                }]
            )
            .is_err()
        );
    }
    achievement_block_on(async {
        let (client, mut server) = tokio::io::duplex(64 * 1024);
        let peer = tokio::spawn(async move {
            join_peer_with(
                &mut server,
                &[8],
                &[8],
                &[],
                (SNAPSHOT_TYPE_WORLD_SPATIAL_V1, spatial(0)),
                &[(13, 0, 1, vec![])],
            )
            .await
        });
        let mut session = Session::admit(client, admission_with(&[8])?).await?;
        peer.await??;
        let value = AchievementEarned {
            key: "oteryn:achievement/first".into(),
            name: "First Steps".into(),
            watermark: AchievementWatermark {
                fact_count: 1,
                total_points: 5,
            },
        };
        let valid = encode_achievement_earned(&value).map_err(|e| format!("{e:?}"))?;
        for (base, new, kind, payload) in [
            (0, 1, 1, vec![0]),
            (1, 2, 1, valid.clone()),
            (0, 1, 2, valid.clone()),
        ] {
            let frame = encode_state_delta(1, 41, 13, base, new, kind, &payload)?;
            assert!(
                session
                    .apply_state_delta(41, decode_wire_envelope(&frame)?.payload())
                    .is_err()
            );
            assert!(session.gated_revisions.contains(&(13, 0)));
        }
        let frame = encode_state_delta(1, 41, 13, 0, 1, 1, &valid)?;
        assert!(
            matches!(session.apply_state_delta(41, decode_wire_envelope(&frame)?.payload())?, SessionEvent::AchievementEarned(delta) if delta.value == value)
        );
        assert!(session.gated_revisions.contains(&(13, 1)));
        Ok(())
    })?
}

fn achievement_block_on<F>(future: F) -> Result<F::Output, BoxError>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || block_on(future))?
        .join()
        .map_err(|_| "achievement session test thread panicked")?
}
#[test]
fn achievement_notices_preserve_typed_payload_when_selected() -> Result<(), BoxError> {
    use oteryn_protocol_oteryn::achievement_notices::{
        AchievementEarned, AchievementWatermark, encode_achievement_earned,
        encode_achievement_notices_snapshot,
    };
    block_on(async {
        for selected in [false, true] {
            let (client, mut server) = tokio::io::duplex(64 * 1024);
            let earned = AchievementEarned {
                key: "oteryn:achievement/allow_cookies".into(),
                name: "Allow Cookies?!".into(),
                watermark: AchievementWatermark {
                    fact_count: 1,
                    total_points: 1,
                },
            };
            let expected = earned.clone();
            let peer = tokio::spawn(async move {
                let raw = if selected {
                    vec![(
                        13,
                        0,
                        1,
                        encode_achievement_notices_snapshot(AchievementWatermark::default()),
                    )]
                } else {
                    Vec::new()
                };
                join_peer_with(
                    &mut server,
                    &[8],
                    if selected { &[8] } else { &[] },
                    &[],
                    (SNAPSHOT_TYPE_WORLD_SPATIAL_V1, spatial(0)),
                    &raw,
                )
                .await?;
                read_frame(&mut server).await?;
                write_frame(
                    &mut server,
                    &encode_state_delta(
                        1,
                        41,
                        13,
                        0,
                        1,
                        1,
                        &encode_achievement_earned(&earned)
                            .map_err(|error| format!("{error:?}"))?,
                    )?,
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
            let mut session = Session::admit(client, admission_with(&[8])?).await?;
            let step = session.step(StepDirection::North).await;
            peer.await??;
            if selected {
                assert_eq!(step?.disposition, StepDisposition::Blocked);
                assert!(
                    matches!(session.take_events().as_slice(), [SessionEvent::AchievementEarned(delta)] if delta.base_revision == 0 && delta.new_revision == 1 && delta.value == expected)
                );
            } else {
                assert!(matches!(
                    step,
                    Err(SessionError::UnselectedDomain { domain_id: 13 })
                ));
            }
        }
        Ok(())
    })?
}

#[test]
fn achievement_notices_join_requires_the_domain_13_snapshot_when_selected() {
    use oteryn_protocol_oteryn::achievement_notices::{
        AchievementWatermark, encode_achievement_notices_snapshot,
    };
    assert!(matches!(
        crate::achievement_notices_transport::validate_join(&[8], &[]),
        Err(SessionError::MissingDomain(13))
    ));
    assert!(crate::achievement_notices_transport::validate_join(&[], &[]).is_ok());
    let snapshot = GatedSnapshot {
        domain_id: 13,
        snapshot_type: 1,
        revision: 0,
        payload: encode_achievement_notices_snapshot(AchievementWatermark::default()),
    };
    let snapshots = std::slice::from_ref(&snapshot);
    assert!(crate::achievement_notices_transport::validate_join(&[8], snapshots).is_ok());
    assert!(matches!(
        crate::achievement_notices_transport::validate_join(
            &[8],
            &[GatedSnapshot {
                snapshot_type: 2,
                ..snapshot
            }]
        ),
        Err(SessionError::Protocol(_))
    ));
}
