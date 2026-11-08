//! Crystal201189f Tile:getCreatureCount is the raw registered tile vector.
//! tile.cpp128-133 returns vector.size(), insertion1043-1049/1669-1674
//! accepts every Creature subclass, and removal1259-1267 erases registration.
//! Ghost visibility, pushability, HP and NPC/hireling subtype do not filter it.
//! This DATA query intentionally does not alter ordinary spawn or movement.
use super::*;

impl ChannelRuntimeV1 {
    /// Complete current physical tile registrations under this exact owner/frame.
    /// HP0 remains until source death completion; no reserved/uncommitted actor
    /// counts. Callers must hold the same runtime borrow while using these facts.
    pub fn source_registered_tile_actor_positions(
        &self,
    ) -> Result<Vec<MovementLocalPosition>, CarrierError> {
        self.owner_fence()?;
        self.carrier.validate_current_continuity(&self.continuity)?;
        let context = self.pinned_position_context();
        self.carrier.validate_position_context(context)?;
        let mut positions = Vec::new();
        positions
            .try_reserve_exact(self.carrier.occupied.len())
            .map_err(|_| CarrierError::AllocationFailed)?;
        for &raw in &self.carrier.occupied {
            let index =
                usize::try_from(raw).map_err(|_| CarrierError::CapacityArithmeticOverflow)?;
            let (generation, position) = match self.carrier.slots.get(index) {
                Some(Slot::Occupied {
                    generation,
                    committed: true,
                    position: Some(position),
                    ..
                })
                | Some(Slot::NpcOccupied {
                    generation,
                    position,
                    ..
                }) => (*generation, position),
                Some(Slot::CreatureOccupied {
                    generation,
                    position: Some(position),
                    spell_combat,
                    ..
                }) if !spell_combat.source_death_completed => (*generation, position),
                _ => continue,
            };
            if position.context != context {
                return Err(CarrierError::PositionContextMismatch);
            }
            let expected_id = u32::try_from(
                index
                    .checked_add(1)
                    .ok_or(CarrierError::CapacityArithmeticOverflow)?,
            )
            .map_err(|_| CarrierError::CapacityArithmeticOverflow)?;
            if position.actor_local_id != ActorLocalId(expected_id)
                || position.actor_local_generation != ActorLocalGeneration(generation)
            {
                return Err(CarrierError::PositionSnapshotMismatch);
            }
            positions.push(MovementLocalPosition {
                x: position.position.x,
                y: position.position.y,
                floor: position.position.floor,
            });
        }
        Ok(positions)
    }
}

#[cfg(test)]
mod controls {
    use super::*;
    use crate::carrier::house_custody::HouseActorState;
    use crate::carrier::npc_actor::{HirelingProjection, NpcPlacement};
    type TestResult<T = ()> = Result<T, String>;
    fn id(seed: u8) -> [u8; 16] {
        let mut id = [seed; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        id
    }
    fn at(x: i32) -> MovementLocalPosition {
        MovementLocalPosition {
            x,
            y: 100,
            floor: 0,
        }
    }
    fn runtime() -> TestResult<ChannelRuntimeV1> {
        let world = WorldId::decode(&id(1)).map_err(|error| format!("world: {error:?}"))?;
        ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&id(2)).map_err(|error| format!("scope: {error:?}"))?,
            NodeId::decode(&id(3)).map_err(|error| format!("node: {error:?}"))?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            12,
            ChannelContentPin::from_activation(
                world,
                1,
                [1; 32],
                [2; 32],
                [3; 32],
                [4; 32],
                (0, 0, 0),
            ),
        )
        .map_err(|error| format!("runtime: {error:?}"))
    }
    fn count(runtime: &ChannelRuntimeV1, cell: MovementLocalPosition) -> TestResult<usize> {
        Ok(runtime
            .source_registered_tile_actor_positions()
            .map_err(|error| format!("current raw tile registry: {error:?}"))?
            .iter()
            .filter(|position| **position == cell)
            .count())
    }
    #[test]
    fn raw_source_tile_count_keeps_ghost_and_hp0_until_completed_registration() -> TestResult {
        let mut runtime = runtime()?;
        let session =
            GameSessionId::decode(&id(4)).map_err(|error| format!("session: {error:?}"))?;
        let reservation = runtime
            .reserve_fresh_session(session)
            .map_err(|error| format!("reserve: {error:?}"))?;
        let player = runtime
            .commit_fresh_session(reservation)
            .map_err(|error| format!("player: {error:?}"))?;
        runtime
            .initialize_pinned_test_position(player, at(101))
            .map_err(|error| format!("position: {error:?}"))?;
        runtime
            .set_staff_visibility(player, session, true, false)
            .map_err(|error| format!("actual ghost flag: {error:?}"))?;
        assert!(
            runtime.native_summon_cell_free(at(101)),
            "generic spawn deliberately ignores ghost visibility"
        );
        assert_eq!(
            count(&runtime, at(101))?,
            1,
            "literal tile vector still contains the committed player"
        );
        let creature = runtime
            .admit_pinned_monster_lab_creature(at(102), "fixture:tile-count", 20)
            .map_err(|error| format!("creature: {error:?}"))?;
        let hit = runtime
            .borrow_exact_actor_commit_at_us(1000)
            .commit_damage(
                creature,
                OwnerDamageCommand {
                    target: b"fixture:tile-count",
                    occurrence: b"raw-tile-hp0",
                    binding: b"raw-tile-hp0",
                    damage: 20,
                },
            )
            .map_err(|error| format!("actual lethal: {error:?}"))?;
        assert_eq!(hit.health_after, 0);
        assert!(
            runtime.native_summon_cell_free(at(102)),
            "generic spawn ignores a killed creature"
        );
        assert_eq!(
            count(&runtime, at(102))?,
            1,
            "HP0 is still raw registration without fabricating any Lua descriptor"
        );
        let index = runtime
            .carrier
            .validate_ref(&runtime.continuity, creature.0)
            .map_err(|error| format!("exact creature: {error:?}"))?;
        let Slot::CreatureOccupied { spell_combat, .. } = &mut runtime.carrier.slots[index] else {
            return Err("actual creature slot".into());
        };
        assert!(
            spell_combat.native_death_events.is_none(),
            "this predicate control never invents a copied event"
        );
        // This is the registration-completed DATA input to the predicate. The
        // separate source-death owner tests prove its sealed publication.
        spell_combat.source_death_completed = true;
        assert_eq!(
            count(&runtime, at(102))?,
            0,
            "corpse-only retained slots are not registered creatures"
        );
        assert_eq!(count(&runtime, at(101))?, 1);
        Ok(())
    }
    #[test]
    fn raw_source_tile_count_keeps_service_and_house_hireling_npc_roles() -> TestResult {
        let mut runtime = runtime()?;
        runtime
            .place_dynamic_npc(
                "oteryn:npc.source_count",
                NpcPlacement {
                    x: 103,
                    y: 100,
                    floor: 0,
                    direction: MovementFacing::North,
                },
            )
            .map_err(|error| format!("service NPC: {error:?}"))?;
        assert_eq!(count(&runtime, at(103))?, 1);
        let session =
            GameSessionId::decode(&id(5)).map_err(|error| format!("session: {error:?}"))?;
        let lease = CharacterLease::new(
            CharacterId::decode(&id(6)).map_err(|error| format!("character: {error:?}"))?,
            1,
        )
        .map_err(|error| format!("lease: {error:?}"))?;
        let resident = runtime
            .restore_transfer_actor(
                GameSessionId::decode(&id(7))
                    .map_err(|error| format!("origin session: {error:?}"))?,
                session,
                lease,
                LocalPosition {
                    x: 100,
                    y: 100,
                    floor: 0,
                },
                Some(HouseActorState {
                    house: "fixture:tile-count-house".into(),
                    scope: RuntimeScopeRefV1::instance(runtime.binding.world_id, id(8))
                        .map_err(|error| format!("house scope: {error:?}"))?,
                    generation: ScopeOwnershipGeneration::new(1)
                        .map_err(|error| format!("house generation: {error:?}"))?,
                    position: LocalPosition {
                        x: 100,
                        y: 100,
                        floor: 0,
                    },
                    lease,
                    origin_position: LocalPosition {
                        x: 0,
                        y: 0,
                        floor: 0,
                    },
                }),
            )
            .map_err(|error| format!("current house resident: {error:?}"))?;
        let key: String = id(9).iter().map(|byte| format!("{byte:02x}")).collect();
        let projection = HirelingProjection {
            key: format!("oteryn:npc.hireling:{key}"),
            name: "Alex Moon".into(),
            placement: NpcPlacement {
                x: 104,
                y: 100,
                floor: 0,
                direction: MovementFacing::South,
            },
            look_type: 1110,
            colors: [1, 2, 3, 4],
        };
        let hirelings = runtime
            .reconcile_house_hirelings(resident, session, &[projection])
            .map_err(|error| format!("actual house projection: {error:?}"))?;
        assert_eq!(hirelings.len(), 1);
        assert!(
            runtime.native_summon_cell_free(at(104)),
            "generic spawn intentionally ignores hireling projection"
        );
        assert_eq!(
            count(&runtime, at(104))?,
            1,
            "raw source tile registration includes that NPC subtype"
        );
        assert_eq!(
            count(&runtime, at(100))?,
            0,
            "a House resident without a current public-tile pose is not fabricated into this scope"
        );
        runtime.retire_owner_cycle();
        assert!(
            runtime.source_registered_tile_actor_positions().is_err(),
            "retired scope is unknown, never proved-empty"
        );
        Ok(())
    }
}
