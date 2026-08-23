use crate::foundation::{
    AdmissionAuthority, AdmissionError, ChannelId, CharacterId, ConnectionGeneration,
    ControlLossDisposition, FreshAdmissionAuthoritySnapshot, FreshAdmissionCommit,
    FreshAdmissionFacts, FreshAdmissionReplayKey, GameSessionId, GameSessionState,
    ReconnectAttemptClaim, ReconnectAttemptDisposition, ReconnectAttemptJournal,
    ReconnectAttemptRef, ReconnectCommitBinding, ScopeOwnershipGeneration, WorldId,
};
use std::cell::RefCell;
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

#[derive(Default)]
struct RecoveryState {
    session: Option<DurableSession>,
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

    fn mark_control_loss(
        &self,
        _game_session_id: GameSessionId,
        _observed_transport: u64,
        _observed_generation: ConnectionGeneration,
    ) -> Result<ControlLossDisposition, AdmissionError> {
        Err(AdmissionError::ReconciliationUnavailable)
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
        _game_session_id: GameSessionId,
        _attempt: ReconnectAttemptRef,
    ) -> Result<Option<ReconnectAttemptDisposition>, AdmissionError> {
        Ok(None)
    }

    fn claim_prepared(
        &self,
        _game_session_id: GameSessionId,
        _attempt: ReconnectAttemptRef,
        _binding: ReconnectCommitBinding<u64>,
    ) -> Result<ReconnectAttemptClaim, AdmissionError> {
        Err(AdmissionError::ReconciliationUnavailable)
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
        _game_session_id: GameSessionId,
        _attempt: ReconnectAttemptRef,
        _binding: ReconnectCommitBinding<u64>,
    ) -> Result<(), AdmissionError> {
        Err(AdmissionError::ReconciliationUnavailable)
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
