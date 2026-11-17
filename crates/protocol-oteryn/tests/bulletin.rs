use oteryn_protocol_oteryn::{bulletin::*, *};
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn accepted() -> Result<Vec<u8>, FoundationProtocolError> {
    let id = [1, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1];
    encode_server_accepted(&ServerAcceptedValue {
        game_session_id: GameSessionId::decode(&id)?,
        world_id: WorldId::decode(&id)?,
        channel_id: ChannelId::decode(&id)?,
        connection_generation: 1,
        current_server_sequence: 0,
        next_command_id: 1,
        schema_revision: 1,
        selected_capabilities: &[],
    })
}
#[test]
fn bounded_bulletin_preserves_legacy_acceptance_and_rejects_duplicates() -> TestResult {
    let frame = accepted()?;
    let legacy = decode_wire_envelope(&frame)?;
    assert_eq!(document_from_acceptance(legacy.payload())?, None);
    let document = r#"{"news":[],"events":[]}"#;
    let with_feed = append_to_acceptance(&frame, document)?;
    let envelope = decode_wire_envelope(&with_feed)?;
    assert_eq!(
        document_from_acceptance(envelope.payload())?,
        Some(document)
    );
    assert_eq!(
        decode_server_accepted(envelope.payload())?.connection_generation,
        1
    );
    assert!(append_to_acceptance(&with_feed, document).is_err());
    for invalid in [
        String::new(),
        "x".repeat(MAX_DOCUMENT_BYTES + 1),
        "nul\0byte".into(),
    ] {
        assert!(append_to_acceptance(&frame, &invalid).is_err());
    }
    assert!(document_from_acceptance(&[0xaa, 1, 1, 0xff]).is_err());
    assert!(document_from_acceptance(&[0xa8, 1, 1]).is_err());
    assert!(document_from_acceptance(&[0xaa, 1, 1, b'a', 0xaa, 1, 1, b'b']).is_err());
    Ok(())
}
