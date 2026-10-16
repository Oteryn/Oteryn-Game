//! SPECTATE-GRANT-1 / SESSION-1, D1035–D1039 and D1095.
//!
//! One native owner serializes transitions through `&mut self`. An observer has no actor,
//! Character lease, gameplay command lifecycle or durable state. Every use takes independently
//! refreshed native target facts; an immutable grant is never current target authority.
//! This module does not authenticate Gateway admission or offer a protocol capability.
//! CONTACTS is closed until its native consent owner supplies the accepted consumer.

mod projection;
pub(crate) use projection::{Projection, WorldFrame};
#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::domain::AccountId;
use crate::foundation::{
    CharacterId, ConnectionGeneration, GameSessionAuthoritySnapshot, GameSessionId,
    GameSessionState, RuntimeScopeRefV1, ScopeOwnershipGeneration,
};
use aws_lc_rs::{constant_time::verify_slices_are_equal, rand::fill};

const OBSERVERS_PER_TARGET: usize = 50; // SPECTATE1-RL-01
const OBSERVERS_PER_ACCOUNT: usize = 2; // SPECTATE1-RL-02, node-local; O2 owns cross-node admission
const CODE_ATTEMPTS: u8 = 5; // SPECTATE1-RL-04
const CODE_WINDOW: Duration = Duration::from_secs(60);
const EGRESS_ENTRIES: usize = 64; // NET03-OUTBOUND-QUEUE-ENTRIES
const EGRESS_BYTES: usize = 1_048_576; // NET03-OUTBOUND-QUEUE-BYTES

/// Measured node/account-table capacities supplied by composition before allocation. There is
/// no guessed default for SPECTATE1-RL-03. Zero/overflow is rejected before any allocation.
#[derive(Clone, Copy)]
pub(crate) struct Limits {
    pub(crate) node_sessions: usize,
    pub(crate) target_grants: usize,
    pub(crate) attempt_accounts: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Refusal {
    NotAvailable,
    Full,
    Conflict,
    NotAllowed,
    AuditBackpressure,
    InvalidLimits,
    RandomUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EndReason {
    Revoked,
    TargetGone,
    TargetMoved,
    Overload,
    NotObservable,
    ObserverStopped,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct TargetBinding {
    session: GameSessionId,
    character: CharacterId,
    generation: ConnectionGeneration,
    lease: u64,
    scope: RuntimeScopeRefV1,
    owner: ScopeOwnershipGeneration,
}

/// Independently resolved at the consuming owner cycle, never reconstructed from a grant.
/// Content/ruleset observability and PvP are current owner inputs, not client flags.
pub(crate) struct CurrentTarget {
    binding: TargetBinding,
    active: bool,
    observable: bool,
}
impl CurrentTarget {
    pub(crate) fn from_current<T: Copy + Eq>(
        current: GameSessionAuthoritySnapshot<T>,
        scope_observable: bool,
        pvp_enabled: bool,
    ) -> Self {
        let lease = current.current_character_lease();
        Self {
            binding: TargetBinding {
                session: current.current_game_session_id(),
                character: lease.character_id(),
                generation: current.current_connection_generation(),
                lease: lease.generation(),
                scope: current.current_runtime_scope(),
                owner: current.current_scope_generation(),
            },
            active: current.session_state() == GameSessionState::Active
                && current.current_transport().is_some()
                && lease.character_id() == current.commit().character_id()
                && current
                    .current_character_world_eligibility()
                    .is_some_and(|eligible| {
                        eligible.character_id() == lease.character_id()
                            && eligible.world_id() == current.current_runtime_scope().world_id()
                    }),
            observable: scope_observable && !pvp_enabled,
        }
    }
    pub(crate) fn binding(&self) -> TargetBinding {
        self.binding
    }
}

/// Local compare-and-set identity; it grants no authority and is never reused in this owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GrantRevision(u64);

/// A single 128-bit CSPRNG secret (SPECTATE1-RL-06). No Debug, Clone or audit serialization.
pub(crate) struct GrantCode([u8; 16]);
impl GrantCode {
    pub(crate) fn expose_to_target(&self) -> &[u8; 16] {
        &self.0
    }
}
impl fmt::Debug for GrantCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("GrantCode(REDACTED)")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Audience {
    Code,
    Contacts,
}

struct Grant {
    target: TargetBinding,
    revision: GrantRevision,
    audience: Audience,
    code: [u8; 16],
}

/// Normalized Game-side identity only. O2 must authenticate the account, issue/consume admission
/// and enforce its cross-node account cap. This record cannot authenticate itself or open a
/// transport, create an actor, select an endpoint or offer SPECTATE_V1.
#[derive(Clone, Copy)]
pub(crate) struct ObserverIdentity {
    pub(crate) session: GameSessionId,
    pub(crate) generation: ConnectionGeneration,
    pub(crate) account: AccountId,
}

struct Observer {
    identity: ObserverIdentity,
    grant: GrantRevision,
    target: TargetBinding,
    queue: VecDeque<Arc<WorldFrame>>,
    queued_bytes: usize,
}

/// Semantic facts consumed by the existing ANL-01 event foundation adapter. They deliberately
/// contain no code, position, Character/account identity or disclosure payload. This is not a
/// new event codec, retention policy or event-type allocation. Until the real registered adapter
/// acknowledges these facts, backpressure prevents new permissions; no durable audit is claimed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AuditFact {
    pub(crate) ordinal: u64,
    pub(crate) target_session: GameSessionId,
    pub(crate) observer_session: Option<GameSessionId>,
    pub(crate) kind: AuditKind,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AuditKind {
    GrantCreated,
    GrantEnded(EndReason),
    ObserverAdmitted,
    ObserverEnded(EndReason),
}

struct AttemptWindow {
    attempts: VecDeque<Instant>,
    rejected: u64,
}

/// Native process-local owner; destroying it ends every observer, with no restart recovery.
/// Audit capacity reserves one terminal fact for each active grant and observer. Revocation and
/// privacy-triggered termination therefore never wait on an audit consumer or a slow observer.
pub(crate) struct SpectateRuntime {
    limits: Limits,
    grants: BTreeMap<GameSessionId, Grant>,
    observers: BTreeMap<GameSessionId, Observer>,
    attempts: BTreeMap<AccountId, AttemptWindow>,
    audit: VecDeque<AuditFact>,
    audit_limit: usize,
    next_ordinal: u64,
    next_revision: u64,
}

impl SpectateRuntime {
    pub(crate) fn new(limits: Limits) -> Result<Self, Refusal> {
        let audit_limit = limits
            .node_sessions
            .checked_add(limits.target_grants)
            .and_then(|n| n.checked_mul(2))
            .filter(|_| {
                limits.node_sessions > 0 && limits.target_grants > 0 && limits.attempt_accounts > 0
            })
            .ok_or(Refusal::InvalidLimits)?;
        Ok(Self {
            limits,
            grants: BTreeMap::new(),
            observers: BTreeMap::new(),
            attempts: BTreeMap::new(),
            audit: VecDeque::new(),
            audit_limit,
            next_ordinal: 1,
            next_revision: 1,
        })
    }

    fn can_open(&self) -> bool {
        self.audit
            .len()
            .checked_add(self.grants.len())
            .and_then(|n| n.checked_add(self.observers.len()))
            .and_then(|n| n.checked_add(2))
            .is_some_and(|n| n <= self.audit_limit)
            && self.next_ordinal < u64::MAX.saturating_sub(self.audit_limit as u64)
    }
    fn record(&mut self, target: GameSessionId, observer: Option<GameSessionId>, kind: AuditKind) {
        // New permissions reserve terminal space/ordinals before mutation; terminal paths consume
        // that reservation. No fallible work follows a permissive state mutation.
        self.audit.push_back(AuditFact {
            ordinal: self.next_ordinal,
            target_session: target,
            observer_session: observer,
            kind,
        });
        self.next_ordinal += 1;
    }
    /// The adapter acknowledges only after the existing event foundation accepted the exact
    /// head fact. Failure leaves it in place; no dropping, out-of-order ACK or background task.
    pub(crate) fn next_audit(&self) -> Option<&AuditFact> {
        self.audit.front()
    }
    pub(crate) fn acknowledge_audit(&mut self, ordinal: u64) -> Result<(), Refusal> {
        if self
            .audit
            .front()
            .is_none_or(|fact| fact.ordinal != ordinal)
        {
            return Err(Refusal::Conflict);
        }
        self.audit.pop_front();
        Ok(())
    }

    /// Explicit target command, with independently resolved current owner state and expected
    /// grant revision. A losing/stale writer cannot rotate or revoke a newer consent grant.
    pub(crate) fn set_grant(
        &mut self,
        current: &CurrentTarget,
        expected: Option<GrantRevision>,
        audience: Audience,
    ) -> Result<(GrantRevision, Option<GrantCode>), Refusal> {
        if self.observers.contains_key(&current.binding.session) {
            return Err(Refusal::NotAllowed);
        }
        if self
            .grants
            .get(&current.binding.session)
            .map(|g| g.revision)
            != expected
        {
            return Err(Refusal::Conflict);
        }
        self.refresh(current);
        if !current.active || !current.observable {
            return Err(Refusal::NotAvailable);
        }
        let existing = self
            .grants
            .get(&current.binding.session)
            .map(|g| g.revision);
        if existing != expected {
            return Err(Refusal::Conflict);
        }
        if existing.is_none() && self.grants.len() >= self.limits.target_grants {
            return Err(Refusal::Full);
        }
        if !self.can_open() {
            return Err(Refusal::AuditBackpressure);
        }
        let next = self.next_revision.checked_add(1).ok_or(Refusal::Full)?;
        let mut code = [0; 16];
        if audience == Audience::Code {
            fill(&mut code).map_err(|_| Refusal::RandomUnavailable)?;
        }
        let revision = GrantRevision(self.next_revision);
        self.end_target(current.binding.session, EndReason::Revoked);
        self.next_revision = next;
        self.grants.insert(
            current.binding.session,
            Grant {
                target: current.binding,
                revision,
                audience,
                code,
            },
        );
        self.record(current.binding.session, None, AuditKind::GrantCreated);
        Ok((
            revision,
            (audience == Audience::Code).then_some(GrantCode(code)),
        ))
    }

    pub(crate) fn revoke(
        &mut self,
        current: &CurrentTarget,
        expected: GrantRevision,
    ) -> Result<(), Refusal> {
        if self
            .grants
            .get(&current.binding.session)
            .is_none_or(|g| g.revision != expected)
        {
            return Err(Refusal::Conflict);
        }
        self.refresh(current);
        if !current.active
            || self
                .grants
                .get(&current.binding.session)
                .is_none_or(|g| g.revision != expected)
        {
            return Err(Refusal::Conflict);
        }
        self.end_target(current.binding.session, EndReason::Revoked);
        Ok(())
    }

    /// Every native target lifecycle hook and every consuming boundary uses this same end path.
    /// There are no asynchronous in-flight frame tokens: queues are cleared in this owner cycle.
    pub(crate) fn end_target(&mut self, target: GameSessionId, reason: EndReason) {
        let sessions: Vec<_> = self
            .observers
            .iter()
            .filter_map(|(id, o)| (o.target.session == target).then_some(*id))
            .collect();
        for session in sessions {
            self.end_observer(session, reason);
        }
        if self.grants.remove(&target).is_some() {
            self.record(target, None, AuditKind::GrantEnded(reason));
        }
    }
    fn end_observer(&mut self, session: GameSessionId, reason: EndReason) {
        if let Some(observer) = self.observers.remove(&session) {
            self.record(
                observer.target.session,
                Some(session),
                AuditKind::ObserverEnded(reason),
            );
        }
    }
    fn refresh(&mut self, current: &CurrentTarget) {
        // Session replacement is found by Character identity, not UUID ordering. A new session
        // cannot inherit (or leave behind) consent from its predecessor.
        let predecessors: Vec<_> = self
            .grants
            .iter()
            .filter_map(|(id, grant)| {
                (grant.target.character == current.binding.character
                    && *id != current.binding.session)
                    .then_some(*id)
            })
            .collect();
        for session in predecessors {
            self.end_target(session, EndReason::TargetGone);
        }
        let reason = if !current.active {
            Some(EndReason::TargetGone)
        } else if !current.observable {
            Some(EndReason::NotObservable)
        } else {
            self.grants.get(&current.binding.session).and_then(|g| {
                (g.target != current.binding).then_some(
                    if g.target.scope != current.binding.scope
                        || g.target.owner != current.binding.owner
                    {
                        EndReason::TargetMoved
                    } else {
                        EndReason::TargetGone
                    },
                )
            })
        };
        if let Some(reason) = reason {
            self.end_target(current.binding.session, reason);
        }
    }

    fn take_attempt(&mut self, account: AccountId, now: Instant) -> bool {
        // Monotonic owner clock, never a client timestamp; expire before bounded insertion.
        self.attempts.retain(|_, window| {
            while window
                .attempts
                .front()
                .is_some_and(|at| now.saturating_duration_since(*at) >= CODE_WINDOW)
            {
                window.attempts.pop_front();
            }
            !window.attempts.is_empty()
        });
        if !self.attempts.contains_key(&account)
            && self.attempts.len() >= self.limits.attempt_accounts
        {
            return false;
        }
        let window = self.attempts.entry(account).or_insert(AttemptWindow {
            attempts: VecDeque::new(),
            rejected: 0,
        });
        if window.attempts.len() >= usize::from(CODE_ATTEMPTS) {
            return false;
        }
        window.attempts.push_back(now);
        true
    }

    pub(crate) fn admit_code(
        &mut self,
        current: Option<&CurrentTarget>,
        observer: ObserverIdentity,
        code: &[u8],
    ) -> Result<(), Refusal> {
        self.admit_code_at(current, observer, code, Instant::now())
    }
    fn admit_code_at(
        &mut self,
        current: Option<&CurrentTarget>,
        observer: ObserverIdentity,
        code: &[u8],
        now: Instant,
    ) -> Result<(), Refusal> {
        if let Some(current) = current {
            self.refresh(current);
        }
        // Same budget/constant-time fixed-size comparison for missing, wrong, revoked and
        // offline targets. No target existence result or secret is returned on failure.
        let budget = self.take_attempt(observer.account, now);
        let grant = current
            .filter(|t| t.active && t.observable)
            .and_then(|t| self.grants.get(&t.binding.session));
        let expected = grant.map_or([0; 16], |g| g.code);
        let mut supplied = [0; 16];
        if code.len() == 16 {
            supplied.copy_from_slice(code);
        }
        let equal = verify_slices_are_equal(&expected, &supplied).is_ok();
        if !budget
            || !equal
            || code.len() != 16
            || grant.is_none_or(|g| g.audience != Audience::Code)
        {
            if let Some(window) = self.attempts.get_mut(&observer.account) {
                window.rejected = window.rejected.saturating_add(1);
            }
            return Err(Refusal::NotAvailable);
        }
        let grant = grant.ok_or(Refusal::NotAvailable)?;
        if observer.session == grant.target.session
            || self.observers.contains_key(&observer.session)
            || self.grants.contains_key(&observer.session)
        {
            return Err(Refusal::NotAvailable);
        }
        let per_target = self
            .observers
            .values()
            .filter(|o| o.grant == grant.revision)
            .count();
        let per_account = self
            .observers
            .values()
            .filter(|o| o.identity.account == observer.account)
            .count();
        if per_target >= OBSERVERS_PER_TARGET
            || per_account >= OBSERVERS_PER_ACCOUNT
            || self.observers.len() >= self.limits.node_sessions
        {
            return Err(Refusal::Full);
        }
        if !self.can_open() {
            return Err(Refusal::AuditBackpressure);
        }
        let target = grant.target;
        let revision = grant.revision;
        self.observers.insert(
            observer.session,
            Observer {
                identity: observer,
                grant: revision,
                target,
                queue: VecDeque::new(),
                queued_bytes: 0,
            },
        );
        self.record(
            target.session,
            Some(observer.session),
            AuditKind::ObserverAdmitted,
        );
        Ok(())
    }

    /// CONTACTS never means same-account/party/guild or an unverified client list.
    pub(crate) fn admit_contacts(&mut self) -> Result<(), Refusal> {
        Err(Refusal::NotAvailable)
    }

    /// Game-internal abuse signal, never an observer-visible account or target response.
    pub(crate) fn rejected_code_attempts(&self, account: AccountId) -> u64 {
        self.attempts
            .get(&account)
            .map_or(0, |window| window.rejected)
    }

    /// Pre-dispatch table: the caller cannot pass a gameplay handler into this API. Future and
    /// unknown command IDs are refused with the same result as all existing gameplay commands.
    pub(crate) fn dispatch(
        &mut self,
        observer: ObserverIdentity,
        command: ObserverInput,
    ) -> Result<(), Refusal> {
        if self.observers.get(&observer.session).is_none_or(|o| {
            o.identity.generation != observer.generation || o.identity.account != observer.account
        }) {
            return Err(Refusal::NotAvailable);
        }
        match command {
            ObserverInput::Stop => {
                self.end_observer(observer.session, EndReason::ObserverStopped);
                Ok(())
            }
            ObserverInput::Keepalive => Ok(()),
            ObserverInput::Gameplay(_) => Err(Refusal::NotAllowed),
        }
    }

    /// Reuse only existing world payload encoders. No authoritative reads, awaits or locks;
    /// overflow closes that observer and releases all its queued data, never the target.
    pub(crate) fn project(
        &mut self,
        current: &CurrentTarget,
        projection: Projection<'_>,
    ) -> Result<(), Refusal> {
        self.refresh(current);
        let Some(grant) = self.grants.get(&current.binding.session) else {
            return Err(Refusal::NotAvailable);
        };
        let revision = grant.revision;
        let Some(frame) = projection.encode()? else {
            return Ok(());
        };
        let frame = Arc::new(frame);
        let mut overloaded = Vec::new();
        for (id, observer) in &mut self.observers {
            if observer.grant != revision {
                continue;
            }
            let next = observer.queued_bytes.checked_add(frame.len());
            if observer.queue.len() >= EGRESS_ENTRIES || next.is_none_or(|n| n > EGRESS_BYTES) {
                overloaded.push(*id);
            } else {
                observer.queued_bytes = next.ok_or(Refusal::Full)?;
                observer.queue.push_back(Arc::clone(&frame));
            }
        }
        for id in overloaded {
            self.end_observer(id, EndReason::Overload);
        }
        Ok(())
    }

    /// Immediate transport attempt while this owner retains exclusive access. `Blocked` means
    /// no byte was accepted; the entire frame retains its byte/entry budget. A partial/failing
    /// write must return `Failed` and close that observer. WIRE/O2 supplies the separately
    /// reviewed nonblocking adapter; it must not retain/copy payload for asynchronous writes.
    pub(crate) fn try_write(
        &mut self,
        current: &CurrentTarget,
        identity: ObserverIdentity,
        write_now: impl FnOnce(&WorldFrame) -> WriteAttempt,
    ) -> Result<bool, Refusal> {
        self.refresh(current);
        let observer = self
            .observers
            .get_mut(&identity.session)
            .ok_or(Refusal::NotAvailable)?;
        if observer.target != current.binding
            || observer.identity.generation != identity.generation
            || observer.identity.account != identity.account
        {
            return Err(Refusal::NotAvailable);
        }
        let Some(frame) = observer.queue.front() else {
            return Ok(false);
        };
        match write_now(frame) {
            WriteAttempt::Blocked => return Ok(false),
            WriteAttempt::Failed => {
                self.end_observer(identity.session, EndReason::ObserverStopped);
                return Err(Refusal::NotAvailable);
            }
            WriteAttempt::Written => {}
        }
        observer.queued_bytes -= frame.len();
        observer.queue.pop_front();
        Ok(true)
    }

    /// Target-only state has no observer identities or account linkage.
    pub(crate) fn status(&mut self, current: &CurrentTarget) -> Option<(GrantRevision, usize)> {
        self.refresh(current);
        self.grants.get(&current.binding.session).map(|g| {
            (
                g.revision,
                self.observers
                    .values()
                    .filter(|o| o.grant == g.revision)
                    .count(),
            )
        })
    }
}

#[derive(Clone, Copy)]
pub(crate) enum WriteAttempt {
    Written,
    Blocked,
    Failed,
}

#[derive(Clone, Copy)]
pub(crate) enum ObserverInput {
    Stop,
    Keepalive,
    Gameplay(u32),
}
