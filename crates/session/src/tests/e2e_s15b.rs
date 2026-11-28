//! Re-decode the original logout response envelopes from the selected live journey.
use super::*;
use oteryn_protocol_oteryn::{decode_command_result, logout::LogoutResult};

#[test]
#[ignore = "requires the current node-boot S15b wire capture"]
fn e2e_s15b_in_fight_logout_refusal_keeps_the_session_playing() -> Result<(), BoxError> {
    let root = std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("S15b requires a live capture")?;
    let root = std::path::Path::new(&root);
    let manifest = std::fs::read_to_string(root.join("s15b-expected.txt"))?;
    let fields: Vec<_> = manifest.split_whitespace().collect();
    assert_eq!(fields.join(","), std::env::var("OTERYN_S15B_CAPTURE_KEY")?);
    let [
        session,
        rejected_id,
        rejected_sequence,
        accepted_id,
        accepted_sequence,
        retry,
    ] = fields.as_slice()
    else {
        return Err("invalid S15b capture manifest".into());
    };
    assert_eq!(session.len(), 36);
    let refused_id = rejected_id.parse::<u64>()?;
    let accepted_id = accepted_id.parse::<u64>()?;
    let refused_sequence = rejected_sequence.parse::<u64>()?;
    let accepted_sequence = accepted_sequence.parse::<u64>()?;
    assert!(refused_id < accepted_id && refused_sequence < accepted_sequence);
    let refused = std::fs::read(root.join("s15b-refused.bin"))?;
    let accepted = std::fs::read(root.join("s15b-accepted.bin"))?;
    let refused = decode_wire_envelope(&refused)?;
    let accepted = decode_wire_envelope(&accepted)?;
    assert_eq!(
        refused.connection_generation(),
        accepted.connection_generation()
    );
    for (frame, id, sequence, status, expected) in [
        (
            refused,
            refused_id,
            refused_sequence,
            CommandStatus::Rejected,
            LogoutResult::InFight {
                retry_after_ms: retry.parse()?,
            },
        ),
        (
            accepted,
            accepted_id,
            accepted_sequence,
            CommandStatus::Accepted,
            LogoutResult::Accepted,
        ),
    ] {
        assert_eq!(frame.message_type(), MessageType::CommandResult);
        assert_eq!(frame.server_sequence(), sequence);
        let result = decode_command_result(frame.payload())?;
        assert_eq!((result.command_id, result.status), (id, status));
        assert_eq!(
            logout::decode_logout_result(result.payload).map_err(|err| format!("{err:?}"))?,
            expected
        );
    }
    Ok(())
}
