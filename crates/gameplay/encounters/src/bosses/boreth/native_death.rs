use oteryn_gameplay_entities::carrier::Slot;
use oteryn_gameplay_entities::*;
pub trait BorethCarrierExt {
    #[allow(clippy::too_many_arguments)]
    fn commit_boreth_death(
        &mut self,
        ledger: &mut BorethDeathLedger,
        boreth: ExactActorRef,
    ) -> Result<BorethDeathResult, CarrierError>;
}
// Append beside native physical runtime_actor_carrier owner. No generic interpreter.
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
    /// Pinned Crystal 00ce02a5 BorethDeath: remove all exact Plaguethrower monsters in
    /// x32936..32944,y31474..31482,floor1; no caller role/name/position or reward authority.
    fn commit_boreth_death(
        &mut self,
        ledger: &mut BorethDeathLedger,
        boreth: ExactActorRef,
    ) -> Result<BorethDeathResult, CarrierError> {
        let owner = self.borrow_combat_death();
        if owner.creature_target_identity(boreth)? != b"oteryn:creature.boreth" {
            return Err(CarrierError::NotCreature);
        }
        let (death, _) = owner.projected_death(boreth)?;
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
                && target_identity.as_ref() == b"oteryn:creature.plaguethrower"
                && p.context == context
                && p.position.floor == 1
                && (32936..=32944).contains(&p.position.x)
                && (31474..=31482).contains(&p.position.y)
            {
                removed.push(ExactActorRef(ActorRef::from_owner_parts(
                    self.binding().world_id(),
                    self.binding().channel_id(),
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
