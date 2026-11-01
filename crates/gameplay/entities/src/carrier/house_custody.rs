//! Native House custody of an existing carrier slot. No actor state is copied.
use super::*;
use oteryn_gameplay_foundation::foundation::{RuntimeScopeRefV1, ScopeRuntimeFence};

mod movement;
pub use movement::HouseMovementSnapshot;
mod admission;
mod lifecycle;

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
    pub origin_position: LocalPosition,
}

impl ChannelRuntimeV1 {
    /// Resolve the session from this live slot, then recheck its current House owner.
    /// A retained actor reference alone never establishes House custody.
    pub fn house_observer(
        &self,
        actor: ExactActorRef,
    ) -> Result<Option<(GameSessionId, HouseActorState)>, CarrierError> {
        let Some(resident) = self.carrier.house_custody.iter().find(|r| r.actor == actor) else {
            return Ok(None);
        };
        self.house_actor_state(actor, resident.session)
            .map(|state| Some((resident.session, state)))
    }

    /// Read lifecycle under actual House custody; generic Channel reads remain closed.
    pub fn house_player_control_facts(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Result<PlayerControlFacts, CarrierError> {
        self.house_actor_state(actor, session)?;
        let index = usize::try_from(actor.actor_local_id())
            .ok()
            .and_then(|i| i.checked_sub(1))
            .ok_or(CarrierError::InvalidActorIdentity)?;
        let Slot::Occupied { lifecycle, .. } = &self.carrier.slots[index] else {
            return Err(CarrierError::WrongScope);
        };
        Ok(PlayerControlFacts {
            placement_identity: actor.placement_identity(),
            placement_revision: actor.actor_local_generation(),
            control_loss: lifecycle.control_loss,
        })
    }

    /// The current House actor of an authenticated session, for scoped chat and visibility.
    pub fn house_player_for_session(
        &self,
        session: GameSessionId,
    ) -> Result<Option<(ExactActorRef, HouseActorState)>, CarrierError> {
        let Some(resident) = self
            .carrier
            .house_custody
            .iter()
            .find(|r| r.session == session)
        else {
            return Ok(None);
        };
        self.house_actor_state(resident.actor, session)
            .map(|state| Some((resident.actor, state)))
    }

    /// Observe only current residents of the same named House and generation.
    pub fn house_visible_players(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Result<Vec<VisibleRuntimeActor>, CarrierError> {
        let state = self.house_actor_state(actor, session)?;
        Ok(self
            .carrier
            .house_custody
            .iter()
            .filter_map(|resident| {
                let current = self
                    .house_actor_state(resident.actor, resident.session)
                    .ok()?;
                (current.house == state.house
                    && current.scope == state.scope
                    && current.generation == state.generation
                    && current.position.floor == state.position.floor)
                    .then_some(VisibleRuntimeActor {
                        actor: resident.actor,
                        generation: resident.actor.actor_local_generation(),
                        position: MovementLocalPosition {
                            x: current.position.x,
                            y: current.position.y,
                            floor: current.position.floor,
                        },
                        revision: resident.channel_position.revision,
                    })
            })
            .collect())
    }

    /// Reconstruct one committed destination after process-local state was lost.
    /// The composition must first authenticate recovery and prove current durable
    /// session, lease, assignment and node authority. No receipt alone is a grant.
    pub fn restore_transfer_actor(
        &mut self,
        source: GameSessionId,
        session: GameSessionId,
        lease: CharacterLease,
        position: LocalPosition,
        house: Option<HouseActorState>,
    ) -> Result<ExactActorRef, CarrierError> {
        self.carrier.validate_current_continuity(&self.continuity)?;
        if let Some(resident) = self
            .carrier
            .house_custody
            .iter()
            .find(|r| r.session == session)
        {
            let actor = resident.actor;
            let current = self.house_actor_state(actor, session)?;
            // The transfer receipt records the admitted entry cell. A current
            // resident may have moved since that receipt; recovery proves the
            // same named owner/session/lease and preserves its live position.
            return if house.as_ref().is_some_and(|expected| {
                expected.house == current.house
                    && expected.scope == current.scope
                    && expected.generation == current.generation
                    && expected.lease == current.lease
                    && expected.origin_position == current.origin_position
            }) {
                Ok(actor)
            } else {
                Err(CarrierError::WrongScope)
            };
        }
        if let Some(resident) = self
            .carrier
            .house_custody
            .iter()
            .find(|r| r.session == source)
        {
            if house.is_some() {
                return Err(CarrierError::WrongScope);
            }
            let actor = resident.actor;
            let before = self.house_actor_state(actor, source)?;
            self.commit_house_custody(
                actor,
                source,
                session,
                lease,
                &before.house,
                before.scope,
                before.generation,
                position,
                true,
            )?;
            return Ok(actor);
        }
        if let Some((actor, _)) = self.positioned_player_for_session(session)? {
            return if house.is_none() && self.bound_attacker_lease(actor, session)? == lease {
                Ok(actor)
            } else {
                Err(CarrierError::WrongScope)
            };
        }
        if let Some((actor, _)) = self.positioned_player_for_session(source)? {
            let state = house.as_ref().ok_or(CarrierError::WrongScope)?;
            let index = actor.actor_local_id() as usize - 1;
            let authority = self
                .carrier
                .attackers
                .iter_mut()
                .find(|a| a.index == index && a.generation == actor.actor_local_generation())
                .ok_or(CarrierError::WrongScope)?;
            if !authority.authority.lease.is_some_and(|old| {
                old.character_id() == lease.character_id()
                    && old.generation().checked_add(1) == Some(lease.generation())
            }) {
                return Err(CarrierError::WrongScope);
            }
            authority.authority.fence = None;
            self.commit_house_custody(
                actor,
                source,
                session,
                lease,
                &state.house,
                state.scope,
                state.generation,
                position,
                false,
            )?;
            return Ok(actor);
        }
        if self.carrier.attackers.iter().any(|a| {
            a.authority
                .lease
                .is_some_and(|l| l.character_id() == lease.character_id())
        }) {
            return Err(CarrierError::WrongScope);
        }
        if let Some(state) = &house {
            if state.lease != lease
                || state.scope.world_id() != self.binding.world_id
                || !matches!(state.scope, RuntimeScopeRefV1::Instance { .. })
            {
                return Err(CarrierError::WrongScope);
            }
            if let Some(owner) = self.house_owners.get(&state.house)
                && (owner.scope != state.scope || owner.generation != state.generation)
            {
                return Err(CarrierError::WrongScope);
            }
            self.carrier
                .house_custody
                .try_reserve(1)
                .map_err(|_| CarrierError::AllocationFailed)?;
        }
        let reservation = self.reserve_fresh_session(session)?;
        let actor = self.commit_fresh_session(reservation)?;
        let context = self.pinned_position_context();
        let origin = house.as_ref().map_or(position, |h| h.origin_position);
        let snapshot =
            self.carrier
                .initialize_position(&self.continuity, actor.0, context, origin)?;
        self.bind_attacker_lease(actor, session, lease)?;
        if let Some(state) = house {
            self.house_owners
                .entry(state.house.clone())
                .or_insert_with(|| HouseScopeOwner {
                    scope: state.scope,
                    generation: state.generation,
                    cycle: ScopeRuntimeFence::from_owner_cycle_grant(HouseOwnerGrant(
                        state.scope,
                        state.generation,
                    )),
                });
            let index = actor.actor_local_id() as usize - 1;
            if let Slot::Occupied { position, .. } = &mut self.carrier.slots[index] {
                *position = None;
            }
            self.carrier.house_custody.push(HouseResident {
                actor,
                key: state.house,
                scope: state.scope,
                generation: state.generation,
                session,
                position: state.position,
                channel_position: snapshot.version,
            });
        }
        Ok(actor)
    }

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
            origin_position: resident.channel_position.position,
        })
    }

    /// Current House occupants under the runtime owner lock, excluding the acting resident.
    /// Channel positions cannot answer this question: House custody clears them.
    pub fn house_cell_occupied_except(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        position: LocalPosition,
    ) -> Result<bool, CarrierError> {
        let state = self.house_actor_state(actor, session)?;
        Ok(self.house_hireling_cell_occupied(actor, session, position)?
            || self.carrier.house_custody.iter().any(|resident| {
                resident.actor != actor
                    && resident.scope == state.scope
                    && resident.generation == state.generation
                    && resident.position == position
            }))
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
            if restored.position != resident.channel_position.position {
                restored.last_translation_revision = restored.revision;
            }
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
            if !self.carrier.house_custody.iter().any(|resident| {
                resident.key == house
                    && resident.scope == scope
                    && resident.generation == generation
            }) {
                self.retire_house_hireling_projection(house, scope, generation);
            }
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

#[cfg(test)]
mod translation_tests;
