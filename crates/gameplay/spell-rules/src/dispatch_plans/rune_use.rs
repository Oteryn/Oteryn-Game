//! Pure carrier eligibility for the gated legacy USE qualification.
use super::ordinary_combat::has_world_geometry;
use oteryn_gameplay_content::content::spell_data::{
    Carrier, Execution, SpellDefinition, SpellEffect,
};

/// Rune families implemented by the admitted USE adapter. Geometry, source Item
/// policy, World policy and payment remain qualified by the existing cast owner.
pub fn direct_rune(spell: &SpellDefinition) -> bool {
    match (&spell.carrier, &spell.execution) {
        (Carrier::Rune { item: 3174, .. }, Execution::Effects(effects)) => {
            !has_world_geometry(spell)
                && !effects.is_empty()
                && effects.iter().all(|effect| {
                    matches!(
                        effect,
                        SpellEffect::Damage { .. } | SpellEffect::Heal { .. }
                    )
                })
        }
        (
            Carrier::Rune {
                item: 3149 | 3164 | 3166 | 3172 | 3173 | 3176 | 3188 | 3190 | 3192,
                ..
            },
            Execution::Effects(effects),
        ) => {
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
        (
            Carrier::Rune {
                item: 3156 | 3180, ..
            },
            Execution::NativeProfile(profile),
        ) => profile.dependencies()["effects"]
            .as_array()
            .is_some_and(|effects| {
                effects.len() == 1
                    && effects[0]["operation"] == "create_item"
                    && effects[0]["duration_selection"] == "uniform_integer_seconds"
                    && effects[0].get("pvp_safe_item").is_some()
            }),
        _ => false,
    }
}
