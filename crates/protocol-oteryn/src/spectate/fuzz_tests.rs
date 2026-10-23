#![allow(clippy::expect_used)]

use super::*;
use crate::decoder_fuzz_support::{codec, uuid};
use oteryn_test_support::property::{Rng, for_each_case, fuzz_decoder};

fn message(rng: &mut Rng) -> SpectateMessage {
    match rng.below(10) {
        0 => SpectateMessage::GrantSet(if rng.bool() {
            Audience::Code
        } else {
            Audience::Contacts
        }),
        1 => SpectateMessage::GrantRevoke,
        2 => {
            let active = rng.bool();
            SpectateMessage::TargetState {
                active,
                count: if active { rng.below(51) as u8 } else { 0 },
            }
        }
        3 => SpectateMessage::Stop,
        4 => SpectateMessage::Frame {
            domain: 17,
            payload_type: 1,
            snapshot: true,
            payload: {
                let length = rng.below(128) as usize;
                rng.bytes(length)
            },
        },
        5 => SpectateMessage::End(*rng.pick(&[
            EndReason::Revoked,
            EndReason::TargetGone,
            EndReason::ObserverStopped,
        ])),
        6 => SpectateMessage::NotAvailable,
        7 => SpectateMessage::NotAllowed,
        _ => SpectateMessage::Keepalive,
    }
}

#[test]
fn fuzz_spectate_message() {
    codec(
        "SpectateMessage::try_from",
        message,
        |b| SpectateMessage::try_from(b),
        |v| Vec::try_from(v).expect("message"),
    );
}

#[test]
fn fuzz_observer_envelope() {
    codec(
        "ObserverEnvelope::try_from",
        |rng| ObserverEnvelope {
            generation: rng.u64(),
            message: message(rng),
        },
        |b| ObserverEnvelope::try_from(b),
        |v| Vec::try_from(v).expect("envelope"),
    );
    let sample = Vec::try_from(&ObserverEnvelope {
        generation: 1,
        message: SpectateMessage::Keepalive,
    })
    .expect("envelope");
    assert!(ObserverEnvelope::is_ingress(&sample));
    assert!(!ObserverEnvelope::is_ingress(&[]));
    assert!(!ObserverEnvelope::is_ingress(&[255; 11]));
    fuzz_decoder(
        "ObserverEnvelope::is_ingress",
        &[sample, vec![]],
        ObserverEnvelope::is_ingress,
    );
}

#[test]
fn fuzz_spectate_observation() {
    codec(
        "SpectateObservation::try_from",
        |rng| {
            let kind = *rng.pick(&[
                ObservationKind::GrantCreated,
                ObservationKind::GrantEnded,
                ObservationKind::ObserverAdmitted,
                ObservationKind::ObserverEnded,
            ]);
            SpectateObservation {
                kind,
                observer_session_id: matches!(
                    kind,
                    ObservationKind::ObserverAdmitted | ObservationKind::ObserverEnded
                )
                .then(|| crate::GameSessionId::decode(&uuid(rng)).expect("UUID")),
                end_reason: matches!(
                    kind,
                    ObservationKind::GrantEnded | ObservationKind::ObserverEnded
                )
                .then(|| {
                    *rng.pick(&[
                        EndReason::Revoked,
                        EndReason::TargetGone,
                        EndReason::Overload,
                    ])
                }),
            }
        },
        |b| SpectateObservation::try_from(b),
        |v| Vec::try_from(v).expect("observation"),
    );
}

#[test]
fn fuzz_end_reason_scalar() {
    for_each_case("EndReason::try_from", |rng, _| {
        let value = rng.u64();
        assert_eq!(EndReason::try_from(value).is_ok(), (1..=6).contains(&value));
        let valid = rng.below(6) + 1;
        assert_eq!(EndReason::try_from(valid).expect("reason") as u64, valid);
        Ok(())
    });
}
