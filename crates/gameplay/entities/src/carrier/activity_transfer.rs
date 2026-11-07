//! Physical scope transfer on the existing native actor carrier.
//!
//! Source actors are frozen before SQL and destination fixed slots are reserved.
//! A commit moves their owned payloads without cloning a second playable actor.
//! The caller holds both native owners in deterministic scope order throughout
//! each synchronous method. Uncertain durability retains the reservation.
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivityTransferMember {
    pub source_actor: ExactActorRef,
    pub source_session: GameSessionId,
    pub source_lease: CharacterLease,
    pub destination_session: GameSessionId,
    pub destination_lease: CharacterLease,
    pub destination_cell: MovementLocalPosition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ActivityFrozenPlayer {
    pub(super) actor: ExactActorRef,
    source_session: GameSessionId,
    destination_session: GameSessionId,
    token: u64,
}

#[derive(Debug)]
struct ReservedMember {
    member: ActivityTransferMember,
    destination: PlayerActorReservation,
    freeze: ActivityFrozenPlayer,
    source_index: usize,
    destination_index: usize,
}

/// A unique in-process reservation, never durable admission authority. It is
/// deliberately neither Clone nor Copy. A rejected/unknown commit keeps it.
#[derive(Debug)]
#[must_use]
pub struct ActivityTransferReservation {
    source: ScopeRuntimeAssignmentBinding,
    destination: ScopeRuntimeAssignmentBinding,
    members: Vec<ReservedMember>,
    completed: bool,
    result: Vec<ActivityTransferredActor>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivityTransferredActor {
    pub character_id: CharacterId,
    pub source_actor: ExactActorRef,
    pub source_session: GameSessionId,
    pub destination_actor: ExactActorRef,
    pub destination_session: GameSessionId,
    pub destination_lease: CharacterLease,
}

impl ScopeRuntimeV1 {
    /// Activity lifecycle census includes control-lost and dead participants.
    /// It excludes uncommitted reservations, which independently block retirement.
    pub fn activity_participant_sessions(&self) -> Result<Vec<GameSessionId>, CarrierError> {
        self.owner_fence()?;
        let mut sessions = Vec::new();
        sessions
            .try_reserve(self.carrier.occupied.len())
            .map_err(|_| CarrierError::AllocationFailed)?;
        for index in &self.carrier.occupied {
            let slot = self
                .carrier
                .slots
                .get(
                    usize::try_from(*index)
                        .map_err(|_| CarrierError::CapacityArithmeticOverflow)?,
                )
                .ok_or(CarrierError::PlayerReservationMismatch)?;
            if let Slot::Occupied {
                game_session_id: Some(session),
                committed: true,
                ..
            } = slot
            {
                sessions.push(*session);
            }
        }
        Ok(sessions)
    }

    /// No player payload, fresh reservation, frozen transfer or pending
    /// familiar may be discarded when the completed Instance is unregistered.
    pub fn activity_retirement_empty(&self) -> Result<bool, CarrierError> {
        self.owner_fence()?;
        if !matches!(
            self.binding.runtime_scope,
            RuntimeScopeRefV1::Instance { .. }
        ) {
            return Err(CarrierError::WrongScope);
        }
        Ok(self.carrier.activity_transfers.is_empty()
            && !self
                .carrier
                .slots
                .iter()
                .any(|slot| matches!(slot, Slot::Occupied { .. } | Slot::CreatureReserved { .. })))
    }

    pub fn prepare_activity_transfer(
        &mut self,
        destination: &mut Self,
        members: &[ActivityTransferMember],
    ) -> Result<ActivityTransferReservation, CarrierError> {
        self.prepare_activity_transfer_inner(destination, members, false)
    }

    /// Crystal's authored boss entry may place multiple members on the same
    /// teleport cell. Only this one fresh Channel-to-Instance batch may stack;
    /// existing player admissions still block reuse, and normal transfers stay distinct.
    pub fn prepare_activity_group_entry(
        &mut self,
        destination: &mut Self,
        members: &[ActivityTransferMember],
    ) -> Result<ActivityTransferReservation, CarrierError> {
        if !matches!(
            (
                self.binding.runtime_scope,
                destination.binding.runtime_scope
            ),
            (
                RuntimeScopeRefV1::Channel { .. },
                RuntimeScopeRefV1::Instance { .. }
            )
        ) || destination
            .carrier
            .slots
            .iter()
            .any(|slot| matches!(slot, Slot::Occupied { .. }))
        {
            return Err(CarrierError::WrongScope);
        }
        self.prepare_activity_transfer_inner(destination, members, true)
    }

    /// The retained boss-room participant proof selects an exact authored exit.
    /// Crystal internalTeleport uses FLAG_NOLIMIT for it, including occupants.
    /// Map/door qualification remains with the caller before this reservation.
    pub fn prepare_activity_boss_return(
        &mut self,
        destination: &mut Self,
        members: &[ActivityTransferMember],
    ) -> Result<ActivityTransferReservation, CarrierError> {
        if !matches!(
            (
                self.binding.runtime_scope,
                destination.binding.runtime_scope
            ),
            (
                RuntimeScopeRefV1::Instance { .. },
                RuntimeScopeRefV1::Channel { .. }
            )
        ) {
            return Err(CarrierError::WrongScope);
        }
        self.prepare_activity_transfer_inner(destination, members, true)
    }

    fn prepare_activity_transfer_inner(
        &mut self,
        destination: &mut Self,
        members: &[ActivityTransferMember],
        authored_teleport: bool,
    ) -> Result<ActivityTransferReservation, CarrierError> {
        self.owner_fence()?;
        destination.owner_fence()?;
        if members.is_empty()
            || members.len() > 15
            || self.binding.runtime_scope == destination.binding.runtime_scope
            || self.binding.world_id != destination.binding.world_id
            || self.binding.node_id != destination.binding.node_id
            || self.binding.node_registration_revision
                != destination.binding.node_registration_revision
            || !matches!(
                (
                    self.binding.runtime_scope,
                    destination.binding.runtime_scope
                ),
                (
                    RuntimeScopeRefV1::Channel { .. },
                    RuntimeScopeRefV1::Instance { .. }
                ) | (
                    RuntimeScopeRefV1::Instance { .. },
                    RuntimeScopeRefV1::Channel { .. }
                )
            )
        {
            return Err(CarrierError::WrongScope);
        }
        let context = destination.pinned_position_context();
        let mut checked = Vec::new();
        checked
            .try_reserve_exact(members.len())
            .map_err(|_| CarrierError::AllocationFailed)?;
        let mut result = Vec::new();
        result
            .try_reserve_exact(members.len())
            .map_err(|_| CarrierError::AllocationFailed)?;
        self.carrier
            .activity_transfers
            .try_reserve(members.len())
            .map_err(|_| CarrierError::AllocationFailed)?;
        destination
            .carrier
            .attackers
            .try_reserve(members.len())
            .map_err(|_| CarrierError::AllocationFailed)?;
        for (ordinal, member) in members.iter().enumerate() {
            let index = self.carrier.player_slot_index(
                &self.continuity,
                member.source_actor.0,
                member.source_session,
            )?;
            self.carrier.assert_slot_spell_unreserved(index)?;
            if self.bound_attacker_lease(member.source_actor, member.source_session)?
                != member.source_lease
                || member.source_session == member.destination_session
                || member.source_lease.character_id() != member.destination_lease.character_id()
                || member.source_lease.generation().checked_add(1)
                    != Some(member.destination_lease.generation())
                || (matches!(
                    self.binding.runtime_scope,
                    RuntimeScopeRefV1::Channel { .. }
                ) && self
                    .player_control_facts(member.source_actor, member.source_session)?
                    .control_loss
                    .is_some())
                || members[..ordinal].iter().any(|earlier| {
                    earlier.source_actor == member.source_actor
                        || earlier.destination_session == member.destination_session
                        || (!authored_teleport
                            && earlier.destination_cell == member.destination_cell)
                })
                || (!authored_teleport
                    && destination.carrier.slots.iter().any(|slot| {
                        slot_position(slot).is_some_and(|p| {
                            p.context == context
                                && p.position
                                    == LocalPosition {
                                        x: member.destination_cell.x,
                                        y: member.destination_cell.y,
                                        floor: member.destination_cell.floor,
                                    }
                        })
                    }))
            {
                return Err(CarrierError::PlayerReservationMismatch);
            }
            checked.push((index, *member));
        }
        let token = self
            .carrier
            .fence_transitions
            .checked_add(1)
            .ok_or(CarrierError::CapacityArithmeticOverflow)?;
        let mut reserved: Vec<ReservedMember> = Vec::new();
        reserved
            .try_reserve_exact(members.len())
            .map_err(|_| CarrierError::AllocationFailed)?;
        for (source_index, member) in checked {
            let reservation = match destination.reserve_fresh_session(member.destination_session) {
                Ok(value) => value,
                Err(error) => {
                    for old in &reserved {
                        destination.rollback_definitely_uncommitted(old.destination)?;
                    }
                    return Err(error);
                }
            };
            let destination_index = destination
                .carrier
                .validate_ref(&destination.continuity, reservation.actor_ref)?;
            // The reserved cell is visible to the same native occupancy owner.
            if let Slot::Occupied { position, .. } =
                &mut destination.carrier.slots[destination_index]
            {
                *position = Some(VersionedPosition {
                    actor_local_id: reservation.actor_ref.actor_local_id,
                    actor_local_generation: reservation.actor_ref.actor_local_generation,
                    context,
                    position: LocalPosition {
                        x: member.destination_cell.x,
                        y: member.destination_cell.y,
                        floor: member.destination_cell.floor,
                    },
                    revision: 1,
                    last_translation_revision: 0,
                    facing: None,
                });
            }
            reserved.push(ReservedMember {
                member,
                destination: reservation,
                freeze: ActivityFrozenPlayer {
                    actor: member.source_actor,
                    source_session: member.source_session,
                    destination_session: member.destination_session,
                    token,
                },
                source_index,
                destination_index,
            });
        }
        self.carrier.fence_transitions = token;
        // validate_ref now rejects every read/mutation through an ordinary owner
        // capability for these actors, including movement, spells and AI damage.
        for member in &reserved {
            self.carrier.activity_transfers.push(member.freeze);
        }
        Ok(ActivityTransferReservation {
            source: self.binding,
            destination: destination.binding,
            members: reserved,
            completed: false,
            result,
        })
    }

    pub fn abort_activity_transfer(
        &mut self,
        destination: &mut Self,
        reservation: &mut ActivityTransferReservation,
    ) -> Result<(), CarrierError> {
        // A definite non-commit releases only this exact local reservation. It
        // creates no playable successor, so retirement must not strand a frozen
        // source merely because the destination owner ended while SQL refused.
        self.validate_activity_reservation_local(destination, reservation)?;
        for member in &reservation.members {
            let index = member.destination_index;
            let generation = member.destination.actor_ref.actor_local_generation.0;
            destination.carrier.slots[index] = Slot::VacantReusable {
                generation,
                next_free: destination.carrier.free_head,
            };
            destination.carrier.free_head = Some(index as u32);
            destination
                .carrier
                .occupied
                .retain(|value| *value as usize != index);
        }
        self.carrier
            .activity_transfers
            .retain(|f| !reservation.members.iter().any(|m| m.freeze == *f));
        reservation.completed = true;
        Ok(())
    }

    /// Call only after the complete durable group (or participant return) is
    /// committed. All fallible checks precede the first player payload move.
    pub fn commit_activity_transfer(
        &mut self,
        destination: &mut Self,
        reservation: &mut ActivityTransferReservation,
    ) -> Result<Vec<ActivityTransferredActor>, CarrierError> {
        self.validate_activity_reservation(destination, reservation)?;
        // Remove owned companions in their original scope, through their existing
        // owner, without manufacturing combat deaths or transporting familiars.
        self.carrier
            .activity_transfers
            .retain(|f| !reservation.members.iter().any(|m| m.freeze == *f));
        for member in &reservation.members {
            if let Err(error) = self
                .remove_owned_companions(member.member.source_actor, member.member.source_session)
            {
                for member in &reservation.members {
                    self.carrier.activity_transfers.push(member.freeze);
                }
                return Err(error);
            }
        }
        for member in &reservation.members {
            let source_generation = member.member.source_actor.0.actor_local_generation.0;
            let source_free = Slot::VacantReusable {
                generation: source_generation,
                next_free: self.carrier.free_head,
            };
            let mut payload =
                std::mem::replace(&mut self.carrier.slots[member.source_index], source_free);
            self.carrier.free_head = Some(member.source_index as u32);
            self.carrier
                .occupied
                .retain(|index| *index as usize != member.source_index);
            self.carrier.attackers.retain(|entry| {
                entry.index != member.source_index || entry.generation != source_generation
            });
            let destination_position =
                slot_position(&destination.carrier.slots[member.destination_index]);
            let fresh_actor = ExactActorRef(member.destination.actor_ref);
            if let Slot::Occupied {
                generation,
                game_session_id,
                committed,
                position,
                lifecycle,
                spell_combat,
                ..
            } = &mut payload
            {
                *generation = member.destination.actor_ref.actor_local_generation.0;
                *game_session_id = Some(member.member.destination_session);
                *committed = true;
                *position = destination_position;
                lifecycle.control_loss = None;
                spell_combat.reset_for_scope_handoff();
            } else {
                unreachable!("exclusive transfer preflight proved every source player slot");
            }
            destination.carrier.slots[member.destination_index] = payload;
            destination.carrier.attackers.push(PlayerAttackerEntry {
                index: member.destination_index,
                generation: fresh_actor.0.actor_local_generation.0,
                authority: PlayerAttackerAuthority {
                    lease: Some(member.member.destination_lease),
                    fence: None,
                },
            });
            reservation.result.push(ActivityTransferredActor {
                character_id: member.member.destination_lease.character_id(),
                source_actor: member.member.source_actor,
                source_session: member.member.source_session,
                destination_actor: fresh_actor,
                destination_session: member.member.destination_session,
                destination_lease: member.member.destination_lease,
            });
        }
        reservation.completed = true;
        Ok(std::mem::take(&mut reservation.result))
    }

    fn validate_activity_reservation(
        &self,
        destination: &Self,
        reservation: &ActivityTransferReservation,
    ) -> Result<(), CarrierError> {
        self.owner_fence()?;
        destination.owner_fence()?;
        self.validate_activity_reservation_local(destination, reservation)
    }

    fn validate_activity_reservation_local(
        &self,
        destination: &Self,
        reservation: &ActivityTransferReservation,
    ) -> Result<(), CarrierError> {
        if reservation.completed
            || self.binding != reservation.source
            || destination.binding != reservation.destination
        {
            return Err(CarrierError::WrongScope);
        }
        for reserved in &reservation.members {
            if !self.carrier.activity_transfers.contains(&reserved.freeze) {
                return Err(CarrierError::PlayerReservationMismatch);
            }
            match &self.carrier.slots[reserved.source_index] {
                Slot::Occupied {
                    generation,
                    game_session_id: Some(session),
                    committed: true,
                    spell_combat,
                    ..
                } if *generation == reserved.member.source_actor.0.actor_local_generation.0
                    && *session == reserved.member.source_session
                    && spell_combat.pending_owner.is_none()
                    && spell_combat.pending_source.is_none() => {}
                _ => return Err(CarrierError::PlayerReservationMismatch),
            }
            match &destination.carrier.slots[reserved.destination_index] {
                Slot::Occupied {
                    generation,
                    game_session_id: Some(session),
                    committed: false,
                    position: Some(position),
                    ..
                } if *generation == reserved.destination.actor_ref.actor_local_generation.0
                    && *session == reserved.member.destination_session
                    && position.context == destination.pinned_position_context()
                    && position.position
                        == LocalPosition {
                            x: reserved.member.destination_cell.x,
                            y: reserved.member.destination_cell.y,
                            floor: reserved.member.destination_cell.floor,
                        } => {}
                _ => return Err(CarrierError::PlayerReservationMismatch),
            }
        }
        Ok(())
    }
}

fn slot_position(slot: &Slot) -> Option<VersionedPosition> {
    match slot {
        Slot::Occupied { position, .. } | Slot::CreatureOccupied { position, .. } => *position,
        Slot::NpcOccupied { position, .. } => Some(*position),
        Slot::CreatureReserved { planned, .. } => slot_position(planned),
        _ => None,
    }
}
