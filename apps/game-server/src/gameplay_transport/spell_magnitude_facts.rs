//! Numerical observations from the already fenced Character/Equipment/Wheel reads.
//! Baseline approximation is selected by qualified Content, never by a client or env flag.
use super::spell_access_facts::AccessFactsError;
use crate::content::native_gameplay::NativeGameplayState;
use crate::durability::character_equipment::RawCastDurableFacts;
use crate::foundation::CharacterId;
use crate::spell::magnitude_owner::{
    MagnitudeOwnedAttributes, MagnitudeOwnerBinding, MagnitudePolicy,
};
use crate::spell::owned_cast_facts::CastFactsBinding;

pub(crate) fn project(
    raw: &RawCastDurableFacts,
    binding: &CastFactsBinding,
    native: &NativeGameplayState,
) -> Result<Option<MagnitudeOwnedAttributes>, AccessFactsError> {
    let equipment = &raw.equipment;
    if equipment.character != binding.character
        || equipment.character_revision != binding.character_revision
        || equipment.revision != binding.equipment_revision
        || equipment.content_digest != binding.content_digest
        || native.source_digest() != binding.content_digest
        || raw.level == 0
        || binding.character_revision == 0
        || binding.equipment_revision == 0
    {
        return Err(AccessFactsError::Unavailable(
            "magnitude durable owner binding",
        ));
    }
    // No advanced modifier owner is registered yet. Strict Content keeps its refusal.
    if native.magnitude_policy() == MagnitudePolicy::Strict {
        return Ok(None);
    }
    for item in &equipment.items {
        if item.quantity == 0
            || item.state_revision == 0
            || native
                .item_policy(
                    &item.definition.production_key,
                    &item.definition.revision_ref,
                )
                .is_none()
        {
            return Err(AccessFactsError::Unavailable(
                "magnitude current Item policy",
            ));
        }
    }
    // Main's Wheel owner admits no runtime effects (SPELL-WHEEL-GATE-1); the flat Wheel
    // bonus stays unknown and the baseline policy resolves it to zero.
    let wheel_flat: Option<(i32, i32)> = None;
    let wheel_revision: Option<u64> = None;
    // Empty actual equipment proves no equipment bonus. Nonempty equipment has not
    // qualified modifier units, phases and stacking; its numeric fields stay unknown.
    let empty_equipment = equipment.items.is_empty().then_some(0);
    let attributes = MagnitudeOwnedAttributes {
        policy: native.magnitude_policy(),
        owner_revisions: Some((
            equipment.character_revision,
            equipment.revision,
            wheel_revision,
        )),
        binding: MagnitudeOwnerBinding {
            actor: binding.actor,
            session: binding.session,
            character: CharacterId::decode(&binding.character)
                .map_err(|_| AccessFactsError::Unavailable("magnitude Character identity"))?,
            lease_generation: binding.lease_generation,
            player_revision: binding.player_revision,
            content_digest: binding.content_digest,
        },
        base_critical_chance_permyriad: None,
        base_critical_extra_permyriad: None,
        equipment_critical_chance_permyriad: empty_equipment,
        equipment_critical_extra_permyriad: empty_equipment.map(i32::from),
        fatal_chance_permyriad: None,
        armor_penetration_permyriad: None,
        elemental_pierce_permyriad: None,
        wheel_flat_damage: wheel_flat.map(|value| value.0),
        wheel_flat_healing: wheel_flat.map(|value| value.1),
        source_damage_multiplier_percent: None,
        source_healing_multiplier_percent: None,
        healing_dealt_percent: None,
        monster_armor_disabled: None,
        inactive_extra_stages: None,
    };
    #[cfg(test)]
    eprintln!(
        "spell magnitude baseline omitted={:?}",
        attributes.omitted_modifiers()
    );
    Ok(Some(attributes))
}
