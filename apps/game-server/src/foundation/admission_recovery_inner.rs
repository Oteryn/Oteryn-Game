/// Current fenced authority required to reconstruct one GameSession projection
/// after process replacement.
///
/// `commit` remains the immutable fresh-admission receipt. Every other field is
/// the current authoritative value and must come from the same fenced authority
/// read. Rehydration never treats GameSessionId as bearer proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameSessionAuthoritySnapshot<T: Copy + Eq> {
    commit: FreshAdmissionCommit<T>,
    session_state: GameSessionState,
    current_connection_generation: ConnectionGeneration,
    current_transport: Option<T>,
    current_character_lease: CharacterLease,
    current_scope_generation: ScopeOwnershipGeneration,
}

impl<T: Copy + Eq> GameSessionAuthoritySnapshot<T> {
    #[must_use]
    pub const fn new(
        commit: FreshAdmissionCommit<T>,
        session_state: GameSessionState,
        current_connection_generation: ConnectionGeneration,
        current_transport: Option<T>,
        current_character_lease: CharacterLease,
        current_scope_generation: ScopeOwnershipGeneration,
    ) -> Self {
        Self {
            commit,
            session_state,
            current_connection_generation,
            current_transport,
            current_character_lease,
            current_scope_generation,
        }
    }

    #[must_use]
    pub const fn commit(self) -> FreshAdmissionCommit<T> {
        self.commit
    }

    #[must_use]
    pub const fn session_state(self) -> GameSessionState {
        self.session_state
    }

    #[must_use]
    pub const fn current_connection_generation(self) -> ConnectionGeneration {
        self.current_connection_generation
    }

    #[must_use]
    pub const fn current_transport(self) -> Option<T> {
        self.current_transport
    }

    #[must_use]
    pub const fn current_character_lease(self) -> CharacterLease {
        self.current_character_lease
    }

    #[must_use]
    pub const fn current_scope_generation(self) -> ScopeOwnershipGeneration {
        self.current_scope_generation
    }
}

impl CharacterLease {
    pub fn new(character_id: CharacterId, generation: u64) -> Result<Self, AdmissionError> {
        if generation == 0 {
            return Err(AdmissionError::InvalidFacts);
        }
        Ok(Self {
            character_id,
            generation,
        })
    }
}

fn validate_current_authority<T: Copy + Eq>(
    expected_game_session_id: GameSessionId,
    snapshot: GameSessionAuthoritySnapshot<T>,
) -> Result<(), AdmissionError> {
    let committed = snapshot.commit();
    if committed.game_session_id() != expected_game_session_id {
        return Err(AdmissionError::ReconciliationUnavailable);
    }

    let lease = snapshot.current_character_lease();
    if lease.character_id() != committed.character_id() {
        return Err(AdmissionError::StaleLease);
    }
    // A same-GameSession recovery may reconstruct current placement/runtime
    // authority, but it cannot adopt a different CharacterLease generation.
    // Any lease generation change is a superseding character-authority fence.
    if lease.generation() != committed.character_lease_generation() {
        return Err(AdmissionError::StaleLease);
    }
    if snapshot.current_scope_generation().get() < committed.scope_ownership_generation() {
        return Err(AdmissionError::StaleRuntime);
    }
    if snapshot.current_connection_generation().get() < committed.connection_generation().get() {
        return Err(AdmissionError::StaleConnection);
    }

    match snapshot.session_state() {
        GameSessionState::Active if snapshot.current_transport().is_some() => Ok(()),
        GameSessionState::Reconnectable if snapshot.current_transport().is_none() => Ok(()),
        GameSessionState::Terminal if snapshot.current_transport().is_none() => Ok(()),
        _ => Err(AdmissionError::ReconciliationUnavailable),
    }
}

impl<T: Copy + Eq, J: ReconnectAttemptJournal<T>> AdmissionAuthority<T, J> {
    pub(super) const fn journal(&self) -> &J {
        &self.reconnect_attempts
    }

    pub(super) fn prepared_reconnect_projection(
        &self,
    ) -> Option<(ReconnectAttemptRef, T, ConnectionGeneration)> {
        self.prepared
            .map(|prepared| (prepared.attempt, prepared.candidate_transport, prepared.candidate))
    }

    pub(super) fn restore_reconciled_prepared_projection(
        &mut self,
        attempt: ReconnectAttemptRef,
        binding: ReconnectCommitBinding<T>,
    ) -> Result<ConnectionGeneration, AdmissionError> {
        if self.control_loss_pending.is_some() || self.runtime_scope_reconciliation_pending {
            return Err(AdmissionError::ReconciliationUnavailable);
        }

        let session = self.current.as_ref().ok_or(AdmissionError::Terminal)?;
        if session.state == GameSessionState::Terminal {
            return Err(AdmissionError::Terminal);
        }
        if session.state != GameSessionState::Reconnectable
            || self.current_transport.is_some()
            || session.connection_generation() != binding.predecessor_generation()
            || binding
                .predecessor_generation()
                .get()
                .checked_add(1)
                != Some(binding.candidate_generation().get())
        {
            return Err(AdmissionError::StaleConnection);
        }
        if session.character_lease() != binding.character_lease() {
            return Err(AdmissionError::StaleLease);
        }
        if session.runtime_scope_generation() != binding.scope_generation() {
            return Err(AdmissionError::StaleRuntime);
        }

        let prepared = PreparedReconnect {
            attempt,
            predecessor: binding.predecessor_generation(),
            candidate: binding.candidate_generation(),
            candidate_transport: binding.candidate_transport(),
            lease_generation: binding.character_lease().generation(),
            scope_generation: binding.scope_generation().get(),
        };
        match self.prepared {
            Some(current) if current == prepared => Ok(prepared.candidate),
            Some(_) => Err(AdmissionError::ReconciliationUnavailable),
            None => {
                self.prepared = Some(prepared);
                Ok(prepared.candidate)
            }
        }
    }

    pub(super) fn clear_process_projection(&mut self) {
        if let Some(session) = self.current.as_mut() {
            session.runtime_scope.invalidate();
        }
        self.current = None;
        self.current_transport = None;
        self.prepared = None;
        self.control_loss_pending = None;
        self.runtime_scope_reconciliation_pending = false;
    }

    pub(super) fn refresh_fresh_projection(
        &mut self,
        authenticated_transport: T,
        snapshot: GameSessionAuthoritySnapshot<T>,
    ) -> Result<&GameSession, AdmissionError> {
        let committed = snapshot.commit();
        validate_current_authority(committed.game_session_id(), snapshot)?;

        if committed.initial_transport() != authenticated_transport
            || snapshot.session_state() != GameSessionState::Active
            || snapshot.current_connection_generation() != committed.connection_generation()
            || snapshot.current_transport() != Some(authenticated_transport)
        {
            self.clear_process_projection();
            return Err(AdmissionError::GrantReplayed);
        }

        let session = self
            .current
            .as_mut()
            .ok_or(AdmissionError::ReconciliationUnavailable)?;
        if session.game_session_id != committed.game_session_id()
            || session.character_id != committed.character_id()
            || session.world_id != committed.world_id()
            || session.channel_id != committed.channel_id()
            || session.connection_generation() != snapshot.current_connection_generation()
        {
            self.clear_process_projection();
            return Err(AdmissionError::ReconciliationUnavailable);
        }

        session.lease_generation = snapshot.current_character_lease().generation();
        let current_scope = session.runtime_scope.generation();
        let authoritative_scope = snapshot.current_scope_generation();
        if authoritative_scope < current_scope {
            self.clear_process_projection();
            return Err(AdmissionError::StaleRuntime);
        }
        if authoritative_scope > current_scope {
            session
                .runtime_scope
                .apply_external_grant(authoritative_scope)
                .map_err(|_| AdmissionError::StaleRuntime)?;
        }
        session.state = GameSessionState::Active;
        self.current_transport = Some(authenticated_transport);
        self.prepared = None;
        self.control_loss_pending = None;
        self.runtime_scope_reconciliation_pending = false;
        self.current.as_ref().ok_or(AdmissionError::ReconciliationUnavailable)
    }

    pub(super) fn install_rehydrated_session(
        &mut self,
        game_session_id: GameSessionId,
        snapshot: GameSessionAuthoritySnapshot<T>,
    ) -> Result<&GameSession, AdmissionError> {
        if let Some(current) = self.current.as_ref() {
            return if current.state == GameSessionState::Terminal {
                Err(AdmissionError::Terminal)
            } else {
                Err(AdmissionError::IncumbentHealthy)
            };
        }
        if self.prepared.is_some()
            || self.control_loss_pending.is_some()
            || self.runtime_scope_reconciliation_pending
        {
            return Err(AdmissionError::ReconciliationUnavailable);
        }

        validate_current_authority(game_session_id, snapshot)?;
        if snapshot.session_state() == GameSessionState::Terminal {
            return Err(AdmissionError::Terminal);
        }

        let committed = snapshot.commit();
        let current_lease = snapshot.current_character_lease();
        let runtime_scope = ScopeRuntimeFence::from_external_grant(snapshot.current_scope_generation());
        let connection = ConnectionFence {
            current: snapshot.current_connection_generation(),
        };

        self.current = Some(GameSession {
            game_session_id,
            character_id: committed.character_id(),
            world_id: committed.world_id(),
            channel_id: committed.channel_id(),
            lease_generation: current_lease.generation(),
            runtime_scope,
            connection,
            state: snapshot.session_state(),
        });
        self.current_transport = snapshot.current_transport();
        self.prepared = None;
        self.control_loss_pending = None;
        self.runtime_scope_reconciliation_pending = false;
        self.current.as_ref().ok_or(AdmissionError::ReconciliationUnavailable)
    }
}

/// Stable non-process-local equality/fencing reference for one authenticated
/// reconnect candidate transport. The bytes carry no chronology or authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AuthenticatedTransportRefV1([u8; 16]);

impl AuthenticatedTransportRefV1 {
    pub fn decode(input: &[u8]) -> Result<Self, AdmissionError> {
        let bytes: [u8; 16] = input.try_into().map_err(|_| AdmissionError::InvalidFacts)?;
        if bytes.iter().all(|byte| *byte == 0) {
            return Err(AdmissionError::InvalidFacts);
        }
        Ok(Self(bytes))
    }

    #[must_use]
    pub const fn to_bytes(self) -> [u8; 16] {
        self.0
    }
}

#[cfg(test)]
mod durability_reconnect_v1_tests {
    use super::*;

    fn uuid_v7(raw: u64) -> [u8; 16] {
        let mut out = [0u8; 16];
        out[8..].copy_from_slice(&raw.to_be_bytes());
        out[6] = 0x70;
        out[8] = (out[8] & 0x3f) | 0x80;
        out
    }

    fn sample_record(
        attempt_raw: u64,
        transport_byte: u8,
    ) -> Result<ReconnectDurabilityRecordV1, ReconnectDurabilityErrorV1> {
        let game_session_id = GameSessionId::decode(&uuid_v7(10)).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let character_id = CharacterId::decode(&uuid_v7(11)).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let world_id = WorldId::decode(&uuid_v7(12)).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let channel_id = ChannelId::decode(&uuid_v7(13)).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let identity = ReconnectIdentityV1::new(
            game_session_id,
            ReconnectAttemptRef::new(attempt_raw).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            "123e4567-e89b-12d3-a456-426614174000",
            character_id,
            world_id,
            RuntimeScopeRefV1::channel(world_id, channel_id),
        )?;
        let predecessor = ConnectionGeneration::new(7).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let candidate = ConnectionGeneration::new(8).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let transport_ref = AuthenticatedTransportRefV1::decode(&[transport_byte; 16])
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let connection = ReconnectConnectionFenceV1::new(predecessor, candidate, transport_ref)?;
        let authority = ReconnectAuthorityFenceV1::new(
            9,
            ScopeOwnershipGeneration::new(10).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        )?;
        let continuity = ReconnectContinuityV1::new(
            ControlLossEpochRefV1::new(3)?,
            120,
            115,
            ProtectionEntitlementV1::unused(),
        )?;
        let fnd02 = Fnd02ReconciliationFenceV1::new(
            CommandId::new(3).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            vec![
                PendingCommandReconciliationV1::new(
                    CommandId::new(1).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
                    PendingCommandDispositionV1::PendingOriginal,
                ),
                PendingCommandReconciliationV1::new(
                    CommandId::new(2).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
                    PendingCommandDispositionV1::TerminalOutcomeRetained,
                ),
            ],
            41,
            vec![
                StateDomainRevisionV1::new(1, 4)?,
                StateDomainRevisionV1::new(2, 7)?,
            ],
        )?;
        let platform = AuthorityEvidenceFenceV1::new(
            "platform-security",
            "reconnect",
            "account",
            "sec:17",
            "decision:sec:17",
            100,
        )?;
        let trust = AuthorityEvidenceFenceV1::new(
            "proof-trust",
            "reconnect",
            "recovery-key",
            "trust:21",
            "decision:trust:21",
            101,
        )?;
        let compatibility = ReconnectCompatibilityEvidenceV1::new(
            1,
            1,
            "rules:1",
            "content:2",
            "map:3",
            "world:4",
            12,
            platform,
            trust,
            Some(110),
        )?;
        ReconnectDurabilityRecordV1::new(
            identity,
            connection,
            authority,
            continuity,
            ReconnectProofV1::ReauthenticatedRecovery {
                recovery_grant_nonce: [0x55; 32],
            },
            fnd02,
            compatibility,
        )
    }

    #[test]
    fn authenticated_transport_ref_v1_is_exact_nonzero_16_bytes() -> Result<(), AdmissionError> {
        let encoded = [0xA5u8; 16];
        let transport_ref = AuthenticatedTransportRefV1::decode(&encoded)?;
        assert_eq!(transport_ref.to_bytes(), encoded);
        assert_eq!(
            AuthenticatedTransportRefV1::decode(&[0u8; 16]),
            Err(AdmissionError::InvalidFacts)
        );
        assert_eq!(
            AuthenticatedTransportRefV1::decode(&[0xA5u8; 15]),
            Err(AdmissionError::InvalidFacts)
        );
        Ok(())
    }

    #[test]
    fn same_attempt_ref_is_immutable_and_ninth_attempt_fails_before_allocation()
    -> Result<(), ReconnectDurabilityErrorV1> {
        let epoch = ControlLossEpochRefV1::new(9)?;
        let mut budget = ReconnectAttemptBudgetV1::new(epoch);
        let first_attempt = ReconnectAttemptRef::new(1).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let first_ref = AuthenticatedTransportRefV1::decode(&[1u8; 16]).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let changed_ref = AuthenticatedTransportRefV1::decode(&[2u8; 16]).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        assert_eq!(budget.reserve(first_attempt, first_ref)?, ReconnectAttemptReservationV1::New);
        assert_eq!(budget.reserve(first_attempt, first_ref)?, ReconnectAttemptReservationV1::Existing);
        assert_eq!(
            budget.reserve(first_attempt, changed_ref),
            Err(ReconnectDurabilityErrorV1::IdempotencyConflict)
        );
        for raw in 2u64..=8 {
            let attempt = ReconnectAttemptRef::new(raw).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
            let transport_ref = AuthenticatedTransportRefV1::decode(&[u8::try_from(raw).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?; 16])
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
            assert_eq!(budget.reserve(attempt, transport_ref)?, ReconnectAttemptReservationV1::New);
        }
        let ninth = ReconnectAttemptRef::new(9).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let ninth_ref = AuthenticatedTransportRefV1::decode(&[9u8; 16]).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        assert_eq!(
            budget.reserve(ninth, ninth_ref),
            Err(ReconnectDurabilityErrorV1::AttemptCapacityExceeded)
        );
        assert_eq!(budget.distinct_attempts(), 8);
        Ok(())
    }

    #[test]
    fn collision_allows_only_new_attempt_under_capacity_and_never_same_attempt_remint()
    -> Result<(), ReconnectDurabilityErrorV1> {
        let mut budget = ReconnectAttemptBudgetV1::new(ControlLossEpochRefV1::new(1)?);
        let attempt = ReconnectAttemptRef::new(1).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let transport_ref = AuthenticatedTransportRefV1::decode(&[1u8; 16]).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        budget.reserve(attempt, transport_ref)?;
        budget.accept_prepare_completion(
            attempt,
            transport_ref,
            ReconnectPrepareDispositionV1::RejectedTransportRefCollision,
        )?;
        assert!(budget.replacement_allowed_after_collision(attempt));
        assert_eq!(
            budget.reserve(
                attempt,
                AuthenticatedTransportRefV1::decode(&[2u8; 16]).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            ),
            Err(ReconnectDurabilityErrorV1::IdempotencyConflict)
        );
        let replacement = ReconnectAttemptRef::new(2).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        assert_eq!(
            budget.reserve(
                replacement,
                AuthenticatedTransportRefV1::decode(&[2u8; 16]).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            )?,
            ReconnectAttemptReservationV1::New
        );

        let mut exhausted = ReconnectAttemptBudgetV1::new(ControlLossEpochRefV1::new(2)?);
        for raw in 1u64..=8 {
            exhausted.reserve(
                ReconnectAttemptRef::new(raw).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
                AuthenticatedTransportRefV1::decode(&[u8::try_from(raw).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?; 16])
                    .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            )?;
        }
        let final_attempt = ReconnectAttemptRef::new(8).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let final_ref = AuthenticatedTransportRefV1::decode(&[8u8; 16]).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        exhausted.accept_prepare_completion(
            final_attempt,
            final_ref,
            ReconnectPrepareDispositionV1::RejectedTransportRefCollision,
        )?;
        assert!(!exhausted.replacement_allowed_after_collision(final_attempt));
        Ok(())
    }

    #[test]
    fn record_preserves_complete_authority_reconciliation_and_security_evidence()
    -> Result<(), ReconnectDurabilityErrorV1> {
        let record = sample_record(3, 3)?;
        assert_eq!(record.version(), 1);
        assert_eq!(record.identity().account_id(), "123e4567-e89b-12d3-a456-426614174000");
        assert!(matches!(record.identity().runtime_scope(), RuntimeScopeRefV1::Channel { .. }));
        assert_eq!(record.connection().predecessor().get(), 7);
        assert_eq!(record.connection().candidate().get(), 8);
        assert_eq!(record.authority().character_lease_generation(), 9);
        assert_eq!(record.continuity().control_loss_epoch().get(), 3);
        assert_eq!(record.fnd02().pending().len(), 2);
        assert_eq!(record.fnd02().domain_revisions().len(), 2);
        assert_eq!(record.compatibility().protocol_major(), 1);
        assert_eq!(record.compatibility().transport_profile(), 1);
        assert_eq!(record.compatibility().account_security_generation(), 12);
        assert_eq!(record.compatibility().credential_expiration(), Some(110));
        assert_eq!(record.compatibility().platform_security_evidence().source_observed_at(), 100);
        assert_eq!(record.compatibility().proof_trust_evidence().source_observed_at(), 101);
        Ok(())
    }

    #[test]
    fn prepare_unavailable_retries_same_request_and_ambiguous_requires_reconciliation()
    -> Result<(), ReconnectDurabilityErrorV1> {
        let record = sample_record(4, 4)?;
        let (mut unavailable_flow, request) = ReconnectDurabilityFlowV1::begin(record.clone());
        let unavailable = ReconnectPrepareCompletionV1::for_request(
            &request,
            ReconnectPrepareDispositionV1::Unavailable,
        );
        assert_eq!(
            unavailable_flow.accept_prepare_completion(unavailable)?,
            ReconnectPrepareActionV1::RetrySameRequest(request.clone())
        );
        assert_eq!(unavailable_flow.phase(), ReconnectDurabilityPhaseV1::PendingPrepare);

        let (mut ambiguous_flow, ambiguous_request) = ReconnectDurabilityFlowV1::begin(record);
        let ambiguous = ReconnectPrepareCompletionV1::for_request(
            &ambiguous_request,
            ReconnectPrepareDispositionV1::Ambiguous,
        );
        assert_eq!(
            ambiguous_flow.accept_prepare_completion(ambiguous)?,
            ReconnectPrepareActionV1::ReconcileSameAttempt
        );
        assert_eq!(
            ambiguous_flow.phase(),
            ReconnectDurabilityPhaseV1::ReconciliationRequired
        );
        Ok(())
    }

    #[test]
    fn prepared_completion_requires_fresh_complete_revalidation_before_commit()
    -> Result<(), ReconnectDurabilityErrorV1> {
        let record = sample_record(5, 5)?;
        let (mut flow, request) = ReconnectDurabilityFlowV1::begin(record.clone());
        let prepared = ReconnectPrepareCompletionV1::for_request(
            &request,
            ReconnectPrepareDispositionV1::Prepared,
        );
        assert_eq!(
            flow.accept_prepare_completion(prepared)?,
            ReconnectPrepareActionV1::AwaitFinalRevalidation
        );
        let current = ReconnectCurrentAuthorityV1::from_record(&record, 105)?;
        let commit = flow.authorize_commit(current.clone(), 104)?;
        assert_eq!(commit.authorization().authorization_deadline(), 105);
        assert_eq!(flow.phase(), ReconnectDurabilityPhaseV1::PendingCommit);

        let (mut stale_flow, stale_request) = ReconnectDurabilityFlowV1::begin(record.clone());
        stale_flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
            &stale_request,
            ReconnectPrepareDispositionV1::Prepared,
        ))?;
        let mut stale = ReconnectCurrentAuthorityV1::from_record(&record, 105)?;
        stale.account_security_generation = stale
            .account_security_generation
            .checked_add(1)
            .ok_or(ReconnectDurabilityErrorV1::InvalidRecord)?;
        assert_eq!(
            stale_flow.authorize_commit(stale, 104),
            Err(ReconnectDurabilityErrorV1::StaleAuthority)
        );
        Ok(())
    }

    #[test]
    fn ambiguous_commit_installs_controller_only_after_exact_committed_reconciliation()
    -> Result<(), ReconnectDurabilityErrorV1> {
        let record = sample_record(6, 6)?;
        let (mut flow, request) = ReconnectDurabilityFlowV1::begin(record.clone());
        flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
            &request,
            ReconnectPrepareDispositionV1::Prepared,
        ))?;
        let current = ReconnectCurrentAuthorityV1::from_record(&record, 105)?;
        let commit_request = flow.authorize_commit(current, 104)?;
        let ambiguous = ReconnectCommitCompletionV1::for_request(
            &commit_request,
            ReconnectCommitDispositionV1::Ambiguous,
        );
        assert_eq!(
            flow.accept_commit_completion(ambiguous)?,
            ReconnectCommitActionV1::ReconcileSameAttempt
        );
        assert_eq!(flow.phase(), ReconnectDurabilityPhaseV1::ReconciliationRequired);

        let snapshot = ReconnectDurableReconciliationSnapshotV1::committed(record.clone());
        assert_eq!(
            flow.accept_reconciliation(
                snapshot,
                record.authority().scope_ownership_generation(),
            )?,
            ReconnectProjectionDecisionV1::InstallController {
                generation: record.connection().candidate(),
                transport_ref: record.connection().transport_ref(),
            }
        );

        let (mut mismatch_flow, mismatch_request) = ReconnectDurabilityFlowV1::begin(record.clone());
        mismatch_flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
            &mismatch_request,
            ReconnectPrepareDispositionV1::Ambiguous,
        ))?;
        let mut mismatch = ReconnectDurableReconciliationSnapshotV1::committed(record);
        mismatch.current_transport_ref = Some(
            AuthenticatedTransportRefV1::decode(&[7u8; 16])
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        );
        assert_eq!(
            mismatch_flow.accept_reconciliation(
                mismatch,
                ScopeOwnershipGeneration::new(10).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            ),
            Err(ReconnectDurabilityErrorV1::ReconciliationMismatch)
        );
        Ok(())
    }
}