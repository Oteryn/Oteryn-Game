use super::*;
fn request() -> Request {
    Request {
        action: Action::Mute {
            character_id: "2902030405067008800a0b0c0d0e0f29".into(),
            duration_ms: MAX_MUTE_MS.to_string(),
        },
        activation_sha256: "a".repeat(64),
        channel_id: "2b02030405067008800a0b0c0d0e0f2b".into(),
        control_seq: "2".into(),
        environment: "qualification".into(),
        issued_at: "1000000".into(),
        operation_id: "5a02030405067008800a0b0c0d0e0f5a".into(),
        previous_link_sha256: "a".repeat(64),
        reason: Reason::Support,
        roster_sha256: "b".repeat(64),
        world_id: "2a02030405067008800a0b0c0d0e0f2a".into(),
    }
}
#[test]
fn staff_rules_scope_mute_and_trusted_window_boundaries() {
    let r = request();
    assert!(r.valid(StaffRole::Gm));
    assert!(r.fresh(700_000, 700_000));
    assert!(!r.fresh(699_999, 699_999));
    assert!(r.fresh(1_600_000, 1_600_000));
    assert!(!r.fresh(1_600_000, 1_600_001));
    let mut bad = r.clone();
    bad.channel_id = bad.world_id.clone();
    assert!(!bad.valid(StaffRole::Admin));
    let mut bad = r.clone();
    bad.control_seq = "02".into();
    assert!(!bad.valid(StaffRole::Gm));
    for duration in [
        "0".to_string(),
        "01".into(),
        (MAX_MUTE_MS + 1).to_string(),
        u64::MAX.to_string(),
    ] {
        let mut bad = r.clone();
        bad.action = Action::Mute {
            character_id: "2902030405067008800a0b0c0d0e0f29".into(),
            duration_ms: duration,
        };
        assert!(!bad.valid(StaffRole::Gm));
    }
}
#[test]
fn staff_rules_closed_signed_grammar() -> Result<(), Box<dyn std::error::Error>> {
    let value = serde_json::to_value(request())?;
    for field in ["unknown", "reason", "issued_at", "action"] {
        let mut invalid = value.clone();
        invalid[field] = match field {
            "reason" => serde_json::json!("OTHER"),
            "issued_at" => serde_json::json!(1000000),
            "action" => {
                serde_json::json!({"kind":"TELEPORT","character_id":"2902030405067008800a0b0c0d0e0f29"})
            }
            _ => serde_json::json!("unknown"),
        };
        assert!(serde_json::from_value::<Request>(invalid).is_err());
    }
    assert!(serde_json::from_str::<Action>(r#"{"kind":"INSPECT","kind":"MUTE","character_id":"2902030405067008800a0b0c0d0e0f29","duration_ms":"1"}"#).is_err());
    assert!(serde_json::from_str::<StaffRole>("\"PLAYER\"").is_err());
    Ok(())
}
