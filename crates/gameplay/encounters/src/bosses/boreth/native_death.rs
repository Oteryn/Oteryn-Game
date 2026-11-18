use oteryn_gameplay_entities::carrier::Slot;
use oteryn_gameplay_entities::*;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

pub trait BorethCarrierExt {
    fn commit_boreth_death(
        &mut self,
        ledger: &mut BorethDeathLedger,
        boreth: ExactActorRef,
    ) -> Result<BorethDeathResult, CarrierError> {
        // Crystal 201189f8 data-global/scripts/quests/blood_brothers_quest/boreth_death.lua:3-20.
        self.commit_encounter_add_cleanup(
            ledger,
            boreth,
            "oteryn:creature.boreth",
            "oteryn:creature.plaguethrower",
            Position {
                x: 32936,
                y: 31474,
                floor: -1,
            },
            Position {
                x: 32944,
                y: 31482,
                floor: -1,
            },
        )
    }

    fn commit_encounter_add_cleanup(
        &mut self,
        ledger: &mut BorethDeathLedger,
        actor: ExactActorRef,
        boss: &str,
        add: &str,
        minimum: Position,
        maximum: Position,
    ) -> Result<BorethDeathResult, CarrierError>;
}

#[derive(Debug, Default)]
pub struct BorethDeathLedger {
    entries: Vec<CreatureDeathOccurrenceKey>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BorethDeathResult {
    pub newly_committed: bool,
    pub removed: Vec<ExactActorRef>,
}
impl BorethCarrierExt for ChannelRuntimeV1 {
    /// Retire matching live adds after an exact death projected by the combat owner.
    fn commit_encounter_add_cleanup(
        &mut self,
        ledger: &mut BorethDeathLedger,
        actor: ExactActorRef,
        boss: &str,
        add: &str,
        minimum: Position,
        maximum: Position,
    ) -> Result<BorethDeathResult, CarrierError> {
        let owner = self.borrow_combat_death();
        if owner.creature_target_identity(actor)? != boss.as_bytes() {
            return Err(CarrierError::NotCreature);
        }
        let (death, _) = owner.projected_death(actor)?;
        ledger
            .entries
            .retain(|key| self.encounter_death_is_current(*key));
        if ledger.entries.contains(&death) {
            return Ok(BorethDeathResult {
                newly_committed: false,
                removed: Vec::new(),
            });
        }
        let context = self.encounter_position_context();
        let mut removed = Vec::new();
        removed
            .try_reserve(self.encounter_slots().len())
            .map_err(|_| CarrierError::AllocationFailed)?;
        for (i, slot) in self.encounter_slots().iter().enumerate() {
            if let Slot::CreatureOccupied {
                generation,
                target_identity,
                health,
                position: Some(p),
                ..
            } = slot
                && *health > 0
                && target_identity.as_ref() == add.as_bytes()
                && p.context == context
                && p.position.floor == -i16::from(minimum.floor)
                && (i32::from(minimum.x)..=i32::from(maximum.x)).contains(&p.position.x)
                && (i32::from(minimum.y)..=i32::from(maximum.y)).contains(&p.position.y)
            {
                removed.push(ExactActorRef(ActorRef::from_scoped_owner_parts(
                    self.binding().runtime_scope(),
                    self.binding().scope_generation(),
                    ActorLocalId(
                        u32::try_from(i + 1)
                            .map_err(|_| CarrierError::CapacityArithmeticOverflow)?,
                    ),
                    ActorLocalGeneration(*generation),
                )));
            }
        }
        // Preflight every fallible bound/ref/allocation before first physical owner mutation.
        ledger
            .entries
            .try_reserve(1)
            .map_err(|_| CarrierError::AllocationFailed)?;
        for actor in &removed {
            let index = self.encounter_actor_index(*actor)?;
            u32::try_from(index).map_err(|_| CarrierError::CapacityArithmeticOverflow)?;
        }
        for actor in &removed {
            self.remove_encounter_actor(*actor)?;
        }
        ledger.entries.push(death);
        Ok(BorethDeathResult {
            newly_committed: true,
            removed,
        })
    }
}

#[cfg(test)]
#[path = "native_death_tests.rs"]
mod tests;
