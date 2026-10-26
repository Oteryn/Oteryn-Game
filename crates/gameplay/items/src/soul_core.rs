//! SOULPIT: two species cores become one random species core through the item craft owner.
//! Catalogue: canary:data/items/items.xml, primarytype="soul cores". The prism and ominous
//! core are excluded. Missing or unmaterializable active definitions fail before consumption.
use crate::item_craft::{CookingConversion, output_definition};
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_domain::domain::item_definition::TypedDefinitionRef;
use std::sync::OnceLock;

fn catalogue() -> Option<&'static [u32]> {
    static IDS: OnceLock<Option<Vec<u32>>> = OnceLock::new();
    IDS.get_or_init(|| serde_json::from_str(include_str!("soul_core_ids.json")).ok())
        .as_deref()
}

fn species_core(native: &NativeGameplayState, item: &TypedDefinitionRef) -> Option<()> {
    let id = item
        .production_key
        .strip_prefix("oteryn:item.tibia.i")?
        .parse()
        .ok()?;
    catalogue()?.binary_search(&id).ok()?;
    let record = native.item_policy_for_source_id(id)?.record();
    (output_definition(native, &record.production_binding.target)? == *item).then_some(())
}

/// Whether the active generation can materialize every outcome of core fusion.
/// Boot and session admission use this same closure before offering the capability.
pub fn soul_core_fusion_available(native: &NativeGameplayState) -> bool {
    catalogue().is_some_and(|ids| {
        !ids.is_empty()
            && ids.iter().all(|id| {
                native.item_policy_for_source_id(*id).is_some_and(|policy| {
                    output_definition(native, &policy.record().production_binding.target).is_some()
                })
            })
    })
}

/// A bounded, two-input fusion plan. The caller commits both decrements and this output
/// together, and retains the drawn output in the command receipt rather than redrawing it.
pub fn soul_core_fusion(
    native: &NativeGameplayState,
    used: &TypedDefinitionRef,
    target: &TypedDefinitionRef,
    draw: u64,
) -> Option<CookingConversion> {
    species_core(native, used)?;
    species_core(native, target)?;
    let ids = catalogue()?;
    let at = draw.checked_rem(u64::try_from(ids.len()).ok()?)?;
    let id = ids.get(usize::try_from(at).ok()?)?;
    let record = native.item_policy_for_source_id(*id)?.record();
    let output = &record.production_binding.target;
    Some(CookingConversion {
        recipe: "oteryn:soulpit.fuse".into(),
        outputs: vec![(output_definition(native, output)?, 1)],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soulpit_catalogue_is_sorted_bounded_and_excludes_special_items() -> Result<(), &'static str>
    {
        let ids = catalogue().ok_or("catalogue must parse")?;
        assert_eq!(ids.len(), 680);
        assert!(ids.windows(2).all(|w| w[0] < w[1]));
        assert!(ids.contains(&47384)); // Orc
        assert!(ids.contains(&47398)); // Rat
        for special in [37110, 49163, 49164] {
            assert!(!ids.contains(&special));
        }
        Ok(())
    }
}

#[derive(serde::Deserialize)]
struct Species {
    creature: String,
    revision: String,
    core: u32,
    class: String,
    difficulty: String,
    #[serde(rename = "occurrence")]
    _occurrence: String,
}

fn species() -> &'static [Species] {
    static SPECIES: OnceLock<Vec<Species>> = OnceLock::new();
    SPECIES.get_or_init(|| {
        serde_json::from_str(include_str!("soulpit_species.json")).unwrap_or_default()
    })
}

fn definition(native: &NativeGameplayState, id: u32) -> Option<TypedDefinitionRef> {
    output_definition(
        native,
        &native
            .item_policy_for_source_id(id)?
            .record()
            .production_binding
            .target,
    )
}

fn select(ids: &[u32], draw: u64) -> Option<u32> {
    let position = (u128::from(draw) * ids.len() as u128) >> 64;
    ids.get(usize::try_from(position).ok()?).copied()
}

/// Pinned Canary/Crystal drop behavior, not verified against Global. Called only for the
/// runtime owner's captured Fiendish classification. These rolls are independent of ordinary
/// loot bonuses; the selected entries then enter the existing retained corpse plan.
pub fn soulpit_acquisition(
    native: &NativeGameplayState,
    creature: &str,
    revision: &str,
    draws: [u64; 4],
) -> Option<Vec<TypedDefinitionRef>> {
    let own = species()
        .iter()
        .find(|s| s.creature == creature && s.revision == revision)?;
    let percent = |draw: u64| 1 + ((u128::from(draw) * 100) >> 64);
    let mut drops = Vec::new();
    if percent(draws[0]) < 5 {
        let id = if percent(draws[1]) <= 15 {
            Some(own.core)
        } else {
            let mut same_class: Vec<_> = species()
                .iter()
                .filter(|s| s.class == own.class)
                .map(|s| s.core)
                .collect();
            same_class.sort_unstable();
            same_class.dedup();
            select(&same_class, draws[2])
        };
        drops.push(definition(native, id?)?);
    }
    if percent(draws[3]) < 4 {
        drops.push(definition(native, 49164)?);
    }
    Some(drops)
}

fn difficulty(name: &str) -> Option<u8> {
    Some(match name.to_ascii_lowercase().as_str() {
        "harmless" => 0,
        "trivial" => 1,
        "easy" => 2,
        "medium" => 3,
        "hard" => 4,
        "challenging" | "challenge" => 5,
        _ => return None,
    })
}

/// Prism/Exalted conversion through the same atomic two-input owner. Reference probabilities
/// and boundary behavior have not been verified against Global. A Prism's ominous branch
/// preserves the target core; that retained core is returned with the ominous output.
pub fn soulpit_transformation(
    native: &NativeGameplayState,
    used: &TypedDefinitionRef,
    target: &TypedDefinitionRef,
    draw: u64,
    ominous_draw: u64,
) -> Option<CookingConversion> {
    let tool = if definition(native, 49164).as_ref() == Some(used) {
        49164
    } else if definition(native, 37110).as_ref() == Some(used) {
        37110
    } else {
        return None;
    };
    species_core(native, target)?;
    let own = species()
        .iter()
        .find(|s| format!("oteryn:item.tibia.i{}", s.core) == target.production_key)?;
    let current = difficulty(&own.difficulty)?;
    let desired = if tool == 49164 {
        current.saturating_add(1).min(5)
    } else {
        current.saturating_sub(1)
    };
    let mut candidates: Vec<_> = species()
        .iter()
        .filter(|s| difficulty(&s.difficulty) == Some(desired))
        .map(|s| s.core)
        .collect();
    candidates.sort_unstable();
    candidates.dedup();
    let output = definition(native, select(&candidates, draw)?)?;
    let ominous = tool == 49164 && ((u128::from(ominous_draw) * 100) >> 64) < 2;
    Some(CookingConversion {
        recipe: if ominous {
            "oteryn:soulpit.prism-ominous"
        } else if tool == 49164 {
            "oteryn:soulpit.prism"
        } else {
            "oteryn:soulpit.exalted"
        }
        .into(),
        outputs: if ominous {
            vec![(definition(native, 49163)?, 1), (target.clone(), 1)]
        } else {
            vec![(output, 1)]
        },
    })
}
