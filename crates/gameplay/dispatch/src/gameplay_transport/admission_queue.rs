//! Bounded authenticated wait leases, scoped to the actual runtime owner.
use super::connection::AdmissionRefusal;
use super::{
    CarrierError, ChannelRuntimeV1, GameSessionId, PlayerActorReservation, RuntimeScopeRefV1,
};
use oteryn_protocol_oteryn::admission_queue::{AdmissionQueue, MAX_QUEUE_POSITION};
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

struct WaitingAdmission {
    scope: RuntimeScopeRefV1,
    key: [u8; 32],
    until: Instant,
}

#[derive(Default)]
pub(super) struct AdmissionWaitQueue {
    entries: VecDeque<WaitingAdmission>,
}

impl AdmissionWaitQueue {
    pub(super) fn reserve(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        session: GameSessionId,
        key: [u8; 32],
        now: Instant,
    ) -> Result<PlayerActorReservation, AdmissionRefusal> {
        self.entries.retain(|entry| entry.until > now);
        let scope = runtime.binding().runtime_scope();
        let blocked = self
            .entries
            .iter()
            .find(|entry| entry.scope == scope)
            .is_some_and(|entry| entry.key != key);
        if blocked {
            return Err(self.wait(scope, key, now));
        }
        match runtime.reserve_fresh_session(session) {
            Ok(reservation) => {
                self.entries
                    .retain(|entry| entry.scope != scope || entry.key != key);
                Ok(reservation)
            }
            Err(CarrierError::CapacityExceeded) => Err(self.wait(scope, key, now)),
            Err(_) => {
                self.entries
                    .retain(|entry| entry.scope != scope || entry.key != key);
                Err(AdmissionRefusal::Unavailable)
            }
        }
    }

    fn wait(&mut self, scope: RuntimeScopeRefV1, key: [u8; 32], now: Instant) -> AdmissionRefusal {
        if let Some(entry) = self
            .entries
            .iter_mut()
            .find(|entry| entry.scope == scope && entry.key == key)
        {
            entry.until = now + Duration::from_secs(6);
        } else if self.entries.len() < MAX_QUEUE_POSITION as usize {
            self.entries.push_back(WaitingAdmission {
                scope,
                key,
                until: now + Duration::from_secs(6),
            });
        } else {
            return AdmissionRefusal::Classified(
                crate::foundation::FoundationProtocolError::AdmissionCapacityExceeded,
            );
        }
        let position = self
            .entries
            .iter()
            .filter(|entry| entry.scope == scope)
            .position(|entry| entry.key == key)
            .map_or(1, |index| index as u32 + 1);
        AdmissionRefusal::Queued(AdmissionQueue {
            position,
            retry_after_ms: 2_000,
        })
    }
}

#[cfg(test)]
mod tests;
