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

#[derive(Debug, Clone, Copy)]
enum ClaimAuthorityMutation {
    RecoveredController {
        transport: u64,
    },
    CharacterLease {
        generation: u64,
    },
    RuntimeScope {
        generation: ScopeOwnershipGeneration,
    },
    PeerCommittedSameBinding,
}

#[derive(Default)]
struct RecoveryState {
    session: Option<DurableSession>,
    dispositions: HashMap<AttemptKey, ReconnectAttemptDisposition>,
    bindings: HashMap<AttemptKey, ReconnectCommitBinding<u64>>,
    claim_authority_mutation: Option<ClaimAuthorityMutation>,
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

    fn set_current_scope(&self, generation: u64) -> Result<(), AdmissionError> {
        let generation =
            ScopeOwnershipGeneration::new(generation).map_err(|_| AdmissionError::InvalidFacts)?;
        self.state
            .borrow_mut()
            .session
            .as_mut()
            .ok_or(AdmissionError::ReconciliationUnavailable)?
            .scope_generation = generation;
        Ok(())
    }

    fn mutate_authority_before_next_claim(&self, mutation: ClaimAuthorityMutation) {
        self.state.borrow_mut().claim_authority_mutation = Some(mutation);
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
        let key = (game_session_id, attempt);
        let lease = CharacterLease::new(session.commit.character_id(), session.lease_generation)?;
        let snapshot = GameSessionAuthoritySnapshot::new(
            session.commit,
            session.state,
            session.connection_generation,
            session.current_transport,
            lease,
            session.scope_generation,
        );
        Ok(ReconnectAttemptAuthoritySnapshot::new(
            snapshot,
            state.dispositions.get(&key).copied(),
            state.bindings.get(&key).copied(),
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

        if let Some(mutation) = state.claim_authority_mutation.take() {
            let session = state
                .session
                .as_mut()
                .ok_or(AdmissionError::ReconciliationUnavailable)?;
            if session.commit.game_session_id() != game_session_id {
                return Err(AdmissionError::ReconciliationUnavailable);
            }
            match mutation {
                ClaimAuthorityMutation::RecoveredController { transport } => {
                    session.state = GameSessionState::Active;
                    session.current_transport = Some(transport);
                }
                ClaimAuthorityMutation::CharacterLease { generation } => {
                    session.lease_generation = generation;
                }
                ClaimAuthorityMutation::RuntimeScope { generation } => {
                    session.scope_generation = generation;
                }
                ClaimAuthorityMutation::PeerCommittedSameBinding => {
                    session.state = GameSessionState::Active;
                    session.connection_generation = binding.candidate_generation();
                    session.current_transport = Some(binding.candidate_transport());
                }
            }
            if matches!(mutation, ClaimAuthorityMutation::PeerCommittedSameBinding) {
                let disposition = ReconnectAttemptDisposition::Committed {
                    generation: binding.candidate_generation(),
                };
                state.dispositions.insert(key, disposition);
                state.bindings.insert(key, binding);
                return Ok(ReconnectAttemptClaim::Existing(disposition));
            }
        }

        let session = state
            .session
            .ok_or(AdmissionError::ReconciliationUnavailable)?;
        if session.commit.game_session_id() != game_session_id {
            return Err(AdmissionError::ReconciliationUnavailable);
        }

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
        } else if binding.character_lease().character_id() != session.commit.character_id()
            || binding.character_lease().generation() != session.lease_generation
        {
            Some(AdmissionError::StaleLease)
        } else if binding.scope_generation() != session.scope_generation {
            Some(AdmissionError::StaleRuntime)
        } else {
            None
        };
        if let Some(error) = claim_error {
            state
                .dispositions
                .insert(key, ReconnectAttemptDisposition::TerminallySuperseded);
            state.bindings.remove(&key);
            return Err(error);
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
fn fresh_reconciliation_restores_current_runtime_authority() -> Result<(), AdmissionError> {
    let journal = RecoveryJournal::default();
    let admission = facts(1)?;
    let session_id = game_session_id(100)?;
    let mut original = AdmissionAuthority::new(journal.clone());
    original.commit_fresh(admission, 100, || Ok(session_id))?;
    original.observe_runtime_ownership_generation(12)?;
    drop(original);
    journal.advance_runtime_scope(
        session_id,
        ScopeOwnershipGeneration::new(12).map_err(|_| AdmissionError::InvalidFacts)?,
        ScopeOwnershipGeneration::new(13).map_err(|_| AdmissionError::InvalidFacts)?,
    )?;

    let mut recovered = AdmissionAuthority::new(journal.clone());
    let recovered_session = recovered.commit_fresh(admission, 100, || game_session_id(101))?;

    assert_eq!(recovered_session.runtime_scope_generation().get(), 13);
    assert_eq!(recovered_session.character_lease().generation(), 7);
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
    journal.advance_runtime_scope(
        session_id,
        ScopeOwnershipGeneration::new(11).map_err(|_| AdmissionError::InvalidFacts)?,
        ScopeOwnershipGeneration::new(12).map_err(|_| AdmissionError::InvalidFacts)?,
    )?;

    let mut recovered = AdmissionAuthority::new(journal.clone());
    let recovered_session = recovered.rehydrate_session(session_id)?;
    assert_eq!(recovered_session.runtime_scope_generation().get(), 12);
    assert_eq!(recovered_session.connection_generation(), generation_two);
    assert_eq!(recovered.current_transport(), Some(200));
    assert_eq!(
        recovered.commit_reconnect(attempt, 201, 7, 12),
        Err(AdmissionError::StaleConnection)
    );
    assert_eq!(
        recovered.commit_reconnect(attempt, 200, 7, 12)?,
        generation_two
    );
    Ok(())
}

#[test]
fn rehydrate_missing_session_fails_closed() -> Result<(), AdmissionError> {
    let journal = RecoveryJournal::default();
    let mut recovered = AdmissionAuthority::new(journal);
    assert!(matches!(
        recovered.rehydrate_session(game_session_id(300)?),
        Err(AdmissionError::ReconciliationUnavailable)
    ));
    Ok(())
}

#[test]
fn rehydrate_terminal_session_cannot_revive_control() -> Result<(), AdmissionError> {
    let journal = RecoveryJournal::default();
    let admission = facts(3)?;
    let session_id = game_session_id(301)?;
    let mut original = AdmissionAuthority::new(journal.clone());
    original.commit_fresh(admission, 100, || Ok(session_id))?;
    journal.terminate_session(
        session_id,
        ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?,
    )?;
    drop(original);

    let mut recovered = AdmissionAuthority::new(journal);
    assert!(matches!(
        recovered.rehydrate_session(session_id),
        Err(AdmissionError::Terminal)
    ));
    Ok(())
}

#[test]
fn rehydrate_rejects_rolled_back_character_lease() -> Result<(), AdmissionError> {
    let journal = RecoveryJournal::default();
    let admission = facts(4)?;
    let session_id = game_session_id(302)?;
    let mut original = AdmissionAuthority::new(journal.clone());
    original.commit_fresh(admission, 100, || Ok(session_id))?;
    journal.set_current_lease(6)?;
    drop(original);

    let mut recovered = AdmissionAuthority::new(journal);
    assert!(matches!(
        recovered.rehydrate_session(session_id),
        Err(AdmissionError::StaleLease)
    ));
    Ok(())
}

#[test]
fn rehydrate_rejects_advanced_character_lease_for_same_session() -> Result<(), AdmissionError> {
    let journal = RecoveryJournal::default();
    let admission = facts(5)?;
    let session_id = game_session_id(303)?;
    let mut original = AdmissionAuthority::new(journal.clone());
    original.commit_fresh(admission, 100, || Ok(session_id))?;
    journal.set_current_lease(8)?;
    drop(original);

    let mut recovered = AdmissionAuthority::new(journal);
    assert!(matches!(
        recovered.rehydrate_session(session_id),
        Err(AdmissionError::StaleLease)
    ));
    Ok(())
}

#[test]
fn rehydrate_rejects_rolled_back_runtime_generation() -> Result<(), AdmissionError> {
    let journal = RecoveryJournal::default();
    let admission = facts(6)?;
    let session_id = game_session_id(304)?;
    let mut original = AdmissionAuthority::new(journal.clone());
    original.commit_fresh(admission, 100, || Ok(session_id))?;
    journal.set_current_scope(10)?;
    drop(original);

    let mut recovered = AdmissionAuthority::new(journal);
    assert!(matches!(
        recovered.rehydrate_session(session_id),
        Err(AdmissionError::StaleRuntime)
    ));
    Ok(())
}

#[test]
fn prepared_reconnect_commits_after_process_replacement() -> Result<(), AdmissionError> {
    let journal = RecoveryJournal::default();
    let session_id = game_session_id(400)?;
    let mut original = AdmissionAuthority::new(journal.clone());
    original.commit_fresh(facts(7)?, 100, || Ok(session_id))?;
    let generation_one = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
    assert_eq!(
        original.mark_unexpected_control_loss(100, generation_one)?,
        ControlLossDisposition::Applied
    );
    let attempt = ReconnectAttemptRef::new(7)?;
    let generation_two = original.prepare_reconnect(attempt, generation_one, 200, 7, 11)?;
    drop(original);

    let mut recovered = AdmissionAuthority::new(journal);
    recovered.rehydrate_session(session_id)?;
    assert_eq!(
        recovered.commit_reconnect(attempt, 201, 7, 11),
        Err(AdmissionError::AttemptMismatch)
    );
    assert_eq!(
        recovered.commit_reconnect(attempt, 200, 7, 11)?,
        generation_two
    );
    assert_eq!(recovered.current_transport(), Some(200));
    assert_eq!(
        recovered
            .current()
            .ok_or(AdmissionError::Terminal)?
            .connection_generation(),
        generation_two
    );
    Ok(())
}

#[test]
fn prepare_claim_revalidates_recovered_controller_at_linearization_point()
-> Result<(), AdmissionError> {
    let journal = RecoveryJournal::default();
    let session_id = game_session_id(401)?;
    let mut original = AdmissionAuthority::new(journal.clone());
    original.commit_fresh(facts(8)?, 100, || Ok(session_id))?;
    let generation_one = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
    original.mark_unexpected_control_loss(100, generation_one)?;
    journal.mutate_authority_before_next_claim(ClaimAuthorityMutation::RecoveredController {
        transport: 300,
    });
    let attempt = ReconnectAttemptRef::new(8)?;

    assert_eq!(
        original.prepare_reconnect(attempt, generation_one, 200, 7, 11),
        Err(AdmissionError::IncumbentHealthy)
    );
    assert!(original.current().is_none());
    assert_eq!(
        journal
            .reconcile_reconnect_attempt(session_id, attempt)?
            .disposition(),
        Some(ReconnectAttemptDisposition::TerminallySuperseded)
    );

    let mut recovered = AdmissionAuthority::new(journal);
    let state = recovered.rehydrate_session(session_id)?.state();
    assert_eq!(state, GameSessionState::Active);
    assert_eq!(recovered.current_transport(), Some(300));
    Ok(())
}

#[test]
fn prepare_claim_revalidates_character_lease_at_linearization_point() -> Result<(), AdmissionError>
{
    let journal = RecoveryJournal::default();
    let session_id = game_session_id(402)?;
    let mut original = AdmissionAuthority::new(journal.clone());
    original.commit_fresh(facts(9)?, 100, || Ok(session_id))?;
    let generation_one = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
    original.mark_unexpected_control_loss(100, generation_one)?;
    journal.mutate_authority_before_next_claim(ClaimAuthorityMutation::CharacterLease {
        generation: 8,
    });
    let attempt = ReconnectAttemptRef::new(9)?;

    assert_eq!(
        original.prepare_reconnect(attempt, generation_one, 200, 7, 11),
        Err(AdmissionError::StaleLease)
    );
    assert!(original.current().is_none());
    assert_eq!(
        journal
            .reconcile_reconnect_attempt(session_id, attempt)?
            .disposition(),
        Some(ReconnectAttemptDisposition::TerminallySuperseded)
    );

    let mut recovered = AdmissionAuthority::new(journal);
    assert!(matches!(
        recovered.rehydrate_session(session_id),
        Err(AdmissionError::StaleLease)
    ));
    Ok(())
}

#[test]
fn prepare_claim_revalidates_runtime_scope_at_linearization_point() -> Result<(), AdmissionError> {
    let journal = RecoveryJournal::default();
    let session_id = game_session_id(403)?;
    let mut original = AdmissionAuthority::new(journal.clone());
    original.commit_fresh(facts(10)?, 100, || Ok(session_id))?;
    let generation_one = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
    original.mark_unexpected_control_loss(100, generation_one)?;
    let scope_twelve =
        ScopeOwnershipGeneration::new(12).map_err(|_| AdmissionError::InvalidFacts)?;
    journal.mutate_authority_before_next_claim(ClaimAuthorityMutation::RuntimeScope {
        generation: scope_twelve,
    });
    let superseded_attempt = ReconnectAttemptRef::new(10)?;

    assert_eq!(
        original.prepare_reconnect(superseded_attempt, generation_one, 200, 7, 11),
        Err(AdmissionError::StaleRuntime)
    );
    assert!(original.current().is_none());
    assert_eq!(
        journal
            .reconcile_reconnect_attempt(session_id, superseded_attempt)?
            .disposition(),
        Some(ReconnectAttemptDisposition::TerminallySuperseded)
    );

    let mut recovered = AdmissionAuthority::new(journal);
    assert_eq!(
        recovered
            .rehydrate_session(session_id)?
            .runtime_scope_generation(),
        scope_twelve
    );
    let current_attempt = ReconnectAttemptRef::new(11)?;
    assert_eq!(
        recovered.prepare_reconnect(current_attempt, generation_one, 201, 7, 12)?,
        ConnectionGeneration::new(2).map_err(|_| AdmissionError::InvalidFacts)?
    );
    Ok(())
}

#[test]
fn prepare_reconciles_peer_commit_between_initial_read_and_claim() -> Result<(), AdmissionError> {
    let journal = RecoveryJournal::default();
    let session_id = game_session_id(404)?;
    let mut original = AdmissionAuthority::new(journal.clone());
    original.commit_fresh(facts(11)?, 100, || Ok(session_id))?;
    let generation_one = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
    original.mark_unexpected_control_loss(100, generation_one)?;
    journal.mutate_authority_before_next_claim(ClaimAuthorityMutation::PeerCommittedSameBinding);
    let attempt = ReconnectAttemptRef::new(12)?;
    let generation_two = ConnectionGeneration::new(2).map_err(|_| AdmissionError::InvalidFacts)?;

    assert_eq!(
        original.prepare_reconnect(attempt, generation_one, 200, 7, 11)?,
        generation_two
    );
    let current = original.current().ok_or(AdmissionError::Terminal)?;
    assert_eq!(current.state(), GameSessionState::Active);
    assert_eq!(current.connection_generation(), generation_two);
    assert_eq!(original.current_transport(), Some(200));
    assert_eq!(
        journal
            .reconcile_reconnect_attempt(session_id, attempt)?
            .disposition(),
        Some(ReconnectAttemptDisposition::Committed {
            generation: generation_two,
        })
    );
    Ok(())
}

#[test]
fn durable_transport_ref_v1_codec_is_exact_and_zero_rejected() -> Result<(), AdmissionError> {
    let encoded = [0xA5u8; 16];
    let reference = crate::foundation::AuthenticatedTransportRefV1::decode(&encoded)?;
    assert_eq!(reference.to_bytes(), encoded);
    assert_eq!(
        crate::foundation::AuthenticatedTransportRefV1::decode(&[0u8; 16]),
        Err(AdmissionError::InvalidFacts)
    );
    assert_eq!(
        crate::foundation::AuthenticatedTransportRefV1::decode(&[0xA5u8; 15]),
        Err(AdmissionError::InvalidFacts)
    );
    Ok(())
}
