//! Carry the existing talk-rate window; NPC prompts are contextual and close.
use super::{GameSessionId, NpcConversations, SessionNpc};

impl NpcConversations {
    pub(in crate::gameplay_transport) fn house_transfer_ready(
        &self,
        source: GameSessionId,
        destination: GameSessionId,
    ) -> bool {
        source != destination
            && self.sessions.lock().is_ok_and(|sessions| {
                !sessions.contains_key(&source) || !sessions.contains_key(&destination)
            })
    }

    /// The caller owns an actual committed edge. Preserve only monotone talk
    /// history; never retarget a Bank confirmation, native receipt or NPC handle.
    pub(in crate::gameplay_transport) fn transfer_house_session(
        &self,
        source: GameSessionId,
        destination: GameSessionId,
    ) -> bool {
        if source == destination {
            return false;
        }
        let Ok(mut sessions) = self.sessions.lock() else {
            return false;
        };
        if sessions.contains_key(&source) && sessions.contains_key(&destination) {
            return false;
        }
        if let Some(old) = sessions.remove(&source) {
            sessions.insert(
                destination,
                SessionNpc {
                    recent: old.recent,
                    ..SessionNpc::default()
                },
            );
        }
        true
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, reason = "native NPC owner data fixtures")]
    use super::*;

    fn session(n: u8) -> GameSessionId {
        GameSessionId::decode(&[1, 0, 0, 0, 0, n, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, n])
            .expect("session")
    }

    #[test]
    fn house_transfer_keeps_talk_rate_and_starts_no_new_npc_context() {
        let owner = NpcConversations::default();
        owner.sessions.lock().expect("owner").insert(
            session(1),
            SessionNpc {
                recent: [1, 2, 3, 4].into_iter().collect(),
                ..SessionNpc::default()
            },
        );
        assert!(owner.house_transfer_ready(session(1), session(2)));
        assert!(owner.transfer_house_session(session(1), session(2)));
        assert!(owner.transfer_house_session(session(1), session(2)));
        let held = owner.sessions.lock().expect("owner");
        assert!(!held.contains_key(&session(1)));
        let next = held.get(&session(2)).expect("successor");
        assert_eq!(
            next.recent.iter().copied().collect::<Vec<_>>(),
            vec![1, 2, 3, 4]
        );
        assert!(
            next.window.is_none()
                && next.bank.is_none()
                && next.vocation.is_none()
                && next.exchange.is_none()
                && next.premium.is_none()
                && next.monk.is_none()
                && next.hireling.is_none()
        );
    }
}
