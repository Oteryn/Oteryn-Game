//! Pure carrier eligibility for the gated legacy USE qualification.
use super::ordinary_combat::has_world_geometry;
use oteryn_gameplay_content::content::spell_data::{
    Carrier, Execution, SpellDefinition, SpellEffect,
};

/// The legacy USE qualification admits Light Magic Missile and the single-tile
/// Fire Field carrier. Field creation still requires current World/tile policy
/// and ADMIT0 eligibility in the atomic spell-item writer.
pub fn direct_rune(spell: &SpellDefinition) -> bool {
    if has_world_geometry(spell) {
        return false;
    }
    match (&spell.carrier, &spell.execution) {
        (Carrier::Rune { item: 3174, .. }, Execution::Effects(effects)) => {
            !effects.is_empty()
                && effects.iter().all(|effect| {
                    matches!(
                        effect,
                        SpellEffect::Damage { .. } | SpellEffect::Heal { .. }
                    )
                })
        }
        (Carrier::Rune { item: 3188, .. }, Execution::Effects(effects)) => {
            effects.iter().any(|effect| {
                matches!(effect,
                SpellEffect::ResolvedOther { profile, .. }
                    if profile.operation == "create_item" && profile.created_item.is_some())
            }) && effects.iter().all(|effect| {
                matches!(effect,
                    SpellEffect::ResolvedOther { profile, .. }
                        if (profile.operation == "create_item" && profile.created_item.is_some())
                            || profile.operation == "presentation_only")
            })
        }
        _ => false,
    }
}
