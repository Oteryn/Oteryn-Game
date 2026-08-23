use super::admission as core;
use super::{
    ConnectionGeneration, GameSessionAuthoritySnapshot, GameSessionId, ScopeOwnershipGeneration,
};

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
        <J as ReconnectAttemptJournal<T>>::lookup(self, game_session_id, attempt)
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

    pub fn prepare_reconnect(
        &mut self,
        attempt: core::ReconnectAttemptRef,
        predecessor: ConnectionGeneration,
        candidate_transport: T,
        lease: u64,
        scope: u64,
    ) -> Result<ConnectionGeneration, core::AdmissionError> {
        self.core
            .prepare_reconnect(attempt, predecessor, candidate_transport, lease, scope)
    }

    fn revalidate_current_session_authority(&mut self) -> Result<(), core::AdmissionError> {
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

        let snapshot = match self.core.journal().load_session(game_session_id) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.core.clear_process_projection();
                return Err(error);
            }
        };
        let committed = snapshot.commit();
        if committed.game_session_id() != game_session_id
            || committed.character_id() != character_id
            || committed.world_id() != world_id
            || committed.channel_id() != channel_id
        {
            self.core.clear_process_projection();
            return Err(core::AdmissionError::ReconciliationUnavailable);
        }
        if snapshot.session_state() == core::GameSessionState::Terminal {
            self.core.clear_process_projection();
            return Err(core::AdmissionError::Terminal);
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
        Ok(())
    }

    pub fn commit_reconnect(
        &mut self,
        attempt: core::ReconnectAttemptRef,
        candidate_transport: T,
        lease: u64,
        scope: u64,
    ) -> Result<ConnectionGeneration, core::AdmissionError> {
        // A recovered COMMITTED attempt may be replayed only while the durable
        // GameSession binding still matches this process projection. Re-read the
        // current fenced authority immediately before attempt reconciliation so
        // a terminal/control-loss/new-controller transition that happened after
        // rehydration cannot be masked by a stable COMMITTED journal outcome.
        self.revalidate_current_session_authority()?;
        self.core
            .commit_reconnect(attempt, candidate_transport, lease, scope)
    }
}
