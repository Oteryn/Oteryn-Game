use super::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

type AttemptKey = (GameSessionId, ReconnectAttemptRef);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TestAuthoritativeSessionBinding {
    state: GameSessionState,
    connection_generation: ConnectionGeneration,
    current_transport: Option<u64>,
}

#[derive(Default)]
struct TestReconnectAuthorityState {
    committed_grants: std::collections::BTreeMap<GrantReplayKey, FreshAdmissionCommit<u64>>,
    committed_session_ids: std::collections::BTreeSet<GameSessionId>,
    fail_next_fresh_response_after_commit: bool,
    fail_next_control_loss_response_after_commit: bool,
    fail_next_runtime_scope_response_after_commit: bool,
    last_control_losses: HashMap<GameSessionId, (u64, ConnectionGeneration)>,
    records: HashMap<AttemptKey, ReconnectAttemptDisposition>,
    bindings: HashMap<AttemptKey, ReconnectCommitBinding<u64>>,
    authoritative_fences: HashMap<GameSessionId, (CharacterLease, ScopeOwnershipGeneration)>,
    authoritative_sessions: HashMap<GameSessionId, TestAuthoritativeSessionBinding>,
}

#[derive(Clone, Default)]
struct TestReconnectAttemptJournal {
    state: Rc<RefCell<TestReconnectAuthorityState>>,
}

impl TestReconnectAttemptJournal {
    fn len(&self) -> usize {
        self.state.borrow().records.len()
    }

    fn set_authoritative_fences(
        &self,
        game_session_id: GameSessionId,
        character_lease: CharacterLease,
        scope_generation: ScopeOwnershipGeneration,
    ) {
        self.state
            .borrow_mut()
            .authoritative_fences
            .insert(game_session_id, (character_lease, scope_generation));
    }

    fn fail_next_fresh_response_after_commit(&self) {
        self.state
            .borrow_mut()
            .fail_next_fresh_response_after_commit = true;
    }

    fn fail_next_control_loss_response_after_commit(&self) {
        self.state
            .borrow_mut()
            .fail_next_control_loss_response_after_commit = true;
    }

    fn fail_next_runtime_scope_response_after_commit(&self) {
        self.state
            .borrow_mut()
            .fail_next_runtime_scope_response_after_commit = true;
    }

    fn set_authoritative_session(
        &self,
        game_session_id: GameSessionId,
        state: GameSessionState,
        connection_generation: ConnectionGeneration,
        current_transport: Option<u64>,
    ) {
        self.state.borrow_mut().authoritative_sessions.insert(
            game_session_id,
            TestAuthoritativeSessionBinding {
                state,
                connection_generation,
                current_transport,
            },
        );
    }
}

impl ReconnectAttemptJournal<u64> for TestReconnectAttemptJournal {
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
        if let Some(commit) = state.committed_grants.get(&replay_key).copied() {
            let session = state
                .authoritative_sessions
                .get(&commit.game_session_id())
                .copied()
                .ok_or(AdmissionError::ReconciliationUnavailable)?;
            return Ok(FreshAdmissionAuthoritySnapshot::new(
                commit,
                session.state,
                session.connection_generation,
                session.current_transport,
            ));
        }

        let game_session_id = issue_game_session_id()?;
        if state.committed_session_ids.contains(&game_session_id) {
            return Err(AdmissionError::InvalidFacts);
        }
        let commit =
            FreshAdmissionCommit::from_facts(game_session_id, facts, authenticated_transport)?;
        let scope_generation = ScopeOwnershipGeneration::new(commit.scope_ownership_generation())
            .map_err(|_| AdmissionError::InvalidFacts)?;
        state.committed_session_ids.insert(game_session_id);
        state.committed_grants.insert(replay_key, commit);
        state.authoritative_fences.insert(
            game_session_id,
            (
                CharacterLease {
                    character_id: commit.character_id(),
                    generation: commit.character_lease_generation(),
                },
                scope_generation,
            ),
        );
        state.authoritative_sessions.insert(
            game_session_id,
            TestAuthoritativeSessionBinding {
                state: GameSessionState::Active,
                connection_generation: commit.connection_generation(),
                current_transport: Some(authenticated_transport),
            },
        );
        if state.fail_next_fresh_response_after_commit {
            state.fail_next_fresh_response_after_commit = false;
            return Err(AdmissionError::ReconciliationUnavailable);
        }
        Ok(FreshAdmissionAuthoritySnapshot::active(commit))
    }

    fn mark_control_loss(
        &self,
        game_session_id: GameSessionId,
        observed_transport: u64,
        observed_generation: ConnectionGeneration,
    ) -> Result<ControlLossDisposition, AdmissionError> {
        let mut state = self.state.borrow_mut();
        let session = state
            .authoritative_sessions
            .get(&game_session_id)
            .copied()
            .ok_or(AdmissionError::ReconciliationUnavailable)?;
        if session.state == GameSessionState::Terminal {
            return Err(AdmissionError::Terminal);
        }
        if session.state == GameSessionState::Reconnectable
            && session.connection_generation == observed_generation
            && session.current_transport.is_none()
        {
            return if state.last_control_losses.get(&game_session_id).copied()
                == Some((observed_transport, observed_generation))
            {
                Ok(ControlLossDisposition::Applied)
            } else {
                Ok(ControlLossDisposition::StaleIgnored)
            };
        }
        if session.current_transport != Some(observed_transport)
            || session.connection_generation != observed_generation
        {
            return Ok(ControlLossDisposition::StaleIgnored);
        }
        state.authoritative_sessions.insert(
            game_session_id,
            TestAuthoritativeSessionBinding {
                state: GameSessionState::Reconnectable,
                connection_generation: observed_generation,
                current_transport: None,
            },
        );
        state
            .last_control_losses
            .insert(game_session_id, (observed_transport, observed_generation));
        let prepared_keys: Vec<_> = state
            .records
            .iter()
            .filter_map(|(key, disposition)| {
                (key.0 == game_session_id
                    && matches!(disposition, ReconnectAttemptDisposition::Prepared { .. }))
                .then_some(*key)
            })
            .collect();
        for key in prepared_keys {
            state
                .records
                .insert(key, ReconnectAttemptDisposition::TerminallySuperseded);
            state.bindings.remove(&key);
        }
        if state.fail_next_control_loss_response_after_commit {
            state.fail_next_control_loss_response_after_commit = false;
            return Err(AdmissionError::ReconciliationUnavailable);
        }
        Ok(ControlLossDisposition::Applied)
    }

    fn terminate_session(
        &self,
        game_session_id: GameSessionId,
        expected_generation: ConnectionGeneration,
    ) -> Result<(), AdmissionError> {
        let mut state = self.state.borrow_mut();
        let session = state
            .authoritative_sessions
            .get(&game_session_id)
            .copied()
            .ok_or(AdmissionError::ReconciliationUnavailable)?;
        if session.connection_generation != expected_generation {
            return Err(AdmissionError::StaleConnection);
        }
        state.authoritative_sessions.insert(
            game_session_id,
            TestAuthoritativeSessionBinding {
                state: GameSessionState::Terminal,
                connection_generation: expected_generation,
                current_transport: None,
            },
        );
        let prepared_keys: Vec<_> = state
            .records
            .iter()
            .filter_map(|(key, disposition)| {
                (key.0 == game_session_id
                    && matches!(disposition, ReconnectAttemptDisposition::Prepared { .. }))
                .then_some(*key)
            })
            .collect();
        for key in prepared_keys {
            state
                .records
                .insert(key, ReconnectAttemptDisposition::TerminallySuperseded);
            state.bindings.remove(&key);
        }
        Ok(())
    }

    fn advance_runtime_scope(
        &self,
        game_session_id: GameSessionId,
        expected_current: ScopeOwnershipGeneration,
        observed: ScopeOwnershipGeneration,
    ) -> Result<ScopeOwnershipGeneration, AdmissionError> {
        let mut state = self.state.borrow_mut();
        let session = state
            .authoritative_sessions
            .get(&game_session_id)
            .copied()
            .ok_or(AdmissionError::ReconciliationUnavailable)?;
        if session.state == GameSessionState::Terminal {
            return Err(AdmissionError::Terminal);
        }
        let fences = state
            .authoritative_fences
            .get(&game_session_id)
            .copied()
            .ok_or(AdmissionError::ReconciliationUnavailable)?;
        let authoritative = fences.1;
        if observed < expected_current {
            return Err(AdmissionError::StaleRuntime);
        }
        if authoritative < expected_current {
            return Err(AdmissionError::ReconciliationUnavailable);
        }
        if authoritative >= observed {
            return Ok(authoritative);
        }

        state
            .authoritative_fences
            .insert(game_session_id, (fences.0, observed));
        let prepared_keys: Vec<_> = state
            .records
            .iter()
            .filter_map(|(key, disposition)| {
                (key.0 == game_session_id
                    && matches!(disposition, ReconnectAttemptDisposition::Prepared { .. }))
                .then_some(*key)
            })
            .collect();
        for key in prepared_keys {
            state
                .records
                .insert(key, ReconnectAttemptDisposition::TerminallySuperseded);
            state.bindings.remove(&key);
        }
        if state.fail_next_runtime_scope_response_after_commit {
            state.fail_next_runtime_scope_response_after_commit = false;
            return Err(AdmissionError::ReconciliationUnavailable);
        }
        Ok(observed)
    }

    fn lookup(
        &self,
        game_session_id: GameSessionId,
        attempt: ReconnectAttemptRef,
    ) -> Result<Option<ReconnectAttemptDisposition>, AdmissionError> {
        Ok(self
            .state
            .borrow()
            .records
            .get(&(game_session_id, attempt))
            .copied())
    }

    fn claim_prepared(
        &self,
        game_session_id: GameSessionId,
        attempt: ReconnectAttemptRef,
        binding: ReconnectCommitBinding<u64>,
    ) -> Result<ReconnectAttemptClaim, AdmissionError> {
        let mut state = self.state.borrow_mut();
        let key = (game_session_id, attempt);
        if let Some(disposition) = state.records.get(&key).copied() {
            return Ok(ReconnectAttemptClaim::Existing(disposition));
        }
        let session = state
            .authoritative_sessions
            .get(&game_session_id)
            .copied()
            .ok_or(AdmissionError::ReconciliationUnavailable)?;
        let fences = state
            .authoritative_fences
            .get(&game_session_id)
            .copied()
            .ok_or(AdmissionError::ReconciliationUnavailable)?;

        let claim_error = if session.state == GameSessionState::Terminal {
            Some(AdmissionError::Terminal)
        } else if session.state == GameSessionState::Active && session.current_transport.is_some() {
            Some(AdmissionError::IncumbentHealthy)
        } else if session.state != GameSessionState::Reconnectable
            || session.current_transport.is_some()
            || session.connection_generation != binding.predecessor_generation()
            || binding.predecessor_generation().get().checked_add(1)
                != Some(binding.candidate_generation().get())
        {
            Some(AdmissionError::StaleConnection)
        } else if fences.0 != binding.character_lease() {
            Some(AdmissionError::StaleLease)
        } else if fences.1 != binding.scope_generation() {
            Some(AdmissionError::StaleRuntime)
        } else {
            None
        };
        if let Some(error) = claim_error {
            state
                .records
                .insert(key, ReconnectAttemptDisposition::TerminallySuperseded);
            state.bindings.remove(&key);
            return Err(error);
        }

        if state.records.iter().any(|((session, _), disposition)| {
            *session == game_session_id
                && matches!(disposition, ReconnectAttemptDisposition::Prepared { .. })
        }) {
            state
                .records
                .insert(key, ReconnectAttemptDisposition::TerminallySuperseded);
            return Ok(ReconnectAttemptClaim::RejectedConcurrent);
        }
        state.records.insert(
            key,
            ReconnectAttemptDisposition::Prepared {
                candidate_generation: binding.candidate_generation(),
            },
        );
        state.bindings.insert(key, binding);
        Ok(ReconnectAttemptClaim::Claimed)
    }

    fn retire_if_unseen(
        &self,
        game_session_id: GameSessionId,
        attempt: ReconnectAttemptRef,
    ) -> Result<ReconnectAttemptDisposition, AdmissionError> {
        let mut state = self.state.borrow_mut();
        let key = (game_session_id, attempt);
        if let Some(disposition) = state.records.get(&key).copied() {
            return Ok(disposition);
        }
        state
            .records
            .insert(key, ReconnectAttemptDisposition::TerminallySuperseded);
        Ok(ReconnectAttemptDisposition::TerminallySuperseded)
    }

    fn commit_prepared(
        &self,
        game_session_id: GameSessionId,
        attempt: ReconnectAttemptRef,
        binding: ReconnectCommitBinding<u64>,
    ) -> Result<(), AdmissionError> {
        let mut state = self.state.borrow_mut();
        let key = (game_session_id, attempt);
        let disposition = state
            .records
            .get(&key)
            .copied()
            .ok_or(AdmissionError::ReconciliationUnavailable)?;

        if let ReconnectAttemptDisposition::Committed { generation } = disposition {
            if generation != binding.candidate_generation()
                || state.bindings.get(&key).copied() != Some(binding)
            {
                return Err(AdmissionError::StaleConnection);
            }
            let session = state
                .authoritative_sessions
                .get(&game_session_id)
                .copied()
                .ok_or(AdmissionError::ReconciliationUnavailable)?;
            if session.state == GameSessionState::Terminal {
                return Err(AdmissionError::Terminal);
            }
            return if session.state == GameSessionState::Active
                && session.connection_generation == binding.candidate_generation()
                && session.current_transport == Some(binding.candidate_transport())
            {
                Ok(())
            } else {
                Err(AdmissionError::StaleConnection)
            };
        }
        if disposition == ReconnectAttemptDisposition::TerminallySuperseded {
            return Err(AdmissionError::StaleConnection);
        }

        let ReconnectAttemptDisposition::Prepared {
            candidate_generation,
        } = disposition
        else {
            return Err(AdmissionError::ReconciliationUnavailable);
        };
        if candidate_generation != binding.candidate_generation()
            || state.bindings.get(&key).copied() != Some(binding)
        {
            state
                .records
                .insert(key, ReconnectAttemptDisposition::TerminallySuperseded);
            state.bindings.remove(&key);
            return Err(AdmissionError::StaleConnection);
        }

        let session = state
            .authoritative_sessions
            .get(&game_session_id)
            .copied()
            .ok_or(AdmissionError::ReconciliationUnavailable)?;
        let fences = state
            .authoritative_fences
            .get(&game_session_id)
            .copied()
            .ok_or(AdmissionError::ReconciliationUnavailable)?;

        if session.state == GameSessionState::Terminal {
            state
                .records
                .insert(key, ReconnectAttemptDisposition::TerminallySuperseded);
            state.bindings.remove(&key);
            return Err(AdmissionError::Terminal);
        }
        if session.state != GameSessionState::Reconnectable
            || session.connection_generation != binding.predecessor_generation()
            || session.current_transport.is_some()
            || binding.predecessor_generation().get().checked_add(1)
                != Some(binding.candidate_generation().get())
        {
            state
                .records
                .insert(key, ReconnectAttemptDisposition::TerminallySuperseded);
            state.bindings.remove(&key);
            return Err(AdmissionError::StaleConnection);
        }
        if fences.0 != binding.character_lease() {
            state
                .records
                .insert(key, ReconnectAttemptDisposition::TerminallySuperseded);
            state.bindings.remove(&key);
            return Err(AdmissionError::StaleLease);
        }
        if fences.1 != binding.scope_generation() {
            state
                .records
                .insert(key, ReconnectAttemptDisposition::TerminallySuperseded);
            state.bindings.remove(&key);
            return Err(AdmissionError::StaleRuntime);
        }

        state.records.insert(
            key,
            ReconnectAttemptDisposition::Committed {
                generation: binding.candidate_generation(),
            },
        );
        state.authoritative_sessions.insert(
            game_session_id,
            TestAuthoritativeSessionBinding {
                state: GameSessionState::Active,
                connection_generation: binding.candidate_generation(),
                current_transport: Some(binding.candidate_transport()),
            },
        );
        Ok(())
    }

    fn retire_prepared(
        &self,
        game_session_id: GameSessionId,
        attempt: ReconnectAttemptRef,
        candidate_generation: ConnectionGeneration,
    ) -> Result<(), AdmissionError> {
        let mut state = self.state.borrow_mut();
        let key = (game_session_id, attempt);
        let disposition = state
            .records
            .get(&key)
            .copied()
            .ok_or(AdmissionError::ReconciliationUnavailable)?;
        match disposition {
            ReconnectAttemptDisposition::Prepared {
                candidate_generation: prepared_generation,
            } if prepared_generation == candidate_generation => {
                state
                    .records
                    .insert(key, ReconnectAttemptDisposition::TerminallySuperseded);
                state.bindings.remove(&key);
                Ok(())
            }
            ReconnectAttemptDisposition::TerminallySuperseded => Ok(()),
            _ => Err(AdmissionError::ReconciliationUnavailable),
        }
    }
}

type TestAdmissionAuthority = AdmissionAuthority<u64, TestReconnectAttemptJournal>;

fn test_authority() -> TestAdmissionAuthority {
    AdmissionAuthority::new(TestReconnectAttemptJournal::default())
}

fn lose_current(authority: &mut TestAdmissionAuthority) -> Result<(), AdmissionError> {
    let transport = authority
        .current_transport()
        .ok_or(AdmissionError::StaleConnection)?;
    let generation = authority
        .current()
        .map(GameSession::connection_generation)
        .ok_or(AdmissionError::Terminal)?;
    assert_eq!(
        authority.mark_unexpected_control_loss(transport, generation)?,
        ControlLossDisposition::Applied
    );
    Ok(())
}

fn raw_uuid_v7(value: u64) -> [u8; 16] {
    let mut raw = [0u8; 16];
    raw[8..].copy_from_slice(&value.to_be_bytes());
    raw[6] = (raw[6] & 0x0f) | 0x70;
    raw[8] = (raw[8] & 0x3f) | 0x80;
    raw
}

fn game_session_id(value: u64) -> Result<GameSessionId, AdmissionError> {
    GameSessionId::decode(&raw_uuid_v7(value)).map_err(|_| AdmissionError::InvalidFacts)
}

fn facts(nonce: u64) -> Result<FreshAdmissionFacts, AdmissionError> {
    FreshAdmissionFacts::for_test(nonce, 7, 11)
}

fn reconnect_binding_for_test(
    predecessor: u64,
    candidate: u64,
    transport: u64,
) -> Result<ReconnectCommitBinding<u64>, AdmissionError> {
    let admission = facts(999)?;
    Ok(ReconnectCommitBinding::new(
        ConnectionGeneration::new(predecessor).map_err(|_| AdmissionError::InvalidFacts)?,
        ConnectionGeneration::new(candidate).map_err(|_| AdmissionError::InvalidFacts)?,
        transport,
        CharacterLease {
            character_id: admission.character_id,
            generation: admission.character_lease_generation,
        },
        ScopeOwnershipGeneration::new(admission.scope_ownership_generation)
            .map_err(|_| AdmissionError::InvalidFacts)?,
    ))
}

fn admit(
    authority: &mut TestAdmissionAuthority,
    nonce: u64,
    session: u64,
    transport: u64,
) -> Result<&GameSession, AdmissionError> {
    let admission = facts(nonce)?;
    let game_session_id = game_session_id(session)?;
    authority.commit_fresh(admission, transport, || Ok(game_session_id))?;
    authority.current().ok_or(AdmissionError::Terminal)
}
