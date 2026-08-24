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