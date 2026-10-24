use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    /// PREM-1b: start (or wake) the account's Premium pulls for `session`.
    pub(in crate::gameplay_transport) fn admit_premium(
        &self,
        account_id: [u8; 16],
        session: GameSessionId,
    ) {
        let mut sessions = self
            .states
            .premium_sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        sessions.admit(account_id, session);
        self.premium.admit(account_id);
    }

    /// PREM-1b (D476): `session` is over; the account's Premium pulls stop only when no other
    /// session of the account holds them. Decided under the same lock as admission, so a
    /// successor admitted before a late release keeps its pulls.
    /// Keep this here: the non-moved refresher must release under the registration lock.
    pub(in crate::gameplay_transport) fn release_premium(
        &self,
        account_id: [u8; 16],
        session: GameSessionId,
    ) {
        let mut sessions = self
            .states
            .premium_sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if sessions.release(account_id, session) {
            self.premium.release(account_id);
        }
    }
}
