impl FreshAdmissionStore {
    /// FND-04B §6 grace expiry: once the original deadline of a committed loss
    /// has passed on the durable clock, terminally release the session. The
    /// release is prepared from the *current* claim rows (an owner refresh
    /// since admission is not an ownership change) and committed through the
    /// exact fenced [`Self::release`], which rejects any concurrent change of
    /// the session or its claims. A lost acknowledgement is reconciled, never
    /// decided again.
    pub async fn release_expired_loss(
        &self,
        session: GameSessionId,
        account_id: &str,
    ) -> Result<ExpiredLossReleaseV1> {
        let (current, now) = self.current_session_at(session).await?;
        if current.session_state() == GameSessionState::Terminal {
            return Ok(ExpiredLossReleaseV1::Terminal);
        }
        let Some(deadline) = current.current_original_grace_deadline() else {
            return Ok(ExpiredLossReleaseV1::NotApplicable);
        };
        if current.session_state() != GameSessionState::Reconnectable
            || current.current_control_loss_epoch().is_none()
        {
            return Ok(ExpiredLossReleaseV1::NotApplicable);
        }
        if now < deadline {
            return Ok(ExpiredLossReleaseV1::NotExpired { deadline, now });
        }
        self.release_current_claims(current, now, account_id).await
    }

    /// Terminal release of a resumed session whose recovered connection ended again when
    /// its loss after the resume could not be recorded, so it is never stranded ACTIVE on a
    /// dead transport: the player loses grace, never the ability to enter again. Only the
    /// exact ended controller transport of a session that was already lost once is
    /// released; a newer controller keeps its session.
    pub async fn release_abandoned_session(
        &self,
        session: GameSessionId,
        account_id: &str,
        transport: AuthenticatedTransportRefV1,
    ) -> Result<ExpiredLossReleaseV1> {
        let (current, now) = self.current_session_at(session).await?;
        if current.session_state() == GameSessionState::Terminal {
            return Ok(ExpiredLossReleaseV1::Terminal);
        }
        if current.session_state() != GameSessionState::Active
            || current.current_transport() != Some(transport)
        {
            return Ok(ExpiredLossReleaseV1::NotApplicable);
        }
        // A native Activity successor can have an unacknowledged adopted
        // transport without any owning loss yet. Its immutable issuance or
        // recovery journal must prove the exact current Activity transport;
        // an ordinary never-lost initial Channel still cannot use this owner.
        if current.current_control_loss_epoch().is_none()
            && !matches!(
                self.activity_loss_origin(session).await?,
                Some(ControlLossHistoryV1::ActivityOrigin { .. })
            )
        {
            return Ok(ExpiredLossReleaseV1::NotApplicable);
        }
        self.release_current_claims(current, now, account_id).await
    }

    /// LOGOUT-WIRE-1 (ARCH-KILL-REWARD-LOGOUT-1 §1.6): terminal release of an ACTIVE session's
    /// own logout, on its exact current controller transport, with or without a control-loss
    /// epoch. A session on another transport, or one that is not ACTIVE, is not this logout's to
    /// release. A replay after the commit returns `Terminal`, never a second commit. No
    /// protection marker is written: a TERMINAL session is never resumed, and PvE re-entry
    /// protection is granted only on a resume.
    pub async fn release_logout_session(
        &self,
        session: GameSessionId,
        account_id: &str,
        transport: AuthenticatedTransportRefV1,
    ) -> Result<ExpiredLossReleaseV1> {
        let (current, now) = self.current_session_at(session).await?;
        if current.session_state() == GameSessionState::Terminal {
            return Ok(ExpiredLossReleaseV1::Terminal);
        }
        if current.session_state() != GameSessionState::Active
            || current.current_transport() != Some(transport)
        {
            return Ok(ExpiredLossReleaseV1::NotApplicable);
        }
        let mut store = self.clone();
        store.explicit_logout = true;
        store.release_current_claims(current, now, account_id).await
    }

    /// Signed staff removal of the exact recorded loss, independent of its
    /// grace deadline. Uses the same current-claims save/terminal transaction.
    pub async fn release_staff_lost_session(
        &self,
        session: GameSessionId,
        account_id: &str,
        epoch: ControlLossEpochRefV1,
    ) -> Result<ExpiredLossReleaseV1> {
        let (current, now) = self.current_session_at(session).await?;
        if current.session_state() == GameSessionState::Terminal {
            return Ok(ExpiredLossReleaseV1::Terminal);
        }
        if current.session_state() != GameSessionState::Reconnectable
            || current.current_control_loss_epoch() != Some(epoch)
            || current.current_transport().is_some()
        {
            return Ok(ExpiredLossReleaseV1::NotApplicable);
        }
        let mut staff = self.clone();
        staff.staff_loss = Some(epoch);
        staff.release_current_claims(current, now, account_id).await
    }

    /// D449 (ARCH-BATCH-ITEM-EQUIP-PACKETS §1.13): terminal release, with no successor, of a
    /// RECONNECTABLE session whose resume passed every check but the capability check. The
    /// client gave up the same-session path by changing its supported set, so its fresh
    /// admission must find the character free at once, not after the reconnect window. Only
    /// the exact loss `epoch` the resume verified, still within its original grace deadline,
    /// is released; a resumed, newer or expired loss is not this release's. A replay after
    /// the commit returns `Terminal`, never a second commit.
    pub async fn release_capability_mismatch(
        &self,
        session: GameSessionId,
        account_id: &str,
        epoch: ControlLossEpochRefV1,
    ) -> Result<ExpiredLossReleaseV1> {
        let (current, now) = self.current_session_at(session).await?;
        if current.session_state() == GameSessionState::Terminal {
            return Ok(ExpiredLossReleaseV1::Terminal);
        }
        if current.session_state() != GameSessionState::Reconnectable
            || current.current_control_loss_epoch() != Some(epoch)
            || current
                .current_original_grace_deadline()
                .is_none_or(|deadline| now >= deadline)
        {
            return Ok(ExpiredLossReleaseV1::NotApplicable);
        }
        self.release_current_claims(current, now, account_id).await
    }
}
