use super::*;
use crate::foundation::{ChannelId, WorldId};
use crate::gameplay_transport::connection::{FirstEntryOutcome, SessionContinuity};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
    encode_step_intent,
};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, decode_wire_envelope,
    encode_client_command,
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn admitted() -> TestResult<AdmittedSession> {
    let id = [1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 16];
    Ok(AdmittedSession {
        game_session_id: GameSessionId::decode(&id)?,
        world_id: WorldId::decode(&id)?,
        channel_id: ChannelId::decode(&id)?,
        runtime_actor: None,
        first_entry: FirstEntryOutcome::NotApplicable,
        controller: None,
        continuity: SessionContinuity::FRESH,
        item_fence: None,
    })
}

fn step(generation: u64, command_id: u64, command_type: u32) -> TestResult<Vec<u8>> {
    Ok(encode_client_command(
        generation,
        &ClientCommandValue {
            command_id,
            command_type,
            payload: &encode_step_intent(StepDirection::West),
        },
    )?)
}

#[test]
fn buffered_step_cancellation_retains_rejection_after_intervening_pushes() -> TestResult {
    let owner = SessionCommands::default();
    let mut admitted = admitted()?;
    let mut stream = SessionCommandStream::open(Some(&owner), &admitted).ok_or("open")?;
    assert!(matches!(
        stream.begin(1, 1, 4, false),
        Ok(CommandAction::Execute)
    ));
    admitted.continuity.next_command_id = stream.next_command_id();
    // Periodic pushes advanced past the sequence captured when the step was reserved.
    admitted.continuity.server_sequence = 7;
    let frame = step(1, 1, COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT)?;
    stream
        .cancel_buffered_step(&frame, 1, 7)
        .ok_or("cancel exact buffer")?;
    assert!(
        !owner.resumable(&admitted),
        "the original stream is still active"
    );
    drop(stream);
    assert!(
        !owner.resumable(&admitted),
        "the retained result needs its sequence floor"
    );
    owner
        .recover_trade_continuity(&mut admitted)
        .ok_or("recover cancellation")?;
    assert_eq!(admitted.continuity.next_command_id, 2);
    assert_eq!(admitted.continuity.server_sequence, 8);
    assert!(owner.resumable(&admitted));
    owner
        .recover_trade_continuity(&mut admitted)
        .ok_or("idempotent recovery")?;
    assert_eq!(admitted.continuity.server_sequence, 8);

    admitted.continuity.connection_generation = 2;
    let mut resumed = SessionCommandStream::open(Some(&owner), &admitted).ok_or("resume")?;
    let CommandAction::Replay(replay) = resumed
        .begin(1, 2, 8, false)
        .map_err(|error| format!("{error:?}"))?
    else {
        return Err("cancelled step was re-executed".into());
    };
    let envelope = decode_wire_envelope(&replay)?;
    let result = decode_command_result(envelope.payload())?;
    assert_eq!(
        (envelope.connection_generation(), envelope.server_sequence()),
        (2, 9)
    );
    assert_eq!(
        (result.command_id, result.status),
        (1, CommandStatus::DuplicateReplay)
    );
    assert_eq!(
        decode_step_result(result.payload)?,
        StepDisposition::Rejected
    );
    assert!(matches!(
        resumed.begin(2, 2, 9, false),
        Ok(CommandAction::Execute)
    ));
    Ok(())
}

#[test]
fn buffered_step_cancellation_fails_closed_for_other_or_unproven_reservations() -> TestResult {
    let valid = step(1, 1, COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT)?;
    let cases = [
        (step(1, 2, COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT)?, 1, 4),
        (step(2, 1, COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT)?, 1, 4),
        (step(1, 1, 2)?, 1, 4),
        (valid.clone(), 2, 4),
        (valid.clone(), 1, 3),
        (valid, 1, u64::MAX),
    ];
    for (frame, generation, sequence) in cases {
        let owner = SessionCommands::default();
        let mut admitted = admitted()?;
        let mut stream = SessionCommandStream::open(Some(&owner), &admitted).ok_or("open")?;
        assert!(matches!(
            stream.begin(1, 1, 4, false),
            Ok(CommandAction::Execute)
        ));
        admitted.continuity.next_command_id = stream.next_command_id();
        admitted.continuity.server_sequence = 4;
        assert!(
            stream
                .cancel_buffered_step(&frame, generation, sequence)
                .is_none()
        );
        drop(stream);
        owner
            .recover_trade_continuity(&mut admitted)
            .ok_or("unchanged continuity")?;
        assert_eq!(admitted.continuity.server_sequence, 4);
        assert!(
            !owner.resumable(&admitted),
            "unknown original must remain reserved"
        );
    }
    Ok(())
}

#[test]
fn buffered_step_cancellation_never_replaces_a_proven_native_outcome() -> TestResult {
    let owner = SessionCommands::default();
    let mut admitted = admitted()?;
    let mut stream = SessionCommandStream::open(Some(&owner), &admitted).ok_or("open")?;
    assert!(matches!(
        stream.begin(1, 1, 0, false),
        Ok(CommandAction::Execute)
    ));
    let command = crate::foundation::CommandRef::new(admitted.game_session_id, CommandId::new(1)?);
    owner
        .complete_hireling_summon(command, true)
        .ok_or("native verdict")?;
    assert!(
        stream
            .cancel_buffered_step(&step(1, 1, COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT)?, 1, 0)
            .is_none()
    );
    drop(stream);
    owner
        .recover_trade_continuity(&mut admitted)
        .ok_or("original native recovery")?;
    admitted.continuity.connection_generation = 2;
    let mut resumed = SessionCommandStream::open(Some(&owner), &admitted).ok_or("resume")?;
    let CommandAction::Replay(frame) = resumed
        .begin(1, 2, 1, false)
        .map_err(|error| format!("{error:?}"))?
    else {
        return Err("native original was not retained".into());
    };
    let envelope = decode_wire_envelope(&frame)?;
    let result = decode_command_result(envelope.payload())?;
    assert_eq!(result.status, CommandStatus::DuplicateReplay);
    assert_eq!(result.payload, &[1]);
    Ok(())
}
