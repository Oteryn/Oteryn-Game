use crate::foundation::{
    AdmissionAuthority, AdmissionError, ChannelId, CharacterId, CharacterLease,
    ConnectionGeneration, ControlLossDisposition, FreshAdmissionAuthoritySnapshot,
    FreshAdmissionCommit, FreshAdmissionFacts, FreshAdmissionReplayKey, GameSessionAuthoritySnapshot,
    GameSessionId, GameSessionState, ReconnectAttemptClaim, ReconnectAttemptDisposition,
    ReconnectAttemptJournal, ReconnectAttemptRef, ReconnectCommitBinding, ScopeOwnershipGeneration,
    WorldId,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, Clone, Copy)]
struct DurableSession {
    replay_key: FreshAdmissionReplayKey,
    commit: FreshAdmissionCommit<u64>,
    state: GameSessionState,
    connection_generation: ConnectionGeneration,
    current_transport: Option<u64>,
    lease_generation: u64,
    scope_generation: ScopeOwnershipGeneration,
}

type AttemptKey = (GameSessionId, ReconnectAttemptRef);

#[derive(Default)]
struct RecoveryState {
    session: Option<DurableSession>,
    dispositions: HashMap<AttemptKey, ReconnectAttemptDisposition>,
    bindings: HashMap<AttemptKey, ReconnectCommitBinding<u64>>,
}

#[derive(Clone, Default)]
struct RecoveryJournal {
    state: Rc<RefCell<RecoveryState>>,
}

impl RecoveryJournal {
    fn set_current_lease(&self, generation: u64) -> Result<(), AdmissionError> {
        self.state
            .borrow_mut()
            .session
            .as_mut()
            .ok_or(AdmissionError::ReconciliationUnavailable)?
            .lease_generation = generation;
        Ok(())
    }

    fn commit_stored_attempt(
        &self,
        game_session_id: GameSessionId,
        attempt: ReconnectAttemptRef,
    ) -> Result<(), AdmissionError> {
        let binding = self
            .state
            .borrow()
            .bindings
            .get(&(game_session_id, attempt))
            .copied()
            .ok_or(AdmissionError::ReconciliationUnavailable)?;
        self.commit_prepared(game_session_id, attempt, binding)
    }
}

impl ReconnectAttemptJournal<u64> for RecoveryJournal {
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
                session.connection_generation,
                session.current_transport,
            ));
        }

        let game_session_id = issue_game_session_id()?;
        let commit =
            FreshAdmissionCommit::from_facts(game_session_id, facts, authenticated_transport)?;
        let scope_generation = ScopeOwnershipGeneration::new(commit.scope_ownership_generation())
            .map_err(|_| AdmissionError::InvalidFacts)?;
        state.session = Some(DurableSession {
            replay_key,
            commit,
            state: GameSessionState::Active,
            connection_generation: commit.connection_generation(),
            current_transport: Some(authenticated_transport),
            lease_generation: commit.character_lease_generation(),
            scope_generation,
        });
        Ok(FreshAdmissionAuthoritySnapshot::active(commit))
    }

    fn load_session(
        &self,
        game_session_id: GameSessionId,
    ) -> Result<GameSessionAuthoritySnapshot<u64>, AdmissionError> {
        let state = self.state.borrow();
        let session = state
            .session
            .ok_or(AdmissionError::ReconciliationUnavailable)?;
        if session.commit.game_session_id() != game_session_id {
            return Err(AdmissionError::ReconciliationUnavailable);
        }
        let lease = CharacterLease::new(session.commit.character_id(), session.lease_generation)?;
        Ok(GameSessionAuthoritySnapshot::new(
            session.commit,
            session.state,
            session.connection_generation,
            session.current_transport,
            lease,
            session.scope_generation,
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
        if session.state == GameSessionState::Terminal {
            return Err(AdmissionError::Terminal);
        }
        if session.state == GameSessionState::Reconnectable
            && session.connection_generation == observed_generation
            && session.current_transport.is_none()
        {
            return Ok(ControlLossDisposition::Applied);
        }
        if session.state != GameSessionState::Active
            || session.connection_generation != observed_generation
            || session.current_transport != Some(observed_transport)
        {
            return Ok(ControlLossDisposition::StaleIgnored);
        }
        session.state = GameSessionState::Reconnectable;
        session.current_transport = None;
        Ok(ControlLossDisposition::Applied)
    }

    fn terminate_session(
        &self,
        game_session_id: GameSessionId,
        expected_generation: ConnectionGeneration,
    ) -> Result<(), AdmissionError> {
        let mut state = self.state.borrow_mut();
        let session = state
            .session
            .as_mut()
            .ok_or(AdmissionError::ReconciliationUnavailable)?;
        if session.commit.game_session_id() != game_session_id
            || session.connection_generation != expected_generation
        {
            return Err(AdmissionError::StaleConnection);
        }
        session.state = GameSessionState::Terminal;
        session.current_transport = None;
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
            .session
            .as_mut()
            .ok_or(AdmissionError::ReconciliationUnavailable)?;
        if session.commit.game_session_id() != game_session_id {
            return Err(AdmissionError::ReconciliationUnavailable);
        }
        if session.scope_generation < expected_current {
            return Err(AdmissionError::ReconciliationUnavailable);
        }
        if observed < expected_current {
            return Err(AdmissionError::StaleRuntime);
        }
        if observed > session.scope_generation {
            session.scope_generation = observed;
        }
        Ok(session.scope_generation)
    }

    fn lookup(
        &self,
        game_session_id: GameSessionId,
        attempt: ReconnectAttemptRef,
    ) -> Result<Option<ReconnectAttemptDisposition>, AdmissionError> {
        Ok(self
            .state
            .borrow()
            .dispositions
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
        if let Some(disposition) = state.dispositions.get(&key).copied() {
            return Ok(ReconnectAttemptClaim::Existing(disposition));
        }
        if state
            .dispositions
            .iter()
            .any(|((session, _), disposition)| {
                *session == game_session_id
                    && matches!(disposition, ReconnectAttemptDisposition::Prepared { .. })
            })
        {
            state
                .dispositions
                .insert(key, ReconnectAttemptDisposition::TerminallySuperseded);
            return Ok(ReconnectAttemptClaim::RejectedConcurrent);
        }
        state.dispositions.insert(
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
        if let Some(disposition) = state.dispositions.get(&key).copied() {
            return Ok(disposition);
        }
        state
            .dispositions
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
            .dispositions
            .get(&key)
            .copied()
            .ok_or(AdmissionError::ReconciliationUnavailable)?;
        if let ReconnectAttemptDisposition::Committed { generation } = disposition {
            return if generation == binding.candidate_generation() {
                Ok(())
            } else {
                Err(AdmissionError::StaleConnection)
            };
        }
        if disposition == ReconnectAttemptDisposition::TerminallySuperseded
            || state.bindings.get(&key).copied() != Some(binding)
        {
            return Err(AdmissionError::StaleConnection);
        }

        let session = state
            .session
            .as_mut()
            .ok_or(AdmissionError::ReconciliationUnavailable)?;
        if session.commit.game_session_id() != game_session_id
            || session.state != GameSessionState::Reconnectable
            || session.current_transport.is_some()
            || session.connection_generation != binding.predecessor_generation()
        {
            return Err(AdmissionError::StaleConnection);
        }
        if session.lease_generation != binding.character_lease().generation() {
            return Err(AdmissionError::StaleLease);
        }
        if session.scope_generation != binding.scope_generation() {
            return Err(AdmissionError::StaleRuntime);
        }

        session.state = GameSessionState::Active;
        session.connection_generation = binding.candidate_generation();
        session.current_transport = Some(binding.candidate_transport());
        state.dispositions.insert(
            key,
            ReconnectAttemptDisposition::Committed {
                generation: binding.candidate_generation(),
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
        match state.dispositions.get(&key).copied() {
            Some(ReconnectAttemptDisposition::Prepared {
                candidate_generation: current,
            }) if current == candidate_generation => {
                state
                    .dispositions
                    .insert(key, ReconnectAttemptDisposition::TerminallySuperseded);
                state.bindings.remove(&key);
                Ok(())
            }
            Some(ReconnectAttemptDisposition::TerminallySuperseded) => Ok(()),
            _ => Err(AdmissionError::ReconciliationUnavailable),
        }
    }
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
    let map = |_| AdmissionError::InvalidFacts;
    let mut grant_nonce = [0u8; 32];
    grant_nonce[24..].copy_from_slice(&nonce.to_be_bytes());
    FreshAdmissionFacts::new(
        grant_nonce,
        CharacterId::decode(&raw_uuid_v7(1)).map_err(map)?,
        WorldId::decode(&raw_uuid_v7(2)).map_err(map)?,
        ChannelId::decode(&raw_uuid_v7(3)).map_err(map)?,
        7,
        11,
    )
}

#[test]
fn fresh_reconciliation_restores_current_runtime_and_lease_authority() -> Result<(), AdmissionError>
{
    let journal = RecoveryJournal::default();
    let admission = facts(1)?;
    let session_id = game_session_id(100)?;
    let mut original = AdmissionAuthority::new(journal.clone());
    original.commit_fresh(admission, 100, || Ok(session_id))?;
    original.observe_runtime_ownership_generation(12)?;
    journal.set_current_lease(8)?;
    drop(original);

    let mut recovered = AdmissionAuthority::new(journal.clone());
    let recovered_session = recovered.commit_fresh(admission, 100, || game_session_id(101))?;

    assert_eq!(recovered_session.runtime_scope_generation().get(), 12);
    assert_eq!(recovered_session.character_lease().generation(), 8);
    assert_eq!(recovered_session.game_session_id(), session_id);
    Ok(())
}

#[test]
fn committed_reconnect_rehydrates_after_process_restart() -> Result<(), AdmissionError> {
    let journal = RecoveryJournal::default();
    let admission = facts(2)?;
    let session_id = game_session_id(200)?;
    let mut original = AdmissionAuthority::new(journal.clone());
    original.commit_fresh(admission, 100, || Ok(session_id))?;
    let generation_one = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
    assert_eq!(
        original.mark_unexpected_control_loss(100, generation_one)?,
        ControlLossDisposition::Applied
    );
    let attempt = ReconnectAttemptRef::new(2)?;
    let generation_two = original.prepare_reconnect(attempt, generation_one, 200, 7, 11)?;

    journal.commit_stored_attempt(session_id, attempt)?;
    drop(original);

    let mut recovered = AdmissionAuthority::new(journal.clone());
    recovered.rehydrate_session(session_id)?;
    assert_eq!(
        recovered
            .current()
            .ok_or(AdmissionError::Terminal)?
            .connection_generation(),
        generation_two
    );
    assert_eq!(recovered.current_transport(), Some(200));
    assert_eq!(
        recovered.commit_reconnect(attempt, 201, 7, 11),
        Err(AdmissionError::StaleConnection)
    );
    assert_eq!(
        recovered.commit_reconnect(attempt, 200, 7, 11)?,
        generation_two
    );
    Ok(())
}
