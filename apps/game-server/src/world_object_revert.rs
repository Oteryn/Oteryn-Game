//! #162 §7: scope-owned `revert_after_ms` lifecycle records and the per-scope driver that fires
//! them (docs/architecture/OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md
//! §7, direction owner-accepted 2026-09-28; §9 for the attribute-bearing teleporter shape).
//!
//! One record per revert `InteractionChildOccurrenceRef`, `PENDING` -> `IN_FLIGHT` ->
//! `TERMINAL`. A record is created only in the same staged commit as a non-timer-origin forward
//! operation whose `prepare` returned `Publish`, and capacity for it is checked only then. The
//! driver fires due records through `LocalObjectRuntime::apply_scope_operation` with the bound
//! inverse; a timer-origin firing never schedules another record (§7 Rounds 7/8).
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
    ContentError, LogicalCell, LoweredActionId, PlacementKey, ProductionKey, TransitionKey,
};
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
pub(crate) const MAX_REVERT_RECORDS_PER_SCOPE_GENERATION: usize = 4_096;
/// `WOBJ-RL-05`: due records one driver wake admits; the remainder waits for a later wake.
pub(crate) const MAX_DUE_REVERTS_PER_WAKE: usize = 32;

const DISPOSITION_COMMITTED: &str = "COMMITTED";

/// The forward operation's own GAME-INTERACTION occurrence identity, supplied by its caller (for
/// an encounter, its trigger occurrence). Bounded as a first-production key (512 bytes).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ForwardOccurrenceRef(ProductionKey);

impl ForwardOccurrenceRef {
    pub(crate) fn new(value: &str) -> Result<Self, ContentError> {
        ProductionKey::new(value).map(Self)
    }
}

/// The revert's own identity: a nested child of the forward occurrence, distinguished by the
/// authored action that scheduled it. Computed once at scheduling, never from an ordinal.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct InteractionChildOccurrenceRef {
    parent: ForwardOccurrenceRef,
    action: LoweredActionId,
}

impl InteractionChildOccurrenceRef {
    #[must_use]
    pub(crate) fn revert_of(parent: &ForwardOccurrenceRef, action: &LoweredActionId) -> Self {
        Self {
            parent: parent.clone(),
            action: action.clone(),
        }
    }
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

    fn due_key(&self, id: &InteractionChildOccurrenceRef) -> DueKey {
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
    UnknownOccurrence,
    InvalidLimits(&'static str),
    /// Ordinal space exhausted or another fatal step error: no work until a new generation.
    ScopeTerminal,
    Ordinal(GenerationError),
    Time(TimeError),
    Runtime(WorldRuntimeError),
}

impl Display for RevertError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::CapacityExceeded => formatter.write_str("CAPACITY_EXCEEDED: revert records"),
            Self::DuplicateOccurrence => formatter.write_str("revert occurrence already scheduled"),
            Self::UnknownOccurrence => formatter.write_str("unknown revert occurrence"),
            Self::InvalidLimits(reason) => write!(formatter, "invalid revert limits: {reason}"),
            Self::ScopeTerminal => formatter.write_str("revert driver is scope-terminal"),
            Self::Ordinal(error) => write!(formatter, "{error}"),
            Self::Time(error) => write!(formatter, "{error}"),
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
    pub(crate) scheduled: Option<InteractionChildOccurrenceRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WakeReport {
    /// Admitted records in (deadline, scheduling ordinal, sequence) order, with their outcome.
    pub(crate) fired: Vec<(InteractionChildOccurrenceRef, RevertOutcome)>,
    /// More records are already due; they wait for a later wake (bounded batch).
    pub(crate) more_due: bool,
}

type DueKey = (
    Deadline,
    RuntimeExecutionOrdinal,
    u32,
    InteractionChildOccurrenceRef,
);

/// One per scope owner (`ChannelRuntimeV1`/`InstanceRuntime`), never per object: owns the scope's
/// ordinal issuer, its single clock and its lifecycle-record store.
pub(crate) struct ScopeRevertDriver<I> {
    scope: RuntimeScopeRefV1,
    issuer: I,
    clock: Arc<dyn MonotonicClock>,
    limits: RevertDriverLimits,
    records: BTreeMap<InteractionChildOccurrenceRef, RevertLifecycle>,
    due: BTreeSet<DueKey>,
    scope_terminal: bool,
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
        }
    }

    #[must_use]
    pub(crate) fn issuer(&self) -> &I {
        &self.issuer
    }

    #[must_use]
    pub(crate) fn lifecycle(&self, id: &InteractionChildOccurrenceRef) -> Option<&RevertLifecycle> {
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
        occurrence: &ForwardOccurrenceRef,
        action: &LoweredActionId,
        operation: &ScopeLocalObjectOperation,
        occupied_cells: &BTreeSet<LogicalCell>,
    ) -> Result<ForwardOutcome, RevertError> {
        if self.scope_terminal {
            return Err(RevertError::ScopeTerminal);
        }
        let generation = self.issuer.generation();
        let scheduling_ordinal = self.accept(generation)?;
        let transition = operation.transition_key();
        let timed = match runtime.revert_after_ms(transition, action) {
            None => None,
            Some(ms) => Some((
                ms,
                runtime
                    .revert_inverse(transition)
                    .cloned()
                    .ok_or(RevertError::Runtime(WorldRuntimeError::InvalidBinding(
                        "timed transition has no bound inverse",
                    )))?,
            )),
        };
        let id = InteractionChildOccurrenceRef::revert_of(occurrence, action);
        let placement = runtime.placement_key().clone();
        let incarnation = runtime.incarnation();
        let content_generation = runtime.content_generation().clone();
        let scope = self.scope;
        let (records, due, clock, capacity) = (
            &mut self.records,
            &mut self.due,
            &self.clock,
            self.limits.record_capacity,
        );
        let mut scheduled = None;
        let staged = runtime.apply_scope_operation(
            scope,
            generation,
            operation,
            occupied_cells,
            |publish| {
                let Some((ms, inverse)) = timed else {
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
        id: &InteractionChildOccurrenceRef,
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
        );
        // Timer origin: never stages a new record, even for a timed inverse (§7 Rounds 7/8).
        let result = runtime.apply_scope_operation(
            pending.scope,
            pending.scope_generation,
            &operation,
            occupied_cells,
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
        id: &InteractionChildOccurrenceRef,
        pending: &PendingRevert,
        outcome: RevertOutcome,
    ) -> RevertPresentation {
        self.due.remove(&pending.due_key(id));
        self.records
            .insert(id.clone(), RevertLifecycle::Terminal(outcome.clone()));
        RevertPresentation::Terminal(outcome)
    }

    #[cfg(test)]
    fn force_in_flight_for_test(&mut self, id: &InteractionChildOccurrenceRef) -> Option<()> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::encounter_map_item::{
        anchor_placement_key, lower_map_item_transforms, marker_placement,
    };
    use crate::content::{
        CanonicalReferencePlayableContent, ClientProjectionClass, ContentLockBinding,
        ContentLockEntry, CoordinateFrameRef, DefinitionFamily, DefinitionRevisionRef,
        EvidenceBindingRef, EvidenceDisposition, LOCAL_OBJECT_TRANSFORM_INTENT_FAMILY,
        LocalObjectCollisionPresence, LocalObjectStateDefinition, MapRevisionRef,
        OwnerCapabilityRequirement, PackageManifestBinding, PlacementRef, ProductionAtom,
        REFERENCE_PLAYABLE_CAPABILITY_PROFILE, REFERENCE_PLAYABLE_CONTENT_PROFILE_ID,
        ReferenceDefinition, ReferenceDefinitionKind, ReferencePlayableContentSource,
        Sha256HexDigest, SpatialAddress, TransitionBinding, TypedDefinitionRef,
        link_reference_playable,
    };
    use crate::foundation::WorldId;
    use crate::world_runtime::{LocalObjectUseOutcome, ScopeContentGenerationFence};
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
        RuntimeScopeRefV1::instance(world(1)?, uuid_v7(4)).map_err(fixture)
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
    /// given `(transition, action, ms)` revert durations on every placement.
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

    /// Test-only stand-in for the scope's one `ScopeRuntimeFence` (constructible only inside
    /// Foundation): the same `accept_input` contract, plus a count of minted ordinals.
    #[derive(Debug)]
    struct TestIssuer {
        generation: ScopeOwnershipGeneration,
        next: Option<u64>,
        minted: u64,
    }

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

    fn driver(
        clock: &ManualClock,
        limits: RevertDriverLimits,
    ) -> TestResult<ScopeRevertDriver<TestIssuer>> {
        Ok(ScopeRevertDriver::new(
            scope()?,
            TestIssuer {
                generation: generation(1)?,
                next: Some(1),
                minted: 0,
            },
            Arc::new(clock.clone()),
            limits,
        ))
    }

    fn operation(
        runtime: &LocalObjectRuntime,
        transition: &str,
        expected_revision: u64,
    ) -> TestResult<ScopeLocalObjectOperation> {
        Ok(ScopeLocalObjectOperation::new(
            runtime.placement_key().clone(),
            runtime.incarnation(),
            runtime.content_generation().clone(),
            LocalObjectOperation::new(TransitionKey::new(transition)?),
            expected_revision,
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
            &ForwardOccurrenceRef::new(occurrence)?,
            &LoweredActionId::new(action)?,
            &operation,
            &BTreeSet::new(),
        )?)
    }

    fn scheduled(
        outcome: ForwardOutcome,
    ) -> Result<InteractionChildOccurrenceRef, WorldRuntimeError> {
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
            bind_at(&content, &anchor, &[OPEN_TELEPORTER, revert.as_str()], 1)?,
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
    fn a_user_driven_revert_resolves_the_timer_terminal_without_a_double_revert() -> TestResult {
        // #1133 carry-over (b): USE selects the bound inverse from the forward target state.
        // §7's one path for an intervening change applies: the timer reaches `prepare` and
        // terminalizes STALE_STATE; it never reverts a second time.
        let (content, revert) = duke_content(DUKE)?;
        let anchor = duke_anchor()?;
        let mut runtimes = BTreeMap::from([(
            anchor.clone(),
            bind_at(&content, &anchor, &[OPEN_TELEPORTER, revert.as_str()], 1)?,
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
            LocalObjectUseOutcome::Committed {
                state: ProductionKey::new(&post_revert)?,
                revision: 2,
            }
        );
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
        let occurrence = ForwardOccurrenceRef::new("oteryn:occurrence/wall/2")?;
        let action = LoweredActionId::new(WALL_ACTION)?;
        let runtime = runtimes.get_mut(&wall).ok_or(fixture("runtime"))?;
        let crack = operation(runtime, CRACK, runtime.revision())?;
        let full = driver.apply_forward(runtime, &occurrence, &action, &crack, &BTreeSet::new());
        assert!(matches!(full, Err(RevertError::CapacityExceeded)));
        assert_eq!(state_of(&runtimes, WALL_A)?, at(WALL_SEALED, 2));
        assert_eq!(driver.record_count(), 1);

        // An unchanged forward never reaches the capacity check: STALE_STATE, not a failure.
        let runtime = runtimes.get_mut(&wall).ok_or(fixture("runtime"))?;
        let stale = operation(runtime, CRACK, 0)?;
        let result = driver.apply_forward(
            runtime,
            &ForwardOccurrenceRef::new("oteryn:occurrence/wall/3")?,
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
        let transitions = [OPEN_TELEPORTER, revert.as_str()];
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
        assert!(matches!(
            bind_at(
                &content,
                &duke_anchor()?,
                &[OPEN_TELEPORTER, revert.as_str(), &second_revert],
                1
            ),
            Err(WorldRuntimeError::InvalidBinding(
                "revert_after_ms transition has an ambiguous bound inverse at this placement"
            ))
        ));
        Ok(())
    }
}
