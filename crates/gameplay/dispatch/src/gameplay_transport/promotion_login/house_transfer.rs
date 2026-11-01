//! The existing Premium registration owner carries its own data after a native handoff.
use super::{GameSessionId, PremiumSessions};

impl PremiumSessions {
    /// The caller independently proves the committed same-account handoff.
    /// Register its successor before removing the predecessor, so a late
    /// release cannot cancel this account's live puller or lose promotion data.
    pub(in crate::gameplay_transport) fn transfer_session(
        &mut self,
        account: [u8; 16],
        source: GameSessionId,
        destination: GameSessionId,
    ) -> bool {
        if source == destination
            || self.account_of(source).is_some_and(|held| held != account)
            || self
                .account_of(destination)
                .is_some_and(|held| held != account)
            || conflict(&self.1, source, destination)
            || conflict(&self.2, source, destination)
        {
            return false;
        }
        self.admit(account, destination);
        if let Some(value) = self.1.remove(&source) {
            self.1.insert(destination, value);
        }
        if let Some(value) = self.2.remove(&source) {
            self.2.insert(destination, value);
        }
        if let Some(sessions) = self.0.get_mut(&account) {
            sessions.remove(&source);
        }
        true
    }

    /// Only observations: Premium eligibility is still read from its account
    /// owner at use. A failed promotion read leaves an already retained view.
    pub(in crate::gameplay_transport) fn restore_promotion_snapshot(
        &mut self,
        account: [u8; 16],
        session: GameSessionId,
        promoted: bool,
        premium: bool,
    ) -> bool {
        if self.account_of(session) != Some(account) {
            return false;
        }
        self.2.insert(session, promoted);
        self.1.insert(
            session,
            oteryn_gameplay_progression::promotion::login_projection(promoted, premium),
        );
        true
    }
}

fn conflict(
    values: &std::collections::HashMap<GameSessionId, bool>,
    source: GameSessionId,
    destination: GameSessionId,
) -> bool {
    matches!((values.get(&source), values.get(&destination)), (Some(before), Some(after)) if before != after)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, reason = "native session identity fixtures")]
    use super::*;

    fn session(n: u8) -> GameSessionId {
        GameSessionId::decode(&[1, 0, 0, 0, 0, n, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, n])
            .expect("session")
    }

    #[test]
    fn house_transfer_keeps_premium_registration_and_promotion_through_late_release() {
        let mut owner = PremiumSessions::default();
        owner.admit([1; 16], session(1));
        assert!(owner.restore_promotion_snapshot([1; 16], session(1), true, true));
        assert!(owner.transfer_session([1; 16], session(1), session(2)));
        assert_eq!(owner.account_of(session(1)), None);
        assert_eq!(owner.account_of(session(2)), Some([1; 16]));
        assert!(owner.promoted(session(2)));
        assert_eq!(owner.1.get(&session(2)), Some(&true));
        assert!(!owner.release([1; 16], session(1)));
        assert!(owner.transfer_session([1; 16], session(1), session(2)));
        assert!(owner.release([1; 16], session(2)));
    }

    #[test]
    fn house_transfer_cannot_move_another_accounts_registration_or_conflicting_snapshot() {
        let mut owner = PremiumSessions::default();
        owner.admit([1; 16], session(1));
        owner.admit([2; 16], session(2));
        assert!(!owner.transfer_session([1; 16], session(1), session(2)));
        assert_eq!(owner.account_of(session(1)), Some([1; 16]));
        assert_eq!(owner.account_of(session(2)), Some([2; 16]));
        owner.admit([1; 16], session(3));
        assert!(owner.restore_promotion_snapshot([1; 16], session(1), true, true));
        assert!(owner.restore_promotion_snapshot([1; 16], session(3), false, false));
        assert!(!owner.transfer_session([1; 16], session(1), session(3)));
        assert!(owner.promoted(session(1)));
        assert!(!owner.promoted(session(3)));
    }
}
