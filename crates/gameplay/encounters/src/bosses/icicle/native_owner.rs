//! Source Icicle raw addHealth(-100): reuse native damage staging, no armor/resistance pipeline.
use {
    oteryn_gameplay_entities::{
        CurrentOwnerExactActorCommit, OwnerDamageCommand, OwnerDamageResult,
    },
    oteryn_gameplay_foundation::foundation::{CarrierError, ExactActorRef},
};

pub trait IcicleCarrierExt {
    fn commit_icicle_egg_raw_batch<T>(
        &mut self,
        actors: &[ExactActorRef],
        occurrence: &[u8],
        binding: &[u8],
        prepare_receipt: impl FnOnce(&[OwnerDamageResult]) -> Result<T, CarrierError>,
    ) -> Result<T, CarrierError>;
}
impl IcicleCarrierExt for CurrentOwnerExactActorCommit<'_> {
    fn commit_icicle_egg_raw_batch<T>(
        &mut self,
        actors: &[ExactActorRef],
        occurrence: &[u8],
        binding: &[u8],
        prepare_receipt: impl FnOnce(&[OwnerDamageResult]) -> Result<T, CarrierError>,
    ) -> Result<T, CarrierError> {
        self.commit_raw_creature_damage_batch(
            actors,
            OwnerDamageCommand {
                target: b"oteryn:creature.dragon_egg",
                occurrence,
                binding,
                damage: 100,
            },
            prepare_receipt,
        )
    }
}
