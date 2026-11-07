//! Pure carrier eligibility for the gated legacy USE qualification.
use super::ordinary_combat::has_world_geometry;
use oteryn_gameplay_content::content::spell_data::{
    Carrier, Execution, SpellDefinition, SpellEffect,
};

/// Rune families implemented by the admitted USE adapter. Geometry, source Item
/// policy, World policy and payment remain qualified by the existing cast owner.
pub fn direct_rune(spell: &SpellDefinition) -> bool {
    match (&spell.carrier, &spell.execution) {
        (
            Carrier::Rune {
                item: 3177 | 3203, ..
            },
            Execution::NativeProfile(profile),
        ) => {
            let behavior = &profile.spell()["execution"]["native_behavior"];
            behavior["key"] == "acquire_summon"
                && matches!(
                    (&spell.carrier, behavior["parameters"]["source"].as_str()),
                    (Carrier::Rune { item: 3177, .. }, Some("target_creature"))
                        | (Carrier::Rune { item: 3203, .. }, Some("corpse_tile"))
                )
        }
        (Carrier::Rune { item: 3165, .. }, Execution::Effects(effects)) => {
            !has_world_geometry(spell)
                && effects.len() == 1
                && effects.iter().all(|effect| {
                    matches!(effect, SpellEffect::ResolvedOther { profile, .. }
                        if profile.operation == "condition"
                            && profile.condition.as_ref().is_some_and(|c| c.kind == "paralyze"))
                })
        }
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
        (
            Carrier::Rune {
                item: item @ (3148 | 3178 | 3197),
                ..
            },
            Execution::NativeProfile(profile),
        ) => {
            // docs/reference/spells/utility-runes-2026-10-29.md:9,12:
            // the support runes use the existing qualified tile-item owner.
            let native = &profile.spell()["execution"]["native_behavior"];
            native["key"] == "tile_item_operation"
                && native["parameters"]["operation"]
                    == match *item {
                        // Crystal 201189f8 data/scripts/runes/destroy_field_rune.lua:6-24.
                        3148 => "remove_field",
                        3178 => "mimic_item",
                        _ => "disintegrate",
                    }
        }
        _ => false,
    }
}
