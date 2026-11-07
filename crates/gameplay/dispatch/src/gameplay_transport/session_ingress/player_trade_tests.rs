use super::super::connection::{FirstEntryOutcome, SessionContinuity};
use super::*;
use crate::foundation::{ChannelId, WorldId};
#[test]
fn player_trade_first_accept_waits_for_terminal_preflight_and_unknown_swap_recovers()
-> Result<(), Box<dyn std::error::Error>> {
    let id = |tag| {
        [
            tag, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, tag,
        ]
    };
    let mut admitted = AdmittedSession {
        game_session_id: GameSessionId::decode(&id(1))?,
        world_id: WorldId::decode(&id(2))?,
        channel_id: ChannelId::decode(&id(3))?,
        runtime_actor: None,
        first_entry: FirstEntryOutcome::NotApplicable,
        controller: None,
        continuity: SessionContinuity::FRESH,
        item_fence: None,
    };
    let commands = SessionCommands::default();
    let mut stream = SessionCommandStream::open(Some(&commands), &admitted).ok_or("open")?;
    assert!(matches!(
        stream.begin(1, 1, 0, false),
        Ok(CommandAction::Execute)
    ));
    let mut installed = false;
    let result = oteryn_protocol_oteryn::encode_command_result(
        1,
        1,
        2,
        oteryn_protocol_oteryn::CommandStatus::Accepted,
        &[8, 2],
    )?;
    assert!(
        stream
            .terminalize_with(2, &result, || installed = true)
            .is_none()
    );
    assert!(!installed);
    let result = oteryn_protocol_oteryn::encode_command_result(
        1,
        1,
        1,
        oteryn_protocol_oteryn::CommandStatus::Accepted,
        &[8, 2],
    )?;
    stream
        .terminalize_with(1, &result, || installed = true)
        .ok_or("terminal first accept")?;
    assert!(installed);
    // A verified final writer verdict completes the original reserved second accept.
    assert!(matches!(
        stream.begin(2, 1, 1, false),
        Ok(CommandAction::Execute)
    ));
    admitted.continuity.next_command_id = 3;
    admitted.continuity.server_sequence = 1;
    drop(stream);
    assert!(!commands.resumable(&admitted));
    commands
        .complete_trade(
            crate::foundation::CommandRef::new(admitted.game_session_id, CommandId::new(2)?),
            oteryn_protocol_oteryn::player_trade::ResultCode::Completed,
        )
        .ok_or("complete original")?;
    commands
        .recover_trade_continuity(&mut admitted)
        .ok_or("recover verified result")?;
    assert_eq!(admitted.continuity.server_sequence, 2);
    commands
        .recover_trade_continuity(&mut admitted)
        .ok_or("repeated continuity recovery")?;
    assert!(commands.resumable(&admitted));
    admitted.continuity.connection_generation = 2;
    let mut stream = SessionCommandStream::open(Some(&commands), &admitted).ok_or("resume")?;
    let CommandAction::Replay(frame) = stream.begin(2, 2, 2, false).map_err(|_| "replay")? else {
        return Err("must replay original second accept".into());
    };
    let envelope = oteryn_protocol_oteryn::decode_wire_envelope(&frame)?;
    let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
    assert_eq!(
        result.status,
        oteryn_protocol_oteryn::CommandStatus::DuplicateReplay
    );
    assert_eq!(result.payload, [8, 3]);
    assert_eq!(envelope.connection_generation(), 2);
    assert!(matches!(
        stream.begin(3, 2, 3, false),
        Ok(CommandAction::Execute)
    ));
    admitted.continuity.next_command_id = 4;
    admitted.continuity.server_sequence = 3;
    commands
        .complete_trade(
            crate::foundation::CommandRef::new(admitted.game_session_id, CommandId::new(3)?),
            oteryn_protocol_oteryn::player_trade::ResultCode::Rejected,
        )
        .ok_or("resolve refusal")?;
    {
        let directory = commands.0.lock().map_err(|_| "directory")?;
        let retained = directory
            .get(&admitted.game_session_id)
            .ok_or("state")?
            .lock()
            .map_err(|_| "state lock")?;
        let original = oteryn_protocol_oteryn::decode_wire_envelope(
            retained.retained.as_deref().ok_or("retained refusal")?,
        )?;
        assert_eq!(
            oteryn_protocol_oteryn::decode_command_result(original.payload())?.status,
            oteryn_protocol_oteryn::CommandStatus::Rejected
        );
    }
    drop(stream);
    commands
        .recover_trade_continuity(&mut admitted)
        .ok_or("recover refusal")?;
    admitted.continuity.connection_generation = 3;
    let mut stream =
        SessionCommandStream::open(Some(&commands), &admitted).ok_or("resume refusal")?;
    let CommandAction::Replay(frame) =
        stream.begin(3, 3, 4, false).map_err(|_| "replay refusal")?
    else {
        return Err("must replay original refusal".into());
    };
    let envelope = oteryn_protocol_oteryn::decode_wire_envelope(&frame)?;
    let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
    assert_eq!(
        result.status,
        oteryn_protocol_oteryn::CommandStatus::DuplicateReplay
    );
    assert_eq!(result.payload, [8, 12]);
    Ok(())
}
