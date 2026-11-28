use oteryn_protocol_oteryn::{
    self as wire, CommandStatus, FrameLength, MessageType, presentation, tutorial_notices,
    world_object,
};
use std::error::Error;

fn frames(mut bytes: &[u8]) -> Result<Vec<wire::WireEnvelopeView<'_>>, Box<dyn Error>> {
    let mut frames = Vec::new();
    while !bytes.is_empty() {
        let size =
            FrameLength::from_prefix(bytes.get(..4).ok_or("truncated prefix")?)?.get() as usize;
        let (frame, rest) = bytes.split_at_checked(4 + size).ok_or("truncated frame")?;
        frames.push(wire::decode_framed_envelope(frame)?);
        bytes = rest;
    }
    Ok(frames)
}

#[test]
fn e2e_w09_replays_aid_clearance() -> Result<(), Box<dyn Error>> {
    let root = std::path::PathBuf::from(
        std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("missing live capture directory")?,
    );
    let inbound = std::fs::read(root.join("w09-aid-server.bin"))?;
    let outbound = std::fs::read(root.join("w09-aid-client.bin"))?;
    let session = std::fs::read(root.join("w09-aid-session.bin"))?;
    wire::GameSessionId::decode(&session)?;
    let mut outgoing = frames(&outbound)?
        .into_iter()
        .filter(|frame| frame.message_type() == MessageType::ClientCommand);
    let request = outgoing.next().ok_or("missing captured USE")?;
    assert!(
        outgoing.next().is_none(),
        "capture must contain one command"
    );
    let generation = request.connection_generation();
    assert_eq!(
        request.client_command_session_id()?,
        Some(session.as_slice())
    );
    let command = request.client_command(generation)?;
    assert_eq!(command.command_type, world_object::COMMAND_TYPE_USE_INTENT);

    let mut result_sequence = None;
    let mut clear = false;
    let mut notice = false;
    for frame in frames(&inbound)? {
        assert_eq!(frame.connection_generation(), generation);
        match frame.message_type() {
            MessageType::CommandResult => {
                let result = wire::decode_command_result(frame.payload())?;
                assert!(clear, "Aid clearance must precede the USE result");
                assert!(result_sequence.replace(frame.server_sequence()).is_none());
                assert_eq!(result.command_id, command.command_id);
                assert_eq!(result.status, CommandStatus::Accepted);
                let (body, _) = wire::refusal_detail::split_refusal_detail(result.payload)
                    .map_err(|error| format!("{error:?}"))?;
                assert_eq!(
                    world_object::decode_use_result(body).map_err(|error| format!("{error:?}"))?,
                    world_object::UseDisposition::Committed
                );
            }
            MessageType::StateDelta => {
                let delta = wire::decode_state_delta(frame.payload())?;
                if delta.domain_id == presentation::STATE_DOMAIN_ACTOR_COOLDOWNS {
                    assert!(result_sequence.is_none());
                    assert_eq!(
                        delta.delta_type,
                        presentation::DELTA_TYPE_ACTOR_COOLDOWNS_SPARSE_V1
                    );
                    let update = presentation::decode_actor_cooldowns(delta.payload)
                        .map_err(|error| format!("{error:?}"))?;
                    assert_eq!(update.server_now_ms, 0);
                    assert_eq!(update.entries.len(), 4);
                    assert!(update.entries.iter().all(|entry| entry.expires_at_ms == 0));
                    assert!(
                        update
                            .entries
                            .iter()
                            .any(|entry| entry.kind == presentation::ActorCooldownKind::Group
                                && entry.id == 3)
                    );
                    assert!(!clear);
                    clear = true;
                } else if delta.domain_id == tutorial_notices::STATE_DOMAIN_TUTORIAL_NOTICES
                    && tutorial_notices::decode_tutorial_notice(delta.payload)
                        .map_err(|error| format!("{error:?}"))?
                        == tutorial_notices::TutorialNotice::StatusText(
                            "Your spells are no longer on cooldown.".into(),
                        )
                {
                    assert!(result_sequence.is_some_and(|seq| frame.server_sequence() > seq));
                    assert!(!notice);
                    notice = true;
                }
            }
            _ => {}
        }
    }
    assert!(result_sequence.is_some() && clear && notice);
    let before = gift(&root, "before", &session)?;
    let after = gift(&root, "after", &session)?;
    let relog_session = std::fs::read(root.join("w09-gift-relog-session.bin"))?;
    assert_ne!(relog_session, session);
    let relog = gift(&root, "relog", &relog_session)?;
    assert_eq!([before, after, relog], [3_600_000; 3]);
    Ok(())
}

fn gift(
    root: &std::path::Path,
    label: &str,
    expected_session: &[u8],
) -> Result<u64, Box<dyn Error>> {
    let session = std::fs::read(root.join(format!("w09-gift-{label}-session.bin")))?;
    wire::GameSessionId::decode(&session)?;
    assert_eq!(session, expected_session);
    let outgoing = std::fs::read(root.join(format!("w09-gift-{label}-client.bin")))?;
    let incoming = std::fs::read(root.join(format!("w09-gift-{label}-server.bin")))?;
    let requests = frames(&outgoing)?
        .into_iter()
        .filter(|frame| frame.message_type() == MessageType::ClientCommand)
        .collect::<Vec<_>>();
    let [request] = requests.as_slice() else {
        return Err("expected exactly one Gift status command".into());
    };
    assert_eq!(request.client_command_session_id()?, Some(expected_session));
    let generation = request.connection_generation();
    let command = request.client_command(generation)?;
    assert_eq!(
        command.command_type,
        wire::wheel_presentation::COMMAND_TYPE_WHEEL_GIFT_OF_LIFE_STATUS
    );
    assert!(command.payload.is_empty());
    let results = frames(&incoming)?
        .into_iter()
        .filter(|frame| frame.message_type() == MessageType::CommandResult)
        .collect::<Vec<_>>();
    let [result] = results.as_slice() else {
        return Err("expected exactly one Gift status result".into());
    };
    assert_eq!(result.connection_generation(), generation);
    let result = wire::decode_command_result(result.payload())?;
    assert_eq!(result.command_id, command.command_id);
    assert_eq!(result.status, CommandStatus::Accepted);
    assert_eq!(
        result.payload.len(),
        wire::wheel_presentation::GIFT_OF_LIFE_STATUS_BYTES
    );
    assert_eq!(result.payload[0], 1);
    Ok(u64::from_le_bytes(result.payload[1..].try_into()?))
}
