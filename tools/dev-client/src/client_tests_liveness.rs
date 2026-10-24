/// Codex P2: a probe that arrives while the session is idle is acked by `service_liveness`,
/// which then returns when the window ends; the session stays usable.
#[test]
fn service_liveness_acks_probes_that_arrive_while_idle() -> Result<(), BoxError> {
    block_on(run_idle_liveness_case())?
}

async fn run_idle_liveness_case() -> Result<(), BoxError> {
    let (mut session, server) = joined_session(TEST_DEADLINE, |mut stream| async move {
        for probe_id in [5, 6] {
            send(
                &mut stream,
                &[oteryn_protocol_oteryn::encode_liveness_probe(
                    GENERATION, probe_id,
                )?],
            )
            .await?;
            let body = read_frame(&mut stream).await?;
            assert_eq!(
                decode_wire_envelope(&body)?.liveness_ack(GENERATION)?,
                oteryn_protocol_oteryn::LivenessAckView {
                    probe_id,
                    last_applied_server_sequence: Some(JOIN_SEQUENCE),
                }
            );
        }
        expect_step(&mut stream, 7, StepDirection::East).await?;
        send(
            &mut stream,
            &[step_result_frame(41, 7, StepDisposition::Blocked)?],
        )
        .await?;
        wait_for_client_close(&mut stream).await;
        Ok(())
    })
    .await?;
    let started = tokio::time::Instant::now();
    session.service_liveness(Duration::from_millis(400)).await?;
    assert!(started.elapsed() >= Duration::from_millis(350));
    assert_eq!(
        session.step(StepDirection::East).await?,
        step_outcome(7, 41, StepDisposition::Blocked, None)
    );
    drop(session);
    server.await??;
    Ok(())
}

/// `service_liveness` fails closed on a reused probe ID, and the session is then unusable.
#[test]
fn service_liveness_rejects_a_reused_probe_id() -> Result<(), BoxError> {
    block_on(run_idle_reused_probe_case())?
}

async fn run_idle_reused_probe_case() -> Result<(), BoxError> {
    let (mut session, server) = joined_session(TEST_DEADLINE, |mut stream| async move {
        for _ in 0..2 {
            send(
                &mut stream,
                &[oteryn_protocol_oteryn::encode_liveness_probe(
                    GENERATION, 5,
                )?],
            )
            .await?;
        }
        let _first_ack = read_frame(&mut stream).await?;
        wait_for_client_close(&mut stream).await;
        Ok(())
    })
    .await?;
    assert!(matches!(
        session.service_liveness(Duration::from_secs(2)).await,
        Err(DevClientError::ProbeIdNotAdvancing {
            last: 5,
            received: 5
        })
    ));
    assert!(matches!(
        session.service_liveness(Duration::from_millis(10)).await,
        Err(DevClientError::SessionUnusable)
    ));
    drop(session);
    server.await??;
    Ok(())
}

#[test]
fn service_liveness_fails_closed_on_an_unexpected_frame() -> Result<(), BoxError> {
    block_on(run_idle_unexpected_frame_case())?
}

async fn run_idle_unexpected_frame_case() -> Result<(), BoxError> {
    let (mut session, server) = joined_session(TEST_DEADLINE, |mut stream| async move {
        send(
            &mut stream,
            &[step_result_frame(41, 7, StepDisposition::Blocked)?],
        )
        .await?;
        wait_for_client_close(&mut stream).await;
        Ok(())
    })
    .await?;
    assert!(matches!(
        session.service_liveness(Duration::from_secs(2)).await,
        Err(DevClientError::UnexpectedMessage {
            expected: MessageType::LivenessProbe,
            actual: MessageType::CommandResult
        })
    ));
    assert!(matches!(
        session.service_liveness(Duration::from_millis(10)).await,
        Err(DevClientError::SessionUnusable)
    ));
    drop(session);
    server.await??;
    Ok(())
}

/// Codex P1: FND-02 §13.2 duplicate results for earlier CommandIds this session sent are
/// surfaced as distinct outcomes (no delta), do not end the wait for the current command's
/// own result, and do not poison the session.
#[test]
fn duplicate_results_for_sent_commands_are_surfaced_and_do_not_poison_the_session()
-> Result<(), BoxError> {
    block_on(run_duplicate_outcome_case())?
}

async fn run_duplicate_outcome_case() -> Result<(), BoxError> {
    let replayed = world_spatial::encode_step_result(StepDisposition::Blocked);
    let replayed_frames = replayed.clone();
    let (mut session, server) = joined_session(TEST_DEADLINE, move |mut stream| async move {
        expect_step(&mut stream, 7, StepDirection::West).await?;
        send(
            &mut stream,
            &[step_result_frame(41, 7, StepDisposition::Blocked)?],
        )
        .await?;
        expect_step(&mut stream, 8, StepDirection::East).await?;
        send(
            &mut stream,
            &[
                command_result_frame(42, 7, CommandStatus::DuplicateReplay, &replayed_frames)?,
                command_result_frame(43, 7, CommandStatus::DuplicateOutcomeExpired, &[])?,
                step_result_frame(44, 8, StepDisposition::Moved)?,
                spatial_delta_frame(45, 5, 1, 0)?,
            ],
        )
        .await?;
        expect_step(&mut stream, 9, StepDirection::West).await?;
        send(
            &mut stream,
            &[step_result_frame(46, 9, StepDisposition::Blocked)?],
        )
        .await?;
        wait_for_client_close(&mut stream).await;
        Ok(())
    })
    .await?;
    session.step(StepDirection::West).await?;
    assert!(session.take_duplicate_outcomes().is_empty());
    assert_eq!(
        session.step(StepDirection::East).await?,
        step_outcome(8, 44, StepDisposition::Moved, moved_to(45, 5, 1, 0))
    );
    assert_eq!(
        session.take_duplicate_outcomes(),
        vec![
            DuplicateOutcome {
                command_id: 7,
                status: CommandStatus::DuplicateReplay,
                server_sequence: 42,
                payload: replayed,
            },
            DuplicateOutcome {
                command_id: 7,
                status: CommandStatus::DuplicateOutcomeExpired,
                server_sequence: 43,
                payload: Vec::new(),
            },
        ]
    );
    assert!(session.take_duplicate_outcomes().is_empty());
    assert_eq!(
        session.step(StepDirection::West).await?,
        step_outcome(9, 46, StepDisposition::Blocked, None)
    );
    drop(session);
    server.await??;
    Ok(())
}

/// A duplicate for a CommandId this session never sent (below its first, or not yet sent), or
/// for the command just sent, fails closed.
#[test]
fn a_duplicate_for_an_unsent_command_id_fails_closed() -> Result<(), BoxError> {
    for unsent in [6, 8, 99] {
        block_on(assert_command_fails(
            Action::Step,
            vec![command_result_frame(
                41,
                unsent,
                CommandStatus::DuplicateReplay,
                &[],
            )?],
            move |error| {
                matches!(
                    error,
                    DevClientError::DuplicateForUnsentCommand { command_id }
                        if *command_id == unsent
                )
            },
        ))??;
    }
    block_on(assert_command_fails(
        Action::Step,
        vec![command_result_frame(
            41,
            7,
            CommandStatus::DuplicateOutcomeExpired,
            &[],
        )?],
        |error| {
            matches!(
                error,
                DevClientError::InconsistentCommandResult {
                    command_id: 7,
                    status: CommandStatus::DuplicateOutcomeExpired
                }
            )
        },
    ))?
}
