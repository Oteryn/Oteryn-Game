#![allow(clippy::expect_used)]
use super::*;

#[test]
fn spectate_messages_round_trip_with_strict_field_presence() {
    let messages = [
        SpectateMessage::GrantSet(Audience::Code),
        SpectateMessage::GrantSet(Audience::Contacts),
        SpectateMessage::GrantRevoke,
        SpectateMessage::TargetState {
            active: false,
            count: 0,
        },
        SpectateMessage::TargetState {
            active: true,
            count: 50,
        },
        SpectateMessage::Stop,
        SpectateMessage::NotAvailable,
        SpectateMessage::NotAllowed,
        SpectateMessage::Keepalive,
        SpectateMessage::Frame {
            domain: 17,
            payload_type: 1,
            snapshot: true,
            payload: vec![1, 2, 3],
        },
    ];
    for message in messages {
        let bytes = Vec::<u8>::try_from(&message).expect("encode");
        assert_eq!(
            SpectateMessage::try_from(bytes.as_slice()).expect("decode"),
            message
        );
        let envelope = ObserverEnvelope {
            generation: 3,
            message,
        };
        let frame = Vec::<u8>::try_from(&envelope).expect("envelope");
        assert_eq!(
            ObserverEnvelope::try_from(frame.as_slice()).expect("decode envelope"),
            envelope
        );
        let mut repeated = bytes.clone();
        repeated.extend_from_slice(&[8, 1]);
        assert!(SpectateMessage::try_from(repeated.as_slice()).is_err());
        let mut unknown = bytes;
        unknown.extend_from_slice(&[80, 1]);
        assert!(SpectateMessage::try_from(unknown.as_slice()).is_err());
    }
    for malformed in [
        vec![],
        vec![8, 0],
        vec![8, 10],
        vec![8, 1, 16, 0],
        vec![8, 3, 24, 0, 32, 1],
        vec![8, 4, 24, 0],
    ] {
        assert!(SpectateMessage::try_from(malformed.as_slice()).is_err());
    }
    assert!(
        Vec::<u8>::try_from(&SpectateMessage::TargetState {
            active: false,
            count: 1
        })
        .is_err()
    );
}

#[test]
fn spectate_projection_wire_has_no_private_domains() {
    for domain in [2, 3, 4, 5, 7, 8, 9, 10, 11, 13, 14, 15, 16, 20, 23, 24, 25] {
        let frame = SpectateMessage::Frame {
            domain,
            payload_type: 1,
            snapshot: false,
            payload: vec![],
        };
        assert!(Vec::<u8>::try_from(&frame).is_err());
    }
    let frame = SpectateMessage::Frame {
        domain: 1,
        payload_type: 2,
        snapshot: false,
        payload: vec![0; crate::MAX_STATE_DELTA_PAYLOAD_BYTES + 1],
    };
    assert!(Vec::<u8>::try_from(&frame).is_err());
}

#[test]
fn spectate_audit_golden_all_kinds_reasons_and_uuid_presence() {
    let id = crate::GameSessionId::decode(&[1, 1, 1, 1, 1, 1, 0x71, 1, 0x81, 1, 1, 1, 1, 1, 1, 1])
        .expect("session");
    for kind in [
        ObservationKind::GrantCreated,
        ObservationKind::GrantEnded,
        ObservationKind::ObserverAdmitted,
        ObservationKind::ObserverEnded,
    ] {
        for reason in [
            None,
            Some(EndReason::Revoked),
            Some(EndReason::TargetGone),
            Some(EndReason::TargetMoved),
            Some(EndReason::Overload),
            Some(EndReason::NotObservable),
            Some(EndReason::ObserverStopped),
        ] {
            for session in [None, Some(id)] {
                let value = SpectateObservation {
                    kind,
                    observer_session_id: session,
                    end_reason: reason,
                };
                let valid = matches!(
                    kind,
                    ObservationKind::ObserverAdmitted | ObservationKind::ObserverEnded
                ) == session.is_some()
                    && matches!(
                        kind,
                        ObservationKind::GrantEnded | ObservationKind::ObserverEnded
                    ) == reason.is_some()
                    && !(kind == ObservationKind::GrantEnded
                        && reason == Some(EndReason::ObserverStopped));
                let encoded = Vec::<u8>::try_from(&value);
                assert_eq!(encoded.is_ok(), valid);
                if let Ok(bytes) = encoded {
                    let mut golden = vec![8, kind as u8];
                    if let Some(session) = session {
                        golden.extend_from_slice(&[18, 16]);
                        golden.extend_from_slice(session.as_bytes());
                    }
                    if let Some(reason) = reason {
                        golden.extend_from_slice(&[24, reason as u8]);
                    }
                    assert_eq!(bytes, golden);
                    assert_eq!(
                        SpectateObservation::try_from(bytes.as_slice()).expect("decode"),
                        value
                    );
                    for key in [[8, 1], [32, 1]] {
                        let mut malformed = bytes.clone();
                        malformed.extend_from_slice(&key);
                        assert!(SpectateObservation::try_from(malformed.as_slice()).is_err());
                    }
                }
            }
        }
    }
    for malformed in [
        vec![],
        vec![8, 0],
        vec![8, 5],
        vec![8, 2, 24, 0],
        vec![8, 2, 24, 7],
        vec![8, 3, 18, 0],
        vec![8, 3, 18, 16, 0],
    ] {
        assert!(SpectateObservation::try_from(malformed.as_slice()).is_err());
    }
}
