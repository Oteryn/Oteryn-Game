use super::{
    ChannelId, CharacterId, ConnectionFence, ConnectionGeneration, GameSessionId,
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
    ReconciliationUnavailable,
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
            Self::ReconciliationUnavailable => {
                "authority outcome requires authoritative reconciliation"
            }
            Self::GenerationExhausted => "connection generation space is exhausted",
            Self::Terminal => "game session is terminal",
        })
    }
}
impl Error for AdmissionError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReconnectAttemptRef(u64);
impl ReconnectAttemptRef {
    pub fn new(v: u64) -> Result<Self, AdmissionError> {
        if v == 0 {
            Err(AdmissionError::InvalidFacts)
        } else {
            Ok(Self(v))
        }
    }

    /// Stable durable encoding for exact journal keys. Byte order is an
    /// encoding detail only; it grants no ordering or recency semantics.
    #[must_use]
    pub const fn to_be_bytes(self) -> [u8; 8] {
        self.0.to_be_bytes()
    }

    pub fn decode(input: &[u8]) -> Result<Self, AdmissionError> {
        let bytes: [u8; 8] = input.try_into().map_err(|_| AdmissionError::InvalidFacts)?;
        Self::new(u64::from_be_bytes(bytes))
    }
}

pub const PRE_ADMISSION_TRUSTED_ISSUER: &str = "urn:oteryn:platform:game-admission";
pub const PRE_ADMISSION_PROFILE: &str = "oteryn-pre-admission-v1";

/// Stable durable replay identity for one accepted pre-admission grant.
///
/// This type is semantically namespaced by `PRE_ADMISSION_TRUSTED_ISSUER` and
/// `PRE_ADMISSION_PROFILE`; its byte encoding is exactly the canonical 32-byte
/// GrantNonce and grants no ordering semantics. Persistence adapters should keep
/// this typed namespace distinct from unrelated nonce/key domains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FreshAdmissionReplayKey([u8; 32]);

const FRESH_ADMISSION_REPLAY_KEY_TAG: u8 = 1;

impl FreshAdmissionReplayKey {
    #[must_use]
    pub const fn to_bytes(self) -> [u8; 33] {
        let mut encoded = [0u8; 33];
        encoded[0] = FRESH_ADMISSION_REPLAY_KEY_TAG;
        let mut index = 0;
        while index < 32 {
            encoded[index + 1] = self.0[index];
            index += 1;
        }
        encoded
    }

    pub fn decode(input: &[u8]) -> Result<Self, AdmissionError> {
        let encoded: [u8; 33] = input.try_into().map_err(|_| AdmissionError::InvalidFacts)?;
        if encoded[0] != FRESH_ADMISSION_REPLAY_KEY_TAG {
            return Err(AdmissionError::InvalidFacts);
        }
        let mut nonce = [0u8; 32];
        nonce.copy_from_slice(&encoded[1..]);
        if nonce.iter().all(|byte| *byte == 0) {
            return Err(AdmissionError::InvalidFacts);
        }
        Ok(Self(nonce))
    }

    #[must_use]
    pub const fn trusted_issuer(self) -> &'static str {
        PRE_ADMISSION_TRUSTED_ISSUER
    }

    #[must_use]
    pub const fn profile(self) -> &'static str {
        PRE_ADMISSION_PROFILE
    }
}

#[cfg(test)]
type GrantReplayKey = FreshAdmissionReplayKey;

/// A production authority adapter can derive a stable, typed durable replay key
/// without accessing private grant fields.
///
/// ```
/// use oteryn_gameplay_foundation::foundation::{FreshAdmissionFacts, FreshAdmissionReplayKey};
///
/// fn durable_key(facts: FreshAdmissionFacts) -> [u8; 33] {
///     facts.replay_key().to_bytes()
/// }
/// ```
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
    #[must_use]
    pub const fn replay_key(&self) -> FreshAdmissionReplayKey {
        FreshAdmissionReplayKey(self.grant_nonce)
    }

    #[cfg(any(test, feature = "test-support"))]
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

/// Receipt for the game-domain atomic fresh-admission authority commit.
///
/// The trusted commit seam must persist enough information to return this exact
/// receipt again after a lost response or process recovery. Local runtime state
/// is only a projection of this committed authority and may be reconstructed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreshAdmissionCommit<T: Copy + Eq> {
    game_session_id: GameSessionId,
    character_id: CharacterId,
    world_id: WorldId,
    channel_id: ChannelId,
    character_lease_generation: u64,
    scope_ownership_generation: u64,
    connection_generation: ConnectionGeneration,
    initial_transport: T,
}

impl<T: Copy + Eq> FreshAdmissionCommit<T> {
    pub fn from_facts(
        game_session_id: GameSessionId,
        facts: FreshAdmissionFacts,
        initial_transport: T,
    ) -> Result<Self, AdmissionError> {
        let connection_generation = ConnectionFence::fresh_admission().current();
        ScopeOwnershipGeneration::new(facts.scope_ownership_generation)
            .map_err(|_| AdmissionError::InvalidFacts)?;
        Ok(Self {
            game_session_id,
            character_id: facts.character_id,
            world_id: facts.world_id,
            channel_id: facts.channel_id,
            character_lease_generation: facts.character_lease_generation,
            scope_ownership_generation: facts.scope_ownership_generation,
            connection_generation,
            initial_transport,
        })
    }

    #[must_use]
    pub const fn game_session_id(self) -> GameSessionId {
        self.game_session_id
    }

    #[must_use]
    pub const fn connection_generation(self) -> ConnectionGeneration {
        self.connection_generation
    }

    #[must_use]
    pub const fn character_id(self) -> CharacterId {
        self.character_id
    }

    #[must_use]
    pub const fn world_id(self) -> WorldId {
        self.world_id
    }

    #[must_use]
    pub const fn channel_id(self) -> ChannelId {
        self.channel_id
    }

    #[must_use]
    pub const fn character_lease_generation(self) -> u64 {
        self.character_lease_generation
    }

    #[must_use]
    pub const fn scope_ownership_generation(self) -> u64 {
        self.scope_ownership_generation
    }

    #[must_use]
    pub const fn initial_transport(self) -> T {
        self.initial_transport
    }

    fn matches_facts(self, facts: FreshAdmissionFacts) -> bool {
        self.character_id == facts.character_id
            && self.world_id == facts.world_id
            && self.channel_id == facts.channel_id
            && self.character_lease_generation == facts.character_lease_generation
            && self.scope_ownership_generation == facts.scope_ownership_generation
            && self.connection_generation == ConnectionFence::fresh_admission().current()
    }
}

/// Current durable authority result for a consumed fresh-admission grant.
///
/// The trusted fresh-admission seam must return the immutable committed binding
/// together with the current authoritative GameSession lifecycle state from the
/// same fenced authority. A process-local cache is not sufficient recovery
/// evidence: `Reconnectable` and `Terminal` sessions cannot be promoted back to
/// `Active` by replaying the original fresh-entry grant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreshAdmissionAuthoritySnapshot<T: Copy + Eq> {
    commit: FreshAdmissionCommit<T>,
    session_state: GameSessionState,
    current_connection_generation: ConnectionGeneration,
    current_transport: Option<T>,
}

impl<T: Copy + Eq> FreshAdmissionAuthoritySnapshot<T> {
    #[must_use]
    pub const fn new(
        commit: FreshAdmissionCommit<T>,
        session_state: GameSessionState,
        current_connection_generation: ConnectionGeneration,
        current_transport: Option<T>,
    ) -> Self {
        Self {
            commit,
            session_state,
            current_connection_generation,
            current_transport,
        }
    }

    #[must_use]
    pub const fn active(commit: FreshAdmissionCommit<T>) -> Self {
        Self::new(
            commit,
            GameSessionState::Active,
            commit.connection_generation(),
            Some(commit.initial_transport()),
        )
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
pub enum ControlLossDisposition {
    Applied,
    StaleIgnored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameSessionState {
    Active,
    Reconnectable,
    Terminal,
}
#[derive(Debug, PartialEq, Eq)]
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
pub struct ReconnectCommitBinding<T: Copy + Eq> {
    predecessor_generation: ConnectionGeneration,
    candidate_generation: ConnectionGeneration,
    candidate_transport: T,
    character_lease: CharacterLease,
    scope_generation: ScopeOwnershipGeneration,
}

impl<T: Copy + Eq> ReconnectCommitBinding<T> {
    const fn new(
        predecessor_generation: ConnectionGeneration,
        candidate_generation: ConnectionGeneration,
        candidate_transport: T,
        character_lease: CharacterLease,
        scope_generation: ScopeOwnershipGeneration,
    ) -> Self {
        Self {
            predecessor_generation,
            candidate_generation,
            candidate_transport,
            character_lease,
            scope_generation,
        }
    }

    #[must_use]
    pub const fn predecessor_generation(self) -> ConnectionGeneration {
        self.predecessor_generation
    }
    #[must_use]
    pub const fn candidate_generation(self) -> ConnectionGeneration {
        self.candidate_generation
    }
    #[must_use]
    pub const fn candidate_transport(self) -> T {
        self.candidate_transport
    }
    #[must_use]
    pub const fn character_lease(self) -> CharacterLease {
        self.character_lease
    }
    #[must_use]
    pub const fn scope_generation(self) -> ScopeOwnershipGeneration {
        self.scope_generation
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconnectAttemptDisposition {
    Prepared {
        candidate_generation: ConnectionGeneration,
    },
    Committed {
        generation: ConnectionGeneration,
    },
    TerminallySuperseded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconnectAttemptClaim {
    Claimed,
    Existing(ReconnectAttemptDisposition),
    RejectedConcurrent,
}

/// Trusted reconciliation seam for reconnect idempotency.
///
/// `ReconnectAttemptRef` is an opaque equality key. Implementations MUST NOT
/// infer recency, authority or supersession from its numeric value. Durable
/// retention/lifecycle policy belongs to the owning game-domain authority; the
/// Foundation kernel intentionally does not invent a deferred numeric limit.
pub trait ReconnectAttemptJournal<T: Copy + Eq> {
    /// Atomically consumes/reconciles one fresh-admission grant, issues a
    /// never-reused GameSessionId on first success, and publishes the complete
    /// initial session + exact transport binding. Lost-response recovery returns
    /// the same immutable commit together with current authoritative lifecycle,
    /// connection generation and controller transport.
    fn commit_fresh<F>(
        &self,
        facts: FreshAdmissionFacts,
        authenticated_transport: T,
        issue_game_session_id: F,
    ) -> Result<FreshAdmissionAuthoritySnapshot<T>, AdmissionError>
    where
        F: FnOnce() -> Result<GameSessionId, AdmissionError>;

    /// Atomically publishes unexpected control loss only for the exact current
    /// authenticated transport and generation. `Applied` means the durable
    /// GameSession is now reconnectable with no current controller. The exact
    /// same `(transport, generation)` observation MUST reconcile as `Applied`
    /// after a committed transition whose response was lost; unrelated stale
    /// observations remain `StaleIgnored`.
    fn mark_control_loss(
        &self,
        game_session_id: GameSessionId,
        observed_transport: T,
        observed_generation: ConnectionGeneration,
    ) -> Result<ControlLossDisposition, AdmissionError>;

    /// Atomically makes the current GameSession terminal at the exact expected
    /// generation and permanently supersedes any PREPARED reconnect candidate.
    fn terminate_session(
        &self,
        game_session_id: GameSessionId,
        expected_generation: ConnectionGeneration,
    ) -> Result<(), AdmissionError>;

    /// Atomically advances the authoritative RuntimeScope generation from the
    /// exact expected current generation and supersedes any PREPARED reconnect.
    /// Applies/reconciles a monotonic RuntimeScope ownership observation and
    /// returns the current authoritative generation. An implementation must
    /// return the already-current generation when an identical prior transition
    /// committed but its response was lost.
    fn advance_runtime_scope(
        &self,
        game_session_id: GameSessionId,
        expected_current: ScopeOwnershipGeneration,
        observed: ScopeOwnershipGeneration,
    ) -> Result<ScopeOwnershipGeneration, AdmissionError>;

    fn lookup(
        &self,
        game_session_id: GameSessionId,
        attempt: ReconnectAttemptRef,
    ) -> Result<Option<ReconnectAttemptDisposition>, AdmissionError>;

    /// Atomically claims an unseen operation as PREPARED, or returns its
    /// already-authoritative disposition without changing it. At the same
    /// transaction/lock/fenced linearization point as a new disposition and
    /// binding are written, implementations MUST prove the GameSession remains
    /// reconnectable with no current controller, and exact predecessor,
    /// strict-successor candidate, CharacterLease and RuntimeScope ownership
    /// match the supplied binding. A stale candidate must never be published as
    /// PREPARED. Across all authorities for one GameSession, at most one distinct
    /// attempt may be PREPARED: a different concurrent claim must be terminalized
    /// and return `RejectedConcurrent` without disturbing the incumbent candidate.
    fn claim_prepared(
        &self,
        game_session_id: GameSessionId,
        attempt: ReconnectAttemptRef,
        binding: ReconnectCommitBinding<T>,
    ) -> Result<ReconnectAttemptClaim, AdmissionError>;

    /// Atomically gives an unseen losing operation a permanent terminal
    /// disposition, or returns the disposition already recorded for that key.
    fn retire_if_unseen(
        &self,
        game_session_id: GameSessionId,
        attempt: ReconnectAttemptRef,
    ) -> Result<ReconnectAttemptDisposition, AdmissionError>;

    /// Atomically changes the exact PREPARED operation to COMMITTED only when
    /// the complete authoritative reconnect binding is still current.
    ///
    /// At the same linearization point as PREPARED -> COMMITTED, implementations
    /// MUST revalidate: the exact prepared candidate transport, predecessor and
    /// candidate generations, reconnectable lifecycle/no-current-controller,
    /// CharacterLease and RuntimeScope ownership fences. Success must atomically
    /// make the candidate generation/transport authoritative; the process-local
    /// state update after this call is only a projection of that committed fact.
    /// Any superseding lifecycle/controller/fence mismatch must terminalize the
    /// candidate and fail closed before COMMITTED is published.
    fn commit_prepared(
        &self,
        game_session_id: GameSessionId,
        attempt: ReconnectAttemptRef,
        binding: ReconnectCommitBinding<T>,
    ) -> Result<(), AdmissionError>;

    /// Atomically makes the exact PREPARED operation terminal after a newer
    /// authority/lifecycle fact supersedes its candidate.
    fn retire_prepared(
        &self,
        game_session_id: GameSessionId,
        attempt: ReconnectAttemptRef,
        candidate_generation: ConnectionGeneration,
    ) -> Result<(), AdmissionError>;
}

#[derive(Debug)]
pub struct AdmissionAuthority<T: Copy + Eq, J: ReconnectAttemptJournal<T>> {
    current: Option<GameSession>,
    current_transport: Option<T>,
    prepared: Option<PreparedReconnect<T>>,
    control_loss_pending: Option<(T, ConnectionGeneration)>,
    runtime_scope_reconciliation_pending: bool,
    reconnect_attempts: J,
}

impl<T: Copy + Eq, J: ReconnectAttemptJournal<T> + Default> Default for AdmissionAuthority<T, J> {
    fn default() -> Self {
        Self::new(J::default())
    }
}

impl<T: Copy + Eq, J: ReconnectAttemptJournal<T>> AdmissionAuthority<T, J> {
    pub const fn new(reconnect_attempts: J) -> Self {
        Self {
            current: None,
            current_transport: None,
            prepared: None,
            control_loss_pending: None,
            runtime_scope_reconciliation_pending: false,
            reconnect_attempts,
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

    fn retire_prepared_candidate(&mut self) -> Result<(), AdmissionError> {
        let Some(prepared) = self.prepared else {
            return Ok(());
        };
        let game_session_id = self
            .current
            .as_ref()
            .ok_or(AdmissionError::Terminal)?
            .game_session_id;
        self.reconnect_attempts.retire_prepared(
            game_session_id,
            prepared.attempt,
            prepared.candidate,
        )?;
        self.prepared = None;
        Ok(())
    }

    pub fn commit_fresh<F>(
        &mut self,
        facts: FreshAdmissionFacts,
        authenticated_transport: T,
        issue_game_session_id: F,
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
        ScopeOwnershipGeneration::new(facts.scope_ownership_generation)
            .map_err(|_| AdmissionError::InvalidFacts)?;
        self.retire_prepared_candidate()?;

        // The same trusted GameSession authority that owns control-loss and
        // reconnect COMMIT also owns fresh admission. This prevents a durable
        // fresh binding and the reconnect journal from becoming split sources of
        // truth. The returned snapshot is the authoritative committed/reconciled
        // result; local fields below are only a process projection.
        let authority_snapshot = self.reconnect_attempts.commit_fresh(
            facts,
            authenticated_transport,
            issue_game_session_id,
        )?;
        let committed = authority_snapshot.commit();
        if !committed.matches_facts(facts) {
            return Err(AdmissionError::InvalidFacts);
        }
        if committed.initial_transport() != authenticated_transport
            || authority_snapshot.session_state() != GameSessionState::Active
            || authority_snapshot.current_connection_generation()
                != committed.connection_generation()
            || authority_snapshot.current_transport() != Some(authenticated_transport)
        {
            return Err(AdmissionError::GrantReplayed);
        }
        if self.current.as_ref().is_some_and(|session| {
            session.state == GameSessionState::Terminal
                && session.game_session_id == committed.game_session_id
        }) {
            return Err(AdmissionError::GrantReplayed);
        }

        let runtime_generation =
            ScopeOwnershipGeneration::new(committed.scope_ownership_generation)
                .map_err(|_| AdmissionError::InvalidFacts)?;
        self.current_transport = Some(authenticated_transport);
        self.control_loss_pending = None;
        self.runtime_scope_reconciliation_pending = false;
        self.current = Some(GameSession {
            game_session_id: committed.game_session_id,
            character_id: committed.character_id,
            world_id: committed.world_id,
            channel_id: committed.channel_id,
            lease_generation: committed.character_lease_generation,
            runtime_scope: ScopeRuntimeFence::from_external_grant(runtime_generation),
            connection: ConnectionFence::fresh_admission(),
            state: GameSessionState::Active,
        });
        self.current.as_ref().ok_or(AdmissionError::InvalidFacts)
    }

    pub fn terminate_current(&mut self) -> Result<(), AdmissionError> {
        let s = self.current.as_ref().ok_or(AdmissionError::Terminal)?;
        let game_session_id = s.game_session_id;
        let generation = s.connection_generation();
        match self
            .reconnect_attempts
            .terminate_session(game_session_id, generation)
        {
            Ok(()) => {}
            Err(AdmissionError::ReconciliationUnavailable) => {
                let s = self.current.as_mut().ok_or(AdmissionError::Terminal)?;
                s.state = GameSessionState::Terminal;
                self.prepared = None;
                self.current_transport = None;
                self.control_loss_pending = None;
                self.runtime_scope_reconciliation_pending = false;
                return Err(AdmissionError::ReconciliationUnavailable);
            }
            Err(error) => return Err(error),
        }
        let s = self.current.as_mut().ok_or(AdmissionError::Terminal)?;
        s.state = GameSessionState::Terminal;
        self.prepared = None;
        self.current_transport = None;
        self.control_loss_pending = None;
        self.runtime_scope_reconciliation_pending = false;
        Ok(())
    }

    pub fn mark_unexpected_control_loss(
        &mut self,
        observed_transport: T,
        observed_generation: ConnectionGeneration,
    ) -> Result<ControlLossDisposition, AdmissionError> {
        let s = self.current.as_ref().ok_or(AdmissionError::Terminal)?;
        if s.state == GameSessionState::Terminal {
            return Err(AdmissionError::Terminal);
        }
        let observed = (observed_transport, observed_generation);
        match self.control_loss_pending {
            Some(pending) if pending != observed => {
                return Err(AdmissionError::ReconciliationUnavailable);
            }
            None => {
                if self.current_transport != Some(observed_transport)
                    || s.connection_generation() != observed_generation
                {
                    return Ok(ControlLossDisposition::StaleIgnored);
                }
            }
            Some(_) => {}
        }
        let game_session_id = s.game_session_id;
        match self.reconnect_attempts.mark_control_loss(
            game_session_id,
            observed_transport,
            observed_generation,
        ) {
            Ok(ControlLossDisposition::Applied) => {
                let s = self.current.as_mut().ok_or(AdmissionError::Terminal)?;
                s.state = GameSessionState::Reconnectable;
                self.prepared = None;
                self.current_transport = None;
                self.control_loss_pending = None;
                Ok(ControlLossDisposition::Applied)
            }
            Ok(ControlLossDisposition::StaleIgnored)
                if self.control_loss_pending == Some(observed) =>
            {
                Err(AdmissionError::ReconciliationUnavailable)
            }
            Ok(ControlLossDisposition::StaleIgnored) => Ok(ControlLossDisposition::StaleIgnored),
            Err(AdmissionError::ReconciliationUnavailable) => {
                // The durable outcome is unknown. Fence local controller authority
                // immediately and retain the exact operation identity for retry.
                self.current_transport = None;
                self.prepared = None;
                self.control_loss_pending = Some(observed);
                Err(AdmissionError::ReconciliationUnavailable)
            }
            Err(error) => Err(error),
        }
    }

    pub fn observe_runtime_ownership_generation(
        &mut self,
        generation: u64,
    ) -> Result<(), AdmissionError> {
        let observed =
            ScopeOwnershipGeneration::new(generation).map_err(|_| AdmissionError::InvalidFacts)?;
        let s = self.current.as_ref().ok_or(AdmissionError::Terminal)?;
        if s.state == GameSessionState::Terminal {
            return Err(AdmissionError::Terminal);
        }
        let local_current = s.runtime_scope.generation();
        if observed < local_current {
            return Err(AdmissionError::StaleRuntime);
        }
        let game_session_id = s.game_session_id;
        let authoritative = match self.reconnect_attempts.advance_runtime_scope(
            game_session_id,
            local_current,
            observed,
        ) {
            Ok(authoritative) => authoritative,
            Err(error) => {
                let s = self.current.as_mut().ok_or(AdmissionError::Terminal)?;
                s.runtime_scope.invalidate();
                self.prepared = None;
                self.runtime_scope_reconciliation_pending = true;
                return Err(error);
            }
        };
        if authoritative < local_current {
            let s = self.current.as_mut().ok_or(AdmissionError::Terminal)?;
            s.runtime_scope.invalidate();
            self.prepared = None;
            self.runtime_scope_reconciliation_pending = true;
            return Err(AdmissionError::ReconciliationUnavailable);
        }
        if authoritative == local_current {
            if self.runtime_scope_reconciliation_pending {
                return Err(AdmissionError::ReconciliationUnavailable);
            }
            return Ok(());
        }
        let s = self.current.as_mut().ok_or(AdmissionError::Terminal)?;
        s.runtime_scope
            .apply_external_grant(authoritative)
            .map_err(|_| AdmissionError::StaleRuntime)?;
        self.prepared = None;
        self.runtime_scope_reconciliation_pending = false;
        Ok(())
    }

    fn reconcile_known_disposition(
        &self,
        disposition: ReconnectAttemptDisposition,
        candidate_transport: T,
    ) -> Result<ConnectionGeneration, AdmissionError> {
        let s = self.current.as_ref().ok_or(AdmissionError::Terminal)?;
        if s.state == GameSessionState::Terminal {
            return Err(AdmissionError::Terminal);
        }
        match disposition {
            ReconnectAttemptDisposition::TerminallySuperseded => {
                Err(AdmissionError::StaleConnection)
            }
            ReconnectAttemptDisposition::Prepared { .. } => {
                Err(AdmissionError::ReconciliationUnavailable)
            }
            ReconnectAttemptDisposition::Committed { generation } => {
                if self.current_transport != Some(candidate_transport)
                    || s.connection_generation() != generation
                {
                    return Err(AdmissionError::StaleConnection);
                }
                Ok(generation)
            }
        }
    }

    fn reconcile_attempt(
        &self,
        attempt: ReconnectAttemptRef,
        candidate_transport: T,
    ) -> Result<Option<ConnectionGeneration>, AdmissionError> {
        let s = self.current.as_ref().ok_or(AdmissionError::Terminal)?;
        if s.state == GameSessionState::Terminal {
            return Err(AdmissionError::Terminal);
        }
        let Some(disposition) = self.reconnect_attempts.lookup(s.game_session_id, attempt)? else {
            return Ok(None);
        };
        self.reconcile_known_disposition(disposition, candidate_transport)
            .map(Some)
    }

    pub fn prepare_reconnect(
        &mut self,
        attempt: ReconnectAttemptRef,
        predecessor: ConnectionGeneration,
        candidate_transport: T,
        lease: u64,
        scope: u64,
    ) -> Result<ConnectionGeneration, AdmissionError> {
        if self.control_loss_pending.is_some() || self.runtime_scope_reconciliation_pending {
            return Err(AdmissionError::ReconciliationUnavailable);
        }
        if let Some(existing) = self.prepared {
            if existing.attempt == attempt {
                return if existing.candidate_transport == candidate_transport {
                    Ok(existing.candidate)
                } else {
                    Err(AdmissionError::AttemptMismatch)
                };
            }
            if let Some(generation) = self.reconcile_attempt(attempt, candidate_transport)? {
                return Ok(generation);
            }
            let game_session_id = self
                .current
                .as_ref()
                .ok_or(AdmissionError::Terminal)?
                .game_session_id;
            let disposition = self
                .reconnect_attempts
                .retire_if_unseen(game_session_id, attempt)?;
            return self.reconcile_known_disposition(disposition, candidate_transport);
        }
        if let Some(generation) = self.reconcile_attempt(attempt, candidate_transport)? {
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
        let game_session_id = s.game_session_id;
        let candidate = ConnectionGeneration::new(
            predecessor
                .get()
                .checked_add(1)
                .ok_or(AdmissionError::GenerationExhausted)?,
        )
        .map_err(|_| AdmissionError::GenerationExhausted)?;
        let commit_binding = ReconnectCommitBinding::new(
            predecessor,
            candidate,
            candidate_transport,
            s.character_lease(),
            s.runtime_scope.generation(),
        );
        let claim = match self
            .reconnect_attempts
            .claim_prepared(game_session_id, attempt, commit_binding)
        {
            Ok(claim) => claim,
            Err(error) => {
                // The trusted journal observed an authority change at the claim
                // linearization point. The older process projection is no longer
                // admissible evidence and must be reconstructed before retry.
                self.clear_process_projection();
                return Err(error);
            }
        };
        match claim {
            ReconnectAttemptClaim::Claimed => {}
            ReconnectAttemptClaim::Existing(disposition) => {
                let reconciled = self.reconcile_known_disposition(disposition, candidate_transport);
                if matches!(
                    disposition,
                    ReconnectAttemptDisposition::Committed { .. }
                ) && reconciled.is_err()
                {
                    // A peer may have committed this exact attempt after our last
                    // durable read. The reconnectable predecessor projection is
                    // stale and must not survive into the facade's atomic retry.
                    self.clear_process_projection();
                }
                return reconciled;
            }
            ReconnectAttemptClaim::RejectedConcurrent => {
                return Err(AdmissionError::StaleConnection);
            }
        }
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
            if let Some(generation) = self.reconcile_attempt(attempt, candidate_transport)? {
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
            if let Some(generation) = self.reconcile_attempt(attempt, candidate_transport)? {
                return Ok(generation);
            }
            return Err(AdmissionError::AttemptMismatch);
        }
        if prepared.candidate_transport != candidate_transport {
            return Err(AdmissionError::AttemptMismatch);
        }
        let s = self.current.as_ref().ok_or(AdmissionError::Terminal)?;
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
        if prepared.candidate.get()
            != prepared
                .predecessor
                .get()
                .checked_add(1)
                .ok_or(AdmissionError::GenerationExhausted)?
        {
            return Err(AdmissionError::StaleConnection);
        }
        let game_session_id = s.game_session_id;
        let expected_character_lease = s.character_lease();
        let expected_scope_generation = s.runtime_scope.generation();
        let commit_binding = ReconnectCommitBinding::new(
            prepared.predecessor,
            prepared.candidate,
            prepared.candidate_transport,
            expected_character_lease,
            expected_scope_generation,
        );
        match self
            .reconnect_attempts
            .commit_prepared(game_session_id, attempt, commit_binding)
        {
            Ok(()) => {}
            Err(AdmissionError::StaleLease) => {
                self.prepared = None;
                return Err(AdmissionError::StaleLease);
            }
            Err(AdmissionError::StaleRuntime) => {
                self.prepared = None;
                return Err(AdmissionError::StaleRuntime);
            }
            Err(AdmissionError::StaleConnection) => {
                self.prepared = None;
                return Err(AdmissionError::StaleConnection);
            }
            Err(AdmissionError::Terminal) => {
                self.prepared = None;
                return Err(AdmissionError::Terminal);
            }
            Err(error) => return Err(error),
        }

        // Every fallible authority/reconciliation check, including the current
        // lease/runtime fence comparison, is complete at the trusted journal
        // linearization point. The local fence update is now an infallible
        // projection of that already-validated strict successor.
        let s = self.current.as_mut().ok_or(AdmissionError::Terminal)?;
        s.connection
            .commit_prevalidated_successor(prepared.candidate);
        s.state = GameSessionState::Active;
        self.prepared = None;
        self.current_transport = Some(candidate_transport);
        self.control_loss_pending = None;
        self.runtime_scope_reconciliation_pending = false;
        Ok(prepared.candidate)
    }
}

#[cfg(test)]
#[path = "admission_tests.rs"]
mod tests;

/// Process projection for the durable fresh port, independent of the synchronous
/// compatibility journal. A durable receipt alone cannot populate it.
#[derive(Debug, Default)]
pub struct FreshAdmissionProjectionV1 {
    controller: Option<FreshAdmissionCommit<AuthenticatedTransportRefV1>>,
}
impl FreshAdmissionProjectionV1 {
    pub(super) const fn controller(
        &self,
    ) -> Option<FreshAdmissionCommit<AuthenticatedTransportRefV1>> {
        self.controller
    }
    pub(super) fn clear(&mut self) {
        self.controller = None;
    }
    pub(super) fn install_current(
        &mut self,
        expected: FreshAdmissionCommit<AuthenticatedTransportRefV1>,
        current: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
    ) -> Result<(), AdmissionError> {
        self.clear();
        if current.commit() != expected
            || current.session_state() != GameSessionState::Active
            || current.current_connection_generation().get() != 1
            || current.current_transport() != Some(expected.initial_transport())
            || current.current_character_lease().character_id() != expected.character_id()
            || current.current_character_lease().generation()
                != expected.character_lease_generation()
            || current.current_character_world_eligibility()
                != Some(CharacterWorldEligibilityClaimV1::new(
                    expected.character_id(),
                    expected.world_id(),
                ))
            || current.current_runtime_scope()
                != RuntimeScopeRefV1::channel(expected.world_id(), expected.channel_id())
            || current.current_scope_generation().get() != expected.scope_ownership_generation()
            || current.current_control_loss_epoch().is_some()
            || current.current_original_grace_deadline().is_some()
        {
            return Err(AdmissionError::StaleConnection);
        }
        self.controller = Some(expected);
        Ok(())
    }
}

#[cfg(any(test, feature = "test-support"))]
#[path = "admission_test_support.rs"]
mod admission_test_support;
