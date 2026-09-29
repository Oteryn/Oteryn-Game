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
use crate::foundation::{
    GenerationError, RuntimeExecutionOrdinal, RuntimeScopeRefV1, ScopeOwnershipGeneration,
    ScopeRuntimeFence, TerminalSemanticOutcome,
};
use crate::interaction::{ChildOccurrenceRef, InteractionError, SemanticRevisionContext};
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
pub(crate) const MAX_REVERT_RECORDS_PER_SCOPE_GENERATION: usize = 1_024;
/// `WOBJ-RL-05`: due records one driver wake admits; the remainder waits for a later wake.
pub(crate) const MAX_DUE_REVERTS_PER_WAKE: usize = 32;
/// `WOBJ-RL-06`: nesting depth of a revert child occurrence (its forward is one less).
pub(crate) const MAX_REVERT_OCCURRENCE_DEPTH: usize = 4;
/// `WOBJ-RL-07`: bytes of an occurrence identity's `Debug` rendering, which contains every one
/// of its semantic-key bytes at least once, so it bounds the total key bytes the record retains.
pub(crate) const MAX_REVERT_OCCURRENCE_RENDERED_BYTES: usize = 4_096;

/// Unforgeable proof that the caller is the §7 revert driver: its field is private to this
/// module, so no other code can build one. `apply_scope_operation` refuses a timed transition
/// without it, so a timed transition never commits without its lifecycle record.
#[derive(Debug)]
pub(crate) struct RevertSchedulingCapability(());

const DISPOSITION_COMMITTED: &str = "COMMITTED";

/// The revert's own GAME-INTERACTION identity (§7 Round 11), derived once at scheduling through
/// the canonical nested-cascade constructor from the caller-supplied forward occurrence, never
/// from a `RuntimeExecutionOrdinal`. §7 names no discriminator values, so this lane uses: the
/// authored action as the interaction definition, the placement as the target, the bound
/// inverse as the edge, and no ordinal (one revert per forward child, action and placement).
pub(crate) fn revert_child_occurrence(
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
pub(crate) fn select_timed_forward<'a>(
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
pub(crate) fn select_open_create<'a>(
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
pub(crate) struct PendingRevert {
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
    pub(crate) fn inverse(&self) -> &TransitionKey {
        &self.inverse
    }

    #[must_use]
    pub(crate) fn expected_state(&self) -> &ProductionKey {
        &self.expected_state
    }

    #[must_use]
    pub(crate) const fn expected_revision(&self) -> u64 {
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
pub(crate) enum RevertFence {
    ScopeGeneration,
    ContentGeneration,
    /// The stored incarnation no longer matches, or no object is bound at the placement.
    Incarnation,
}

/// A record's one answer to every later presentation of its identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RevertOutcome {
    /// `prepare`'s own outcome: `COMMITTED`, or a rejection with an unchanged disposition.
    Prepared(TerminalSemanticOutcome),
    Fenced(RevertFence),
}

impl RevertOutcome {
    #[must_use]
    pub(crate) fn committed(&self) -> bool {
        matches!(self, Self::Prepared(outcome) if outcome.disposition() == DISPOSITION_COMMITTED)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RevertLifecycle {
    Pending(PendingRevert),
    InFlight { ordinal: RuntimeExecutionOrdinal },
    Terminal(RevertOutcome),
}

/// What one presentation of an identity observed or did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RevertPresentation {
    /// `PENDING` and not yet due: nothing ran.
    NotDue,
    /// `IN_FLIGHT`: converged onto the execution already under way; nothing ran.
    InFlight,
    Terminal(RevertOutcome),
}

#[derive(Debug)]
pub(crate) enum RevertError {
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
pub(crate) trait ScopeOrdinalIssuer {
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
pub(crate) struct RevertDriverLimits {
    record_capacity: usize,
    due_batch: usize,
}

impl RevertDriverLimits {
    pub(crate) fn new(record_capacity: usize, due_batch: usize) -> Result<Self, RevertError> {
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
    pub(crate) const fn registered() -> Self {
        Self {
            record_capacity: MAX_REVERT_RECORDS_PER_SCOPE_GENERATION,
            due_batch: MAX_DUE_REVERTS_PER_WAKE,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ForwardOutcome {
    pub(crate) outcome: TerminalSemanticOutcome,
    pub(crate) scheduled: Option<ChildOccurrenceRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WakeReport {
    /// Admitted records in (deadline, scheduling ordinal, sequence) order, with their outcome.
    pub(crate) fired: Vec<(ChildOccurrenceRef, RevertOutcome)>,
    /// More records are already due; they wait for a later wake (bounded batch).
    pub(crate) more_due: bool,
}

type DueKey = (Deadline, RuntimeExecutionOrdinal, u32, ChildOccurrenceRef);

/// One per scope owner (`ChannelRuntimeV1`/`InstanceRuntime`), never per object: owns the scope's
/// ordinal issuer, its single clock and its lifecycle-record store.
pub(crate) struct ScopeRevertDriver<I> {
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
    pub(crate) fn new(
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
    pub(crate) fn issuer(&self) -> &I {
        &self.issuer
    }

    #[must_use]
    pub(crate) fn lifecycle(&self, id: &ChildOccurrenceRef) -> Option<&RevertLifecycle> {
        self.records.get(id)
    }

    #[must_use]
    pub(crate) fn record_count(&self) -> usize {
        self.records.len()
    }

    /// The earliest pending deadline: when the scope owner should wake this driver next.
    #[must_use]
    pub(crate) fn next_deadline(&self) -> Option<Deadline> {
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
    pub(crate) fn apply_forward(
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
    /// deadline. Returns the re-armed record. No matching `PENDING` record fails closed with
    /// nothing changed.
    pub(crate) fn rearm_open_create(
        &mut self,
        runtime: &LocalObjectRuntime,
        action: &LoweredActionId,
        create: &TransitionKey,
        owner: &TransitionEventOwner,
    ) -> Result<ChildOccurrenceRef, RevertError> {
        if self.scope_terminal {
            return Err(RevertError::ScopeTerminal);
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
        let generation = self.issuer.generation();
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
    pub(crate) fn wake(
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
    pub(crate) fn present(
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
#[cfg(test)]
#[derive(Debug)]
pub(crate) struct TestIssuer {
    generation: ScopeOwnershipGeneration,
    next: Option<u64>,
    minted: u64,
}

#[cfg(test)]
impl TestIssuer {
    #[must_use]
    pub(crate) const fn new(generation: ScopeOwnershipGeneration) -> Self {
        Self {
            generation,
            next: Some(1),
            minted: 0,
        }
    }
}

#[cfg(test)]
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
mod tests {
    use super::*;
    use crate::content::encounter_map_item::{
        anchor_placement_key, lower_map_item_transforms, marker_placement,
    };
    use crate::content::{
        CanonicalReferencePlayableContent, ClientProjectionClass, ContentError, ContentLockBinding,
        ContentLockEntry, CoordinateFrameRef, DefinitionFamily, DefinitionRevisionRef,
        EvidenceBindingRef, EvidenceDisposition, LOCAL_OBJECT_TRANSFORM_INTENT_FAMILY,
        LocalObjectCollisionPresence, LocalObjectStateDefinition, MapRevisionRef,
        OwnerCapabilityRequirement, PackageManifestBinding, PlacementRef, ProductionAtom,
        REFERENCE_PLAYABLE_CAPABILITY_PROFILE, REFERENCE_PLAYABLE_CONTENT_PROFILE_ID,
        ReferenceDefinition, ReferenceDefinitionKind, ReferencePlayableContentSource,
        Sha256HexDigest, SpatialAddress, TransitionBinding, TransitionEventOwner,
        TypedDefinitionRef, link_reference_playable,
    };
    use crate::foundation::{
        ChannelId, CharacterId, CharacterLease, CommandId, CommandIngress, CommandRef,
        ConnectionGeneration, FreshAdmissionCommit, FreshAdmissionFacts,
        GameSessionAuthoritySnapshot, GameSessionId, GameSessionState, WorldId,
    };
    use crate::interaction::RootSourceOccurrenceRef;
    use crate::world_runtime::{
        LocalObjectCommand, LocalObjectUseOutcome, ScopeContentGenerationFence,
    };
    use oteryn_foundation::{ManualClock, Moment};
    use serde_json::Value;
    use std::error::Error;

    type TestResult<T = ()> = Result<T, Box<dyn Error>>;

    const DUKE: &str = include_str!(
        "../../../tools/content-schema/encounter-authoring/samples/the_duke_of_the_depths/encounter.json"
    );
    const DUKE_KEY: &str = "canary:encounter/the_duke_of_the_depths";
    const DUKE_ACTION: &str =
        "canary:encounter/the_duke_of_the_depths/the_duke_of_the_depths_death/0";
    const DUKE_DEATH: &str = "canary:occurrence/the_duke_of_the_depths/death/1";
    const SEALED_ITEM: &str = "canary:item/1949";
    const OPEN_ITEM: &str = "canary:item/22761";
    const OPEN_TELEPORTER: &str = "oteryn:reference.transition.depth-teleporter-open";
    // Synthetic, clearly-labeled state-only fixture (§7 Round 18): no authored sample qualifies.
    const WALL_SEALED: &str = "oteryn:reference.state.timed-sealed-wall";
    const WALL_CRACKED: &str = "oteryn:reference.state.timed-cracked-wall";
    const CRACK: &str = "oteryn:reference.transition.timed-wall-crack";
    const MEND: &str = "oteryn:reference.transition.timed-wall-mend";
    const WALL_ACTION: &str = "oteryn:encounter/timed-wall/boss-death/0";
    const WALL_OWNER: &str = "oteryn:encounter/timed-wall";
    const WALL_A: &str = "oteryn:reference.placement.timed-wall-a";
    const WALL_B: &str = "oteryn:reference.placement.timed-wall-b";

    fn fixture<E>(_error: E) -> WorldRuntimeError {
        WorldRuntimeError::InvalidBinding("invalid §7 revert fixture")
    }

    fn uuid_v7(seed: u8) -> [u8; 16] {
        let mut bytes = [0_u8; 16];
        bytes[0] = 1;
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        bytes[15] = seed;
        bytes
    }

    fn world(seed: u8) -> Result<WorldId, WorldRuntimeError> {
        WorldId::decode(&uuid_v7(seed)).map_err(fixture)
    }

    fn scope() -> Result<RuntimeScopeRefV1, WorldRuntimeError> {
        Ok(RuntimeScopeRefV1::channel(
            world(1)?,
            ChannelId::decode(&uuid_v7(3)).map_err(fixture)?,
        ))
    }

    /// An active session in `scope()` at scope generation 1, for the session command path.
    fn session() -> TestResult<(GameSessionAuthoritySnapshot<u64>, GameSessionId)> {
        let character = CharacterId::decode(&uuid_v7(90)).map_err(fixture)?;
        let session = GameSessionId::decode(&uuid_v7(10)).map_err(fixture)?;
        let mut nonce = [0_u8; 32];
        nonce[31] = 10;
        let channel = ChannelId::decode(&uuid_v7(3)).map_err(fixture)?;
        let facts = FreshAdmissionFacts::new(nonce, character, world(1)?, channel, 1, 1)
            .map_err(fixture)?;
        let commit = FreshAdmissionCommit::from_facts(session, facts, 99_u64).map_err(fixture)?;
        let snapshot = GameSessionAuthoritySnapshot::new(
            commit,
            GameSessionState::Active,
            ConnectionGeneration::new(1).map_err(fixture)?,
            Some(99_u64),
            CharacterLease::new(character, 1).map_err(fixture)?,
            generation(1)?,
        );
        Ok((snapshot, session))
    }

    /// The durable session command path (`LocalObjectRuntime::apply`) for one transition.
    fn apply_session(
        runtime: &mut LocalObjectRuntime,
        ingress: &mut CommandIngress,
        command_id: u64,
        transition: &str,
    ) -> TestResult<Result<String, WorldRuntimeError>> {
        let (authority, session) = session()?;
        let command = LocalObjectCommand::new(
            CommandRef::new(session, CommandId::new(command_id).map_err(fixture)?),
            ConnectionGeneration::new(1).map_err(fixture)?,
            runtime.placement_key().clone(),
            runtime.incarnation(),
            runtime.content_generation().clone(),
            LocalObjectOperation::new(TransitionKey::new(transition)?),
            runtime.revision(),
        );
        Ok(runtime
            .apply(&authority, &command, ingress, &BTreeSet::new())
            .map(|result| result.disposition().to_owned()))
    }

    fn generation(value: u64) -> Result<ScopeOwnershipGeneration, WorldRuntimeError> {
        ScopeOwnershipGeneration::new(value).map_err(fixture)
    }

    fn object_definition() -> Result<TypedDefinitionRef, ContentError> {
        Ok(TypedDefinitionRef::new(
            DefinitionFamily::LocalObject,
            ProductionKey::new("oteryn:reference.object.timed-object")?,
            DefinitionRevisionRef::new("definition-r1")?,
        ))
    }

    fn marker_definition() -> Result<TypedDefinitionRef, ContentError> {
        Ok(TypedDefinitionRef::new(
            DefinitionFamily::Terrain,
            ProductionKey::new("oteryn:reference.terrain.encounter-anchor-marker")?,
            DefinitionRevisionRef::new("definition-r1")?,
        ))
    }

    fn state(
        key: &str,
        collision: LocalObjectCollisionPresence,
    ) -> Result<LocalObjectStateDefinition, ContentError> {
        Ok(LocalObjectStateDefinition {
            key: ProductionKey::new(key)?,
            collision,
            attribute_variant_of: None,
            absent: false,
        })
    }

    fn transition(
        key: &str,
        source: &str,
        target: &str,
    ) -> Result<TransitionBinding, ContentError> {
        Ok(TransitionBinding {
            key: TransitionKey::new(key)?,
            definition: object_definition()?,
            source_state: ProductionKey::new(source)?,
            normalized_intent_family: ProductionKey::new(LOCAL_OBJECT_TRANSFORM_INTENT_FAMILY)?,
            target_state: ProductionKey::new(target)?,
            owner_capability: OwnerCapabilityRequirement {
                capability_key: ProductionKey::new(
                    "oteryn:runtime.capability.local-object-transition",
                )?,
            },
            policy_guard_refs: vec![],
        })
    }

    fn source_with(
        states: Vec<LocalObjectStateDefinition>,
        transitions: Vec<TransitionBinding>,
        lock: &str,
    ) -> TestResult<ReferencePlayableContentSource> {
        let package_key = ProductionKey::new("oteryn:content.timed-revert")?;
        let package_revision = ProductionAtom::new("package revision", "package-r1")?;
        let package_manifest = PackageManifestBinding::new(
            package_key.clone(),
            package_revision.clone(),
            ProductionAtom::new("schema", "schema-v1")?,
            ProductionAtom::new("license", "license:project-owned-v1")?,
            Sha256HexDigest::new(
                "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
            )?,
        );
        let provenance = package_manifest.package_provenance_digest()?;
        Ok(ReferencePlayableContentSource {
            profile_revision: ProductionAtom::new(
                "profile",
                REFERENCE_PLAYABLE_CONTENT_PROFILE_ID,
            )?,
            capability_profile: ProductionAtom::new(
                "capability profile",
                REFERENCE_PLAYABLE_CAPABILITY_PROFILE,
            )?,
            package_manifest,
            content_lock: ContentLockBinding {
                revision_digest_token: ProductionAtom::new("content lock", lock)?,
                entries: vec![ContentLockEntry::exact(
                    package_key,
                    package_revision,
                    provenance,
                )],
            },
            world_id: world(1)?,
            coordinate_frame: CoordinateFrameRef::new("global-target-2026-09-27")?,
            definitions: vec![
                ReferenceDefinition {
                    definition: object_definition()?,
                    kind: ReferenceDefinitionKind::LocalObjectStates(states),
                    client_projection: ClientProjectionClass::ClientSafe,
                },
                ReferenceDefinition {
                    definition: marker_definition()?,
                    kind: ReferenceDefinitionKind::Generic,
                    client_projection: ClientProjectionClass::ClientSafe,
                },
            ],
            placements: vec![],
            ordered_placements: vec![],
            transitions,
        })
    }

    /// A deliberately unpromoted placement injected after linking, as the CW4 fixtures do.
    fn placement_at(
        content: &CanonicalReferencePlayableContent,
        key: PlacementKey,
        definition: TypedDefinitionRef,
        x: i32,
    ) -> TestResult<PlacementRef> {
        let witness = EvidenceBindingRef::new(
            ProductionAtom::new("reference manifest revision", "manifest-r0")?,
            ProductionKey::new("oteryn:cw4.timed-revert-placement")?,
            EvidenceDisposition::Unknown,
        );
        Ok(marker_placement(
            key,
            definition,
            MapRevisionRef::new("map-r1")?,
            SpatialAddress {
                world_id: content.world_id,
                coordinate_frame: content.coordinate_frame.clone(),
                cell: LogicalCell { x, y: 10, z: 7 },
                evidence: witness,
            },
        ))
    }

    fn object_at(
        content: &CanonicalReferencePlayableContent,
        key: &PlacementKey,
        initial: &str,
        x: i32,
    ) -> TestResult<PlacementRef> {
        let mut placement = placement_at(content, key.clone(), object_definition()?, x)?;
        placement.local_object_initial_state = Some(ProductionKey::new(initial)?);
        Ok(placement)
    }

    fn duke_anchor() -> Result<PlacementKey, ContentError> {
        anchor_placement_key(DUKE_KEY, "exit_teleporter")
    }

    fn duke_destination(anchor: &str) -> Result<PlacementKey, ContentError> {
        anchor_placement_key(DUKE_KEY, anchor)
    }

    /// D90: the lowered re-arm forward (post-revert variant -> open), bound beside the forward.
    fn duke_rearm() -> String {
        format!("{DUKE_ACTION}/rearm")
    }

    /// `the_duke_of_the_depths` lowered by task A's §9 lowering, linked, with its anchor and
    /// destination markers injected. Returns the content and the dedicated inverse.
    fn duke_content(
        encounter: &str,
    ) -> TestResult<(CanonicalReferencePlayableContent, TransitionKey)> {
        let mut source = source_with(
            vec![
                state(SEALED_ITEM, LocalObjectCollisionPresence::Absent)?,
                state(OPEN_ITEM, LocalObjectCollisionPresence::Absent)?,
            ],
            vec![transition(OPEN_TELEPORTER, SEALED_ITEM, OPEN_ITEM)?],
            "lock:timed-revert-r1",
        )?;
        let anchors = BTreeMap::from([("exit_teleporter".to_owned(), object_definition()?)]);
        let lowered = lower_map_item_transforms(encounter, &source, &anchors)?;
        lowered.apply_to_source(&mut source)?;
        let mut content = link_reference_playable(source)?;
        let anchor = duke_anchor()?;
        let tables = lowered
            .placements
            .get(&anchor)
            .ok_or(fixture("lowered anchor tables"))?;
        let mut placement = object_at(&content, &anchor, SEALED_ITEM, 100)?;
        placement.local_object_state_attributes = tables.state_attributes.clone();
        placement.local_object_revert_after_ms = tables.revert_after_ms.clone();
        placement.local_object_event_transitions = tables.event_transitions.clone();
        let mut placements = vec![placement];
        for (x, marker) in (200..).zip(&lowered.destination_markers) {
            placements.push(placement_at(
                &content,
                marker.clone(),
                marker_definition()?,
                x,
            )?);
        }
        content.placements = placements;
        let revert = TransitionKey::new(&format!("{DUKE_ACTION}/revert"))?;
        Ok((content, revert))
    }

    /// Synthetic walls: `sealed` (blocking) --CRACK--> `cracked` --MEND--> `sealed`, with the
    /// given `(transition, action, ms)` revert durations on every placement. D91: when any
    /// duration is given, CRACK and MEND are both event-owned by `WALL_OWNER`; untimed walls keep
    /// both `PlayerUse`.
    fn wall_content(
        timed: &[(&str, &str, u64)],
        placements: &[(&str, i32)],
        lock: &str,
    ) -> TestResult<CanonicalReferencePlayableContent> {
        let mut content = link_reference_playable(source_with(
            vec![
                state(WALL_SEALED, LocalObjectCollisionPresence::Present)?,
                state(WALL_CRACKED, LocalObjectCollisionPresence::Absent)?,
            ],
            vec![
                transition(CRACK, WALL_SEALED, WALL_CRACKED)?,
                transition(MEND, WALL_CRACKED, WALL_SEALED)?,
            ],
            lock,
        )?)?;
        let mut injected = Vec::new();
        for (key, x) in placements {
            let mut placement = object_at(&content, &PlacementKey::new(key)?, WALL_SEALED, *x)?;
            for (transition, action, ms) in timed {
                placement.local_object_revert_after_ms.insert(
                    (
                        TransitionKey::new(transition)?,
                        LoweredActionId::new(action)?,
                    ),
                    *ms,
                );
            }
            if !timed.is_empty() {
                for transition in [CRACK, MEND] {
                    placement.local_object_event_transitions.insert(
                        TransitionKey::new(transition)?,
                        TransitionEventOwner::new(WALL_OWNER)?,
                    );
                }
            }
            injected.push(placement);
        }
        content.placements = injected;
        Ok(content)
    }

    fn bind_at(
        content: &CanonicalReferencePlayableContent,
        placement: &PlacementKey,
        transitions: &[&str],
        incarnation: u64,
    ) -> Result<LocalObjectRuntime, WorldRuntimeError> {
        let fence = ScopeContentGenerationFence::for_test(
            scope()?,
            generation(1)?,
            ReferenceContentGeneration::from_content(content)?,
        );
        let keys = transitions
            .iter()
            .copied()
            .map(TransitionKey::new)
            .collect::<Result<Vec<_>, _>>()?;
        LocalObjectRuntime::bind(
            content,
            &fence,
            scope()?,
            generation(1)?,
            placement,
            incarnation,
            &keys,
        )
    }

    fn walls(
        content: &CanonicalReferencePlayableContent,
        keys: &[&str],
    ) -> TestResult<BTreeMap<PlacementKey, LocalObjectRuntime>> {
        let mut runtimes = BTreeMap::new();
        for key in keys {
            let key = PlacementKey::new(key)?;
            runtimes.insert(key.clone(), bind_at(content, &key, &[CRACK, MEND], 1)?);
        }
        Ok(runtimes)
    }

    fn driver(
        clock: &ManualClock,
        limits: RevertDriverLimits,
    ) -> TestResult<ScopeRevertDriver<TestIssuer>> {
        Ok(ScopeRevertDriver::new(
            scope()?,
            TestIssuer::new(generation(1)?),
            Arc::new(clock.clone()),
            limits,
        ))
    }

    /// A scope operation executed by the fixture event that owns `transition`: the timed
    /// wall's owner for CRACK/MEND, the duke encounter for every other edge.
    fn operation(
        runtime: &LocalObjectRuntime,
        transition: &str,
        expected_revision: u64,
    ) -> TestResult<ScopeLocalObjectOperation> {
        let owner = if transition == CRACK || transition == MEND {
            WALL_OWNER
        } else {
            DUKE_KEY
        };
        operation_owned(
            runtime,
            transition,
            expected_revision,
            &TransitionEventOwner::new(owner)?,
        )
    }

    fn operation_owned(
        runtime: &LocalObjectRuntime,
        transition: &str,
        expected_revision: u64,
        owner: &TransitionEventOwner,
    ) -> TestResult<ScopeLocalObjectOperation> {
        Ok(ScopeLocalObjectOperation::new(
            runtime.placement_key().clone(),
            runtime.incarnation(),
            runtime.content_generation().clone(),
            LocalObjectOperation::new(TransitionKey::new(transition)?),
            expected_revision,
            owner.clone(),
        ))
    }

    fn forward(
        driver: &mut ScopeRevertDriver<TestIssuer>,
        runtimes: &mut BTreeMap<PlacementKey, LocalObjectRuntime>,
        placement: &str,
        occurrence: &str,
        action: &str,
        transition: &str,
    ) -> TestResult<ForwardOutcome> {
        let runtime = runtimes
            .get_mut(&PlacementKey::new(placement)?)
            .ok_or(fixture("runtime"))?;
        let operation = operation(runtime, transition, runtime.revision())?;
        Ok(driver.apply_forward(
            runtime,
            &forward_child(occurrence, None)?,
            &revisions()?,
            &LoweredActionId::new(action)?,
            &operation,
            &BTreeSet::new(),
        )?)
    }

    fn revisions() -> Result<SemanticRevisionContext, InteractionError> {
        SemanticRevisionContext::new("content:timed-revert-r1", "ruleset:r1", "sim:v1")
    }

    /// The forward operation's own GAME-INTERACTION child (an encounter action under its trigger
    /// root), as the encounter owner would supply it.
    fn forward_child(
        root: &str,
        ordinal: Option<u16>,
    ) -> Result<ChildOccurrenceRef, InteractionError> {
        ChildOccurrenceRef::for_root(
            &RootSourceOccurrenceRef::new(root)?,
            "encounter:map_item",
            "encounter:anchor",
            "transform",
            ordinal,
            &revisions()?,
        )
    }

    fn scheduled(outcome: ForwardOutcome) -> Result<ChildOccurrenceRef, WorldRuntimeError> {
        outcome.scheduled.ok_or(fixture("no revert scheduled"))
    }

    /// An untimed scope operation applied directly (another input in the meantime).
    fn apply_plain(runtime: &mut LocalObjectRuntime, transition: &str) -> TestResult<String> {
        let operation = operation(runtime, transition, runtime.revision())?;
        let staged = runtime.apply_scope_operation(
            scope()?,
            generation(1)?,
            &operation,
            &BTreeSet::new(),
            None,
            |_| Ok::<(), Infallible>(()),
        )?;
        let outcome = match staged {
            Ok(outcome) => outcome,
            Err(never) => match never {},
        };
        Ok(outcome.disposition().to_owned())
    }

    fn state_of(
        runtimes: &BTreeMap<PlacementKey, LocalObjectRuntime>,
        placement: &str,
    ) -> TestResult<(String, u64)> {
        let runtime = runtimes
            .get(&PlacementKey::new(placement)?)
            .ok_or(fixture("runtime"))?;
        Ok((runtime.state_key().as_str().to_owned(), runtime.revision()))
    }

    fn at(state: &str, revision: u64) -> (String, u64) {
        (state.to_owned(), revision)
    }

    fn destination_of(runtime: &LocalObjectRuntime) -> Option<&str> {
        runtime
            .attributes()
            .and_then(|attributes| attributes.destination.as_ref())
            .map(PlacementKey::as_str)
    }

    fn disposition(outcome: &RevertOutcome) -> Option<&str> {
        match outcome {
            RevertOutcome::Prepared(outcome) => Some(outcome.disposition()),
            RevertOutcome::Fenced(_) => None,
        }
    }

    fn millis(ms: u64) -> Duration {
        Duration::from_millis(ms)
    }

    #[test]
    fn duke_teleporter_reverts_to_the_warzone_exit_after_its_authored_duration() -> TestResult {
        let (content, revert) = duke_content(DUKE)?;
        let anchor = duke_anchor()?;
        let mut runtimes = BTreeMap::from([(
            anchor.clone(),
            bind_at(
                &content,
                &anchor,
                &[OPEN_TELEPORTER, &duke_rearm(), revert.as_str()],
                1,
            )?,
        )]);
        let clock = ManualClock::new(Moment::ZERO);
        let mut driver = driver(&clock, RevertDriverLimits::registered())?;

        // Natural source state: no destination before the forward operation.
        assert_eq!(state_of(&runtimes, anchor.as_str())?, at(SEALED_ITEM, 0));
        let runtime = runtimes.get(&anchor).ok_or(fixture("runtime"))?;
        assert_eq!(destination_of(runtime), None);

        let forward = forward(
            &mut driver,
            &mut runtimes,
            anchor.as_str(),
            DUKE_DEATH,
            DUKE_ACTION,
            OPEN_TELEPORTER,
        )?;
        assert_eq!(forward.outcome.disposition(), "COMMITTED");
        let id = scheduled(forward)?;
        let runtime = runtimes.get(&anchor).ok_or(fixture("runtime"))?;
        assert_eq!(runtime.state_key().as_str(), OPEN_ITEM);
        assert_eq!(
            destination_of(runtime),
            Some(duke_destination("reward_destination")?.as_str())
        );
        let Some(RevertLifecycle::Pending(pending)) = driver.lifecycle(&id) else {
            return Err("revert record is not PENDING".into());
        };
        assert_eq!(pending.inverse(), &revert);
        assert_eq!(pending.expected_state().as_str(), OPEN_ITEM);
        assert_eq!(pending.expected_revision(), 1);

        clock.advance(millis(1_199_999))?;
        assert!(
            driver
                .wake(&mut runtimes, &BTreeSet::new())?
                .fired
                .is_empty()
        );
        clock.advance(millis(1))?;
        let report = driver.wake(&mut runtimes, &BTreeSet::new())?;
        let [(fired, outcome)] = report.fired.as_slice() else {
            return Err("expected exactly one fired revert".into());
        };
        assert_eq!(fired, &id);
        assert!(outcome.committed());

        // Post-revert variant: same rendered item, own state key, destination = warzone exit.
        let post_revert = format!("{DUKE_ACTION}/post-revert");
        assert_eq!(state_of(&runtimes, anchor.as_str())?, at(&post_revert, 2));
        let runtime = runtimes.get(&anchor).ok_or(fixture("runtime"))?;
        assert_eq!(
            destination_of(runtime),
            Some(duke_destination("warzone_exit")?.as_str())
        );
        assert_eq!(
            driver.lifecycle(&id),
            Some(&RevertLifecycle::Terminal(outcome.clone()))
        );

        // A duplicate presentation returns the TERMINAL outcome: no ordinal, no mutation.
        let minted = driver.issuer().minted;
        assert_eq!(
            driver.present(&id, &mut runtimes, &BTreeSet::new())?,
            RevertPresentation::Terminal(outcome.clone())
        );
        assert_eq!(driver.issuer().minted, minted);
        assert_eq!(state_of(&runtimes, anchor.as_str())?, at(&post_revert, 2));
        Ok(())
    }

    /// One duke death as the encounter owner delivers it: select the timed forward from the
    /// teleporter's current state (D90) and, if there is one, commit it through the driver.
    fn duke_kill(
        driver: &mut ScopeRevertDriver<TestIssuer>,
        runtimes: &mut BTreeMap<PlacementKey, LocalObjectRuntime>,
        content: &CanonicalReferencePlayableContent,
        death: &str,
    ) -> TestResult<Option<ForwardOutcome>> {
        let anchor = duke_anchor()?;
        let runtime = runtimes.get(&anchor).ok_or(fixture("runtime"))?;
        let action = LoweredActionId::new(DUKE_ACTION)?;
        let Some(transition) = select_timed_forward(runtime, &action, &content.transitions)? else {
            return Ok(None);
        };
        Ok(Some(forward(
            driver,
            runtimes,
            anchor.as_str(),
            death,
            DUKE_ACTION,
            transition.as_str(),
        )?))
    }

    #[test]
    fn duke_teleporter_re_arms_on_every_kill_and_a_kill_while_open_is_a_no_op() -> TestResult {
        // Owner decision D90: after the timed revert lands on the post-revert variant, the next
        // kill reopens the teleporter through the lowered re-arm forward.
        let (content, revert) = duke_content(DUKE)?;
        let anchor = duke_anchor()?;
        let rearm = duke_rearm();
        let mut runtimes = BTreeMap::from([(
            anchor.clone(),
            bind_at(
                &content,
                &anchor,
                &[OPEN_TELEPORTER, &rearm, revert.as_str()],
                1,
            )?,
        )]);
        let clock = ManualClock::new(Moment::ZERO);
        let mut driver = driver(&clock, RevertDriverLimits::registered())?;
        let post_revert = format!("{DUKE_ACTION}/post-revert");
        let reward = duke_destination("reward_destination")?;
        let warzone = duke_destination("warzone_exit")?;
        let destination = |runtimes: &BTreeMap<PlacementKey, LocalObjectRuntime>| {
            runtimes
                .get(&anchor)
                .and_then(destination_of)
                .map(str::to_owned)
        };

        // Kill 1 from the natural state: A -> B, reward destination, one PENDING record.
        let kill_1 = duke_kill(&mut driver, &mut runtimes, &content, DUKE_DEATH)?
            .ok_or(fixture("kill 1 selected no forward"))?;
        assert_eq!(kill_1.outcome.disposition(), "COMMITTED");
        let first = scheduled(kill_1)?;
        assert_eq!(state_of(&runtimes, anchor.as_str())?, at(OPEN_ITEM, 1));
        assert_eq!(destination(&runtimes), Some(reward.as_str().to_owned()));

        // A kill while open selects no forward: no ordinal, no record, no mutation.
        let minted = driver.issuer().minted;
        assert!(
            duke_kill(
                &mut driver,
                &mut runtimes,
                &content,
                "canary:occurrence/the_duke_of_the_depths/death/open-1"
            )?
            .is_none()
        );
        assert_eq!(driver.issuer().minted, minted);
        assert_eq!(driver.record_count(), 1);
        assert_eq!(state_of(&runtimes, anchor.as_str())?, at(OPEN_ITEM, 1));

        // 1,200,000 ms later the revert B -> C lands on the warzone exit.
        clock.advance(millis(1_200_000))?;
        let report = driver.wake(&mut runtimes, &BTreeSet::new())?;
        let [(fired, first_outcome)] = report.fired.as_slice() else {
            return Err("expected the first revert to fire".into());
        };
        assert_eq!(fired, &first);
        assert!(first_outcome.committed());
        let first_outcome = first_outcome.clone();
        assert_eq!(state_of(&runtimes, anchor.as_str())?, at(&post_revert, 2));
        assert_eq!(destination(&runtimes), Some(warzone.as_str().to_owned()));

        // Kill 2 from the post-revert variant: the re-arm forward C -> B reopens the teleporter
        // to the reward destination and schedules a second, distinct revert of the same inverse.
        let runtime = runtimes.get(&anchor).ok_or(fixture("runtime"))?;
        let selected = select_timed_forward(
            runtime,
            &LoweredActionId::new(DUKE_ACTION)?,
            &content.transitions,
        )?;
        assert_eq!(
            selected.as_ref().map(TransitionKey::as_str),
            Some(rearm.as_str())
        );
        let kill_2 = duke_kill(
            &mut driver,
            &mut runtimes,
            &content,
            "canary:occurrence/the_duke_of_the_depths/death/2",
        )?
        .ok_or(fixture("kill 2 selected no forward"))?;
        assert_eq!(kill_2.outcome.disposition(), "COMMITTED");
        let second = scheduled(kill_2)?;
        assert_ne!(second, first);
        assert_eq!(state_of(&runtimes, anchor.as_str())?, at(OPEN_ITEM, 3));
        assert_eq!(destination(&runtimes), Some(reward.as_str().to_owned()));
        let Some(RevertLifecycle::Pending(pending)) = driver.lifecycle(&second) else {
            return Err("second revert record is not PENDING".into());
        };
        assert_eq!(pending.inverse(), &revert);
        assert_eq!(pending.expected_state().as_str(), OPEN_ITEM);
        assert_eq!(pending.expected_revision(), 3);

        // A kill while open again is a no-op, and the first revert still stands.
        let minted = driver.issuer().minted;
        assert!(
            duke_kill(
                &mut driver,
                &mut runtimes,
                &content,
                "canary:occurrence/the_duke_of_the_depths/death/open-2"
            )?
            .is_none()
        );
        assert_eq!(driver.issuer().minted, minted);
        assert_eq!(driver.record_count(), 2);
        assert_eq!(state_of(&runtimes, anchor.as_str())?, at(OPEN_ITEM, 3));
        assert_eq!(
            driver.lifecycle(&first),
            Some(&RevertLifecycle::Terminal(first_outcome.clone()))
        );

        // 1,200,000 ms after kill 2 the teleporter reverts to the warzone exit again.
        clock.advance(millis(1_199_999))?;
        assert!(
            driver
                .wake(&mut runtimes, &BTreeSet::new())?
                .fired
                .is_empty()
        );
        clock.advance(millis(1))?;
        let report = driver.wake(&mut runtimes, &BTreeSet::new())?;
        let [(fired, second_outcome)] = report.fired.as_slice() else {
            return Err("expected the second revert to fire".into());
        };
        assert_eq!(fired, &second);
        assert!(second_outcome.committed());
        assert_eq!(state_of(&runtimes, anchor.as_str())?, at(&post_revert, 4));
        assert_eq!(destination(&runtimes), Some(warzone.as_str().to_owned()));

        // Two distinct TERMINAL lifecycle records, one per kill.
        assert_eq!(driver.record_count(), 2);
        assert_eq!(
            driver.lifecycle(&first),
            Some(&RevertLifecycle::Terminal(first_outcome))
        );
        assert_eq!(
            driver.lifecycle(&second),
            Some(&RevertLifecycle::Terminal(second_outcome.clone()))
        );
        assert_eq!(driver.next_deadline(), None);
        Ok(())
    }

    #[test]
    fn a_due_record_fires_once_mints_one_ordinal_and_restores_the_literal_source() -> TestResult {
        let content = wall_content(
            &[(CRACK, WALL_ACTION, 300_000)],
            &[(WALL_A, 100)],
            "lock:r1",
        )?;
        let mut runtimes = walls(&content, &[WALL_A])?;
        let clock = ManualClock::new(Moment::ZERO);
        let mut driver = driver(&clock, RevertDriverLimits::registered())?;
        let id = scheduled(forward(
            &mut driver,
            &mut runtimes,
            WALL_A,
            "oteryn:occurrence/wall/1",
            WALL_ACTION,
            CRACK,
        )?)?;
        assert_eq!(driver.issuer().minted, 1);
        // Not yet due: presenting it early runs nothing.
        assert_eq!(
            driver.present(&id, &mut runtimes, &BTreeSet::new())?,
            RevertPresentation::NotDue
        );
        clock.advance(millis(300_000))?;
        let first = driver.present(&id, &mut runtimes, &BTreeSet::new())?;
        let second = driver.present(&id, &mut runtimes, &BTreeSet::new())?;
        assert_eq!(first, second);
        assert!(matches!(&first, RevertPresentation::Terminal(outcome) if outcome.committed()));
        assert_eq!(driver.issuer().minted, 2);
        assert_eq!(state_of(&runtimes, WALL_A)?, at(WALL_SEALED, 2));
        assert!(
            driver
                .wake(&mut runtimes, &BTreeSet::new())?
                .fired
                .is_empty()
        );
        Ok(())
    }

    #[test]
    fn an_in_flight_duplicate_converges_without_an_ordinal_or_prepare() -> TestResult {
        let content = wall_content(&[(CRACK, WALL_ACTION, 1_000)], &[(WALL_A, 100)], "lock:r1")?;
        let mut runtimes = walls(&content, &[WALL_A])?;
        let clock = ManualClock::new(Moment::ZERO);
        let mut driver = driver(&clock, RevertDriverLimits::registered())?;
        let id = scheduled(forward(
            &mut driver,
            &mut runtimes,
            WALL_A,
            "oteryn:occurrence/wall/1",
            WALL_ACTION,
            CRACK,
        )?)?;
        clock.advance(millis(1_000))?;
        driver
            .force_in_flight_for_test(&id)
            .ok_or(fixture("force in flight"))?;
        // Even with the scope generation moved, an IN_FLIGHT record never re-runs fences.
        driver.issuer_mut_for_test().generation = generation(2)?;
        assert_eq!(
            driver.present(&id, &mut runtimes, &BTreeSet::new())?,
            RevertPresentation::InFlight
        );
        assert_eq!(driver.issuer().minted, 1);
        assert_eq!(state_of(&runtimes, WALL_A)?, at(WALL_CRACKED, 1));
        assert!(
            driver
                .wake(&mut runtimes, &BTreeSet::new())?
                .fired
                .is_empty()
        );
        Ok(())
    }

    #[derive(Clone, Copy)]
    enum FenceChange {
        ScopeGeneration,
        Incarnation,
        ContentGeneration,
    }

    #[test]
    fn pending_fences_reject_without_prepare_and_duplicates_return_the_first_outcome() -> TestResult
    {
        let timed = [(CRACK, WALL_ACTION, 1_000)];
        let content = wall_content(&timed, &[(WALL_A, 100)], "lock:r1")?;
        let other_generation = wall_content(&timed, &[(WALL_A, 100)], "lock:r2")?;
        let wall = PlacementKey::new(WALL_A)?;
        for (change, fence) in [
            (FenceChange::ScopeGeneration, RevertFence::ScopeGeneration),
            (FenceChange::Incarnation, RevertFence::Incarnation),
            (
                FenceChange::ContentGeneration,
                RevertFence::ContentGeneration,
            ),
        ] {
            let mut runtimes = walls(&content, &[WALL_A])?;
            let clock = ManualClock::new(Moment::ZERO);
            let mut driver = driver(&clock, RevertDriverLimits::registered())?;
            let id = scheduled(forward(
                &mut driver,
                &mut runtimes,
                WALL_A,
                "oteryn:occurrence/wall/1",
                WALL_ACTION,
                CRACK,
            )?)?;
            match change {
                FenceChange::ScopeGeneration => {
                    driver.issuer_mut_for_test().generation = generation(2)?;
                }
                FenceChange::Incarnation => {
                    runtimes.insert(wall.clone(), bind_at(&content, &wall, &[CRACK, MEND], 2)?);
                }
                FenceChange::ContentGeneration => {
                    runtimes.insert(
                        wall.clone(),
                        bind_at(&other_generation, &wall, &[CRACK, MEND], 1)?,
                    );
                }
            }
            let before = state_of(&runtimes, WALL_A)?;
            clock.advance(millis(1_000))?;
            let report = driver.wake(&mut runtimes, &BTreeSet::new())?;
            assert_eq!(
                report.fired,
                vec![(id.clone(), RevertOutcome::Fenced(fence))]
            );
            // No ordinal minted and no prepare/mutation for a fenced record.
            assert_eq!(driver.issuer().minted, 1);
            assert_eq!(state_of(&runtimes, WALL_A)?, before);
            assert_eq!(
                driver.present(&id, &mut runtimes, &BTreeSet::new())?,
                RevertPresentation::Terminal(RevertOutcome::Fenced(fence))
            );
        }
        Ok(())
    }

    #[test]
    fn a_terminal_duplicate_returns_the_first_outcome_after_the_target_is_replaced() -> TestResult {
        let content = wall_content(&[(CRACK, WALL_ACTION, 1_000)], &[(WALL_A, 100)], "lock:r1")?;
        let mut runtimes = walls(&content, &[WALL_A])?;
        let clock = ManualClock::new(Moment::ZERO);
        let mut driver = driver(&clock, RevertDriverLimits::registered())?;
        let id = scheduled(forward(
            &mut driver,
            &mut runtimes,
            WALL_A,
            "oteryn:occurrence/wall/1",
            WALL_ACTION,
            CRACK,
        )?)?;
        clock.advance(millis(1_000))?;
        let report = driver.wake(&mut runtimes, &BTreeSet::new())?;
        let [(_, first)] = report.fired.as_slice() else {
            return Err("expected one fired revert".into());
        };
        assert!(first.committed());
        let wall = PlacementKey::new(WALL_A)?;
        runtimes.insert(wall.clone(), bind_at(&content, &wall, &[CRACK, MEND], 2)?);
        assert_eq!(
            driver.present(&id, &mut runtimes, &BTreeSet::new())?,
            RevertPresentation::Terminal(first.clone())
        );
        assert_eq!(driver.issuer().minted, 2);
        assert_eq!(state_of(&runtimes, WALL_A)?, at(WALL_SEALED, 0));
        Ok(())
    }

    #[test]
    fn distinct_occurrences_keep_distinct_records_and_an_intervening_change_is_stale() -> TestResult
    {
        let content = wall_content(&[(CRACK, WALL_ACTION, 1_000)], &[(WALL_A, 100)], "lock:r1")?;
        let mut runtimes = walls(&content, &[WALL_A])?;
        let clock = ManualClock::new(Moment::ZERO);
        let mut driver = driver(&clock, RevertDriverLimits::registered())?;
        let first = scheduled(forward(
            &mut driver,
            &mut runtimes,
            WALL_A,
            "oteryn:occurrence/wall/1",
            WALL_ACTION,
            CRACK,
        )?)?;
        // Something else mends the wall, then the same transform fires again.
        let wall = PlacementKey::new(WALL_A)?;
        let runtime = runtimes.get_mut(&wall).ok_or(fixture("runtime"))?;
        assert_eq!(apply_plain(runtime, MEND)?, "COMMITTED");
        clock.advance(millis(10))?;
        let second = scheduled(forward(
            &mut driver,
            &mut runtimes,
            WALL_A,
            "oteryn:occurrence/wall/2",
            WALL_ACTION,
            CRACK,
        )?)?;
        assert_ne!(first, second);
        assert_eq!(driver.record_count(), 2);

        clock.advance(millis(1_000))?;
        let report = driver.wake(&mut runtimes, &BTreeSet::new())?;
        let [(first_id, first_outcome), (second_id, second_outcome)] = report.fired.as_slice()
        else {
            return Err("expected two fired reverts".into());
        };
        assert_eq!((first_id, second_id), (&first, &second));
        // The first record meets a changed object: STALE_STATE through `prepare`, no overwrite.
        assert_eq!(disposition(first_outcome), Some("STALE_STATE"));
        assert!(second_outcome.committed());
        assert_eq!(state_of(&runtimes, WALL_A)?, at(WALL_SEALED, 4));
        Ok(())
    }

    #[test]
    fn distinct_forward_children_derive_distinct_canonical_revert_children() -> TestResult {
        // #1144 P1 4125535254: the record key is the canonical nested `ChildOccurrenceRef`, so
        // forward children differing in any discriminator (here only the ordinal) never share a
        // record.
        let content = wall_content(
            &[(CRACK, WALL_ACTION, 1_000)],
            &[(WALL_A, 100), (WALL_B, 300)],
            "lock:r1",
        )?;
        let mut runtimes = walls(&content, &[WALL_A, WALL_B])?;
        let clock = ManualClock::new(Moment::ZERO);
        let mut driver = driver(&clock, RevertDriverLimits::registered())?;
        let action = LoweredActionId::new(WALL_ACTION)?;
        let mut ids = Vec::new();
        for (wall, ordinal) in [(WALL_A, Some(0)), (WALL_B, Some(1))] {
            let key = PlacementKey::new(wall)?;
            let runtime = runtimes.get_mut(&key).ok_or(fixture("runtime"))?;
            let crack = operation(runtime, CRACK, 0)?;
            let forward = forward_child("oteryn:occurrence/boss-death", ordinal)?;
            let outcome = driver.apply_forward(
                runtime,
                &forward,
                &revisions()?,
                &action,
                &crack,
                &BTreeSet::new(),
            )?;
            let id = scheduled(outcome)?;
            // Derived through the canonical constructor: nested one level below the forward
            // child, deterministic, with the bound inverse as its edge.
            assert_eq!(id.ancestry_depth(), 2);
            assert_eq!(
                id,
                revert_child_occurrence(
                    &forward,
                    &action,
                    &key,
                    &TransitionKey::new(MEND)?,
                    &revisions()?
                )?
            );
            ids.push(id);
        }
        assert_ne!(ids.first(), ids.get(1));
        assert_eq!(driver.record_count(), 2);
        Ok(())
    }

    #[test]
    fn use_and_session_commands_cannot_commit_an_event_owned_transition() -> TestResult {
        // #1144 P1 4125535249, subsumed by owner decision D91: a timed transition is event-owned,
        // and USE and `apply` refuse every event-owned edge, exactly like an unbound one.
        let timed = wall_content(&[(CRACK, WALL_ACTION, 1_000)], &[(WALL_A, 100)], "lock:r1")?;
        let mut runtimes = walls(&timed, &[WALL_A])?;
        let wall = PlacementKey::new(WALL_A)?;
        let runtime = runtimes.get_mut(&wall).ok_or(fixture("runtime"))?;
        assert_eq!(
            runtime.attempt_use(0, &BTreeSet::new())?,
            LocalObjectUseOutcome::NothingToUse
        );
        let mut ingress = CommandIngress::new();
        assert!(matches!(
            apply_session(runtime, &mut ingress, 1, CRACK)?,
            Err(WorldRuntimeError::InvalidBinding(
                "command names a transition this local-object runtime does not bind"
            ))
        ));
        assert_eq!(ingress.outstanding(), 0);
        assert_eq!(state_of(&runtimes, WALL_A)?, at(WALL_SEALED, 0));

        // D91 replaces carry-over (b): the untimed but event-owned inverse is neither
        // USE-selectable nor session-invocable once the forward has committed.
        let clock = ManualClock::new(Moment::ZERO);
        let mut driver = driver(&clock, RevertDriverLimits::registered())?;
        scheduled(forward(
            &mut driver,
            &mut runtimes,
            WALL_A,
            "oteryn:occurrence/wall/1",
            WALL_ACTION,
            CRACK,
        )?)?;
        let runtime = runtimes.get_mut(&wall).ok_or(fixture("runtime"))?;
        assert_eq!(
            runtime.attempt_use(1, &BTreeSet::new())?,
            LocalObjectUseOutcome::NothingToUse
        );
        assert!(matches!(
            apply_session(runtime, &mut ingress, 1, MEND)?,
            Err(WorldRuntimeError::InvalidBinding(
                "command names a transition this local-object runtime does not bind"
            ))
        ));
        assert_eq!(ingress.outstanding(), 0);
        assert_eq!(state_of(&runtimes, WALL_A)?, at(WALL_CRACKED, 1));

        // The duke teleporter cannot be opened by USE from its natural state.
        let (content, revert) = duke_content(DUKE)?;
        let anchor = duke_anchor()?;
        let mut duke = bind_at(
            &content,
            &anchor,
            &[OPEN_TELEPORTER, &duke_rearm(), revert.as_str()],
            1,
        )?;
        assert_eq!(
            duke.attempt_use(0, &BTreeSet::new())?,
            LocalObjectUseOutcome::NothingToUse
        );
        assert_eq!(
            (duke.state_key().as_str(), duke.revision()),
            (SEALED_ITEM, 0)
        );
        Ok(())
    }

    #[test]
    fn untimed_use_and_session_commands_are_unchanged() -> TestResult {
        let untimed = wall_content(&[], &[(WALL_A, 100), (WALL_B, 300)], "lock:r1")?;
        let mut runtimes = walls(&untimed, &[WALL_A, WALL_B])?;
        let runtime = runtimes
            .get_mut(&PlacementKey::new(WALL_A)?)
            .ok_or(fixture("runtime"))?;
        assert_eq!(
            runtime.attempt_use(0, &BTreeSet::new())?,
            LocalObjectUseOutcome::Committed {
                state: ProductionKey::new(WALL_CRACKED)?,
                revision: 1,
            }
        );
        let runtime = runtimes
            .get_mut(&PlacementKey::new(WALL_B)?)
            .ok_or(fixture("runtime"))?;
        let mut ingress = CommandIngress::new();
        assert_eq!(
            apply_session(runtime, &mut ingress, 1, CRACK)??,
            "COMMITTED"
        );
        assert_eq!(state_of(&runtimes, WALL_B)?, at(WALL_CRACKED, 1));
        Ok(())
    }

    #[test]
    fn a_scope_operation_without_the_scheduling_capability_refuses_a_timed_transition() -> TestResult
    {
        // #1144 P1 4125881398: only the driver holds the capability, so no other crate caller
        // can commit a timed transition without its lifecycle record.
        let content = wall_content(&[(CRACK, WALL_ACTION, 1_000)], &[(WALL_A, 100)], "lock:r1")?;
        let mut runtimes = walls(&content, &[WALL_A])?;
        let wall = PlacementKey::new(WALL_A)?;
        let runtime = runtimes.get_mut(&wall).ok_or(fixture("runtime"))?;
        let refused = apply_plain(runtime, CRACK)
            .err()
            .and_then(|error| error.downcast::<WorldRuntimeError>().ok());
        assert!(matches!(
            refused.as_deref(),
            Some(WorldRuntimeError::InvalidBinding(
                "timed transition commits only through the revert scheduler"
            ))
        ));
        assert_eq!(state_of(&runtimes, WALL_A)?, at(WALL_SEALED, 0));

        // The driver commits it (with its record); an untimed operation needs no capability.
        let clock = ManualClock::new(Moment::ZERO);
        let mut driver = driver(&clock, RevertDriverLimits::registered())?;
        scheduled(forward(
            &mut driver,
            &mut runtimes,
            WALL_A,
            "oteryn:occurrence/wall/1",
            WALL_ACTION,
            CRACK,
        )?)?;
        let runtime = runtimes.get_mut(&wall).ok_or(fixture("runtime"))?;
        assert_eq!(apply_plain(runtime, MEND)?, "COMMITTED");
        assert_eq!(state_of(&runtimes, WALL_A)?, at(WALL_SEALED, 2));
        Ok(())
    }

    fn nested_forward(depth: usize) -> Result<ChildOccurrenceRef, InteractionError> {
        let mut occurrence = forward_child("oteryn:occurrence/nested", None)?;
        for _ in 1..depth {
            occurrence = ChildOccurrenceRef::for_child(
                &occurrence,
                "encounter:cascade",
                "encounter:anchor",
                "transform",
                None,
                &revisions()?,
            )?;
        }
        Ok(occurrence)
    }

    fn forward_with(
        driver: &mut ScopeRevertDriver<TestIssuer>,
        runtimes: &mut BTreeMap<PlacementKey, LocalObjectRuntime>,
        occurrence: &ChildOccurrenceRef,
        revisions: &SemanticRevisionContext,
    ) -> TestResult<Result<ForwardOutcome, RevertError>> {
        let runtime = runtimes
            .get_mut(&PlacementKey::new(WALL_A)?)
            .ok_or(fixture("runtime"))?;
        let crack = operation(runtime, CRACK, runtime.revision())?;
        Ok(driver.apply_forward(
            runtime,
            occurrence,
            revisions,
            &LoweredActionId::new(WALL_ACTION)?,
            &crack,
            &BTreeSet::new(),
        ))
    }

    #[test]
    fn over_deep_or_over_large_occurrences_are_refused_before_commit() -> TestResult {
        // #1144 P2 4125881422: WOBJ-RL-06/07 bound what one record retains.
        let content = wall_content(&[(CRACK, WALL_ACTION, 1_000)], &[(WALL_A, 100)], "lock:r1")?;
        let clock = ManualClock::new(Moment::ZERO);
        let long = format!(
            "encounter:{}",
            "x".repeat(MAX_REVERT_OCCURRENCE_RENDERED_BYTES)
        );
        let too_large_forward = ChildOccurrenceRef::for_root(
            &RootSourceOccurrenceRef::new("oteryn:occurrence/large")?,
            "encounter:map_item",
            &long,
            "transform",
            None,
            &revisions()?,
        )?;
        let too_large_revisions = SemanticRevisionContext::new(&long, "ruleset:r1", "sim:v1")?;
        let cases = [
            (
                nested_forward(MAX_REVERT_OCCURRENCE_DEPTH)?,
                revisions()?,
                "OccurrenceTooDeep",
            ),
            (too_large_forward, revisions()?, "OccurrenceTooLarge"),
            (
                forward_child("oteryn:occurrence/wall/1", None)?,
                too_large_revisions,
                "OccurrenceTooLarge",
            ),
        ];
        for (occurrence, context, expected) in cases {
            let mut runtimes = walls(&content, &[WALL_A])?;
            let mut driver = driver(&clock, RevertDriverLimits::registered())?;
            let refused = forward_with(&mut driver, &mut runtimes, &occurrence, &context)?;
            assert!(
                matches!(
                    (&refused, expected),
                    (Err(RevertError::OccurrenceTooDeep), "OccurrenceTooDeep")
                        | (Err(RevertError::OccurrenceTooLarge), "OccurrenceTooLarge")
                ),
                "{expected}: {refused:?}"
            );
            // Nothing minted, committed or recorded.
            assert_eq!(driver.issuer().minted, 0);
            assert_eq!(driver.record_count(), 0);
            assert_eq!(state_of(&runtimes, WALL_A)?, at(WALL_SEALED, 0));
        }

        // The deepest admitted forward (one below the revert bound) still schedules.
        let mut runtimes = walls(&content, &[WALL_A])?;
        let mut driver = driver(&clock, RevertDriverLimits::registered())?;
        let deepest = nested_forward(MAX_REVERT_OCCURRENCE_DEPTH - 1)?;
        let admitted = forward_with(&mut driver, &mut runtimes, &deepest, &revisions()?)??;
        assert_eq!(
            scheduled(admitted)?.ancestry_depth(),
            MAX_REVERT_OCCURRENCE_DEPTH
        );
        Ok(())
    }

    #[test]
    fn use_on_the_open_duke_teleporter_cannot_select_its_revert_and_an_early_event_revert_is_stale()
    -> TestResult {
        // Owner decision D91 gap regression: the open teleporter's only edge from its current
        // state is the event-owned `/revert` inverse, so USE selects nothing and a session
        // command naming it is refused; nothing mutates. §7's one path for an intervening
        // change stays defensive: an early event-owned scope revert makes the timer reach
        // `prepare` and terminalize STALE_STATE; it never reverts a second time.
        let (content, revert) = duke_content(DUKE)?;
        let anchor = duke_anchor()?;
        let mut runtimes = BTreeMap::from([(
            anchor.clone(),
            bind_at(
                &content,
                &anchor,
                &[OPEN_TELEPORTER, &duke_rearm(), revert.as_str()],
                1,
            )?,
        )]);
        let clock = ManualClock::new(Moment::ZERO);
        let mut driver = driver(&clock, RevertDriverLimits::registered())?;
        let id = scheduled(forward(
            &mut driver,
            &mut runtimes,
            anchor.as_str(),
            DUKE_DEATH,
            DUKE_ACTION,
            OPEN_TELEPORTER,
        )?)?;
        let runtime = runtimes.get_mut(&anchor).ok_or(fixture("runtime"))?;
        let post_revert = format!("{DUKE_ACTION}/post-revert");
        assert_eq!(
            runtime.attempt_use(1, &BTreeSet::new())?,
            LocalObjectUseOutcome::NothingToUse
        );
        let mut ingress = CommandIngress::new();
        assert!(matches!(
            apply_session(runtime, &mut ingress, 1, revert.as_str())?,
            Err(WorldRuntimeError::InvalidBinding(
                "command names a transition this local-object runtime does not bind"
            ))
        ));
        assert_eq!(ingress.outstanding(), 0);
        assert_eq!(state_of(&runtimes, anchor.as_str())?, at(OPEN_ITEM, 1));

        let runtime = runtimes.get_mut(&anchor).ok_or(fixture("runtime"))?;
        assert_eq!(apply_plain(runtime, revert.as_str())?, "COMMITTED");
        assert_eq!(state_of(&runtimes, anchor.as_str())?, at(&post_revert, 2));
        clock.advance(millis(1_200_000))?;
        let report = driver.wake(&mut runtimes, &BTreeSet::new())?;
        let [(fired, outcome)] = report.fired.as_slice() else {
            return Err("expected one fired revert".into());
        };
        assert_eq!(fired, &id);
        assert_eq!(disposition(outcome), Some("STALE_STATE"));
        assert_eq!(state_of(&runtimes, anchor.as_str())?, at(&post_revert, 2));
        let runtime = runtimes.get(&anchor).ok_or(fixture("runtime"))?;
        assert_eq!(
            destination_of(runtime),
            Some(duke_destination("warzone_exit")?.as_str())
        );
        Ok(())
    }

    #[test]
    fn scope_operations_commit_only_their_own_owners_event_edges() -> TestResult {
        // Owner decision D91 (#1187 P1): the executing event names its owner; another owner's
        // forward or inverse is refused before any ordinal, record or mutation.
        let (content, revert) = duke_content(DUKE)?;
        let anchor = duke_anchor()?;
        let mut runtimes = BTreeMap::from([(
            anchor.clone(),
            bind_at(
                &content,
                &anchor,
                &[OPEN_TELEPORTER, &duke_rearm(), revert.as_str()],
                1,
            )?,
        )]);
        let clock = ManualClock::new(Moment::ZERO);
        let mut driver = driver(&clock, RevertDriverLimits::registered())?;
        let foreign = TransitionEventOwner::new("oteryn:encounter/another-boss")?;

        // A foreign-owner forward.
        let runtime = runtimes.get_mut(&anchor).ok_or(fixture("runtime"))?;
        let foreign_forward = operation_owned(runtime, OPEN_TELEPORTER, 0, &foreign)?;
        let refused = driver.apply_forward(
            runtime,
            &forward_child(DUKE_DEATH, None)?,
            &revisions()?,
            &LoweredActionId::new(DUKE_ACTION)?,
            &foreign_forward,
            &BTreeSet::new(),
        );
        assert!(matches!(
            refused,
            Err(RevertError::Runtime(WorldRuntimeError::EventOwnerMismatch))
        ));
        assert!(matches!(
            runtime.apply_scope_operation(
                scope()?,
                generation(1)?,
                &foreign_forward,
                &BTreeSet::new(),
                None,
                |_| Ok::<(), Infallible>(()),
            ),
            Err(WorldRuntimeError::EventOwnerMismatch)
        ));
        assert_eq!(driver.issuer().minted, 0);
        assert_eq!(driver.record_count(), 0);
        assert_eq!(state_of(&runtimes, anchor.as_str())?, at(SEALED_ITEM, 0));

        // The correct owner commits the forward and schedules its revert.
        let id = scheduled(forward(
            &mut driver,
            &mut runtimes,
            anchor.as_str(),
            DUKE_DEATH,
            DUKE_ACTION,
            OPEN_TELEPORTER,
        )?)?;
        assert_eq!(state_of(&runtimes, anchor.as_str())?, at(OPEN_ITEM, 1));

        // A foreign-owner inverse.
        let runtime = runtimes.get_mut(&anchor).ok_or(fixture("runtime"))?;
        let foreign_inverse = operation_owned(runtime, revert.as_str(), 1, &foreign)?;
        assert!(matches!(
            runtime.apply_scope_operation(
                scope()?,
                generation(1)?,
                &foreign_inverse,
                &BTreeSet::new(),
                None,
                |_| Ok::<(), Infallible>(()),
            ),
            Err(WorldRuntimeError::EventOwnerMismatch)
        ));
        assert_eq!(state_of(&runtimes, anchor.as_str())?, at(OPEN_ITEM, 1));

        // The driver fires the inverse on the forward's own owner, and it commits.
        clock.advance(millis(1_200_000))?;
        let report = driver.wake(&mut runtimes, &BTreeSet::new())?;
        let [(fired, outcome)] = report.fired.as_slice() else {
            return Err("expected one fired revert".into());
        };
        assert_eq!(fired, &id);
        assert_eq!(disposition(outcome), Some("COMMITTED"));
        let post_revert = format!("{DUKE_ACTION}/post-revert");
        assert_eq!(state_of(&runtimes, anchor.as_str())?, at(&post_revert, 2));
        Ok(())
    }

    #[test]
    fn a_full_store_fails_the_forward_before_commit_and_unchanged_forwards_reserve_nothing()
    -> TestResult {
        let content = wall_content(&[(CRACK, WALL_ACTION, 1_000)], &[(WALL_A, 100)], "lock:r1")?;
        let mut runtimes = walls(&content, &[WALL_A])?;
        let clock = ManualClock::new(Moment::ZERO);
        let mut driver = driver(&clock, RevertDriverLimits::new(1, 1)?)?;
        scheduled(forward(
            &mut driver,
            &mut runtimes,
            WALL_A,
            "oteryn:occurrence/wall/1",
            WALL_ACTION,
            CRACK,
        )?)?;
        let wall = PlacementKey::new(WALL_A)?;
        let runtime = runtimes.get_mut(&wall).ok_or(fixture("runtime"))?;
        assert_eq!(apply_plain(runtime, MEND)?, "COMMITTED");

        // Full store: the whole forward fails before commit.
        let occurrence = forward_child("oteryn:occurrence/wall/2", None)?;
        let action = LoweredActionId::new(WALL_ACTION)?;
        let runtime = runtimes.get_mut(&wall).ok_or(fixture("runtime"))?;
        let crack = operation(runtime, CRACK, runtime.revision())?;
        let full = driver.apply_forward(
            runtime,
            &occurrence,
            &revisions()?,
            &action,
            &crack,
            &BTreeSet::new(),
        );
        assert!(matches!(full, Err(RevertError::CapacityExceeded)));
        assert_eq!(state_of(&runtimes, WALL_A)?, at(WALL_SEALED, 2));
        assert_eq!(driver.record_count(), 1);

        // An unchanged forward never reaches the capacity check: STALE_STATE, not a failure.
        let runtime = runtimes.get_mut(&wall).ok_or(fixture("runtime"))?;
        let stale = operation(runtime, CRACK, 0)?;
        let result = driver.apply_forward(
            runtime,
            &forward_child("oteryn:occurrence/wall/3", None)?,
            &revisions()?,
            &action,
            &stale,
            &BTreeSet::new(),
        )?;
        assert_eq!(result.outcome.disposition(), "STALE_STATE");
        assert_eq!(result.scheduled, None);
        assert_eq!(driver.record_count(), 1);
        Ok(())
    }

    #[test]
    fn an_action_without_a_duration_schedules_nothing() -> TestResult {
        let content = wall_content(&[(CRACK, WALL_ACTION, 1_000)], &[(WALL_A, 100)], "lock:r1")?;
        let mut runtimes = walls(&content, &[WALL_A])?;
        let clock = ManualClock::new(Moment::ZERO);
        let mut driver = driver(&clock, RevertDriverLimits::registered())?;
        let outcome = forward(
            &mut driver,
            &mut runtimes,
            WALL_A,
            "oteryn:occurrence/wall/1",
            "oteryn:encounter/timed-wall/other/0",
            CRACK,
        )?;
        assert_eq!(outcome.outcome.disposition(), "COMMITTED");
        assert_eq!(outcome.scheduled, None);
        assert_eq!(driver.record_count(), 0);
        Ok(())
    }

    #[test]
    fn two_anchors_fire_independently_in_scheduling_order() -> TestResult {
        let content = wall_content(
            &[(CRACK, WALL_ACTION, 1_000)],
            &[(WALL_A, 100), (WALL_B, 300)],
            "lock:r1",
        )?;
        let mut runtimes = walls(&content, &[WALL_A, WALL_B])?;
        let clock = ManualClock::new(Moment::ZERO);
        let mut driver = driver(&clock, RevertDriverLimits::registered())?;
        // Equal deadlines and revisions; B is scheduled first but sorts last by identity.
        let on_b = scheduled(forward(
            &mut driver,
            &mut runtimes,
            WALL_B,
            "oteryn:occurrence/z",
            WALL_ACTION,
            CRACK,
        )?)?;
        let on_a = scheduled(forward(
            &mut driver,
            &mut runtimes,
            WALL_A,
            "oteryn:occurrence/a",
            WALL_ACTION,
            CRACK,
        )?)?;
        assert!(on_a < on_b);
        clock.advance(millis(1_000))?;
        let report = driver.wake(&mut runtimes, &BTreeSet::new())?;
        let order = report
            .fired
            .iter()
            .map(|(id, outcome)| (id.clone(), outcome.committed()))
            .collect::<Vec<_>>();
        assert_eq!(order, vec![(on_b, true), (on_a, true)]);
        assert_eq!(state_of(&runtimes, WALL_A)?, at(WALL_SEALED, 2));
        assert_eq!(state_of(&runtimes, WALL_B)?, at(WALL_SEALED, 2));
        Ok(())
    }

    #[test]
    fn a_mutually_timed_pair_fires_once_and_stops() -> TestResult {
        let content = wall_content(
            &[(CRACK, WALL_ACTION, 1_000), (MEND, WALL_ACTION, 1_000)],
            &[(WALL_A, 100)],
            "lock:r1",
        )?;
        let mut runtimes = walls(&content, &[WALL_A])?;
        let clock = ManualClock::new(Moment::ZERO);
        let mut driver = driver(&clock, RevertDriverLimits::registered())?;
        scheduled(forward(
            &mut driver,
            &mut runtimes,
            WALL_A,
            "oteryn:occurrence/wall/1",
            WALL_ACTION,
            CRACK,
        )?)?;
        clock.advance(millis(1_000))?;
        let report = driver.wake(&mut runtimes, &BTreeSet::new())?;
        assert_eq!(report.fired.len(), 1);
        assert_eq!(driver.issuer().minted, 2);
        assert_eq!(driver.record_count(), 1);
        assert_eq!(driver.next_deadline(), None);
        assert_eq!(state_of(&runtimes, WALL_A)?, at(WALL_SEALED, 2));
        Ok(())
    }

    #[test]
    fn an_occupied_revert_is_refused_once_and_never_retried() -> TestResult {
        let content = wall_content(&[(CRACK, WALL_ACTION, 1_000)], &[(WALL_A, 100)], "lock:r1")?;
        let mut runtimes = walls(&content, &[WALL_A])?;
        let clock = ManualClock::new(Moment::ZERO);
        let mut driver = driver(&clock, RevertDriverLimits::registered())?;
        let id = scheduled(forward(
            &mut driver,
            &mut runtimes,
            WALL_A,
            "oteryn:occurrence/wall/1",
            WALL_ACTION,
            CRACK,
        )?)?;
        let occupied = runtimes
            .get(&PlacementKey::new(WALL_A)?)
            .ok_or(fixture("runtime"))?
            .collision_cells()
            .clone();
        clock.advance(millis(1_000))?;
        let report = driver.wake(&mut runtimes, &occupied)?;
        let [(_, outcome)] = report.fired.as_slice() else {
            return Err("expected one fired revert".into());
        };
        assert_eq!(disposition(outcome), Some("OCCUPIED"));
        assert!(
            driver
                .wake(&mut runtimes, &BTreeSet::new())?
                .fired
                .is_empty()
        );
        assert_eq!(
            driver.present(&id, &mut runtimes, &BTreeSet::new())?,
            RevertPresentation::Terminal(outcome.clone())
        );
        assert_eq!(state_of(&runtimes, WALL_A)?, at(WALL_CRACKED, 1));
        Ok(())
    }

    #[test]
    fn a_bounded_wake_leaves_the_remainder_for_a_later_wake() -> TestResult {
        let content = wall_content(
            &[(CRACK, WALL_ACTION, 1_000)],
            &[(WALL_A, 100), (WALL_B, 300)],
            "lock:r1",
        )?;
        let mut runtimes = walls(&content, &[WALL_A, WALL_B])?;
        let clock = ManualClock::new(Moment::ZERO);
        let mut driver = driver(&clock, RevertDriverLimits::new(4, 1)?)?;
        for (wall, occurrence) in [
            (WALL_A, "oteryn:occurrence/a"),
            (WALL_B, "oteryn:occurrence/b"),
        ] {
            scheduled(forward(
                &mut driver,
                &mut runtimes,
                wall,
                occurrence,
                WALL_ACTION,
                CRACK,
            )?)?;
        }
        clock.advance(millis(1_000))?;
        let first = driver.wake(&mut runtimes, &BTreeSet::new())?;
        assert_eq!((first.fired.len(), first.more_due), (1, true));
        let second = driver.wake(&mut runtimes, &BTreeSet::new())?;
        assert_eq!((second.fired.len(), second.more_due), (1, false));
        assert!(RevertDriverLimits::new(1, MAX_DUE_REVERTS_PER_WAKE + 1).is_err());
        assert!(RevertDriverLimits::new(MAX_REVERT_RECORDS_PER_SCOPE_GENERATION + 1, 1).is_err());
        Ok(())
    }

    #[test]
    fn only_the_drivers_own_clock_decides_what_is_due() -> TestResult {
        let content = wall_content(&[(CRACK, WALL_ACTION, 1_000)], &[(WALL_A, 100)], "lock:r1")?;
        let mut runtimes = walls(&content, &[WALL_A])?;
        let clock = ManualClock::new(Moment::ZERO);
        let unrelated = ManualClock::new(Moment::ZERO);
        let mut driver = driver(&clock, RevertDriverLimits::registered())?;
        scheduled(forward(
            &mut driver,
            &mut runtimes,
            WALL_A,
            "oteryn:occurrence/wall/1",
            WALL_ACTION,
            CRACK,
        )?)?;
        // No driver call takes a clock: another clock's origin cannot make the record due.
        unrelated.advance(millis(1_000_000))?;
        assert!(
            driver
                .wake(&mut runtimes, &BTreeSet::new())?
                .fired
                .is_empty()
        );
        clock.advance(millis(1_000))?;
        assert_eq!(driver.wake(&mut runtimes, &BTreeSet::new())?.fired.len(), 1);
        Ok(())
    }

    #[test]
    fn ordinal_exhaustion_is_scope_terminal_and_leaves_the_record_pending() -> TestResult {
        let content = wall_content(&[(CRACK, WALL_ACTION, 1_000)], &[(WALL_A, 100)], "lock:r1")?;
        let mut runtimes = walls(&content, &[WALL_A])?;
        let clock = ManualClock::new(Moment::ZERO);
        let mut driver = driver(&clock, RevertDriverLimits::registered())?;
        let id = scheduled(forward(
            &mut driver,
            &mut runtimes,
            WALL_A,
            "oteryn:occurrence/wall/1",
            WALL_ACTION,
            CRACK,
        )?)?;
        driver.issuer_mut_for_test().next = None;
        clock.advance(millis(1_000))?;
        assert!(matches!(
            driver.wake(&mut runtimes, &BTreeSet::new()),
            Err(RevertError::ScopeTerminal)
        ));
        assert!(matches!(
            driver.lifecycle(&id),
            Some(RevertLifecycle::Pending(_))
        ));
        assert_eq!(state_of(&runtimes, WALL_A)?, at(WALL_CRACKED, 1));
        assert!(matches!(
            driver.wake(&mut runtimes, &BTreeSet::new()),
            Err(RevertError::ScopeTerminal)
        ));
        Ok(())
    }

    #[test]
    fn a_fresh_driver_after_scope_restart_holds_no_records() -> TestResult {
        let content = wall_content(&[(CRACK, WALL_ACTION, 1_000)], &[(WALL_A, 100)], "lock:r1")?;
        let mut runtimes = walls(&content, &[WALL_A])?;
        let clock = ManualClock::new(Moment::ZERO);
        let mut first = driver(&clock, RevertDriverLimits::registered())?;
        let id = scheduled(forward(
            &mut first,
            &mut runtimes,
            WALL_A,
            "oteryn:occurrence/wall/1",
            WALL_ACTION,
            CRACK,
        )?)?;
        drop(first);
        let mut restarted = driver(&clock, RevertDriverLimits::registered())?;
        let mut runtimes = walls(&content, &[WALL_A])?;
        assert_eq!(restarted.lifecycle(&id), None);
        assert_eq!(restarted.next_deadline(), None);
        clock.advance(millis(1_000))?;
        assert!(
            restarted
                .wake(&mut runtimes, &BTreeSet::new())?
                .fired
                .is_empty()
        );
        assert!(matches!(
            restarted.present(&id, &mut runtimes, &BTreeSet::new()),
            Err(RevertError::UnknownOccurrence)
        ));
        Ok(())
    }

    #[test]
    fn a_destination_must_resolve_to_exactly_one_placement_in_the_bound_world() -> TestResult {
        // #1133 carry-over (a).
        let (content, revert) = duke_content(DUKE)?;
        let anchor = duke_anchor()?;
        let rearm = duke_rearm();
        let transitions = [OPEN_TELEPORTER, &rearm, revert.as_str()];
        let reward = duke_destination("reward_destination")?;

        let mut duplicate = content.clone();
        duplicate.placements.push(placement_at(
            &content,
            reward.clone(),
            marker_definition()?,
            400,
        )?);
        assert!(matches!(
            bind_at(&duplicate, &anchor, &transitions, 1),
            Err(WorldRuntimeError::Content(ContentError::DuplicateKey(key))) if key == reward.as_str()
        ));

        let mut cross_world = content.clone();
        let marker = cross_world
            .placements
            .iter_mut()
            .find(|placement| placement.key == reward)
            .ok_or(fixture("reward marker"))?;
        marker.address.world_id = world(2)?;
        assert!(matches!(
            bind_at(&cross_world, &anchor, &transitions, 1),
            Err(WorldRuntimeError::Content(ContentError::InvalidArtifact(_)))
        ));

        let mut cross_frame = content.clone();
        let marker = cross_frame
            .placements
            .iter_mut()
            .find(|placement| placement.key == reward)
            .ok_or(fixture("reward marker"))?;
        marker.address.coordinate_frame = CoordinateFrameRef::new("another-frame")?;
        assert!(matches!(
            bind_at(&cross_frame, &anchor, &transitions, 1),
            Err(WorldRuntimeError::Content(ContentError::InvalidArtifact(_)))
        ));

        assert!(bind_at(&content, &anchor, &transitions, 1).is_ok());
        Ok(())
    }

    #[test]
    fn two_revert_destination_occurrences_on_one_forward_stay_rejected() -> TestResult {
        // #1133 carry-over (c): each occurrence gets its own qualifying inverse, so `bind`
        // finds two and rejects the placement as ambiguous, fail-closed.
        let mut encounter: Value = serde_json::from_str(DUKE)?;
        let actions = encounter
            .pointer_mut("/rules/0/actions")
            .and_then(Value::as_array_mut)
            .ok_or(fixture("duke actions"))?;
        let first = actions.first().cloned().ok_or(fixture("duke action"))?;
        actions.push(first);
        let (content, revert) = duke_content(&serde_json::to_string(&encounter)?)?;
        let second_revert = format!("{}/1/revert", DUKE_ACTION.trim_end_matches("/0"));
        let second_rearm = format!("{}/1/rearm", DUKE_ACTION.trim_end_matches("/0"));
        assert!(matches!(
            bind_at(
                &content,
                &duke_anchor()?,
                &[
                    OPEN_TELEPORTER,
                    &duke_rearm(),
                    &second_rearm,
                    revert.as_str(),
                    &second_revert,
                ],
                1
            ),
            Err(WorldRuntimeError::InvalidBinding(
                "revert_after_ms transition has an ambiguous bound inverse at this placement"
            ))
        ));
        Ok(())
    }

    // §10.4 (OD9): the pre-authored `CREATE` teleporters.
    const OD9_SAMPLES: [(&str, &str, &str); 2] = [
        (
            "canary:encounter/death_priest_shargon",
            "shargon_exit",
            include_str!(
                "../../../tools/content-schema/encounter-authoring/samples/death_priest_shargon/encounter.json"
            ),
        ),
        (
            "canary:encounter/the_ravager",
            "ravager_exit",
            include_str!(
                "../../../tools/content-schema/encounter-authoring/samples/the_ravager/encounter.json"
            ),
        ),
    ];

    /// One OD9 sample bound at its anchor from the lowered tables, as `od9_bound` returns it.
    struct Od9 {
        content: CanonicalReferencePlayableContent,
        runtimes: BTreeMap<PlacementKey, LocalObjectRuntime>,
        anchor: PlacementKey,
        action: String,
        absent: String,
        create: String,
        remove: String,
    }

    /// One OD9 sample lowered, linked and bound at its anchor from the lowered absent start, with
    /// its destination marker injected.
    fn od9_bound(key: &str, encounter: &str) -> TestResult<Od9> {
        let mut source = source_with(
            vec![state(SEALED_ITEM, LocalObjectCollisionPresence::Absent)?],
            vec![],
            "lock:od9-r1",
        )?;
        let anchors = BTreeMap::from([("exit_teleporter".to_owned(), object_definition()?)]);
        let lowered = lower_map_item_transforms(encounter, &source, &anchors)?;
        lowered.apply_to_source(&mut source)?;
        let mut content = link_reference_playable(source)?;
        let anchor = anchor_placement_key(key, "exit_teleporter")?;
        let tables = lowered
            .placements
            .get(&anchor)
            .ok_or(fixture("lowered anchor tables"))?;
        let mut placement = placement_at(&content, anchor.clone(), object_definition()?, 100)?;
        placement.local_object_initial_state = tables.initial_state.clone();
        placement.local_object_state_attributes = tables.state_attributes.clone();
        placement.local_object_revert_after_ms = tables.revert_after_ms.clone();
        placement.local_object_event_transitions = tables.event_transitions.clone();
        let mut placements = vec![placement];
        for (x, marker) in (200..).zip(&lowered.destination_markers) {
            placements.push(placement_at(
                &content,
                marker.clone(),
                marker_definition()?,
                x,
            )?);
        }
        content.placements = placements;
        let rule = key.rsplit('/').next().ok_or(fixture("encounter key"))?;
        let action = format!("{key}/{rule}_death/0");
        let (create, remove) = (format!("{action}/create"), format!("{action}/remove"));
        let runtime = bind_at(&content, &anchor, &[&create, &remove], 1)?;
        Ok(Od9 {
            content,
            runtimes: BTreeMap::from([(anchor.clone(), runtime)]),
            anchor,
            absent: format!("{action}/absent"),
            action,
            create,
            remove,
        })
    }

    /// What one boss death did to the teleporter.
    #[derive(Debug)]
    enum Od9Kill {
        /// The create committed (absent -> present) and scheduled its revert.
        Opened(ForwardOutcome),
        /// Owner decision Q2=b: the teleporter was open; its revert was re-armed.
        Rearmed(ChildOccurrenceRef),
    }

    /// One boss death as the encounter owner delivers it: from absent, select and commit the
    /// create through the driver; while open, re-arm the pending revert (owner decision Q2=b).
    fn od9_kill(
        driver: &mut ScopeRevertDriver<TestIssuer>,
        od9: &mut Od9,
        owner: &str,
        death: &str,
    ) -> TestResult<Option<Od9Kill>> {
        let runtime = od9
            .runtimes
            .get_mut(&od9.anchor)
            .ok_or(fixture("runtime"))?;
        let action = LoweredActionId::new(&od9.action)?;
        let owner = TransitionEventOwner::new(owner)?;
        if let Some(transition) = select_timed_forward(runtime, &action, &od9.content.transitions)?
        {
            let operation =
                operation_owned(runtime, transition.as_str(), runtime.revision(), &owner)?;
            return Ok(Some(Od9Kill::Opened(driver.apply_forward(
                runtime,
                &forward_child(death, None)?,
                &revisions()?,
                &action,
                &operation,
                &BTreeSet::new(),
            )?)));
        }
        let Some(create) = select_open_create(runtime, &action, &od9.content.transitions)? else {
            return Ok(None);
        };
        Ok(Some(Od9Kill::Rearmed(
            driver.rearm_open_create(runtime, &action, &create, &owner)?,
        )))
    }

    fn opened(kill: Option<Od9Kill>) -> TestResult<ChildOccurrenceRef> {
        match kill {
            Some(Od9Kill::Opened(forward)) => {
                assert_eq!(forward.outcome.disposition(), "COMMITTED");
                Ok(scheduled(forward)?)
            }
            other => Err(format!("expected the create to commit, got {other:?}").into()),
        }
    }

    /// `(is_absent, destination, blocks nothing)` of the bound teleporter.
    fn od9_view(od9: &Od9) -> Option<(bool, Option<String>, bool)> {
        od9.runtimes.get(&od9.anchor).map(|runtime| {
            (
                runtime.is_absent(),
                destination_of(runtime).map(str::to_owned),
                runtime.blocking_cells().is_empty(),
            )
        })
    }

    #[test]
    fn od9_create_teleporter_opens_a_re_kill_re_arms_its_revert_and_it_re_opens_after_closing()
    -> TestResult {
        for (key, exit, encounter) in OD9_SAMPLES {
            let mut od9 = od9_bound(key, encounter)?;
            let anchor = od9.anchor.clone();
            let absent = od9.absent.clone();
            let exit = Some(anchor_placement_key(key, exit)?.as_str().to_owned());
            let clock = ManualClock::new(Moment::ZERO);
            let mut driver = driver(&clock, RevertDriverLimits::registered())?;
            let state = |od9: &Od9| state_of(&od9.runtimes, anchor.as_str());

            // Before the first kill: bound, absent, non-colliding, no destination.
            assert_eq!(state(&od9)?, at(&absent, 0));
            assert_eq!(od9_view(&od9), Some((true, None, true)));

            // Kill 1 at t=0: create commits absent -> present with the destination, one record.
            let first = opened(od9_kill(
                &mut driver,
                &mut od9,
                key,
                &format!("{key}/death/1"),
            )?)?;
            assert_eq!(state(&od9)?, at(SEALED_ITEM, 1));
            assert_eq!(od9_view(&od9), Some((false, exit.clone(), true)));
            let Some(RevertLifecycle::Pending(pending)) = driver.lifecycle(&first) else {
                return Err("revert record is not PENDING".into());
            };
            assert_eq!(pending.inverse().as_str(), od9.remove);
            assert_eq!(pending.expected_state().as_str(), SEALED_ITEM);
            assert_eq!(pending.expected_revision(), 1);

            // Re-kill at t=4 min while open (owner decision Q2=b): the same record is re-armed
            // to the full 5 min from now. One ordinal for the accepted input; no second record,
            // no second object and no mutation of the teleporter.
            clock.advance(millis(240_000))?;
            let minted = driver.issuer().minted;
            let deadline = driver.next_deadline();
            let Some(Od9Kill::Rearmed(rearmed)) =
                od9_kill(&mut driver, &mut od9, key, &format!("{key}/death/open"))?
            else {
                return Err("a kill while open did not re-arm the revert".into());
            };
            assert_eq!(rearmed, first);
            assert_eq!(driver.issuer().minted, minted + 1);
            assert_eq!(driver.record_count(), 1);
            assert_ne!(driver.next_deadline(), deadline);
            assert_eq!(state(&od9)?, at(SEALED_ITEM, 1));
            let Some(RevertLifecycle::Pending(pending)) = driver.lifecycle(&first) else {
                return Err("re-armed record is not PENDING".into());
            };
            assert_eq!(pending.expected_revision(), 1);

            // At t=5 min it stays open; it closes at t=4+5 min, not at 5 min.
            clock.advance(millis(60_000))?;
            assert!(
                driver
                    .wake(&mut od9.runtimes, &BTreeSet::new())?
                    .fired
                    .is_empty()
            );
            assert_eq!(state(&od9)?, at(SEALED_ITEM, 1));
            clock.advance(millis(239_999))?;
            assert!(
                driver
                    .wake(&mut od9.runtimes, &BTreeSet::new())?
                    .fired
                    .is_empty()
            );
            clock.advance(millis(1))?;
            let report = driver.wake(&mut od9.runtimes, &BTreeSet::new())?;
            let [(fired, first_outcome)] = report.fired.as_slice() else {
                return Err("expected the re-armed revert to fire at 4+5 min".into());
            };
            assert_eq!(fired, &first);
            assert!(first_outcome.committed());
            let first_outcome = first_outcome.clone();
            assert_eq!(state(&od9)?, at(&absent, 2));
            assert_eq!(od9_view(&od9), Some((true, None, true)));

            // A kill after it closed re-opens it normally: the create again from the natural
            // absent state (D90: no re-arm edge), with its own new record.
            let runtime = od9.runtimes.get(&anchor).ok_or(fixture("runtime"))?;
            let selected = select_timed_forward(
                runtime,
                &LoweredActionId::new(&od9.action)?,
                &od9.content.transitions,
            )?;
            assert_eq!(
                selected.as_ref().map(TransitionKey::as_str),
                Some(od9.create.as_str())
            );
            let second = opened(od9_kill(
                &mut driver,
                &mut od9,
                key,
                &format!("{key}/death/2"),
            )?)?;
            assert_ne!(second, first);
            assert_eq!(state(&od9)?, at(SEALED_ITEM, 3));
            assert_eq!(od9_view(&od9), Some((false, exit.clone(), true)));
            clock.advance(millis(299_999))?;
            assert!(
                driver
                    .wake(&mut od9.runtimes, &BTreeSet::new())?
                    .fired
                    .is_empty()
            );
            clock.advance(millis(1))?;
            let report = driver.wake(&mut od9.runtimes, &BTreeSet::new())?;
            let [(fired, second_outcome)] = report.fired.as_slice() else {
                return Err("expected the second revert to fire".into());
            };
            assert_eq!(fired, &second);
            assert!(second_outcome.committed());
            let second_outcome = second_outcome.clone();
            assert_eq!(state(&od9)?, at(&absent, 4));

            // Two distinct TERMINAL records, one per opening kill.
            assert_eq!(driver.record_count(), 2);
            assert_eq!(
                driver.lifecycle(&first),
                Some(&RevertLifecycle::Terminal(first_outcome))
            );
            assert_eq!(
                driver.lifecycle(&second),
                Some(&RevertLifecycle::Terminal(second_outcome))
            );
            assert_eq!(driver.next_deadline(), None);
        }
        Ok(())
    }

    #[test]
    fn od9_create_edges_refuse_use_session_and_foreign_owners() -> TestResult {
        let not_bound = "command names a transition this local-object runtime does not bind";
        for (key, _, encounter) in OD9_SAMPLES {
            let mut od9 = od9_bound(key, encounter)?;
            let anchor = od9.anchor.clone();
            let absent = od9.absent.clone();
            let clock = ManualClock::new(Moment::ZERO);
            let mut driver = driver(&clock, RevertDriverLimits::registered())?;
            let foreign = TransitionEventOwner::new(DUKE_KEY)?;
            let mut ingress = CommandIngress::new();

            // Absent: USE selects nothing, and a session command naming create is refused.
            let runtime = od9.runtimes.get_mut(&anchor).ok_or(fixture("runtime"))?;
            assert_eq!(
                runtime.attempt_use(0, &BTreeSet::new())?,
                LocalObjectUseOutcome::NothingToUse
            );
            assert!(matches!(
                apply_session(runtime, &mut ingress, 1, &od9.create)?,
                Err(WorldRuntimeError::InvalidBinding(reason)) if reason == not_bound
            ));

            // A foreign owner's create is refused before any ordinal, record or mutation.
            let foreign_create = operation_owned(runtime, &od9.create, 0, &foreign)?;
            assert!(matches!(
                driver.apply_forward(
                    runtime,
                    &forward_child(&format!("{key}/death/1"), None)?,
                    &revisions()?,
                    &LoweredActionId::new(&od9.action)?,
                    &foreign_create,
                    &BTreeSet::new(),
                ),
                Err(RevertError::Runtime(WorldRuntimeError::EventOwnerMismatch))
            ));
            assert_eq!(driver.issuer().minted, 0);
            assert_eq!(driver.record_count(), 0);
            assert_eq!(state_of(&od9.runtimes, anchor.as_str())?, at(&absent, 0));

            // Open: USE cannot select the remove edge, a session command naming it is refused,
            // and a foreign owner's remove is refused; nothing mutates.
            let id = opened(od9_kill(
                &mut driver,
                &mut od9,
                key,
                &format!("{key}/death/1"),
            )?)?;
            let runtime = od9.runtimes.get_mut(&anchor).ok_or(fixture("runtime"))?;
            // Q2=b re-arm: another owner's re-kill is refused (D91) before any ordinal, and the
            // deadline is unchanged.
            let (minted, deadline) = (driver.issuer().minted, driver.next_deadline());
            let create = TransitionKey::new(&od9.create)?;
            let action = LoweredActionId::new(&od9.action)?;
            assert!(matches!(
                driver.rearm_open_create(runtime, &action, &create, &foreign),
                Err(RevertError::Runtime(WorldRuntimeError::EventOwnerMismatch))
            ));
            assert_eq!(driver.issuer().minted, minted);
            assert_eq!(driver.next_deadline(), deadline);
            assert_eq!(
                runtime.attempt_use(1, &BTreeSet::new())?,
                LocalObjectUseOutcome::NothingToUse
            );
            assert!(matches!(
                apply_session(runtime, &mut ingress, 2, &od9.remove)?,
                Err(WorldRuntimeError::InvalidBinding(reason)) if reason == not_bound
            ));
            assert_eq!(ingress.outstanding(), 0);
            let foreign_remove = operation_owned(runtime, &od9.remove, 1, &foreign)?;
            assert!(matches!(
                runtime.apply_scope_operation(
                    scope()?,
                    generation(1)?,
                    &foreign_remove,
                    &BTreeSet::new(),
                    None,
                    |_| Ok::<(), Infallible>(()),
                ),
                Err(WorldRuntimeError::EventOwnerMismatch)
            ));
            assert_eq!(
                state_of(&od9.runtimes, anchor.as_str())?,
                at(SEALED_ITEM, 1)
            );

            // The driver still reverts on the create's own owner.
            clock.advance(millis(300_000))?;
            let report = driver.wake(&mut od9.runtimes, &BTreeSet::new())?;
            let [(fired, outcome)] = report.fired.as_slice() else {
                return Err("expected one fired revert".into());
            };
            assert_eq!(fired, &id);
            assert!(outcome.committed());
            assert_eq!(state_of(&od9.runtimes, anchor.as_str())?, at(&absent, 2));
        }
        Ok(())
    }

    #[test]
    fn a_re_kill_without_one_pending_revert_fails_closed_and_the_duke_keeps_its_no_op() -> TestResult
    {
        // Q2=b: an open teleporter whose revert is no longer PENDING re-arms nothing.
        let (key, _, encounter) = OD9_SAMPLES[0];
        let mut od9 = od9_bound(key, encounter)?;
        let clock = ManualClock::new(Moment::ZERO);
        let mut driver = driver(&clock, RevertDriverLimits::registered())?;
        let id = opened(od9_kill(
            &mut driver,
            &mut od9,
            key,
            &format!("{key}/death/1"),
        )?)?;
        driver
            .force_in_flight_for_test(&id)
            .ok_or(fixture("force IN_FLIGHT"))?;
        let minted = driver.issuer().minted;
        assert!(matches!(
            od9_kill(&mut driver, &mut od9, key, &format!("{key}/death/open")),
            Err(error) if matches!(
                error.downcast_ref::<RevertError>(),
                Some(RevertError::NoOpenRevert)
            )
        ));
        assert_eq!(driver.issuer().minted, minted);
        assert_eq!(driver.record_count(), 1);

        // Owner decision 3 still covers the duke's TRANSFORM teleporter: no CREATE edge opens it.
        let (content, revert) = duke_content(DUKE)?;
        let anchor = duke_anchor()?;
        let mut runtimes = BTreeMap::from([(
            anchor.clone(),
            bind_at(
                &content,
                &anchor,
                &[OPEN_TELEPORTER, &duke_rearm(), revert.as_str()],
                1,
            )?,
        )]);
        let mut duke_driver = self::driver(&clock, RevertDriverLimits::registered())?;
        scheduled(forward(
            &mut duke_driver,
            &mut runtimes,
            anchor.as_str(),
            DUKE_DEATH,
            DUKE_ACTION,
            OPEN_TELEPORTER,
        )?)?;
        let runtime = runtimes.get(&anchor).ok_or(fixture("runtime"))?;
        let action = LoweredActionId::new(DUKE_ACTION)?;
        assert_eq!(
            select_timed_forward(runtime, &action, &content.transitions)?,
            None
        );
        assert_eq!(
            select_open_create(runtime, &action, &content.transitions)?,
            None
        );
        Ok(())
    }
}
