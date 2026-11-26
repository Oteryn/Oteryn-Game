//! Session-bound command replies for the N03 and I07 client captures.
use super::{TestResult, uuid_text};

pub(super) fn encode(
    session: &[u8; 16],
    command_id: u64,
    server_sequence: u64,
    status: u64,
    payload: &[u8],
) -> TestResult<Vec<u8>> {
    if session[6] >> 4 != 7 || session[8] >> 6 != 2 {
        return Err("capture game_session_id must be an RFC UUIDv7".into());
    }
    Ok(serde_json::to_vec(&serde_json::json!({
        "command_id": command_id,
        "game_session_id": uuid_text(session),
        "payload": payload,
        "server_sequence": server_sequence,
        "status": status,
    }))?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_envelope_matches_client_contract() -> TestResult {
        let mut session = [0; 16];
        session[6] = 0x70;
        for (variant, digit) in [(0x80, '8'), (0x90, '9'), (0xa0, 'a'), (0xb0, 'b')] {
            session[8] = variant;
            let bytes = encode(&session, 17, 31, 1, &[0, 255])?;
            // Exact serialized text catches repeated and unknown keys before parsing can hide them.
            assert_eq!(
                std::str::from_utf8(&bytes)?,
                format!(r#"{{"command_id":17,"game_session_id":"00000000-0000-7000-{digit}000-000000000000","payload":[0,255],"server_sequence":31,"status":1}}"#),
            );
        }
        for (version, variant) in [(0x40, 0x80), (0x70, 0x00), (0x70, 0xc0)] {
            session[6] = version;
            session[8] = variant;
            assert!(encode(&session, 17, 31, 1, &[]).is_err());
        }
        Ok(())
    }
}
