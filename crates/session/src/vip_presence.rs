//! A presence confirmation expires locally even when no further network frame arrives.
use super::{Session, SessionError, SessionStream};
use oteryn_protocol_oteryn::vip_presence::{self as wire, PresenceBatch, Status};
use std::time::Duration;
use tokio::time::Instant;

#[derive(Debug)]
pub(super) struct VipPresenceStore {
    batch: PresenceBatch,
    confirmed_at: Instant,
}

impl VipPresenceStore {
    pub(super) fn new(batch: PresenceBatch) -> Self {
        Self {
            batch,
            confirmed_at: Instant::now(),
        }
    }

    fn at(&self, now: Instant) -> PresenceBatch {
        let mut batch = self.batch.clone();
        let age = now.saturating_duration_since(self.confirmed_at);
        for entry in &mut batch.entries {
            // Announcements are events; rendering the persistent view must not repeat one.
            entry.announce = false;
            if entry.status == Status::Online {
                let lifetime = Duration::from_millis(u64::from(entry.confirmed_for_ms));
                if age >= lifetime {
                    entry.status = Status::Unconfirmed;
                    entry.confirmed_for_ms = 0;
                } else {
                    entry.confirmed_for_ms =
                        u32::try_from((lifetime - age).as_millis()).unwrap_or(0);
                    if entry.confirmed_for_ms == 0 {
                        entry.status = Status::Unconfirmed;
                    }
                }
            }
        }
        batch
    }
}

pub(super) fn check_selection(selected: &[u32]) -> Result<(), SessionError> {
    if selected.contains(&wire::CAPABILITY_VIP_PRESENCE_V1) {
        for capability in [
            oteryn_protocol_oteryn::chat::CAPABILITY_CHAT_V1,
            oteryn_protocol_oteryn::vip::CAPABILITY_VIP_V1,
        ] {
            if !selected.contains(&capability) {
                return Err(SessionError::CapabilityNotSelected { capability });
            }
        }
    }
    Ok(())
}

impl<S: SessionStream> Session<S> {
    /// Current coarse statuses. ONLINE entries are demoted after their monotonic deadline;
    /// callers never need a live connection or an extra timer to avoid showing stale ONLINE.
    pub fn vip_presence(&self) -> Option<PresenceBatch> {
        self.vip_presence
            .as_ref()
            .map(|store| store.at(Instant::now()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wire::PresenceEntry;

    #[test]
    fn vip_presence_client_expires_without_network_and_does_not_reannounce() {
        let at = Instant::now();
        let store = VipPresenceStore {
            batch: PresenceBatch {
                entries: vec![PresenceEntry {
                    entry: 1,
                    name: "Alice".into(),
                    status: Status::Online,
                    announce: true,
                    confirmed_for_ms: 10_000,
                }],
            },
            confirmed_at: at,
        };
        let before = store.at(at + Duration::from_millis(9_999));
        assert_eq!(before.entries[0].status, Status::Online);
        assert_eq!(before.entries[0].confirmed_for_ms, 1);
        assert!(!before.entries[0].announce);
        let expired = store.at(at + Duration::from_secs(10));
        assert_eq!(expired.entries[0].status, Status::Unconfirmed);
        assert_eq!(expired.entries[0].confirmed_for_ms, 0);
        assert_eq!(store.at(at + Duration::from_secs(100)), expired);
    }

    #[test]
    fn vip_presence_selection_requires_chat_and_the_existing_vip_owner() {
        assert!(check_selection(&[258]).is_err());
        assert!(check_selection(&[7, 258]).is_err());
        assert!(check_selection(&[43, 258]).is_err());
        assert!(check_selection(&[7, 43, 258]).is_ok());
        assert!(check_selection(&[7, 43]).is_ok());
    }
}
