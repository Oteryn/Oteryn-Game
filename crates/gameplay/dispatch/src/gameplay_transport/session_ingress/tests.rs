use super::*;
use crate::foundation::{ChannelId, WorldId};
use crate::gameplay_transport::connection::{FirstEntryOutcome, SessionContinuity};
use oteryn_protocol_oteryn::{CommandStatus, encode_command_result};

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
