use super::*;
fn request() -> Request {
    Request {
        issuer_character_id: None,
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

#[test]
fn staff_rules_accept_a_bounded_kick_target() {
    let mut request = request();
    request.action = Action::Kick {
        character_id: "2902030405067008800a0b0c0d0e0f29".into(),
    };
    assert!(request.valid(StaffRole::Gm));
    request.action = Action::Kick {
        character_id: "00000000000000000000000000000000".into(),
    };
    assert!(!request.valid(StaffRole::Admin));
}

fn tools_actions() -> Vec<Action> {
    let target = "2902030405067008800a0b0c0d0e0f29".to_owned();
    vec![
        Action::Ban {
            character_id: target.clone(),
            duration_ms: "86400000".into(),
            reason: "Rule violation".into(),
        },
        Action::Unban {
            character_id: target.clone(),
        },
        Action::Teleport {
            character_id: target.clone(),
            x: 100,
            y: 200,
            floor: 7,
        },
        Action::TeleportToPlayer {
            character_id: target.clone(),
            destination_character_id: "2a02030405067008800a0b0c0d0e0f2a".into(),
        },
        Action::Bring {
            character_id: target.clone(),
        },
        Action::Notify {
            character_id: target.clone(),
            text: "Please contact support.".into(),
        },
        Action::Broadcast {
            character_id: target.clone(),
            text: "Server maintenance in five minutes.".into(),
        },
        Action::Ghost {
            character_id: target.clone(),
            enabled: true,
        },
        Action::Invisible {
            character_id: target,
            enabled: false,
        },
    ]
}

#[test]
fn nx1_donor_staff_catalogue_and_permission_tiers() {
    let mut request = request();
    let mut gm_actions = tools_actions();
    gm_actions.extend([
        Action::Inspect {
            character_id: "2902030405067008800a0b0c0d0e0f29".into(),
        },
        Action::Kick {
            character_id: "2902030405067008800a0b0c0d0e0f29".into(),
        },
    ]);
    for action in gm_actions {
        assert_eq!(
            action.is_extended(),
            !matches!(action, Action::Inspect { .. } | Action::Kick { .. })
        );
        assert_eq!(action.minimum_role(), StaffRole::Gm);
        assert!(!action.kind().is_empty());
        request.action = action;
        for role in [StaffRole::Gm, StaffRole::SeniorGm, StaffRole::Admin] {
            assert!(request.valid_tools(role));
        }
    }
    for action in [
        Action::Mute {
            character_id: "2902030405067008800a0b0c0d0e0f29".into(),
            duration_ms: "1".into(),
        },
        Action::Unmute {
            character_id: "2902030405067008800a0b0c0d0e0f29".into(),
            sanction_id: "5a02030405067008800a0b0c0d0e0f5a".into(),
        },
    ] {
        assert_eq!(action.minimum_role(), StaffRole::Admin);
        assert!(!action.is_extended());
        request.action = action;
        assert!(!request.valid_tools(StaffRole::Gm));
        assert!(!request.valid_tools(StaffRole::SeniorGm));
        assert!(request.valid_tools(StaffRole::Admin));
        // Historical pre-NX-1 GM mute signatures retain their shape validation.
        assert!(request.valid(StaffRole::Gm));
    }
}

#[test]
fn nx1_staff_tools_refuse_unbounded_inputs_and_bad_identities() {
    let mut request = request();
    let target = "2902030405067008800a0b0c0d0e0f29";
    for duration in [
        "0".into(),
        "01".into(),
        (MAX_BAN_MS + 1).to_string(),
        u64::MAX.to_string(),
    ] {
        request.action = Action::Ban {
            character_id: target.into(),
            duration_ms: duration,
            reason: "Rule violation".into(),
        };
        assert!(!request.valid_tools(StaffRole::Admin));
    }
    for text in [
        String::new(),
        " ".into(),
        "a".repeat(MAX_STAFF_TEXT_BYTES + 1),
        "line\nfeed".into(),
        "\u{7f}".into(),
        "zażółć".into(),
    ] {
        for action in [
            Action::Ban {
                character_id: target.into(),
                duration_ms: "1".into(),
                reason: text.clone(),
            },
            Action::Notify {
                character_id: target.into(),
                text: text.clone(),
            },
            Action::Broadcast {
                character_id: target.into(),
                text,
            },
        ] {
            request.action = action;
            assert!(!request.valid_tools(StaffRole::Admin));
        }
    }
    for (x, y, floor) in [
        (-1, 0, 0),
        (0, -1, 0),
        (65536, 0, 0),
        (0, 65536, 0),
        (0, 0, -1),
        (0, 0, 16),
    ] {
        request.action = Action::Teleport {
            character_id: target.into(),
            x,
            y,
            floor,
        };
        assert!(!request.valid_tools(StaffRole::Admin));
    }
    for destination in ["0".repeat(32), "A".repeat(32), "ff".into()] {
        request.action = Action::TeleportToPlayer {
            character_id: target.into(),
            destination_character_id: destination,
        };
        assert!(!request.valid_tools(StaffRole::Admin));
    }
    for mut action in tools_actions() {
        // Validate the common target guard independently of each action's payload.
        let mut value = serde_json::to_value(&action).unwrap_or_default();
        value["character_id"] = serde_json::json!("0".repeat(32));
        if let Ok(invalid) = serde_json::from_value(value) {
            action = invalid;
        }
        request.action = action;
        assert!(!request.valid_tools(StaffRole::Admin));
    }
}

#[test]
fn nx1_staff_tools_accept_exact_limits_and_retain_canonical_grammar()
-> Result<(), Box<dyn std::error::Error>> {
    let mut request = request();
    request.action = Action::Ban {
        character_id: "2902030405067008800a0b0c0d0e0f29".into(),
        duration_ms: MAX_BAN_MS.to_string(),
        reason: "a".repeat(MAX_STAFF_TEXT_BYTES),
    };
    assert!(request.valid_tools(StaffRole::Gm));
    assert_eq!(request.action.duration_ms(), MAX_BAN_MS);
    assert_eq!(request.action.kind(), "BAN");
    request.issued_at = (i64::MAX as u64 - MAX_BAN_MS + 1).to_string();
    assert!(!request.valid_tools(StaffRole::Admin));
    request.issued_at = "1000000".into();
    request.action = Action::Teleport {
        character_id: "2902030405067008800a0b0c0d0e0f29".into(),
        x: 65535,
        y: 65535,
        floor: 15,
    };
    assert!(request.valid_tools(StaffRole::Gm));
    assert_eq!(request.action.duration_ms(), 0);
    assert_eq!(
        request.action.character_id(),
        "2902030405067008800a0b0c0d0e0f29"
    );
    let mut value = serde_json::to_value(request)?;
    value["action"]["x"] = serde_json::json!("65535");
    assert!(serde_json::from_value::<Request>(value).is_err());
    assert_eq!(
        serde_json::from_str::<StaffRole>("\"CM\"")?,
        StaffRole::SeniorGm
    );
    assert_eq!(
        serde_json::from_str::<StaffRole>("\"GOD\"")?,
        StaffRole::Admin
    );
    Ok(())
}

#[test]
fn nx1_staff_notices_match_existing_native_message_limit_without_truncation() {
    let mut r = request();
    for broadcast in [false, true] {
        for (length, accepted) in [
            (MAX_STAFF_NOTICE_BYTES, true),
            (MAX_STAFF_NOTICE_BYTES + 1, false),
        ] {
            let text = "x".repeat(length);
            r.action = if broadcast {
                Action::Broadcast {
                    character_id: r.action.character_id().into(),
                    text,
                }
            } else {
                Action::Notify {
                    character_id: r.action.character_id().into(),
                    text,
                }
            };
            assert_eq!(r.valid_tools(StaffRole::Gm), accepted);
        }
    }
}
