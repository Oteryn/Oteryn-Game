//! Movement of the existing native player slot while its current owner is a House.
//! A snapshot is only a compare predecessor; the named owner and lease are checked again.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HouseMovementSnapshot {
    actor: ExactActorRef,
    session: GameSessionId,
    state: HouseActorState,
    revision: u64,
    facing: Option<MovementFacing>,
}

impl HouseMovementSnapshot {
    pub fn state(&self) -> &HouseActorState {
        &self.state
    }
    pub const fn revision(&self) -> u64 {
        self.revision
    }
    pub const fn facing(&self) -> Option<MovementFacing> {
        self.facing
    }
}

impl ChannelRuntimeV1 {
    /// Read the actual House position and its native revision, never the retained
    /// origin Channel position. A stale lease/session/owner cannot obtain a snapshot.
    pub fn read_house_movement(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Result<HouseMovementSnapshot, CarrierError> {
        let state = self.house_actor_state(actor, session)?;
        let resident = self
            .carrier
            .house_custody
            .iter()
            .find(|resident| resident.actor == actor && resident.session == session)
            .ok_or(CarrierError::WrongScope)?;
        Ok(HouseMovementSnapshot {
            actor,
            session,
            state,
            revision: resident.channel_position.revision,
            facing: resident.channel_position.facing,
        })
    }

    /// Collision observation for a placement already authorized by the native House
    /// admission/recovery transaction. It grants no session or scope authority.
    pub fn house_placement_clear(
        &self,
        candidate: &HouseActorState,
        except: Option<ExactActorRef>,
    ) -> bool {
        if candidate.scope.world_id() != self.binding.world_id
            || self
                .house_owners
                .get(&candidate.house)
                .is_some_and(|owner| {
                    owner.scope != candidate.scope || owner.generation != candidate.generation
                })
        {
            return false;
        }
        for resident in &self.carrier.house_custody {
            if resident.key != candidate.house {
                continue;
            }
            let Ok(current) = self.house_actor_state(resident.actor, resident.session) else {
                return false;
            };
            if current.scope != candidate.scope || current.generation != candidate.generation {
                return false;
            }
            if Some(resident.actor) != except && current.position == candidate.position {
                return false;
            }
        }
        true
    }

    /// Publish a destination already resolved by the active native map and House
    /// floor owner. The composition retains the runtime lock from map/collision
    /// validation through this compare-write. House custody, revision, control,
    /// and scoped occupants are independently checked at publication.
    pub fn compare_move_house(
        &mut self,
        expected: &HouseMovementSnapshot,
        destination: LocalPosition,
        facing: MovementFacing,
    ) -> Result<HouseMovementSnapshot, CarrierError> {
        if self.read_house_movement(expected.actor, expected.session)? != *expected
            || self.house_player_writes_fenced(expected.actor, expected.session)?
            || destination == expected.state.position
            || self
                .house_player_control_facts(expected.actor, expected.session)?
                .control_loss
                .is_some()
            || self.house_cell_occupied_except(expected.actor, expected.session, destination)?
        {
            return Err(CarrierError::WrongScope);
        }
        let index = usize::try_from(expected.actor.actor_local_id())
            .ok()
            .and_then(|index| index.checked_sub(1))
            .ok_or(CarrierError::InvalidActorIdentity)?;
        if !matches!(self.carrier.slots.get(index), Some(Slot::Occupied { spell_combat, .. })
            if spell_combat.pending_owner.is_none() && spell_combat.pending_source.is_none())
        {
            return Err(CarrierError::WrongScope);
        }
        let revision = expected
            .revision
            .checked_add(1)
            .ok_or(CarrierError::CapacityArithmeticOverflow)?;
        let mut state = expected.state.clone();
        state.position = destination;
        let after = HouseMovementSnapshot {
            actor: expected.actor,
            session: expected.session,
            state,
            revision,
            facing: Some(facing),
        };
        let resident = self
            .carrier
            .house_custody
            .iter_mut()
            .find(|resident| {
                resident.actor == expected.actor && resident.session == expected.session
            })
            .ok_or(CarrierError::WrongScope)?;
        resident.position = destination;
        resident.channel_position.revision = revision;
        resident.channel_position.facing = Some(facing);
        // Retain the origin for a later native House exit. Only the current
        // House position and the single native actor revision have changed.
        Ok(after)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
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
    fn resident(
        runtime: &mut ChannelRuntimeV1,
        seed: u8,
        house: u8,
        position: LocalPosition,
    ) -> (ExactActorRef, GameSessionId) {
        let session = session(u64::from(seed));
        let lease = CharacterLease::new(CharacterId::decode(&uuid(seed)).expect("character"), 1)
            .expect("lease");
        let state = HouseActorState {
            house: format!("house:{house}"),
            scope: RuntimeScopeRefV1::Instance {
                world_id: runtime.binding.world_id,
                instance_id: uuid(house),
            },
            generation: ScopeOwnershipGeneration::new(1).expect("owner generation"),
            position,
            lease,
            origin_position: LocalPosition {
                x: 0,
                y: 0,
                floor: 7,
            },
        };
        let actor = runtime
            .restore_transfer_actor(
                crate::carrier::tests::session(100 + u64::from(seed)),
                session,
                lease,
                position,
                Some(state),
            )
            .expect("native House resident");
        (actor, session)
    }
    #[test]
    fn house_step_uses_actual_owner_preserves_origin_and_rejects_aba() {
        let mut runtime = runtime(5);
        let origin = LocalPosition {
            x: 100,
            y: 100,
            floor: 7,
        };
        let (actor, session) = resident(&mut runtime, 40, 80, origin);
        let before = runtime
            .read_house_movement(actor, session)
            .expect("House position");
        let next = LocalPosition { x: 101, ..origin };
        let moved = runtime
            .compare_move_house(&before, next, MovementFacing::East)
            .expect("step");
        assert_eq!(moved.state().position, next);
        assert_eq!(
            moved.state().origin_position,
            before.state().origin_position
        );
        assert_eq!(moved.revision(), before.revision() + 1);
        assert!(runtime.borrow_movement_position().read(actor).is_err());
        let restored = runtime
            .restore_transfer_actor(
                crate::carrier::tests::session(140),
                session,
                before.state().lease,
                origin,
                Some(before.state().clone()),
            )
            .expect("resume proves the same owner without rewinding the moved actor");
        assert_eq!(restored, actor);
        assert_eq!(
            runtime
                .read_house_movement(actor, session)
                .expect("preserved step"),
            moved
        );
        assert!(
            runtime
                .compare_move_house(&before, next, MovementFacing::East)
                .is_err()
        );
        let back = runtime
            .compare_move_house(&moved, origin, MovementFacing::West)
            .expect("step back");
        assert_eq!(back.state().position, before.state().position);
        assert!(
            runtime
                .compare_move_house(&before, next, MovementFacing::East)
                .is_err()
        );
        assert!(
            runtime
                .read_house_movement(actor, crate::carrier::tests::session(41))
                .is_err()
        );
    }
    #[test]
    fn house_step_rejects_same_house_occupant_but_ignores_other_house() {
        let mut runtime = runtime(5);
        let origin = LocalPosition {
            x: 100,
            y: 100,
            floor: 7,
        };
        let next = LocalPosition { x: 101, ..origin };
        let (actor, session) = resident(&mut runtime, 40, 80, origin);
        let _ = resident(&mut runtime, 41, 81, next);
        let before = runtime
            .read_house_movement(actor, session)
            .expect("House position");
        let moved = runtime
            .compare_move_house(&before, next, MovementFacing::East)
            .expect("other House is disjoint");
        let _ = resident(&mut runtime, 42, 80, origin);
        assert!(
            runtime
                .compare_move_house(&moved, origin, MovementFacing::West)
                .is_err()
        );
        assert_eq!(
            runtime
                .read_house_movement(actor, session)
                .expect("unchanged"),
            moved
        );
    }

    #[test]
    fn house_step_rechecks_current_hireling_occupancy_at_publication() {
        use crate::carrier::npc_actor::{HirelingProjection, NpcPlacement};
        let mut runtime = runtime(5);
        let origin = LocalPosition {
            x: 100,
            y: 100,
            floor: 7,
        };
        let next = LocalPosition { x: 101, ..origin };
        let (actor, session) = resident(&mut runtime, 40, 80, origin);
        let before = runtime
            .read_house_movement(actor, session)
            .expect("House position");
        let projection = HirelingProjection {
            key: "oteryn:npc.hireling:00000000000070008000000000000001".into(),
            name: "Alex Moon".into(),
            placement: NpcPlacement {
                x: 101,
                y: 100,
                floor: -7,
                direction: MovementFacing::South,
            },
            look_type: 1110,
            colors: [1, 2, 3, 4],
        };
        runtime
            .reconcile_house_hirelings(actor, session, &[projection])
            .expect("current Hireling");
        assert!(
            runtime
                .compare_move_house(&before, next, MovementFacing::East)
                .is_err()
        );
        assert_eq!(
            runtime
                .read_house_movement(actor, session)
                .expect("unchanged"),
            before
        );
        runtime
            .reconcile_house_hirelings(actor, session, &[])
            .expect("returned Hireling");
        assert!(
            runtime
                .compare_move_house(&before, next, MovementFacing::East)
                .is_ok()
        );
    }
}
