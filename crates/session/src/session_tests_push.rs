// --- SESSION-PUSH-1: server-initiated deltas, the domain store and the bounded event queue.

enum Step {
    Send(Vec<u8>),
    /// Frames written in one `write_all`, so the client can never read the first frame
    /// and drop the stream before the rest is in the pipe.
    SendTogether(Vec<Vec<u8>>),
    ReadCommand,
}

/// Joins (core domains only, no selected capability) and plays `script`, then holds the
/// stream open until the client drops it.
fn script_peer(script: Vec<Step>) -> (DuplexStream, tokio::task::JoinHandle<Result<(), BoxError>>) {
    let (client, mut server) = tokio::io::duplex(256 * 1024);
    let peer = tokio::spawn(async move {
        join_peer(&mut server, &[13], &[], &[]).await?;
        for step in script {
            play_step(&mut server, step).await?;
        }
        let mut rest = Vec::new();
        let _ = server.read_to_end(&mut rest).await;
        Ok::<(), BoxError>(())
    });
    (client, peer)
}

/// Plays one scripted step on the peer side.
async fn play_step(server: &mut DuplexStream, step: Step) -> Result<(), BoxError> {
    match step {
        Step::Send(frame) => write_frame(server, &frame).await?,
        Step::SendTogether(frames) => {
            let mut bytes = Vec::new();
            for frame in frames {
                bytes
                    .extend_from_slice(&FrameLength::new(u32::try_from(frame.len())?)?.to_prefix());
                bytes.extend_from_slice(&frame);
            }
            server.write_all(&bytes).await?;
            server.flush().await?;
        }
        Step::ReadCommand => {
            read_frame(server).await?;
        }
    }
    Ok(())
}

/// Joins the bodies of `Send` steps into one `SendTogether` step.
fn together(steps: Vec<Step>) -> Result<Step, BoxError> {
    steps
        .into_iter()
        .map(|step| match step {
            Step::Send(frame) => Ok(frame),
            Step::SendTogether(_) | Step::ReadCommand => {
                Err("only Send steps can be joined".into())
            }
        })
        .collect::<Result<Vec<_>, BoxError>>()
        .map(Step::SendTogether)
}

fn result_frame(sequence: u64, id: u64, disposition: StepDisposition) -> Result<Step, BoxError> {
    Ok(Step::Send(encode_command_result(
        1,
        sequence,
        id,
        CommandStatus::Accepted,
        &encode_step_result(disposition),
    )?))
}

fn spatial_push(sequence: u64, base: u64, new: u64, x: i32) -> Result<Step, BoxError> {
    Ok(Step::Send(encode_state_delta(
        1,
        sequence,
        STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
        base,
        new,
        DELTA_TYPE_WORLD_SPATIAL_V1,
        &spatial(x),
    )?))
}

fn vitals_push(sequence: u64, base: u64, new: u64, health: u32) -> Result<Step, BoxError> {
    vitals_push_typed(
        sequence,
        base,
        new,
        health,
        actor_spell::DELTA_TYPE_ACTOR_VITALS_V1,
    )
}

fn vitals_push_typed(
    sequence: u64,
    base: u64,
    new: u64,
    health: u32,
    delta_type: u32,
) -> Result<Step, BoxError> {
    Ok(Step::Send(encode_state_delta(
        1,
        sequence,
        actor_spell::STATE_DOMAIN_ACTOR_VITALS,
        base,
        new,
        delta_type,
        &actor_spell::encode_actor_vitals(&vitals(health, 55))
            .map_err(|error| format!("vitals: {error:?}"))?,
    )?))
}

fn door_entry(revision: u64) -> WireOverlayEntry {
    WireOverlayEntry {
        content_generation: CONTENT_GENERATION,
        placement: b"oteryn:cell/entry-door".to_vec(),
        state: b"oteryn:reference.state.open".to_vec(),
        revision,
    }
}

fn overlay_push(sequence: u64, base: u64, new: u64) -> Result<Step, BoxError> {
    Ok(Step::Send(encode_state_delta(
        1,
        sequence,
        STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
        base,
        new,
        world_object::DELTA_TYPE_WORLD_OBJECT_OVERLAY_V1,
        &world_object::encode_world_object_overlay_delta(&door_entry(new))
            .map_err(|error| format!("overlay delta: {error:?}"))?,
    )?))
}

fn vitals_event(sequence: u64, base: u64, new: u64, health: u32) -> SessionEvent {
    SessionEvent::ActorVitals(AppliedDelta {
        server_sequence: sequence,
        base_revision: base,
        new_revision: new,
        value: vitals(health, 55),
    })
}

fn spatial_event(sequence: u64, base: u64, new: u64, x: i32) -> SessionEvent {
    SessionEvent::WorldSpatial(AppliedDelta {
        server_sequence: sequence,
        base_revision: base,
        new_revision: new,
        value: WireSpatialObservation {
            content_generation: CONTENT_GENERATION,
            actor_position: ActorPosition { x, y: 0, floor: 0 },
        },
    })
}

#[test]
fn a_pushed_vitals_delta_during_idle_applies_and_is_queued_once() -> Result<(), BoxError> {
    block_on(async {
        let (client, peer) = script_peer(vec![vitals_push(41, 0, 1, 120)?]);
        let mut session = Session::admit(client, admission()?).await?;
        session.service_liveness(Duration::from_millis(100)).await?;
        assert_eq!(session.actor_vitals(), Some(&vitals(120, 55)));
        assert_eq!(session.last_server_sequence(), 41);
        assert_eq!(session.take_events(), vec![vitals_event(41, 0, 1, 120)]);
        assert!(session.take_events().is_empty());
        drop(session);
        peer.await??;
        Ok(())
    })?
}

#[test]
fn a_delta_pushed_before_the_result_applies_and_the_steps_own_delta_still_applies()
-> Result<(), BoxError> {
    block_on(async {
        let (client, peer) = script_peer(vec![
            Step::ReadCommand,
            vitals_push(41, 0, 1, 120)?,
            result_frame(42, 7, StepDisposition::Moved)?,
            spatial_push(43, 5, 6, 1)?,
        ]);
        let mut session = Session::admit(client, admission()?).await?;
        let outcome = session.step(StepDirection::East).await?;
        assert_eq!(outcome.result_server_sequence, 42);
        let delta = outcome
            .world_spatial_delta
            .ok_or("Moved carries its delta")?;
        assert_eq!((delta.server_sequence, delta.new_revision), (43, 6));
        assert_eq!(session.take_events(), vec![vitals_event(41, 0, 1, 120)]);
        drop(session);
        peer.await??;
        Ok(())
    })?
}

#[test]
fn a_delta_between_the_result_and_the_steps_own_delta_applies_as_pushed() -> Result<(), BoxError> {
    block_on(async {
        let (client, peer) = script_peer(vec![
            Step::ReadCommand,
            result_frame(41, 7, StepDisposition::Moved)?,
            vitals_push(42, 0, 1, 120)?,
            spatial_push(43, 5, 6, 1)?,
        ]);
        let mut session = Session::admit(client, admission()?).await?;
        let outcome = session.step(StepDirection::East).await?;
        let delta = outcome
            .world_spatial_delta
            .ok_or("Moved carries its delta")?;
        assert_eq!((delta.server_sequence, delta.base_revision), (43, 5));
        assert_eq!(session.world_spatial().actor_position.x, 1);
        assert_eq!(session.take_events(), vec![vitals_event(42, 0, 1, 120)]);
        drop(session);
        peer.await??;
        Ok(())
    })?
}

#[test]
fn a_cast_claims_its_vitals_delta_after_a_pushed_spatial_delta() -> Result<(), BoxError> {
    block_on(async {
        let (client, peer) = script_peer(vec![
            Step::ReadCommand,
            Step::Send(encode_command_result(
                1,
                41,
                7,
                CommandStatus::Accepted,
                &actor_spell::encode_spell_cast_result(SpellCastDisposition::Cast),
            )?),
            spatial_push(42, 5, 6, 1)?,
            vitals_push(43, 0, 1, 90)?,
        ]);
        let mut session = Session::admit(client, admission()?).await?;
        let spell = NonZeroU32::new(2).ok_or("nonzero")?;
        let cast = session.cast_spell(spell, SpellTarget::None, false).await?;
        let delta = cast.actor_vitals_delta.ok_or("Cast carries its delta")?;
        assert_eq!((delta.server_sequence, delta.value), (43, vitals(90, 55)));
        assert_eq!(session.take_events(), vec![spatial_event(42, 5, 6, 1)]);
        drop(session);
        peer.await??;
        Ok(())
    })?
}

#[test]
fn two_consecutive_spatial_deltas_end_the_step_at_the_first_and_nothing_is_lost()
-> Result<(), BoxError> {
    block_on(async {
        let (client, peer) = script_peer(vec![
            Step::ReadCommand,
            result_frame(41, 7, StepDisposition::Moved)?,
            spatial_push(42, 5, 6, 1)?,
            spatial_push(43, 6, 7, 2)?,
            // The second step's exchange reads the leftover delta before its own result.
            Step::ReadCommand,
            result_frame(44, 8, StepDisposition::Blocked)?,
        ]);
        let mut session = Session::admit(client, admission()?).await?;
        let first = session.step(StepDirection::East).await?;
        let delta = first.world_spatial_delta.ok_or("Moved carries its delta")?;
        assert_eq!(delta.new_revision, 6);
        assert_eq!(session.last_server_sequence(), 42);
        assert!(session.take_events().is_empty());
        let second = session.step(StepDirection::East).await?;
        assert_eq!(second.disposition, StepDisposition::Blocked);
        assert_eq!(session.world_spatial().actor_position.x, 2);
        assert_eq!(session.take_events(), vec![spatial_event(43, 6, 7, 2)]);
        drop(session);
        peer.await??;
        Ok(())
    })?
}

#[test]
fn a_leftover_delta_applies_on_the_next_idle_read() -> Result<(), BoxError> {
    block_on(async {
        let (client, peer) = script_peer(vec![
            Step::ReadCommand,
            result_frame(41, 7, StepDisposition::Moved)?,
            spatial_push(42, 5, 6, 1)?,
            spatial_push(43, 6, 7, 2)?,
        ]);
        let mut session = Session::admit(client, admission()?).await?;
        session.step(StepDirection::East).await?;
        session.service_liveness(Duration::from_millis(100)).await?;
        assert_eq!(session.take_events(), vec![spatial_event(43, 6, 7, 2)]);
        drop(session);
        peer.await??;
        Ok(())
    })?
}

#[test]
fn a_committed_use_returns_at_its_result_and_its_domain_2_delta_arrives_as_an_event()
-> Result<(), BoxError> {
    block_on(async {
        let committed = encode_command_result(
            1,
            41,
            7,
            CommandStatus::Accepted,
            &world_object::encode_use_result(UseDisposition::Committed).unwrap_or_default(),
        )?;
        let (client, peer) = script_peer(vec![
            Step::ReadCommand,
            Step::Send(committed),
            overlay_push(42, 2, 3)?,
        ]);
        let mut session = Session::admit(client, admission()?).await?;
        let outcome = session.use_object(b"oteryn:cell/entry-door", 2).await?;
        assert_eq!(outcome.disposition, UseDisposition::Committed);
        assert_eq!(outcome.world_object_overlay_delta, None);
        assert_eq!(outcome.world_spatial_delta, None);
        // Returned at the result: the delta behind it is still unread.
        assert_eq!(session.last_server_sequence(), 41);
        assert!(session.world_object_overlay().is_empty());
        session.service_liveness(Duration::from_millis(100)).await?;
        assert_eq!(session.world_object_overlay().len(), 1);
        let events = session.take_events();
        assert!(matches!(
            events.as_slice(),
            [SessionEvent::WorldObjectOverlay(delta)]
                if (delta.server_sequence, delta.base_revision, delta.new_revision) == (42, 2, 3)
        ));
        drop(session);
        peer.await??;
        Ok(())
    })?
}

#[test]
fn a_committed_use_with_no_delta_returns_at_its_result() -> Result<(), BoxError> {
    block_on(async {
        let committed = encode_command_result(
            1,
            41,
            7,
            CommandStatus::Accepted,
            &world_object::encode_use_result(UseDisposition::Committed).unwrap_or_default(),
        )?;
        let (client, peer) = script_peer(vec![Step::ReadCommand, Step::Send(committed)]);
        let mut session = Session::admit(client, admission()?).await?;
        let outcome = session.use_object(b"oteryn:cell/entry-door", 2).await?;
        assert_eq!(outcome.disposition, UseDisposition::Committed);
        assert_eq!(outcome.world_object_overlay_delta, None);
        assert!(session.take_events().is_empty());
        drop(session);
        peer.await??;
        Ok(())
    })?
}

/// Each of these frames fails closed on an idle read and leaves the session unusable.
async fn assert_idle_push_poisons(
    push: Step,
    check: impl FnOnce(&SessionError) -> bool,
) -> Result<(), BoxError> {
    let (client, peer) = script_peer(vec![push]);
    let mut session = Session::admit(client, admission()?).await?;
    let error = session
        .service_liveness(Duration::from_millis(200))
        .await
        .err()
        .ok_or("the push must fail closed")?;
    assert!(check(&error), "{error:?}");
    assert!(matches!(
        session.step(StepDirection::East).await,
        Err(SessionError::SessionUnusable)
    ));
    drop(session);
    peer.await??;
    Ok(())
}

#[test]
fn a_base_mismatch_an_unselected_domain_and_an_unregistered_type_poison_the_session()
-> Result<(), BoxError> {
    block_on(async {
        assert_idle_push_poisons(spatial_push(41, 4, 6, 1)?, |error| {
            matches!(
                error,
                SessionError::StateRevisionMismatch {
                    domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    expected_base: 5,
                    actual_base: 4
                }
            )
        })
        .await?;
        assert_idle_push_poisons(
            Step::Send(encode_state_delta(
                1,
                41,
                STATE_DOMAIN_CHAT,
                0,
                1,
                1,
                b"line",
            )?),
            |error| {
                matches!(
                    error,
                    SessionError::UnselectedDomain {
                        domain_id: STATE_DOMAIN_CHAT
                    }
                )
            },
        )
        .await?;
        assert_idle_push_poisons(vitals_push_typed(41, 0, 1, 90, 2)?, |error| {
            matches!(
                error,
                SessionError::UnregisteredDeltaType {
                    domain_id: actor_spell::STATE_DOMAIN_ACTOR_VITALS,
                    delta_type: 2
                }
            )
        })
        .await?;
        // A domain the session keeps no store for is not silently dropped either.
        assert_idle_push_poisons(
            Step::Send(encode_state_delta(1, 41, 4242, 0, 1, 1, b"x")?),
            |error| {
                matches!(
                    error,
                    SessionError::UnsupportedPushedDomain { domain_id: 4242 }
                )
            },
        )
        .await
    })?
}

#[test]
fn a_pushed_delta_mismatch_during_an_exchange_poisons_the_session() -> Result<(), BoxError> {
    block_on(async {
        // The result frame shares the push's write: the session stops reading at the
        // mismatched push, and a separate later write could race the drop below.
        let (client, peer) = script_peer(vec![
            Step::ReadCommand,
            together(vec![
                spatial_push(41, 4, 6, 1)?,
                result_frame(42, 7, StepDisposition::Blocked)?,
            ])?,
        ]);
        let mut session = Session::admit(client, admission()?).await?;
        assert!(matches!(
            session.step(StepDirection::East).await,
            Err(SessionError::StateRevisionMismatch { .. })
        ));
        assert!(matches!(
            session.step(StepDirection::East).await,
            Err(SessionError::SessionUnusable)
        ));
        drop(session);
        peer.await??;
        Ok(())
    })?
}

fn queue_script(count: usize) -> Result<Vec<Step>, BoxError> {
    (0..count)
        .map(|index| {
            let index = u64::try_from(index)?;
            vitals_push(41 + index, index, index + 1, 100)
        })
        .collect()
}

#[test]
fn the_event_queue_holds_exactly_the_limit_and_fails_closed_one_past_it() -> Result<(), BoxError> {
    block_on(async {
        let (client, peer) = script_peer(queue_script(MAX_QUEUED_EVENTS)?);
        let mut session = Session::admit(client, admission()?).await?;
        session.service_liveness(Duration::from_millis(500)).await?;
        assert_eq!(session.take_events().len(), MAX_QUEUED_EVENTS);
        drop(session);
        peer.await??;

        let (client, peer) = script_peer(queue_script(MAX_QUEUED_EVENTS + 1)?);
        let mut session = Session::admit(client, admission()?).await?;
        assert!(matches!(
            session.service_liveness(Duration::from_millis(500)).await,
            Err(SessionError::EventQueueOverflow {
                limit: MAX_QUEUED_EVENTS
            })
        ));
        assert!(matches!(
            session.step(StepDirection::East).await,
            Err(SessionError::SessionUnusable)
        ));
        drop(session);
        peer.await??;
        Ok(())
    })?
}
