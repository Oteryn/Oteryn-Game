//! Bounded raw damage staging with receipt preflight before publication.
use super::*;
impl CurrentOwnerExactActorCommit<'_> {
    pub fn commit_raw_creature_damage_batch<T>(
        &mut self,
        actors: &[ExactActorRef],
        command: OwnerDamageCommand<'_>,
        prepare_receipt: impl FnOnce(&[OwnerDamageResult]) -> Result<T, CarrierError>,
    ) -> Result<T, CarrierError> {
        if actors.len() > 64 {
            return Err(
                oteryn_gameplay_domain::domain::carrier_values::CarrierError::AllocationFailed,
            );
        }
        let mut prepared = Vec::new();
        let mut results = Vec::new();
        prepared.try_reserve(actors.len()).map_err(|_| {
            oteryn_gameplay_domain::domain::carrier_values::CarrierError::AllocationFailed
        })?;
        results.try_reserve(actors.len()).map_err(|_| {
            oteryn_gameplay_domain::domain::carrier_values::CarrierError::AllocationFailed
        })?;
        for actor in actors {
            let (index, next, result) = self.carrier.prepare_creature_damage_inner(
                self.continuity,
                actor.0,
                command,
                None,
                false,
            )?;
            if prepared.iter().any(|(prior, _)| *prior == index) {
                return Err(
                    oteryn_gameplay_domain::domain::carrier_values::CarrierError::PlanConflict,
                );
            }
            prepared.push((index, next));
            results.push(result);
        }
        let receipt = prepare_receipt(&results)?;
        let mut deaths = Vec::new();
        deaths
            .try_reserve(prepared.len())
            .map_err(|_| CarrierError::CommitAllocationFailed)?;
        for ((actor, (_, next)), result) in actors.iter().zip(prepared.iter_mut()).zip(&results) {
            if let Some(next) = next {
                if result.applied && result.health_before > 0 && result.health_after == 0 {
                    let Slot::CreatureOccupied { committed, .. } = next else {
                        return Err(CarrierError::NotCreature);
                    };
                    committed.stamp_prepared_lethal_owner_time(self.now_ms);
                }
                if let Some(notice) =
                    self.carrier
                        .native_death_notice_for_slot(*actor, next, self.now_us)?
                {
                    deaths.push(notice);
                }
            }
        }
        self.carrier.reserve_native_death_capacity(&deaths)?;
        // Damage staging includes native lethal receipt, own conditions clear and receipt capacity.
        // No callbacks/allocations/possible errors remain at publication under this owner borrow.
        for (index, next) in prepared {
            if let Some(next) = next {
                self.carrier.slots[index] = next;
            }
        }
        self.carrier.install_native_death_events(deaths);
        Ok(receipt)
    }
}
