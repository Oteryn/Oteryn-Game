enum Action {
    Step,
    Use,
}

/// Drives one command against a server that answers it with exactly `reply`, asserts the
/// client fails with `expected`, and that the session is then unusable.
async fn assert_command_fails(
    action: Action,
    reply: Vec<Vec<u8>>,
    expected: impl Fn(&DevClientError) -> bool,
) -> Result<(), BoxError> {
    let (mut session, server) = joined_session(TEST_DEADLINE, move |mut stream| async move {
        let (id, _, _) = read_command(&mut stream).await?;
        assert_eq!(id, FIRST_COMMAND_ID);
        send(&mut stream, &reply).await?;
        wait_for_client_close(&mut stream).await;
        Ok(())
    })
    .await?;
    let result = match action {
        Action::Step => session.step(StepDirection::East).await.map(|_| ()),
        // A use returns at its result, so a bad delta behind it fails the next idle read.
        Action::Use => match session
            .use_object(DOOR_PLACEMENT.as_bytes(), JOIN_DOOR_REVISION)
            .await
        {
            Ok(_) => session.service_liveness(Duration::from_millis(200)).await,
            Err(error) => Err(error),
        },
    };
    match &result {
        Err(error) if expected(error) => {}
        other => return Err(format!("unexpected command result: {other:?}").into()),
    }
    assert!(matches!(
        session.step(StepDirection::East).await,
        Err(DevClientError::Session(SessionError::SessionUnusable))
    ));
    drop(session);
    server.await??;
    Ok(())
}

#[test]
fn a_command_result_for_another_command_id_is_rejected() -> Result<(), BoxError> {
    block_on(assert_command_fails(
        Action::Step,
        vec![step_result_frame(41, 8, StepDisposition::Blocked)?],
        |error| {
            matches!(
                error,
                DevClientError::Session(SessionError::CommandIdMismatch {
                    expected: 7,
                    actual: 8
                })
            )
        },
    ))?
}

#[test]
fn a_server_sequence_gap_or_replay_is_rejected() -> Result<(), BoxError> {
    // A gap (42), a replay of the join's own sequence (40) and a reset (1).
    for wrong in [42, 40, 1] {
        block_on(assert_command_fails(
            Action::Step,
            vec![step_result_frame(wrong, 7, StepDisposition::Blocked)?],
            move |error| {
                matches!(
                    error,
                    DevClientError::Session(SessionError::ServerSequenceMismatch {
                        expected: 41,
                        actual
                    }) if *actual == wrong
                )
            },
        ))??;
    }
    Ok(())
}

#[test]
fn a_frame_for_another_connection_generation_is_rejected() -> Result<(), BoxError> {
    let stale = oteryn_protocol_oteryn::encode_command_result(
        2,
        41,
        7,
        CommandStatus::Accepted,
        &world_spatial::encode_step_result(StepDisposition::Blocked),
    )?;
    block_on(assert_command_fails(Action::Step, vec![stale], |error| {
        matches!(
            error,
            DevClientError::Session(SessionError::ConnectionGenerationMismatch {
                expected: 1,
                actual: 2
            })
        )
    }))?
}

/// `CommandResult` is a server-sequenced message: an unsequenced one (`server_sequence` 0) is
/// refused by `WireEnvelopeView::validate` before its payload is consumed.
#[test]
fn an_unsequenced_command_result_is_rejected_by_envelope_validation() -> Result<(), BoxError> {
    let unsequenced = test_server_frame_with_sequence(
        MessageType::CommandResult,
        GENERATION,
        0,
        &[0x08, 0x07, 0x10, 0x01, 0x2a, 0x02, 0x08, 0x02],
    );
    block_on(assert_command_fails(
        Action::Step,
        vec![unsequenced],
        |error| {
            matches!(
                error,
                DevClientError::Session(SessionError::Protocol(
                    FoundationProtocolError::MalformedEnvelope
                ))
            )
        },
    ))?
}

#[test]
fn a_protocol_error_in_place_of_a_command_result_is_reported() -> Result<(), BoxError> {
    let gap = oteryn_protocol_oteryn::encode_command_protocol_error(
        FoundationProtocolError::CommandSequenceGap,
        GENERATION,
        7,
        6,
    )?;
    block_on(assert_command_fails(Action::Step, vec![gap], |error| {
        matches!(
            error,
            DevClientError::Session(SessionError::UnexpectedMessage {
                expected: MessageType::CommandResult,
                actual: MessageType::ProtocolError
            })
        )
    }))?
}

#[test]
fn a_moved_step_without_its_delta_is_rejected() -> Result<(), BoxError> {
    block_on(assert_command_fails(
        Action::Step,
        vec![
            step_result_frame(41, 7, StepDisposition::Moved)?,
            step_result_frame(42, 8, StepDisposition::Blocked)?,
        ],
        |error| {
            matches!(
                error,
                DevClientError::Session(SessionError::UnexpectedMessage {
                    expected: MessageType::StateDelta,
                    actual: MessageType::CommandResult
                })
            )
        },
    ))?
}

#[test]
fn a_spatial_delta_naming_the_wrong_domain_or_type_or_base_is_rejected() -> Result<(), BoxError> {
    let moved = step_result_frame(41, 7, StepDisposition::Moved)?;
    let spatial = spatial_payload(1, 0, CONTENT_GENERATION);
    // A domain the session keeps no store for.
    block_on(assert_command_fails(
        Action::Step,
        vec![moved.clone(), state_delta_frame(42, 4242, 0, 1, 1, b"x")?],
        |error| {
            matches!(
                error,
                DevClientError::Session(SessionError::UnsupportedPushedDomain { domain_id: 4242 })
            )
        },
    ))??;
    // Unregistered delta type.
    block_on(assert_command_fails(
        Action::Step,
        vec![moved.clone(), state_delta_frame(42, 1, 5, 6, 2, &spatial)?],
        |error| {
            matches!(
                error,
                DevClientError::Session(SessionError::UnregisteredDeltaType {
                    domain_id: 1,
                    delta_type: 2
                })
            )
        },
    ))??;
    // Base revision that is not the applied one (5).
    block_on(assert_command_fails(
        Action::Step,
        vec![moved.clone(), spatial_delta_frame(42, 6, 1, 0)?],
        |error| {
            matches!(
                error,
                DevClientError::Session(SessionError::StateRevisionMismatch {
                    domain_id: 1,
                    expected_base: 5,
                    actual_base: 6
                })
            )
        },
    ))??;
    // A content generation other than the loaded one.
    block_on(assert_command_fails(
        Action::Step,
        vec![
            moved.clone(),
            state_delta_frame(42, 1, 5, 6, 1, &spatial_payload(1, 0, OTHER_GENERATION))?,
        ],
        |error| {
            matches!(
                error,
                DevClientError::Session(SessionError::ContentGenerationMismatch { domain_id: 1 })
            )
        },
    ))??;
    // The delta itself is sequenced: a gap after the result is rejected too.
    block_on(assert_command_fails(
        Action::Step,
        vec![moved, spatial_delta_frame(43, 5, 1, 0)?],
        |error| {
            matches!(
                error,
                DevClientError::Session(SessionError::ServerSequenceMismatch {
                    expected: 42,
                    actual: 43
                })
            )
        },
    ))?
}

#[test]
fn an_overlay_delta_naming_the_wrong_domain_or_base_or_generation_is_rejected()
-> Result<(), BoxError> {
    let committed = use_result_frame(41, 7, UseDisposition::Committed)?;
    // A spatial delta based on the wrong revision.
    block_on(assert_command_fails(
        Action::Use,
        vec![committed.clone(), spatial_delta_frame(42, 4, 1, 0)?],
        |error| {
            matches!(
                error,
                DevClientError::Session(SessionError::StateRevisionMismatch {
                    domain_id: 1,
                    expected_base: 5,
                    actual_base: 4
                })
            )
        },
    ))??;
    // Base revision that is not the applied one (2).
    block_on(assert_command_fails(
        Action::Use,
        vec![
            committed.clone(),
            overlay_delta_frame(42, 1, DOOR_OPEN_STATE)?,
        ],
        |error| {
            matches!(
                error,
                DevClientError::Session(SessionError::StateRevisionMismatch {
                    domain_id: 2,
                    expected_base: 2,
                    actual_base: 1
                })
            )
        },
    ))??;
    // A content generation other than the loaded one.
    let foreign = world_object::encode_world_object_overlay_delta(&door_entry(
        DOOR_OPEN_STATE,
        3,
        OTHER_GENERATION,
    ))
    .map_err(|error| format!("encode_world_object_overlay_delta: {error:?}"))?;
    block_on(assert_command_fails(
        Action::Use,
        vec![
            committed,
            state_delta_frame(
                42,
                2,
                2,
                3,
                world_object::DELTA_TYPE_WORLD_OBJECT_OVERLAY_V1,
                &foreign,
            )?,
        ],
        |error| {
            matches!(
                error,
                DevClientError::Session(SessionError::ContentGenerationMismatch { domain_id: 2 })
            )
        },
    ))?
}

/// A server that receives the command and never answers fails with a bounded diagnostic
/// naming the stage, and the session is then unusable.
#[test]
fn a_stalled_command_result_times_out_and_poisons_the_session() -> Result<(), BoxError> {
    block_on(run_command_stall_case())?
}

async fn run_command_stall_case() -> Result<(), BoxError> {
    let (mut session, server) =
        joined_session(Duration::from_millis(200), |mut stream| async move {
            read_command(&mut stream).await?;
            wait_for_client_close(&mut stream).await;
            Ok(())
        })
        .await?;
    assert!(matches!(
        session.step(StepDirection::East).await,
        Err(DevClientError::Session(SessionError::Timeout(
            "CommandResult"
        )))
    ));
    assert!(matches!(
        session.step(StepDirection::East).await,
        Err(DevClientError::Session(SessionError::SessionUnusable))
    ));
    drop(session);
    server.await??;
    Ok(())
}

/// Codex P2: an overlay delta whose decoded entry `revision` differs from the delta's own
/// `new_revision` is refused and nothing from it is installed.
#[test]
fn an_overlay_entry_revision_differing_from_new_revision_is_not_installed() -> Result<(), BoxError>
{
    block_on(run_overlay_entry_revision_case())?
}

async fn run_overlay_entry_revision_case() -> Result<(), BoxError> {
    // base 2 -> new 3, but the entry claims revision 4.
    let entry = world_object::encode_world_object_overlay_delta(&door_entry(
        DOOR_OPEN_STATE,
        4,
        CONTENT_GENERATION,
    ))
    .map_err(|error| format!("encode_world_object_overlay_delta: {error:?}"))?;
    let frames = vec![
        use_result_frame(41, 7, UseDisposition::Committed)?,
        state_delta_frame(
            42,
            STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
            2,
            3,
            world_object::DELTA_TYPE_WORLD_OBJECT_OVERLAY_V1,
            &entry,
        )?,
    ];
    let (mut session, server) = joined_session(TEST_DEADLINE, move |mut stream| async move {
        read_command(&mut stream).await?;
        send(&mut stream, &frames).await?;
        wait_for_client_close(&mut stream).await;
        Ok(())
    })
    .await?;
    session
        .use_object(DOOR_PLACEMENT.as_bytes(), JOIN_DOOR_REVISION)
        .await?;
    let result = session.service_liveness(Duration::from_millis(200)).await;
    assert!(matches!(
        result,
        Err(DevClientError::Session(
            SessionError::OverlayEntryRevisionMismatch {
                new_revision: 3,
                entry_revision: 4
            }
        ))
    ));
    assert_eq!(
        session.world_object_overlay(),
        &[door_entry(
            DOOR_STATE,
            JOIN_DOOR_REVISION,
            CONTENT_GENERATION
        )]
    );
    assert!(matches!(
        session.step(StepDirection::East).await,
        Err(DevClientError::Session(SessionError::SessionUnusable))
    ));
    drop(session);
    server.await??;
    Ok(())
}

/// Codex P2: a `CommandResult` status that does not pair with its disposition fails closed
/// (the server sends `ACCEPTED` for everything except `Rejected`, which gets `REJECTED`).
#[test]
fn an_inconsistent_status_and_disposition_pairing_is_rejected() -> Result<(), BoxError> {
    let step = |disposition| world_spatial::encode_step_result(disposition);
    let use_ = |disposition| world_object::encode_use_result(disposition).unwrap_or_default();
    for (action, status, payload) in [
        (
            Action::Step,
            CommandStatus::Accepted,
            step(StepDisposition::Rejected),
        ),
        (
            Action::Step,
            CommandStatus::Rejected,
            step(StepDisposition::Blocked),
        ),
        (
            Action::Step,
            CommandStatus::Rejected,
            step(StepDisposition::Moved),
        ),
        (
            Action::Use,
            CommandStatus::Rejected,
            use_(UseDisposition::Committed),
        ),
        (
            Action::Use,
            CommandStatus::Rejected,
            use_(UseDisposition::StaleState),
        ),
        (
            Action::Use,
            CommandStatus::Accepted,
            use_(UseDisposition::Rejected),
        ),
    ] {
        block_on(assert_command_fails(
            action,
            vec![command_result_frame(41, 7, status, &payload)?],
            move |error| {
                matches!(
                    error,
                    DevClientError::Session(SessionError::InconsistentCommandResult { command_id: 7, status: actual })
                        if *actual == status
                )
            },
        ))??;
    }
    Ok(())
}
