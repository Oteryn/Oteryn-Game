use super::*;

#[test]
fn raid_server_messages_retain_status_bytes_and_distinguish_warning_from_advance()
-> Result<(), ServerMessageError> {
    use crate::tutorial_notices::{TutorialNotice, encode_tutorial_notice};
    let old = encode_tutorial_notice(&TutorialNotice::StatusText("Welcome!".into()))?;
    assert_eq!(
        encode_server_message(&ServerMessage::StatusText("Welcome!".into()))?,
        old
    );
    for (message, tag) in [
        (ServerMessage::StatusText("é".into()), 0x12),
        (ServerMessage::Warning("é".into()), 0x1a),
        (ServerMessage::Advance("é".into()), 0x22),
    ] {
        let bytes = encode_server_message(&message)?;
        assert_eq!(bytes, vec![tag, 2, 0xc3, 0xa9]);
        assert_eq!(decode_server_message(&bytes)?, message);
    }
    Ok(())
}

#[test]
fn raid_server_messages_enforce_wire_limit_and_reject_ambiguous_payloads()
-> Result<(), ServerMessageError> {
    let maximum = ServerMessage::Warning("a".repeat(255));
    let encoded = encode_server_message(&maximum)?;
    assert_eq!(encoded.len(), 258);
    assert_eq!(decode_server_message(&encoded)?, maximum);
    assert!(encode_server_message(&ServerMessage::Advance("a".repeat(256))).is_err());
    assert!(encode_server_message(&ServerMessage::Warning(String::new())).is_err());
    for bytes in [
        vec![],
        vec![8, 1],
        vec![0x1a, 0],
        vec![0x1a, 1, 0xff],
        vec![0x22, 2, b'a'],
        vec![0x1a, 1, b'a', 0x22, 1, b'b'],
    ] {
        assert!(decode_server_message(&bytes).is_err());
    }
    assert!(decode_server_message(&vec![0; 261]).is_err());
    Ok(())
}

#[test]
fn boss_defeat_and_category_use_distinct_fields() -> Result<(), ServerMessageError> {
    let notice = ServerNotification {
        message: ServerMessage::BossDefeated("a".repeat(255)),
        category: ServerMessageCategory::Raid,
    };
    let mut encoded = encode_server_notification(&notice)?;
    assert_eq!(encoded.len(), 260);
    assert_eq!(encoded[0], 0x2a);
    assert_eq!(&encoded[258..], &[0x30, 2]);
    assert_eq!(decode_server_notification(&encoded)?, notice);
    encoded[258] = 0x28;
    assert_eq!(decode_server_notification(&encoded)?, notice);
    Ok(())
}
