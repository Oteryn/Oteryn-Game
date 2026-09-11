//! Fixed-bound, pre-production-only Channel actor storage.
//!
//! This module is intentionally private and uncomposed. In particular, it does
//! not issue the continuity authority required to create a carrier.

use super::{ChannelId, ScopeOwnershipGeneration, WorldId};
use std::mem::size_of;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CarrierError {
    InvalidCapacity,
    CapacityArithmeticOverflow,
    AllocationFailed,
    CapacityExceeded,
    WrongScope,
    InvalidActorIdentity,
    StaleActorGeneration,
    ActorGenerationExhausted,
    InjectedAdmissionFailure,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActorState(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Slot {
    VacantReusable { generation: u64 },
    Occupied { generation: u64, actor: ActorState },
    Exhausted { generation: u64 },
}

#[derive(Debug, PartialEq, Eq)]
struct ChannelActorCarrier {
    world_id: WorldId,
    channel_id: ChannelId,
    scope_generation: ScopeOwnershipGeneration,
    slots: Box<[Slot]>,
}

impl ChannelActorCarrier {
    fn bootstrap_pre_production(
        grant: PreProductionContinuityGrant,
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

        let mut slots = Vec::new();
        slots
            .try_reserve_exact(explicit_capacity)
            .map_err(|_| CarrierError::AllocationFailed)?;
        slots.resize(explicit_capacity, Slot::VacantReusable { generation: 0 });

        Ok(Self {
            world_id: grant.world_id,
            channel_id: grant.channel_id,
            scope_generation: grant.scope_generation,
            slots: slots.into_boxed_slice(),
        })
    }

    fn admit(&mut self, actor: ActorState) -> Result<ActorRef, CarrierError> {
        self.admit_inner(actor, false)
    }

    fn admit_inner(
        &mut self,
        actor: ActorState,
        fail_after_selection: bool,
    ) -> Result<ActorRef, CarrierError> {
        let Some(index) = self
            .slots
            .iter()
            .position(|slot| matches!(slot, Slot::VacantReusable { .. }))
        else {
            return Err(CarrierError::CapacityExceeded);
        };

        let Slot::VacantReusable { generation } = self.slots[index] else {
            unreachable!("selection accepts reusable slots only");
        };
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
        self.slots[index] = Slot::Occupied {
            generation: next_generation,
            actor,
        };
        Ok(actor_ref)
    }

    fn lookup(&self, actor_ref: ActorRef) -> Result<&ActorState, CarrierError> {
        let index = self.validate_ref(actor_ref)?;
        match &self.slots[index] {
            Slot::Occupied { generation, actor }
                if *generation == actor_ref.actor_local_generation.0 =>
            {
                Ok(actor)
            }
            Slot::Occupied { .. } | Slot::VacantReusable { .. } | Slot::Exhausted { .. } => {
                Err(CarrierError::StaleActorGeneration)
            }
        }
    }

    fn remove(&mut self, actor_ref: ActorRef) -> Result<ActorState, CarrierError> {
        let index = self.validate_ref(actor_ref)?;
        match self.slots[index] {
            Slot::Occupied { generation, actor }
                if generation == actor_ref.actor_local_generation.0 =>
            {
                self.slots[index] = Slot::VacantReusable { generation };
                Ok(actor)
            }
            Slot::Occupied { .. } | Slot::VacantReusable { .. } | Slot::Exhausted { .. } => {
                Err(CarrierError::StaleActorGeneration)
            }
        }
    }

    fn validate_ref(&self, actor_ref: ActorRef) -> Result<usize, CarrierError> {
        if actor_ref.world_id != self.world_id
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
}

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

    fn carrier(capacity: usize) -> ChannelActorCarrier {
        ChannelActorCarrier::bootstrap_pre_production(grant(10, 1), capacity)
            .expect("valid explicit test capacity")
    }

    fn prove_boundary(capacity: usize) {
        let mut carrier = carrier(capacity);
        let mut refs = Vec::new();
        for value in 0..capacity {
            refs.push(carrier.admit(ActorState(value as u64)).expect("M succeeds"));
        }
        let before = carrier.slots.clone();
        assert_eq!(
            carrier.admit(ActorState(99)),
            Err(CarrierError::CapacityExceeded)
        );
        assert_eq!(carrier.slots, before);
        for actor_ref in refs {
            assert!(carrier.lookup(actor_ref).is_ok());
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
            ChannelActorCarrier::bootstrap_pre_production(grant(10, 1), 0),
            Err(CarrierError::InvalidCapacity)
        );
        assert_eq!(
            ChannelActorCarrier::bootstrap_pre_production(grant(10, 1), usize::MAX),
            Err(CarrierError::CapacityArithmeticOverflow)
        );
    }

    #[test]
    fn exact_lookup_rejects_cross_scope_and_invalid_identity() {
        let mut carrier = carrier(1);
        let actor_ref = carrier.admit(ActorState(7)).expect("admit");
        assert_eq!(carrier.lookup(actor_ref), Ok(&ActorState(7)));

        let mut wrong_world = actor_ref;
        wrong_world.world_id = grant(20, 1).world_id;
        assert_eq!(carrier.lookup(wrong_world), Err(CarrierError::WrongScope));
        let mut wrong_channel = actor_ref;
        wrong_channel.channel_id = grant(20, 1).channel_id;
        assert_eq!(carrier.lookup(wrong_channel), Err(CarrierError::WrongScope));
        let mut wrong_generation = actor_ref;
        wrong_generation.scope_generation = ScopeOwnershipGeneration::new(2).expect("valid");
        assert_eq!(
            carrier.lookup(wrong_generation),
            Err(CarrierError::WrongScope)
        );
        let mut missing = actor_ref;
        missing.actor_local_id = ActorLocalId(2);
        assert_eq!(
            carrier.lookup(missing),
            Err(CarrierError::InvalidActorIdentity)
        );
    }

    #[test]
    fn removal_retains_generation_and_reuse_stales_old_reference() {
        let mut carrier = carrier(1);
        let first = carrier.admit(ActorState(1)).expect("first admit");
        assert_eq!(carrier.remove(first), Ok(ActorState(1)));
        assert_eq!(
            carrier.lookup(first),
            Err(CarrierError::StaleActorGeneration)
        );
        let second = carrier.admit(ActorState(2)).expect("reuse");
        assert_eq!(second.actor_local_id, first.actor_local_id);
        assert_eq!(second.actor_local_generation.0, 2);
        assert_eq!(
            carrier.lookup(first),
            Err(CarrierError::StaleActorGeneration)
        );
    }

    #[test]
    fn representable_post_selection_failure_rolls_back_everything() {
        let mut carrier = carrier(2);
        let first = carrier.admit(ActorState(1)).expect("unrelated actor");
        let before = carrier.slots.clone();
        assert_eq!(
            carrier.admit_inner(ActorState(2), true),
            Err(CarrierError::InjectedAdmissionFailure)
        );
        assert_eq!(carrier.slots, before);
        assert_eq!(carrier.lookup(first), Ok(&ActorState(1)));
    }

    #[test]
    fn exhausted_reuse_marks_only_selected_slot_and_never_reselects_it() {
        let mut carrier = carrier(2);
        carrier.slots[0] = Slot::VacantReusable {
            generation: u64::MAX,
        };
        let unrelated = carrier.slots[1];
        assert_eq!(
            carrier.admit(ActorState(1)),
            Err(CarrierError::ActorGenerationExhausted)
        );
        assert_eq!(
            carrier.slots[0],
            Slot::Exhausted {
                generation: u64::MAX
            }
        );
        assert_eq!(carrier.slots[1], unrelated);
        let admitted = carrier.admit(ActorState(2)).expect("skip exhausted slot");
        assert_eq!(admitted.actor_local_id, ActorLocalId(2));
    }

    #[test]
    fn continuity_grant_is_consumed_by_bootstrap() {
        let grant = grant(50, 9);
        let carrier = ChannelActorCarrier::bootstrap_pre_production(grant, 1).expect("bootstrap");
        assert_eq!(carrier.scope_generation.get(), 9);
        // A second same-generation bootstrap cannot be expressed without a new
        // independently supplied grant; raw scope facts are insufficient.
    }
}
