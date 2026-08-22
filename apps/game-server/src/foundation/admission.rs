use super::{ConnectionFence, ConnectionGeneration, GenerationError};
use std::collections::BTreeSet;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreshAdmissionFacts {
    grant_nonce: [u8; 16],
    game_session_id: [u8; 16],
    character_id: [u8; 16],
    world_id: [u8; 16],
    channel_id: [u8; 16],
    character_lease_generation: u64,
    scope_ownership_generation: u64,
}
impl FreshAdmissionFacts {
    pub fn new(
        grant_nonce: [u8; 16],
        game_session_id: [u8; 16],
        character_id: [u8; 16],
        world_id: [u8; 16],
        channel_id: [u8; 16],
        lease: u64,
        scope: u64,
    ) -> Result<Self, AdmissionError> {
        if [
            grant_nonce,
            game_session_id,
            character_id,
            world_id,
            channel_id,
        ]
        .iter()
        .any(|id| id.iter().all(|b| *b == 0))
            || lease == 0
            || scope == 0
        {
            return Err(AdmissionError::InvalidFacts);
        }
        Ok(Self {
            grant_nonce,
            game_session_id,
            character_id,
            world_id,
            channel_id,
            character_lease_generation: lease,
            scope_ownership_generation: scope,
        })
    }
    #[cfg(test)]
    fn for_test(nonce: u64, session: u64, lease: u64, scope: u64) -> Result<Self, AdmissionError> {
        fn id(v: u64) -> [u8; 16] {
            let mut x = [0u8; 16];
            x[8..].copy_from_slice(&v.to_be_bytes());
            x
        }
        Self::new(id(nonce), id(session), id(1), id(2), id(3), lease, scope)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterLease {
    character_id: [u8; 16],
    generation: u64,
}
impl CharacterLease {
    #[must_use]
    pub const fn character_id(self) -> [u8; 16] {
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
    game_session_id: [u8; 16],
    character_id: [u8; 16],
    world_id: [u8; 16],
    channel_id: [u8; 16],
    lease_generation: u64,
    scope_generation: u64,
    connection: ConnectionFence,
    state: GameSessionState,
}
impl GameSession {
    pub const fn game_session_id(&self) -> [u8; 16] {
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
    pub const fn character_id(&self) -> [u8; 16] {
        self.character_id
    }
    #[must_use]
    pub const fn character_lease(&self) -> CharacterLease {
        CharacterLease {
            character_id: self.character_id,
            generation: self.lease_generation,
        }
    }
    pub const fn world_id(&self) -> [u8; 16] {
        self.world_id
    }
    pub const fn channel_id(&self) -> [u8; 16] {
        self.channel_id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PreparedReconnect {
    attempt: ReconnectAttemptRef,
    predecessor: ConnectionGeneration,
    candidate: ConnectionGeneration,
    lease_generation: u64,
    scope_generation: u64,
}

#[derive(Debug, Default)]
pub struct AdmissionAuthority {
    consumed_grants: BTreeSet<[u8; 16]>,
    current: Option<GameSession>,
    prepared: Option<PreparedReconnect>,
    last_committed: Option<(ReconnectAttemptRef, ConnectionGeneration)>,
}
impl AdmissionAuthority {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn current(&self) -> Option<&GameSession> {
        self.current.as_ref()
    }
    pub fn commit_fresh(
        &mut self,
        facts: FreshAdmissionFacts,
    ) -> Result<&GameSession, AdmissionError> {
        if self.consumed_grants.contains(&facts.grant_nonce) {
            return Err(AdmissionError::GrantReplayed);
        }
        if self
            .current
            .as_ref()
            .is_some_and(|s| s.state != GameSessionState::Terminal)
        {
            return Err(AdmissionError::IncumbentHealthy);
        }
        self.consumed_grants.insert(facts.grant_nonce);
        self.prepared = None;
        self.last_committed = None;
        self.current = Some(GameSession {
            game_session_id: facts.game_session_id,
            character_id: facts.character_id,
            world_id: facts.world_id,
            channel_id: facts.channel_id,
            lease_generation: facts.character_lease_generation,
            scope_generation: facts.scope_ownership_generation,
            connection: ConnectionFence::fresh_admission(),
            state: GameSessionState::Active,
        });
        self.current.as_ref().ok_or(AdmissionError::InvalidFacts)
    }
    pub fn terminate_current(&mut self) -> Result<(), AdmissionError> {
        let s = self.current.as_mut().ok_or(AdmissionError::Terminal)?;
        s.state = GameSessionState::Terminal;
        self.prepared = None;
        Ok(())
    }
    pub fn mark_unexpected_control_loss(&mut self) -> Result<(), AdmissionError> {
        let s = self.current.as_mut().ok_or(AdmissionError::Terminal)?;
        if s.state == GameSessionState::Terminal {
            return Err(AdmissionError::Terminal);
        }
        s.state = GameSessionState::Reconnectable;
        self.prepared = None;
        Ok(())
    }
    pub fn prepare_reconnect(
        &mut self,
        attempt: ReconnectAttemptRef,
        predecessor: ConnectionGeneration,
        lease: u64,
        scope: u64,
    ) -> Result<ConnectionGeneration, AdmissionError> {
        let s = self.current.as_ref().ok_or(AdmissionError::Terminal)?;
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
        if scope != s.scope_generation {
            return Err(AdmissionError::StaleRuntime);
        }
        if let Some(existing) = self.prepared {
            if existing.attempt == attempt {
                return Ok(existing.candidate);
            }
            return Err(AdmissionError::AttemptMismatch);
        }
        let candidate = ConnectionGeneration::new(
            predecessor
                .get()
                .checked_add(1)
                .ok_or(AdmissionError::GenerationExhausted)?,
        )
        .map_err(|_| AdmissionError::GenerationExhausted)?;
        self.prepared = Some(PreparedReconnect {
            attempt,
            predecessor,
            candidate,
            lease_generation: lease,
            scope_generation: scope,
        });
        Ok(candidate)
    }
    pub fn commit_reconnect(
        &mut self,
        attempt: ReconnectAttemptRef,
        lease: u64,
        scope: u64,
    ) -> Result<ConnectionGeneration, AdmissionError> {
        if let Some((done, generation)) = self.last_committed
            && done == attempt
        {
            return Ok(generation);
        }
        let prepared = *self
            .prepared
            .as_ref()
            .ok_or(AdmissionError::AttemptMismatch)?;
        if prepared.attempt != attempt {
            return Err(AdmissionError::AttemptMismatch);
        }
        self.prepared = None;
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
        if scope != s.scope_generation || scope != prepared.scope_generation {
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
        self.last_committed = Some((attempt, generation));
        Ok(generation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(nonce: u64, session: u64) -> Result<FreshAdmissionFacts, AdmissionError> {
        FreshAdmissionFacts::for_test(nonce, session, 7, 11)
    }

    #[test]
    fn fresh_admission_commits_generation_one_and_nonce_replay_fails() -> Result<(), AdmissionError>
    {
        let mut authority = AdmissionAuthority::new();
        let session = authority.commit_fresh(facts(1, 100)?)?;
        assert_eq!(session.connection_generation().get(), 1);
        assert_eq!(session.character_lease().generation(), 7);
        authority.terminate_current()?;
        assert_eq!(
            authority.commit_fresh(facts(1, 101)?),
            Err(AdmissionError::GrantReplayed)
        );
        Ok(())
    }

    #[test]
    fn healthy_binding_cannot_be_preempted_by_reconnect() -> Result<(), AdmissionError> {
        let mut authority = AdmissionAuthority::new();
        authority.commit_fresh(facts(2, 200)?)?;
        assert_eq!(
            authority.prepare_reconnect(
                ReconnectAttemptRef::new(1)?,
                ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?,
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
        authority.commit_fresh(facts(3, 300)?)?;
        authority.mark_unexpected_control_loss()?;
        let attempt = ReconnectAttemptRef::new(9)?;
        let first = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
        let candidate = authority.prepare_reconnect(attempt, first, 7, 11)?;
        assert_eq!(candidate.get(), 2);
        assert_eq!(
            authority
                .current()
                .ok_or(AdmissionError::Terminal)?
                .connection_generation()
                .get(),
            1
        );
        let committed = authority.commit_reconnect(attempt, 7, 11)?;
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
        authority.commit_fresh(facts(4, 400)?)?;
        authority.mark_unexpected_control_loss()?;
        let attempt = ReconnectAttemptRef::new(10)?;
        let first = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
        authority.prepare_reconnect(attempt, first, 7, 11)?;
        assert_eq!(
            authority.commit_reconnect(attempt, 8, 11),
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
        authority.commit_fresh(facts(6, 600)?)?;
        authority.mark_unexpected_control_loss()?;
        let prepared_attempt = ReconnectAttemptRef::new(12)?;
        let wrong_attempt = ReconnectAttemptRef::new(13)?;
        let first = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
        let candidate = authority.prepare_reconnect(prepared_attempt, first, 7, 11)?;
        assert_eq!(
            authority.commit_reconnect(wrong_attempt, 7, 11),
            Err(AdmissionError::AttemptMismatch)
        );
        assert_eq!(
            authority.commit_reconnect(prepared_attempt, 7, 11)?,
            candidate
        );
        Ok(())
    }

    #[test]
    fn lost_commit_response_reconciliation_is_idempotent() -> Result<(), AdmissionError> {
        let mut authority = AdmissionAuthority::new();
        authority.commit_fresh(facts(5, 500)?)?;
        authority.mark_unexpected_control_loss()?;
        let attempt = ReconnectAttemptRef::new(11)?;
        let first_generation =
            ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
        authority.prepare_reconnect(attempt, first_generation, 7, 11)?;
        let first = authority.commit_reconnect(attempt, 7, 11)?;
        assert_eq!(authority.commit_reconnect(attempt, 7, 11), Ok(first));
        assert_eq!(
            authority
                .current()
                .ok_or(AdmissionError::Terminal)?
                .connection_generation(),
            first
        );
        Ok(())
    }
}
