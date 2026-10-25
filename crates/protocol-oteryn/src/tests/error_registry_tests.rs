#[test]
fn registry_error_dispositions_are_stable_and_safe() {
    assert_eq!(
        FoundationProtocolError::MalformedFrame.disposition(),
        ProtocolDisposition::TransportFatal
    );
    assert_eq!(
        FoundationProtocolError::CommandSequenceGap.disposition(),
        ProtocolDisposition::ResyncRequired
    );
    assert_eq!(
        FoundationProtocolError::SnapshotLimitExceeded.disposition(),
        ProtocolDisposition::SessionFatal
    );
    assert_eq!(
        FoundationProtocolError::AccountDataIntegrity.disposition(),
        ProtocolDisposition::OperationTerminal
    );
}

/// FND-02 §18: every code of this crate is the registry's, with the same name and default
/// disposition, and the registry holds no code the crate lacks.
#[test]
#[allow(clippy::expect_used)]
fn error_codes_match_the_registry() {
    use FoundationProtocolError as E;
    let registry: serde_json::Value =
        serde_json::from_str(crate::PROTOCOL_OTERYN_V1_REGISTRY_JSON).expect("protocol registry");
    let registered: Vec<(u64, &str, &str)> = registry["error_codes"]
        .as_array()
        .expect("error_codes")
        .iter()
        .map(|entry| {
            (
                entry["code"].as_u64().expect("code"),
                entry["name"].as_str().expect("name"),
                entry["default_disposition"].as_str().expect("disposition"),
            )
        })
        .collect();
    let crate_codes: Vec<(u64, &str, &str)> = [
        (E::MalformedFrame, "MALFORMED_FRAME"),
        (E::FrameTooLarge, "FRAME_TOO_LARGE"),
        (E::MalformedEnvelope, "MALFORMED_ENVELOPE"),
        (E::UnknownMessageType, "UNKNOWN_MESSAGE_TYPE"),
        (E::ProtocolMajorMismatch, "PROTOCOL_MAJOR_MISMATCH"),
        (E::TransportProfileMismatch, "TRANSPORT_PROFILE_MISMATCH"),
        (E::CapabilityMismatch, "CAPABILITY_MISMATCH"),
        (E::InvalidWireIdentifier, "INVALID_WIRE_IDENTIFIER"),
        (E::PayloadLimitExceeded, "PAYLOAD_LIMIT_EXCEEDED"),
        (E::StaleConnectionGeneration, "STALE_CONNECTION_GENERATION"),
        (E::CommandOutcomeExpired, "COMMAND_OUTCOME_EXPIRED"),
        (E::CommandSequenceGap, "COMMAND_SEQUENCE_GAP"),
        (
            E::TooManyOutstandingCommands,
            "TOO_MANY_OUTSTANDING_COMMANDS",
        ),
        (E::ServerSequenceGap, "SERVER_SEQUENCE_GAP"),
        (E::StateRevisionMismatch, "STATE_REVISION_MISMATCH"),
        (E::SnapshotAssemblyInvalid, "SNAPSHOT_ASSEMBLY_INVALID"),
        (E::SnapshotLimitExceeded, "SNAPSHOT_LIMIT_EXCEEDED"),
        (E::BootstrapLimitExceeded, "BOOTSTRAP_LIMIT_EXCEEDED"),
        (E::InvalidCapabilitySet, "INVALID_CAPABILITY_SET"),
        (E::AccountDataIntegrity, "ACCOUNT_DATA_INTEGRITY"),
        (E::NotSupported, "NOT_SUPPORTED"),
        (E::SnapshotExpired, "SNAPSHOT_EXPIRED"),
        (E::AdmissionGrantMalformed, "ADMISSION_GRANT_MALFORMED"),
        (
            E::AdmissionGrantAuthenticationFailed,
            "ADMISSION_GRANT_AUTHENTICATION_FAILED",
        ),
        (
            E::AdmissionGrantBindingMismatch,
            "ADMISSION_GRANT_BINDING_MISMATCH",
        ),
        (
            E::AdmissionGrantNotYetValid,
            "ADMISSION_GRANT_NOT_YET_VALID",
        ),
        (E::AdmissionGrantExpired, "ADMISSION_GRANT_EXPIRED"),
        (E::AdmissionGrantReplayed, "ADMISSION_GRANT_REPLAYED"),
        (
            E::AdmissionAttemptReconciliationRequired,
            "ADMISSION_ATTEMPT_RECONCILIATION_REQUIRED",
        ),
        (
            E::AdmissionGrantSecurityStateRevoked,
            "ADMISSION_GRANT_SECURITY_STATE_REVOKED",
        ),
        (
            E::AdmissionGrantSecurityEvidenceStale,
            "ADMISSION_GRANT_SECURITY_EVIDENCE_STALE",
        ),
        (E::AdmissionGrantRouteStale, "ADMISSION_GRANT_ROUTE_STALE"),
        (
            E::AdmissionGrantRuntimeGenerationStale,
            "ADMISSION_GRANT_RUNTIME_GENERATION_STALE",
        ),
        (E::AdmissionGrantWorldStale, "ADMISSION_GRANT_WORLD_STALE"),
        (
            E::AdmissionGrantRevisionUnsupported,
            "ADMISSION_GRANT_REVISION_UNSUPPORTED",
        ),
        (
            E::AdmissionAccountCharacterConflict,
            "ADMISSION_ACCOUNT_CHARACTER_CONFLICT",
        ),
        (
            E::AdmissionIncumbentProtected,
            "ADMISSION_INCUMBENT_PROTECTED",
        ),
        (E::AdmissionCapacityExceeded, "ADMISSION_CAPACITY_EXCEEDED"),
        (
            E::AdmissionCapabilityRequired,
            "ADMISSION_CAPABILITY_REQUIRED",
        ),
        (
            E::ReconnectCapabilityRequired,
            "RECONNECT_CAPABILITY_REQUIRED",
        ),
        (
            E::RecoveryCapabilityRequired,
            "RECOVERY_CAPABILITY_REQUIRED",
        ),
    ]
    .into_iter()
    .map(|(error, name)| {
        let disposition = match error.disposition() {
            ProtocolDisposition::OperationTerminal => "OPERATION_TERMINAL",
            ProtocolDisposition::ResyncRequired => "RESYNC_REQUIRED",
            ProtocolDisposition::SessionFatal => "SESSION_FATAL",
            ProtocolDisposition::TransportFatal => "TRANSPORT_FATAL",
        };
        (u64::from(error.code()), name, disposition)
    })
    .collect();
    assert_eq!(crate_codes, registered);
}

/// N8 (ARCH-LOGIN-FIRST-PACKETS-V1 §1.2): exactly the admission refusal codes carry a
/// `progression` and `public_class`, and `admission_refusal_class` returns the registry's.
#[test]
#[allow(clippy::expect_used)]
fn admission_refusal_classes_match_the_registry() {
    let registry: serde_json::Value =
        serde_json::from_str(crate::PROTOCOL_OTERYN_V1_REGISTRY_JSON).expect("protocol registry");
    let mut admission = Vec::new();
    for entry in registry["error_codes"].as_array().expect("error_codes") {
        let code = u32::try_from(entry["code"].as_u64().expect("code")).expect("u32 code");
        let progression = entry.get("progression").and_then(|value| value.as_str());
        let class = entry.get("public_class").and_then(|value| value.as_str());
        if !ADMISSION_REFUSAL_CODES.contains(&code) {
            assert_eq!((progression, class), (None, None), "{code}");
            assert_eq!(admission_refusal_class(code), None, "{code}");
            continue;
        }
        // CP D1187: the ADMIT0 reconnect/recovery rows end the operation, not the transport.
        let disposition = if matches!(code, 1117 | 1118) {
            "OPERATION_TERMINAL"
        } else {
            "TRANSPORT_FATAL"
        };
        assert_eq!(entry["default_disposition"], disposition, "{code}");
        let (crate_progression, crate_class) =
            admission_refusal_class(code).expect("admission code");
        let crate_progression = match crate_progression {
            AdmissionProgression::Retryable => "RETRYABLE",
            AdmissionProgression::Terminal => "TERMINAL",
            AdmissionProgression::SecurityTerminal => "SECURITY_TERMINAL",
        };
        let crate_class = match crate_class {
            AdmissionPublicClass::RetryLogin => "RETRY_LOGIN",
            AdmissionPublicClass::AuthenticationRequired => "AUTHENTICATION_REQUIRED",
            AdmissionPublicClass::TemporarilyUnavailable => "TEMPORARILY_UNAVAILABLE",
            AdmissionPublicClass::SessionUnavailable => "SESSION_UNAVAILABLE",
            AdmissionPublicClass::ClientUpdateRequired => "CLIENT_UPDATE_REQUIRED",
            AdmissionPublicClass::CharacterAlreadyActive => "CHARACTER_ALREADY_ACTIVE",
        };
        assert_eq!(
            (progression, class),
            (Some(crate_progression), Some(crate_class)),
            "{code}"
        );
        admission.push(code);
    }
    assert_eq!(admission, (1100..=1118).collect::<Vec<_>>());
    assert_eq!(
        ADMISSION_REFUSAL_ROWS
            .map(|(error, _, _)| error.code())
            .to_vec(),
        admission
    );
}

/// The 17 rows of §1.2 and the two ADMIT0 §5 rows, written out once more so a table edit cannot pass unnoticed.
#[test]
fn admission_refusal_class_table() {
    use AdmissionProgression::{Retryable as R, SecurityTerminal as S, Terminal as T};
    use AdmissionPublicClass::*;
    let rows = [
        (1100, T, RetryLogin),
        (1101, S, AuthenticationRequired),
        (1102, S, RetryLogin),
        (1103, R, TemporarilyUnavailable),
        (1104, T, RetryLogin),
        (1105, S, SessionUnavailable),
        (1106, R, TemporarilyUnavailable),
        (1107, S, AuthenticationRequired),
        (1108, R, TemporarilyUnavailable),
        (1109, T, RetryLogin),
        (1110, T, RetryLogin),
        (1111, T, RetryLogin),
        (1112, T, ClientUpdateRequired),
        (1113, T, SessionUnavailable),
        (1114, T, CharacterAlreadyActive),
        (1115, R, TemporarilyUnavailable),
        (1116, T, ClientUpdateRequired),
        (1117, T, ClientUpdateRequired),
        (1118, T, ClientUpdateRequired),
    ];
    for (code, progression, class) in rows {
        assert_eq!(admission_refusal_class(code), Some((progression, class)));
    }
    // Unassigned in range: generic, no retry. Outside the range: not an admission refusal.
    for code in [1119, 1150, 1199] {
        assert_eq!(
            admission_refusal_class(code),
            Some((T, TemporarilyUnavailable))
        );
    }
    for code in [0, 1050, 1099, 1200, u32::MAX] {
        assert_eq!(admission_refusal_class(code), None);
    }
}

/// The N8 frame: type 14, generation 0, no sequence, and a payload of exactly the code and
/// `TRANSPORT_FATAL`; the zero correlation fields are absent.
#[test]
fn admission_refusal_frame_carries_only_code_and_disposition() -> Result<(), FoundationProtocolError>
{
    let frame = encode_protocol_error(FoundationProtocolError::AdmissionGrantExpired, 0)?;
    // 08 0e (type 14), 22 05 (payload, 5 bytes): 08 d0 08 (code 1104), 10 04 (TRANSPORT_FATAL).
    assert_eq!(
        frame,
        [0x08, 0x0e, 0x22, 0x05, 0x08, 0xd0, 0x08, 0x10, 0x04]
    );
    let envelope = decode_wire_envelope(&frame)?;
    assert_eq!(envelope.message_type(), MessageType::ProtocolError);
    assert_eq!(envelope.connection_generation(), 0);
    assert_eq!(
        decode_protocol_error(envelope.payload())?,
        ProtocolErrorView {
            error_code: 1104,
            disposition: ProtocolDisposition::TransportFatal,
            related_command_id: 0,
            expected_command_id: 0,
            expected_server_sequence: 0,
        }
    );
    Ok(())
}

#[test]
fn protocol_error_decode_fails_closed() {
    for payload in [
        &[][..],
        &[0x10, 0x04][..],                                     // no code
        &[0x08, 0xd0, 0x08][..],                               // no disposition
        &[0x08, 0xd0, 0x08, 0x10, 0x05][..],                   // unregistered disposition
        &[0x08, 0x00, 0x10, 0x04][..],                         // zero code
        &[0x08, 0xd0, 0x08, 0x08, 0xd0, 0x08, 0x10, 0x04][..], // repeated code
    ] {
        assert_eq!(
            decode_protocol_error(payload),
            Err(FoundationProtocolError::MalformedEnvelope),
            "{payload:02x?}"
        );
    }
}

#[test]
fn semantic_wire_ids_require_uuidv7_and_rfc_variant() -> Result<(), FoundationProtocolError> {
    let mut valid = [0u8; 16];
    valid[6] = 0x70;
    valid[8] = 0x80;
    valid[15] = 1;
    assert!(GameSessionId::decode(&valid).is_ok());
    assert!(CharacterId::decode(&valid).is_ok());
    assert!(WorldId::decode(&valid).is_ok());
    assert!(ChannelId::decode(&valid).is_ok());
    assert!(NodeId::decode(&valid).is_ok());
    let mut wrong_version = valid;
    wrong_version[6] = 0x40;
    assert_eq!(
        NodeId::decode(&wrong_version),
        Err(FoundationProtocolError::InvalidWireIdentifier)
    );
    let mut wrong_variant = valid;
    wrong_variant[8] = 0x00;
    assert_eq!(
        NodeId::decode(&wrong_variant),
        Err(FoundationProtocolError::InvalidWireIdentifier)
    );
    Ok(())
}
