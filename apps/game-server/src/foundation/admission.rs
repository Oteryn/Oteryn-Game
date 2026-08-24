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
/// use oteryn_game_server::foundation::{FreshAdmissionFacts, FreshAdmissionReplayKey};
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
mod tests {
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
            let scope_generation =
                ScopeOwnershipGeneration::new(commit.scope_ownership_generation())
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
            } else if session.state == GameSessionState::Active
                && session.current_transport.is_some()
            {
                Some(AdmissionError::IncumbentHealthy)
            } else if session.state != GameSessionState::Reconnectable
                || session.current_transport.is_some()
                || session.connection_generation != binding.predecessor_generation()
                || binding
                    .predecessor_generation()
                    .get()
                    .checked_add(1)
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

    #[test]
    fn reconnect_attempt_ref_has_stable_durable_encoding() -> Result<(), AdmissionError> {
        let attempt = ReconnectAttemptRef::new(0x0102_0304_0506_0708)?;
        let encoded = attempt.to_be_bytes();
        assert_eq!(encoded, [1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(ReconnectAttemptRef::decode(&encoded)?, attempt);
        assert_eq!(
            ReconnectAttemptRef::decode(&[0u8; 8]),
            Err(AdmissionError::InvalidFacts)
        );
        let short = [1u8; 7];
        assert_eq!(
            ReconnectAttemptRef::decode(&short),
            Err(AdmissionError::InvalidFacts)
        );
        Ok(())
    }

    #[test]
    fn fresh_admission_replay_key_has_stable_durable_encoding() -> Result<(), AdmissionError> {
        let facts = facts(0x0102_0304_0506_0708)?;
        let key = facts.replay_key();
        let encoded = key.to_bytes();
        assert_eq!(encoded.len(), 33);
        assert_eq!(encoded[0], FRESH_ADMISSION_REPLAY_KEY_TAG);
        assert_eq!(FreshAdmissionReplayKey::decode(&encoded)?, key);
        assert_eq!(key.trusted_issuer(), PRE_ADMISSION_TRUSTED_ISSUER);
        assert_eq!(key.profile(), PRE_ADMISSION_PROFILE);

        let mut wrong_tag = encoded;
        wrong_tag[0] = 0xff;
        assert_eq!(
            FreshAdmissionReplayKey::decode(&wrong_tag),
            Err(AdmissionError::InvalidFacts)
        );
        let mut zero_nonce = [0u8; 33];
        zero_nonce[0] = FRESH_ADMISSION_REPLAY_KEY_TAG;
        assert_eq!(
            FreshAdmissionReplayKey::decode(&zero_nonce),
            Err(AdmissionError::InvalidFacts)
        );
        assert_eq!(
            FreshAdmissionReplayKey::decode(&encoded[..32]),
            Err(AdmissionError::InvalidFacts)
        );
        Ok(())
    }

    #[test]
    fn reconnect_journal_serializes_distinct_prepares_per_session() -> Result<(), AdmissionError> {
        let journal = TestReconnectAttemptJournal::default();
        let mut authority = AdmissionAuthority::new(journal.clone());
        admit(&mut authority, 9000, 9000, 100u64)?;
        lose_current(&mut authority)?;
        let session = authority
            .current()
            .ok_or(AdmissionError::Terminal)?
            .game_session_id();
        let generation_two =
            ConnectionGeneration::new(2).map_err(|_| AdmissionError::InvalidFacts)?;
        let attempt_b = ReconnectAttemptRef::new(5)?;
        let attempt_c = ReconnectAttemptRef::new(u64::MAX)?;

        assert_eq!(
            journal.claim_prepared(
                session,
                attempt_b,
                reconnect_binding_for_test(1, 2, 200u64)?
            )?,
            ReconnectAttemptClaim::Claimed
        );
        assert_eq!(
            journal.claim_prepared(
                session,
                attempt_c,
                reconnect_binding_for_test(1, 2, 300u64)?
            )?,
            ReconnectAttemptClaim::RejectedConcurrent
        );
        assert_eq!(
            journal.lookup(session, attempt_c)?,
            Some(ReconnectAttemptDisposition::TerminallySuperseded)
        );
        assert_eq!(
            journal.lookup(session, attempt_b)?,
            Some(ReconnectAttemptDisposition::Prepared {
                candidate_generation: generation_two,
            })
        );
        Ok(())
    }

    #[test]
    fn fresh_identity_replay_history_is_not_retained_in_session_authority() {
        let authority = test_authority();
        assert_eq!(authority.retained_fresh_identity_history(), 0);
    }

    #[test]
    fn fresh_admission_commits_generation_one_and_nonce_replay_fails() -> Result<(), AdmissionError>
    {
        let journal = TestReconnectAttemptJournal::default();
        let mut authority = AdmissionAuthority::new(journal.clone());
        let session = admit(&mut authority, 1, 100, 100u64)?;
        assert_eq!(session.connection_generation().get(), 1);
        assert_eq!(session.character_lease().generation(), 7);
        authority.terminate_current()?;
        assert_eq!(
            authority.commit_fresh(facts(1)?, 100u64, || game_session_id(101)),
            Err(AdmissionError::GrantReplayed)
        );
        Ok(())
    }

    #[test]
    fn fresh_admission_reconciliation_cannot_bind_two_transports_to_one_session()
    -> Result<(), AdmissionError> {
        let journal = TestReconnectAttemptJournal::default();
        let mut first_authority = AdmissionAuthority::new(journal.clone());
        let mut second_authority = AdmissionAuthority::new(journal.clone());
        admit(&mut first_authority, 95, 9500, 100u64)?;
        assert_eq!(first_authority.current_transport(), Some(100));

        assert_eq!(
            second_authority.commit_fresh(facts(95)?, 200u64, || game_session_id(9501)),
            Err(AdmissionError::GrantReplayed)
        );
        assert!(second_authority.current().is_none());
        assert_eq!(second_authority.current_transport(), None);
        Ok(())
    }

    #[test]
    fn fresh_admission_process_recovery_cannot_revive_terminal_session()
    -> Result<(), AdmissionError> {
        let journal = TestReconnectAttemptJournal::default();
        let mut authority = AdmissionAuthority::new(journal.clone());
        admit(&mut authority, 91, 901, 100u64)?;
        authority.terminate_current()?;
        drop(authority);

        let mut recovered_authority = AdmissionAuthority::new(journal.clone());
        assert_eq!(
            recovered_authority.commit_fresh(facts(91)?, 101u64, || game_session_id(902)),
            Err(AdmissionError::GrantReplayed)
        );
        assert!(recovered_authority.current().is_none());
        Ok(())
    }

    #[test]
    fn fresh_admission_process_recovery_cannot_rollback_reconnected_active_generation()
    -> Result<(), AdmissionError> {
        let journal = TestReconnectAttemptJournal::default();
        let mut authority = AdmissionAuthority::new(journal.clone());
        admit(&mut authority, 94, 9400, 100u64)?;
        lose_current(&mut authority)?;
        let attempt = ReconnectAttemptRef::new(94)?;
        let first = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
        let generation_two = authority.prepare_reconnect(attempt, first, 200u64, 7, 11)?;
        authority.commit_reconnect(attempt, 200u64, 7, 11)?;
        assert_eq!(generation_two.get(), 2);
        drop(authority);

        let mut recovered_authority = AdmissionAuthority::new(journal.clone());
        assert_eq!(
            recovered_authority.commit_fresh(facts(94)?, 300u64, || game_session_id(9401)),
            Err(AdmissionError::GrantReplayed)
        );
        assert!(recovered_authority.current().is_none());
        Ok(())
    }

    #[test]
    fn fresh_admission_lost_commit_response_can_reconstruct_the_same_session()
    -> Result<(), AdmissionError> {
        let journal = TestReconnectAttemptJournal::default();
        let mut authority = AdmissionAuthority::new(journal.clone());
        let admission = facts(90)?;
        let session_id = game_session_id(900)?;
        journal.fail_next_fresh_response_after_commit();

        let lost_response = authority.commit_fresh(admission, 100u64, || Ok(session_id));
        assert_eq!(
            lost_response.map(|_| ()),
            Err(AdmissionError::ReconciliationUnavailable)
        );
        assert!(authority.current().is_none());

        let recovered = authority.commit_fresh(admission, 100u64, || game_session_id(901))?;
        assert_eq!(recovered.game_session_id(), session_id);
        assert_eq!(recovered.connection_generation().get(), 1);
        assert_eq!(authority.current_transport(), Some(100));
        Ok(())
    }

    #[test]
    fn healthy_binding_cannot_be_preempted_by_reconnect() -> Result<(), AdmissionError> {
        let mut authority = test_authority();
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
        let mut authority = test_authority();
        admit(&mut authority, 3, 300, 100u64)?;
        lose_current(&mut authority)?;
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
    fn control_loss_lost_response_reconciles_on_exact_retry() -> Result<(), AdmissionError> {
        let journal = TestReconnectAttemptJournal::default();
        let mut authority = AdmissionAuthority::new(journal.clone());
        admit(&mut authority, 97, 9700, 100u64)?;
        let generation_one =
            ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
        journal.fail_next_control_loss_response_after_commit();

        assert_eq!(
            authority.mark_unexpected_control_loss(100u64, generation_one),
            Err(AdmissionError::ReconciliationUnavailable)
        );
        assert_eq!(authority.current_transport(), None);
        assert_eq!(
            authority.prepare_reconnect(
                ReconnectAttemptRef::new(970)?,
                generation_one,
                200u64,
                7,
                11,
            ),
            Err(AdmissionError::ReconciliationUnavailable)
        );

        assert_eq!(
            authority.mark_unexpected_control_loss(100u64, generation_one)?,
            ControlLossDisposition::Applied
        );
        assert_eq!(authority.current_transport(), None);
        assert_eq!(
            authority.current().map(GameSession::state),
            Some(GameSessionState::Reconnectable)
        );
        Ok(())
    }

    #[test]
    fn runtime_scope_lost_response_reconciles_authoritative_generation()
    -> Result<(), AdmissionError> {
        let journal = TestReconnectAttemptJournal::default();
        let mut authority = AdmissionAuthority::new(journal.clone());
        admit(&mut authority, 98, 9800, 100u64)?;
        let generation_eleven =
            ScopeOwnershipGeneration::new(11).map_err(|_| AdmissionError::InvalidFacts)?;
        let old_stamp = {
            let session = authority.current.as_mut().ok_or(AdmissionError::Terminal)?;
            let ordinal = session
                .runtime_scope
                .accept_input(generation_eleven)
                .map_err(|_| AdmissionError::StaleRuntime)?;
            session.runtime_scope.stamp(ordinal)
        };
        journal.fail_next_runtime_scope_response_after_commit();

        assert_eq!(
            authority.observe_runtime_ownership_generation(12),
            Err(AdmissionError::ReconciliationUnavailable)
        );
        assert_eq!(
            authority
                .current()
                .map(GameSession::runtime_scope_generation),
            Some(generation_eleven)
        );
        assert!(
            !authority
                .current()
                .ok_or(AdmissionError::Terminal)?
                .runtime_scope
                .accepts_stamp(old_stamp)
        );

        authority.observe_runtime_ownership_generation(12)?;
        assert_eq!(
            authority
                .current()
                .map(GameSession::runtime_scope_generation),
            Some(ScopeOwnershipGeneration::new(12).map_err(|_| AdmissionError::InvalidFacts)?)
        );
        Ok(())
    }

    #[test]
    fn control_loss_is_authoritative_without_test_only_resynchronization()
    -> Result<(), AdmissionError> {
        let mut authority = test_authority();
        admit(&mut authority, 96, 9600, 100u64)?;
        let generation_one =
            ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;

        assert_eq!(
            authority.mark_unexpected_control_loss(100u64, generation_one)?,
            ControlLossDisposition::Applied
        );

        let attempt = ReconnectAttemptRef::new(96)?;
        let generation_two = authority.prepare_reconnect(attempt, generation_one, 200u64, 7, 11)?;
        assert_eq!(
            authority.commit_reconnect(attempt, 200u64, 7, 11)?,
            generation_two
        );
        Ok(())
    }

    #[test]
    fn reconnect_commit_revalidates_authoritative_lease_inside_atomic_journal_boundary()
    -> Result<(), AdmissionError> {
        let mut authority = test_authority();
        admit(&mut authority, 92, 9200, 100u64)?;
        lose_current(&mut authority)?;
        let attempt = ReconnectAttemptRef::new(92)?;
        let predecessor = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
        let candidate = authority.prepare_reconnect(attempt, predecessor, 200u64, 7, 11)?;
        let (game_session_id, character_id, scope_generation) = {
            let session = authority.current().ok_or(AdmissionError::Terminal)?;
            (
                session.game_session_id(),
                session.character_id(),
                session.runtime_scope_generation(),
            )
        };
        authority.reconnect_attempts.set_authoritative_fences(
            game_session_id,
            CharacterLease {
                character_id,
                generation: 8,
            },
            scope_generation,
        );

        assert_eq!(
            authority.commit_reconnect(attempt, 200u64, 7, 11),
            Err(AdmissionError::StaleLease)
        );
        assert_eq!(
            authority
                .current()
                .ok_or(AdmissionError::Terminal)?
                .connection_generation(),
            predecessor
        );
        assert!(authority.prepared.is_none());
        assert_eq!(
            authority
                .reconnect_attempts
                .lookup(game_session_id, attempt)?,
            Some(ReconnectAttemptDisposition::TerminallySuperseded)
        );
        assert_eq!(candidate.get(), 2);
        Ok(())
    }

    #[test]
    fn reconnect_commit_revalidates_authoritative_runtime_inside_atomic_journal_boundary()
    -> Result<(), AdmissionError> {
        let mut authority = test_authority();
        admit(&mut authority, 93, 9300, 100u64)?;
        lose_current(&mut authority)?;
        let attempt = ReconnectAttemptRef::new(93)?;
        let predecessor = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
        authority.prepare_reconnect(attempt, predecessor, 200u64, 7, 11)?;
        let (game_session_id, character_lease) = {
            let session = authority.current().ok_or(AdmissionError::Terminal)?;
            (session.game_session_id(), session.character_lease())
        };
        let newer_scope =
            ScopeOwnershipGeneration::new(12).map_err(|_| AdmissionError::InvalidFacts)?;
        authority.reconnect_attempts.set_authoritative_fences(
            game_session_id,
            character_lease,
            newer_scope,
        );

        assert_eq!(
            authority.commit_reconnect(attempt, 200u64, 7, 11),
            Err(AdmissionError::StaleRuntime)
        );
        assert_eq!(
            authority
                .current()
                .ok_or(AdmissionError::Terminal)?
                .connection_generation(),
            predecessor
        );
        assert!(authority.prepared.is_none());
        assert_eq!(
            authority
                .reconnect_attempts
                .lookup(game_session_id, attempt)?,
            Some(ReconnectAttemptDisposition::TerminallySuperseded)
        );
        Ok(())
    }

    #[test]
    fn reconnect_commit_revalidates_authoritative_session_binding_inside_atomic_journal_boundary()
    -> Result<(), AdmissionError> {
        let mut authority = test_authority();
        admit(&mut authority, 94, 9400, 100u64)?;
        lose_current(&mut authority)?;
        let attempt = ReconnectAttemptRef::new(94)?;
        let predecessor = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
        authority.prepare_reconnect(attempt, predecessor, 200u64, 7, 11)?;
        let (game_session_id, character_lease, scope_generation) = {
            let session = authority.current().ok_or(AdmissionError::Terminal)?;
            (
                session.game_session_id(),
                session.character_lease(),
                session.runtime_scope_generation(),
            )
        };
        authority.reconnect_attempts.set_authoritative_fences(
            game_session_id,
            character_lease,
            scope_generation,
        );
        authority.reconnect_attempts.set_authoritative_session(
            game_session_id,
            GameSessionState::Active,
            ConnectionGeneration::new(2).map_err(|_| AdmissionError::InvalidFacts)?,
            Some(300u64),
        );

        assert_eq!(
            authority.commit_reconnect(attempt, 200u64, 7, 11),
            Err(AdmissionError::StaleConnection)
        );
        assert_eq!(
            authority
                .current()
                .ok_or(AdmissionError::Terminal)?
                .connection_generation(),
            predecessor
        );
        Ok(())
    }

    #[test]
    fn reconnect_commit_rejects_recovered_controller_without_generation_change()
    -> Result<(), AdmissionError> {
        let mut authority = test_authority();
        admit(&mut authority, 95, 9500, 100u64)?;
        lose_current(&mut authority)?;
        let attempt = ReconnectAttemptRef::new(95)?;
        let predecessor = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
        authority.prepare_reconnect(attempt, predecessor, 200u64, 7, 11)?;
        let game_session_id = authority
            .current()
            .ok_or(AdmissionError::Terminal)?
            .game_session_id();
        authority.reconnect_attempts.set_authoritative_session(
            game_session_id,
            GameSessionState::Active,
            predecessor,
            Some(300u64),
        );

        assert_eq!(
            authority.commit_reconnect(attempt, 200u64, 7, 11),
            Err(AdmissionError::StaleConnection)
        );
        assert_eq!(
            authority
                .current()
                .ok_or(AdmissionError::Terminal)?
                .connection_generation(),
            predecessor
        );
        assert!(authority.prepared.is_none());
        Ok(())
    }

    #[test]
    fn reconnect_commit_rejects_terminal_transition_inside_atomic_journal_boundary()
    -> Result<(), AdmissionError> {
        let mut authority = test_authority();
        admit(&mut authority, 96, 9600, 100u64)?;
        lose_current(&mut authority)?;
        let attempt = ReconnectAttemptRef::new(96)?;
        let predecessor = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
        authority.prepare_reconnect(attempt, predecessor, 200u64, 7, 11)?;
        let game_session_id = authority
            .current()
            .ok_or(AdmissionError::Terminal)?
            .game_session_id();
        authority.reconnect_attempts.set_authoritative_session(
            game_session_id,
            GameSessionState::Terminal,
            predecessor,
            None,
        );

        assert_eq!(
            authority.commit_reconnect(attempt, 200u64, 7, 11),
            Err(AdmissionError::Terminal)
        );
        assert_eq!(
            authority
                .current()
                .ok_or(AdmissionError::Terminal)?
                .connection_generation(),
            predecessor
        );
        assert!(authority.prepared.is_none());
        Ok(())
    }

    #[test]
    fn stale_lease_or_runtime_generation_fails_before_switch() -> Result<(), AdmissionError> {
        let mut authority = test_authority();
        admit(&mut authority, 4, 400, 100u64)?;
        lose_current(&mut authority)?;
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
        let mut authority = test_authority();
        admit(&mut authority, 6, 600, 100u64)?;
        lose_current(&mut authority)?;
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
        let mut authority = test_authority();
        admit(&mut authority, 9, 900, 100u64)?;
        lose_current(&mut authority)?;
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
        let mut authority = test_authority();
        admit(&mut authority, 8, 800, 100u64)?;
        lose_current(&mut authority)?;
        let attempt = ReconnectAttemptRef::new(21)?;
        let first = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
        let second = authority.prepare_reconnect(attempt, first, 200u64, 7, 11)?;
        assert_eq!(authority.commit_reconnect(attempt, 200u64, 7, 11)?, second);
        lose_current(&mut authority)?;
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
    fn delayed_predecessor_loss_cannot_drop_reconnected_controller() -> Result<(), AdmissionError> {
        let mut authority = test_authority();
        admit(&mut authority, 29, 2900, 100u64)?;
        lose_current(&mut authority)?;
        let generation_one =
            ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
        let attempt = ReconnectAttemptRef::new(29)?;
        let generation_two = authority.prepare_reconnect(attempt, generation_one, 200u64, 7, 11)?;
        authority.commit_reconnect(attempt, 200u64, 7, 11)?;

        // Models a delayed close/liveness callback from predecessor transport 100/generation 1.
        assert_eq!(
            authority.mark_unexpected_control_loss(100u64, generation_one)?,
            ControlLossDisposition::StaleIgnored
        );
        assert_eq!(authority.current_transport(), Some(200));
        assert_eq!(
            authority.current().map(GameSession::state),
            Some(GameSessionState::Active)
        );
        assert_eq!(
            authority.current().map(GameSession::connection_generation),
            Some(generation_two)
        );
        assert_eq!(
            authority.mark_unexpected_control_loss(200u64, generation_two)?,
            ControlLossDisposition::Applied
        );
        assert_eq!(authority.current_transport(), None);
        assert_eq!(
            authority.current().map(GameSession::state),
            Some(GameSessionState::Reconnectable)
        );
        Ok(())
    }

    #[test]
    fn lost_current_transport_cannot_replay_committed_success() -> Result<(), AdmissionError> {
        let mut authority = test_authority();
        admit(&mut authority, 14, 1400, 100u64)?;
        lose_current(&mut authority)?;
        let attempt = ReconnectAttemptRef::new(28)?;
        let generation_one =
            ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
        authority.prepare_reconnect(attempt, generation_one, 200u64, 7, 11)?;
        authority.commit_reconnect(attempt, 200u64, 7, 11)?;
        lose_current(&mut authority)?;
        assert_eq!(authority.current_transport(), None);
        assert_eq!(
            authority.commit_reconnect(attempt, 200u64, 7, 11),
            Err(AdmissionError::StaleConnection)
        );
        Ok(())
    }

    #[test]
    fn superseded_committed_attempt_cannot_replay_success() -> Result<(), AdmissionError> {
        let mut authority = test_authority();
        admit(&mut authority, 13, 1300, 100u64)?;
        lose_current(&mut authority)?;
        let attempt_a = ReconnectAttemptRef::new(26)?;
        let generation_one =
            ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
        let generation_two =
            authority.prepare_reconnect(attempt_a, generation_one, 200u64, 7, 11)?;
        authority.commit_reconnect(attempt_a, 200u64, 7, 11)?;

        lose_current(&mut authority)?;
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
        let mut authority = test_authority();
        admit(&mut authority, 10, 1000, 100u64)?;
        lose_current(&mut authority)?;
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
        let mut authority = test_authority();
        admit(&mut authority, 11, 1100, 100u64)?;
        lose_current(&mut authority)?;
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
        let mut authority = test_authority();
        admit(&mut authority, 12, 1200, 100u64)?;
        lose_current(&mut authority)?;
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
    fn reconnect_attempt_history_is_external_and_old_refs_stay_terminal()
    -> Result<(), AdmissionError> {
        let journal = TestReconnectAttemptJournal::default();
        let mut authority = AdmissionAuthority::new(journal.clone());
        admit(&mut authority, 50, 5000, 100u64)?;
        let mut predecessor =
            ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
        let first_attempt = ReconnectAttemptRef::new(50)?;

        for (raw_attempt, transport) in [(50u64, 200u64), (51, 201), (52, 202)] {
            lose_current(&mut authority)?;
            let attempt = ReconnectAttemptRef::new(raw_attempt)?;
            let candidate = authority.prepare_reconnect(attempt, predecessor, transport, 7, 11)?;
            authority.commit_reconnect(attempt, transport, 7, 11)?;
            predecessor = candidate;
        }

        assert!(authority.prepared.is_none());
        assert_eq!(journal.len(), 3);
        lose_current(&mut authority)?;
        assert_eq!(
            authority.prepare_reconnect(first_attempt, predecessor, 300u64, 7, 11),
            Err(AdmissionError::StaleConnection)
        );
        Ok(())
    }

    #[test]
    fn superseded_prepared_attempt_cannot_be_reused_after_runtime_owner_change()
    -> Result<(), AdmissionError> {
        let mut authority = test_authority();
        admit(&mut authority, 51, 5100, 100u64)?;
        lose_current(&mut authority)?;
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
        let mut authority = test_authority();
        admit(&mut authority, 70, 7000, 100u64)?;
        let generation_one =
            ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;

        lose_current(&mut authority)?;
        let attempt_a = ReconnectAttemptRef::new(70)?;
        let generation_two =
            authority.prepare_reconnect(attempt_a, generation_one, 200u64, 7, 11)?;
        authority.commit_reconnect(attempt_a, 200u64, 7, 11)?;

        lose_current(&mut authority)?;
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
        let mut authority = test_authority();
        admit(&mut authority, 71, 7100, 100u64)?;
        let generation_one =
            ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;

        lose_current(&mut authority)?;
        let attempt_a = ReconnectAttemptRef::new(80)?;
        let generation_two =
            authority.prepare_reconnect(attempt_a, generation_one, 200u64, 7, 11)?;
        authority.commit_reconnect(attempt_a, 200u64, 7, 11)?;

        lose_current(&mut authority)?;
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
    fn opaque_reconnect_ref_value_cannot_poison_future_attempts() -> Result<(), AdmissionError> {
        let mut authority = test_authority();
        admit(&mut authority, 73, 7300, 100u64)?;
        lose_current(&mut authority)?;
        let generation_one =
            ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
        let winning_attempt = ReconnectAttemptRef::new(40)?;
        let poisoned_value = ReconnectAttemptRef::new(u64::MAX)?;
        let generation_two =
            authority.prepare_reconnect(winning_attempt, generation_one, 200u64, 7, 11)?;

        assert_eq!(
            authority.prepare_reconnect(poisoned_value, generation_one, 300u64, 7, 11),
            Err(AdmissionError::StaleConnection)
        );
        assert_eq!(
            authority.commit_reconnect(winning_attempt, 200u64, 7, 11)?,
            generation_two
        );

        lose_current(&mut authority)?;
        let later_opaque_attempt = ReconnectAttemptRef::new(7)?;
        let generation_three =
            authority.prepare_reconnect(later_opaque_attempt, generation_two, 400u64, 7, 11)?;
        assert_eq!(
            authority.commit_reconnect(later_opaque_attempt, 400u64, 7, 11)?,
            generation_three
        );
        Ok(())
    }

    #[test]
    fn rejected_concurrent_reconnect_ref_is_terminally_retired() -> Result<(), AdmissionError> {
        let mut authority = test_authority();
        admit(&mut authority, 72, 7200, 100u64)?;
        lose_current(&mut authority)?;
        let generation_one =
            ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
        let attempt_b = ReconnectAttemptRef::new(90)?;
        let attempt_c = ReconnectAttemptRef::new(91)?;
        let generation_two =
            authority.prepare_reconnect(attempt_b, generation_one, 200u64, 7, 11)?;

        assert_eq!(
            authority.prepare_reconnect(attempt_c, generation_one, 300u64, 7, 11),
            Err(AdmissionError::StaleConnection)
        );
        assert_eq!(
            authority.commit_reconnect(attempt_b, 200u64, 7, 11)?,
            generation_two
        );
        lose_current(&mut authority)?;
        assert_eq!(
            authority.prepare_reconnect(attempt_c, generation_two, 300u64, 7, 11),
            Err(AdmissionError::StaleConnection)
        );
        Ok(())
    }
    #[test]
    fn lost_commit_response_reconciliation_is_idempotent() -> Result<(), AdmissionError> {
        let mut authority = test_authority();
        admit(&mut authority, 5, 500, 100u64)?;
        lose_current(&mut authority)?;
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
        let journal = TestReconnectAttemptJournal::default();
        let mut authority = AdmissionAuthority::new(journal.clone());
        admit(&mut authority, 20, 2000, 100u64)?;
        authority.terminate_current()?;
        assert_eq!(
            authority.commit_fresh(facts(21)?, 101u64, || game_session_id(2000)),
            Err(AdmissionError::InvalidFacts)
        );
        authority.commit_fresh(facts(21)?, 102u64, || game_session_id(2001))?;
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

        let journal = TestReconnectAttemptJournal::default();
        let mut authority = AdmissionAuthority::new(journal.clone());
        let issue_calls = Cell::new(0u32);
        let first = facts(40)?;
        authority.commit_fresh(first, 100u64, || {
            issue_calls.set(issue_calls.get() + 1);
            game_session_id(4000)
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
                issue_calls.set(issue_calls.get() + 1);
                game_session_id(4200)
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

        let journal = TestReconnectAttemptJournal::default();
        let mut authority = AdmissionAuthority::new(journal.clone());
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
