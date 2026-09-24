// The carrier model is exercised by this example target's focused test suite;
// the non-test binary emits only its reproducible physical evidence matrix.
#![cfg_attr(not(test), allow(dead_code))]

#[cfg(test)]
use oteryn_game_server::foundation::{
    ChannelId, RuntimeScopeRefV1, ScopeOwnershipGeneration, WorldId,
};
use serde_json::json;
use std::error::Error;
#[cfg(test)]
use std::fmt::{self, Display, Formatter};
use std::mem::size_of;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum ActorKind {
    Player = 1,
    Creature = 2,
    NpcSystem = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum SlotLifecycle {
    VacantReusable = 0,
    Occupied = 1,
    Exhausted = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ActorSlot {
    generation: u8,
    lifecycle: SlotLifecycle,
    kind: ActorKind,
    actionable: u8,
    x: i32,
    y: i32,
    z: i32,
}

impl ActorSlot {
    const fn vacant() -> Self {
        Self {
            generation: 0,
            lifecycle: SlotLifecycle::VacantReusable,
            kind: ActorKind::Player,
            actionable: 0,
            x: 0,
            y: 0,
            z: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct ActorLocalId(u32);

#[cfg(test)]
impl ActorLocalId {
    fn from_slot_index(index: usize) -> Result<Self, CarrierFailure> {
        let one_based = index
            .checked_add(1)
            .ok_or_else(CarrierFailure::arithmetic)?;
        let raw = u32::try_from(one_based).map_err(|_| CarrierFailure::arithmetic())?;
        Ok(Self(raw))
    }

    fn slot_index(self) -> Result<usize, CarrierFailure> {
        let raw = self
            .0
            .checked_sub(1)
            .ok_or_else(CarrierFailure::invalid_reference)?;
        usize::try_from(raw).map_err(|_| CarrierFailure::arithmetic())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct ActorLocalGeneration(u8);

#[cfg(test)]
impl ActorLocalGeneration {
    fn new(raw: u8) -> Result<Self, CarrierFailure> {
        if raw == 0 {
            return Err(CarrierFailure::invalid_reference());
        }
        Ok(Self(raw))
    }
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActorTargetRefPrototype {
    scope: RuntimeScopeRefV1,
    scope_generation: ScopeOwnershipGeneration,
    actor_local_id: ActorLocalId,
    actor_local_generation: ActorLocalGeneration,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LocalPosition {
    x: i32,
    y: i32,
    z: i32,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActorSeed {
    kind: ActorKind,
    actionable: bool,
    position: LocalPosition,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActorReadView {
    kind: ActorKind,
    actionable: bool,
    position: LocalPosition,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FailureCategory {
    CapacityExceeded,
    StaleGeneration,
    InvalidReference,
    AdmissionFailure,
    RecoveryBlocked,
    ArithmeticOverflow,
}

#[cfg(test)]
impl FailureCategory {
    const fn as_str(self) -> &'static str {
        match self {
            Self::CapacityExceeded => "CAPACITY_EXCEEDED",
            Self::StaleGeneration => "STALE_GENERATION",
            Self::InvalidReference => "INVALID_REFERENCE",
            Self::AdmissionFailure => "ADMISSION_FAILURE",
            Self::RecoveryBlocked => "RECOVERY_BLOCKED",
            Self::ArithmeticOverflow => "ARITHMETIC_OVERFLOW",
        }
    }
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FailureCode {
    ActorCapacityExceeded,
    ActorLocalGenerationExhausted,
    StaleScopeOrGeneration,
    StaleActorReference,
    InjectedPostSelectionFailure,
    SameGenerationReconstructionBlocked,
    OuterGenerationNotNewer,
    ArithmeticOverflow,
    NonChannelScope,
}

#[cfg(test)]
impl FailureCode {
    const fn as_str(self) -> &'static str {
        match self {
            Self::ActorCapacityExceeded => "ACTOR_CAPACITY_EXCEEDED",
            Self::ActorLocalGenerationExhausted => "ACTOR_LOCAL_GENERATION_EXHAUSTED",
            Self::StaleScopeOrGeneration => "STALE_SCOPE_OR_GENERATION",
            Self::StaleActorReference => "STALE_ACTOR_REFERENCE",
            Self::InjectedPostSelectionFailure => "INJECTED_POST_SELECTION_FAILURE",
            Self::SameGenerationReconstructionBlocked => "SAME_GENERATION_RECONSTRUCTION_BLOCKED",
            Self::OuterGenerationNotNewer => "OUTER_GENERATION_NOT_NEWER",
            Self::ArithmeticOverflow => "ARITHMETIC_OVERFLOW",
            Self::NonChannelScope => "NON_CHANNEL_SCOPE",
        }
    }
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CarrierFailure {
    code: FailureCode,
    category: FailureCategory,
    work_units: usize,
}

#[cfg(test)]
impl CarrierFailure {
    const fn new(code: FailureCode, category: FailureCategory, work_units: usize) -> Self {
        Self {
            code,
            category,
            work_units,
        }
    }

    const fn arithmetic() -> Self {
        Self::new(
            FailureCode::ArithmeticOverflow,
            FailureCategory::ArithmeticOverflow,
            0,
        )
    }

    const fn invalid_reference() -> Self {
        Self::new(
            FailureCode::StaleActorReference,
            FailureCategory::InvalidReference,
            0,
        )
    }
}

#[cfg(test)]
impl Display for CarrierFailure {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} / {} (work_units={})",
            self.code.as_str(),
            self.category.as_str(),
            self.work_units
        )
    }
}

#[cfg(test)]
impl Error for CarrierFailure {}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AdmissionFault {
    None,
    AfterGenerationSelection,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AdmissionSuccess {
    target: ActorTargetRefPrototype,
    insertion_work_units: usize,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LookupSuccess {
    actor: ActorReadView,
    direct_lookup_work_units: usize,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RemovalSuccess {
    removal_work_units: usize,
}

#[cfg(test)]
#[derive(Debug, PartialEq, Eq)]
struct ChannelActorCarrier<const M: usize> {
    scope: RuntimeScopeRefV1,
    scope_generation: ScopeOwnershipGeneration,
    slots: [ActorSlot; M],
}

// A rollback comparison is data, not a second authority-bearing carrier.
#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CarrierStateSnapshot<const M: usize> {
    scope: RuntimeScopeRefV1,
    scope_generation: ScopeOwnershipGeneration,
    slots: [ActorSlot; M],
}

#[cfg(test)]
impl<const M: usize> ChannelActorCarrier<M> {
    // Raw carrier materialization is deliberately private to the surviving
    // namespace-continuity authority below. Carrier loss alone cannot call it.
    fn materialize_after_namespace_claim(
        scope: RuntimeScopeRefV1,
        scope_generation: ScopeOwnershipGeneration,
    ) -> Result<Self, CarrierFailure> {
        if !matches!(scope, RuntimeScopeRefV1::Channel { .. }) || M == 0 {
            return Err(CarrierFailure::new(
                FailureCode::NonChannelScope,
                FailureCategory::InvalidReference,
                0,
            ));
        }
        let _ = u32::try_from(M).map_err(|_| CarrierFailure::arithmetic())?;
        checked_retained_slot_bytes(M)?;
        Ok(Self {
            scope,
            scope_generation,
            slots: [ActorSlot::vacant(); M],
        })
    }

    fn actor_ref_for(
        &self,
        index: usize,
        generation: u8,
    ) -> Result<ActorTargetRefPrototype, CarrierFailure> {
        Ok(ActorTargetRefPrototype {
            scope: self.scope,
            scope_generation: self.scope_generation,
            actor_local_id: ActorLocalId::from_slot_index(index)?,
            actor_local_generation: ActorLocalGeneration::new(generation)?,
        })
    }

    fn admit(
        &mut self,
        seed: ActorSeed,
        fault: AdmissionFault,
    ) -> Result<AdmissionSuccess, CarrierFailure> {
        let mut work_units = 0usize;
        for index in 0..M {
            work_units = work_units
                .checked_add(1)
                .ok_or_else(CarrierFailure::arithmetic)?;
            let slot = self.slots[index];
            if slot.lifecycle != SlotLifecycle::VacantReusable {
                continue;
            }

            let next_generation = if slot.generation == 0 {
                1
            } else {
                match slot.generation.checked_add(1) {
                    Some(value) => value,
                    None => {
                        self.slots[index].lifecycle = SlotLifecycle::Exhausted;
                        return Err(CarrierFailure::new(
                            FailureCode::ActorLocalGenerationExhausted,
                            FailureCategory::CapacityExceeded,
                            work_units,
                        ));
                    }
                }
            };

            let target = self.actor_ref_for(index, next_generation)?;
            if fault == AdmissionFault::AfterGenerationSelection {
                return Err(CarrierFailure::new(
                    FailureCode::InjectedPostSelectionFailure,
                    FailureCategory::AdmissionFailure,
                    work_units,
                ));
            }

            self.slots[index] = ActorSlot {
                generation: next_generation,
                lifecycle: SlotLifecycle::Occupied,
                kind: seed.kind,
                actionable: u8::from(seed.actionable),
                x: seed.position.x,
                y: seed.position.y,
                z: seed.position.z,
            };
            return Ok(AdmissionSuccess {
                target,
                insertion_work_units: work_units,
            });
        }

        Err(CarrierFailure::new(
            FailureCode::ActorCapacityExceeded,
            FailureCategory::CapacityExceeded,
            work_units,
        ))
    }

    fn verify_owner(
        &self,
        current: &NamespaceContinuityGuard,
        target: ActorTargetRefPrototype,
    ) -> Result<usize, CarrierFailure> {
        if current.scope != self.scope
            || current.generation != self.scope_generation
            || target.scope != self.scope
            || target.scope_generation != self.scope_generation
        {
            return Err(CarrierFailure::new(
                FailureCode::StaleScopeOrGeneration,
                FailureCategory::StaleGeneration,
                0,
            ));
        }

        let index = target.actor_local_id.slot_index()?;
        if index >= M {
            return Err(CarrierFailure::invalid_reference());
        }
        ActorLocalGeneration::new(target.actor_local_generation.0)?;
        Ok(index)
    }

    fn lookup(
        &self,
        current: &NamespaceContinuityGuard,
        target: ActorTargetRefPrototype,
    ) -> Result<LookupSuccess, CarrierFailure> {
        let index = self.verify_owner(current, target)?;
        let slot = self.slots[index];
        if slot.lifecycle != SlotLifecycle::Occupied {
            return Err(slot_reference_failure(slot, 1));
        }
        if slot.generation != target.actor_local_generation.0 {
            return Err(stale_actor_reference(1));
        }
        Ok(LookupSuccess {
            actor: ActorReadView {
                kind: slot.kind,
                actionable: slot.actionable != 0,
                position: LocalPosition {
                    x: slot.x,
                    y: slot.y,
                    z: slot.z,
                },
            },
            direct_lookup_work_units: 1,
        })
    }

    fn remove(
        &mut self,
        current: &NamespaceContinuityGuard,
        target: ActorTargetRefPrototype,
    ) -> Result<RemovalSuccess, CarrierFailure> {
        let index = self.verify_owner(current, target)?;
        let slot = self.slots[index];
        if slot.lifecycle != SlotLifecycle::Occupied {
            return Err(slot_reference_failure(slot, 1));
        }
        if slot.generation != target.actor_local_generation.0 {
            return Err(stale_actor_reference(1));
        }
        self.slots[index].lifecycle = SlotLifecycle::VacantReusable;
        self.slots[index].kind = ActorKind::Player;
        self.slots[index].actionable = 0;
        self.slots[index].x = 0;
        self.slots[index].y = 0;
        self.slots[index].z = 0;
        Ok(RemovalSuccess {
            removal_work_units: 1,
        })
    }

    const fn retained_generation_cells(&self) -> usize {
        M
    }

    const fn independent_retirement_history_entries(&self) -> usize {
        0
    }

    fn exhausted_slots(&self) -> usize {
        self.slots
            .iter()
            .filter(|slot| slot.lifecycle == SlotLifecycle::Exhausted)
            .count()
    }

    const fn snapshot(&self) -> CarrierStateSnapshot<M> {
        CarrierStateSnapshot {
            scope: self.scope,
            scope_generation: self.scope_generation,
            slots: self.slots,
        }
    }
}

#[cfg(test)]
const fn stale_actor_reference(work_units: usize) -> CarrierFailure {
    CarrierFailure::new(
        FailureCode::StaleActorReference,
        FailureCategory::StaleGeneration,
        work_units,
    )
}

#[cfg(test)]
fn slot_reference_failure(slot: ActorSlot, work_units: usize) -> CarrierFailure {
    if slot.lifecycle == SlotLifecycle::VacantReusable && slot.generation == 0 {
        CarrierFailure::new(
            FailureCode::StaleActorReference,
            FailureCategory::InvalidReference,
            work_units,
        )
    } else {
        stale_actor_reference(work_units)
    }
}

// This move-only value represents an already-authorized *newer* scope-owner
// transition. It intentionally has no constructor or issuer API.
#[cfg(test)]
#[derive(Debug)]
struct AuthorizedNewerScopeGenerationGrant {
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
}

// This state is outside carrier backing and deliberately neither Clone nor Copy.
#[cfg(test)]
#[derive(Debug, PartialEq, Eq)]
struct NamespaceContinuityGuard {
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
    namespace_initialized: bool,
}

#[cfg(test)]
impl NamespaceContinuityGuard {
    const fn scope(&self) -> RuntimeScopeRefV1 {
        self.scope
    }

    const fn generation(&self) -> ScopeOwnershipGeneration {
        self.generation
    }

    fn bootstrap<const M: usize>(&mut self) -> Result<ChannelActorCarrier<M>, CarrierFailure> {
        if self.namespace_initialized {
            return Err(CarrierFailure::new(
                FailureCode::SameGenerationReconstructionBlocked,
                FailureCategory::RecoveryBlocked,
                0,
            ));
        }
        let carrier =
            ChannelActorCarrier::materialize_after_namespace_claim(self.scope, self.generation)?;
        self.namespace_initialized = true;
        Ok(carrier)
    }

    fn apply_independently_authorized_newer_generation(
        &mut self,
        grant: AuthorizedNewerScopeGenerationGrant,
    ) -> Result<(), CarrierFailure> {
        if grant.scope != self.scope || grant.generation <= self.generation {
            return Err(CarrierFailure::new(
                FailureCode::OuterGenerationNotNewer,
                FailureCategory::StaleGeneration,
                0,
            ));
        }
        self.generation = grant.generation;
        self.namespace_initialized = false;
        Ok(())
    }
}

#[cfg(test)]
fn checked_retained_slot_bytes(configured_slots: usize) -> Result<usize, CarrierFailure> {
    configured_slots
        .checked_mul(size_of::<ActorSlot>())
        .ok_or_else(CarrierFailure::arithmetic)
}

#[cfg(test)]
fn uuid_v7(raw: u64) -> [u8; 16] {
    let mut out = [0_u8; 16];
    out[8..].copy_from_slice(&raw.to_be_bytes());
    out[6] = 0x70;
    out[8] = (out[8] & 0x3f) | 0x80;
    out
}

#[cfg(test)]
fn channel_scope(seed: u64) -> Result<RuntimeScopeRefV1, Box<dyn Error>> {
    let world = WorldId::decode(&uuid_v7(seed))?;
    let channel_seed = seed
        .checked_add(1)
        .ok_or_else(|| std::io::Error::other("seed overflow"))?;
    let channel = ChannelId::decode(&uuid_v7(channel_seed))?;
    Ok(RuntimeScopeRefV1::channel(world, channel))
}

fn physical_point<const M: usize>() -> Result<serde_json::Value, Box<dyn Error>> {
    let retained_bytes = M
        .checked_mul(size_of::<ActorSlot>())
        .ok_or_else(|| std::io::Error::other("retained slot byte overflow"))?;

    Ok(json!({
        "configured_actor_slots": M,
        "slot_or_record_size_bytes": size_of::<ActorSlot>(),
        "retained_slot_or_record_bytes": retained_bytes,
        "lookup_or_index_entries": M,
        "lookup_or_index_physically_same_as_slot_backing": true,
        "lookup_or_index_entry_size_bytes_if_distinct": 0,
        "lookup_or_index_retained_bytes_if_distinct": 0,
        "retained_generation_cells": M,
        "independent_retirement_history_entries": 0,
        "authority_dependent_operations_executed": false,
        "production_capacity_claim": false
    }))
}

fn main() -> Result<(), Box<dyn Error>> {
    let evidence = json!({
        "candidate": "CHANNEL_RUNTIME_ACTOR_CARRIER_V1_PROTOTYPE",
        "foundation_scope_type": "RuntimeScopeRefV1::Channel",
        "foundation_generation_type": "ScopeOwnershipGeneration",
        "actor_local_id_width_bytes": size_of::<ActorLocalId>(),
        "actor_local_generation_width_bytes": size_of::<ActorLocalGeneration>(),
        "actor_slot_width_bytes": size_of::<ActorSlot>(),
        "tested_points": [
            physical_point::<1>()?,
            physical_point::<2>()?,
            physical_point::<3>()?,
            physical_point::<4>()?
        ],
        "accepted_production_maximum_selected": false,
        "resource_registry_mutated": false,
        "production_capacity_claim": false
    });
    println!("{}", serde_json::to_string_pretty(&evidence)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generation(raw: u64) -> ScopeOwnershipGeneration {
        ScopeOwnershipGeneration::new(raw).expect("test generation is non-zero")
    }

    fn seed(kind: ActorKind, n: i32) -> ActorSeed {
        ActorSeed {
            kind,
            actionable: true,
            position: LocalPosition {
                x: n,
                y: n + 1,
                z: n + 2,
            },
        }
    }

    fn authority(seed_value: u64) -> NamespaceContinuityGuard {
        authority_for(
            channel_scope(seed_value).expect("valid test scope"),
            generation(1),
        )
    }

    fn newer_grant_for(
        scope: RuntimeScopeRefV1,
        generation: ScopeOwnershipGeneration,
    ) -> AuthorizedNewerScopeGenerationGrant {
        // Test-only injection of an already-authorized transition. There is no
        // runtime issuer/reissuer API in the prototype.
        AuthorizedNewerScopeGenerationGrant { scope, generation }
    }

    fn authority_for(
        scope: RuntimeScopeRefV1,
        generation: ScopeOwnershipGeneration,
    ) -> NamespaceContinuityGuard {
        // Test-only injection of the one surviving authority. This is not a
        // runtime constructor and does not model same-generation recovery.
        NamespaceContinuityGuard {
            scope,
            generation,
            namespace_initialized: false,
        }
    }

    fn prove_m_boundary<const M: usize>() {
        let mut owner = authority(100 + u64::try_from(M).expect("small M") * 10);
        let mut carrier = owner.bootstrap::<M>().expect("carrier bootstrap");
        let mut refs = Vec::new();
        let mut insertion_work = Vec::new();
        for index in 0..M {
            let index_i32 = i32::try_from(index).expect("tested M fits i32");
            let admitted = carrier
                .admit(seed(ActorKind::Player, index_i32), AdmissionFault::None)
                .expect("exact M admission");
            refs.push(admitted.target);
            insertion_work.push(admitted.insertion_work_units);
        }
        assert_eq!(refs.len(), M);
        assert_eq!(insertion_work.first().copied(), Some(1));
        assert_eq!(insertion_work.last().copied(), Some(M));

        let before = carrier.snapshot();
        let failure = carrier
            .admit(seed(ActorKind::Creature, 99), AdmissionFault::None)
            .expect_err("M+1 must reject");
        assert_eq!(failure.code, FailureCode::ActorCapacityExceeded);
        assert_eq!(failure.category, FailureCategory::CapacityExceeded);
        assert_eq!(failure.work_units, M);
        assert_eq!(carrier.snapshot(), before);
        assert_eq!(carrier.retained_generation_cells(), M);
        assert_eq!(carrier.independent_retirement_history_entries(), 0);
        for target in refs.iter().copied() {
            assert_eq!(
                carrier
                    .lookup(&owner, target)
                    .expect("existing actor")
                    .direct_lookup_work_units,
                1
            );
        }

        let last = *refs.last().expect("M is non-zero");
        assert_eq!(
            carrier
                .remove(&owner, last)
                .expect("remove boundary slot")
                .removal_work_units,
            1
        );
        let replacement = carrier
            .admit(seed(ActorKind::NpcSystem, 101), AdmissionFault::None)
            .expect("reinsert fragmented boundary slot");
        assert_eq!(replacement.insertion_work_units, M);
        assert_eq!(
            carrier
                .lookup(&owner, last)
                .expect_err("retired boundary ref stays stale")
                .code,
            FailureCode::StaleActorReference
        );
    }

    #[test]
    fn exact_m_and_m_plus_one_hold_for_every_tested_m() {
        prove_m_boundary::<1>();
        prove_m_boundary::<2>();
        prove_m_boundary::<3>();
        prove_m_boundary::<4>();
    }

    #[test]
    fn direct_lookup_requires_live_world_channel_and_generation_authority() {
        let mut owner = authority(200);
        let mut carrier = owner.bootstrap::<2>().expect("bootstrap");
        let admitted = carrier
            .admit(seed(ActorKind::Player, 1), AdmissionFault::None)
            .expect("admit");
        assert_eq!(
            carrier
                .lookup(&owner, admitted.target)
                .expect("lookup")
                .direct_lookup_work_units,
            1
        );

        let (world_id, channel_id) = match owner.scope() {
            RuntimeScopeRefV1::Channel {
                world_id,
                channel_id,
            } => (world_id, channel_id),
            RuntimeScopeRefV1::Instance { .. } => unreachable!("test authority is Channel"),
        };
        let other_world = WorldId::decode(&uuid_v7(300)).expect("other world");
        let other_channel = ChannelId::decode(&uuid_v7(301)).expect("other channel");

        let wrong_world_owner = authority_for(
            RuntimeScopeRefV1::channel(other_world, channel_id),
            owner.generation(),
        );
        assert_eq!(
            carrier
                .lookup(&wrong_world_owner, admitted.target)
                .expect_err("cross-world reject")
                .code,
            FailureCode::StaleScopeOrGeneration
        );

        let wrong_channel_owner = authority_for(
            RuntimeScopeRefV1::channel(world_id, other_channel),
            owner.generation(),
        );
        assert_eq!(
            carrier
                .lookup(&wrong_channel_owner, admitted.target)
                .expect_err("cross-channel reject")
                .code,
            FailureCode::StaleScopeOrGeneration
        );

        owner
            .apply_independently_authorized_newer_generation(newer_grant_for(
                owner.scope(),
                generation(2),
            ))
            .expect("new live owner generation");
        assert_eq!(
            carrier
                .lookup(&owner, admitted.target)
                .expect_err("old carrier rejected after live owner transition")
                .category,
            FailureCategory::StaleGeneration
        );
    }

    #[test]
    fn missing_and_vacant_actor_lookups_fail_closed() {
        let mut owner = authority(350);
        let mut carrier = owner.bootstrap::<2>().expect("bootstrap");
        let admitted = carrier
            .admit(seed(ActorKind::Player, 1), AdmissionFault::None)
            .expect("admit");

        let missing = ActorTargetRefPrototype {
            actor_local_id: ActorLocalId(3),
            ..admitted.target
        };
        assert_eq!(
            carrier
                .lookup(&owner, missing)
                .expect_err("out-of-range actor id must reject")
                .category,
            FailureCategory::InvalidReference
        );

        let never_used = ActorTargetRefPrototype {
            actor_local_id: ActorLocalId(2),
            ..admitted.target
        };
        assert_eq!(
            carrier
                .lookup(&owner, never_used)
                .expect_err("never-used vacant identity must reject")
                .category,
            FailureCategory::InvalidReference
        );

        carrier
            .remove(&owner, admitted.target)
            .expect("vacate admitted slot");
        assert_eq!(
            carrier
                .lookup(&owner, admitted.target)
                .expect_err("retired reference must be stale")
                .category,
            FailureCategory::StaleGeneration
        );
    }

    #[test]
    fn zero_actor_generation_is_invalid_and_preserves_carrier_state() {
        let mut owner = authority(375);
        let mut carrier = owner.bootstrap::<1>().expect("bootstrap");
        let admitted = carrier
            .admit(seed(ActorKind::Player, 1), AdmissionFault::None)
            .expect("admit");
        let before = carrier.snapshot();
        let malformed = ActorTargetRefPrototype {
            actor_local_generation: ActorLocalGeneration(0),
            ..admitted.target
        };

        let failure = carrier
            .lookup(&owner, malformed)
            .expect_err("zero generation must be invalid");
        assert_eq!(failure.code, FailureCode::StaleActorReference);
        assert_eq!(failure.category, FailureCategory::InvalidReference);
        assert_eq!(failure.category.as_str(), "INVALID_REFERENCE");
        assert_eq!(carrier.snapshot(), before);
        assert_eq!(
            carrier
                .lookup(&owner, admitted.target)
                .expect("valid target remains intact")
                .actor
                .kind,
            ActorKind::Player
        );
    }

    #[test]
    fn stale_reference_never_revives_after_reuse() {
        let mut owner = authority(400);
        let mut carrier = owner.bootstrap::<1>().expect("bootstrap");
        let first = carrier
            .admit(seed(ActorKind::Player, 1), AdmissionFault::None)
            .expect("first");
        assert_eq!(
            carrier
                .remove(&owner, first.target)
                .expect("remove")
                .removal_work_units,
            1
        );
        let retired_lookup = carrier
            .lookup(&owner, first.target)
            .expect_err("vacant old ref rejects");
        assert_eq!(retired_lookup.code, FailureCode::StaleActorReference);
        assert_eq!(retired_lookup.category, FailureCategory::StaleGeneration);
        let second = carrier
            .admit(seed(ActorKind::Creature, 2), AdmissionFault::None)
            .expect("reuse");
        assert!(second.target.actor_local_generation > first.target.actor_local_generation);
        let stale_lookup = carrier
            .lookup(&owner, first.target)
            .expect_err("old ref stale");
        assert_eq!(stale_lookup.code, FailureCode::StaleActorReference);
        assert_eq!(stale_lookup.category, FailureCategory::StaleGeneration);
        let stale_remove = carrier
            .remove(&owner, first.target)
            .expect_err("old ref cannot remove replacement");
        assert_eq!(stale_remove.code, FailureCode::StaleActorReference);
        assert_eq!(stale_remove.category, FailureCategory::StaleGeneration);
        assert_eq!(
            carrier
                .lookup(&owner, second.target)
                .expect("new ref")
                .actor
                .kind,
            ActorKind::Creature
        );
    }

    #[test]
    fn distinct_actor_ids_cannot_alias_one_generation_cell() {
        let mut owner = authority(500);
        let mut carrier = owner.bootstrap::<2>().expect("bootstrap");
        let a = carrier
            .admit(seed(ActorKind::Player, 10), AdmissionFault::None)
            .expect("a");
        let b = carrier
            .admit(seed(ActorKind::Creature, 20), AdmissionFault::None)
            .expect("b");
        carrier.remove(&owner, b.target).expect("remove b");
        let b2 = carrier
            .admit(seed(ActorKind::NpcSystem, 30), AdmissionFault::None)
            .expect("reuse b");

        let forged_a_to_b_generation = ActorTargetRefPrototype {
            actor_local_generation: b2.target.actor_local_generation,
            ..a.target
        };
        assert_eq!(
            carrier
                .lookup(&owner, forged_a_to_b_generation)
                .expect_err("id A cannot resolve B cell")
                .code,
            FailureCode::StaleActorReference
        );
        assert_eq!(
            carrier
                .lookup(&owner, a.target)
                .expect("a still resolves")
                .actor
                .kind,
            ActorKind::Player
        );
        assert_eq!(
            carrier
                .lookup(&owner, b2.target)
                .expect("b2 resolves")
                .actor
                .kind,
            ActorKind::NpcSystem
        );
    }

    #[test]
    fn post_selection_failure_rolls_back_complete_carrier_state() {
        let mut owner = authority(600);
        let mut carrier = owner.bootstrap::<2>().expect("bootstrap");
        let first = carrier
            .admit(seed(ActorKind::Player, 1), AdmissionFault::None)
            .expect("first");
        let _second = carrier
            .admit(seed(ActorKind::Creature, 2), AdmissionFault::None)
            .expect("second");
        carrier.remove(&owner, first.target).expect("remove first");
        let before = carrier.snapshot();
        let failure = carrier
            .admit(
                seed(ActorKind::NpcSystem, 3),
                AdmissionFault::AfterGenerationSelection,
            )
            .expect_err("injected failure");
        assert_eq!(failure.code, FailureCode::InjectedPostSelectionFailure);
        assert_eq!(carrier.snapshot(), before);
    }

    #[test]
    fn generation_exhaustion_is_terminal_atomic_and_classified_as_capacity() {
        let mut owner = authority(700);
        let mut carrier = owner.bootstrap::<3>().expect("bootstrap");
        let mut slot_zero_ref = carrier
            .admit(seed(ActorKind::Player, 1), AdmissionFault::None)
            .expect("slot0")
            .target;
        let slot_one_ref = carrier
            .admit(seed(ActorKind::Creature, 2), AdmissionFault::None)
            .expect("slot1")
            .target;

        for n in 1..u8::MAX {
            carrier.remove(&owner, slot_zero_ref).expect("retire slot0");
            slot_zero_ref = carrier
                .admit(seed(ActorKind::Player, i32::from(n)), AdmissionFault::None)
                .expect("reuse slot0")
                .target;
        }
        assert_eq!(slot_zero_ref.actor_local_generation.0, u8::MAX);
        carrier
            .remove(&owner, slot_zero_ref)
            .expect("vacate max generation");
        let before = carrier.snapshot();

        let failure = carrier
            .admit(seed(ActorKind::NpcSystem, 9), AdmissionFault::None)
            .expect_err("successor must exhaust");
        assert_eq!(failure.code, FailureCode::ActorLocalGenerationExhausted);
        assert_eq!(failure.code.as_str(), "ACTOR_LOCAL_GENERATION_EXHAUSTED");
        assert_eq!(failure.category, FailureCategory::CapacityExceeded);
        assert_eq!(failure.category.as_str(), "CAPACITY_EXCEEDED");
        assert_eq!(carrier.exhausted_slots(), 1);
        assert_eq!(carrier.slots[0].lifecycle, SlotLifecycle::Exhausted);
        assert_eq!(carrier.slots[0].generation, u8::MAX);
        assert_eq!(carrier.slots[1], before.slots[1]);
        assert_eq!(carrier.slots[2], before.slots[2]);
        assert_eq!(
            carrier
                .lookup(&owner, slot_one_ref)
                .expect("unrelated actor preserved")
                .actor
                .kind,
            ActorKind::Creature
        );

        let remaining_slot = carrier
            .admit(seed(ActorKind::NpcSystem, 10), AdmissionFault::None)
            .expect("remaining eligible slot admits");
        assert_eq!(remaining_slot.target.actor_local_id, ActorLocalId(3));
        assert_eq!(remaining_slot.target.actor_local_generation.0, 1);
        assert_eq!(carrier.slots[0].lifecycle, SlotLifecycle::Exhausted);
        assert_eq!(carrier.slots[0].generation, u8::MAX);
        assert_eq!(carrier.slots[1], before.slots[1]);
        assert_eq!(
            carrier
                .lookup(&owner, slot_one_ref)
                .expect("unrelated actor still preserved")
                .actor
                .kind,
            ActorKind::Creature
        );
        assert_eq!(
            carrier
                .lookup(&owner, remaining_slot.target)
                .expect("new target resolves")
                .actor
                .kind,
            ActorKind::NpcSystem
        );
        assert_eq!(carrier.retained_generation_cells(), 3);
        assert_eq!(carrier.independent_retirement_history_entries(), 0);
    }

    #[test]
    fn churn_across_every_slot_keeps_generation_state_exactly_m() {
        let mut owner = authority(800);
        let mut carrier = owner.bootstrap::<3>().expect("bootstrap");
        let mut refs = [
            carrier
                .admit(seed(ActorKind::Player, 1), AdmissionFault::None)
                .expect("0")
                .target,
            carrier
                .admit(seed(ActorKind::Creature, 2), AdmissionFault::None)
                .expect("1")
                .target,
            carrier
                .admit(seed(ActorKind::NpcSystem, 3), AdmissionFault::None)
                .expect("2")
                .target,
        ];
        for index in 0..3 {
            carrier.remove(&owner, refs[index]).expect("retire");
            let index_i32 = i32::try_from(index).expect("tested index fits i32");
            refs[index] = carrier
                .admit(
                    seed(ActorKind::Player, 10 + index_i32),
                    AdmissionFault::None,
                )
                .expect("reuse exact sole vacant slot")
                .target;
        }
        assert_eq!(carrier.retained_generation_cells(), 3);
        assert_eq!(carrier.independent_retirement_history_entries(), 0);
        assert!(
            refs.iter()
                .all(|target| target.actor_local_generation.0 == 2)
        );
    }

    #[test]
    fn insertion_work_is_measured_at_sparse_full_and_fragmented_boundaries() {
        let mut owner = authority(900);
        let mut carrier = owner.bootstrap::<4>().expect("bootstrap");
        let first = carrier
            .admit(seed(ActorKind::Player, 1), AdmissionFault::None)
            .expect("sparse first");
        assert_eq!(first.insertion_work_units, 1);
        let second = carrier
            .admit(seed(ActorKind::Player, 2), AdmissionFault::None)
            .expect("2");
        let third = carrier
            .admit(seed(ActorKind::Player, 3), AdmissionFault::None)
            .expect("3");
        let fourth = carrier
            .admit(seed(ActorKind::Player, 4), AdmissionFault::None)
            .expect("4");
        let full = carrier
            .admit(seed(ActorKind::Creature, 5), AdmissionFault::None)
            .expect_err("full");
        assert_eq!(full.work_units, 4);
        carrier.remove(&owner, fourth.target).expect("remove last");
        let fragmented = carrier
            .admit(seed(ActorKind::NpcSystem, 6), AdmissionFault::None)
            .expect("last slot only");
        assert_eq!(fragmented.insertion_work_units, 4);
        assert_eq!(
            carrier
                .lookup(&owner, second.target)
                .expect("lookup")
                .direct_lookup_work_units,
            1
        );
        assert_eq!(
            carrier
                .remove(&owner, third.target)
                .expect("remove")
                .removal_work_units,
            1
        );
    }

    #[test]
    fn carrier_loss_preserves_guard_and_blocks_same_generation_reconstruction() {
        let mut owner = authority(1_000);
        let mut old_carrier = owner.bootstrap::<1>().expect("first namespace");
        let old_ref = old_carrier
            .admit(seed(ActorKind::Player, 1), AdmissionFault::None)
            .expect("old actor")
            .target;
        assert!(old_carrier.lookup(&owner, old_ref).is_ok());

        // Carrier backing is lost, but the continuity authority survives.
        drop(old_carrier);
        assert_eq!(
            owner
                .bootstrap::<1>()
                .expect_err("same-generation rebuild blocked")
                .code,
            FailureCode::SameGenerationReconstructionBlocked
        );

        // There is no issuer/reissuer or raw-fact guard constructor in the
        // prototype. A fresh namespace can be established only by consuming an
        // already-authorized strictly newer outer-generation grant.
        owner
            .apply_independently_authorized_newer_generation(newer_grant_for(
                owner.scope(),
                generation(2),
            ))
            .expect("new owner generation");
        let new_carrier = owner.bootstrap::<1>().expect("new namespace allowed");
        assert_eq!(
            new_carrier
                .lookup(&owner, old_ref)
                .expect_err("new outer generation fences old ref")
                .category,
            FailureCategory::StaleGeneration
        );
    }

    #[test]
    fn actor_id_count_and_retained_byte_overflow_are_checked() {
        assert_eq!(size_of::<ActorSlot>(), 16);
        assert_eq!(size_of::<ActorLocalId>(), 4);
        assert_eq!(size_of::<ActorLocalGeneration>(), 1);
        assert_eq!(checked_retained_slot_bytes(1).expect("M1"), 16);
        assert_eq!(checked_retained_slot_bytes(2).expect("M2"), 32);
        assert_eq!(checked_retained_slot_bytes(3).expect("M3"), 48);
        assert_eq!(checked_retained_slot_bytes(4).expect("M4"), 64);

        let max_u32_index = usize::try_from(u32::MAX).expect("CI usize represents u32::MAX");
        assert_eq!(
            ActorLocalId::from_slot_index(max_u32_index)
                .expect_err("one-based actor id count/index overflow must reject")
                .code,
            FailureCode::ArithmeticOverflow
        );
        assert_eq!(
            checked_retained_slot_bytes(usize::MAX)
                .expect_err("byte overflow must reject")
                .code,
            FailureCode::ArithmeticOverflow
        );
    }
}
