use super::*;
#[test]
fn staff_jcs_closed_ascii_grammar_has_stable_sorted_bytes() -> Result<(), TrustError> {
    let action = rules::Action::Mute {
        character_id: "2902030405067008800a0b0c0d0e0f29".into(),
        duration_ms: "1".into(),
    };
    let golden =
        br#"{"character_id":"2902030405067008800a0b0c0d0e0f29","duration_ms":"1","kind":"MUTE"}"#;
    assert_eq!(canonical(&action)?, golden);
    assert_eq!(decode::<rules::Action>(golden)?, action);
    assert!(decode::<rules::Action>(br#"{"kind":"MUTE","character_id":"2902030405067008800a0b0c0d0e0f29","duration_ms":"1"}"#).is_err());
    assert!(decode::<rules::Action>(br#"{"character_id":"2902030405067008800a0b0c0d0e0f29","duration_ms":"1","kind":"MUTE","unknown":"x"}"#).is_err());
    assert!(decode::<rules::Action>(br#"{"character_id":"2902030405067008800a0b0c0d0e0f29","duration_ms":"1","kind":"MUTE","kind":"MUTE"}"#).is_err());
    Ok(())
}
