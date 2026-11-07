use oteryn_gameplay_foundation::foundation::GameSessionId;
use oteryn_protocol_oteryn::spectate::{EndReason, ObservationKind, SpectateObservation};
use std::collections::VecDeque;

/// Immutable producer packet for the existing ANL EventEnvelope: target only in
/// game_session_id, type 4, DURABLE_AUDIT, SECURITY_SENSITIVE, P30D. SPECTATE-AUDIT-1
/// supplies durable envelope identities/outbox IO; this owner never acknowledges an IO attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditFact {
    pub incarnation: [u8; 16],
    pub ordinal: u64,
    pub target_session: GameSessionId,
    pub payload: Vec<u8>,
}

#[derive(Default)]
pub(super) struct AuditJournal {
    pub(super) pending: VecDeque<AuditFact>,
    incarnation: Option<[u8; 16]>,
    ordinal: u64,
    reserved: usize,
}
impl AuditJournal {
    pub(super) fn reserve_start(&mut self, capacity: usize) -> bool {
        if self
            .pending
            .len()
            .saturating_add(self.reserved)
            .saturating_add(2)
            > capacity
            || self
                .ordinal
                .checked_add(self.reserved as u64)
                .and_then(|n| n.checked_add(2))
                .is_none()
        {
            return false;
        }
        if self.incarnation.is_none() {
            let mut id = [0; 16];
            if rustls::crypto::aws_lc_rs::default_provider()
                .secure_random
                .fill(&mut id)
                .is_err()
            {
                return false;
            }
            self.incarnation = Some(id);
        }
        self.reserved += 1;
        true
    }
    pub(super) fn start(&mut self, target: GameSessionId, observer: Option<GameSessionId>) {
        self.append(
            target,
            SpectateObservation {
                kind: if observer.is_some() {
                    ObservationKind::ObserverAdmitted
                } else {
                    ObservationKind::GrantCreated
                },
                observer_session_id: observer,
                end_reason: None,
            },
        );
    }
    pub(super) fn end(
        &mut self,
        target: GameSessionId,
        observer: Option<GameSessionId>,
        reason: EndReason,
    ) {
        self.reserved -= 1;
        self.append(
            target,
            SpectateObservation {
                kind: if observer.is_some() {
                    ObservationKind::ObserverEnded
                } else {
                    ObservationKind::GrantEnded
                },
                observer_session_id: observer,
                end_reason: Some(reason),
            },
        );
    }
    fn append(&mut self, target: GameSessionId, observation: SpectateObservation) {
        // Only the closed producers above construct payloads. Capacity and ordinals were reserved
        // before any grant/session start; terminal facts cannot fail due to a full queue.
        let payload = Vec::<u8>::try_from(&observation)
            .unwrap_or_else(|_| unreachable!("closed spectate audit producer"));
        self.ordinal += 1;
        self.pending.push_back(AuditFact {
            incarnation: self
                .incarnation
                .unwrap_or_else(|| unreachable!("reserved audit incarnation")),
            ordinal: self.ordinal,
            target_session: target,
            payload,
        });
    }
    pub(super) fn ack(&mut self, incarnation: [u8; 16], through: u64) -> bool {
        if self.incarnation != Some(incarnation) || through == 0 || through > self.ordinal {
            return false;
        }
        while self
            .pending
            .front()
            .is_some_and(|fact| fact.ordinal <= through)
        {
            self.pending.pop_front();
        }
        true
    }
}
