//! Install a current native House destination into its existing fresh-admission slot.
use super::*;

impl ChannelRuntimeV1 {
    /// The composition has just committed the native House handoff from this exact
    /// freshly authenticated source. Reuse that unpositioned slot and its real
    /// source lease. `origin_position` is retained Channel routing history only;
    /// it is never published as a playable Channel position during this transition.
    pub fn restore_admitted_house_actor(
        &mut self,
        actor: ExactActorRef,
        source: GameSessionId,
        destination: GameSessionId,
        source_lease: CharacterLease,
        house: HouseActorState,
    ) -> Result<(), CarrierError> {
        if source == destination
            || house.lease.character_id() != source_lease.character_id()
            || source_lease.generation().checked_add(1) != Some(house.lease.generation())
            || house.scope.world_id() != self.binding.world_id
            || house.house.is_empty()
            || !matches!(house.scope, RuntimeScopeRefV1::Instance { .. })
        {
            return Err(CarrierError::WrongScope);
        }
        // All House custody preconditions precede the internal origin-position
        // initialization. A rejected owner or pending actor cannot leave a
        // previously unpositioned admission published in its Channel.
        let index = self
            .carrier
            .player_slot_index(&self.continuity, actor.0, source)?;
        if !matches!(&self.carrier.slots[index], Slot::Occupied {
            position: None, lifecycle, spell_combat, ..
        } if lifecycle.control_loss.is_none() && spell_combat.pending_owner.is_none()
            && spell_combat.pending_source.is_none())
            || self.house_owners.get(&house.house).is_some_and(|owner| {
                owner.scope != house.scope || owner.generation != house.generation
            })
        {
            return Err(CarrierError::WrongScope);
        }
        self.carrier
            .house_custody
            .try_reserve(1)
            .map_err(|_| CarrierError::AllocationFailed)?;
        self.bind_attacker_lease(actor, source, source_lease)?;
        if self.bound_attacker_lease(actor, source)? != source_lease {
            return Err(CarrierError::WrongScope);
        }
        // The same native initialization used by restore_transfer_actor's new
        // slot branch. The owner retains the runtime lock through House custody.
        let context = self.pinned_position_context();
        self.carrier.initialize_position(
            &self.continuity,
            actor.0,
            context,
            house.origin_position,
        )?;
        self.commit_house_custody(
            actor,
            source,
            destination,
            house.lease,
            &house.house,
            house.scope,
            house.generation,
            house.position,
            false,
        )
    }

    /// Recover the existing unpositioned fresh slot only after the composition
    /// authenticates the closed admission-restore receipt and current destination.
    /// The source lease is immutable receipt provenance, never current authority.
    /// No source means the ordinary cold/destination recovery path still owns it.
    pub fn restore_retained_house_admission(
        &mut self,
        source: GameSessionId,
        destination: GameSessionId,
        source_lease: CharacterLease,
        house: HouseActorState,
    ) -> Result<Option<ExactActorRef>, CarrierError> {
        self.carrier.validate_current_continuity(&self.continuity)?;
        let mut matching = self
            .carrier
            .slots
            .iter()
            .enumerate()
            .filter_map(|(index, slot)| match slot {
                Slot::Occupied {
                    generation,
                    game_session_id: Some(session),
                    committed: true,
                    ..
                } if *session == source => Some((index, *generation)),
                _ => None,
            });
        let Some((index, generation)) = matching.next() else {
            return Ok(None);
        };
        if matching.next().is_some() {
            return Err(CarrierError::WrongScope);
        }
        let local = index
            .checked_add(1)
            .and_then(|index| u32::try_from(index).ok())
            .map(ActorLocalId)
            .ok_or(CarrierError::CapacityArithmeticOverflow)?;
        let actor = ExactActorRef(ActorRef::from_scoped_owner_parts(
            self.carrier.runtime_scope,
            self.carrier.scope_generation,
            local,
            ActorLocalGeneration(generation),
        ));
        self.restore_admitted_house_actor(actor, source, destination, source_lease, house)?;
        Ok(Some(actor))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)] // Native fixture setup must fail loudly if its validated identifiers are invalid.
mod tests {
    use super::*;
    use crate::carrier::tests::{runtime, session};

    fn uuid(seed: u8) -> [u8; 16] {
        let mut value = [0; 16];
        value[6] = 0x70;
        value[8] = 0x80;
        value[15] = seed;
        value
    }

    #[test]
    fn committed_fresh_slot_recovers_without_a_channel_position_or_second_actor() {
        let mut runtime = runtime(3);
        let source = session(100);
        let destination = session(101);
        let reserved = runtime
            .reserve_fresh_session(source)
            .expect("native source reservation");
        let actor = runtime
            .commit_fresh_session(reserved)
            .expect("committed fresh source");
        let character = CharacterId::decode(&uuid(1)).expect("character");
        let before = CharacterLease::new(character, 2).expect("source lease");
        runtime
            .bind_attacker_lease(actor, source, before)
            .expect("actual source lease");
        let house = HouseActorState {
            house: "house:admission".into(),
            scope: RuntimeScopeRefV1::Instance {
                world_id: runtime.binding.world_id,
                instance_id: uuid(7),
            },
            generation: ScopeOwnershipGeneration::new(1).expect("House generation"),
            position: LocalPosition {
                x: 1,
                y: 1,
                floor: 7,
            },
            lease: CharacterLease::new(character, 3).expect("current destination lease"),
            origin_position: LocalPosition {
                x: 0,
                y: 0,
                floor: 7,
            },
        };
        let wrong = CharacterLease::new(character, 1).expect("wrong historical lease");
        assert!(
            runtime
                .restore_retained_house_admission(source, destination, wrong, house.clone())
                .is_err()
        );
        assert!(runtime.read_actor_position(actor).is_err());
        assert_eq!(runtime.bound_attacker_lease(actor, source), Ok(before));
        let fence_id = runtime
            .mint_transition_fence()
            .expect("native transition fence");
        let fence = ChannelRuntimeV1::transition_fence(fence_id);
        runtime
            .fence_player_writes(actor, source, fence)
            .expect("fence actual source");
        assert!(
            runtime
                .restore_retained_house_admission(source, destination, before, house.clone())
                .is_err()
        );
        assert!(
            runtime.read_actor_position(actor).is_err(),
            "a fenced source remains unpositioned"
        );
        runtime
            .lift_player_fence(actor, source, fence)
            .expect("restore actual source owner");
        assert_eq!(
            runtime
                .restore_retained_house_admission(source, destination, before, house.clone())
                .expect("actual committed admission recovery"),
            Some(actor)
        );
        assert_eq!(
            runtime
                .house_actor_state(actor, destination)
                .expect("named House actor"),
            house
        );
        assert!(runtime.read_actor_position(actor).is_err());
        assert!(runtime.bound_attacker_lease(actor, source).is_err());
        assert_eq!(
            runtime
                .restore_retained_house_admission(source, destination, before, house)
                .expect("source is already retired"),
            None
        );
    }
}
