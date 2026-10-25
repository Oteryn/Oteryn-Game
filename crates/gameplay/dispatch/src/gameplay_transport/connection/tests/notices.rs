const NOTICE_ACCOUNT: [u8; 16] = [0x7a; 16];

fn earned(fact_count: u32, total_points: u32) -> AchievementEarned {
    AchievementEarned {
        key: "oteryn:achievement/allow_cookies".to_owned(),
        name: "Allow Cookies?!".to_owned(),
        watermark: AchievementWatermark {
            fact_count,
            total_points,
        },
    }
}

/// A chest `USE` fixture whose every call commits with the notice `earned`.
fn notice_authority(
    watermark: Option<AchievementWatermark>,
    earned: EarnedNotice,
) -> UseAuthority {
    UseAuthority {
        uses: RefCell::new(Vec::new()),
        commands: RefCell::new(Vec::new()),
        overlay: None,
        watermark,
        outcome: UseOutcome {
            disposition: UseDisposition::Committed,
            committed: None,
            earned,
            gate_message_id: None, refusal_detail: None,
        },
    }
}

/// A positioned session with a controller and `continuity`.
fn controlled(continuity: SessionContinuity) -> Result<AdmittedSession, Box<dyn Error>> {
    Ok(AdmittedSession {
        controller: Some(ControllerBinding {
            transport: AuthenticatedTransportRefV1::decode(&[0x5a; 16])?,
            account_id: NOTICE_ACCOUNT,
        }),
        continuity,
        ..positioned()?
    })
}

/// A session that selected capability 8, at notice `revision`.
fn with_notices(revision: u64) -> SessionContinuity {
    SessionContinuity {
        achievement_notice_revision: Some(revision),
        selected_capabilities: SelectedCapabilities::select(
            &[OfferedCapability {
                id: CAPABILITY_ACHIEVEMENT_NOTICES_V1,
                requires: &[],
            }],
            &[CAPABILITY_ACHIEVEMENT_NOTICES_V1],
        )
        .expect("selection"),
        ..SessionContinuity::FRESH
    }
}

fn notice_snapshot(
    generation: u64,
    sequence: u64,
    revision: u64,
    watermark: AchievementWatermark,
) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
    Ok(encode_single_chunk_snapshot(
        generation,
        1,
        sequence,
        &[
            DomainSnapshot {
                domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                revision: 1,
                snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                payload: &encode_world_spatial(&at(0)),
            },
            empty_overlay_snapshot(),
            DomainSnapshot {
                domain_id: STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES,
                revision,
                snapshot_type: SNAPSHOT_TYPE_ACHIEVEMENT_NOTICES_V1,
                payload: &encode_achievement_notices_snapshot(watermark),
            },
        ],
    )?
    .into())
}

fn committed_use(generation: u64, sequence: u64, id: u64) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok(encode_command_result(
        generation,
        sequence,
        id,
        CommandStatus::Accepted,
        &encode_use_result(UseDisposition::Committed).unwrap_or_default(),
    )?)
}

fn notice_delta(
    generation: u64,
    sequence: u64,
    from: u64,
    notice: &AchievementEarned,
) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok(encode_state_delta(
        generation,
        sequence,
        STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES,
        from,
        from + 1,
        DELTA_TYPE_ACHIEVEMENT_EARNED_V1,
        &encode_achievement_earned(notice).map_err(|error| format!("{error:?}"))?,
    )?)
}

fn ended_continuity(end: ConnectionEnd) -> Result<SessionContinuity, Box<dyn Error>> {
    match end {
        ConnectionEnd::AdmittedThenDisconnected(ended) => Ok(ended.continuity),
        other => Err(format!("unexpected end {other:?}").into()),
    }
}

/// ACHIEVEMENT-0 §5: with capability 8 the join snapshot carries the watermark at revision 0
/// and each committed `Granted` sends exactly one delta after its result, from the current
/// revision to the next. Without capability 8 nothing of domain 13 is sent.
#[test]
fn each_granted_use_sends_one_notice_only_with_capability_8() -> Result<(), Box<dyn Error>> {
    run(async {
        let use_type = u64::from(COMMAND_TYPE_USE_INTENT);
        let joined = AchievementWatermark {
            fact_count: 1,
            total_points: 3,
        };
        let notice = earned(2, 13);
        let uses = [
            use_command(1, 1, use_type, b"oteryn:placement/entry-chest", 0),
            use_command(1, 2, use_type, b"oteryn:placement/entry-chest", 0),
        ];
        let authority = notice_authority(Some(joined), EarnedNotice::Earned(notice.clone()));
        let (end, frames) =
            drive_session(&authority, controlled(with_notices(0))?, &uses).await?;
        let mut expected = notice_snapshot(1, 0, 0, joined)?;
        expected.extend([
            committed_use(1, 1, 1)?,
            notice_delta(1, 2, 0, &notice)?,
            committed_use(1, 3, 2)?,
            notice_delta(1, 4, 1, &notice)?,
        ]);
        assert_eq!(frames, expected);
        let ended = ended_continuity(end)?;
        assert_eq!(ended.achievement_notice_revision, Some(2));
        assert_eq!(ended.server_sequence, 4);

        // Without capability 8 (production today): the same grants send no domain 13 frame.
        let (end, frames) =
            drive_session(&authority, controlled(SessionContinuity::FRESH)?, &uses).await?;
        let mut expected = baseline();
        expected.extend([committed_use(1, 1, 1)?, committed_use(1, 2, 2)?]);
        assert_eq!(frames, expected);
        assert_eq!(ended_continuity(end)?.achievement_notice_revision, None);

        // A commit without a `Granted` grant (none, `AlreadyHeld` or `Retired`): no delta.
        let authority = notice_authority(Some(joined), EarnedNotice::NoneEarned);
        let (end, frames) =
            drive_session(&authority, controlled(with_notices(0))?, &uses[..1]).await?;
        let mut expected = notice_snapshot(1, 0, 0, joined)?;
        expected.push(committed_use(1, 1, 1)?);
        assert_eq!(frames, expected);
        assert_eq!(ended_continuity(end)?.achievement_notice_revision, Some(0));
        Ok(())
    })
}

/// ACHIEVEMENT-0 §5: a reconnect within the GameSession sends only the snapshot, at the
/// cumulative revision, and never replays a delta; the next delta continues from it. A
/// selected domain whose watermark cannot be read fails closed before any frame.
#[test]
fn a_reconnect_sends_only_the_snapshot_and_the_revision_stays_monotonic()
-> Result<(), Box<dyn Error>> {
    run(async {
        let use_type = u64::from(COMMAND_TYPE_USE_INTENT);
        let notice = earned(2, 13);
        let watermark = notice.watermark;
        let authority = notice_authority(Some(watermark), EarnedNotice::Earned(notice.clone()));
        let (end, _) = drive_session(
            &authority,
            controlled(with_notices(0))?,
            &[use_command(
                1,
                1,
                use_type,
                b"oteryn:placement/entry-chest",
                0,
            )],
        )
        .await?;
        let lost = ended_continuity(end)?;
        assert_eq!(lost.achievement_notice_revision, Some(1));

        // The resumed connection, as `resume.rs` builds it: the lost continuity on the next
        // generation.
        let resumed = SessionContinuity {
            connection_generation: 2,
            ..lost
        };
        let (end, frames) = drive_session(&authority, controlled(resumed)?, &[]).await?;
        assert_eq!(frames, notice_snapshot(2, 2, 1, watermark)?);
        assert_eq!(ended_continuity(end)?.achievement_notice_revision, Some(1));

        let (end, frames) = drive_session(
            &authority,
            controlled(resumed)?,
            &[use_command(
                2,
                2,
                use_type,
                b"oteryn:placement/entry-chest",
                0,
            )],
        )
        .await?;
        let mut expected = notice_snapshot(2, 2, 1, watermark)?;
        expected.extend([committed_use(2, 3, 2)?, notice_delta(2, 4, 1, &notice)?]);
        assert_eq!(frames, expected);
        assert_eq!(ended_continuity(end)?.achievement_notice_revision, Some(2));

        // Unreadable watermark, or no controller account: no snapshot, nothing sent.
        let unreadable = notice_authority(None, EarnedNotice::NoneEarned);
        let (end, frames) =
            drive_session(&unreadable, controlled(with_notices(0))?, &[]).await?;
        assert!(frames.is_empty());
        assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)));
        let uncontrolled = AdmittedSession {
            continuity: with_notices(0),
            ..positioned()?
        };
        let (_, frames) = drive_session(&authority, uncontrolled, &[]).await?;
        assert!(frames.is_empty());
        Ok(())
    })
}

/// ACH-NOTIFY-2: an `Unknown` notice with capability 8 ends the connection right after the
/// command result, with no delta and the revision unchanged; the resumed connection's
/// snapshot carries the watermark with the grant. Without capability 8 it changes nothing.
#[test]
fn an_unknown_notice_disconnects_only_with_capability_8_and_resume_restores_the_watermark()
-> Result<(), Box<dyn Error>> {
    run(async {
        let use_type = u64::from(COMMAND_TYPE_USE_INTENT);
        let before = AchievementWatermark {
            fact_count: 1,
            total_points: 3,
        };
        let uses = [
            use_command(1, 1, use_type, b"oteryn:placement/entry-chest", 0),
            use_command(1, 2, use_type, b"oteryn:placement/entry-chest", 0),
        ];
        let authority = notice_authority(Some(before), EarnedNotice::Unknown);
        let (end, frames) =
            drive_session(&authority, controlled(with_notices(0))?, &uses).await?;
        let mut expected = notice_snapshot(1, 0, 0, before)?;
        expected.push(committed_use(1, 1, 1)?);
        assert_eq!(frames, expected);
        assert_eq!(
            authority.uses.borrow().len(),
            1,
            "nothing after the disconnect"
        );
        let lost = ended_continuity(end)?;
        assert_eq!(lost.achievement_notice_revision, Some(0));
        assert_eq!(lost.server_sequence, 1);

        // The grant is durable: the resumed connection's snapshot carries it, at the
        // unchanged revision.
        let after = AchievementWatermark {
            fact_count: 2,
            total_points: 8,
        };
        let resumed = SessionContinuity {
            connection_generation: 2,
            ..lost
        };
        let authority = notice_authority(Some(after), EarnedNotice::NoneEarned);
        let (_, frames) = drive_session(&authority, controlled(resumed)?, &[]).await?;
        assert_eq!(frames, notice_snapshot(2, 1, 0, after)?);

        // Without capability 8 the same outcome sends nothing and keeps the connection.
        let authority = notice_authority(Some(before), EarnedNotice::Unknown);
        let (end, frames) =
            drive_session(&authority, controlled(SessionContinuity::FRESH)?, &uses).await?;
        let mut expected = baseline();
        expected.extend([committed_use(1, 1, 1)?, committed_use(1, 2, 2)?]);
        assert_eq!(frames, expected);
        assert_eq!(authority.uses.borrow().len(), 2);
        assert_eq!(ended_continuity(end)?.server_sequence, 2);
        Ok(())
    })
}
