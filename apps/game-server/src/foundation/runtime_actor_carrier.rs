//! Fixed-bound, pre-production-only Channel actor storage.
//!
//! The module stays crate-private. Its only continuity grant is derived by
//! `ChannelRuntimeV1::from_committed_assignment` from the committed Channel
//! assignment that `serve` consumes before readiness; one runtime per
//! ownership generation is enforced by that composition, not by this module.

use super::{ChannelId, GameSessionId, NodeId, ScopeOwnershipGeneration, WorldId};
use std::mem::size_of;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CarrierError {
    InvalidCapacity,
    CapacityArithmeticOverflow,
    AllocationFailed,
    CapacityExceeded,
    InvalidAssignmentBinding,
    ContentPinWorldMismatch,
    PlayerReservationMismatch,
    WrongScope,
    InvalidActorIdentity,
    StaleActorGeneration,
    ActorGenerationExhausted,
    InjectedAdmissionFailure,
    NamespaceAlreadyClaimed,
    ContinuityGenerationNotNewer,
    PositionUnavailable,
    PositionAlreadyInitialized,
    InvalidPreProductionPositionContext,
    PositionContextMismatch,
    PositionSnapshotMismatch,
    PositionRevisionExhausted,
    MovementCreatureUnavailable,
    MovementNonCardinal,
    InvalidCreatureHealth,
    InvalidCreatureTarget,
    CreatureTargetMismatch,
    NotCreature,
    CreatureNotActionable,
    InvalidDamage,
    DamageOverflow,
    InvalidCommitBinding,
    CommitBindingTooLarge,
    CommitAllocationFailed,
    OccurrenceConflict,
    PlanConflict,
    InjectedCommitFailure,
    CommittedLethalUnavailable,
    CorpseReceiptMismatch,
    CorpseProjectionConflict,
    InjectedCorpseProjectionFailure,
    InjectedCorpseResponseFailure,
}

impl std::fmt::Display for CarrierError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}
impl std::error::Error for CarrierError {}

/// Authority supplied by a future, independently accepted assignment consumer.
///
/// There is deliberately no constructor, issuer, reissuer, `Clone`, or `Copy`
/// implementation here. Consuming this value makes namespace bootstrap one-shot.
#[derive(Debug)]
struct PreProductionContinuityGrant {
    world_id: WorldId,
    channel_id: ChannelId,
    scope_generation: ScopeOwnershipGeneration,
}

/// Surviving pre-production namespace continuity state.
///
/// This guard deliberately lives outside carrier backing and is neither
/// `Clone` nor `Copy`. A carrier claims the current generation once; losing
/// that carrier therefore cannot make the generation claimable again.
#[derive(Debug)]
struct NamespaceContinuityGuard {
    world_id: WorldId,
    channel_id: ChannelId,
    current_generation: ScopeOwnershipGeneration,
    current_generation_claimed: bool,
}

impl NamespaceContinuityGuard {
    fn from_pre_production_grant(grant: PreProductionContinuityGrant) -> Self {
        Self {
            world_id: grant.world_id,
            channel_id: grant.channel_id,
            current_generation: grant.scope_generation,
            current_generation_claimed: false,
        }
    }

    fn advance(&mut self, grant: PreProductionContinuityGrant) -> Result<(), CarrierError> {
        if grant.world_id != self.world_id || grant.channel_id != self.channel_id {
            return Err(CarrierError::WrongScope);
        }
        if grant.scope_generation.get() <= self.current_generation.get() {
            return Err(CarrierError::ContinuityGenerationNotNewer);
        }

        self.current_generation = grant.scope_generation;
        self.current_generation_claimed = false;
        Ok(())
    }

    fn claim_current_generation(&mut self) -> Result<(), CarrierError> {
        self.ensure_current_generation_unclaimed()?;
        self.current_generation_claimed = true;
        Ok(())
    }

    fn ensure_current_generation_unclaimed(&self) -> Result<(), CarrierError> {
        if self.current_generation_claimed {
            return Err(CarrierError::NamespaceAlreadyClaimed);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActorLocalId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActorLocalGeneration(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActorRef {
    world_id: WorldId,
    channel_id: ChannelId,
    scope_generation: ScopeOwnershipGeneration,
    actor_local_id: ActorLocalId,
    actor_local_generation: ActorLocalGeneration,
}

/// Inseparable semantic actor handle issued by this one Channel carrier.
/// Its fields and constructor are private to Foundation; it is never decoded
/// from a client handle or derived from the fixture Ability `TargetId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExactActorRef(ActorRef);

/// One fixed-slot reservation for a fresh GameSession. It is not a playable
/// actor until the owning durable admission is proven committed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlayerActorReservation {
    game_session_id: GameSessionId,
    actor_ref: ActorRef,
}

/// Immutable provenance of the committed Channel assignment that created this
/// runtime. Source revision identifies the exact assignment decision; no
/// production capacity value lives here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChannelRuntimeAssignmentBinding {
    world_id: WorldId,
    channel_id: ChannelId,
    node_id: NodeId,
    node_registration_revision: u64,
    scope_generation: ScopeOwnershipGeneration,
    source_revision: u64,
}

impl ChannelRuntimeAssignmentBinding {
    pub(crate) const fn world_id(self) -> WorldId {
        self.world_id
    }

    pub(crate) const fn channel_id(self) -> ChannelId {
        self.channel_id
    }

    pub(crate) const fn scope_generation(self) -> ScopeOwnershipGeneration {
        self.scope_generation
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn matches_committed_assignment(
        self,
        world_id: WorldId,
        channel_id: ChannelId,
        node_id: NodeId,
        node_registration_revision: u64,
        ownership_generation: u64,
        source_revision: u64,
        decision_identity: &str,
    ) -> bool {
        let decision_revision = decision_identity
            .strip_prefix("runtime-scope-assignment:")
            .and_then(|value| value.parse::<u64>().ok());
        self.world_id == world_id
            && self.channel_id == channel_id
            && self.node_id == node_id
            && self.node_registration_revision == node_registration_revision
            && self.scope_generation.get() == ownership_generation
            && self.source_revision == source_revision
            && decision_revision == Some(source_revision)
    }
}

/// A borrow of independently current owner continuity and its matching carrier.
/// No mutation, admission or continuity-grant method crosses this boundary.
pub(crate) struct CurrentOwnerExactActorLookup<'a> {
    carrier: &'a ChannelActorCarrier,
    continuity: &'a NamespaceContinuityGuard,
}

/// A short-lived mutable owner borrow. No resolved snapshot can grant this
/// capability: the carrier checks independent continuity and slot generation
/// again at the sole write.
pub(crate) struct CurrentOwnerExactActorCommit<'a> {
    carrier: &'a mut ChannelActorCarrier,
    continuity: &'a NamespaceContinuityGuard,
}

/// Short-lived current-owner authority for the fixed one-creature death
/// projection. It cannot commit Ability damage, move actors, admit actors or
/// issue continuity.
pub(crate) struct CurrentOwnerCombatDeath<'a> {
    carrier: &'a mut ChannelActorCarrier,
    continuity: &'a NamespaceContinuityGuard,
}

/// Borrowed, current-owner position capability for one ordinary actor slot.
/// It cannot admit/remove an actor, issue continuity, or commit Ability damage.
pub(crate) struct CurrentOwnerMovementPosition<'a> {
    carrier: &'a mut ChannelActorCarrier,
    continuity: &'a NamespaceContinuityGuard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MovementLocalPosition {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) floor: i16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MovementPositionContext(PreProductionPositionContext);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MovementPositionSnapshot(PositionSnapshot);

impl MovementPositionSnapshot {
    pub(crate) const fn position(self) -> MovementLocalPosition {
        let position = self.0.version.position;
        MovementLocalPosition {
            x: position.x,
            y: position.y,
            floor: position.floor,
        }
    }

    pub(crate) const fn context(self) -> MovementPositionContext {
        MovementPositionContext(self.0.version.context)
    }

    pub(crate) const fn revision(self) -> u64 {
        self.0.version.revision
    }

    pub(crate) const fn world_id(self) -> WorldId {
        self.0.version.context.world_id
    }
}

impl CurrentOwnerMovementPosition<'_> {
    pub(crate) fn read(
        &self,
        actor: ExactActorRef,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        self.carrier
            .read_movement_position(self.continuity, actor.0)
    }

    pub(crate) fn commit_cardinal(
        &mut self,
        expected: MovementPositionSnapshot,
        next: MovementLocalPosition,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        let before = expected.position();
        let east = before.x.checked_add(1).is_some_and(|x| x == next.x) && next.y == before.y;
        let west = before.x.checked_sub(1).is_some_and(|x| x == next.x) && next.y == before.y;
        let south = before.y.checked_add(1).is_some_and(|y| y == next.y) && next.x == before.x;
        let north = before.y.checked_sub(1).is_some_and(|y| y == next.y) && next.x == before.x;
        if next.floor != before.floor || !(east || west || south || north) {
            return Err(CarrierError::MovementNonCardinal);
        }
        self.carrier.commit_movement_position(
            self.continuity,
            expected.0,
            LocalPosition {
                x: next.x,
                y: next.y,
                floor: next.floor,
            },
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OwnerDamageResult {
    pub(crate) applied: bool,
    pub(crate) health_before: i64,
    pub(crate) health_after: i64,
}

/// Immutable, borrowed Ability proposal. Authority is checked by the owner,
/// independently of these bytes, before the single slot replacement.
pub(crate) struct OwnerDamageCommand<'a> {
    pub(crate) target: &'a [u8],
    pub(crate) occurrence: &'a [u8],
    pub(crate) binding: &'a [u8],
    pub(crate) damage: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OwnerCommitRecord {
    binding: Box<[u8]>,
    damage: i64,
    result: OwnerDamageResult,
}

/// Stable identity of the one committed lethal occurrence. Construction stays
/// private to the physical Channel owner; callers cannot supply occurrence
/// bytes, HP facts or actor generation.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct CreatureDeathOccurrenceRef {
    actor: ExactActorRef,
    commit_binding: Box<[u8]>,
    damage: i64,
    health_before: i64,
}

impl CreatureDeathOccurrenceRef {
    pub(crate) const fn actor(&self) -> ExactActorRef {
        self.actor
    }

    pub(crate) fn commit_binding(&self) -> &[u8] {
        &self.commit_binding
    }

    pub(crate) const fn damage(&self) -> i64 {
        self.damage
    }

    pub(crate) const fn health_before(&self) -> i64 {
        self.health_before
    }
}

/// One runtime-owned, non-persistent corpse projection. The position is the
/// immutable current-owner position captured by the same slot state that holds
/// the lethal commit; dead creatures cannot mutate or initialize it later.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct RuntimeCorpseProjection {
    occurrence: CreatureDeathOccurrenceRef,
    position: VersionedPosition,
}

impl RuntimeCorpseProjection {
    pub(crate) const fn occurrence(&self) -> &CreatureDeathOccurrenceRef {
        &self.occurrence
    }

    pub(crate) const fn position(&self) -> MovementLocalPosition {
        MovementLocalPosition {
            x: self.position.position.x,
            y: self.position.position.y,
            floor: self.position.position.floor,
        }
    }

    pub(crate) const fn position_revision(&self) -> u64 {
        self.position.revision
    }

    pub(crate) const fn context_markers(&self) -> (u64, u64, u64) {
        (
            self.position.context.coordinate_frame_marker,
            self.position.context.map_revision_marker,
            self.position.context.content_generation_marker,
        )
    }
}

/// Opaque, single-use handoff from the owner commit record to Combat. It is
/// deliberately neither Clone nor Copy and has no caller-visible constructor.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct CommittedLethalReceipt {
    projection: RuntimeCorpseProjection,
}

/// 4096 is the registered Ability plan bound; the owner still independently
/// checks the actual complete encoding before allocating its one receipt.
const MAX_OWNER_COMMIT_BINDING_BYTES: usize = 4_096;

impl CurrentOwnerExactActorCommit<'_> {
    pub(crate) fn commit_damage(
        &mut self,
        actor: ExactActorRef,
        command: OwnerDamageCommand<'_>,
    ) -> Result<OwnerDamageResult, CarrierError> {
        self.carrier
            .commit_creature_damage_inner(self.continuity, actor.0, command, false)
    }
}

impl CurrentOwnerCombatDeath<'_> {
    pub(crate) fn committed_lethal_receipt(
        &self,
        actor: ExactActorRef,
    ) -> Result<CommittedLethalReceipt, CarrierError> {
        self.carrier
            .committed_lethal_receipt_inner(self.continuity, actor.0)
    }

    pub(crate) fn project_committed_lethal(
        &mut self,
        receipt: CommittedLethalReceipt,
    ) -> Result<&RuntimeCorpseProjection, CarrierError> {
        self.carrier
            .project_committed_lethal_inner(self.continuity, receipt, false, false)
    }

    #[cfg(test)]
    pub(crate) fn project_committed_lethal_with_failures(
        &mut self,
        receipt: CommittedLethalReceipt,
        fail_before_write: bool,
        fail_after_write: bool,
    ) -> Result<&RuntimeCorpseProjection, CarrierError> {
        self.carrier.project_committed_lethal_inner(
            self.continuity,
            receipt,
            fail_before_write,
            fail_after_write,
        )
    }
}

impl CurrentOwnerExactActorLookup<'_> {
    /// One direct slot/generation lookup. Invalid, vacant and misrouted refs
    /// share one failure; no actor payload or carrier authority is returned.
    pub(crate) fn contains(&self, actor: ExactActorRef) -> bool {
        let Ok(index) = self.carrier.validate_ref(self.continuity, actor.0) else {
            return false;
        };
        match &self.carrier.slots[index] {
            Slot::Occupied {
                generation,
                committed,
                ..
            } => *committed && *generation == actor.0.actor_local_generation.0,
            Slot::CreatureOccupied {
                generation, health, ..
            } => *generation == actor.0.actor_local_generation.0 && *health > 0,
            Slot::VacantReusable { .. } | Slot::Exhausted { .. } => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActorState(u64);

/// The binding a position is valid under. In production the Channel's fixed
/// Content pin (#935) holds the full binding once; each slot keeps only this
/// compact per-runtime reference to it (the measured 192-byte slot footprint
/// is unchanged): the frame-binding and map-revision digest prefixes and the
/// activation sequence. Test fixtures use synthetic values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PreProductionPositionContext {
    world_id: WorldId,
    channel_id: ChannelId,
    scope_generation: ScopeOwnershipGeneration,
    coordinate_frame_marker: u64,
    map_revision_marker: u64,
    content_generation_marker: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LocalPosition {
    x: i32,
    y: i32,
    floor: i16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VersionedPosition {
    actor_local_id: ActorLocalId,
    actor_local_generation: ActorLocalGeneration,
    context: PreProductionPositionContext,
    position: LocalPosition,
    revision: u64,
}

/// A value snapshot, never an authority token. Compare-commit revalidates
/// actor, independently current owner, context and revision at the write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PositionSnapshot {
    actor_ref: ActorRef,
    version: VersionedPosition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Slot {
    VacantReusable {
        generation: u64,
        next_free: Option<u32>,
    },
    Occupied {
        generation: u64,
        actor: ActorState,
        /// Present only for the production player binding; fixture actors use None.
        game_session_id: Option<GameSessionId>,
        /// False while capacity is reserved but durable fresh admission is unresolved.
        committed: bool,
        position: Option<VersionedPosition>,
    },
    CreatureOccupied {
        generation: u64,
        actor: ActorState,
        position: Option<VersionedPosition>,
        target_identity: Arc<[u8]>,
        health: i64,
        committed: Option<OwnerCommitRecord>,
    },
    Exhausted {
        generation: u64,
    },
}

#[derive(Debug, PartialEq, Eq)]
struct ChannelActorCarrier {
    world_id: WorldId,
    channel_id: ChannelId,
    scope_generation: ScopeOwnershipGeneration,
    slots: Box<[Slot]>,
    free_head: Option<u32>,
    has_creature: bool,
    corpse_projection: Option<RuntimeCorpseProjection>,
}

/// The exact active Content generation a Channel runtime is created with (#935 "Activation issuer
/// and current pin"): scope World, activation sequence, pair digests and the source-qualified
/// frame binding digest. In production it is minted only by the Content activation path from an
/// activated native entry generation; it is fixed for the runtime's lifetime.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ChannelContentPin {
    world_id: WorldId,
    activation_sequence: u64,
    server_artifact_digest: [u8; 32],
    client_artifact_digest: [u8; 32],
    frame_binding_digest: [u8; 32],
    map_revision_digest: [u8; 32],
    /// The first-entry start cell the active generation attests as Walkable.
    entry_start: LocalPosition,
}

impl ChannelContentPin {
    pub(crate) const fn from_activation(
        world_id: WorldId,
        activation_sequence: u64,
        server_artifact_digest: [u8; 32],
        client_artifact_digest: [u8; 32],
        frame_binding_digest: [u8; 32],
        map_revision_digest: [u8; 32],
        entry_start: (i32, i32, i16),
    ) -> Self {
        Self {
            world_id,
            activation_sequence,
            server_artifact_digest,
            client_artifact_digest,
            frame_binding_digest,
            map_revision_digest,
            entry_start: LocalPosition {
                x: entry_start.0,
                y: entry_start.1,
                floor: entry_start.2,
            },
        }
    }

    /// Synthetic pin for runtime tests that do not exercise Content activation.
    #[cfg(test)]
    pub(crate) const fn test(world_id: WorldId) -> Self {
        Self::from_activation(world_id, 1, [1; 32], [2; 32], [3; 32], [4; 32], (0, 0, 0))
    }

    pub(crate) const fn world_id(&self) -> WorldId {
        self.world_id
    }

    pub(crate) const fn activation_sequence(&self) -> u64 {
        self.activation_sequence
    }

    pub(crate) const fn server_artifact_digest(&self) -> [u8; 32] {
        self.server_artifact_digest
    }

    pub(crate) const fn client_artifact_digest(&self) -> [u8; 32] {
        self.client_artifact_digest
    }

    pub(crate) const fn frame_binding_digest(&self) -> [u8; 32] {
        self.frame_binding_digest
    }
}

/// Outcome of a first-entry position initialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FirstEntryPosition {
    /// This call wrote the start position.
    Initialized(MovementPositionSnapshot),
    /// The same initialization had already completed; nothing was written.
    Reconciled(MovementPositionSnapshot),
}

/// The first composed Channel runtime. The fixed-slot carrier is the only actor
/// resource: no session map or second index is introduced.
#[derive(Debug)]
pub(crate) struct ChannelRuntimeV1 {
    binding: ChannelRuntimeAssignmentBinding,
    continuity: NamespaceContinuityGuard,
    carrier: ChannelActorCarrier,
    content: ChannelContentPin,
}

impl ChannelRuntimeV1 {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_committed_assignment(
        world_id: WorldId,
        channel_id: ChannelId,
        node_id: NodeId,
        node_registration_revision: u64,
        ownership_generation: u64,
        source_revision: u64,
        decision_identity: &str,
        explicit_capacity: usize,
        content: ChannelContentPin,
    ) -> Result<Self, CarrierError> {
        if node_registration_revision == 0 || source_revision == 0 {
            return Err(CarrierError::InvalidAssignmentBinding);
        }
        if content.world_id != world_id {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        let decision_revision = decision_identity
            .strip_prefix("runtime-scope-assignment:")
            .and_then(|value| value.parse::<u64>().ok());
        if decision_revision != Some(source_revision) {
            return Err(CarrierError::InvalidAssignmentBinding);
        }
        let scope_generation = ScopeOwnershipGeneration::new(ownership_generation)
            .map_err(|_| CarrierError::InvalidAssignmentBinding)?;
        let grant = PreProductionContinuityGrant {
            world_id,
            channel_id,
            scope_generation,
        };
        let mut continuity = NamespaceContinuityGuard::from_pre_production_grant(grant);
        let carrier =
            ChannelActorCarrier::bootstrap_pre_production(&mut continuity, explicit_capacity)?;
        Ok(Self {
            binding: ChannelRuntimeAssignmentBinding {
                world_id,
                channel_id,
                node_id,
                node_registration_revision,
                scope_generation,
                source_revision,
            },
            continuity,
            carrier,
            content,
        })
    }

    pub(crate) const fn binding(&self) -> ChannelRuntimeAssignmentBinding {
        self.binding
    }

    pub(crate) const fn content_pin(&self) -> &ChannelContentPin {
        &self.content
    }

    /// Unactivated Movement proof: one exclusive borrow of the composed Channel owner.
    /// This grants neither initial position authority nor an owner scheduler.
    pub(crate) fn borrow_movement_position(&mut self) -> CurrentOwnerMovementPosition<'_> {
        self.carrier
            .current_owner_movement_position(&self.continuity)
    }

    /// Unactivated Combat proof: one exclusive borrow of the physical Channel
    /// owner. It grants no scheduler, production activation or corpse lifetime.
    pub(crate) fn borrow_combat_death(&mut self) -> CurrentOwnerCombatDeath<'_> {
        self.carrier.current_owner_combat_death(&self.continuity)
    }

    /// The compact position context of this runtime's fixed Content pin.
    fn pinned_position_context(&self) -> PreProductionPositionContext {
        let prefix = |digest: &[u8; 32]| {
            u64::from_be_bytes([
                digest[0], digest[1], digest[2], digest[3], digest[4], digest[5], digest[6],
                digest[7],
            ])
        };
        PreProductionPositionContext {
            world_id: self.binding.world_id,
            channel_id: self.binding.channel_id,
            scope_generation: self.binding.scope_generation,
            coordinate_frame_marker: prefix(&self.content.frame_binding_digest),
            map_revision_marker: prefix(&self.content.map_revision_digest),
            content_generation_marker: self.content.activation_sequence,
        }
    }

    /// #935 first-entry position initialization: the Channel owner writes the
    /// pinned generation's start cell for one committed, unpositioned player
    /// actor. The context comes only from this runtime's assignment binding and
    /// Content pin. An exact retry reconciles the completed initialization
    /// without another write (later Movement is not undone); a stale or
    /// recycled actor, or one positioned under another context, gets no write.
    pub(crate) fn initialize_first_entry_position(
        &mut self,
        actor: ExactActorRef,
    ) -> Result<FirstEntryPosition, CarrierError> {
        let context = self.pinned_position_context();
        match self.carrier.initialize_position(
            &self.continuity,
            actor.0,
            context,
            self.content.entry_start,
        ) {
            Ok(snapshot) => Ok(FirstEntryPosition::Initialized(MovementPositionSnapshot(
                snapshot,
            ))),
            Err(CarrierError::PositionAlreadyInitialized) => {
                let current = self.carrier.read_position(&self.continuity, actor.0)?;
                if current.version.context == context {
                    Ok(FirstEntryPosition::Reconciled(MovementPositionSnapshot(
                        current,
                    )))
                } else {
                    Err(CarrierError::PositionAlreadyInitialized)
                }
            }
            Err(error) => Err(error),
        }
    }

    /// Synthetic context is confined to tests. A committed session still goes through
    /// the actual runtime reservation and carrier position initialization checks.
    #[cfg(test)]
    pub(crate) fn initialize_movement_test_position(
        &mut self,
        actor: ExactActorRef,
        position: MovementLocalPosition,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        let context = PreProductionPositionContext {
            world_id: self.binding.world_id,
            channel_id: self.binding.channel_id,
            scope_generation: self.binding.scope_generation,
            coordinate_frame_marker: 11,
            map_revision_marker: 12,
            content_generation_marker: 13,
        };
        self.carrier
            .initialize_position(
                &self.continuity,
                actor.0,
                context,
                LocalPosition {
                    x: position.x,
                    y: position.y,
                    floor: position.floor,
                },
            )
            .map(MovementPositionSnapshot)
    }

    pub(crate) fn reserve_fresh_session(
        &mut self,
        game_session_id: GameSessionId,
    ) -> Result<PlayerActorReservation, CarrierError> {
        self.carrier
            .reserve_player(&self.continuity, game_session_id)
    }

    pub(crate) fn commit_fresh_session(
        &mut self,
        reservation: PlayerActorReservation,
    ) -> Result<ExactActorRef, CarrierError> {
        self.carrier
            .commit_reserved_player(&self.continuity, reservation)
    }

    pub(crate) fn rollback_definitely_uncommitted(
        &mut self,
        reservation: PlayerActorReservation,
    ) -> Result<(), CarrierError> {
        self.carrier
            .rollback_reserved_player(&self.continuity, reservation)
    }

    /// Exact-ref cleanup only. Callers still need an authoritative terminal
    /// GameSession fact; ordinary socket loss is not such a fact.
    pub(crate) fn remove_terminal_session(
        &mut self,
        game_session_id: GameSessionId,
        actor: ExactActorRef,
    ) -> Result<(), CarrierError> {
        self.carrier
            .remove_terminal_player(&self.continuity, game_session_id, actor)
    }

    /// Test-only census: (committed player actors, pending player reservations).
    #[cfg(test)]
    pub(crate) fn player_slot_counts(&self) -> (usize, usize) {
        self.carrier
            .slots
            .iter()
            .fold((0, 0), |(committed, pending), slot| match slot {
                Slot::Occupied {
                    game_session_id: Some(_),
                    committed: true,
                    ..
                } => (committed + 1, pending),
                Slot::Occupied {
                    game_session_id: Some(_),
                    committed: false,
                    ..
                } => (committed, pending + 1),
                _ => (committed, pending),
            })
    }

    /// Test-only census: committed player actors positioned at the pinned
    /// start cell under the pinned context by one initialization (revision 1).
    #[cfg(test)]
    pub(crate) fn players_positioned_at_entry_start(&self) -> usize {
        self.carrier
            .slots
            .iter()
            .filter(|slot| {
                matches!(slot, Slot::Occupied {
                    game_session_id: Some(_),
                    committed: true,
                    position: Some(version),
                    ..
                } if version.position == self.content.entry_start
                    && version.revision == 1
                    && version.context == self.pinned_position_context())
            })
            .count()
    }

    #[cfg(test)]
    fn contains_committed_session(
        &self,
        game_session_id: GameSessionId,
        actor: ExactActorRef,
    ) -> bool {
        self.carrier
            .contains_committed_player(&self.continuity, game_session_id, actor)
    }
}

impl ChannelActorCarrier {
    fn current_owner_combat_death<'a>(
        &'a mut self,
        continuity: &'a NamespaceContinuityGuard,
    ) -> CurrentOwnerCombatDeath<'a> {
        CurrentOwnerCombatDeath {
            carrier: self,
            continuity,
        }
    }

    fn current_owner_movement_position<'a>(
        &'a mut self,
        continuity: &'a NamespaceContinuityGuard,
    ) -> CurrentOwnerMovementPosition<'a> {
        CurrentOwnerMovementPosition {
            carrier: self,
            continuity,
        }
    }

    fn read_movement_position(
        &self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        let index = self.validate_ref(continuity, actor_ref)?;
        if matches!(&self.slots[index], Slot::CreatureOccupied { .. }) {
            return Err(CarrierError::MovementCreatureUnavailable);
        }
        self.read_position(continuity, actor_ref)
            .map(MovementPositionSnapshot)
    }

    fn commit_movement_position(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        expected: PositionSnapshot,
        next: LocalPosition,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        let index = self.validate_ref(continuity, expected.actor_ref)?;
        if matches!(&self.slots[index], Slot::CreatureOccupied { .. }) {
            return Err(CarrierError::MovementCreatureUnavailable);
        }
        // An exclusive carrier borrow prevents a slot replacement between this check and
        // the existing private exact-snapshot/revision compare-commit.
        self.compare_commit_position(continuity, expected, expected.version.context, next)
            .map(MovementPositionSnapshot)
    }

    fn current_owner_exact_commit<'a>(
        &'a mut self,
        continuity: &'a NamespaceContinuityGuard,
    ) -> CurrentOwnerExactActorCommit<'a> {
        CurrentOwnerExactActorCommit {
            carrier: self,
            continuity,
        }
    }

    fn current_owner_exact_lookup<'a>(
        &'a self,
        continuity: &'a NamespaceContinuityGuard,
    ) -> CurrentOwnerExactActorLookup<'a> {
        CurrentOwnerExactActorLookup {
            carrier: self,
            continuity,
        }
    }

    fn bootstrap_pre_production(
        continuity: &mut NamespaceContinuityGuard,
        explicit_capacity: usize,
    ) -> Result<Self, CarrierError> {
        if explicit_capacity == 0 {
            return Err(CarrierError::InvalidCapacity);
        }

        // Validate every identity/byte calculation before allocation or publication.
        u32::try_from(explicit_capacity).map_err(|_| CarrierError::CapacityArithmeticOverflow)?;
        explicit_capacity
            .checked_mul(size_of::<Slot>())
            .ok_or(CarrierError::CapacityArithmeticOverflow)?;

        // Reject replay before touching allocation while retaining the claim
        // commit until every fallible construction step has succeeded.
        continuity.ensure_current_generation_unclaimed()?;
        let slots = allocate_slots(explicit_capacity)?;

        // Claim only after every fallible construction step has succeeded.
        continuity.claim_current_generation()?;

        Ok(Self {
            world_id: continuity.world_id,
            channel_id: continuity.channel_id,
            scope_generation: continuity.current_generation,
            slots: slots.into_boxed_slice(),
            free_head: Some(0),
            has_creature: false,
            corpse_projection: None,
        })
    }

    fn admit(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor: ActorState,
    ) -> Result<ActorRef, CarrierError> {
        self.admit_inner(continuity, actor, None, true, None, false)
    }

    /// Explicit nonshipping HP fixture. Generic actor admission remains distinct.
    fn admit_creature(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor: ActorState,
        target_identity: &str,
        initial_health: i64,
    ) -> Result<ActorRef, CarrierError> {
        if initial_health <= 0 {
            return Err(CarrierError::InvalidCreatureHealth);
        }
        self.validate_current_continuity(continuity)?;
        if self.has_creature || self.corpse_projection.is_some() {
            return Err(CarrierError::CapacityExceeded);
        }
        if target_identity.is_empty()
            || target_identity.len() > MAX_OWNER_COMMIT_BINDING_BYTES
            || !target_identity.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'.' | b'_' | b'-' | b'/')
            })
        {
            return Err(CarrierError::InvalidCreatureTarget);
        }
        let target_identity = Arc::from(copy_bounded_binding(target_identity.as_bytes())?);
        self.admit_inner(
            continuity,
            actor,
            None,
            true,
            Some((initial_health, target_identity)),
            false,
        )
    }

    fn reserve_player(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        game_session_id: GameSessionId,
    ) -> Result<PlayerActorReservation, CarrierError> {
        let actor_ref = self.admit_inner(
            continuity,
            ActorState(0),
            Some(game_session_id),
            false,
            None,
            false,
        )?;
        Ok(PlayerActorReservation {
            game_session_id,
            actor_ref,
        })
    }

    fn commit_reserved_player(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        reservation: PlayerActorReservation,
    ) -> Result<ExactActorRef, CarrierError> {
        let index = self.validate_ref(continuity, reservation.actor_ref)?;
        match &mut self.slots[index] {
            Slot::Occupied {
                generation,
                game_session_id: Some(game_session_id),
                committed,
                ..
            } if *generation == reservation.actor_ref.actor_local_generation.0
                && *game_session_id == reservation.game_session_id
                && !*committed =>
            {
                *committed = true;
                Ok(ExactActorRef(reservation.actor_ref))
            }
            _ => Err(CarrierError::PlayerReservationMismatch),
        }
    }

    fn rollback_reserved_player(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        reservation: PlayerActorReservation,
    ) -> Result<(), CarrierError> {
        let index = self.validate_ref(continuity, reservation.actor_ref)?;
        match &self.slots[index] {
            Slot::Occupied {
                generation,
                game_session_id: Some(game_session_id),
                committed,
                ..
            } if *generation == reservation.actor_ref.actor_local_generation.0
                && *game_session_id == reservation.game_session_id
                && !*committed => {}
            _ => return Err(CarrierError::PlayerReservationMismatch),
        }
        self.remove(continuity, reservation.actor_ref).map(|_| ())
    }

    fn remove_terminal_player(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        game_session_id: GameSessionId,
        actor: ExactActorRef,
    ) -> Result<(), CarrierError> {
        let index = self.validate_ref(continuity, actor.0)?;
        match &self.slots[index] {
            Slot::Occupied {
                generation,
                game_session_id: Some(stored_session),
                committed,
                ..
            } if *generation == actor.0.actor_local_generation.0
                && *stored_session == game_session_id
                && *committed => {}
            _ => return Err(CarrierError::PlayerReservationMismatch),
        }
        self.remove(continuity, actor.0).map(|_| ())
    }

    fn contains_committed_player(
        &self,
        continuity: &NamespaceContinuityGuard,
        game_session_id: GameSessionId,
        actor: ExactActorRef,
    ) -> bool {
        let Ok(index) = self.validate_ref(continuity, actor.0) else {
            return false;
        };
        matches!(
            &self.slots[index],
            Slot::Occupied {
                generation,
                game_session_id: Some(stored_session),
                committed: true,
                ..
            } if *generation == actor.0.actor_local_generation.0
                && *stored_session == game_session_id
        )
    }

    fn admit_inner(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor: ActorState,
        game_session_id: Option<GameSessionId>,
        committed: bool,
        initial_health: Option<(i64, Arc<[u8]>)>,
        fail_after_selection: bool,
    ) -> Result<ActorRef, CarrierError> {
        // Current outer authority must be proven before slot selection or any
        // mutation, including terminal generation-exhaustion bookkeeping.
        self.validate_current_continuity(continuity)?;
        let Some(free_head) = self.free_head else {
            return Err(CarrierError::CapacityExceeded);
        };
        let index =
            usize::try_from(free_head).map_err(|_| CarrierError::CapacityArithmeticOverflow)?;

        let Slot::VacantReusable {
            generation,
            next_free,
        } = &self.slots[index]
        else {
            unreachable!("free-list head must name a reusable slot");
        };
        let generation = *generation;
        let next_free = *next_free;
        let Some(next_generation) = generation.checked_add(1) else {
            // Protected #541 exception: this is the sole failure that mutates state.
            self.slots[index] = Slot::Exhausted { generation };
            self.free_head = next_free;
            return Err(CarrierError::ActorGenerationExhausted);
        };
        let actor_local_id = index
            .checked_add(1)
            .and_then(|value| u32::try_from(value).ok())
            .map(ActorLocalId)
            .ok_or(CarrierError::CapacityArithmeticOverflow)?;
        let actor_ref = ActorRef {
            world_id: self.world_id,
            channel_id: self.channel_id,
            scope_generation: self.scope_generation,
            actor_local_id,
            actor_local_generation: ActorLocalGeneration(next_generation),
        };

        if fail_after_selection {
            return Err(CarrierError::InjectedAdmissionFailure);
        }
        let is_creature = initial_health.is_some();
        self.slots[index] = if let Some((health, target_identity)) = initial_health {
            Slot::CreatureOccupied {
                generation: next_generation,
                actor,
                position: None,
                target_identity,
                health,
                committed: None,
            }
        } else {
            Slot::Occupied {
                generation: next_generation,
                actor,
                game_session_id,
                committed,
                position: None,
            }
        };
        self.free_head = next_free;
        if is_creature {
            self.has_creature = true;
        }
        Ok(actor_ref)
    }

    fn lookup(
        &self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
    ) -> Result<&ActorState, CarrierError> {
        let index = self.validate_ref(continuity, actor_ref)?;
        match &self.slots[index] {
            Slot::Occupied {
                generation,
                actor,
                committed: true,
                ..
            } if *generation == actor_ref.actor_local_generation.0 => Ok(actor),
            Slot::CreatureOccupied {
                generation,
                actor,
                health,
                ..
            } if *generation == actor_ref.actor_local_generation.0 && *health > 0 => Ok(actor),
            Slot::CreatureOccupied {
                generation, health, ..
            } if *generation == actor_ref.actor_local_generation.0 && *health == 0 => {
                Err(CarrierError::CreatureNotActionable)
            }
            Slot::Occupied { .. }
            | Slot::CreatureOccupied { .. }
            | Slot::VacantReusable { .. }
            | Slot::Exhausted { .. } => Err(CarrierError::StaleActorGeneration),
        }
    }

    fn remove(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
    ) -> Result<ActorState, CarrierError> {
        let index = self.validate_ref(continuity, actor_ref)?;
        let (generation, actor, removed_creature) = match &self.slots[index] {
            Slot::Occupied {
                generation, actor, ..
            } if *generation == actor_ref.actor_local_generation.0 => (*generation, *actor, false),
            Slot::CreatureOccupied {
                generation, actor, ..
            } if *generation == actor_ref.actor_local_generation.0 => (*generation, *actor, true),
            Slot::Occupied { .. }
            | Slot::CreatureOccupied { .. }
            | Slot::VacantReusable { .. }
            | Slot::Exhausted { .. } => return Err(CarrierError::StaleActorGeneration),
        };
        let free_index =
            u32::try_from(index).map_err(|_| CarrierError::CapacityArithmeticOverflow)?;
        self.slots[index] = Slot::VacantReusable {
            generation,
            next_free: self.free_head,
        };
        self.free_head = Some(free_index);
        if removed_creature {
            self.has_creature = false;
        }
        Ok(actor)
    }

    /// One fixed-size receipt is retained in the creature's own slot. On
    /// identical replay the recorded transition is returned without mutation.
    /// All validation, checked arithmetic and bounded allocation precede the
    /// only slot replacement. Administrative removal drops the receipt with
    /// the slot and emits no death or corpse event.
    fn commit_creature_damage_inner(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        command: OwnerDamageCommand<'_>,
        fail_before_write: bool,
    ) -> Result<OwnerDamageResult, CarrierError> {
        let OwnerDamageCommand {
            target,
            occurrence,
            binding,
            damage,
        } = command;
        let index = self.validate_ref(continuity, actor_ref)?;
        if occurrence.is_empty() || binding.is_empty() {
            return Err(CarrierError::InvalidCommitBinding);
        }
        if occurrence.len() > MAX_OWNER_COMMIT_BINDING_BYTES
            || binding.len() > MAX_OWNER_COMMIT_BINDING_BYTES
        {
            return Err(CarrierError::CommitBindingTooLarge);
        }
        if !binding.starts_with(occurrence) || binding.get(occurrence.len()) != Some(&0) {
            return Err(CarrierError::InvalidCommitBinding);
        }
        if damage <= 0 {
            return Err(CarrierError::InvalidDamage);
        }
        let Slot::CreatureOccupied {
            generation,
            target_identity,
            health,
            committed,
            ..
        } = &self.slots[index]
        else {
            return Err(CarrierError::NotCreature);
        };
        if *generation != actor_ref.actor_local_generation.0 {
            return Err(CarrierError::StaleActorGeneration);
        }
        if target_identity.as_ref() != target {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        if let Some(prior) = committed {
            if prior.binding.split(|byte| *byte == 0).next() != Some(occurrence) {
                return Err(CarrierError::OccurrenceConflict);
            }
            if prior.binding.as_ref() != binding || prior.damage != damage {
                return Err(CarrierError::PlanConflict);
            }
            return Ok(OwnerDamageResult {
                applied: false,
                ..prior.result
            });
        }
        if *health == 0 {
            return Err(CarrierError::CreatureNotActionable);
        }
        let next = health
            .checked_sub(damage)
            .ok_or(CarrierError::DamageOverflow)?
            .max(0);
        let result = OwnerDamageResult {
            applied: true,
            health_before: *health,
            health_after: next,
        };
        let binding = copy_bounded_binding(binding)?;
        let receipt = OwnerCommitRecord {
            binding,
            damage,
            result,
        };
        if fail_before_write {
            return Err(CarrierError::InjectedCommitFailure);
        }
        // Recheck current authority and local generation at the mutation
        // boundary, independent of Ability's earlier resolution snapshot.
        let write_index = self.validate_ref(continuity, actor_ref)?;
        if write_index != index {
            return Err(CarrierError::StaleActorGeneration);
        }
        let Slot::CreatureOccupied {
            generation,
            actor,
            position,
            target_identity,
            health,
            committed,
        } = &self.slots[index]
        else {
            return Err(CarrierError::StaleActorGeneration);
        };
        if *generation != actor_ref.actor_local_generation.0
            || target_identity.as_ref() != target
            || *health != result.health_before
            || committed.is_some()
        {
            return Err(CarrierError::StaleActorGeneration);
        }
        let (generation, actor, position, target_identity) =
            (*generation, *actor, *position, Arc::clone(target_identity));
        self.slots[index] = Slot::CreatureOccupied {
            generation,
            actor,
            position,
            target_identity,
            health: next,
            committed: Some(receipt),
        };
        Ok(result)
    }

    fn committed_lethal_receipt_inner(
        &self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
    ) -> Result<CommittedLethalReceipt, CarrierError> {
        let index = self.validate_ref(continuity, actor_ref)?;
        if let Some(existing) = self
            .corpse_projection
            .as_ref()
            .filter(|projection| projection.occurrence.actor == ExactActorRef(actor_ref))
        {
            return Ok(CommittedLethalReceipt {
                projection: RuntimeCorpseProjection {
                    occurrence: CreatureDeathOccurrenceRef {
                        actor: existing.occurrence.actor,
                        commit_binding: copy_bounded_binding(&existing.occurrence.commit_binding)?,
                        damage: existing.occurrence.damage,
                        health_before: existing.occurrence.health_before,
                    },
                    position: existing.position,
                },
            });
        }
        let Slot::CreatureOccupied {
            generation,
            position,
            health,
            committed: Some(committed),
            ..
        } = &self.slots[index]
        else {
            return Err(CarrierError::CommittedLethalUnavailable);
        };
        if *generation != actor_ref.actor_local_generation.0 {
            return Err(CarrierError::StaleActorGeneration);
        }
        if *health != 0
            || !committed.result.applied
            || committed.result.health_before <= 0
            || committed.result.health_after != 0
        {
            return Err(CarrierError::CommittedLethalUnavailable);
        }
        let position = position.ok_or(CarrierError::PositionUnavailable)?;
        Ok(CommittedLethalReceipt {
            projection: RuntimeCorpseProjection {
                occurrence: CreatureDeathOccurrenceRef {
                    actor: ExactActorRef(actor_ref),
                    commit_binding: copy_bounded_binding(&committed.binding)?,
                    damage: committed.damage,
                    health_before: committed.result.health_before,
                },
                position,
            },
        })
    }

    fn validate_lethal_receipt(
        &self,
        continuity: &NamespaceContinuityGuard,
        receipt: &CommittedLethalReceipt,
    ) -> Result<(), CarrierError> {
        let projection = &receipt.projection;
        let actor_ref = projection.occurrence.actor.0;
        let index = self.validate_ref(continuity, actor_ref)?;
        let Slot::CreatureOccupied {
            generation,
            position: Some(position),
            health,
            committed: Some(committed),
            ..
        } = &self.slots[index]
        else {
            return Err(CarrierError::StaleActorGeneration);
        };
        if *generation != actor_ref.actor_local_generation.0 {
            return Err(CarrierError::StaleActorGeneration);
        }
        if *health != 0
            || !committed.result.applied
            || committed.result.health_before <= 0
            || committed.result.health_after != 0
            || committed.binding.as_ref() != projection.occurrence.commit_binding.as_ref()
            || committed.damage != projection.occurrence.damage
            || committed.result.health_before != projection.occurrence.health_before
            || *position != projection.position
        {
            return Err(CarrierError::CorpseReceiptMismatch);
        }
        Ok(())
    }

    fn project_committed_lethal_inner(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        receipt: CommittedLethalReceipt,
        fail_before_write: bool,
        fail_after_write: bool,
    ) -> Result<&RuntimeCorpseProjection, CarrierError> {
        self.validate_ref(continuity, receipt.projection.occurrence.actor.0)?;
        if self.corpse_projection.is_some() {
            return self.existing_corpse_projection(&receipt);
        }
        self.validate_lethal_receipt(continuity, &receipt)?;
        if fail_before_write {
            return Err(CarrierError::InjectedCorpseProjectionFailure);
        }
        let projected = receipt.projection;
        self.corpse_projection = Some(projected);
        if fail_after_write {
            return Err(CarrierError::InjectedCorpseResponseFailure);
        }
        self.corpse_projection
            .as_ref()
            .ok_or(CarrierError::CorpseProjectionConflict)
    }

    fn existing_corpse_projection(
        &self,
        receipt: &CommittedLethalReceipt,
    ) -> Result<&RuntimeCorpseProjection, CarrierError> {
        let existing = self
            .corpse_projection
            .as_ref()
            .ok_or(CarrierError::CorpseProjectionConflict)?;
        if existing == &receipt.projection {
            Ok(existing)
        } else {
            Err(CarrierError::CorpseProjectionConflict)
        }
    }

    fn validate_position_context(
        &self,
        context: PreProductionPositionContext,
    ) -> Result<(), CarrierError> {
        if context.world_id != self.world_id
            || context.channel_id != self.channel_id
            || context.scope_generation != self.scope_generation
            || context.coordinate_frame_marker == 0
            || context.map_revision_marker == 0
            || context.content_generation_marker == 0
        {
            return Err(CarrierError::InvalidPreProductionPositionContext);
        }
        Ok(())
    }

    /// Private preproduction-only initial binding; existing actor admission
    /// cannot silently invent a position or a Content activation context.
    fn initialize_position(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        context: PreProductionPositionContext,
        position: LocalPosition,
    ) -> Result<PositionSnapshot, CarrierError> {
        let index = self.validate_ref(continuity, actor_ref)?;
        self.validate_position_context(context)?;
        match &mut self.slots[index] {
            Slot::CreatureOccupied {
                generation,
                health: 0,
                ..
            } if *generation == actor_ref.actor_local_generation.0 => {
                Err(CarrierError::CreatureNotActionable)
            }
            Slot::Occupied {
                generation,
                committed: true,
                position: stored @ None,
                ..
            }
            | Slot::CreatureOccupied {
                generation,
                position: stored @ None,
                health: 1..,
                ..
            } if *generation == actor_ref.actor_local_generation.0 => {
                let version = VersionedPosition {
                    actor_local_id: actor_ref.actor_local_id,
                    actor_local_generation: actor_ref.actor_local_generation,
                    context,
                    position,
                    revision: 1,
                };
                *stored = Some(version);
                Ok(PositionSnapshot { actor_ref, version })
            }
            Slot::Occupied {
                generation,
                committed: true,
                position: Some(_),
                ..
            }
            | Slot::CreatureOccupied {
                generation,
                position: Some(_),
                ..
            } if *generation == actor_ref.actor_local_generation.0 => {
                Err(CarrierError::PositionAlreadyInitialized)
            }
            _ => Err(CarrierError::StaleActorGeneration),
        }
    }

    /// One direct occupied-slot read under the independently current owner.
    fn read_position(
        &self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
    ) -> Result<PositionSnapshot, CarrierError> {
        let index = self.validate_ref(continuity, actor_ref)?;
        match &self.slots[index] {
            Slot::Occupied {
                generation,
                committed: true,
                position: Some(version),
                ..
            }
            | Slot::CreatureOccupied {
                generation,
                position: Some(version),
                ..
            } if *generation == actor_ref.actor_local_generation.0 => {
                if version.actor_local_id != actor_ref.actor_local_id
                    || version.actor_local_generation != actor_ref.actor_local_generation
                {
                    return Err(CarrierError::PositionSnapshotMismatch);
                }
                Ok(PositionSnapshot {
                    actor_ref,
                    version: *version,
                })
            }
            Slot::Occupied {
                generation,
                committed: true,
                position: None,
                ..
            }
            | Slot::CreatureOccupied {
                generation,
                position: None,
                ..
            } if *generation == actor_ref.actor_local_generation.0 => {
                Err(CarrierError::PositionUnavailable)
            }
            _ => Err(CarrierError::StaleActorGeneration),
        }
    }

    /// Compare and commit exactly one replacement in the same actor slot.
    /// All fallible checks run before the sole mutation and no alternate store
    /// or movement timing authority participates.
    fn compare_commit_position(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        expected: PositionSnapshot,
        next_context: PreProductionPositionContext,
        next_position: LocalPosition,
    ) -> Result<PositionSnapshot, CarrierError> {
        let index = self.validate_ref(continuity, expected.actor_ref)?;
        if matches!(
            &self.slots[index],
            Slot::CreatureOccupied {
                generation,
                health: 0,
                ..
            } if *generation == expected.actor_ref.actor_local_generation.0
        ) {
            return Err(CarrierError::CreatureNotActionable);
        }
        self.validate_position_context(next_context)?;
        if next_context != expected.version.context {
            return Err(CarrierError::PositionContextMismatch);
        }
        let (generation, current) = match &self.slots[index] {
            Slot::Occupied {
                generation,
                committed: true,
                position: Some(current),
                ..
            }
            | Slot::CreatureOccupied {
                generation,
                position: Some(current),
                ..
            } => (*generation, *current),
            _ => return Err(CarrierError::PositionSnapshotMismatch),
        };
        if generation != expected.actor_ref.actor_local_generation.0
            || current.actor_local_id != expected.actor_ref.actor_local_id
            || current.actor_local_generation != expected.actor_ref.actor_local_generation
            || current != expected.version
        {
            return Err(CarrierError::PositionSnapshotMismatch);
        }
        let revision = current
            .revision
            .checked_add(1)
            .ok_or(CarrierError::PositionRevisionExhausted)?;
        let version = VersionedPosition {
            actor_local_id: expected.actor_ref.actor_local_id,
            actor_local_generation: expected.actor_ref.actor_local_generation,
            context: next_context,
            position: next_position,
            revision,
        };
        match &mut self.slots[index] {
            Slot::Occupied { position, .. } | Slot::CreatureOccupied { position, .. } => {
                *position = Some(version)
            }
            _ => unreachable!("validated occupied slot"),
        }
        Ok(PositionSnapshot {
            actor_ref: expected.actor_ref,
            version,
        })
    }

    fn validate_ref(
        &self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
    ) -> Result<usize, CarrierError> {
        self.validate_current_continuity(continuity)?;
        if actor_ref.world_id != continuity.world_id
            || actor_ref.channel_id != continuity.channel_id
            || actor_ref.scope_generation != continuity.current_generation
            || actor_ref.world_id != self.world_id
            || actor_ref.channel_id != self.channel_id
            || actor_ref.scope_generation != self.scope_generation
        {
            return Err(CarrierError::WrongScope);
        }
        let index = actor_ref
            .actor_local_id
            .0
            .checked_sub(1)
            .and_then(|value| usize::try_from(value).ok())
            .ok_or(CarrierError::InvalidActorIdentity)?;
        if index >= self.slots.len() || actor_ref.actor_local_generation.0 == 0 {
            return Err(CarrierError::InvalidActorIdentity);
        }
        Ok(index)
    }

    fn validate_current_continuity(
        &self,
        continuity: &NamespaceContinuityGuard,
    ) -> Result<(), CarrierError> {
        if continuity.world_id != self.world_id
            || continuity.channel_id != self.channel_id
            || continuity.current_generation != self.scope_generation
        {
            return Err(CarrierError::WrongScope);
        }
        Ok(())
    }
}

fn copy_bounded_binding(bytes: &[u8]) -> Result<Box<[u8]>, CarrierError> {
    let mut owned = Vec::new();
    owned
        .try_reserve_exact(bytes.len())
        .map_err(|_| CarrierError::CommitAllocationFailed)?;
    owned.extend_from_slice(bytes);
    Ok(owned.into_boxed_slice())
}

fn allocate_slots(explicit_capacity: usize) -> Result<Vec<Slot>, CarrierError> {
    #[cfg(test)]
    if explicit_capacity == TEST_ALLOCATION_FAILURE_CAPACITY {
        return Err(CarrierError::AllocationFailed);
    }

    let mut slots = Vec::new();
    slots
        .try_reserve_exact(explicit_capacity)
        .map_err(|_| CarrierError::AllocationFailed)?;
    for index in 0..explicit_capacity {
        let next_free = if index + 1 < explicit_capacity {
            Some(u32::try_from(index + 1).map_err(|_| CarrierError::CapacityArithmeticOverflow)?)
        } else {
            None
        };
        slots.push(Slot::VacantReusable {
            generation: 0,
            next_free,
        });
    }
    Ok(slots)
}

#[cfg(test)]
const TEST_ALLOCATION_FAILURE_CAPACITY: usize = u32::MAX as usize;

/// Foundation-only factory. Path-included Foundation integration crates do not import Content;
/// the Movement library tests supply real Content claims outside this module.
#[cfg(test)]
pub(crate) struct MovementActorFixture {
    owner: NamespaceContinuityGuard,
    carrier: ChannelActorCarrier,
    actor: ExactActorRef,
}

#[cfg(test)]
impl MovementActorFixture {
    pub(crate) fn new(
        position: MovementLocalPosition,
        creature: bool,
    ) -> Result<Self, CarrierError> {
        fn uuid(seed: u8) -> [u8; 16] {
            let mut bytes = [seed; 16];
            bytes[6] = 0x70;
            bytes[8] = 0x80;
            bytes
        }
        let grant = PreProductionContinuityGrant {
            world_id: WorldId::decode(&uuid(10)).map_err(|_| CarrierError::WrongScope)?,
            channel_id: ChannelId::decode(&uuid(11)).map_err(|_| CarrierError::WrongScope)?,
            scope_generation: ScopeOwnershipGeneration::new(1)
                .map_err(|_| CarrierError::WrongScope)?,
        };
        let mut owner = NamespaceContinuityGuard::from_pre_production_grant(grant);
        let mut carrier = ChannelActorCarrier::bootstrap_pre_production(&mut owner, 1)?;
        let actor = if creature {
            carrier.admit_creature(&owner, ActorState(1), "engineering:creature", 20)?
        } else {
            carrier.admit(&owner, ActorState(1))?
        };
        let context = PreProductionPositionContext {
            world_id: owner.world_id,
            channel_id: owner.channel_id,
            scope_generation: owner.current_generation,
            coordinate_frame_marker: 11,
            map_revision_marker: 12,
            content_generation_marker: 13,
        };
        carrier.initialize_position(
            &owner,
            actor,
            context,
            LocalPosition {
                x: position.x,
                y: position.y,
                floor: position.floor,
            },
        )?;
        Ok(Self {
            owner,
            carrier,
            actor: ExactActorRef(actor),
        })
    }

    pub(crate) const fn world_id(&self) -> WorldId {
        self.owner.world_id
    }
    pub(crate) const fn actor(&self) -> ExactActorRef {
        self.actor
    }

    pub(crate) fn current_position(&self) -> Result<MovementPositionSnapshot, CarrierError> {
        self.carrier
            .read_movement_position(&self.owner, self.actor.0)
    }

    pub(crate) fn raw_position(&self) -> Result<(MovementLocalPosition, u64), CarrierError> {
        let version = self
            .carrier
            .read_position(&self.owner, self.actor.0)?
            .version;
        Ok((
            MovementLocalPosition {
                x: version.position.x,
                y: version.position.y,
                floor: version.position.floor,
            },
            version.revision,
        ))
    }

    pub(crate) fn borrow_position(&mut self) -> CurrentOwnerMovementPosition<'_> {
        self.carrier.current_owner_movement_position(&self.owner)
    }

    pub(crate) fn wrong_context(&self) -> Result<MovementPositionContext, CarrierError> {
        let mut context = self
            .carrier
            .read_position(&self.owner, self.actor.0)?
            .version
            .context;
        context.coordinate_frame_marker += 1;
        Ok(MovementPositionContext(context))
    }

    pub(crate) fn intervening_write(&mut self) -> Result<(), CarrierError> {
        let expected = self.carrier.read_position(&self.owner, self.actor.0)?;
        self.carrier.compare_commit_position(
            &self.owner,
            expected,
            expected.version.context,
            expected.version.position,
        )?;
        Ok(())
    }

    pub(crate) fn recycle(&mut self) -> Result<(), CarrierError> {
        let old = self.carrier.read_position(&self.owner, self.actor.0)?;
        self.carrier.remove(&self.owner, self.actor.0)?;
        let actor = self.carrier.admit(&self.owner, ActorState(2))?;
        self.carrier.initialize_position(
            &self.owner,
            actor,
            old.version.context,
            old.version.position,
        )?;
        self.actor = ExactActorRef(actor);
        Ok(())
    }

    pub(crate) fn become_creature(&mut self) -> Result<(), CarrierError> {
        let old = self.carrier.read_position(&self.owner, self.actor.0)?;
        self.carrier.remove(&self.owner, self.actor.0)?;
        let actor =
            self.carrier
                .admit_creature(&self.owner, ActorState(3), "engineering:creature", 20)?;
        self.carrier.initialize_position(
            &self.owner,
            actor,
            old.version.context,
            old.version.position,
        )?;
        self.actor = ExactActorRef(actor);
        Ok(())
    }

    pub(crate) fn advance_owner(&mut self) -> Result<(), CarrierError> {
        self.owner.advance(PreProductionContinuityGrant {
            world_id: self.owner.world_id,
            channel_id: self.owner.channel_id,
            scope_generation: ScopeOwnershipGeneration::new(2)
                .map_err(|_| CarrierError::WrongScope)?,
        })
    }

    pub(crate) fn exhaust_position_revision(&mut self) -> Result<(), CarrierError> {
        let index = self.carrier.validate_ref(&self.owner, self.actor.0)?;
        if let Slot::Occupied {
            position: Some(version),
            ..
        } = &mut self.carrier.slots[index]
        {
            version.revision = u64::MAX;
            Ok(())
        } else {
            Err(CarrierError::MovementCreatureUnavailable)
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
#[path = "ability_exact_actor_resolution_tests.rs"]
mod ability_exact_actor_resolution_tests;

#[cfg(test)]
#[allow(clippy::expect_used)]
#[path = "channel_owner_ability_commit_tests.rs"]
mod channel_owner_ability_commit_tests;

#[cfg(test)]
#[allow(clippy::expect_used)]
#[path = "channel_owner_combat_death_tests.rs"]
mod channel_owner_combat_death_tests;

#[cfg(test)]
#[allow(clippy::expect_used)]
#[path = "channel_actor_position_tests.rs"]
mod channel_actor_position_tests;

#[cfg(test)]
#[allow(clippy::expect_used)]
#[path = "movement_static_kernel_structural_tests.rs"]
mod movement_static_kernel_structural_tests;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn uuid_v7(raw: u64) -> [u8; 16] {
        let mut value = [0; 16];
        value[8..].copy_from_slice(&raw.to_be_bytes());
        value[6] = 0x70;
        value[8] = (value[8] & 0x3f) | 0x80;
        value
    }

    fn grant(seed: u64, generation: u64) -> PreProductionContinuityGrant {
        PreProductionContinuityGrant {
            world_id: WorldId::decode(&uuid_v7(seed)).expect("valid WorldId fixture"),
            channel_id: ChannelId::decode(&uuid_v7(seed + 1)).expect("valid ChannelId fixture"),
            scope_generation: ScopeOwnershipGeneration::new(generation)
                .expect("non-zero generation fixture"),
        }
    }

    fn continuity(seed: u64, generation: u64) -> NamespaceContinuityGuard {
        NamespaceContinuityGuard::from_pre_production_grant(grant(seed, generation))
    }

    fn carrier(capacity: usize) -> (NamespaceContinuityGuard, ChannelActorCarrier) {
        let mut continuity = continuity(10, 1);
        let carrier = ChannelActorCarrier::bootstrap_pre_production(&mut continuity, capacity)
            .expect("valid explicit test capacity");
        (continuity, carrier)
    }

    fn prove_boundary(capacity: usize) {
        let (continuity, mut carrier) = carrier(capacity);
        let mut refs = Vec::new();
        for value in 0..capacity {
            refs.push(
                carrier
                    .admit(&continuity, ActorState(value as u64))
                    .expect("M succeeds"),
            );
        }
        let before = carrier.slots.clone();
        assert_eq!(
            carrier.admit(&continuity, ActorState(99)),
            Err(CarrierError::CapacityExceeded)
        );
        assert_eq!(carrier.slots, before);
        for actor_ref in refs {
            assert!(carrier.lookup(&continuity, actor_ref).is_ok());
        }
    }

    fn session(raw: u64) -> GameSessionId {
        GameSessionId::decode(&uuid_v7(raw)).expect("valid GameSessionId fixture")
    }

    fn node(raw: u64) -> NodeId {
        NodeId::decode(&uuid_v7(raw)).expect("valid NodeId fixture")
    }

    fn runtime(capacity: usize) -> ChannelRuntimeV1 {
        ChannelRuntimeV1::from_committed_assignment(
            WorldId::decode(&uuid_v7(20)).expect("world"),
            ChannelId::decode(&uuid_v7(21)).expect("channel"),
            node(22),
            7,
            3,
            11,
            "runtime-scope-assignment:11",
            capacity,
            ChannelContentPin::test(WorldId::decode(&uuid_v7(20)).expect("world")),
        )
        .expect("runtime")
    }

    #[test]
    fn composed_runtime_binds_assignment_and_reserves_before_commit() {
        let mut runtime = runtime(1);
        let binding = runtime.binding();
        assert!(binding.matches_committed_assignment(
            binding.world_id(),
            binding.channel_id(),
            node(22),
            7,
            3,
            11,
            "runtime-scope-assignment:11",
        ));
        assert!(!binding.matches_committed_assignment(
            binding.world_id(),
            binding.channel_id(),
            node(22),
            7,
            4,
            12,
            "runtime-scope-assignment:12",
        ));

        let first_session = session(30);
        let reservation = runtime
            .reserve_fresh_session(first_session)
            .expect("M reservation succeeds");
        assert_eq!(
            runtime.reserve_fresh_session(session(31)),
            Err(CarrierError::CapacityExceeded)
        );
        let actor = runtime
            .commit_fresh_session(reservation)
            .expect("durable success commits actor");
        assert!(runtime.contains_committed_session(first_session, actor));
        assert_eq!(
            runtime.remove_terminal_session(session(31), actor),
            Err(CarrierError::PlayerReservationMismatch)
        );
        runtime
            .remove_terminal_session(first_session, actor)
            .expect("authoritative terminal cleanup");
        assert!(!runtime.contains_committed_session(first_session, actor));
    }

    #[test]
    fn definitely_uncommitted_reservation_rolls_back_but_committed_does_not() {
        let mut runtime = runtime(1);
        let first_session = session(40);
        let reservation = runtime
            .reserve_fresh_session(first_session)
            .expect("reserve");
        runtime
            .rollback_definitely_uncommitted(reservation)
            .expect("definite noncommit rollback");
        let second = runtime
            .reserve_fresh_session(session(41))
            .expect("capacity restored");
        let actor = runtime.commit_fresh_session(second).expect("commit");
        assert_eq!(
            runtime.rollback_definitely_uncommitted(second),
            Err(CarrierError::PlayerReservationMismatch)
        );
        assert!(runtime.contains_committed_session(session(41), actor));
    }

    #[test]
    fn first_entry_position_is_written_once_and_retry_reconciles() {
        let mut runtime = runtime(2);
        let reservation = runtime.reserve_fresh_session(session(60)).expect("reserve");
        let actor = runtime.commit_fresh_session(reservation).expect("commit");
        let outcome = runtime
            .initialize_first_entry_position(actor)
            .expect("initialize");
        assert!(matches!(outcome, FirstEntryPosition::Initialized(_)));
        let first = runtime
            .borrow_movement_position()
            .read(actor)
            .expect("positioned");
        assert_eq!(
            first.position(),
            MovementLocalPosition {
                x: 0,
                y: 0,
                floor: 0
            }
        );
        assert_eq!(first.revision(), 1);
        assert_eq!(runtime.players_positioned_at_entry_start(), 1);
        // Exact retry: same actor generation, same pin -> no second write.
        assert_eq!(
            runtime.initialize_first_entry_position(actor),
            Ok(FirstEntryPosition::Reconciled(first))
        );
        assert_eq!(runtime.players_positioned_at_entry_start(), 1);
    }

    #[test]
    fn first_entry_position_refuses_other_context_and_stale_actor() {
        let mut runtime = runtime(2);
        // An actor positioned under another context never gets a replacement.
        let reservation = runtime.reserve_fresh_session(session(61)).expect("reserve");
        let other = runtime.commit_fresh_session(reservation).expect("commit");
        let synthetic = runtime
            .initialize_movement_test_position(
                other,
                MovementLocalPosition {
                    x: 1,
                    y: 0,
                    floor: 0,
                },
            )
            .expect("synthetic position");
        assert_eq!(
            runtime.initialize_first_entry_position(other),
            Err(CarrierError::PositionAlreadyInitialized)
        );
        assert_eq!(
            runtime.borrow_movement_position().read(other),
            Ok(synthetic)
        );
        // A removed (stale) actor reference gets no write.
        let reservation = runtime.reserve_fresh_session(session(62)).expect("reserve");
        let stale = runtime.commit_fresh_session(reservation).expect("commit");
        runtime
            .remove_terminal_session(session(62), stale)
            .expect("terminal cleanup");
        assert!(runtime.initialize_first_entry_position(stale).is_err());
        assert_eq!(runtime.players_positioned_at_entry_start(), 0);
    }

    #[test]
    fn first_entry_position_refuses_wrong_scope_and_recycled_reference() {
        // A reference from another Channel runtime is refused without a write.
        let mut runtime = runtime(1);
        let mut other = ChannelRuntimeV1::from_committed_assignment(
            WorldId::decode(&uuid_v7(70)).expect("world"),
            ChannelId::decode(&uuid_v7(71)).expect("channel"),
            node(72),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            1,
            ChannelContentPin::test(WorldId::decode(&uuid_v7(70)).expect("world")),
        )
        .expect("other runtime");
        let reservation = other.reserve_fresh_session(session(73)).expect("reserve");
        let foreign = other.commit_fresh_session(reservation).expect("commit");
        assert!(runtime.initialize_first_entry_position(foreign).is_err());
        assert_eq!(runtime.players_positioned_at_entry_start(), 0);

        // A recycled slot: the old reference is refused; the new actor starts
        // unpositioned and gets its own first write.
        let reservation = runtime.reserve_fresh_session(session(74)).expect("reserve");
        let old = runtime.commit_fresh_session(reservation).expect("commit");
        runtime
            .remove_terminal_session(session(74), old)
            .expect("terminal cleanup");
        let reservation = runtime.reserve_fresh_session(session(75)).expect("reserve");
        let recycled = runtime.commit_fresh_session(reservation).expect("commit");
        assert!(runtime.initialize_first_entry_position(old).is_err());
        assert!(matches!(
            runtime.initialize_first_entry_position(recycled),
            Ok(FirstEntryPosition::Initialized(_))
        ));
        assert_eq!(runtime.players_positioned_at_entry_start(), 1);
    }

    #[test]
    fn runtime_rejects_invalid_assignment_identity_and_zero_provenance() {
        let world = WorldId::decode(&uuid_v7(50)).expect("world");
        let channel = ChannelId::decode(&uuid_v7(51)).expect("channel");
        assert!(matches!(
            ChannelRuntimeV1::from_committed_assignment(
                world,
                channel,
                node(52),
                0,
                1,
                1,
                "runtime-scope-assignment:1",
                1,
                ChannelContentPin::test(world),
            ),
            Err(CarrierError::InvalidAssignmentBinding)
        ));
        assert!(matches!(
            ChannelRuntimeV1::from_committed_assignment(
                world,
                channel,
                node(52),
                1,
                1,
                2,
                "runtime-scope-assignment:1",
                1,
                ChannelContentPin::test(world),
            ),
            Err(CarrierError::InvalidAssignmentBinding)
        ));
        let other_world = WorldId::decode(&uuid_v7(53)).expect("other world");
        assert!(matches!(
            ChannelRuntimeV1::from_committed_assignment(
                world,
                channel,
                node(52),
                1,
                1,
                1,
                "runtime-scope-assignment:1",
                1,
                ChannelContentPin::test(other_world),
            ),
            Err(CarrierError::ContentPinWorldMismatch)
        ));
        let runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            channel,
            node(52),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            1,
            ChannelContentPin::test(world),
        )
        .expect("pinned runtime");
        assert_eq!(runtime.content_pin(), &ChannelContentPin::test(world));
        assert_eq!(
            size_of::<Slot>(),
            192,
            "session binding must stay inside the already measured fixed-slot footprint"
        );
    }

    #[test]
    fn multiple_explicit_fixture_bounds_enforce_m_and_m_plus_one() {
        prove_boundary(1);
        prove_boundary(2);
        prove_boundary(4);
    }

    #[test]
    fn construction_rejects_zero_and_checked_overflow() {
        assert_eq!(
            ChannelActorCarrier::bootstrap_pre_production(&mut continuity(10, 1), 0),
            Err(CarrierError::InvalidCapacity)
        );
        assert_eq!(
            ChannelActorCarrier::bootstrap_pre_production(&mut continuity(10, 1), usize::MAX),
            Err(CarrierError::CapacityArithmeticOverflow)
        );
    }

    #[test]
    fn exact_lookup_rejects_cross_scope_and_invalid_identity() {
        let (continuity, mut carrier) = carrier(1);
        let actor_ref = carrier.admit(&continuity, ActorState(7)).expect("admit");
        assert_eq!(carrier.lookup(&continuity, actor_ref), Ok(&ActorState(7)));

        let mut wrong_world = actor_ref;
        wrong_world.world_id = grant(20, 1).world_id;
        assert_eq!(
            carrier.lookup(&continuity, wrong_world),
            Err(CarrierError::WrongScope)
        );
        let mut wrong_channel = actor_ref;
        wrong_channel.channel_id = grant(20, 1).channel_id;
        assert_eq!(
            carrier.lookup(&continuity, wrong_channel),
            Err(CarrierError::WrongScope)
        );
        let mut wrong_generation = actor_ref;
        wrong_generation.scope_generation = ScopeOwnershipGeneration::new(2).expect("valid");
        assert_eq!(
            carrier.lookup(&continuity, wrong_generation),
            Err(CarrierError::WrongScope)
        );
        let mut missing = actor_ref;
        missing.actor_local_id = ActorLocalId(2);
        assert_eq!(
            carrier.lookup(&continuity, missing),
            Err(CarrierError::InvalidActorIdentity)
        );
    }

    #[test]
    fn removal_retains_generation_and_reuse_stales_old_reference() {
        let (continuity, mut carrier) = carrier(1);
        let first = carrier
            .admit(&continuity, ActorState(1))
            .expect("first admit");
        assert_eq!(carrier.remove(&continuity, first), Ok(ActorState(1)));
        assert_eq!(
            carrier.lookup(&continuity, first),
            Err(CarrierError::StaleActorGeneration)
        );
        let second = carrier.admit(&continuity, ActorState(2)).expect("reuse");
        assert_eq!(second.actor_local_id, first.actor_local_id);
        assert_eq!(second.actor_local_generation.0, 2);
        assert_eq!(
            carrier.lookup(&continuity, first),
            Err(CarrierError::StaleActorGeneration)
        );
    }

    #[test]
    fn multiple_removed_holes_reuse_in_lifo_removal_order() {
        let (continuity, mut carrier) = carrier(4);
        let refs = (0..4)
            .map(|value| {
                carrier
                    .admit(&continuity, ActorState(value))
                    .expect("initial admit")
            })
            .collect::<Vec<_>>();

        assert_eq!(carrier.remove(&continuity, refs[1]), Ok(ActorState(1)));
        assert_eq!(carrier.remove(&continuity, refs[3]), Ok(ActorState(3)));

        let first_reuse = carrier
            .admit(&continuity, ActorState(10))
            .expect("first recycled admit");
        let second_reuse = carrier
            .admit(&continuity, ActorState(11))
            .expect("second recycled admit");

        assert_eq!(first_reuse.actor_local_id, refs[3].actor_local_id);
        assert_eq!(second_reuse.actor_local_id, refs[1].actor_local_id);
        assert_eq!(first_reuse.actor_local_generation.0, 2);
        assert_eq!(second_reuse.actor_local_generation.0, 2);
    }

    #[test]
    fn representable_post_selection_failure_rolls_back_everything() {
        let (continuity, mut carrier) = carrier(2);
        let first = carrier
            .admit(&continuity, ActorState(1))
            .expect("unrelated actor");
        let before = carrier.slots.clone();
        let free_head_before = carrier.free_head;
        assert_eq!(
            carrier.admit_inner(&continuity, ActorState(2), None, true, None, true),
            Err(CarrierError::InjectedAdmissionFailure)
        );
        assert_eq!(carrier.slots, before);
        assert_eq!(carrier.free_head, free_head_before);
        assert_eq!(carrier.lookup(&continuity, first), Ok(&ActorState(1)));
    }

    #[test]
    fn exhausted_reuse_marks_only_selected_slot_and_never_reselects_it() {
        let (continuity, mut carrier) = carrier(2);
        assert!(matches!(carrier.slots[0], Slot::VacantReusable { .. }));
        if let Slot::VacantReusable { generation, .. } = &mut carrier.slots[0] {
            *generation = u64::MAX;
        }
        let unrelated = carrier.slots[1].clone();
        assert_eq!(
            carrier.admit(&continuity, ActorState(1)),
            Err(CarrierError::ActorGenerationExhausted)
        );
        assert_eq!(
            carrier.slots[0],
            Slot::Exhausted {
                generation: u64::MAX
            }
        );
        assert_eq!(carrier.slots[1], unrelated);
        let admitted = carrier
            .admit(&continuity, ActorState(2))
            .expect("skip exhausted slot");
        assert_eq!(admitted.actor_local_id, ActorLocalId(2));
    }

    #[test]
    fn one_creature_marker_tracks_successful_admit_and_remove_only() {
        let (continuity, mut carrier) = carrier(2);
        assert!(!carrier.has_creature);

        let creature = carrier
            .admit_creature(&continuity, ActorState(1), "target:one", 20)
            .expect("first creature");
        assert!(carrier.has_creature);
        assert_eq!(
            carrier.admit_creature(&continuity, ActorState(2), "target:two", 20),
            Err(CarrierError::CapacityExceeded)
        );
        assert_eq!(carrier.remove(&continuity, creature), Ok(ActorState(1)));
        assert!(!carrier.has_creature);

        carrier
            .admit_creature(&continuity, ActorState(3), "target:three", 20)
            .expect("creature slot is reusable");
        assert!(carrier.has_creature);
    }

    #[test]
    fn carrier_loss_does_not_release_same_generation_claim() {
        let mut continuity = continuity(50, 9);
        let carrier = ChannelActorCarrier::bootstrap_pre_production(&mut continuity, 1)
            .expect("first bootstrap");
        drop(carrier);
        assert_eq!(
            ChannelActorCarrier::bootstrap_pre_production(&mut continuity, 1),
            Err(CarrierError::NamespaceAlreadyClaimed)
        );
    }

    #[test]
    fn claimed_namespace_rejects_before_the_allocation_sentinel() {
        let mut continuity = continuity(55, 3);
        let carrier = ChannelActorCarrier::bootstrap_pre_production(&mut continuity, 1)
            .expect("first bootstrap claims namespace");
        drop(carrier);

        // This huge capacity is arithmetically valid and would hit the test
        // allocation-failure sentinel if authority preflight did not win.
        assert_eq!(
            ChannelActorCarrier::bootstrap_pre_production(
                &mut continuity,
                TEST_ALLOCATION_FAILURE_CAPACITY,
            ),
            Err(CarrierError::NamespaceAlreadyClaimed)
        );
    }

    #[test]
    fn allocation_failure_does_not_consume_namespace_claim() {
        let mut continuity = continuity(56, 3);
        assert_eq!(
            ChannelActorCarrier::bootstrap_pre_production(
                &mut continuity,
                TEST_ALLOCATION_FAILURE_CAPACITY,
            ),
            Err(CarrierError::AllocationFailed)
        );
        assert!(!continuity.current_generation_claimed);

        let mut carrier = ChannelActorCarrier::bootstrap_pre_production(&mut continuity, 1)
            .expect("retry after allocation failure");
        assert!(continuity.current_generation_claimed);
        assert!(carrier.admit(&continuity, ActorState(8)).is_ok());
    }

    #[test]
    fn advancing_live_continuity_immediately_fences_all_old_operations() {
        let mut continuity = continuity(60, 4);
        let mut carrier =
            ChannelActorCarrier::bootstrap_pre_production(&mut continuity, 1).expect("bootstrap");
        let actor_ref = carrier.admit(&continuity, ActorState(7)).expect("admit");

        continuity.advance(grant(60, 5)).expect("strictly newer");
        let slots_before = carrier.slots.clone();

        assert_eq!(
            carrier.admit(&continuity, ActorState(9)),
            Err(CarrierError::WrongScope)
        );
        assert_eq!(
            carrier.lookup(&continuity, actor_ref),
            Err(CarrierError::WrongScope)
        );
        assert_eq!(
            carrier.remove(&continuity, actor_ref),
            Err(CarrierError::WrongScope)
        );
        assert_eq!(carrier.slots, slots_before);
    }

    #[test]
    fn stale_equal_and_cross_scope_advances_leave_guard_unchanged() {
        let mut continuity = continuity(70, 8);
        let before = (
            continuity.world_id,
            continuity.channel_id,
            continuity.current_generation,
            continuity.current_generation_claimed,
        );

        assert_eq!(
            continuity.advance(grant(70, 8)),
            Err(CarrierError::ContinuityGenerationNotNewer)
        );
        assert_eq!(
            continuity.advance(grant(70, 7)),
            Err(CarrierError::ContinuityGenerationNotNewer)
        );
        assert_eq!(
            continuity.advance(grant(80, 9)),
            Err(CarrierError::WrongScope)
        );
        assert_eq!(
            (
                continuity.world_id,
                continuity.channel_id,
                continuity.current_generation,
                continuity.current_generation_claimed,
            ),
            before
        );
    }

    #[test]
    fn strictly_newer_advance_permits_one_fresh_namespace_and_keeps_old_fenced() {
        let mut continuity = continuity(90, 2);
        let mut old_carrier = ChannelActorCarrier::bootstrap_pre_production(&mut continuity, 1)
            .expect("old bootstrap");
        let old_ref = old_carrier
            .admit(&continuity, ActorState(1))
            .expect("old admit");

        continuity.advance(grant(90, 3)).expect("advance");
        let mut new_carrier = ChannelActorCarrier::bootstrap_pre_production(&mut continuity, 1)
            .expect("fresh generation bootstrap");

        assert_eq!(new_carrier.scope_generation.get(), 3);
        let new_ref = new_carrier
            .admit(&continuity, ActorState(2))
            .expect("fresh carrier admits under live continuity");
        assert_eq!(new_carrier.lookup(&continuity, new_ref), Ok(&ActorState(2)));
        assert_eq!(
            old_carrier.lookup(&continuity, old_ref),
            Err(CarrierError::WrongScope)
        );
        assert_eq!(
            ChannelActorCarrier::bootstrap_pre_production(&mut continuity, 1),
            Err(CarrierError::NamespaceAlreadyClaimed)
        );
    }
}
