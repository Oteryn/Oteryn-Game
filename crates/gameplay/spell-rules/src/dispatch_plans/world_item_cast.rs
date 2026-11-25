//! Pure world Item grant eligibility, candidate identities and decay rules.
pub use super::geometry::PlanRejected;
use oteryn_gameplay_content::content::native_items::NativeItemRef;
use oteryn_gameplay_content::content::spell_data::executable_catalog::Conjure;
use oteryn_gameplay_content::content::spell_data::native::CompiledNativeSpell;
use oteryn_gameplay_content::content::spell_data::{Carrier, Execution, SpellDefinition};
use oteryn_gameplay_content::content::{
    ReferenceItemField as F, ReferenceItemTarget, ReferenceItemTemporal, ReferenceMilliseconds,
};
use serde_json::Value;

/// Value projection from a source-qualified Item definition, without custody or commit authority.
#[derive(Debug, Clone, Copy)]
pub struct ItemProperties {
    pub movable: bool,
    pub stack_maximum: u32,
    pub container_capacity: Option<u16>,
    pub inventory_destination: bool,
    pub ground_destination: bool,
}

pub fn grant_allowed(item: ItemProperties) -> bool {
    item.inventory_destination
        && item.ground_destination
        && item.movable
        && item.stack_maximum != 0
        && item.stack_maximum <= 100
}

/// Source ItemType decay selection. The adapter supplies each qualified target identity.
pub fn resolve_decay<D>(
    temporal: &F<ReferenceItemTemporal>,
    mut resolve_target: impl FnMut(&ReferenceItemTarget) -> Result<(D, ItemProperties), PlanRejected>,
) -> Result<Option<(u32, Option<D>)>, PlanRejected> {
    Ok(match temporal {
        F::NotApplicable => None,
        F::Known(temporal)
            if matches!(temporal.duration, F::Known(ReferenceMilliseconds(0)))
                && matches!(temporal.decay_target, F::NotApplicable) =>
        {
            None
        }
        F::Known(temporal) if matches!(temporal.stop_duration, F::Known(false)) => {
            let F::Known(ReferenceMilliseconds(duration)) = temporal.duration else {
                return Err(PlanRejected);
            };
            let duration = u32::try_from(duration)
                .ok()
                .filter(|d| *d > 0)
                .ok_or(PlanRejected)?;
            let target = match &temporal.decay_target {
                F::NotApplicable => None,
                F::Known(target) => {
                    let (definition, target) = resolve_target(target)?;
                    if target.stack_maximum != 1
                        || target.container_capacity.is_some()
                        || !target.inventory_destination
                        || !target.ground_destination
                    {
                        return Err(PlanRejected);
                    }
                    Some(definition)
                }
                _ => return Err(PlanRejected),
            };
            Some((duration, target))
        }
        _ => return Err(PlanRejected),
    })
}

pub fn source_profiles(
    spell: &SpellDefinition,
    has_target: bool,
    aim_at_target: bool,
) -> Result<(Option<&CompiledNativeSpell>, Option<&Conjure>), PlanRejected> {
    let food_profile = match &spell.execution {
        Execution::NativeProfile(profile)
            if profile.spell()["execution"]["native_behavior"]["key"] == "random_item_grant" =>
        {
            Some(profile)
        }
        _ => None,
    };
    let conjure = match &spell.execution {
        Execution::Conjure { .. } => spell
            .authored
            .as_ref()
            .and_then(|s| s.header.execution.conjure.as_ref()),
        _ => None,
    };
    if food_profile.is_none() && conjure.is_none() {
        return Err(PlanRejected);
    }
    // Conjure casts may be aggressive. The normal cast checks enforce their
    // protection-zone and condition gates using the qualified caster tile.
    if has_target
        || aim_at_target
        || (food_profile.is_some() && spell.aggressive)
        || !matches!(spell.carrier, Carrier::Instant { .. })
    {
        return Err(PlanRejected);
    }
    Ok((food_profile, conjure))
}

pub fn grant_references(
    food_profile: bool,
    parameters: &Value,
    conjure: Option<&Conjure>,
) -> Result<Vec<NativeItemRef>, PlanRejected> {
    if food_profile {
        parameters["pool"]
            .as_array()
            .ok_or(PlanRejected)?
            .iter()
            .map(|item| {
                Ok(NativeItemRef {
                    key: item["key"].as_str().ok_or(PlanRejected)?.into(),
                    revision: item["revision"].as_str().ok_or(PlanRejected)?.into(),
                })
            })
            .collect::<Result<_, PlanRejected>>()
    } else {
        let result = &conjure.ok_or(PlanRejected)?.result;
        Ok(vec![NativeItemRef {
            key: result.key.clone(),
            revision: result.revision.clone(),
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ITEM: ItemProperties = ItemProperties {
        movable: true,
        stack_maximum: 1,
        container_capacity: None,
        inventory_destination: true,
        ground_destination: true,
    };

    #[test]
    fn blank_rune_uses_normal_aggressive_cast_admission() -> Result<(), Box<dyn std::error::Error>>
    {
        let catalogue: Value = serde_json::from_slice(include_bytes!(
            "../../../../../tools/content-schema/spell-authoring/samples/executable-spell-catalog.json"
        ))?;
        let row = catalogue["bundles"]
            .as_array()
            .and_then(|rows| {
                rows.iter().find(|row| {
                    row["bundle"]["spell"]["identity"]["key"] == "candidate:spell/blank_rune"
                })
            })
            .ok_or("Blank Rune source bundle")?;
        let spell = oteryn_gameplay_content::content::spell_data::authoring::spell_from_bundle(
            &row["bundle"],
            &row["dependencies"],
        )
        .map_err(|error| format!("{error:?}"))?;
        let profile = spell.authored.as_ref().ok_or("Blank Rune source profile")?;
        assert!(spell.aggressive);
        assert!(super::super::cast_legality::protection_rejected(
            profile, true
        ));
        assert!(!super::super::cast_legality::protection_rejected(
            profile, false
        ));
        let (food, conjure) = source_profiles(&spell, false, false)
            .map_err(|error| format!("blank rune source dispatch: {error:?}"))?;
        assert!(food.is_none());
        assert!(conjure.is_some_and(|conjure| conjure.reagent.is_none() && conjure.count == 1));
        assert!(source_profiles(&spell, true, false).is_err());
        assert!(source_profiles(&spell, false, true).is_err());
        Ok(())
    }

    #[test]
    fn grant_and_decay_keep_unknown_fields_unavailable_and_retain_exact_target() {
        assert!(grant_allowed(ITEM));
        for invalid in [
            ItemProperties {
                movable: false,
                ..ITEM
            },
            ItemProperties {
                stack_maximum: 0,
                ..ITEM
            },
            ItemProperties {
                stack_maximum: 101,
                ..ITEM
            },
            ItemProperties {
                inventory_destination: false,
                ..ITEM
            },
            ItemProperties {
                ground_destination: false,
                ..ITEM
            },
        ] {
            assert!(!grant_allowed(invalid));
        }
        let never = |_: &ReferenceItemTarget| -> Result<(u32, ItemProperties), PlanRejected> {
            Err(PlanRejected)
        };
        assert_eq!(resolve_decay(&F::NotApplicable, never), Ok(None));
        for temporal in [F::Unknown, F::Conflict] {
            assert_eq!(resolve_decay(&temporal, never), Err(PlanRejected));
        }
        let target = ReferenceItemTarget {
            key: "candidate:item/100".into(),
            revision: "1".into(),
        };
        let mut temporal = ReferenceItemTemporal {
            consumption_mode: F::Unknown,
            duration: F::Known(ReferenceMilliseconds(1000)),
            stop_duration: F::Known(false),
            decay_target: F::Known(target.clone()),
        };
        assert_eq!(
            resolve_decay(&F::Known(temporal.clone()), |actual| {
                assert_eq!(actual, &target);
                Ok((42, ITEM))
            }),
            Ok(Some((1000, Some(42))))
        );
        for invalid in [
            ItemProperties {
                stack_maximum: 2,
                ..ITEM
            },
            ItemProperties {
                container_capacity: Some(1),
                ..ITEM
            },
            ItemProperties {
                inventory_destination: false,
                ..ITEM
            },
            ItemProperties {
                ground_destination: false,
                ..ITEM
            },
        ] {
            assert_eq!(
                resolve_decay(&F::Known(temporal.clone()), |_| Ok((42, invalid))),
                Err(PlanRejected)
            );
        }
        temporal.decay_target = F::Unknown;
        assert_eq!(
            resolve_decay(&F::Known(temporal.clone()), never),
            Err(PlanRejected)
        );
        temporal.decay_target = F::NotApplicable;
        assert_eq!(
            resolve_decay(&F::Known(temporal.clone()), never),
            Ok(Some((1000, None)))
        );
        for duration in [0, u64::from(u32::MAX) + 1] {
            temporal.duration = F::Known(ReferenceMilliseconds(duration));
            let expected = if duration == 0 {
                Ok(None)
            } else {
                Err(PlanRejected)
            };
            assert_eq!(resolve_decay(&F::Known(temporal.clone()), never), expected);
        }
    }
}
