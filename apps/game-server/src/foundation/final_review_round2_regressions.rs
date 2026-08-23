use crate::foundation::{
    AdmissionAuthority, AdmissionError, ChannelId, CharacterId, CharacterLease,
    ConnectionGeneration, ControlLossDisposition, FreshAdmissionAuthoritySnapshot,
    FreshAdmissionCommit, FreshAdmissionFacts, GameSessionAuthoritySnapshot, GameSessionId,
    GameSessionState, ReconnectAttemptClaim, ReconnectAttemptDisposition,
    ReconnectAttemptJournal, ReconnectAttemptRef, ReconnectCommitBinding, ScopeOwnershipGeneration,
    SnapshotBarrier, WorldId,
};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Copy)]
struct DurableReplayState {
    commit: FreshAdmissionCommit<u64>,
    session_state: GameSessionState,
    connection_generation: ConnectionGeneration,
    current_transport: Option<u64>,
    lease: CharacterLease,
    scope: ScopeOwnershipGeneration,
    attempt: ReconnectAttemptRef,
    disposition: ReconnectAttemptDisposition,
    terminate_before_next_lookup: bool,
}

#[derive(Clone)]
struct RacyReplayJournal {
    state: Rc<RefCell<DurableReplayState>>,
}

impl RacyReplayJournal {
    fn terminate_before_next_lookup(&self) {
        self.state.borrow_mut().terminate_before_next_lookup = true;
    }
}

impl ReconnectAttemptJournal<u64> for RacyReplayJournal {
    fn commit_fresh<F>(
        &self,
        _facts: FreshAdmissionFacts,
        _authenticated_transport: u64,
        _issue_game_session_id: F,
    ) -> Result<FreshAdmissionAuthoritySnapshot<u64>, AdmissionError>
    where
        F: FnOnce() -> Result<GameSessionId, AdmissionError>,
    {
        Err(AdmissionError::ReconciliationUnavailable)
    }

    fn load_session(
        &self,
        game_session_id: GameSessionId,
    ) -> Result<GameSessionAuthoritySnapshot<u64>, AdmissionError> {
        let state = self.state.borrow();
        if state.commit.game_session_id() != game_session_id {
            return Err(AdmissionError::ReconciliationUnavailable);
        }
        Ok(GameSessionAuthoritySnapshot::new(
            state.commit,
            state.session_state,
            state.connection_generation,
            state.current_transport,
            state.lease,
            state.scope,
        ))
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
        _game_session_id: GameSessionId,
        _expected_current: ScopeOwnershipGeneration,
        _observed: ScopeOwnershipGeneration,
    ) -> Result<ScopeOwnershipGeneration, AdmissionError> {
        Err(AdmissionError::ReconciliationUnavailable)
    }

    fn lookup(
        &self,
        game_session_id: GameSessionId,
        attempt: ReconnectAttemptRef,
    ) -> Result<Option<ReconnectAttemptDisposition>, AdmissionError> {
        let mut state = self.state.borrow_mut();
        if state.commit.game_session_id() != game_session_id || state.attempt != attempt {
            return Ok(None);
        }
        if state.terminate_before_next_lookup {
            state.terminate_before_next_lookup = false;
            state.session_state = GameSessionState::Terminal;
            state.current_transport = None;
        }
        Ok(Some(state.disposition))
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

fn game_session_id(value: u64) -> Result<GameSessionId, AdmissionError> {
    GameSessionId::decode(&raw_uuid_v7(value)).map_err(|_| AdmissionError::InvalidFacts)
}

#[test]
fn committed_replay_cannot_race_authority_change_between_snapshot_and_attempt_lookup()
-> Result<(), AdmissionError> {
    let session_id = game_session_id(700)?;
    let commit = FreshAdmissionCommit::from_facts(session_id, facts(70)?, 100)?;
    let generation_two = ConnectionGeneration::new(2).map_err(|_| AdmissionError::InvalidFacts)?;
    let attempt = ReconnectAttemptRef::new(70)?;
    let lease = CharacterLease::new(commit.character_id(), 7)?;
    let scope = ScopeOwnershipGeneration::new(11).map_err(|_| AdmissionError::InvalidFacts)?;
    let journal = RacyReplayJournal {
        state: Rc::new(RefCell::new(DurableReplayState {
            commit,
            session_state: GameSessionState::Active,
            connection_generation: generation_two,
            current_transport: Some(200),
            lease,
            scope,
            attempt,
            disposition: ReconnectAttemptDisposition::Committed {
                generation: generation_two,
            },
            terminate_before_next_lookup: false,
        })),
    };

    let mut recovered = AdmissionAuthority::new(journal.clone());
    recovered.rehydrate_session(session_id)?;
    journal.terminate_before_next_lookup();

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

fn state_domain_snapshot(domain_id: usize) -> Vec<u8> {
    let mut nested = vec![0x08];
    nested.extend(test_varint(domain_id));
    nested.extend([0x10, 0x01, 0x18, 0x01, 0x22, 0x00]);
    nested
}

fn snapshot_body(domain_count: usize) -> Vec<u8> {
    let mut body = Vec::new();
    for domain_id in 1..=domain_count {
        let nested = state_domain_snapshot(domain_id);
        body.push(0x0a);
        body.extend(test_varint(nested.len()));
        body.extend(nested);
    }
    body
}

#[test]
fn assembled_snapshot_body_rejects_more_than_256_state_domains() {
    let body = snapshot_body(257);
    let mut barrier = SnapshotBarrier::new();
    barrier
        .begin(1, 1, body.len() as u64, 10, 1)
        .expect("bounded snapshot begin must succeed");
    barrier
        .chunk(1, 0, &body, 1)
        .expect("bounded snapshot chunk must succeed");

    assert_eq!(
        barrier.commit(1, 1),
        Err(crate::foundation::FoundationProtocolError::SnapshotLimitExceeded)
    );
}
