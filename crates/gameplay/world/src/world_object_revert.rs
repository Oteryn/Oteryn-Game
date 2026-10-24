//! #162 §7: scope-owned `revert_after_ms` lifecycle records and the per-scope driver that fires
//! them (docs/architecture/OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md
//! §7, direction owner-accepted 2026-09-28; §9 for the attribute-bearing teleporter shape).
//!
//! One record per revert child occurrence (the canonical GAME-INTERACTION
//! `interaction::ChildOccurrenceRef`), `PENDING` -> `IN_FLIGHT` ->
//! `TERMINAL`. A record is created only in the same staged commit as a non-timer-origin forward
//! operation whose `prepare` returned `Publish`, and capacity for it is checked only then. The
//! driver fires due records through `LocalObjectRuntime::apply_scope_operation` with the bound
//! inverse, on behalf of the forward's own event owner (owner decision D91); a timer-origin
//! firing never schedules another record (§7 Rounds 7/8).
//!
//! Lane resolutions of §7's open decisions (recorded at those entries in the proposal):
//! - 1 and 5: `TERMINAL` records are kept for the whole scope generation, which is the
//!   duplicate-delivery horizon. The store holds at most `record_capacity` records in any state
//!   (`WOBJ-RL-04`); a full store fails the forward operation `CAPACITY_EXCEEDED` before commit.
//! - 2, option (a): `PENDING` -> `IN_FLIGHT` -> `TERMINAL` is one synchronous owner-turn step with
//!   no await. An error or panic inside it is fatal to the scope generation, whose restart drops
//!   every record (the driver refuses further work once scope-terminal).
//! - 6: `accept_input` runs first inside that step and the record moves to `IN_FLIGHT` only on
//!   `Ok`; on `Exhausted` it stays `PENDING` and the driver stops as scope-terminal (FND-03).
//!
//! Scope-ephemeral (§7 C2): nothing here is persisted; a scope restart builds a new driver.

use crate::content::{
    LocalObjectIntentFamily, LogicalCell, LoweredActionId, PlacementKey, ProductionKey,
    TransitionBinding, TransitionEventOwner, TransitionKey,
};
use crate::domain::interaction::{ChildOccurrenceRef, InteractionError, SemanticRevisionContext};
use crate::foundation::{
    GenerationError, RuntimeExecutionOrdinal, RuntimeScopeRefV1, ScopeOwnershipGeneration,
    ScopeRuntimeFence, TerminalSemanticOutcome,
};
use crate::world_runtime::{
    LocalObjectOperation, LocalObjectRuntime, ReferenceContentGeneration,
    ScopeLocalObjectOperation, WorldRuntimeError,
};
use oteryn_foundation::{Deadline, MonotonicClock, TimeError};
use std::collections::{BTreeMap, BTreeSet};
use std::convert::Infallible;
use std::fmt::{self, Display, Formatter};
use std::sync::Arc;
use std::time::Duration;

/// `WOBJ-RL-04`: lifecycle records one scope generation may hold, in any state.
pub const MAX_REVERT_RECORDS_PER_SCOPE_GENERATION: usize = 1_024;
/// `WOBJ-RL-05`: due records one driver wake admits; the remainder waits for a later wake.
pub const MAX_DUE_REVERTS_PER_WAKE: usize = 32;
/// `WOBJ-RL-06`: nesting depth of a revert child occurrence (its forward is one less).
pub const MAX_REVERT_OCCURRENCE_DEPTH: usize = 4;
/// `WOBJ-RL-07`: bytes of an occurrence identity's `Debug` rendering, which contains every one
/// of its semantic-key bytes at least once, so it bounds the total key bytes the record retains.
pub const MAX_REVERT_OCCURRENCE_RENDERED_BYTES: usize = 4_096;

/// Unforgeable proof that the caller is the §7 revert driver: its field is private to this
/// module, so no other code can build one. `apply_scope_operation` refuses a timed transition
/// without it, so a timed transition never commits without its lifecycle record.
#[derive(Debug)]
pub struct RevertSchedulingCapability(());

const DISPOSITION_COMMITTED: &str = "COMMITTED";

/// The revert's own GAME-INTERACTION identity (§7 Round 11), derived once at scheduling through
/// the canonical nested-cascade constructor from the caller-supplied forward occurrence, never
/// from a `RuntimeExecutionOrdinal`. §7 names no discriminator values, so this lane uses: the
/// authored action as the interaction definition, the placement as the target, the bound
/// inverse as the edge, and no ordinal (one revert per forward child, action and placement).
pub fn revert_child_occurrence(
    forward: &ChildOccurrenceRef,
    action: &LoweredActionId,
    placement: &PlacementKey,
    inverse: &TransitionKey,
    revisions: &SemanticRevisionContext,
) -> Result<ChildOccurrenceRef, InteractionError> {
    ChildOccurrenceRef::for_child(
        forward,
        action.as_str(),
        placement.as_str(),
        inverse.as_str(),
        None,
        revisions,
    )
}

/// Owner decision D90 (re-arm): the one timed forward `action` may commit from `runtime`'s
/// current state. From the natural state that is the authored forward A -> B; from a §9
/// post-revert variant C it is the lowered re-arm forward C -> B; while no timed edge of
/// `action` leaves the current state (the teleporter is open) it is `None`, and the owning
/// event commits nothing. `transitions` is the bound Content's transition set; only edges this
/// placement times for `action` (and so binds, `LocalObjectRuntime::bind`) are candidates.
/// More than one candidate is an ambiguous authored binding and fails closed.
pub fn select_timed_forward<'a>(
    runtime: &LocalObjectRuntime,
    action: &LoweredActionId,
    transitions: impl IntoIterator<Item = &'a TransitionBinding>,
) -> Result<Option<TransitionKey>, RevertError> {
    let mut candidates = transitions.into_iter().filter(|transition| {
        &transition.source_state == runtime.state_key()
            && runtime.revert_after_ms(&transition.key, action).is_some()
    });
    let Some(first) = candidates.next() else {
        return Ok(None);
    };
    if candidates.any(|other| other.key != first.key) {
        return Err(RevertError::AmbiguousForward);
    }
    Ok(Some(first.key.clone()))
}

/// Owner decision Q2=b (#162 comment 5884513528, §10.4): the timed CREATE forward of `action`
/// whose created state `runtime` is in now, so the pre-authored teleporter is open. `None` from any
/// other state and for every non-CREATE forward, so a TRANSFORM teleporter (the duke, owner
/// decision 3) keeps its kill-while-open no-op. More than one candidate fails closed.
pub fn select_open_create<'a>(
    runtime: &LocalObjectRuntime,
    action: &LoweredActionId,
    transitions: impl IntoIterator<Item = &'a TransitionBinding>,
) -> Result<Option<TransitionKey>, RevertError> {
    let mut candidates = transitions.into_iter().filter(|transition| {
        &transition.target_state == runtime.state_key()
            && LocalObjectIntentFamily::from_key(&transition.normalized_intent_family)
                == Some(LocalObjectIntentFamily::Create)
            && runtime.revert_after_ms(&transition.key, action).is_some()
    });
    let Some(first) = candidates.next() else {
        return Ok(None);
    };
    if candidates.any(|other| other.key != first.key) {
        return Err(RevertError::AmbiguousForward);
    }
    Ok(Some(first.key.clone()))
}

/// Counts rendered bytes and stops, without allocating, once `limit` is exceeded.
struct BoundedRendering {
    limit: usize,
    written: usize,
}

impl fmt::Write for BoundedRendering {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        self.written = self.written.saturating_add(text.len());
        if self.written > self.limit {
            return Err(fmt::Error);
        }
        Ok(())
    }
}

/// `WOBJ-RL-06`/`WOBJ-RL-07`: fail closed on an occurrence identity that is nested deeper than
/// `max_depth` or whose rendering exceeds `MAX_REVERT_OCCURRENCE_RENDERED_BYTES`.
fn check_occurrence_bounds(
    occurrence: &ChildOccurrenceRef,
    max_depth: usize,
) -> Result<(), RevertError> {
    if occurrence.ancestry_depth() > max_depth {
        return Err(RevertError::OccurrenceTooDeep);
    }
    let mut rendering = BoundedRendering {
        limit: MAX_REVERT_OCCURRENCE_RENDERED_BYTES,
        written: 0,
    };
    fmt::write(&mut rendering, format_args!("{occurrence:?}"))
        .map_err(|_| RevertError::OccurrenceTooLarge)
}

/// §7's complete `PENDING` field list, captured once from the forward operation's own commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingRevert {
    scope: RuntimeScopeRefV1,
    scope_generation: ScopeOwnershipGeneration,
    placement: PlacementKey,
    incarnation: u64,
    content_generation: ReferenceContentGeneration,
    deadline: Deadline,
    scheduling_ordinal: RuntimeExecutionOrdinal,
    // One registration per forward resolution in this slice, so always 0.
    sequence: u32,
    inverse: TransitionKey,
    expected_state: ProductionKey,
    expected_revision: u64,
    // Owner decision D91: the forward operation's own event owner. The timer-origin inverse
    // executes on that same owner's behalf; `bind` requires the inverse to be bound to it.
    owner: TransitionEventOwner,
}

impl PendingRevert {
    #[must_use]
    pub fn inverse(&self) -> &TransitionKey {
        &self.inverse
    }

    #[must_use]
    pub fn expected_state(&self) -> &ProductionKey {
        &self.expected_state
    }

    #[must_use]
    pub const fn expected_revision(&self) -> u64 {
        self.expected_revision
    }

    fn due_key(&self, id: &ChildOccurrenceRef) -> DueKey {
        (
            self.deadline,
            self.scheduling_ordinal,
            self.sequence,
            id.clone(),
        )
    }
}

/// The `PENDING`-step fence that rejected a record without it reaching `prepare` (§7's
/// `SCOPE_GENERATION_CHANGED` / `CONTENT_GENERATION_CHANGED` / `INCARNATION_CHANGED`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevertFence {
    ScopeGeneration,
    ContentGeneration,
    /// The stored incarnation no longer matches, or no object is bound at the placement.
    Incarnation,
}

/// A record's one answer to every later presentation of its identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RevertOutcome {
    /// `prepare`'s own outcome: `COMMITTED`, or a rejection with an unchanged disposition.
    Prepared(TerminalSemanticOutcome),
    Fenced(RevertFence),
}

impl RevertOutcome {
    #[must_use]
    pub fn committed(&self) -> bool {
        matches!(self, Self::Prepared(outcome) if outcome.disposition() == DISPOSITION_COMMITTED)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RevertLifecycle {
    Pending(PendingRevert),
    InFlight { ordinal: RuntimeExecutionOrdinal },
    Terminal(RevertOutcome),
}

/// What one presentation of an identity observed or did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RevertPresentation {
    /// `PENDING` and not yet due: nothing ran.
    NotDue,
    /// `IN_FLIGHT`: converged onto the execution already under way; nothing ran.
    InFlight,
    Terminal(RevertOutcome),
}

#[derive(Debug)]
pub enum RevertError {
    /// FND-03 §28: the record store is full; the forward operation committed nothing.
    CapacityExceeded,
    /// The forward occurrence already scheduled this revert; the forward committed nothing.
    DuplicateOccurrence,
    /// `WOBJ-RL-06`: the forward or revert occurrence is nested too deep; nothing committed.
    OccurrenceTooDeep,
    /// `WOBJ-RL-07`: the forward or revert occurrence is too large; nothing committed.
    OccurrenceTooLarge,
    UnknownOccurrence,
    /// D90: more than one timed forward of one action leaves the current state.
    AmbiguousForward,
    /// Owner decision Q2=b: no single `PENDING` revert matches the open teleporter; nothing
    /// changed.
    NoOpenRevert,
    /// Owner decision Q2=b: the re-armed transition is not a CREATE edge of this runtime (a
    /// TRANSFORM teleporter keeps its no-op); nothing changed.
    NotCreateTransition,
    InvalidLimits(&'static str),
    /// Ordinal space exhausted or another fatal step error: no work until a new generation.
    ScopeTerminal,
    Ordinal(GenerationError),
    Time(TimeError),
    Interaction(InteractionError),
    Runtime(WorldRuntimeError),
}

impl Display for RevertError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::CapacityExceeded => formatter.write_str("CAPACITY_EXCEEDED: revert records"),
            Self::DuplicateOccurrence => formatter.write_str("revert occurrence already scheduled"),
            Self::OccurrenceTooDeep => formatter.write_str("revert occurrence exceeds WOBJ-RL-06"),
            Self::OccurrenceTooLarge => formatter.write_str("revert occurrence exceeds WOBJ-RL-07"),
            Self::UnknownOccurrence => formatter.write_str("unknown revert occurrence"),
            Self::NoOpenRevert => formatter.write_str("no pending revert of the open object"),
            Self::NotCreateTransition => formatter.write_str("re-armed transition is not a CREATE"),
            Self::AmbiguousForward => formatter.write_str("ambiguous timed forward transition"),
            Self::InvalidLimits(reason) => write!(formatter, "invalid revert limits: {reason}"),
            Self::ScopeTerminal => formatter.write_str("revert driver is scope-terminal"),
            Self::Ordinal(error) => write!(formatter, "{error}"),
            Self::Time(error) => write!(formatter, "{error}"),
            Self::Interaction(error) => write!(formatter, "{error}"),
            Self::Runtime(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for RevertError {}

/// The scope's one FND-03 §10.2 ordinal issuer; `ScopeRuntimeFence` in production.
pub trait ScopeOrdinalIssuer {
    fn generation(&self) -> ScopeOwnershipGeneration;
    fn accept_input(
        &mut self,
        generation: ScopeOwnershipGeneration,
    ) -> Result<RuntimeExecutionOrdinal, GenerationError>;
}

impl ScopeOrdinalIssuer for ScopeRuntimeFence {
    fn generation(&self) -> ScopeOwnershipGeneration {
        Self::generation(self)
    }

    fn accept_input(
        &mut self,
        generation: ScopeOwnershipGeneration,
    ) -> Result<RuntimeExecutionOrdinal, GenerationError> {
        Self::accept_input(self, generation)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RevertDriverLimits {
    record_capacity: usize,
    due_batch: usize,
}

impl RevertDriverLimits {
    pub fn new(record_capacity: usize, due_batch: usize) -> Result<Self, RevertError> {
        if record_capacity == 0 || record_capacity > MAX_REVERT_RECORDS_PER_SCOPE_GENERATION {
            return Err(RevertError::InvalidLimits(
                "record capacity is outside WOBJ-RL-04",
            ));
        }
        if due_batch == 0 || due_batch > MAX_DUE_REVERTS_PER_WAKE {
            return Err(RevertError::InvalidLimits(
                "due batch is outside WOBJ-RL-05",
            ));
        }
        Ok(Self {
            record_capacity,
            due_batch,
        })
    }

    #[must_use]
    pub const fn registered() -> Self {
        Self {
            record_capacity: MAX_REVERT_RECORDS_PER_SCOPE_GENERATION,
            due_batch: MAX_DUE_REVERTS_PER_WAKE,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForwardOutcome {
    pub outcome: TerminalSemanticOutcome,
    pub scheduled: Option<ChildOccurrenceRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WakeReport {
    /// Admitted records in (deadline, scheduling ordinal, sequence) order, with their outcome.
    pub fired: Vec<(ChildOccurrenceRef, RevertOutcome)>,
    /// More records are already due; they wait for a later wake (bounded batch).
    pub more_due: bool,
}

type DueKey = (Deadline, RuntimeExecutionOrdinal, u32, ChildOccurrenceRef);

/// One per scope owner (`ChannelRuntimeV1`/`InstanceRuntime`), never per object: owns the scope's
/// ordinal issuer, its single clock and its lifecycle-record store.
pub struct ScopeRevertDriver<I> {
    scope: RuntimeScopeRefV1,
    issuer: I,
    clock: Arc<dyn MonotonicClock>,
    limits: RevertDriverLimits,
    records: BTreeMap<ChildOccurrenceRef, RevertLifecycle>,
    due: BTreeSet<DueKey>,
    scope_terminal: bool,
    capability: RevertSchedulingCapability,
}

impl<I: ScopeOrdinalIssuer> ScopeRevertDriver<I> {
    #[must_use]
    pub fn new(
        scope: RuntimeScopeRefV1,
        issuer: I,
        clock: Arc<dyn MonotonicClock>,
        limits: RevertDriverLimits,
    ) -> Self {
        Self {
            scope,
            issuer,
            clock,
            limits,
            records: BTreeMap::new(),
            due: BTreeSet::new(),
            scope_terminal: false,
            capability: RevertSchedulingCapability(()),
        }
    }

    #[must_use]
    pub fn issuer(&self) -> &I {
        &self.issuer
    }

    #[must_use]
    pub fn lifecycle(&self, id: &ChildOccurrenceRef) -> Option<&RevertLifecycle> {
        self.records.get(id)
    }

    #[must_use]
    pub fn record_count(&self) -> usize {
        self.records.len()
    }

    /// The earliest pending deadline: when the scope owner should wake this driver next.
    #[must_use]
    pub fn next_deadline(&self) -> Option<Deadline> {
        self.due.first().map(|(deadline, ..)| *deadline)
    }

    fn accept(
        &mut self,
        generation: ScopeOwnershipGeneration,
    ) -> Result<RuntimeExecutionOrdinal, RevertError> {
        self.issuer.accept_input(generation).map_err(|error| {
            self.scope_terminal = true;
            match error {
                GenerationError::Exhausted => RevertError::ScopeTerminal,
                other => RevertError::Ordinal(other),
            }
        })
    }

    /// A non-timer-origin operation (e.g. an encounter action). When the invoked transition
    /// carries `revert_after_ms` for `action` and `prepare` returns `Publish`, the revert record
    /// is created `PENDING` in the same staged commit; if the store is full, nothing commits.
    pub fn apply_forward(
        &mut self,
        runtime: &mut LocalObjectRuntime,
        occurrence: &ChildOccurrenceRef,
        revisions: &SemanticRevisionContext,
        action: &LoweredActionId,
        operation: &ScopeLocalObjectOperation,
        occupied_cells: &BTreeSet<LogicalCell>,
    ) -> Result<ForwardOutcome, RevertError> {
        if self.scope_terminal {
            return Err(RevertError::ScopeTerminal);
        }
        // D91: another owner's (or a player-use) edge is refused before any ordinal is minted.
        runtime
            .check_scope_owner(operation)
            .map_err(RevertError::Runtime)?;
        let transition = operation.transition_key();
        let placement = runtime.placement_key().clone();
        let timed = match runtime.revert_after_ms(transition, action) {
            None => None,
            Some(ms) => {
                let inverse =
                    runtime
                        .revert_inverse(transition)
                        .cloned()
                        .ok_or(RevertError::Runtime(WorldRuntimeError::InvalidBinding(
                            "timed transition has no bound inverse",
                        )))?;
                // Bounded before anything is minted or committed (WOBJ-RL-06/07).
                check_occurrence_bounds(occurrence, MAX_REVERT_OCCURRENCE_DEPTH - 1)?;
                let id =
                    revert_child_occurrence(occurrence, action, &placement, &inverse, revisions)
                        .map_err(RevertError::Interaction)?;
                check_occurrence_bounds(&id, MAX_REVERT_OCCURRENCE_DEPTH)?;
                Some((ms, inverse, id))
            }
        };
        let generation = self.issuer.generation();
        let scheduling_ordinal = self.accept(generation)?;
        let incarnation = runtime.incarnation();
        let content_generation = runtime.content_generation().clone();
        let owner = operation.owner().clone();
        let scope = self.scope;
        let (records, due, clock, capacity, capability) = (
            &mut self.records,
            &mut self.due,
            &self.clock,
            self.limits.record_capacity,
            &self.capability,
        );
        let mut scheduled = None;
        let staged = runtime.apply_scope_operation(
            scope,
            generation,
            operation,
            occupied_cells,
            Some(capability),
            |publish| {
                let Some((ms, inverse, id)) = timed else {
                    return Ok(());
                };
                // Reached only after `prepare` returned `Publish` (§7 Round 21).
                if records.len() >= capacity {
                    return Err(RevertError::CapacityExceeded);
                }
                if records.contains_key(&id) {
                    return Err(RevertError::DuplicateOccurrence);
                }
                let deadline = Deadline::after(clock.as_ref(), Duration::from_millis(ms))
                    .map_err(RevertError::Time)?;
                let pending = PendingRevert {
                    scope,
                    scope_generation: generation,
                    placement,
                    incarnation,
                    content_generation,
                    deadline,
                    scheduling_ordinal,
                    sequence: 0,
                    inverse,
                    expected_state: publish.next_state.clone(),
                    expected_revision: publish.next_revision,
                    owner,
                };
                due.insert(pending.due_key(&id));
                records.insert(id.clone(), RevertLifecycle::Pending(pending));
                scheduled = Some(id);
                Ok(())
            },
        );
        match staged {
            Ok(Ok(outcome)) => Ok(ForwardOutcome { outcome, scheduled }),
            Ok(Err(error)) => Err(error),
            Err(error) => Err(RevertError::Runtime(error)),
        }
    }

    /// Owner decision Q2=b (#162 comment 5884513528, §10.4): a kill while a pre-authored CREATE
    /// teleporter is open re-arms its one `PENDING` revert to the full `revert_after_ms` of
    /// `create`, counted from now. The object is not touched and no record is added, so
    /// `WOBJ-RL-04` is unaffected. `create` is `select_open_create`'s edge; `owner` is the
    /// executing event's own owner, checked against the edge as for `apply_forward` (D91). The
    /// re-kill is one accepted scope input, so it mints one ordinal, which orders the re-armed
    /// deadline. Returns the re-armed record. A runtime bound under another scope or scope
    /// generation (`StaleRuntimeScope`/`StaleScopeOwnershipGeneration`), a `create` that is not a
    /// CREATE edge of `runtime` (`NotCreateTransition`), or no matching `PENDING` record, fails
    /// closed with nothing changed.
    pub fn rearm_open_create(
        &mut self,
        runtime: &LocalObjectRuntime,
        action: &LoweredActionId,
        create: &TransitionKey,
        owner: &TransitionEventOwner,
    ) -> Result<ChildOccurrenceRef, RevertError> {
        if self.scope_terminal {
            return Err(RevertError::ScopeTerminal);
        }
        let generation = self.issuer.generation();
        // The same scope and generation fence as `apply_scope_operation` (#1204 review): a runtime
        // bound under another scope or scope generation never re-arms this driver's record.
        runtime
            .check_scope_fence(self.scope, generation)
            .map_err(RevertError::Runtime)?;
        // The CREATE family `select_open_create` filters for, enforced at this mutation boundary
        // (#1204 review): an open timed TRANSFORM edge is refused before any ordinal or deadline.
        if runtime.transition_intent_family(create) != Some(LocalObjectIntentFamily::Create) {
            return Err(RevertError::NotCreateTransition);
        }
        let operation = ScopeLocalObjectOperation::new(
            runtime.placement_key().clone(),
            runtime.incarnation(),
            runtime.content_generation().clone(),
            LocalObjectOperation::new(create.clone()),
            runtime.revision(),
            owner.clone(),
        );
        runtime
            .check_scope_owner(&operation)
            .map_err(RevertError::Runtime)?;
        let (Some(ms), Some(inverse)) = (
            runtime.revert_after_ms(create, action),
            runtime.revert_inverse(create),
        ) else {
            return Err(RevertError::Runtime(WorldRuntimeError::InvalidBinding(
                "re-armed transition is not timed for this action at this placement",
            )));
        };
        let mut open = self
            .records
            .iter()
            .filter_map(|(id, lifecycle)| match lifecycle {
                RevertLifecycle::Pending(pending)
                    if pending.scope_generation == generation
                        && &pending.placement == runtime.placement_key()
                        && pending.incarnation == runtime.incarnation()
                        && &pending.content_generation == runtime.content_generation()
                        && &pending.inverse == inverse
                        && &pending.owner == owner
                        && &pending.expected_state == runtime.state_key()
                        && pending.expected_revision == runtime.revision() =>
                {
                    Some((id.clone(), pending.clone()))
                }
                _ => None,
            });
        let (Some((id, pending)), None) = (open.next(), open.next()) else {
            return Err(RevertError::NoOpenRevert);
        };
        let deadline = Deadline::after(self.clock.as_ref(), Duration::from_millis(ms))
            .map_err(RevertError::Time)?;
        let scheduling_ordinal = self.accept(generation)?;
        self.due.remove(&pending.due_key(&id));
        let rearmed = PendingRevert {
            deadline,
            scheduling_ordinal,
            ..pending
        };
        self.due.insert(rearmed.due_key(&id));
        self.records
            .insert(id.clone(), RevertLifecycle::Pending(rearmed));
        Ok(id)
    }

    /// One owner-turn wake: fires at most `due_batch` due records in stored order.
    pub fn wake(
        &mut self,
        runtimes: &mut BTreeMap<PlacementKey, LocalObjectRuntime>,
        occupied_cells: &BTreeSet<LogicalCell>,
    ) -> Result<WakeReport, RevertError> {
        if self.scope_terminal {
            return Err(RevertError::ScopeTerminal);
        }
        let mut fired = Vec::new();
        while fired.len() < self.limits.due_batch {
            let Some((deadline, .., id)) = self.due.first() else {
                break;
            };
            if !deadline.has_elapsed(self.clock.as_ref()) {
                break;
            }
            let id = id.clone();
            match self.present(&id, runtimes, occupied_cells)? {
                RevertPresentation::Terminal(outcome) => fired.push((id, outcome)),
                // Unreachable for a due `PENDING` record in a synchronous step.
                RevertPresentation::NotDue | RevertPresentation::InFlight => {
                    self.scope_terminal = true;
                    return Err(RevertError::ScopeTerminal);
                }
            }
        }
        let more_due = self
            .due
            .first()
            .is_some_and(|(deadline, ..)| deadline.has_elapsed(self.clock.as_ref()));
        Ok(WakeReport { fired, more_due })
    }

    /// §7's fixed presentation order: look up; `TERMINAL` answers verbatim; `IN_FLIGHT`
    /// converges; only a due `PENDING` record reaches the fences, then `accept_input`, then
    /// `prepare`/commit, all in this one synchronous step.
    pub fn present(
        &mut self,
        id: &ChildOccurrenceRef,
        runtimes: &mut BTreeMap<PlacementKey, LocalObjectRuntime>,
        occupied_cells: &BTreeSet<LogicalCell>,
    ) -> Result<RevertPresentation, RevertError> {
        let pending = match self.records.get(id) {
            None => return Err(RevertError::UnknownOccurrence),
            Some(RevertLifecycle::Terminal(outcome)) => {
                return Ok(RevertPresentation::Terminal(outcome.clone()));
            }
            Some(RevertLifecycle::InFlight { .. }) => return Ok(RevertPresentation::InFlight),
            Some(RevertLifecycle::Pending(pending)) => pending.clone(),
        };
        if self.scope_terminal {
            return Err(RevertError::ScopeTerminal);
        }
        if !pending.deadline.has_elapsed(self.clock.as_ref()) {
            return Ok(RevertPresentation::NotDue);
        }
        let fence = if pending.scope_generation != self.issuer.generation() {
            Some(RevertFence::ScopeGeneration)
        } else {
            match runtimes.get(&pending.placement) {
                None => Some(RevertFence::Incarnation),
                Some(runtime) if runtime.content_generation() != &pending.content_generation => {
                    Some(RevertFence::ContentGeneration)
                }
                Some(runtime) if runtime.incarnation() != pending.incarnation => {
                    Some(RevertFence::Incarnation)
                }
                Some(_) => None,
            }
        };
        if let Some(fence) = fence {
            return Ok(self.terminalize(id, &pending, RevertOutcome::Fenced(fence)));
        }
        // Open decision 6: the ordinal first; `IN_FLIGHT` only once it is minted.
        let ordinal = self.accept(pending.scope_generation)?;
        self.due.remove(&pending.due_key(id));
        self.records
            .insert(id.clone(), RevertLifecycle::InFlight { ordinal });
        // Checked by the incarnation fence above; absence here would be a broken owner turn.
        let Some(runtime) = runtimes.get_mut(&pending.placement) else {
            self.scope_terminal = true;
            return Err(RevertError::ScopeTerminal);
        };
        let operation = ScopeLocalObjectOperation::new(
            pending.placement.clone(),
            pending.incarnation,
            pending.content_generation.clone(),
            LocalObjectOperation::new(pending.inverse.clone()),
            pending.expected_revision,
            pending.owner.clone(),
        );
        // Timer origin: never stages a new record, even for a timed inverse (§7 Rounds 7/8).
        let result = runtime.apply_scope_operation(
            pending.scope,
            pending.scope_generation,
            &operation,
            occupied_cells,
            Some(&self.capability),
            |_| Ok::<(), Infallible>(()),
        );
        match result {
            Ok(Ok(outcome)) => Ok(self.terminalize(id, &pending, RevertOutcome::Prepared(outcome))),
            Ok(Err(never)) => match never {},
            Err(error) => {
                // Option (a): the record stays `IN_FLIGHT`; the scope generation is over.
                self.scope_terminal = true;
                Err(RevertError::Runtime(error))
            }
        }
    }

    fn terminalize(
        &mut self,
        id: &ChildOccurrenceRef,
        pending: &PendingRevert,
        outcome: RevertOutcome,
    ) -> RevertPresentation {
        self.due.remove(&pending.due_key(id));
        self.records
            .insert(id.clone(), RevertLifecycle::Terminal(outcome.clone()));
        RevertPresentation::Terminal(outcome)
    }

    #[cfg(test)]
    fn force_in_flight_for_test(&mut self, id: &ChildOccurrenceRef) -> Option<()> {
        let Some(RevertLifecycle::Pending(pending)) = self.records.get(id).cloned() else {
            return None;
        };
        self.due.remove(&pending.due_key(id));
        let ordinal = pending.scheduling_ordinal;
        self.records
            .insert(id.clone(), RevertLifecycle::InFlight { ordinal });
        Some(())
    }

    #[cfg(test)]
    fn issuer_mut_for_test(&mut self) -> &mut I {
        &mut self.issuer
    }
}

/// Test-only stand-in for the scope's one `ScopeRuntimeFence` (constructible only inside
/// Foundation): the same `accept_input` contract, plus a count of minted ordinals.
#[cfg(any(test, feature = "test-support"))]
#[derive(Debug)]
pub struct TestIssuer {
    generation: ScopeOwnershipGeneration,
    next: Option<u64>,
    minted: u64,
}

#[cfg(any(test, feature = "test-support"))]
impl TestIssuer {
    #[must_use]
    pub const fn new(generation: ScopeOwnershipGeneration) -> Self {
        Self {
            generation,
            next: Some(1),
            minted: 0,
        }
    }
}

#[cfg(any(test, feature = "test-support"))]
impl ScopeOrdinalIssuer for TestIssuer {
    fn generation(&self) -> ScopeOwnershipGeneration {
        self.generation
    }

    fn accept_input(
        &mut self,
        generation: ScopeOwnershipGeneration,
    ) -> Result<RuntimeExecutionOrdinal, GenerationError> {
        if generation != self.generation {
            return Err(GenerationError::StaleGeneration);
        }
        let raw = self.next.ok_or(GenerationError::Exhausted)?;
        self.next = raw.checked_add(1);
        self.minted += 1;
        RuntimeExecutionOrdinal::new(raw)
    }
}

#[cfg(test)]
#[path = "world_object_revert_tests.rs"]
mod tests;
