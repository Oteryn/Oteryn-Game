//! Source Icicle raw addHealth(-100): reuse native damage staging, no armor/resistance pipeline.
use super::*;
impl CurrentOwnerExactActorCommit<'_> {
    pub(crate) fn commit_icicle_egg_raw_batch<T>(
        &mut self,
        actors: &[ExactActorRef],
        occurrence: &[u8],
        binding: &[u8],
        prepare_receipt: impl FnOnce(&[OwnerDamageResult]) -> Result<T, CarrierError>,
    ) -> Result<T, CarrierError> {
        if actors.len() > 64 {
            return Err(CarrierError::AllocationFailed);
        }
        let mut prepared = Vec::new();
        let mut results = Vec::new();
        prepared
            .try_reserve(actors.len())
            .map_err(|_| CarrierError::AllocationFailed)?;
        results
            .try_reserve(actors.len())
            .map_err(|_| CarrierError::AllocationFailed)?;
        for actor in actors {
            let (index, next, result) = self.carrier.prepare_creature_damage_inner(
                self.continuity,
                actor.0,
                OwnerDamageCommand {
                    target: b"oteryn:creature.dragon_egg",
                    occurrence,
                    binding,
                    damage: 100,
                },
                None,
                false,
            )?;
            if prepared.iter().any(|(prior, _)| *prior == index) {
                return Err(CarrierError::PlanConflict);
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
