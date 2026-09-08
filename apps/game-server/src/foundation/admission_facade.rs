/// Production fresh-admission entry: bounded submission and normalized completion,
/// independent of `ReconnectAttemptJournal`. SQLx adapters implement only its
/// separate durability port, never the synchronous compatibility journal.
pub use super::fresh_admission_durability::FreshAdmissionDurabilityFlowV1 as DurableFreshAdmissionAuthorityV1;

use super::admission as core;
use super::{
    ConnectionGeneration, GameSessionAuthoritySnapshot, GameSessionId, ScopeOwnershipGeneration,
};

/// One atomic durable read used to reconcile a reconnect operation.
///
/// The GameSession authority, attempt disposition and exact stored PREPARE/COMMIT
/// binding must describe the same fenced linearization point. In particular, a
/// stable `Committed` attempt outcome is not sufficient proof when the current
/// session/controller/lease/runtime authority has since changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReconnectAttemptAuthoritySnapshot<T: Copy + Eq> {
    session: GameSessionAuthoritySnapshot<T>,
    disposition: Option<core::ReconnectAttemptDisposition>,
    binding: Option<core::ReconnectCommitBinding<T>>,
}

impl<T: Copy + Eq> ReconnectAttemptAuthoritySnapshot<T> {
    #[must_use]
    pub const fn new(
        session: GameSessionAuthoritySnapshot<T>,
        disposition: Option<core::ReconnectAttemptDisposition>,
        binding: Option<core::ReconnectCommitBinding<T>>,
    ) -> Self {
        Self {
            session,
            disposition,
            binding,
        }
    }

    #[must_use]
    pub const fn session(self) -> GameSessionAuthoritySnapshot<T> {
        self.session
    }

    #[must_use]
    pub const fn disposition(self) -> Option<core::ReconnectAttemptDisposition> {
        self.disposition
    }

    #[must_use]
    pub const fn binding(self) -> Option<core::ReconnectCommitBinding<T>> {
        self.binding
    }
}

fn validate_atomic_attempt_authority<T: Copy + Eq>(
    expected_game_session_id: GameSessionId,
    snapshot: ReconnectAttemptAuthoritySnapshot<T>,
) -> Result<Option<core::ReconnectAttemptDisposition>, core::AdmissionError> {
    let session = snapshot.session();
    let committed = session.commit();
    if committed.game_session_id() != expected_game_session_id {
        return Err(core::AdmissionError::ReconciliationUnavailable);
    }
    let lease = session.current_character_lease();
    if lease.character_id() != committed.character_id()
        || lease.generation() != committed.character_lease_generation()
    {
        return Err(core::AdmissionError::StaleLease);
    }
    if session.current_scope_generation().get() < committed.scope_ownership_generation() {
        return Err(core::AdmissionError::StaleRuntime);
    }
    if session.current_connection_generation().get() < committed.connection_generation().get() {
        return Err(core::AdmissionError::StaleConnection);
    }

    match session.session_state() {
        core::GameSessionState::Active if session.current_transport().is_some() => {}
        core::GameSessionState::Reconnectable if session.current_transport().is_none() => {}
        core::GameSessionState::Terminal if session.current_transport().is_none() => {
            return Err(core::AdmissionError::Terminal);
        }
        _ => return Err(core::AdmissionError::ReconciliationUnavailable),
    }

    match snapshot.disposition() {
        None => {
            if snapshot.binding().is_some() {
                return Err(core::AdmissionError::ReconciliationUnavailable);
            }
            Ok(None)
        }
        Some(core::ReconnectAttemptDisposition::TerminallySuperseded) => Ok(snapshot.disposition()),
        Some(core::ReconnectAttemptDisposition::Prepared {
            candidate_generation,
        }) => {
            let binding = snapshot
                .binding()
                .ok_or(core::AdmissionError::ReconciliationUnavailable)?;
            if binding.candidate_generation() != candidate_generation
                || session.session_state() != core::GameSessionState::Reconnectable
                || session.current_transport().is_some()
                || session.current_connection_generation() != binding.predecessor_generation()
            {
                return Err(core::AdmissionError::StaleConnection);
            }
            if session.current_character_lease() != binding.character_lease() {
                return Err(core::AdmissionError::StaleLease);
            }
            if session.current_scope_generation() != binding.scope_generation() {
                return Err(core::AdmissionError::StaleRuntime);
            }
            Ok(snapshot.disposition())
        }
        Some(core::ReconnectAttemptDisposition::Committed { generation }) => {
            let binding = snapshot
                .binding()
                .ok_or(core::AdmissionError::ReconciliationUnavailable)?;
            if binding.candidate_generation() != generation
                || session.session_state() != core::GameSessionState::Active
                || session.current_connection_generation() != generation
                || session.current_transport() != Some(binding.candidate_transport())
            {
                return Err(core::AdmissionError::StaleConnection);
            }
            if session.current_character_lease() != binding.character_lease() {
                return Err(core::AdmissionError::StaleLease);
            }
            if session.current_scope_generation() < binding.scope_generation() {
                return Err(core::AdmissionError::StaleRuntime);
            }
            // The stored COMMIT binding proves which candidate transport and
            // generation won. CharacterLease remains exact for the same
            // GameSession. RuntimeScope may advance after COMMIT, but it may
            // never roll back below the generation atomically bound to COMMIT.
            Ok(snapshot.disposition())
        }
    }
}

/// Trusted durable GameSession authority used by the public Foundation admission
/// facade.
///
/// `load_session` is the process-recovery boundary: it must return one fenced,
/// internally consistent current snapshot for the requested GameSessionId. It
/// does not treat that identifier as bearer authority and must fail closed when
/// current authority cannot be proven. For GameNode/process replacement, the
/// snapshot's RuntimeScope generation must come from the current externally
/// established FND-03 recovery ownership grant; implementations must never
/// locally increment, reuse or reconstruct a stale ownership generation.
/// Synchronous in-memory/test compatibility only for fresh admission. Production
/// durability must use `DurableFreshAdmissionAuthorityV1` and its split-phase port.
pub trait ReconnectAttemptJournal<T: Copy + Eq> {
    fn commit_fresh<F>(
        &self,
        facts: core::FreshAdmissionFacts,
        authenticated_transport: T,
        issue_game_session_id: F,
    ) -> Result<core::FreshAdmissionAuthoritySnapshot<T>, core::AdmissionError>
    where
        F: FnOnce() -> Result<GameSessionId, core::AdmissionError>;

    fn load_session(
        &self,
        game_session_id: GameSessionId,
    ) -> Result<GameSessionAuthoritySnapshot<T>, core::AdmissionError>;

    /// Atomically reads the current fenced GameSession authority together with
    /// the exact reconnect attempt disposition and its retained binding.
    /// Implementations MUST obtain all three from one transaction/lock/fenced
    /// linearization point; composing this from separate `load_session` and
    /// `lookup` calls is invalid.
    fn reconcile_reconnect_attempt(
        &self,
        game_session_id: GameSessionId,
        attempt: core::ReconnectAttemptRef,
    ) -> Result<ReconnectAttemptAuthoritySnapshot<T>, core::AdmissionError>;

    fn mark_control_loss(
        &self,
        game_session_id: GameSessionId,
        observed_transport: T,
        observed_generation: ConnectionGeneration,
    ) -> Result<core::ControlLossDisposition, core::AdmissionError>;

    fn terminate_session(
        &self,
        game_session_id: GameSessionId,
        expected_generation: ConnectionGeneration,
    ) -> Result<(), core::AdmissionError>;

    fn advance_runtime_scope(
        &self,
        game_session_id: GameSessionId,
        expected_current: ScopeOwnershipGeneration,
        observed: ScopeOwnershipGeneration,
    ) -> Result<ScopeOwnershipGeneration, core::AdmissionError>;

    fn lookup(
        &self,
        game_session_id: GameSessionId,
        attempt: core::ReconnectAttemptRef,
    ) -> Result<Option<core::ReconnectAttemptDisposition>, core::AdmissionError>;

    /// At the same transaction/lock/fenced linearization point as a new
    /// PREPARED disposition and binding are written, implementations MUST prove
    /// that the GameSession remains reconnectable with no current controller,
    /// and that predecessor, strict-successor candidate, CharacterLease and
    /// RuntimeScope ownership exactly match the supplied binding. A stale
    /// candidate must never be published as PREPARED.
    fn claim_prepared(
        &self,
        game_session_id: GameSessionId,
        attempt: core::ReconnectAttemptRef,
        binding: core::ReconnectCommitBinding<T>,
    ) -> Result<core::ReconnectAttemptClaim, core::AdmissionError>;

    fn retire_if_unseen(
        &self,
        game_session_id: GameSessionId,
        attempt: core::ReconnectAttemptRef,
    ) -> Result<core::ReconnectAttemptDisposition, core::AdmissionError>;

    fn commit_prepared(
        &self,
        game_session_id: GameSessionId,
        attempt: core::ReconnectAttemptRef,
        binding: core::ReconnectCommitBinding<T>,
    ) -> Result<(), core::AdmissionError>;

    fn retire_prepared(
        &self,
        game_session_id: GameSessionId,
        attempt: core::ReconnectAttemptRef,
        candidate_generation: ConnectionGeneration,
    ) -> Result<(), core::AdmissionError>;
}

impl<T: Copy + Eq, J: ReconnectAttemptJournal<T>> core::ReconnectAttemptJournal<T> for J {
    fn commit_fresh<F>(
        &self,
        facts: core::FreshAdmissionFacts,
        authenticated_transport: T,
        issue_game_session_id: F,
    ) -> Result<core::FreshAdmissionAuthoritySnapshot<T>, core::AdmissionError>
    where
        F: FnOnce() -> Result<GameSessionId, core::AdmissionError>,
    {
        <J as ReconnectAttemptJournal<T>>::commit_fresh(
            self,
            facts,
            authenticated_transport,
            issue_game_session_id,
        )
    }

    fn mark_control_loss(
        &self,
        game_session_id: GameSessionId,
        observed_transport: T,
        observed_generation: ConnectionGeneration,
    ) -> Result<core::ControlLossDisposition, core::AdmissionError> {
        <J as ReconnectAttemptJournal<T>>::mark_control_loss(
            self,
            game_session_id,
            observed_transport,
            observed_generation,
        )
    }

    fn terminate_session(
        &self,
        game_session_id: GameSessionId,
        expected_generation: ConnectionGeneration,
    ) -> Result<(), core::AdmissionError> {
        <J as ReconnectAttemptJournal<T>>::terminate_session(
            self,
            game_session_id,
            expected_generation,
        )
    }

    fn advance_runtime_scope(
        &self,
        game_session_id: GameSessionId,
        expected_current: ScopeOwnershipGeneration,
        observed: ScopeOwnershipGeneration,
    ) -> Result<ScopeOwnershipGeneration, core::AdmissionError> {
        <J as ReconnectAttemptJournal<T>>::advance_runtime_scope(
            self,
            game_session_id,
            expected_current,
            observed,
        )
    }

    fn lookup(
        &self,
        game_session_id: GameSessionId,
        attempt: core::ReconnectAttemptRef,
    ) -> Result<Option<core::ReconnectAttemptDisposition>, core::AdmissionError> {
        let snapshot = <J as ReconnectAttemptJournal<T>>::reconcile_reconnect_attempt(
            self,
            game_session_id,
            attempt,
        )?;
        validate_atomic_attempt_authority(game_session_id, snapshot)
    }

    fn claim_prepared(
        &self,
        game_session_id: GameSessionId,
        attempt: core::ReconnectAttemptRef,
        binding: core::ReconnectCommitBinding<T>,
    ) -> Result<core::ReconnectAttemptClaim, core::AdmissionError> {
        <J as ReconnectAttemptJournal<T>>::claim_prepared(self, game_session_id, attempt, binding)
    }

    fn retire_if_unseen(
        &self,
        game_session_id: GameSessionId,
        attempt: core::ReconnectAttemptRef,
    ) -> Result<core::ReconnectAttemptDisposition, core::AdmissionError> {
        <J as ReconnectAttemptJournal<T>>::retire_if_unseen(self, game_session_id, attempt)
    }

    fn commit_prepared(
        &self,
        game_session_id: GameSessionId,
        attempt: core::ReconnectAttemptRef,
        binding: core::ReconnectCommitBinding<T>,
    ) -> Result<(), core::AdmissionError> {
        <J as ReconnectAttemptJournal<T>>::commit_prepared(self, game_session_id, attempt, binding)
    }

    fn retire_prepared(
        &self,
        game_session_id: GameSessionId,
        attempt: core::ReconnectAttemptRef,
        candidate_generation: ConnectionGeneration,
    ) -> Result<(), core::AdmissionError> {
        <J as ReconnectAttemptJournal<T>>::retire_prepared(
            self,
            game_session_id,
            attempt,
            candidate_generation,
        )
    }
}

#[derive(Debug)]
pub struct AdmissionAuthority<T: Copy + Eq, J: ReconnectAttemptJournal<T>> {
    core: core::AdmissionAuthority<T, J>,
}

impl<T: Copy + Eq, J: ReconnectAttemptJournal<T> + Default> Default for AdmissionAuthority<T, J> {
    fn default() -> Self {
        Self::new(J::default())
    }
}

impl<T: Copy + Eq, J: ReconnectAttemptJournal<T>> AdmissionAuthority<T, J> {
    #[must_use]
    pub const fn new(journal: J) -> Self {
        Self {
            core: core::AdmissionAuthority::new(journal),
        }
    }

    #[must_use]
    pub fn current(&self) -> Option<&core::GameSession> {
        self.core.current()
    }

    #[must_use]
    pub const fn current_transport(&self) -> Option<T> {
        self.core.current_transport()
    }

    pub fn commit_fresh<F>(
        &mut self,
        facts: core::FreshAdmissionFacts,
        authenticated_transport: T,
        issue_game_session_id: F,
    ) -> Result<&core::GameSession, core::AdmissionError>
    where
        F: FnOnce() -> Result<GameSessionId, core::AdmissionError>,
    {
        let game_session_id = self
            .core
            .commit_fresh(facts, authenticated_transport, issue_game_session_id)?
            .game_session_id();
        let snapshot = match self.core.journal().load_session(game_session_id) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.core.clear_process_projection();
                return Err(error);
            }
        };
        self.core
            .refresh_fresh_projection(authenticated_transport, snapshot)
    }

    pub fn rehydrate_session(
        &mut self,
        game_session_id: GameSessionId,
    ) -> Result<&core::GameSession, core::AdmissionError> {
        let snapshot = self.core.journal().load_session(game_session_id)?;
        self.core
            .install_rehydrated_session(game_session_id, snapshot)
    }

    pub fn terminate_current(&mut self) -> Result<(), core::AdmissionError> {
        self.core.terminate_current()
    }

    pub fn mark_unexpected_control_loss(
        &mut self,
        observed_transport: T,
        observed_generation: ConnectionGeneration,
    ) -> Result<core::ControlLossDisposition, core::AdmissionError> {
        self.core
            .mark_unexpected_control_loss(observed_transport, observed_generation)
    }

    pub fn observe_runtime_ownership_generation(
        &mut self,
        generation: u64,
    ) -> Result<(), core::AdmissionError> {
        self.core.observe_runtime_ownership_generation(generation)
    }

    fn reconcile_current_attempt(
        &mut self,
        attempt: core::ReconnectAttemptRef,
        candidate_transport: T,
    ) -> Result<Option<core::ReconnectAttemptDisposition>, core::AdmissionError> {
        let (
            game_session_id,
            character_id,
            world_id,
            channel_id,
            session_state,
            connection_generation,
            character_lease,
            scope_generation,
            current_transport,
        ) = {
            let session = self.core.current().ok_or(core::AdmissionError::Terminal)?;
            (
                session.game_session_id(),
                session.character_id(),
                session.world_id(),
                session.channel_id(),
                session.state(),
                session.connection_generation(),
                session.character_lease(),
                session.runtime_scope_generation(),
                self.core.current_transport(),
            )
        };

        let authority = match self
            .core
            .journal()
            .reconcile_reconnect_attempt(game_session_id, attempt)
        {
            Ok(authority) => authority,
            Err(error) => {
                self.core.clear_process_projection();
                return Err(error);
            }
        };
        let disposition = match validate_atomic_attempt_authority(game_session_id, authority) {
            Ok(disposition) => disposition,
            Err(error) => {
                self.core.clear_process_projection();
                return Err(error);
            }
        };
        let snapshot = authority.session();
        let committed = snapshot.commit();
        if committed.character_id() != character_id
            || committed.world_id() != world_id
            || committed.channel_id() != channel_id
        {
            self.core.clear_process_projection();
            return Err(core::AdmissionError::ReconciliationUnavailable);
        }
        if snapshot.current_character_lease() != character_lease {
            self.core.clear_process_projection();
            return Err(core::AdmissionError::StaleLease);
        }
        if snapshot.current_scope_generation() != scope_generation {
            self.core.clear_process_projection();
            return Err(core::AdmissionError::StaleRuntime);
        }
        if snapshot.session_state() != session_state
            || snapshot.current_connection_generation() != connection_generation
            || snapshot.current_transport() != current_transport
        {
            self.core.clear_process_projection();
            return Err(core::AdmissionError::StaleConnection);
        }
        if matches!(
            disposition,
            Some(core::ReconnectAttemptDisposition::Prepared { .. })
                | Some(core::ReconnectAttemptDisposition::Committed { .. })
        ) {
            let binding = authority
                .binding()
                .ok_or(core::AdmissionError::ReconciliationUnavailable)?;
            if binding.candidate_transport() != candidate_transport {
                return match disposition {
                    Some(core::ReconnectAttemptDisposition::Committed { .. }) => {
                        Err(core::AdmissionError::StaleConnection)
                    }
                    _ => Err(core::AdmissionError::AttemptMismatch),
                };
            }
            if matches!(
                disposition,
                Some(core::ReconnectAttemptDisposition::Prepared { .. })
            ) && let Err(error) = self
                .core
                .restore_reconciled_prepared_projection(attempt, binding)
            {
                self.core.clear_process_projection();
                return Err(error);
            }
        }
        Ok(disposition)
    }

    fn reconcile_committed_after_cleared_claim(
        &mut self,
        game_session_id: GameSessionId,
        attempt: core::ReconnectAttemptRef,
        candidate_transport: T,
    ) -> Result<Option<ConnectionGeneration>, core::AdmissionError> {
        let authority = match self
            .core
            .journal()
            .reconcile_reconnect_attempt(game_session_id, attempt)
        {
            Ok(authority) => authority,
            Err(error) => {
                self.core.clear_process_projection();
                return Err(error);
            }
        };
        let disposition = match validate_atomic_attempt_authority(game_session_id, authority) {
            Ok(disposition) => disposition,
            Err(error) => {
                self.core.clear_process_projection();
                return Err(error);
            }
        };
        let Some(core::ReconnectAttemptDisposition::Committed { generation }) = disposition else {
            return Ok(None);
        };
        let binding = authority
            .binding()
            .ok_or(core::AdmissionError::ReconciliationUnavailable)?;
        if binding.candidate_transport() != candidate_transport {
            return Err(core::AdmissionError::StaleConnection);
        }
        if let Err(error) = self
            .core
            .install_rehydrated_session(game_session_id, authority.session())
        {
            self.core.clear_process_projection();
            return Err(error);
        }
        Ok(Some(generation))
    }

    pub fn prepare_reconnect(
        &mut self,
        attempt: core::ReconnectAttemptRef,
        predecessor: ConnectionGeneration,
        candidate_transport: T,
        lease: u64,
        scope: u64,
    ) -> Result<ConnectionGeneration, core::AdmissionError> {
        match self.reconcile_current_attempt(attempt, candidate_transport)? {
            Some(core::ReconnectAttemptDisposition::Committed { generation }) => {
                return Ok(generation);
            }
            Some(core::ReconnectAttemptDisposition::TerminallySuperseded) => {
                return Err(core::AdmissionError::StaleConnection);
            }
            Some(core::ReconnectAttemptDisposition::Prepared {
                candidate_generation,
            }) => {
                return match self.core.prepared_reconnect_projection() {
                    Some((current_attempt, current_transport, current_generation))
                        if current_attempt == attempt
                            && current_transport == candidate_transport
                            && current_generation == candidate_generation =>
                    {
                        Ok(candidate_generation)
                    }
                    _ => Err(core::AdmissionError::ReconciliationUnavailable),
                };
            }
            None => {}
        }
        let game_session_id = self
            .core
            .current()
            .ok_or(core::AdmissionError::Terminal)?
            .game_session_id();
        match self
            .core
            .prepare_reconnect(attempt, predecessor, candidate_transport, lease, scope)
        {
            Err(core::AdmissionError::StaleConnection) if self.core.current().is_none() => {
                match self.reconcile_committed_after_cleared_claim(
                    game_session_id,
                    attempt,
                    candidate_transport,
                )? {
                    Some(generation) => Ok(generation),
                    None => Err(core::AdmissionError::StaleConnection),
                }
            }
            result => result,
        }
    }

    pub fn commit_reconnect(
        &mut self,
        attempt: core::ReconnectAttemptRef,
        candidate_transport: T,
        lease: u64,
        scope: u64,
    ) -> Result<ConnectionGeneration, core::AdmissionError> {
        match self.reconcile_current_attempt(attempt, candidate_transport)? {
            Some(core::ReconnectAttemptDisposition::Committed { generation }) => Ok(generation),
            Some(core::ReconnectAttemptDisposition::TerminallySuperseded) => {
                Err(core::AdmissionError::StaleConnection)
            }
            Some(core::ReconnectAttemptDisposition::Prepared {
                candidate_generation,
            }) => match self.core.prepared_reconnect_projection() {
                Some((current_attempt, current_transport, current_generation))
                    if current_attempt == attempt
                        && current_transport == candidate_transport
                        && current_generation == candidate_generation =>
                {
                    self.core
                        .commit_reconnect(attempt, candidate_transport, lease, scope)
                }
                _ => Err(core::AdmissionError::ReconciliationUnavailable),
            },
            None => Err(core::AdmissionError::AttemptMismatch),
        }
    }
}
