use super::*;
use crate::foundation::{ChannelId, WorldId};
use crate::gameplay_transport::connection::{FirstEntryOutcome, SessionContinuity};
use oteryn_protocol_oteryn::{CommandStatus, encode_command_result};

#[test]
fn activity_recovery_excludes_late_stream_open_and_releases_on_drop()
-> Result<(), Box<dyn std::error::Error>> {
    let owner = SessionCommands::default();
    let admitted = session()?;
    assert!(
        owner.reserve_activity_recovery(&admitted).is_none(),
        "absence is not unused-stream proof"
    );
    owner.admit(admitted.game_session_id);
    let held = owner
        .reserve_activity_recovery(&admitted)
        .ok_or("reserve")?;
    assert!(owner.reserve_activity_recovery(&admitted).is_none());
    assert!(SessionCommandStream::open(Some(&owner), &admitted).is_none());
    assert!(!owner.resumable(&admitted));
    drop(held);
    let stream =
        SessionCommandStream::open(Some(&owner), &admitted).ok_or("original stream preserved")?;
    assert_eq!(stream.next_command_id(), 1);
    assert!(
        owner.reserve_activity_recovery(&admitted).is_none(),
        "live socket keeps its claim"
    );
    drop(stream);
    assert!(owner.reserve_activity_recovery(&admitted).is_some());
    Ok(())
}

#[test]
fn activity_recovery_never_erases_command_or_result_truth() -> Result<(), Box<dyn std::error::Error>>
{
    let owner = SessionCommands::default();
    let mut admitted = session()?;
    let mut stream = SessionCommandStream::open(Some(&owner), &admitted).ok_or("open")?;
    assert!(matches!(
        stream.begin(1, 1, 0, false),
        Ok(CommandAction::Execute)
    ));
    let result = encode_command_result(1, 1, 1, CommandStatus::Accepted, &[9])?;
    stream.terminalize(1, &result).ok_or("terminal")?;
    drop(stream);
    assert!(
        owner.reserve_activity_recovery(&admitted).is_none(),
        "forged initial counter cannot erase a result"
    );
    admitted.continuity.next_command_id = 2;
    admitted.continuity.server_sequence = 1;
    assert!(owner.reserve_activity_recovery(&admitted).is_none());
    assert!(
        owner.resumable(&admitted),
        "ordinary reconnect remains available"
    );
    let mut resumed = SessionCommandStream::open(Some(&owner), &admitted).ok_or("resume")?;
    assert!(matches!(
        resumed.begin(1, 2, 1, false),
        Ok(CommandAction::Replay(_))
    ));
    Ok(())
}

fn session() -> Result<AdmittedSession, Box<dyn std::error::Error>> {
    fn id(seed: u8) -> [u8; 16] {
        let mut bytes = [seed; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        bytes
    }
    Ok(AdmittedSession {
        game_session_id: GameSessionId::decode(&id(1))?,
        world_id: WorldId::decode(&id(2))?,
        channel_id: ChannelId::decode(&id(3))?,
        runtime_actor: None,
        first_entry: FirstEntryOutcome::NotApplicable,
        controller: None,
        continuity: SessionContinuity::FRESH,
        item_fence: None,
    })
}

#[test]
fn session_ingress_reserves_terminalizes_evicts_and_recovers()
-> Result<(), Box<dyn std::error::Error>> {
    let owner = SessionCommands::default();
    let mut admitted = session()?;
    let mut stream = SessionCommandStream::open(Some(&owner), &admitted).ok_or("open")?;
    // A failed second open must not release the first connection's ownership.
    assert!(SessionCommandStream::open(Some(&owner), &admitted).is_none());
    assert!(SessionCommandStream::open(Some(&owner), &admitted).is_none());
    assert!(matches!(
        stream.begin(3, 1, 0, false),
        Err((FoundationProtocolError::CommandSequenceGap, 1))
    ));
    assert!(matches!(
        stream.begin(1, 1, 0, false),
        Ok(CommandAction::Execute)
    ));
    assert_eq!(stream.next_command_id(), 2);
    assert!(matches!(
        stream.begin(1, 1, 0, false),
        Ok(CommandAction::Pending)
    ));
    assert!(matches!(
        stream.begin(1, 1, 0, true),
        Ok(CommandAction::Execute)
    ));
    let result = encode_command_result(1, 1, 1, CommandStatus::Accepted, &[7])?;
    stream.terminalize(1, &result).ok_or("terminalize")?;
    admitted.continuity.next_command_id = 2;
    admitted.continuity.server_sequence = 1;
    assert!(!owner.resumable(&admitted));
    drop(stream);
    assert!(owner.resumable(&admitted));
    let mut stale_sequence = admitted;
    stale_sequence.continuity.server_sequence = 0;
    assert!(!owner.resumable(&stale_sequence));
    assert!(SessionCommandStream::open(Some(&owner), &stale_sequence).is_none());
    admitted.continuity.connection_generation = 2;
    let mut stream = SessionCommandStream::open(Some(&owner), &admitted).ok_or("recover")?;
    assert!(matches!(
        stream.begin(1, 2, 9, false),
        Ok(CommandAction::Replay(_))
    ));
    assert!(matches!(
        stream.begin(2, 2, 10, false),
        Ok(CommandAction::Execute)
    ));
    let result = encode_command_result(2, 11, 2, CommandStatus::Rejected, &[])?;
    stream.terminalize(2, &result).ok_or("second terminal")?;
    assert!(matches!(
        stream.begin(1, 2, 11, false),
        Err((FoundationProtocolError::CommandOutcomeExpired, 0))
    ));
    assert!(matches!(
        stream.begin(2, 2, 11, false),
        Ok(CommandAction::Replay(_))
    ));
    drop(stream);
    admitted.continuity.next_command_id = 3;
    owner.forget(admitted.game_session_id);
    assert!(SessionCommandStream::open(Some(&owner), &admitted).is_none());
    assert!(!owner.resumable(&admitted));
    Ok(())
}

#[test]
fn session_ingress_unknown_pending_work_cannot_resume_or_reexecute()
-> Result<(), Box<dyn std::error::Error>> {
    let owner = SessionCommands::default();
    let mut admitted = session()?;
    let mut stream = SessionCommandStream::open(Some(&owner), &admitted).ok_or("open")?;
    assert!(matches!(
        stream.begin(1, 1, 0, false),
        Ok(CommandAction::Execute)
    ));
    admitted.continuity.next_command_id = stream.next_command_id();
    drop(stream); // Cancellation or an unproven family result preserves the pending original.
    assert!(!owner.resumable(&admitted));
    assert!(SessionCommandStream::open(Some(&owner), &admitted).is_none());
    Ok(())
}

#[test]
fn discarded_step_recovery_fences_generation_and_uses_the_current_egress_cursor()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_protocol_oteryn::world_spatial::{
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, encode_step_intent,
        encode_step_result,
    };
    use oteryn_protocol_oteryn::{
        ClientCommandValue, decode_command_result, decode_wire_envelope, encode_client_command,
    };
    // The Channel copies the reservation's successor immediately; House copies it at reply.
    for copied_next in [1, 2] {
        let owner = SessionCommands::default();
        let mut admitted = session()?;
        let mut stream = SessionCommandStream::open(Some(&owner), &admitted).ok_or("open")?;
        assert!(matches!(
            stream.begin(1, 1, 0, false),
            Ok(CommandAction::Execute)
        ));
        let frame = encode_client_command(
            1,
            &ClientCommandValue {
                command_id: 1,
                command_type: COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
                payload: &encode_step_intent(StepDirection::East),
            },
        )?;
        let envelope = decode_wire_envelope(&frame)?;
        stream
            .buffer_step(envelope.client_command(1)?)
            .ok_or("buffer exact original")?;
        admitted.continuity.next_command_id = copied_next;
        assert!(owner.recover_trade_continuity(&mut admitted).is_none());
        assert!(!owner.resumable(&admitted), "active stream owns its result");
        drop(stream);

        // Existing periodic egress can advance after reservation while the step waits.
        admitted.continuity.server_sequence = 11;
        let mut stale = admitted;
        stale.continuity.connection_generation = 2;
        assert!(owner.recover_trade_continuity(&mut stale).is_none());
        let mut wrong_counter = admitted;
        wrong_counter.continuity.next_command_id = 3;
        assert!(owner.recover_trade_continuity(&mut wrong_counter).is_none());
        assert!(!owner.resumable(&admitted));
        owner
            .recover_trade_continuity(&mut admitted)
            .ok_or("settle original after current egress")?;
        assert_eq!(admitted.continuity.next_command_id, 2);
        assert_eq!(admitted.continuity.server_sequence, 12);
        assert!(owner.resumable(&admitted));
        {
            let commands = owner.0.lock().map_err(|_| "commands")?;
            let state = commands
                .get(&admitted.game_session_id)
                .ok_or("session")?
                .lock()
                .map_err(|_| "ingress")?;
            assert_eq!(state.ingress.outstanding(), 0);
            assert!(state.pending_delivery.is_none());
            let retained = decode_wire_envelope(state.retained.as_deref().ok_or("retained")?)?;
            let result = decode_command_result(retained.payload())?;
            assert_eq!(retained.connection_generation(), 1);
            assert_eq!(retained.server_sequence(), 12);
            assert_eq!(result.command_id, 1);
            assert_eq!(result.status, CommandStatus::Rejected);
            assert_eq!(
                result.payload,
                encode_step_result(StepDisposition::Rejected)
            );
        }
        owner
            .recover_trade_continuity(&mut admitted)
            .ok_or("idempotent recovery")?;
        assert_eq!(admitted.continuity.server_sequence, 12);
    }
    Ok(())
}

#[test]
fn buffered_diagonal_step_requires_negotiation_with_and_without_a_ledger()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES;
    use oteryn_protocol_oteryn::world_spatial::{
        CAPABILITY_DIAGONAL_MOVEMENT_V1, COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection,
        encode_step_intent,
    };
    use oteryn_protocol_oteryn::{ClientCommandValue, decode_wire_envelope, encode_client_command};
    let diagonal = SelectedCapabilities::select(
        BUNDLE_WORLD_OFFERED_CAPABILITIES,
        &[CAPABILITY_DIAGONAL_MOVEMENT_V1],
    )
    .ok_or("offered diagonal capability")?;
    assert!(diagonal.contains(CAPABILITY_DIAGONAL_MOVEMENT_V1));
    let frame = encode_client_command(
        1,
        &ClientCommandValue {
            command_id: 1,
            command_type: COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            payload: &encode_step_intent(StepDirection::NorthEast),
        },
    )?;
    let envelope = decode_wire_envelope(&frame)?;
    for selected in [SelectedCapabilities::NONE, diagonal] {
        for retained in [false, true] {
            let owner = SessionCommands::default();
            let mut admitted = session()?;
            admitted.continuity.selected_capabilities = selected;
            let mut stream = SessionCommandStream::open(retained.then_some(&owner), &admitted)
                .ok_or("open negotiated stream")?;
            assert!(matches!(
                stream.begin(1, 1, 0, false),
                Ok(CommandAction::Execute)
            ));
            assert_eq!(
                stream.buffer_step(envelope.client_command(1)?).is_some(),
                selected == diagonal
            );
            if retained {
                let commands = owner.0.lock().map_err(|_| "commands")?;
                let state = commands
                    .get(&admitted.game_session_id)
                    .ok_or("session")?
                    .lock()
                    .map_err(|_| "ingress")?;
                let pending = state.pending_delivery.ok_or("original reservation")?;
                assert_eq!(pending.selected, selected);
                assert_eq!(pending.buffered_step, selected == diagonal);
                assert_eq!(state.ingress.outstanding(), 1);
                assert!(
                    state.retained.is_none(),
                    "buffering is not a terminal result"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn buffered_step_that_reentered_execution_keeps_an_unknown_outcome_pending()
-> Result<(), Box<dyn std::error::Error>> {
    reentered_buffered_step_keeps_an_unknown_outcome_pending(
        oteryn_protocol_oteryn::world_spatial::StepDirection::East,
        SelectedCapabilities::NONE,
    )
}

#[test]
fn buffered_diagonal_step_that_reentered_execution_keeps_an_unknown_outcome_pending()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES;
    use oteryn_protocol_oteryn::world_spatial::{CAPABILITY_DIAGONAL_MOVEMENT_V1, StepDirection};
    let selected = SelectedCapabilities::select(
        BUNDLE_WORLD_OFFERED_CAPABILITIES,
        &[CAPABILITY_DIAGONAL_MOVEMENT_V1],
    )
    .ok_or("offered diagonal capability")?;
    assert!(selected.contains(CAPABILITY_DIAGONAL_MOVEMENT_V1));
    reentered_buffered_step_keeps_an_unknown_outcome_pending(StepDirection::NorthEast, selected)
}

fn reentered_buffered_step_keeps_an_unknown_outcome_pending(
    direction: oteryn_protocol_oteryn::world_spatial::StepDirection,
    selected: SelectedCapabilities,
) -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_protocol_oteryn::world_spatial::{
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, encode_step_intent,
    };
    use oteryn_protocol_oteryn::{ClientCommandValue, decode_wire_envelope, encode_client_command};
    let owner = SessionCommands::default();
    let mut admitted = session()?;
    admitted.continuity.selected_capabilities = selected;
    let mut stream = SessionCommandStream::open(Some(&owner), &admitted).ok_or("open")?;
    assert!(matches!(
        stream.begin(1, 1, 0, false),
        Ok(CommandAction::Execute)
    ));
    let frame = encode_client_command(
        1,
        &ClientCommandValue {
            command_id: 1,
            command_type: COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            payload: &encode_step_intent(direction),
        },
    )?;
    let envelope = decode_wire_envelope(&frame)?;
    stream
        .buffer_step(envelope.client_command(1)?)
        .ok_or("buffer exact original")?;
    assert!(matches!(
        stream.begin(1, 1, 0, true),
        Ok(CommandAction::Execute)
    ));
    admitted.continuity.next_command_id = stream.next_command_id();
    drop(stream);
    owner
        .recover_trade_continuity(&mut admitted)
        .ok_or("unknown continuity stays unchanged")?;
    assert_eq!(admitted.continuity.next_command_id, 2);
    assert_eq!(admitted.continuity.server_sequence, 0);
    assert!(!owner.resumable(&admitted));
    assert!(SessionCommandStream::open(Some(&owner), &admitted).is_none());
    Ok(())
}

#[test]
fn session_ingress_npc_recovery_requires_the_original_inactive_reservation()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, decode_npc_intent_result};
    for disposition in [NpcIntentDisposition::Ok, NpcIntentDisposition::Rejected] {
        let owner = SessionCommands::default();
        let mut admitted = session()?;
        let original =
            crate::foundation::CommandRef::new(admitted.game_session_id, CommandId::new(1)?);
        let other =
            crate::foundation::CommandRef::new(admitted.game_session_id, CommandId::new(2)?);
        let mut stream = SessionCommandStream::open(Some(&owner), &admitted).ok_or("open")?;
        assert!(matches!(
            stream.begin(1, 1, 0, false),
            Ok(CommandAction::Execute)
        ));
        assert!(!owner.original_terminalized(original));
        assert!(!owner.reconcilable(original));
        assert!(
            owner.complete_npc(original, disposition).is_none(),
            "active stream owns its result"
        );
        admitted.continuity.next_command_id = stream.next_command_id();
        drop(stream);
        assert!(owner.reconcilable(original));
        assert!(!owner.reconcilable(other));
        assert!(
            owner.complete_npc(other, disposition).is_none(),
            "a new command cannot consume the original"
        );
        assert!(SessionCommandStream::open(Some(&owner), &admitted).is_none());
        owner
            .complete_npc(original, disposition)
            .ok_or("original terminal verdict")?;
        assert!(owner.original_terminalized(original));
        assert!(!owner.original_terminalized(other));
        assert!(
            owner.complete_npc(original, disposition).is_none(),
            "terminalize once"
        );
        owner
            .recover_trade_continuity(&mut admitted)
            .ok_or("recovered sequence")?;
        assert_eq!(admitted.continuity.server_sequence, 1);
        assert!(owner.resumable(&admitted));
        admitted.continuity.connection_generation = 2;
        let mut resumed =
            SessionCommandStream::open(Some(&owner), &admitted).ok_or("resumed stream")?;
        let CommandAction::Replay(frame) = resumed
            .begin(1, 2, 1, false)
            .map_err(|_| "original replay")?
        else {
            return Err("resolved NPC command must replay".into());
        };
        let envelope = oteryn_protocol_oteryn::decode_wire_envelope(&frame)?;
        let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
        assert_eq!(result.status, CommandStatus::DuplicateReplay);
        assert_eq!(
            decode_npc_intent_result(result.payload).map_err(|error| format!("{error:?}"))?,
            disposition
        );
    }
    Ok(())
}

#[test]
fn session_ingress_logout_can_terminalize_after_retirement()
-> Result<(), Box<dyn std::error::Error>> {
    let owner = SessionCommands::default();
    let admitted = session()?;
    let mut stream = SessionCommandStream::open(Some(&owner), &admitted).ok_or("open")?;
    assert!(matches!(
        stream.begin(1, 1, 0, false),
        Ok(CommandAction::Execute)
    ));
    // The real logout releases durable authority and retires the Channel session before
    // dispatch can encode its final result. The active stream must retain that result.
    owner.forget(admitted.game_session_id);
    let result = encode_command_result(1, 1, 1, CommandStatus::Accepted, &[1])?;
    stream
        .terminalize(1, &result)
        .ok_or("logout result after retirement")?;
    assert!(!owner.resumable(&admitted));
    Ok(())
}
