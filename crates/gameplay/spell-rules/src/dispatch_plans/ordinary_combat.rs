//! Pure authored spell applicability and source effect eligibility.
use super::geometry::PlanRejected;
use oteryn_gameplay_content::content::spell_data::executable_catalog::{
    AbilityProfile, DefinitionRef, EffectProfile, OperationalProfile,
};
use oteryn_gameplay_content::content::spell_data::{
    Carrier, Execution, SpellDefinition, SpellEffect,
};
use serde_json::Value;
use std::collections::BTreeSet;

pub fn applicable(spell: &SpellDefinition) -> bool {
    matches!(
        spell.execution,
        Execution::Effects(_) | Execution::AbilityVariants(_) | Execution::PartyBuff(_)
    )
}
pub fn has_world_geometry(spell: &SpellDefinition) -> bool {
    spell.chain.is_some()
        || spell.authored.as_ref().is_some_and(|p| {
            p.dependencies
                .abilities
                .iter()
                .any(|a| a.area.is_some() || a.chain.is_some())
        })
}
pub fn ability<'a>(
    spell: &'a SpellDefinition,
    variant: Option<&DefinitionRef>,
) -> Result<&'a AbilityProfile, PlanRejected> {
    let p = spell.authored.as_ref().ok_or(PlanRejected)?;
    let key = variant
        .or(p.header.execution.ability.as_ref())
        .ok_or(PlanRejected)?;
    p.dependencies
        .abilities
        .iter()
        .find(|a| a.identity.key == key.key && a.identity.revision == key.revision)
        .ok_or(PlanRejected)
}
pub fn is_source_self(spell: &SpellDefinition) -> bool {
    !has_world_geometry(spell)
        && matches!(spell.carrier, Carrier::Instant { .. })
        && !spell.needs_target
        && !spell.target_or_direction
        && spell.chain.is_none()
        && matches!(
            &spell.execution,
            Execution::Effects(_) | Execution::AbilityVariants(_) | Execution::ActorFocus { .. }
        )
        || matches!(&spell.execution, Execution::NativeProfile(p)
            if matches!(p.spell()["execution"]["native_behavior"]["key"].as_str(),Some("monk_focus"|"companion_haste")))
}

/// LOOP-PROGRESS-1: shared-experience heal credit needs this attempt's transaction-qualified
/// SourcePartyWorld, so every spell whose source effects can heal a player captures it,
/// whatever the caster vocation (a druid's Heal Friend or Mass Healing). Any other
/// execution captures nothing and therefore can never earn heal credit.
pub fn may_heal_player(spell: &SpellDefinition) -> bool {
    let heals = |effects: &[SpellEffect]| {
        effects
            .iter()
            .any(|effect| matches!(effect, SpellEffect::Heal { .. }))
    };
    match &spell.execution {
        Execution::Effects(effects) => heals(effects),
        Execution::AbilityVariants(variants) => {
            variants.iter().any(|variant| heals(&variant.effects))
        }
        Execution::NativeProfile(profile) => {
            let behavior = &profile.spell()["execution"]["native_behavior"];
            behavior["key"] == "mass_spirit_mend"
                || (behavior["key"] == "wheel_combat"
                    && behavior["parameters"]["element"] == "healing")
        }
        Execution::Conjure { .. } | Execution::PartyBuff(_) | Execution::ActorFocus { .. } => false,
    }
}

pub fn wheel_stage_without_owner(
    wheel: &Value,
    unavailable_wheel: bool,
    has_current_wheel: bool,
) -> Result<u8, PlanRejected> {
    // Source roles B/C keep their base behavior without a current Wheel grant. This is an
    // execution fallback, never positive stage evidence; role A remains refused before costs.
    if !has_current_wheel && unavailable_wheel && wheel["stage_zero"].as_str() == Some("base_cast")
    {
        Ok(0)
    } else {
        Err(PlanRejected)
    }
}

/// Values selecting the ordinary recipients after the owners qualify targets and geometry.
pub struct TargetSelection<'a> {
    pub area: Option<&'a BTreeSet<super::geometry::TilePosition>>,
    pub target: Option<u64>,
    pub caster: u64,
    pub self_target: bool,
    pub target_or_direction: bool,
    pub front: Option<super::geometry::TilePosition>,
}

impl TargetSelection<'_> {
    pub fn select(
        &self,
        creatures: impl Iterator<Item = (u64, super::geometry::TilePosition)>,
        legal: &BTreeSet<u64>,
        mut area_sight_clear: impl FnMut(super::geometry::TilePosition) -> bool,
    ) -> Result<Vec<u64>, PlanRejected> {
        Ok(if let Some(geometry) = self.area {
            creatures
                .filter(|(id, position)| {
                    geometry.contains(position) && legal.contains(id) && area_sight_clear(*position)
                })
                .map(|(id, _)| id)
                .collect()
        } else if let Some(target) = self.target {
            vec![target]
        } else if self.self_target {
            vec![self.caster]
        } else if self.target_or_direction {
            let front = self.front.ok_or(PlanRejected)?;
            creatures
                .filter(|(id, position)| *position == front && legal.contains(id))
                .map(|(id, _)| id)
                .collect()
        } else {
            Vec::new()
        })
    }
}

pub fn block_armor(spell: &SpellDefinition) -> Result<bool, PlanRejected> {
    Ok(spell
        .authored
        .as_ref()
        .ok_or(PlanRejected)?
        .dependencies
        .effects
        .iter()
        .any(|e| {
            e.mitigated_by
                .as_ref()
                .is_some_and(|m| m.iter().any(|m| m == "armor"))
        }))
}

pub fn selected_effects<'a>(
    source: &'a OperationalProfile,
    selected_ability: Option<&AbilityProfile>,
) -> Result<Vec<&'a EffectProfile>, PlanRejected> {
    if let Some(a) = selected_ability {
        a.effects
            .iter()
            .map(|reference| {
                source
                    .dependencies
                    .effects
                    .iter()
                    .find(|e| {
                        e.identity.key == reference.key && e.identity.revision == reference.revision
                    })
                    .ok_or(PlanRejected)
            })
            .collect::<Result<Vec<_>, _>>()
    } else {
        Ok(source.dependencies.effects.iter().collect::<Vec<_>>())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::super::geometry::{attack_target_origin, geometry};
    use super::*;
    use serde_json::Value;

    fn source(key: &str) -> SpellDefinition {
        let catalog: Value = serde_json::from_slice(include_bytes!(
            "../../../../../tools/content-schema/spell-authoring/samples/executable-spell-catalog.json"
        ))
        .unwrap();
        let row = catalog["bundles"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["bundle"]["spell"]["identity"]["key"] == key)
            .unwrap();
        oteryn_gameplay_content::content::spell_data::authoring::spell_from_bundle(
            &row["bundle"],
            &row["dependencies"],
        )
        .unwrap()
    }
    #[test]
    fn directional_held_target_keeps_the_caster_as_geometry_origin() {
        let tile = |x, y| (x, y, 7);
        let caster = tile(10, 10);
        let target = tile(14, 10);
        // Fire Wave / Energy Beam: origin is the tile in front of the caster, facing the target.
        assert_eq!(
            attack_target_origin(caster, target, true).unwrap(),
            (tile(11, 10), Some((1, 0)))
        );
        assert_eq!(
            attack_target_origin(caster, tile(8, 8), true).unwrap(),
            (tile(9, 9), Some((-1, -1)))
        );
        // A single-target spell is centred on the target.
        assert_eq!(
            attack_target_origin(caster, target, false).unwrap(),
            (target, None)
        );
    }
    #[test]
    fn genuine_berserk_and_cancel_invisibility_are_world_areas_not_self_mutations() {
        let center = (100, 100, 7);
        for key in [
            "candidate:spell/berserk",
            "candidate:spell/cancel_invisibility",
        ] {
            let spell = source(key);
            assert!(applicable(&spell));
            assert!(has_world_geometry(&spell));
            assert!(!is_source_self(&spell));
            let cells = geometry(ability(&spell, None).unwrap(), center, (0, -1)).unwrap();
            assert!(cells.contains(&center));
            assert!(cells.contains(&(101, 100, 7)));
        }
    }
    #[test]
    fn genuine_fire_wave_keeps_diagonal_matrix_and_rotates_owner_direction() {
        let spell = source("candidate:spell/fire_wave");
        let a = ability(&spell, None).unwrap();
        let center = (100, 100, 7);
        let north = geometry(a, center, (0, -1)).unwrap();
        let east = geometry(a, center, (1, 0)).unwrap();
        // Canary AREA_WAVE4 uses 3 (active origin), not 2 (origin only).
        assert!(north.contains(&center));
        assert_eq!(north.len(), 12);
        assert!(north.contains(&(100, 99, 7)));
        assert_eq!(north.len(), east.len());
        assert!(east.contains(&(101, 100, 7)));
        let northwest = geometry(a, center, (-1, -1)).unwrap();
        assert!(northwest.contains(&center));
        assert_eq!(northwest.len(), 16);
        assert!(northwest.contains(&(99, 99, 7)));
        assert!(northwest.len() > north.len());
        assert!(!is_source_self(&spell));
    }

    #[test]
    fn ordinary_area_and_directional_targets_keep_qualified_order_and_visibility() {
        let creatures = [
            (4, (1, 0, 7)),
            (2, (2, 0, 7)),
            (3, (3, 0, 7)),
            (5, (4, 0, 7)),
        ];
        let area = BTreeSet::from([(1, 0, 7), (2, 0, 7), (3, 0, 7)]);
        let legal = BTreeSet::from([2, 4, 5]);
        let mut selection = TargetSelection {
            area: Some(&area),
            target: Some(99),
            caster: 1,
            self_target: true,
            target_or_direction: true,
            front: Some((1, 0, 7)),
        };
        assert_eq!(
            selection.select(creatures.into_iter(), &legal, |p| p != (2, 0, 7)),
            Ok(vec![4])
        );
        selection.area = None;
        assert_eq!(
            selection.select(creatures.into_iter(), &legal, |_| false),
            Ok(vec![99])
        );
        selection.target = None;
        assert_eq!(
            selection.select(creatures.into_iter(), &legal, |_| false),
            Ok(vec![1])
        );
        selection.self_target = false;
        assert_eq!(
            selection.select(creatures.into_iter(), &legal, |_| false),
            Ok(vec![4])
        );
        selection.front = None;
        assert_eq!(
            selection.select(creatures.into_iter(), &legal, |_| false),
            Err(PlanRejected)
        );
        selection.target_or_direction = false;
        assert_eq!(
            selection.select(creatures.into_iter(), &legal, |_| false),
            Ok(vec![])
        );
    }

    #[test]
    fn missing_wheel_keeps_only_explicit_stage_zero_base_cast() {
        use serde_json::json;
        assert_eq!(
            wheel_stage_without_owner(&json!({"stage_zero":"base_cast"}), true, false),
            Ok(0)
        );
        for policy in [
            json!({"stage_zero":"reject_before_costs"}),
            json!({}),
            json!({"stage_zero":"invented"}),
        ] {
            assert_eq!(
                wheel_stage_without_owner(&policy, true, false),
                Err(PlanRejected)
            );
        }
        assert_eq!(
            wheel_stage_without_owner(&json!({"stage_zero":"base_cast"}), false, false),
            Err(PlanRejected)
        );
        assert_eq!(
            wheel_stage_without_owner(&json!({"stage_zero":"base_cast"}), true, true),
            Err(PlanRejected)
        );
    }
}
