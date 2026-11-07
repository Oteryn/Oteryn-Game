//! Pure mapping of resolved spell effects to health changes and side effects.

use oteryn_gameplay_content::content::spell_data::{SpellDefinition, executable_catalog};

/// Calculation stages of a player cast, in the order the core ran them.
pub const STAGES: [&str; 2] = [
    "spell.formula.player_expression",
    "spell.draw.world_distribution",
];

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(
    clippy::large_enum_variant,
    reason = "transient owner result; boxing would add an allocation to the owner turn"
)]
pub enum ResolvedEffect {
    Damage {
        damage_type: String,
        magnitude: i64,
    },
    Heal {
        magnitude: i64,
    },
    RemoveCondition {
        condition: String,
    },
    Conjure {
        reagent: Option<u32>,
        result: u32,
        count: u32,
    },
    Unresolved {
        operation: String,
        effect: String,
    },
    ResolvedOther {
        profile: executable_catalog::EffectProfile,
        dependencies: executable_catalog::DependencyProfile,
    },
}

/// A heal of a self-target spell, or of a spell cast without a target, applies to the caster.
pub fn healed<'a>(spell: &SpellDefinition, caster: &'a str, target: Option<&'a str>) -> &'a str {
    if spell.self_target {
        caster
    } else {
        target.unwrap_or(caster)
    }
}

/// A resolved health change or an effect left to its existing runtime owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappedEffect<'a> {
    Damage { target: &'a str, magnitude: i64 },
    Heal { target: &'a str, magnitude: i64 },
    SideEffect(&'a ResolvedEffect),
}

/// A damage effect needs a resolved target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MissingTarget;

/// Map one effect in source order, before the runtime adapter validates its Ability atoms.
pub fn map_effect<'a>(
    resolved: &'a ResolvedEffect,
    target: Option<&'a str>,
    healed: &'a str,
) -> Result<Option<MappedEffect<'a>>, MissingTarget> {
    Ok(match resolved {
        ResolvedEffect::Damage { magnitude, .. } if *magnitude > 0 => Some(MappedEffect::Damage {
            target: target.ok_or(MissingTarget)?,
            magnitude: *magnitude,
        }),
        ResolvedEffect::Heal { magnitude } if *magnitude > 0 => Some(MappedEffect::Heal {
            target: healed,
            magnitude: *magnitude,
        }),
        // A zero draw changes no health; the Ability effects carry only positive magnitudes.
        ResolvedEffect::Damage { .. } | ResolvedEffect::Heal { .. } => None,
        other => Some(MappedEffect::SideEffect(other)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positive_damage_requires_a_target_but_zero_draws_do_not() {
        for magnitude in [0, -1] {
            assert_eq!(
                map_effect(
                    &ResolvedEffect::Damage {
                        damage_type: "fire".into(),
                        magnitude
                    },
                    None,
                    "caster"
                ),
                Ok(None)
            );
            assert_eq!(
                map_effect(&ResolvedEffect::Heal { magnitude }, None, "caster"),
                Ok(None)
            );
        }
        let damage = ResolvedEffect::Damage {
            damage_type: "fire".into(),
            magnitude: 7,
        };
        assert_eq!(map_effect(&damage, None, "caster"), Err(MissingTarget));
        assert_eq!(
            map_effect(&damage, Some("target"), "caster"),
            Ok(Some(MappedEffect::Damage {
                target: "target",
                magnitude: 7
            }))
        );
        assert_eq!(
            map_effect(&ResolvedEffect::Heal { magnitude: 7 }, None, "caster"),
            Ok(Some(MappedEffect::Heal {
                target: "caster",
                magnitude: 7
            }))
        );
        let other = ResolvedEffect::RemoveCondition {
            condition: "poison".into(),
        };
        assert_eq!(
            map_effect(&other, None, "caster"),
            Ok(Some(MappedEffect::SideEffect(&other)))
        );
    }
}
