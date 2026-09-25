//! Fixed-bound, pre-production-only Channel actor storage.
//!
//! This module is intentionally private and uncomposed. In particular, it does
//! not issue the continuity authority required to create a carrier.

use super::{ChannelId, ScopeOwnershipGeneration, WorldId};
use std::mem::size_of;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CarrierError {
    InvalidCapacity,
    CapacityArithmeticOverflow,
    AllocationFailed,
    CapacityExceeded,
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
    InvalidCreatureHealth,
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
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OwnerDamageResult {
    pub(crate) applied: bool,
    pub(crate) health_before: i64,
    pub(crate) health_after: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OwnerCommitRecord {
    binding: Box<[u8]>,
    damage: i64,
    result: OwnerDamageResult,
}

/// 4096 is the registered Ability plan bound; the owner still independently
/// checks the actual complete encoding before allocating its one receipt.
const MAX_OWNER_COMMIT_BINDING_BYTES: usize = 4_096;

impl CurrentOwnerExactActorCommit<'_> {
    pub(crate) fn commit_damage(
        &mut self,
        actor: ExactActorRef,
        occurrence: &[u8],
        binding: &[u8],
        damage: i64,
    ) -> Result<OwnerDamageResult, CarrierError> {
        self.carrier.commit_creature_damage_inner(
            self.continuity,
            actor.0,
            occurrence,
            binding,
            damage,
            false,
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
            Slot::Occupied { generation, .. } => *generation == actor.0.actor_local_generation.0,
            Slot::CreatureOccupied { generation, health, .. } => {
                *generation == actor.0.actor_local_generation.0 && *health > 0
            }
            Slot::VacantReusable { .. } | Slot::Exhausted { .. } => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActorState(u64);

/// These opaque numeric markers are fixture inputs, not Content/Reference
/// activation evidence. Production composition must provide its own accepted
/// context binding before this private carrier can be used by gameplay.
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
    },
    Occupied {
        generation: u64,
        actor: ActorState,
        position: Option<VersionedPosition>,
    },
    CreatureOccupied {
        generation: u64,
        actor: ActorState,
        position: Option<VersionedPosition>,
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
}

impl ChannelActorCarrier {
    fn current_owner_exact_commit<'a>(
        &'a mut self,
        continuity: &'a NamespaceContinuityGuard,
    ) -> CurrentOwnerExactActorCommit<'a> {
        CurrentOwnerExactActorCommit { carrier: self, continuity }
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
        })
    }

    fn admit(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor: ActorState,
    ) -> Result<ActorRef, CarrierError> {
        self.admit_inner(continuity, actor, None, false)
    }

    /// Explicit nonshipping HP fixture. Generic actor admission remains distinct.
    fn admit_creature(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor: ActorState,
        initial_health: i64,
    ) -> Result<ActorRef, CarrierError> {
        if initial_health <= 0 {
            return Err(CarrierError::InvalidCreatureHealth);
        }
        self.validate_current_continuity(continuity)?;
        if self.slots.iter().any(|slot| matches!(slot, Slot::CreatureOccupied { .. })) {
            return Err(CarrierError::CapacityExceeded);
        }
        self.admit_inner(continuity, actor, Some(initial_health), false)
    }

    fn admit_inner(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor: ActorState,
        initial_health: Option<i64>,
        fail_after_selection: bool,
    ) -> Result<ActorRef, CarrierError> {
        // Current outer authority must be proven before slot selection or any
        // mutation, including terminal generation-exhaustion bookkeeping.
        self.validate_current_continuity(continuity)?;
        let Some(index) = self
            .slots
            .iter()
            .position(|slot| matches!(slot, Slot::VacantReusable { .. }))
        else {
            return Err(CarrierError::CapacityExceeded);
        };

        let Slot::VacantReusable { generation } = &self.slots[index] else {
            unreachable!("selection accepts reusable slots only");
        };
        let generation = *generation;
        let Some(next_generation) = generation.checked_add(1) else {
            // Protected #541 exception: this is the sole failure that mutates state.
            self.slots[index] = Slot::Exhausted { generation };
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
        self.slots[index] = if let Some(health) = initial_health {
            Slot::CreatureOccupied {
                generation: next_generation,
                actor,
                position: None,
                health,
                committed: None,
            }
        } else {
            Slot::Occupied {
                generation: next_generation,
                actor,
                position: None,
            }
        };
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
                generation, actor, ..
            } | Slot::CreatureOccupied {
                generation, actor, ..
            } if *generation == actor_ref.actor_local_generation.0 => Ok(actor),
            Slot::Occupied { .. } | Slot::CreatureOccupied { .. } | Slot::VacantReusable { .. } | Slot::Exhausted { .. } => {
                Err(CarrierError::StaleActorGeneration)
            }
        }
    }

    fn remove(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
    ) -> Result<ActorState, CarrierError> {
        let index = self.validate_ref(continuity, actor_ref)?;
        match &self.slots[index] {
            Slot::Occupied {
                generation, actor, ..
            } | Slot::CreatureOccupied {
                generation, actor, ..
            } if *generation == actor_ref.actor_local_generation.0 => {
                let (generation, actor) = (*generation, *actor);
                self.slots[index] = Slot::VacantReusable { generation };
                Ok(actor)
            }
            Slot::Occupied { .. } | Slot::CreatureOccupied { .. } | Slot::VacantReusable { .. } | Slot::Exhausted { .. } => {
                Err(CarrierError::StaleActorGeneration)
            }
        }
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
        occurrence: &[u8],
        binding: &[u8],
        damage: i64,
        fail_before_write: bool,
    ) -> Result<OwnerDamageResult, CarrierError> {
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
        let Slot::CreatureOccupied { generation, health, committed, .. } = &self.slots[index] else {
            return Err(CarrierError::NotCreature);
        };
        if *generation != actor_ref.actor_local_generation.0 {
            return Err(CarrierError::StaleActorGeneration);
        }
        if let Some(prior) = committed {
            if prior.binding.split(|byte| *byte == 0).next() != Some(occurrence) {
                return Err(CarrierError::OccurrenceConflict);
            }
            if prior.binding.as_ref() != binding || prior.damage != damage {
                return Err(CarrierError::PlanConflict);
            }
            return Ok(OwnerDamageResult { applied: false, ..prior.result });
        }
        if *health == 0 {
            return Err(CarrierError::CreatureNotActionable);
        }
        let next = health.checked_sub(damage).ok_or(CarrierError::DamageOverflow)?.max(0);
        let result = OwnerDamageResult {
            applied: true,
            health_before: *health,
            health_after: next,
        };
        let binding = copy_bounded_binding(binding)?;
        let receipt = OwnerCommitRecord { binding, damage, result };
        if fail_before_write {
            return Err(CarrierError::InjectedCommitFailure);
        }
        // Recheck current authority and local generation at the mutation
        // boundary, independent of Ability's earlier resolution snapshot.
        let write_index = self.validate_ref(continuity, actor_ref)?;
        if write_index != index {
            return Err(CarrierError::StaleActorGeneration);
        }
        let Slot::CreatureOccupied { generation, actor, position, health, committed } = &self.slots[index] else {
            return Err(CarrierError::StaleActorGeneration);
        };
        if *generation != actor_ref.actor_local_generation.0 || *health != result.health_before || committed.is_some() {
            return Err(CarrierError::StaleActorGeneration);
        }
        let (generation, actor, position) = (*generation, *actor, *position);
        self.slots[index] = Slot::CreatureOccupied {
            generation,
            actor,
            position,
            health: next,
            committed: Some(receipt),
        };
        Ok(result)
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
            Slot::Occupied {
                generation,
                position: stored @ None,
                ..
            } | Slot::CreatureOccupied {
                generation,
                position: stored @ None,
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
                position: Some(_),
                ..
            } | Slot::CreatureOccupied {
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
                position: Some(version),
                ..
            } | Slot::CreatureOccupied {
                generation,
                position: Some(version),
                ..
            } if *generation == actor_ref.actor_local_generation.0 => {
                if version.actor_local_id != actor_ref.actor_local_id
                    || version.actor_local_generation != actor_ref.actor_local_generation
                {
                    return Err(CarrierError::PositionSnapshotMismatch);
                }
                Ok(PositionSnapshot { actor_ref, version: *version })
            }
            Slot::Occupied {
                generation,
                position: None,
                ..
            } | Slot::CreatureOccupied {
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
        self.validate_position_context(next_context)?;
        if next_context != expected.version.context {
            return Err(CarrierError::PositionContextMismatch);
        }
        let (generation, current) = match &self.slots[index] {
            Slot::Occupied { generation, position: Some(current), .. }
            | Slot::CreatureOccupied { generation, position: Some(current), .. } => (*generation, *current),
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
            Slot::Occupied { position, .. } | Slot::CreatureOccupied { position, .. } => *position = Some(version),
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
    owned.try_reserve_exact(bytes.len()).map_err(|_| CarrierError::CommitAllocationFailed)?;
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
    slots.resize(explicit_capacity, Slot::VacantReusable { generation: 0 });
    Ok(slots)
}

#[cfg(test)]
const TEST_ALLOCATION_FAILURE_CAPACITY: usize = u32::MAX as usize;

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
    fn representable_post_selection_failure_rolls_back_everything() {
        let (continuity, mut carrier) = carrier(2);
        let first = carrier
            .admit(&continuity, ActorState(1))
            .expect("unrelated actor");
        let before = carrier.slots.clone();
        assert_eq!(
            carrier.admit_inner(&continuity, ActorState(2), None, true),
            Err(CarrierError::InjectedAdmissionFailure)
        );
        assert_eq!(carrier.slots, before);
        assert_eq!(carrier.lookup(&continuity, first), Ok(&ActorState(1)));
    }

    #[test]
    fn exhausted_reuse_marks_only_selected_slot_and_never_reselects_it() {
        let (continuity, mut carrier) = carrier(2);
        carrier.slots[0] = Slot::VacantReusable {
            generation: u64::MAX,
        };
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
