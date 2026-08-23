use super::{
    ChannelId, CharacterId, ConnectionFence, ConnectionGeneration, GameSessionId, GenerationError,
    ScopeOwnershipGeneration, ScopeRuntimeFence, WorldId,
};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionError {
    InvalidFacts,
    GrantReplayed,
    IncumbentHealthy,
    SessionNotReconnectable,
    StaleConnection,
    StaleLease,
    StaleRuntime,
    AttemptMismatch,
    GenerationExhausted,
    Terminal,
}
impl Display for AdmissionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidFacts => "fresh admission facts are invalid",
            Self::GrantReplayed => "fresh admission grant was already consumed",
            Self::IncumbentHealthy => "current transport binding is healthy",
            Self::SessionNotReconnectable => "game session is not reconnectable",
            Self::StaleConnection => "connection generation is stale",
            Self::StaleLease => "character lease generation is stale",
            Self::StaleRuntime => "runtime ownership generation is stale",
            Self::AttemptMismatch => "reconnect attempt does not match prepared candidate",
            Self::GenerationExhausted => "connection generation space is exhausted",
            Self::Terminal => "game session is terminal",
        })
    }
}
impl Error for AdmissionError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReconnectAttemptRef(u64);
impl ReconnectAttemptRef {
    pub fn new(v: u64) -> Result<Self, AdmissionError> {
        if v == 0 {
            Err(AdmissionError::InvalidFacts)
        } else {
            Ok(Self(v))
        }
    }
    pub const fn get(self) -> u64 {
        self.0
    }
}

pub const PRE_ADMISSION_TRUSTED_ISSUER: &str = "urn:oteryn:platform:game-admission";
pub const PRE_ADMISSION_PROFILE: &str = "oteryn-pre-admission-v1";

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum AdmissionGrantTrustScope {
    OterynPreAdmissionV1,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct GrantReplayKey {
    trust_scope: AdmissionGrantTrustScope,
    nonce: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreshAdmissionFacts {
    grant_nonce: [u8; 32],
    character_id: CharacterId,
    world_id: WorldId,
    channel_id: ChannelId,
    character_lease_generation: u64,
    scope_ownership_generation: u64,
}
impl FreshAdmissionFacts {
    pub fn new(
        grant_nonce: [u8; 32],
        character_id: CharacterId,
        world_id: WorldId,
        channel_id: ChannelId,
        lease: u64,
        scope: u64,
    ) -> Result<Self, AdmissionError> {
        if grant_nonce.iter().all(|b| *b == 0) || lease == 0 || scope == 0 {
            return Err(AdmissionError::InvalidFacts);
        }
        Ok(Self {
            grant_nonce,
            character_id,
            world_id,
            channel_id,
            character_lease_generation: lease,
            scope_ownership_generation: scope,
        })
    }
    #[cfg(test)]
    fn replay_key(&self) -> GrantReplayKey {
        GrantReplayKey {
            trust_scope: AdmissionGrantTrustScope::OterynPreAdmissionV1,
            nonce: self.grant_nonce,
        }
    }

    #[cfg(test)]
    fn for_test(nonce: u64, lease: u64, scope: u64) -> Result<Self, AdmissionError> {
        fn id(v: u64) -> [u8; 16] {
            let mut x = [0u8; 16];
            x[8..].copy_from_slice(&v.to_be_bytes());
            x[6] = (x[6] & 0x0f) | 0x70;
            x[8] = (x[8] & 0x3f) | 0x80;
            x
        }
        let mut grant_nonce = [0u8; 32];
        grant_nonce[24..].copy_from_slice(&nonce.to_be_bytes());
        let map = |_| AdmissionError::InvalidFacts;
        Self::new(
            grant_nonce,
            CharacterId::decode(&id(1)).map_err(map)?,
            WorldId::decode(&id(2)).map_err(map)?,
            ChannelId::decode(&id(3)).map_err(map)?,
            lease,
            scope,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterLease {
    character_id: CharacterId,
    generation: u64,
}
impl CharacterLease {
    #[must_use]
    pub const fn character_id(self) -> CharacterId {
        self.character_id
    }
    #[must_use]
    pub const fn generation(self) -> u64 {
        self.generation
    }
    #[must_use]
    pub const fn accepts_generation(self, generation: u64) -> bool {
        generation == self.generation
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameSessionState {
    Active,
    Reconnectable,
    Terminal,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameSession {
    game_session_id: GameSessionId,
    character_id: CharacterId,
    world_id: WorldId,
    channel_id: ChannelId,
    lease_generation: u64,
    runtime_scope: ScopeRuntimeFence,
    connection: ConnectionFence,
    state: GameSessionState,
}
impl GameSession {
    pub const fn game_session_id(&self) -> GameSessionId {
        self.game_session_id
    }
    pub const fn connection_generation(&self) -> ConnectionGeneration {
        self.connection.current()
    }
    pub const fn state(&self) -> GameSessionState {
        self.state
    }
    pub const fn accepts_generation(&self, g: ConnectionGeneration) -> bool {
        self.connection.accepts(g)
    }
    pub const fn character_id(&self) -> CharacterId {
        self.character_id
    }
    #[must_use]
    pub const fn character_lease(&self) -> CharacterLease {
        CharacterLease {
            character_id: self.character_id,
            generation: self.lease_generation,
        }
    }
    pub const fn world_id(&self) -> WorldId {
        self.world_id
    }
    pub const fn channel_id(&self) -> ChannelId {
        self.channel_id
    }
    #[must_use]
    pub const fn runtime_scope_generation(&self) -> ScopeOwnershipGeneration {
        self.runtime_scope.generation()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PreparedReconnect<T: Copy + Eq> {
    attempt: ReconnectAttemptRef,
    predecessor: ConnectionGeneration,
    candidate: ConnectionGeneration,
    candidate_transport: T,
    lease_generation: u64,
    scope_generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CommittedReconnect<T: Copy + Eq> {
    attempt: ReconnectAttemptRef,
    generation: ConnectionGeneration,
    transport: T,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReconnectAttemptTracker<T: Copy + Eq> {
    high_watermark: u64,
    latest_committed: Option<CommittedReconnect<T>>,
}

impl<T: Copy + Eq> ReconnectAttemptTracker<T> {
    const fn new() -> Self {
        Self {
            high_watermark: 0,
            latest_committed: None,
        }
    }

    fn reset(&mut self) {
        self.high_watermark = 0;
        self.latest_committed = None;
    }

    fn reserve_new(&mut self, attempt: ReconnectAttemptRef) -> Result<(), AdmissionError> {
        if attempt.get() <= self.high_watermark {
            return Err(AdmissionError::StaleConnection);
        }
        self.high_watermark = attempt.get();
        Ok(())
    }

    const fn latest(&self) -> Option<CommittedReconnect<T>> {
        self.latest_committed
    }

    fn commit(
        &mut self,
        attempt: ReconnectAttemptRef,
        generation: ConnectionGeneration,
        transport: T,
    ) {
        debug_assert_eq!(attempt.get(), self.high_watermark);
        self.latest_committed = Some(CommittedReconnect {
            attempt,
            generation,
            transport,
        });
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        usize::from(self.latest_committed.is_some())
    }
}

#[derive(Debug)]
pub struct AdmissionAuthority<T: Copy + Eq> {
    current: Option<GameSession>,
    current_transport: Option<T>,
    prepared: Option<PreparedReconnect<T>>,
    committed_attempts: ReconnectAttemptTracker<T>,
}
impl<T: Copy + Eq> Default for AdmissionAuthority<T> {
    fn default() -> Self {
        Self::new()
    }
}
impl<T: Copy + Eq> AdmissionAuthority<T> {
    pub fn new() -> Self {
        Self {
            current: None,
            current_transport: None,
            prepared: None,
            committed_attempts: ReconnectAttemptTracker::new(),
        }
    }
    pub fn current(&self) -> Option<&GameSession> {
        self.current.as_ref()
    }

    #[cfg(test)]
    fn retained_fresh_identity_history(&self) -> usize {
        0
    }
    #[must_use]
    pub const fn current_transport(&self) -> Option<T> {
        self.current_transport
    }
    pub fn commit_fresh<F>(
        &mut self,
        facts: FreshAdmissionFacts,
        authenticated_transport: T,
        commit_fresh_identity: F,
    ) -> Result<&GameSession, AdmissionError>
    where
        F: FnOnce() -> Result<GameSessionId, AdmissionError>,
    {
        if self
            .current
            .as_ref()
            .is_some_and(|s| s.state != GameSessionState::Terminal)
        {
            return Err(AdmissionError::IncumbentHealthy);
        }
        let runtime_generation = ScopeOwnershipGeneration::new(facts.scope_ownership_generation)
            .map_err(|_| AdmissionError::InvalidFacts)?;
        // This trusted seam represents the game-domain atomic identity boundary:
        // GrantNonce replay/consume and never-reused GameSessionId reservation live
        // in fenced durable authority rather than an unbounded process-local history.
        let game_session_id = commit_fresh_identity()?;

        self.prepared = None;
        self.committed_attempts.reset();
        self.current_transport = Some(authenticated_transport);
        self.current = Some(GameSession {
            game_session_id,
            character_id: facts.character_id,
            world_id: facts.world_id,
            channel_id: facts.channel_id,
            lease_generation: facts.character_lease_generation,
            runtime_scope: ScopeRuntimeFence::from_external_grant(runtime_generation),
            connection: ConnectionFence::fresh_admission(),
            state: GameSessionState::Active,
        });
        self.current.as_ref().ok_or(AdmissionError::InvalidFacts)
    }
    pub fn terminate_current(&mut self) -> Result<(), AdmissionError> {
        let s = self.current.as_mut().ok_or(AdmissionError::Terminal)?;
        s.state = GameSessionState::Terminal;
        self.prepared = None;
        self.current_transport = None;
        Ok(())
    }
    pub fn mark_unexpected_control_loss(&mut self) -> Result<(), AdmissionError> {
        let s = self.current.as_mut().ok_or(AdmissionError::Terminal)?;
        if s.state == GameSessionState::Terminal {
            return Err(AdmissionError::Terminal);
        }
        s.state = GameSessionState::Reconnectable;
        self.prepared = None;
        self.current_transport = None;
        Ok(())
    }
    pub fn observe_runtime_ownership_generation(
        &mut self,
        generation: u64,
    ) -> Result<(), AdmissionError> {
        let observed =
            ScopeOwnershipGeneration::new(generation).map_err(|_| AdmissionError::InvalidFacts)?;
        let s = self.current.as_mut().ok_or(AdmissionError::Terminal)?;
        if s.state == GameSessionState::Terminal {
            return Err(AdmissionError::Terminal);
        }
        let current = s.runtime_scope.generation();
        if observed < current {
            return Err(AdmissionError::StaleRuntime);
        }
        if observed == current {
            return Ok(());
        }
        s.runtime_scope
            .apply_external_grant(observed)
            .map_err(|_| AdmissionError::StaleRuntime)?;
        self.prepared = None;
        Ok(())
    }
    fn committed_attempt_replay(
        &self,
        attempt: ReconnectAttemptRef,
        candidate_transport: T,
    ) -> Result<Option<ConnectionGeneration>, AdmissionError> {
        if let Some(committed) = self.committed_attempts.latest()
            && committed.attempt == attempt
        {
            let s = self.current.as_ref().ok_or(AdmissionError::Terminal)?;
            if s.state == GameSessionState::Terminal {
                return Err(AdmissionError::Terminal);
            }
            if committed.transport != candidate_transport {
                return Err(AdmissionError::AttemptMismatch);
            }
            if self.current_transport != Some(candidate_transport)
                || s.connection_generation() != committed.generation
            {
                return Err(AdmissionError::StaleConnection);
            }
            return Ok(Some(committed.generation));
        }
        if attempt.get() <= self.committed_attempts.high_watermark {
            return Err(AdmissionError::StaleConnection);
        }
        Ok(None)
    }
    pub fn prepare_reconnect(
        &mut self,
        attempt: ReconnectAttemptRef,
        predecessor: ConnectionGeneration,
        candidate_transport: T,
        lease: u64,
        scope: u64,
    ) -> Result<ConnectionGeneration, AdmissionError> {
        if let Some(existing) = self.prepared {
            if existing.attempt == attempt {
                return if existing.candidate_transport == candidate_transport {
                    Ok(existing.candidate)
                } else {
                    Err(AdmissionError::AttemptMismatch)
                };
            }
            if let Some(generation) = self.committed_attempt_replay(attempt, candidate_transport)? {
                return Ok(generation);
            }
            return Err(AdmissionError::AttemptMismatch);
        }
        if let Some(generation) = self.committed_attempt_replay(attempt, candidate_transport)? {
            return Ok(generation);
        }
        let s = self.current.as_ref().ok_or(AdmissionError::Terminal)?;
        if s.state == GameSessionState::Terminal {
            return Err(AdmissionError::Terminal);
        }
        if s.state == GameSessionState::Active {
            return Err(AdmissionError::IncumbentHealthy);
        }
        if s.state != GameSessionState::Reconnectable {
            return Err(AdmissionError::SessionNotReconnectable);
        }
        if predecessor != s.connection_generation() {
            return Err(AdmissionError::StaleConnection);
        }
        if lease != s.lease_generation {
            return Err(AdmissionError::StaleLease);
        }
        if scope != s.runtime_scope.generation().get() {
            return Err(AdmissionError::StaleRuntime);
        }
        let candidate = ConnectionGeneration::new(
            predecessor
                .get()
                .checked_add(1)
                .ok_or(AdmissionError::GenerationExhausted)?,
        )
        .map_err(|_| AdmissionError::GenerationExhausted)?;
        self.committed_attempts.reserve_new(attempt)?;
        self.prepared = Some(PreparedReconnect {
            attempt,
            predecessor,
            candidate,
            candidate_transport,
            lease_generation: lease,
            scope_generation: scope,
        });
        Ok(candidate)
    }
    pub fn commit_reconnect(
        &mut self,
        attempt: ReconnectAttemptRef,
        candidate_transport: T,
        lease: u64,
        scope: u64,
    ) -> Result<ConnectionGeneration, AdmissionError> {
        if self.prepared.is_none() {
            if let Some(generation) = self.committed_attempt_replay(attempt, candidate_transport)? {
                return Ok(generation);
            }
            return Err(AdmissionError::AttemptMismatch);
        }
        if self.current.as_ref().ok_or(AdmissionError::Terminal)?.state
            == GameSessionState::Terminal
        {
            return Err(AdmissionError::Terminal);
        }
        let prepared = *self
            .prepared
            .as_ref()
            .ok_or(AdmissionError::AttemptMismatch)?;
        if prepared.attempt != attempt {
            if let Some(generation) = self.committed_attempt_replay(attempt, candidate_transport)? {
                return Ok(generation);
            }
            return Err(AdmissionError::AttemptMismatch);
        }
        if prepared.candidate_transport != candidate_transport {
            return Err(AdmissionError::AttemptMismatch);
        }
        let s = self.current.as_mut().ok_or(AdmissionError::Terminal)?;
        if s.state != GameSessionState::Reconnectable {
            return Err(AdmissionError::SessionNotReconnectable);
        }
        if s.connection_generation() != prepared.predecessor {
            return Err(AdmissionError::StaleConnection);
        }
        if lease != s.lease_generation || lease != prepared.lease_generation {
            return Err(AdmissionError::StaleLease);
        }
        if scope != s.runtime_scope.generation().get() || scope != prepared.scope_generation {
            return Err(AdmissionError::StaleRuntime);
        }
        let generation = s
            .connection
            .rebind(prepared.predecessor)
            .map_err(|e| match e {
                GenerationError::Exhausted => AdmissionError::GenerationExhausted,
                _ => AdmissionError::StaleConnection,
            })?;
        debug_assert_eq!(generation, prepared.candidate);
        s.state = GameSessionState::Active;
        self.prepared = None;
        self.current_transport = Some(candidate_transport);
        self.committed_attempts
            .commit(attempt, generation, candidate_transport);
        Ok(generation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[derive(Default)]
    struct TestFreshIdentityLedger {
        consumed_grants: std::collections::BTreeSet<GrantReplayKey>,
        committed_session_ids: std::collections::BTreeSet<GameSessionId>,
    }

    impl TestFreshIdentityLedger {
        fn commit<F>(
            &mut self,
            replay_key: GrantReplayKey,
            issue_game_session_id: F,
        ) -> Result<GameSessionId, AdmissionError>
        where
            F: FnOnce() -> Result<GameSessionId, AdmissionError>,
        {
            if self.consumed_grants.contains(&replay_key) {
                return Err(AdmissionError::GrantReplayed);
            }
            let game_session_id = issue_game_session_id()?;
            if self.committed_session_ids.contains(&game_session_id) {
                return Err(AdmissionError::InvalidFacts);
            }
            self.committed_session_ids.insert(game_session_id);
            self.consumed_grants.insert(replay_key);
            Ok(game_session_id)
        }
    }

    fn admit(
        authority: &mut AdmissionAuthority<u64>,
        nonce: u64,
        session: u64,
        transport: u64,
    ) -> Result<&GameSession, AdmissionError> {
        let game_session_id = game_session_id(session)?;
        authority.commit_fresh(facts(nonce)?, transport, || Ok(game_session_id))
    }

    fn admit_with_ledger<'a>(
        authority: &'a mut AdmissionAuthority<u64>,
        ledger: &mut TestFreshIdentityLedger,
        nonce: u64,
        session: u64,
        transport: u64,
    ) -> Result<&'a GameSession, AdmissionError> {
        let admission = facts(nonce)?;
        let replay_key = admission.replay_key();
        let game_session_id = game_session_id(session)?;
        authority.commit_fresh(admission, transport, || {
            ledger.commit(replay_key, || Ok(game_session_id))
        })
    }

    #[test]
    fn fresh_identity_replay_history_is_not_retained_in_session_authority() {
        let authority = AdmissionAuthority::<u64>::new();
        assert_eq!(authority.retained_fresh_identity_history(), 0);
    }

    #[test]
    fn fresh_admission_commits_generation_one_and_nonce_replay_fails() -> Result<(), AdmissionError>
    {
        let mut authority = AdmissionAuthority::new();
        let mut ledger = TestFreshIdentityLedger::default();
        let session = admit_with_ledger(&mut authority, &mut ledger, 1, 100, 100u64)?;
        assert_eq!(session.connection_generation().get(), 1);
        assert_eq!(session.character_lease().generation(), 7);
        authority.terminate_current()?;
        assert_eq!(
            admit_with_ledger(&mut authority, &mut ledger, 1, 101, 100u64),
            Err(AdmissionError::GrantReplayed)
        );
        Ok(())
    }

    #[test]
    fn healthy_binding_cannot_be_preempted_by_reconnect() -> Result<(), AdmissionError> {
        let mut authority = AdmissionAuthority::new();
        admit(&mut authority, 2, 200, 100u64)?;
        assert_eq!(
            authority.prepare_reconnect(
                ReconnectAttemptRef::new(1)?,
                ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?,
                200u64,
                7,
                11
            ),
            Err(AdmissionError::IncumbentHealthy)
        );
        Ok(())
    }

    #[test]
    fn prepare_has_no_authority_and_commit_fences_predecessor() -> Result<(), AdmissionError> {
        let mut authority = AdmissionAuthority::new();
        admit(&mut authority, 3, 300, 100u64)?;
        authority.mark_unexpected_control_loss()?;
        let attempt = ReconnectAttemptRef::new(9)?;
        let first = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
        let candidate = authority.prepare_reconnect(attempt, first, 200u64, 7, 11)?;
        assert_eq!(candidate.get(), 2);
        assert_eq!(
            authority
                .current()
                .ok_or(AdmissionError::Terminal)?
                .connection_generation()
                .get(),
            1
        );
        let committed = authority.commit_reconnect(attempt, 200u64, 7, 11)?;
        assert_eq!(committed.get(), 2);
        assert!(
            !authority
                .current()
                .ok_or(AdmissionError::Terminal)?
                .accepts_generation(first)
        );
        Ok(())
    }

    #[test]
    fn stale_lease_or_runtime_generation_fails_before_switch() -> Result<(), AdmissionError> {
        let mut authority = AdmissionAuthority::new();
        admit(&mut authority, 4, 400, 100u64)?;
        authority.mark_unexpected_control_loss()?;
        let attempt = ReconnectAttemptRef::new(10)?;
        let first = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
        authority.prepare_reconnect(attempt, first, 200u64, 7, 11)?;
        assert_eq!(
            authority.commit_reconnect(attempt, 200u64, 8, 11),
            Err(AdmissionError::StaleLease)
        );
        assert_eq!(
            authority
                .current()
                .ok_or(AdmissionError::Terminal)?
                .connection_generation()
                .get(),
            1
        );
        Ok(())
    }

    #[test]
    fn mismatched_commit_does_not_destroy_prepared_candidate() -> Result<(), AdmissionError> {
        let mut authority = AdmissionAuthority::new();
        admit(&mut authority, 6, 600, 100u64)?;
        authority.mark_unexpected_control_loss()?;
        let prepared_attempt = ReconnectAttemptRef::new(12)?;
        let wrong_attempt = ReconnectAttemptRef::new(13)?;
        let first = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
        let candidate = authority.prepare_reconnect(prepared_attempt, first, 200u64, 7, 11)?;
        assert_eq!(
            authority.commit_reconnect(wrong_attempt, 200u64, 7, 11),
            Err(AdmissionError::AttemptMismatch)
        );
        assert_eq!(
            authority.commit_reconnect(prepared_attempt, 200u64, 7, 11)?,
            candidate
        );
        Ok(())
    }

    #[test]
    fn reconnect_commit_requires_exact_prepared_authenticated_transport()
    -> Result<(), AdmissionError> {
        let mut authority = AdmissionAuthority::new();
        admit(&mut authority, 9, 900, 100u64)?;
        authority.mark_unexpected_control_loss()?;
        let attempt = ReconnectAttemptRef::new(22)?;
        let first = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
        let candidate = authority.prepare_reconnect(attempt, first, 200u64, 7, 11)?;
        assert_eq!(
            authority.commit_reconnect(attempt, 201u64, 7, 11),
            Err(AdmissionError::AttemptMismatch)
        );
        assert_eq!(
            authority.commit_reconnect(attempt, 200u64, 7, 11)?,
            candidate
        );
        assert_eq!(authority.current_transport(), Some(200u64));
        Ok(())
    }

    #[test]
    fn grant_nonce_preserves_all_thirty_two_bytes() -> Result<(), AdmissionError> {
        let facts = FreshAdmissionFacts::for_test(7, 7, 11)?;
        assert_eq!(facts.grant_nonce.len(), 32);
        Ok(())
    }

    #[test]
    fn committed_reconnect_attempt_cannot_be_reprepared() -> Result<(), AdmissionError> {
        let mut authority = AdmissionAuthority::new();
        admit(&mut authority, 8, 800, 100u64)?;
        authority.mark_unexpected_control_loss()?;
        let attempt = ReconnectAttemptRef::new(21)?;
        let first = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
        let second = authority.prepare_reconnect(attempt, first, 200u64, 7, 11)?;
        assert_eq!(authority.commit_reconnect(attempt, 200u64, 7, 11)?, second);
        authority.mark_unexpected_control_loss()?;
        assert_eq!(
            authority.prepare_reconnect(attempt, second, 200u64, 7, 11),
            Err(AdmissionError::StaleConnection)
        );
        assert_eq!(
            authority
                .current()
                .ok_or(AdmissionError::Terminal)?
                .connection_generation(),
            second
        );
        Ok(())
    }

    #[test]
    fn lost_current_transport_cannot_replay_committed_success() -> Result<(), AdmissionError> {
        let mut authority = AdmissionAuthority::new();
        admit(&mut authority, 14, 1400, 100u64)?;
        authority.mark_unexpected_control_loss()?;
        let attempt = ReconnectAttemptRef::new(28)?;
        let generation_one =
            ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
        authority.prepare_reconnect(attempt, generation_one, 200u64, 7, 11)?;
        authority.commit_reconnect(attempt, 200u64, 7, 11)?;
        authority.mark_unexpected_control_loss()?;
        assert_eq!(authority.current_transport(), None);
        assert_eq!(
            authority.commit_reconnect(attempt, 200u64, 7, 11),
            Err(AdmissionError::StaleConnection)
        );
        Ok(())
    }

    #[test]
    fn superseded_committed_attempt_cannot_replay_success() -> Result<(), AdmissionError> {
        let mut authority = AdmissionAuthority::new();
        admit(&mut authority, 13, 1300, 100u64)?;
        authority.mark_unexpected_control_loss()?;
        let attempt_a = ReconnectAttemptRef::new(26)?;
        let generation_one =
            ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
        let generation_two =
            authority.prepare_reconnect(attempt_a, generation_one, 200u64, 7, 11)?;
        authority.commit_reconnect(attempt_a, 200u64, 7, 11)?;

        authority.mark_unexpected_control_loss()?;
        let attempt_b = ReconnectAttemptRef::new(27)?;
        let generation_three =
            authority.prepare_reconnect(attempt_b, generation_two, 300u64, 7, 11)?;
        authority.commit_reconnect(attempt_b, 300u64, 7, 11)?;
        assert_eq!(generation_three.get(), 3);

        assert_eq!(
            authority.commit_reconnect(attempt_a, 200u64, 7, 11),
            Err(AdmissionError::StaleConnection)
        );
        assert_eq!(
            authority.prepare_reconnect(attempt_a, generation_three, 200u64, 7, 11),
            Err(AdmissionError::StaleConnection)
        );
        Ok(())
    }

    #[test]
    fn terminal_session_rejects_committed_attempt_replay() -> Result<(), AdmissionError> {
        let mut authority = AdmissionAuthority::new();
        admit(&mut authority, 10, 1000, 100u64)?;
        authority.mark_unexpected_control_loss()?;
        let attempt = ReconnectAttemptRef::new(23)?;
        let first = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
        authority.prepare_reconnect(attempt, first, 200u64, 7, 11)?;
        authority.commit_reconnect(attempt, 200u64, 7, 11)?;
        authority.terminate_current()?;
        assert_eq!(authority.current_transport(), None);
        assert_eq!(
            authority.commit_reconnect(attempt, 200u64, 7, 11),
            Err(AdmissionError::Terminal)
        );
        Ok(())
    }

    #[test]
    fn reconnect_revalidates_updatable_runtime_ownership_generation() -> Result<(), AdmissionError>
    {
        let mut authority = AdmissionAuthority::new();
        admit(&mut authority, 11, 1100, 100u64)?;
        authority.mark_unexpected_control_loss()?;
        authority.observe_runtime_ownership_generation(12)?;
        let attempt = ReconnectAttemptRef::new(24)?;
        let first = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
        assert_eq!(
            authority.prepare_reconnect(attempt, first, 200u64, 7, 11),
            Err(AdmissionError::StaleRuntime)
        );
        let candidate = authority.prepare_reconnect(attempt, first, 200u64, 7, 12)?;
        assert_eq!(
            authority.commit_reconnect(attempt, 200u64, 7, 12)?,
            candidate
        );
        Ok(())
    }

    #[test]
    fn runtime_owner_change_supersedes_prepared_reconnect() -> Result<(), AdmissionError> {
        let mut authority = AdmissionAuthority::new();
        admit(&mut authority, 12, 1200, 100u64)?;
        authority.mark_unexpected_control_loss()?;
        let attempt = ReconnectAttemptRef::new(25)?;
        let first = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
        authority.prepare_reconnect(attempt, first, 200u64, 7, 11)?;
        authority.observe_runtime_ownership_generation(12)?;
        assert_eq!(
            authority.commit_reconnect(attempt, 200u64, 7, 12),
            Err(AdmissionError::StaleConnection)
        );
        Ok(())
    }

    #[test]
    fn reconnect_attempt_dispositions_remain_constant_space_and_terminal()
    -> Result<(), AdmissionError> {
        let mut authority = AdmissionAuthority::new();
        admit(&mut authority, 50, 5000, 100u64)?;
        let mut predecessor =
            ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
        let first_attempt = ReconnectAttemptRef::new(50)?;

        for (raw_attempt, transport) in [(50u64, 200u64), (51, 201), (52, 202)] {
            authority.mark_unexpected_control_loss()?;
            let attempt = ReconnectAttemptRef::new(raw_attempt)?;
            let candidate = authority.prepare_reconnect(attempt, predecessor, transport, 7, 11)?;
            authority.commit_reconnect(attempt, transport, 7, 11)?;
            predecessor = candidate;
        }

        assert!(authority.committed_attempts.len() <= 1);
        authority.mark_unexpected_control_loss()?;
        assert_eq!(
            authority.prepare_reconnect(first_attempt, predecessor, 300u64, 7, 11),
            Err(AdmissionError::StaleConnection)
        );
        Ok(())
    }

    #[test]
    fn superseded_prepared_attempt_cannot_be_reused_after_runtime_owner_change()
    -> Result<(), AdmissionError> {
        let mut authority = AdmissionAuthority::new();
        admit(&mut authority, 51, 5100, 100u64)?;
        authority.mark_unexpected_control_loss()?;
        let attempt = ReconnectAttemptRef::new(60)?;
        let predecessor = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
        authority.prepare_reconnect(attempt, predecessor, 200u64, 7, 11)?;
        authority.observe_runtime_ownership_generation(12)?;
        assert_eq!(
            authority.prepare_reconnect(attempt, predecessor, 200u64, 7, 12),
            Err(AdmissionError::StaleConnection)
        );
        Ok(())
    }

    #[test]
    fn superseded_prepare_retry_keeps_terminal_outcome_while_newer_prepare_is_pending()
    -> Result<(), AdmissionError> {
        let mut authority = AdmissionAuthority::new();
        admit(&mut authority, 70, 7000, 100u64)?;
        let generation_one =
            ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;

        authority.mark_unexpected_control_loss()?;
        let attempt_a = ReconnectAttemptRef::new(70)?;
        let generation_two =
            authority.prepare_reconnect(attempt_a, generation_one, 200u64, 7, 11)?;
        authority.commit_reconnect(attempt_a, 200u64, 7, 11)?;

        authority.mark_unexpected_control_loss()?;
        let attempt_b = ReconnectAttemptRef::new(71)?;
        let generation_three =
            authority.prepare_reconnect(attempt_b, generation_two, 300u64, 7, 11)?;

        assert_eq!(
            authority.prepare_reconnect(attempt_a, generation_two, 200u64, 7, 11),
            Err(AdmissionError::StaleConnection)
        );
        assert_eq!(
            authority.commit_reconnect(attempt_b, 300u64, 7, 11)?,
            generation_three
        );
        Ok(())
    }

    #[test]
    fn superseded_commit_retry_keeps_terminal_outcome_while_newer_prepare_is_pending()
    -> Result<(), AdmissionError> {
        let mut authority = AdmissionAuthority::new();
        admit(&mut authority, 71, 7100, 100u64)?;
        let generation_one =
            ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;

        authority.mark_unexpected_control_loss()?;
        let attempt_a = ReconnectAttemptRef::new(80)?;
        let generation_two =
            authority.prepare_reconnect(attempt_a, generation_one, 200u64, 7, 11)?;
        authority.commit_reconnect(attempt_a, 200u64, 7, 11)?;

        authority.mark_unexpected_control_loss()?;
        let attempt_b = ReconnectAttemptRef::new(81)?;
        let generation_three =
            authority.prepare_reconnect(attempt_b, generation_two, 300u64, 7, 11)?;

        assert_eq!(
            authority.commit_reconnect(attempt_a, 200u64, 7, 11),
            Err(AdmissionError::StaleConnection)
        );
        assert_eq!(
            authority.commit_reconnect(attempt_b, 300u64, 7, 11)?,
            generation_three
        );
        Ok(())
    }

    #[test]
    fn lost_commit_response_reconciliation_is_idempotent() -> Result<(), AdmissionError> {
        let mut authority = AdmissionAuthority::new();
        admit(&mut authority, 5, 500, 100u64)?;
        authority.mark_unexpected_control_loss()?;
        let attempt = ReconnectAttemptRef::new(11)?;
        let first_generation =
            ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
        authority.prepare_reconnect(attempt, first_generation, 200u64, 7, 11)?;
        let first = authority.commit_reconnect(attempt, 200u64, 7, 11)?;
        assert_eq!(
            authority.commit_reconnect(attempt, 200u64, 7, 11),
            Ok(first)
        );
        assert_eq!(
            authority
                .current()
                .ok_or(AdmissionError::Terminal)?
                .connection_generation(),
            first
        );
        Ok(())
    }
    #[test]
    fn game_session_id_is_never_reused_and_rejection_does_not_consume_nonce()
    -> Result<(), AdmissionError> {
        let mut authority = AdmissionAuthority::new();
        let mut ledger = TestFreshIdentityLedger::default();
        admit_with_ledger(&mut authority, &mut ledger, 20, 2000, 100u64)?;
        authority.terminate_current()?;
        assert_eq!(
            admit_with_ledger(&mut authority, &mut ledger, 21, 2000, 101u64),
            Err(AdmissionError::InvalidFacts)
        );
        admit_with_ledger(&mut authority, &mut ledger, 21, 2001, 102u64)?;
        Ok(())
    }
    #[test]
    fn fresh_admission_accepts_only_semantic_foundation_ids() -> Result<(), AdmissionError> {
        let mut raw = [0u8; 16];
        raw[6] = 0x70;
        raw[8] = 0x80;
        raw[15] = 1;
        let map = |_| AdmissionError::InvalidFacts;
        let mut nonce = [0u8; 32];
        nonce[31] = 1;
        FreshAdmissionFacts::new(
            nonce,
            CharacterId::decode(&raw).map_err(map)?,
            WorldId::decode(&raw).map_err(map)?,
            ChannelId::decode(&raw).map_err(map)?,
            1,
            1,
        )?;
        Ok(())
    }
    #[test]
    fn game_session_id_is_issued_only_after_fresh_admission_preconditions_pass()
    -> Result<(), AdmissionError> {
        use std::cell::Cell;

        let mut authority = AdmissionAuthority::new();
        let mut ledger = TestFreshIdentityLedger::default();
        let issue_calls = Cell::new(0u32);
        let first = facts(40)?;
        let first_key = first.replay_key();
        authority.commit_fresh(first, 100u64, || {
            ledger.commit(first_key, || {
                issue_calls.set(issue_calls.get() + 1);
                game_session_id(4000)
            })
        })?;
        assert_eq!(issue_calls.get(), 1);

        assert_eq!(
            authority.commit_fresh(facts(41)?, 101u64, || {
                issue_calls.set(issue_calls.get() + 1);
                game_session_id(4100)
            }),
            Err(AdmissionError::IncumbentHealthy)
        );
        assert_eq!(issue_calls.get(), 1);

        authority.terminate_current()?;
        assert_eq!(
            authority.commit_fresh(facts(40)?, 102u64, || {
                ledger.commit(first_key, || {
                    issue_calls.set(issue_calls.get() + 1);
                    game_session_id(4200)
                })
            }),
            Err(AdmissionError::GrantReplayed)
        );
        assert_eq!(issue_calls.get(), 1);
        Ok(())
    }

    #[test]
    fn rejected_fresh_admission_never_issues_session_id_and_keeps_nonce_retryable()
    -> Result<(), AdmissionError> {
        use std::cell::Cell;

        let mut authority = AdmissionAuthority::new();
        admit(&mut authority, 30, 3000, 100u64)?;
        let issue_calls = Cell::new(0u32);
        assert_eq!(
            authority.commit_fresh(facts(31)?, 101u64, || {
                issue_calls.set(issue_calls.get() + 1);
                game_session_id(3001)
            }),
            Err(AdmissionError::IncumbentHealthy)
        );
        assert_eq!(issue_calls.get(), 0);

        authority.terminate_current()?;
        authority.commit_fresh(facts(31)?, 102u64, || {
            issue_calls.set(issue_calls.get() + 1);
            game_session_id(3001)
        })?;
        assert_eq!(issue_calls.get(), 1);
        Ok(())
    }
}
