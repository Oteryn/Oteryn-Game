#[test]
fn achievement_notices_reach_typed_client_events_only_when_selected() -> Result<(), BoxError> {
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
        crate::achievement_notices::validate_join(&[8], &[]),
        Err(SessionError::MissingDomain(13))
    ));
    assert!(crate::achievement_notices::validate_join(&[], &[]).is_ok());
    let snapshot = GatedSnapshot {
        domain_id: 13,
        snapshot_type: 1,
        revision: 0,
        payload: encode_achievement_notices_snapshot(AchievementWatermark::default()),
    };
    let snapshots = std::slice::from_ref(&snapshot);
    assert!(crate::achievement_notices::validate_join(&[8], snapshots).is_ok());
    assert!(matches!(
        crate::achievement_notices::validate_join(
            &[8],
            &[GatedSnapshot {
                snapshot_type: 2,
                ..snapshot
            }]
        ),
        Err(SessionError::Protocol(_))
    ));
}
