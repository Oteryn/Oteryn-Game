//! D1118 audit leaf. Persistence and envelope registration belong to SPECTATE-AUDIT-1.
use super::{AuditFact, AuditKind, EndReason};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SpectateAuditUnavailable;

/// `Ok(())` means the event has committed durably, never merely queued in memory.
/// The adapter uses a deterministic event ID from (node incarnation, fact.ordinal),
/// retaining that identity on retry. It puts target_session only in the envelope.
/// Copy the front fact before awaiting submit: do not hold the mutable runtime owner
/// across storage I/O. Only acknowledge that same ordinal after successful commit;
/// on unavailability/cancellation leave it pending. Privacy termination stays synchronous.
pub(crate) trait SpectateAuditSink {
    async fn submit(&self, fact: &AuditFact) -> Result<(), SpectateAuditUnavailable>;
}

/// Matches docs/contracts/game-events/v1/spectate.proto; no envelope authority is minted here.
#[derive(Clone, PartialEq, prost::Message)]
pub(crate) struct SpectateObservationV1 {
    #[prost(enumeration = "Kind", tag = "1")]
    kind: i32,
    #[prost(bytes = "vec", optional, tag = "2")]
    observer_session_id: Option<Vec<u8>>,
    #[prost(enumeration = "Reason", tag = "3")]
    end_reason: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, prost::Enumeration)]
#[repr(i32)]
enum Kind {
    Unspecified = 0,
    GrantCreated = 1,
    GrantEnded = 2,
    ObserverAdmitted = 3,
    ObserverEnded = 4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, prost::Enumeration)]
#[repr(i32)]
enum Reason {
    Unspecified = 0,
    Revoked = 1,
    TargetGone = 2,
    TargetMoved = 3,
    Overload = 4,
    NotObservable = 5,
    ObserverStopped = 6,
}

impl SpectateObservationV1 {
    pub(crate) fn from_fact(fact: &AuditFact) -> Result<Self, SpectateAuditUnavailable> {
        let (kind, reason, observer) = match fact.kind {
            AuditKind::GrantCreated => (Kind::GrantCreated, None, false),
            AuditKind::GrantEnded(reason) => (Kind::GrantEnded, Some(reason), false),
            AuditKind::ObserverAdmitted => (Kind::ObserverAdmitted, None, true),
            AuditKind::ObserverEnded(reason) => (Kind::ObserverEnded, Some(reason), true),
        };
        if fact.ordinal == 0 || observer != fact.observer_session.is_some() {
            return Err(SpectateAuditUnavailable);
        }
        let end_reason = match reason {
            None => Reason::Unspecified,
            Some(EndReason::Revoked) => Reason::Revoked,
            Some(EndReason::TargetGone) => Reason::TargetGone,
            Some(EndReason::TargetMoved) => Reason::TargetMoved,
            Some(EndReason::Overload) => Reason::Overload,
            Some(EndReason::NotObservable) => Reason::NotObservable,
            Some(EndReason::ObserverStopped) => Reason::ObserverStopped,
        };
        Ok(Self {
            kind: kind as i32,
            observer_session_id: fact.observer_session.map(|id| id.as_bytes().to_vec()),
            end_reason: end_reason as i32,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::GameSessionId;
    use prost::Message;

    #[allow(clippy::expect_used, reason = "fixed valid UUID fixture")]
    fn session(seed: u8) -> GameSessionId {
        let mut bytes = [seed; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        GameSessionId::decode(&bytes).expect("fixed valid UUID fixture")
    }

    fn fact(kind: AuditKind, observer: bool) -> AuditFact {
        AuditFact {
            ordinal: 1,
            target_session: session(1),
            observer_session: observer.then(|| session(2)),
            kind,
        }
    }

    #[test]
    fn wire_tags_and_presence_match_contract() -> Result<(), SpectateAuditUnavailable> {
        for (kind, observer, kind_tag, reason_tag) in [
            (AuditKind::GrantCreated, false, 1, 0),
            (AuditKind::GrantEnded(EndReason::Revoked), false, 2, 1),
            (AuditKind::ObserverAdmitted, true, 3, 0),
            (AuditKind::ObserverEnded(EndReason::TargetGone), true, 4, 2),
        ] {
            let fact = fact(kind, observer);
            let bytes = SpectateObservationV1::from_fact(&fact)?.encode_to_vec();
            let mut expected = vec![8, kind_tag];
            if observer {
                expected.extend([18, 16]);
                expected.extend(session(2).as_bytes());
            }
            if reason_tag != 0 {
                expected.extend([24, reason_tag]);
            }
            assert_eq!(bytes, expected);
            let mut other_target = fact;
            other_target.target_session = session(3);
            assert_eq!(
                bytes,
                SpectateObservationV1::from_fact(&other_target)?.encode_to_vec()
            );
        }
        Ok(())
    }

    #[test]
    fn all_terminal_reasons_have_stable_numbers() -> Result<(), SpectateAuditUnavailable> {
        for (reason, number) in [
            (EndReason::Revoked, 1),
            (EndReason::TargetGone, 2),
            (EndReason::TargetMoved, 3),
            (EndReason::Overload, 4),
            (EndReason::NotObservable, 5),
            (EndReason::ObserverStopped, 6),
        ] {
            let payload =
                SpectateObservationV1::from_fact(&fact(AuditKind::ObserverEnded(reason), true))?;
            assert_eq!(payload.end_reason, number);
        }
        Ok(())
    }

    #[test]
    fn malformed_fact_never_encodes() {
        for kind in [
            AuditKind::GrantCreated,
            AuditKind::GrantEnded(EndReason::Revoked),
        ] {
            assert!(SpectateObservationV1::from_fact(&fact(kind, true)).is_err());
        }
        for kind in [
            AuditKind::ObserverAdmitted,
            AuditKind::ObserverEnded(EndReason::Revoked),
        ] {
            assert!(SpectateObservationV1::from_fact(&fact(kind, false)).is_err());
        }
        let mut zero = fact(AuditKind::GrantCreated, false);
        zero.ordinal = 0;
        assert!(SpectateObservationV1::from_fact(&zero).is_err());
    }
}
