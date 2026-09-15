use crate::foundation::{
    AdmissionAuthority, AdmissionError, ChannelId, CharacterId, CharacterLease,
    ConnectionGeneration, ControlLossDisposition, FreshAdmissionAuthoritySnapshot,
    FreshAdmissionCommit, FreshAdmissionFacts, FreshAdmissionReplayKey,
    GameSessionAuthoritySnapshot, GameSessionId, GameSessionState,
    ReconnectAttemptAuthoritySnapshot, ReconnectAttemptClaim, ReconnectAttemptDisposition,
    ReconnectAttemptJournal, ReconnectAttemptRef, ReconnectCommitBinding, ScopeOwnershipGeneration,
    WorldId,
};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Copy)]
struct SessionState {
    replay_key: FreshAdmissionReplayKey,
    commit: FreshAdmissionCommit<u64>,
    state: GameSessionState,
    generation: ConnectionGeneration,
    transport: Option<u64>,
    lease: CharacterLease,
    scope: ScopeOwnershipGeneration,
}

#[derive(Default)]
struct DurableState {
    session: Option<SessionState>,
    attempt: Option<ReconnectAttemptRef>,
    disposition: Option<ReconnectAttemptDisposition>,
    binding: Option<ReconnectCommitBinding<u64>>,
}

#[derive(Clone, Default)]
struct RollbackJournal {
    state: Rc<RefCell<DurableState>>,
}

impl RollbackJournal {
    fn snapshot(session: SessionState) -> GameSessionAuthoritySnapshot<u64> {
        GameSessionAuthoritySnapshot::new(
            session.commit,
            session.state,
            session.generation,
            session.transport,
            session.lease,
            session.scope,
        )
    }

    fn set_scope(&self, raw: u64) -> Result<(), AdmissionError> {
        let scope = ScopeOwnershipGeneration::new(raw).map_err(|_| AdmissionError::InvalidFacts)?;
        self.state
            .borrow_mut()
            .session
            .as_mut()
            .ok_or(AdmissionError::ReconciliationUnavailable)?
            .scope = scope;
        Ok(())
    }
}

impl ReconnectAttemptJournal<u64> for RollbackJournal {
    fn commit_fresh<F>(
        &self,
        facts: FreshAdmissionFacts,
        authenticated_transport: u64,
        issue_game_session_id: F,
    ) -> Result<FreshAdmissionAuthoritySnapshot<u64>, AdmissionError>
    where
        F: FnOnce() -> Result<GameSessionId, AdmissionError>,
    {
        let replay_key = facts.replay_key();
        let mut state = self.state.borrow_mut();
        if let Some(session) = state.session {
            if session.replay_key != replay_key {
                return Err(AdmissionError::GrantReplayed);
            }
            return Ok(FreshAdmissionAuthoritySnapshot::new(
                session.commit,
                session.state,
                session.generation,
                session.transport,
            ));
        }
        let game_session_id = issue_game_session_id()?;
        let commit =
            FreshAdmissionCommit::from_facts(game_session_id, facts, authenticated_transport)?;
        let lease =
            CharacterLease::new(commit.character_id(), commit.character_lease_generation())?;
        let scope = ScopeOwnershipGeneration::new(commit.scope_ownership_generation())
            .map_err(|_| AdmissionError::InvalidFacts)?;
        state.session = Some(SessionState {
            replay_key,
            commit,
            state: GameSessionState::Active,
            generation: commit.connection_generation(),
            transport: Some(authenticated_transport),
            lease,
            scope,
        });
        Ok(FreshAdmissionAuthoritySnapshot::active(commit))
    }

    fn load_session(
        &self,
        game_session_id: GameSessionId,
    ) -> Result<GameSessionAuthoritySnapshot<u64>, AdmissionError> {
        let session = self
            .state
            .borrow()
            .session
            .ok_or(AdmissionError::ReconciliationUnavailable)?;
        if session.commit.game_session_id() != game_session_id {
            return Err(AdmissionError::ReconciliationUnavailable);
        }
        Ok(Self::snapshot(session))
    }

    fn reconcile_reconnect_attempt(
        &self,
        game_session_id: GameSessionId,
        attempt: ReconnectAttemptRef,
    ) -> Result<ReconnectAttemptAuthoritySnapshot<u64>, AdmissionError> {
        let state = self.state.borrow();
        let session = state
            .session
            .ok_or(AdmissionError::ReconciliationUnavailable)?;
        if session.commit.game_session_id() != game_session_id {
            return Err(AdmissionError::ReconciliationUnavailable);
        }
        let disposition = if state.attempt == Some(attempt) {
            state.disposition
        } else {
            None
        };
        let binding = if state.attempt == Some(attempt) {
            state.binding
        } else {
            None
        };
        Ok(ReconnectAttemptAuthoritySnapshot::new(
            Self::snapshot(session),
            disposition,
            binding,
        ))
    }

    fn mark_control_loss(
        &self,
        game_session_id: GameSessionId,
        observed_transport: u64,
        observed_generation: ConnectionGeneration,
    ) -> Result<ControlLossDisposition, AdmissionError> {
        let mut state = self.state.borrow_mut();
        let session = state
            .session
            .as_mut()
            .ok_or(AdmissionError::ReconciliationUnavailable)?;
        if session.commit.game_session_id() != game_session_id {
            return Err(AdmissionError::ReconciliationUnavailable);
        }
        if session.state != GameSessionState::Active
            || session.generation != observed_generation
            || session.transport != Some(observed_transport)
        {
            return Ok(ControlLossDisposition::StaleIgnored);
        }
        session.state = GameSessionState::Reconnectable;
        session.transport = None;
        Ok(ControlLossDisposition::Applied)
    }

    fn terminate_session(
        &self,
        _game_session_id: GameSessionId,
        _expected_generation: ConnectionGeneration,
    ) -> Result<(), AdmissionError> {
        Err(AdmissionError::ReconciliationUnavailable)
    }

    fn advance_runtime_scope(
        &self,
        game_session_id: GameSessionId,
        expected_current: ScopeOwnershipGeneration,
        observed: ScopeOwnershipGeneration,
    ) -> Result<ScopeOwnershipGeneration, AdmissionError> {
        let mut state = self.state.borrow_mut();
        let session = state
            .session
            .as_mut()
            .ok_or(AdmissionError::ReconciliationUnavailable)?;
        if session.commit.game_session_id() != game_session_id
            || session.scope != expected_current
            || observed <= expected_current
        {
            return Err(AdmissionError::StaleRuntime);
        }
        session.scope = observed;
        Ok(observed)
    }

    fn lookup(
        &self,
        _game_session_id: GameSessionId,
        attempt: ReconnectAttemptRef,
    ) -> Result<Option<ReconnectAttemptDisposition>, AdmissionError> {
        let state = self.state.borrow();
        Ok((state.attempt == Some(attempt))
            .then_some(state.disposition)
            .flatten())
    }

    fn claim_prepared(
        &self,
        game_session_id: GameSessionId,
        attempt: ReconnectAttemptRef,
        binding: ReconnectCommitBinding<u64>,
    ) -> Result<ReconnectAttemptClaim, AdmissionError> {
        let mut state = self.state.borrow_mut();
        if state.attempt.is_some() {
            return Err(AdmissionError::ReconciliationUnavailable);
        }
        let session = state
            .session
            .ok_or(AdmissionError::ReconciliationUnavailable)?;
        if session.commit.game_session_id() != game_session_id {
            return Err(AdmissionError::ReconciliationUnavailable);
        }
        if session.state == GameSessionState::Terminal {
            return Err(AdmissionError::Terminal);
        }
        if session.state == GameSessionState::Active && session.transport.is_some() {
            return Err(AdmissionError::IncumbentHealthy);
        }
        if session.state != GameSessionState::Reconnectable
            || session.transport.is_some()
            || session.generation != binding.predecessor_generation()
            || binding.predecessor_generation().get().checked_add(1)
                != Some(binding.candidate_generation().get())
        {
            return Err(AdmissionError::StaleConnection);
        }
        if session.lease != binding.character_lease() {
            return Err(AdmissionError::StaleLease);
        }
        if session.scope != binding.scope_generation() {
            return Err(AdmissionError::StaleRuntime);
        }
        state.attempt = Some(attempt);
        state.disposition = Some(ReconnectAttemptDisposition::Prepared {
            candidate_generation: binding.candidate_generation(),
        });
        state.binding = Some(binding);
        Ok(ReconnectAttemptClaim::Claimed)
    }

    fn retire_if_unseen(
        &self,
        _game_session_id: GameSessionId,
        _attempt: ReconnectAttemptRef,
    ) -> Result<ReconnectAttemptDisposition, AdmissionError> {
        Err(AdmissionError::ReconciliationUnavailable)
    }

    fn commit_prepared(
        &self,
        game_session_id: GameSessionId,
        attempt: ReconnectAttemptRef,
        binding: ReconnectCommitBinding<u64>,
    ) -> Result<(), AdmissionError> {
        let mut state = self.state.borrow_mut();
        if state.attempt != Some(attempt) || state.binding != Some(binding) {
            return Err(AdmissionError::StaleConnection);
        }
        let session = state
            .session
            .as_mut()
            .ok_or(AdmissionError::ReconciliationUnavailable)?;
        if session.commit.game_session_id() != game_session_id
            || session.state != GameSessionState::Reconnectable
            || session.generation != binding.predecessor_generation()
            || session.transport.is_some()
        {
            return Err(AdmissionError::StaleConnection);
        }
        if session.lease != binding.character_lease() {
            return Err(AdmissionError::StaleLease);
        }
        if session.scope != binding.scope_generation() {
            return Err(AdmissionError::StaleRuntime);
        }
        session.state = GameSessionState::Active;
        session.generation = binding.candidate_generation();
        session.transport = Some(binding.candidate_transport());
        state.disposition = Some(ReconnectAttemptDisposition::Committed {
            generation: binding.candidate_generation(),
        });
        Ok(())
    }

    fn retire_prepared(
        &self,
        _game_session_id: GameSessionId,
        _attempt: ReconnectAttemptRef,
        _candidate_generation: ConnectionGeneration,
    ) -> Result<(), AdmissionError> {
        Err(AdmissionError::ReconciliationUnavailable)
    }
}

fn raw_uuid_v7(value: u64) -> [u8; 16] {
    let mut raw = [0u8; 16];
    raw[8..].copy_from_slice(&value.to_be_bytes());
    raw[6] = (raw[6] & 0x0f) | 0x70;
    raw[8] = (raw[8] & 0x3f) | 0x80;
    raw
}

fn facts() -> Result<FreshAdmissionFacts, AdmissionError> {
    let map = |_| AdmissionError::InvalidFacts;
    FreshAdmissionFacts::new(
        [9; 32],
        CharacterId::decode(&raw_uuid_v7(1)).map_err(map)?,
        WorldId::decode(&raw_uuid_v7(2)).map_err(map)?,
        ChannelId::decode(&raw_uuid_v7(3)).map_err(map)?,
        7,
        11,
    )
}

fn session_id() -> Result<GameSessionId, AdmissionError> {
    GameSessionId::decode(&raw_uuid_v7(900)).map_err(|_| AdmissionError::InvalidFacts)
}

#[test]
fn committed_replay_rejects_runtime_rollback_below_commit_binding() -> Result<(), AdmissionError> {
    let journal = RollbackJournal::default();
    let game_session_id = session_id()?;
    let mut original = AdmissionAuthority::new(journal.clone());
    original.commit_fresh(facts()?, 100, || Ok(game_session_id))?;
    original.observe_runtime_ownership_generation(12)?;

    let generation_one = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
    assert_eq!(
        original.mark_unexpected_control_loss(100, generation_one)?,
        ControlLossDisposition::Applied
    );
    let attempt = ReconnectAttemptRef::new(900)?;
    let generation_two = original.prepare_reconnect(attempt, generation_one, 200, 7, 12)?;
    assert_eq!(
        original.commit_reconnect(attempt, 200, 7, 12)?,
        generation_two
    );
    drop(original);

    // Adversarial durable rollback: still above the admission-time generation
    // (11) but below the generation (12) that was atomically bound to COMMIT.
    journal.set_scope(11)?;
    let mut recovered = AdmissionAuthority::new(journal);
    recovered.rehydrate_session(game_session_id)?;
    assert_eq!(
        recovered.commit_reconnect(attempt, 200, 7, 11),
        Err(AdmissionError::StaleRuntime)
    );
    Ok(())
}
