//! Source-specific Crystal death handoff. No general Encounter interpreter and no persistence.
//! Consumers must implement their existing owner's mutation/publication contract.
use crate::foundation::{
    CarrierError, CharacterId, CreatureDeathOccurrenceKey, CurrentOwnerCombatDeath, ExactActorRef,
    MovementLocalPosition,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CrystalDeathSource {
    WeakSpot,
    LordRetro,
    HeraldOfFire,
}
impl CrystalDeathSource {
    fn from_owner_identity(key: &[u8]) -> Option<Self> {
        match key {
            b"oteryn:creature.weak_spot" => Some(Self::WeakSpot),
            b"oteryn:creature.lord_retro" => Some(Self::LordRetro),
            b"oteryn:creature.herald_of_fire" => Some(Self::HeraldOfFire),
            _ => None,
        }
    }
}
/// Acknowledgement from a real consuming owner; routing alone never reports a grant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CrystalConsumed {
    Applied,
    AlreadyApplied,
    OwnerUnavailable,
}

/// Specific requests carrying the physical owner's unforgeable committed death key.
/// Appearance and quest consumers own eligibility/fence/idempotency and exact mutation.
/// No caller-provided outcome string, character identity or HP fact is accepted by dispatch.
pub(crate) trait CrystalDeathConsumers {
    fn weak_spot_speech(
        &mut self,
        death: CreatureDeathOccurrenceKey,
        position: MovementLocalPosition,
        text: &'static str,
    ) -> CrystalConsumed;
    fn lord_retro_outfits(
        &mut self,
        death: CreatureDeathOccurrenceKey,
        damage_contributors: &[CharacterId],
    ) -> CrystalConsumed;
    fn herald_quest_and_portal(
        &mut self,
        death: CreatureDeathOccurrenceKey,
        most_damage_character: Option<CharacterId>,
    ) -> CrystalConsumed;
}

/// Call only inside the same Channel-owner work item as committed death projection.
/// A still-live/stale/other-scope actor fails before any consumer call. Repeated dispatch
/// carries the same sealed key: each real owner must replay its own first outcome.
/// Missing consumers return OwnerUnavailable, never a synthetic successful grant.
pub(crate) fn dispatch_crystal_committed_death(
    owner: &CurrentOwnerCombatDeath<'_>,
    actor: ExactActorRef,
    consumers: &mut impl CrystalDeathConsumers,
) -> Result<Option<CrystalConsumed>, CarrierError> {
    let source = CrystalDeathSource::from_owner_identity(owner.creature_target_identity(actor)?);
    let Some(source) = source else {
        return Ok(None);
    };
    let (death, position) = owner.projected_death(actor)?;
    Ok(Some(match source {
        CrystalDeathSource::WeakSpot => {
            consumers.weak_spot_speech(death, position, "The weak spot of the gates crashes!")
        }
        CrystalDeathSource::LordRetro => {
            consumers.lord_retro_outfits(death, &owner.damage_contributing_characters(actor)?)
        }
        CrystalDeathSource::HeraldOfFire => {
            consumers.herald_quest_and_portal(death, owner.top_damage_character(actor)?)
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_exact_owner_creature_keys_select_source_callbacks() {
        assert_eq!(
            CrystalDeathSource::from_owner_identity(b"oteryn:creature.weak_spot"),
            Some(CrystalDeathSource::WeakSpot)
        );
        assert_eq!(
            CrystalDeathSource::from_owner_identity(b"oteryn:creature.lord_retro"),
            Some(CrystalDeathSource::LordRetro)
        );
        assert_eq!(
            CrystalDeathSource::from_owner_identity(b"oteryn:creature.herald_of_fire"),
            Some(CrystalDeathSource::HeraldOfFire)
        );
        for rejected in [
            b"Weak Spot".as_slice(),
            b"oteryn:creature.weak_spot_extra",
            b"oteryn:creature.smelly_cheese",
            b"formal_dress_unlocked",
        ] {
            assert_eq!(CrystalDeathSource::from_owner_identity(rejected), None);
        }
    }
}
