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
            return Err(oteryn_gameplay_foundation::foundation::CarrierError::AllocationFailed);
        }
        let mut prepared = Vec::new();
        let mut results = Vec::new();
        prepared
            .try_reserve(actors.len())
            .map_err(|_| oteryn_gameplay_foundation::foundation::CarrierError::AllocationFailed)?;
        results
            .try_reserve(actors.len())
            .map_err(|_| oteryn_gameplay_foundation::foundation::CarrierError::AllocationFailed)?;
        for actor in actors {
            let (index, next, result) = self.carrier.prepare_creature_damage_inner(
                self.continuity,
                actor.0,
                command,
                None,
                false,
            )?;
            if prepared.iter().any(|(prior, _)| *prior == index) {
                return Err(oteryn_gameplay_foundation::foundation::CarrierError::PlanConflict);
            }
            prepared.push((index, next));
            results.push(result);
        }
        let receipt = prepare_receipt(&results)?;
        // Damage staging includes native lethal receipt, own conditions clear and receipt capacity.
        // No callbacks/allocations/possible errors remain at publication under this owner borrow.
        for (index, next) in prepared {
            if let Some(next) = next {
                self.carrier.slots[index] = next;
            }
        }
        Ok(receipt)
    }
}
