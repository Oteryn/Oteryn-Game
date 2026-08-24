use crate::foundation::{
    AdmissionAuthority, AdmissionError, ChannelId, CharacterId, CharacterLease,
    ConnectionGeneration, ControlLossDisposition, FoundationProtocolError,
    FreshAdmissionAuthoritySnapshot, FreshAdmissionCommit, FreshAdmissionFacts,
    FreshAdmissionReplayKey, GameSessionAuthoritySnapshot, GameSessionId, GameSessionState,
    ReconnectAttemptAuthoritySnapshot, ReconnectAttemptClaim, ReconnectAttemptDisposition,
    ReconnectAttemptJournal, ReconnectAttemptRef, ReconnectCommitBinding, ScopeOwnershipGeneration,
    WorldId, decode_wire_envelope,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

type AttemptKey = (GameSessionId, ReconnectAttemptRef);

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
struct ReviewState {
    session: Option<DurableSession>,
    dispositions: HashMap<AttemptKey, ReconnectAttemptDisposition>,
    bindings: HashMap<AttemptKey, ReconnectCommitBinding<u64>>,
}

#[derive(Clone, Default)]
struct ReviewJournal {
    state: Rc<RefCell<ReviewState>>,
}

impl ReviewJournal {
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

impl ReconnectAttemptJournal<u64> for ReviewJournal {
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
        if session.scope_generation != expected_current || observed < expected_current {
            return Err(AdmissionError::StaleRuntime);
        }
        session.scope_generation = observed;
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
            .any(|((session_id, _), disposition)| {
                *session_id == game_session_id
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
        if !matches!(disposition, ReconnectAttemptDisposition::Prepared { .. })
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
fn committed_reconnect_replay_revalidates_authority_after_rehydrate() -> Result<(), AdmissionError>
{
    let journal = ReviewJournal::default();
    let session_id = game_session_id(500)?;
    let mut original = AdmissionAuthority::new(journal.clone());
    original.commit_fresh(facts(50)?, 100, || Ok(session_id))?;
    let generation_one = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
    assert_eq!(
        original.mark_unexpected_control_loss(100, generation_one)?,
        ControlLossDisposition::Applied
    );
    let attempt = ReconnectAttemptRef::new(50)?;
    let generation_two = original.prepare_reconnect(attempt, generation_one, 200, 7, 11)?;
    journal.commit_stored_attempt(session_id, attempt)?;
    drop(original);

    let mut recovered = AdmissionAuthority::new(journal.clone());
    recovered.rehydrate_session(session_id)?;
    journal.terminate_session(session_id, generation_two)?;

    assert_eq!(
        recovered.commit_reconnect(attempt, 200, 7, 11),
        Err(AdmissionError::Terminal)
    );
    Ok(())
}

fn test_varint(mut value: usize) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if value == 0 {
            return out;
        }
    }
}

fn test_envelope(message_type: u8, payload: &[u8]) -> Vec<u8> {
    let mut envelope = vec![0x08, message_type, 0x22];
    envelope.extend(test_varint(payload.len()));
    envelope.extend_from_slice(payload);
    envelope
}

fn length_delimited_field(tag: u8, length: usize) -> Vec<u8> {
    let mut payload = vec![tag];
    payload.extend(test_varint(length));
    payload.resize(payload.len() + length, 0);
    payload
}

#[test]
fn server_to_client_nested_limits_fail_closed_before_returning_payload() {
    let command_result = length_delimited_field(0x2a, 65_537);
    assert_eq!(
        decode_wire_envelope(&test_envelope(8, &command_result)),
        Err(FoundationProtocolError::PayloadLimitExceeded)
    );

    let state_delta = length_delimited_field(0x2a, 262_145);
    assert_eq!(
        decode_wire_envelope(&test_envelope(9, &state_delta)),
        Err(FoundationProtocolError::PayloadLimitExceeded)
    );

    let snapshot_chunk = length_delimited_field(0x1a, 524_289);
    assert_eq!(
        decode_wire_envelope(&test_envelope(12, &snapshot_chunk)),
        Err(FoundationProtocolError::SnapshotLimitExceeded)
    );

    let mut snapshot_begin = vec![0x10];
    snapshot_begin.extend(test_varint(257));
    assert_eq!(
        decode_wire_envelope(&test_envelope(11, &snapshot_begin)),
        Err(FoundationProtocolError::SnapshotLimitExceeded)
    );
}
