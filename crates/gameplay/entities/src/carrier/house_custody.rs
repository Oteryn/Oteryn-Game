//! Native House custody of an existing carrier slot. No actor state is copied.
use super::*;
use oteryn_gameplay_foundation::foundation::{RuntimeScopeRefV1, ScopeRuntimeFence};

#[derive(Debug)]
pub(super) struct HouseScopeOwner {
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
    cycle: ScopeRuntimeFence,
}
struct HouseOwnerGrant(RuntimeScopeRefV1, ScopeOwnershipGeneration);
impl oteryn_gameplay_foundation::foundation::owner_cycle_grant_seal::Sealed for HouseOwnerGrant {}
impl oteryn_gameplay_foundation::foundation::OwnerCycleGrant for HouseOwnerGrant {
    fn owner_scope(&self) -> RuntimeScopeRefV1 {
        self.0
    }
    fn owner_generation(&self) -> ScopeOwnershipGeneration {
        self.1
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct HouseResident {
    pub(super) actor: ExactActorRef,
    key: String,
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
    session: GameSessionId,
    position: LocalPosition,
    channel_position: VersionedPosition,
}

/// A borrow-free value observation; a new operation must recheck current custody.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HouseActorState {
    pub house: String,
    pub scope: RuntimeScopeRefV1,
    pub generation: ScopeOwnershipGeneration,
    pub position: LocalPosition,
    pub lease: CharacterLease,
}

impl ChannelRuntimeV1 {
    /// The actual House owner, not a Channel position or an inventory proof.
    pub fn house_actor_state(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Result<HouseActorState, CarrierError> {
        let resident = self
            .carrier
            .house_custody
            .iter()
            .find(|r| r.actor == actor && r.session == session)
            .ok_or(CarrierError::WrongScope)?;
        let owner = self
            .house_owners
            .get(&resident.key)
            .ok_or(CarrierError::WrongScope)?;
        if owner.scope != resident.scope
            || owner.generation != resident.generation
            || !owner
                .cycle
                .is_current_for_scope(resident.scope, resident.generation)
        {
            return Err(CarrierError::WrongScope);
        }
        let index = usize::try_from(actor.actor_local_id())
            .ok()
            .and_then(|v| v.checked_sub(1))
            .ok_or(CarrierError::InvalidActorIdentity)?;
        if !matches!(self.carrier.slots.get(index),Some(Slot::Occupied {generation,game_session_id:Some(current),committed:true,position:None,..}) if *generation==actor.actor_local_generation() && *current==session)
        {
            return Err(CarrierError::WrongScope);
        }
        let lease = self
            .carrier
            .attackers
            .iter()
            .find(|v| v.index == index && v.generation == actor.actor_local_generation())
            .and_then(|v| v.authority.lease)
            .ok_or(CarrierError::WrongScope)?;
        Ok(HouseActorState {
            house: resident.key.clone(),
            scope: resident.scope,
            generation: resident.generation,
            position: resident.position,
            lease,
        })
    }

    /// Apply a durably committed same-node transfer while the composition holds
    /// the runtime lock. The source slot, conditions and combat state are retained.
    #[allow(clippy::too_many_arguments)]
    pub fn commit_house_custody(
        &mut self,
        actor: ExactActorRef,
        source: GameSessionId,
        destination: GameSessionId,
        lease: CharacterLease,
        house: &str,
        scope: RuntimeScopeRefV1,
        generation: ScopeOwnershipGeneration,
        position: LocalPosition,
        exit: bool,
    ) -> Result<(), CarrierError> {
        self.carrier.validate_current_continuity(&self.continuity)?;
        if scope.world_id() != self.binding.world_id || house.is_empty() || source == destination {
            return Err(CarrierError::WrongScope);
        }
        if exit {
            let before = self.house_actor_state(actor, source)?;
            if before.house != house
                || before.scope != scope
                || before.generation != generation
                || before.lease.character_id() != lease.character_id()
                || before.lease.generation().checked_add(1) != Some(lease.generation())
            {
                return Err(CarrierError::WrongScope);
            }
            let at = self
                .carrier
                .house_custody
                .iter()
                .position(|r| r.actor == actor)
                .ok_or(CarrierError::WrongScope)?;
            let resident = &self.carrier.house_custody[at];
            let mut restored = resident.channel_position;
            restored.position = position;
            restored.revision = restored
                .revision
                .checked_add(1)
                .ok_or(CarrierError::CapacityArithmeticOverflow)?;
            let index = actor.actor_local_id() as usize - 1;
            let attacker = self
                .carrier
                .attackers
                .iter_mut()
                .find(|v| v.index == index && v.generation == actor.actor_local_generation())
                .ok_or(CarrierError::WrongScope)?;
            attacker.authority.lease = Some(lease);
            attacker.authority.fence = None;
            match &mut self.carrier.slots[index] {
                Slot::Occupied {
                    game_session_id,
                    position,
                    lifecycle,
                    ..
                } => {
                    *game_session_id = Some(destination);
                    *position = Some(restored);
                    lifecycle.control_loss = None;
                }
                _ => return Err(CarrierError::WrongScope),
            }
            self.carrier.house_custody.remove(at);
        } else {
            let current = self.bound_attacker_lease(actor, source)?;
            if current.character_id() != lease.character_id()
                || current.generation().checked_add(1) != Some(lease.generation())
                || !matches!(scope, RuntimeScopeRefV1::Instance { .. })
            {
                return Err(CarrierError::WrongScope);
            }
            if let Some(owner) = self.house_owners.get(house)
                && (owner.scope != scope || owner.generation != generation)
            {
                return Err(CarrierError::WrongScope);
            }
            self.carrier
                .house_custody
                .try_reserve(1)
                .map_err(|_| CarrierError::AllocationFailed)?;
            let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
            let channel_position = match &self.carrier.slots[index] {
                Slot::Occupied {
                    position: Some(pos),
                    lifecycle,
                    spell_combat,
                    ..
                } if lifecycle.control_loss.is_none()
                    && spell_combat.pending_owner.is_none()
                    && spell_combat.pending_source.is_none() =>
                {
                    *pos
                }
                _ => return Err(CarrierError::WrongScope),
            };
            self.house_owners
                .entry(house.to_owned())
                .or_insert_with(|| HouseScopeOwner {
                    scope,
                    generation,
                    cycle: ScopeRuntimeFence::from_owner_cycle_grant(HouseOwnerGrant(
                        scope, generation,
                    )),
                });
            let attacker = self
                .carrier
                .attackers
                .iter_mut()
                .find(|v| v.index == index && v.generation == actor.actor_local_generation())
                .ok_or(CarrierError::WrongScope)?;
            attacker.authority.lease = Some(lease);
            // Channel validation refuses custody before any old write can reach this slot.
            attacker.authority.fence = None;
            match &mut self.carrier.slots[index] {
                Slot::Occupied {
                    game_session_id,
                    position,
                    ..
                } => {
                    *game_session_id = Some(destination);
                    *position = None;
                }
                _ => return Err(CarrierError::WrongScope),
            }
            self.carrier.house_custody.push(HouseResident {
                actor,
                key: house.to_owned(),
                scope,
                generation,
                session: destination,
                position,
                channel_position,
            });
        }
        Ok(())
    }
}
