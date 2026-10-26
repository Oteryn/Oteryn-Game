use super::*;

/// A pending authenticated entry holds a pull registration, but no gameplay authority. Drop
/// releases it on refusal, timeout or cancellation without cancelling another session's pulls.
pub(in crate::gameplay_transport) struct PremiumEntry<'a> {
    sessions: &'a std::sync::Mutex<PremiumSessions>,
    refresher: &'a crate::premium::refresh::PremiumRefresher,
    account: [u8; 16],
    session: GameSessionId,
    retained: bool,
}

impl PremiumEntry<'_> {
    pub(in crate::gameplay_transport) fn retain(mut self) {
        self.retained = true;
    }
}

impl Drop for PremiumEntry<'_> {
    fn drop(&mut self) {
        if !self.retained {
            let mut sessions = self
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if sessions.release(self.account, self.session) {
                self.refresher.release(self.account);
            }
        }
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// The deciding transaction already runs this owner predicate under its admission locks.
    /// Carry a live PREM-1 view into it too, so waiting for reservation/DB locks cannot reuse
    /// Premium that expired after the fail-fast check. The activation latch is settled first
    /// by `begin_premium_channel_entry`; an absent coordinator stays closed.
    pub(in crate::gameplay_transport) fn channel_entry_check(
        &self,
        account: [u8; 16],
        capabilities: crate::durability::fresh_admission::CurrentCapabilityCheck,
    ) -> crate::durability::fresh_admission::CurrentCapabilityCheck {
        if !self.premium_only {
            return capabilities;
        }
        let premium = self
            .premium_coordinator
            .map(|coordinator| coordinator.view(account));
        std::sync::Arc::new(move |scope, generation, policy| {
            premium
                .as_ref()
                .is_some_and(|view| view.premium_gate().require().is_ok())
                && capabilities(scope, generation, policy)
        })
    }

    /// PREMCHAN: authenticated first entry may need its first PREM-1 snapshot. Reuse the one
    /// account puller and its registration owner; never create a second Premium source. The
    /// two-second wait (PREMCHAN-RL-02) fits inside the existing entry deadline. Missing,
    /// expired, revoked, uncertain or not-yet-delivered Premium all remain closed.
    pub(in crate::gameplay_transport) async fn begin_premium_channel_entry(
        &self,
        account: [u8; 16],
        session: GameSessionId,
    ) -> Result<Option<PremiumEntry<'_>>, AdmissionRefusal> {
        if !self.premium_only {
            return Ok(None);
        }
        self.admit_premium(account, session);
        let entry = PremiumEntry {
            sessions: &self.states.premium_sessions,
            refresher: &self.premium,
            account,
            session,
            retained: false,
        };
        let wait = async {
            loop {
                if self.premium_gate(account).await.require().is_ok() {
                    return;
                }
                // PREMCHAN-RL-03: bounded local observations; no extra HTTP pull per poll.
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        };
        tokio::time::timeout(Duration::from_secs(2), wait)
            .await
            .map_err(|_| AdmissionRefusal::PremiumRequired)?;
        Ok(Some(entry))
    }

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
