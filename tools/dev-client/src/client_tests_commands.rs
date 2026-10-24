/// The full door scenario (`use_wire`'s command mix), one command at a time: every
/// disposition, the `CommandId` and `server_sequence` discipline, the domain-1 and domain-2
/// revision chains, and the session's own mirrored state after every applied delta.
#[test]
fn step_and_use_follow_command_id_sequence_and_revision_discipline() -> Result<(), BoxError> {
    block_on(run_door_scenario_case())?
}

async fn run_door_scenario_case() -> Result<(), BoxError> {
    let door = DOOR_PLACEMENT.as_bytes();
    let (mut session, server) = joined_session(TEST_DEADLINE, move |mut stream| async move {
        // ids 7..=15, sequences 41..=54: east(Moved), use open(Committed), north(Moved),
        // use(Occupied), south(Moved), use close(Committed), north(Blocked), use with a stale
        // revision, use of an unknown placement.
        expect_step(&mut stream, 7, StepDirection::East).await?;
        send(
            &mut stream,
            &[
                step_result_frame(41, 7, StepDisposition::Moved)?,
                spatial_delta_frame(42, 5, 1, 0)?,
            ],
        )
        .await?;
        expect_use(&mut stream, 8, DOOR_PLACEMENT.as_bytes(), 2).await?;
        send(
            &mut stream,
            &[
                use_result_frame(43, 8, UseDisposition::Committed)?,
                overlay_delta_frame(44, 2, DOOR_OPEN_STATE)?,
            ],
        )
        .await?;
        expect_step(&mut stream, 9, StepDirection::North).await?;
        send(
            &mut stream,
            &[
                step_result_frame(45, 9, StepDisposition::Moved)?,
                spatial_delta_frame(46, 6, 1, -1)?,
            ],
        )
        .await?;
        expect_use(&mut stream, 10, DOOR_PLACEMENT.as_bytes(), 3).await?;
        send(
            &mut stream,
            &[use_result_frame(47, 10, UseDisposition::Occupied)?],
        )
        .await?;
        expect_step(&mut stream, 11, StepDirection::South).await?;
        send(
            &mut stream,
            &[
                step_result_frame(48, 11, StepDisposition::Moved)?,
                spatial_delta_frame(49, 7, 1, 0)?,
            ],
        )
        .await?;
        expect_use(&mut stream, 12, DOOR_PLACEMENT.as_bytes(), 3).await?;
        send(
            &mut stream,
            &[
                use_result_frame(50, 12, UseDisposition::Committed)?,
                overlay_delta_frame(51, 3, DOOR_STATE)?,
            ],
        )
        .await?;
        expect_step(&mut stream, 13, StepDirection::North).await?;
        send(
            &mut stream,
            &[step_result_frame(52, 13, StepDisposition::Blocked)?],
        )
        .await?;
        expect_use(&mut stream, 14, DOOR_PLACEMENT.as_bytes(), 2).await?;
        send(
            &mut stream,
            &[use_result_frame(53, 14, UseDisposition::StaleState)?],
        )
        .await?;
        expect_use(&mut stream, 15, b"oteryn:cell/unknown", 4).await?;
        send(
            &mut stream,
            &[use_result_frame(54, 15, UseDisposition::NothingToUse)?],
        )
        .await?;
        wait_for_client_close(&mut stream).await;
        Ok(())
    })
    .await?;

    let joined = session.join_snapshot();
    assert_eq!(joined.world_spatial_revision, JOIN_SPATIAL_REVISION);
    assert_eq!(joined.world_object_overlay_revision, JOIN_DOOR_REVISION);
    assert_eq!(session.next_command_id(), FIRST_COMMAND_ID);
    assert_eq!(session.last_server_sequence(), JOIN_SEQUENCE);

    assert_eq!(
        session.step(StepDirection::East).await?,
        step_outcome(7, 41, StepDisposition::Moved, moved_to(42, 5, 1, 0))
    );
    assert_eq!(
        session.use_object(door, 2).await?,
        door_use_outcome(8, 43, UseDisposition::Committed, None)
    );
    // A use returns at its result: the door delta behind it is read by the next exchange.
    assert_eq!(
        session.world_object_overlay(),
        &[door_entry(
            DOOR_STATE,
            JOIN_DOOR_REVISION,
            CONTENT_GENERATION
        )]
    );
    assert_eq!(
        session.step(StepDirection::North).await?,
        step_outcome(9, 45, StepDisposition::Moved, moved_to(46, 6, 1, -1))
    );
    assert_eq!(
        session.world_object_overlay(),
        &[door_entry(DOOR_OPEN_STATE, 3, CONTENT_GENERATION)]
    );
    assert_eq!(
        session.take_events(),
        vec![SessionEvent::WorldObjectOverlay(
            door_delta(44, 2, DOOR_OPEN_STATE).ok_or("door delta")?
        )]
    );
    assert_eq!(
        session.use_object(door, 3).await?,
        door_use_outcome(10, 47, UseDisposition::Occupied, None)
    );
    assert_eq!(
        session.step(StepDirection::South).await?,
        step_outcome(11, 48, StepDisposition::Moved, moved_to(49, 7, 1, 0))
    );
    assert_eq!(
        session.use_object(door, 3).await?,
        door_use_outcome(12, 50, UseDisposition::Committed, None)
    );
    assert_eq!(
        session.step(StepDirection::North).await?,
        step_outcome(13, 52, StepDisposition::Blocked, None)
    );
    assert_eq!(
        session.take_events(),
        vec![SessionEvent::WorldObjectOverlay(
            door_delta(51, 3, DOOR_STATE).ok_or("door delta")?
        )]
    );
    assert_eq!(
        session.use_object(door, 2).await?,
        door_use_outcome(14, 53, UseDisposition::StaleState, None)
    );
    assert_eq!(
        session.use_object(b"oteryn:cell/unknown", 4).await?,
        door_use_outcome(15, 54, UseDisposition::NothingToUse, None)
    );
    assert_eq!(session.next_command_id(), 16);
    assert_eq!(session.last_server_sequence(), 54);
    assert_eq!(
        session.world_spatial().actor_position,
        ActorPosition {
            x: 1,
            y: 0,
            floor: 0
        }
    );
    assert_eq!(
        session.world_object_overlay(),
        &[door_entry(DOOR_STATE, 4, CONTENT_GENERATION)]
    );
    // The original join snapshot is untouched by later deltas.
    assert_eq!(session.join_snapshot().world_object_overlay[0].revision, 2);
    drop(session);
    server.await??;
    Ok(())
}

/// The dev client advertises capability 7 and forwards chat; with the server selecting
/// nothing (CHAT-1b-2b is not live) there is no chat state and `chat` is refused unsent.
#[test]
fn chat_is_refused_and_has_no_log_while_capability_7_is_unselected() -> Result<(), BoxError> {
    block_on(async {
        assert!(CLIENT_SUPPORTED_CAPABILITIES.contains(&7));
        let (mut session, server) =
            joined_session(Duration::from_secs(5), |mut stream| async move {
                wait_for_client_close(&mut stream).await;
                Ok(())
            })
            .await?;
        assert!(session.chat_log().is_none());
        let error = session
            .chat(&ChatIntent::OpenRoom(ChatRoom::World))
            .await
            .err()
            .ok_or("chat must be refused")?;
        assert!(matches!(
            error,
            DevClientError::CapabilityNotSelected { capability: 7 }
        ));
        assert_eq!(session.next_command_id(), FIRST_COMMAND_ID);
        drop(session);
        server.await??;
        Ok(())
    })?
}

/// The dev client advertises capability 4; with the server selecting nothing there is no item
/// state and `use_item` and `move_item` are refused unsent.
#[test]
fn item_commands_are_refused_and_have_no_state_while_capability_4_is_unselected()
-> Result<(), BoxError> {
    block_on(async {
        assert!(CLIENT_SUPPORTED_CAPABILITIES.contains(&4));
        let (mut session, server) =
            joined_session(Duration::from_secs(5), |mut stream| async move {
                wait_for_client_close(&mut stream).await;
                Ok(())
            })
            .await?;
        assert!(session.inventory().is_none());
        assert!(session.open_container().is_none());
        let handle = ItemHandle::MIN;
        let error = session
            .use_item(handle)
            .await
            .err()
            .ok_or("use_item must be refused")?;
        assert!(matches!(
            error,
            DevClientError::CapabilityNotSelected { capability: 4 }
        ));
        let error = session
            .move_item(&ItemMoveIntent {
                source: handle,
                destination: ItemMoveDestination::MainBackpack,
            })
            .await
            .err()
            .ok_or("move_item must be refused")?;
        assert!(matches!(
            error,
            DevClientError::CapabilityNotSelected { capability: 4 }
        ));
        assert_eq!(session.next_command_id(), FIRST_COMMAND_ID);
        drop(session);
        server.await??;
        Ok(())
    })?
}

/// A `REJECTED` step (an ineligible actor, `StepDisposition::Rejected`) carries no delta and
/// leaves the session usable for the next command.
#[test]
fn a_rejected_step_carries_no_delta_and_the_session_stays_usable() -> Result<(), BoxError> {
    block_on(run_rejected_step_case())?
}

async fn run_rejected_step_case() -> Result<(), BoxError> {
    let (mut session, server) = joined_session(TEST_DEADLINE, |mut stream| async move {
        expect_step(&mut stream, 7, StepDirection::West).await?;
        send(
            &mut stream,
            &[step_result_frame(41, 7, StepDisposition::Rejected)?],
        )
        .await?;
        expect_step(&mut stream, 8, StepDirection::East).await?;
        send(
            &mut stream,
            &[
                step_result_frame(42, 8, StepDisposition::Moved)?,
                spatial_delta_frame(43, 5, 1, 0)?,
            ],
        )
        .await?;
        wait_for_client_close(&mut stream).await;
        Ok(())
    })
    .await?;
    let rejected = session.step(StepDirection::West).await?;
    assert_eq!(rejected.status, CommandStatus::Rejected);
    assert_eq!(rejected.disposition, StepDisposition::Rejected);
    assert!(rejected.world_spatial_delta.is_none());
    assert_eq!(
        session.step(StepDirection::East).await?,
        step_outcome(8, 42, StepDisposition::Moved, moved_to(43, 5, 1, 0))
    );
    drop(session);
    server.await??;
    Ok(())
}

/// A `LivenessProbe` that arrives while a command's reply is being read (before its
/// `CommandResult`, or between it and its delta) is answered with a `LivenessAck` for that probe (carrying the last applied `server_sequence`)
/// and does not disturb the command flow or the sequence chain.
#[test]
fn liveness_probes_are_acked_and_do_not_disturb_the_command_flow() -> Result<(), BoxError> {
    block_on(run_liveness_case())?
}

async fn run_liveness_case() -> Result<(), BoxError> {
    let (mut session, server) = joined_session(TEST_DEADLINE, |mut stream| async move {
        expect_step(&mut stream, 7, StepDirection::East).await?;
        send(
            &mut stream,
            &[
                oteryn_protocol_oteryn::encode_liveness_probe(GENERATION, 3)?,
                step_result_frame(41, 7, StepDisposition::Moved)?,
                oteryn_protocol_oteryn::encode_liveness_probe(GENERATION, 4)?,
                spatial_delta_frame(42, 5, 1, 0)?,
            ],
        )
        .await?;
        for (probe_id, last_applied) in [(3, JOIN_SEQUENCE), (4, 41)] {
            let body = read_frame(&mut stream).await?;
            let ack = decode_wire_envelope(&body)?.liveness_ack(GENERATION)?;
            assert_eq!(
                ack,
                oteryn_protocol_oteryn::LivenessAckView {
                    probe_id,
                    last_applied_server_sequence: Some(last_applied),
                }
            );
        }
        wait_for_client_close(&mut stream).await;
        Ok(())
    })
    .await?;
    assert_eq!(
        session.step(StepDirection::East).await?,
        step_outcome(7, 41, StepDisposition::Moved, moved_to(42, 5, 1, 0))
    );
    drop(session);
    server.await??;
    Ok(())
}

/// A placement over the accepted key bound is refused before anything is sent: the
/// `CommandId` is not consumed and the session stays usable.
#[test]
fn an_unencodable_use_is_refused_locally_without_consuming_a_command_id() -> Result<(), BoxError> {
    block_on(run_unencodable_use_case())?
}

async fn run_unencodable_use_case() -> Result<(), BoxError> {
    let (mut session, server) = joined_session(TEST_DEADLINE, |mut stream| async move {
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
    let oversized = vec![b'x'; 513];
    assert!(matches!(
        session.use_object(&oversized, 0).await,
        Err(DevClientError::WorldObject(
            world_object::WorldObjectError::LimitExceeded
        ))
    ));
    assert_eq!(session.next_command_id(), FIRST_COMMAND_ID);
    assert_eq!(
        session.step(StepDirection::East).await?,
        step_outcome(7, 41, StepDisposition::Blocked, None)
    );
    drop(session);
    server.await??;
    Ok(())
}
