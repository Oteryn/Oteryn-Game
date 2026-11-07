//! W18: source-authored decoration USE and rotation in the named House owner.
use crate::novelty_items::NoveltyPlan;
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_domain::domain::item_definition::TypedDefinitionRef;
use serde::Deserialize;
use serde_json::json;
use std::sync::OnceLock;

#[derive(Deserialize)]
struct Catalogue {
    construction: Vec<[u32; 2]>,
    npc_wrapped_offers: Vec<NpcFurnitureOffer>,
    r#use: Vec<[u32; 2]>,
    #[serde(default)]
    portable_use: Vec<[u32; 2]>,
    #[serde(default)]
    portable_side: Vec<u32>,
    #[serde(default)]
    use_speech: Vec<(u32, String)>,
    rotate: Vec<[u32; 2]>,
    #[serde(default)]
    side_actions: Vec<side_actions::SideRule>,
    #[serde(default)]
    guarded_use: Vec<u32>,
    #[serde(default)]
    golden_display: Vec<[u32; 2]>,
    #[serde(default)]
    carpet_placement: Vec<[u32; 2]>,
    #[serde(default)]
    carpets: Vec<[u32; 2]>,
    #[serde(default)]
    lifetimes: Vec<(u32, Option<u64>, Option<u32>, bool)>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct NpcFurnitureOffer {
    pub source_row: u32,
    pub npc_key: String,
    pub item_id: u32,
    pub price: u64,
}
pub fn npc_offer_for_row(row: u32) -> Option<&'static NpcFurnitureOffer> {
    catalogue()?
        .npc_wrapped_offers
        .iter()
        .find(|offer| offer.source_row == row)
}
pub fn npc_wrapped_offer(npc: &str, key: &str, price: u64) -> Option<&'static NpcFurnitureOffer> {
    let id = key
        .strip_prefix("oteryn:item.tibia.i")?
        .parse::<u32>()
        .ok()?;
    catalogue()?
        .npc_wrapped_offers
        .iter()
        .find(|offer| offer.npc_key == npc && offer.item_id == id && offer.price == price)
}

mod side_actions;
pub use side_actions::{plan_interactive, plan_portable_effect, side_rule};
pub fn is_carpet(key: &str) -> bool {
    let Some(id) = key
        .strip_prefix("oteryn:item.tibia.i")
        .and_then(|id| id.parse::<u32>().ok())
    else {
        return false;
    };
    catalogue().is_some_and(|rules| {
        rules
            .carpets
            .binary_search_by_key(&id, |row| row[0])
            .is_ok()
    })
}
/// Paused furniture uses the existing House time-budget owner, not an absolute mint deadline.
pub fn pauses_duration(key: &str) -> bool {
    let Some(id) = key
        .strip_prefix("oteryn:item.tibia.i")
        .and_then(|id| id.parse::<u32>().ok())
    else {
        return false;
    };
    catalogue().is_some_and(|rules| {
        rules
            .lifetimes
            .iter()
            .any(|(i, duration, _, stops)| *i == id && duration.is_none() && *stops)
    })
}

pub fn is_golden_display(key: &str) -> bool {
    let Some(id) = key
        .strip_prefix("oteryn:item.tibia.i")
        .and_then(|id| id.parse::<u32>().ok())
    else {
        return false;
    };
    catalogue().is_some_and(|rules| rules.golden_display.iter().any(|pair| pair[0] == id))
}

pub fn requires_house_preflight(key: &str) -> bool {
    let Some(id) = key
        .strip_prefix("oteryn:item.tibia.i")
        .and_then(|id| id.parse::<u32>().ok())
    else {
        return false;
    };
    speech(key).is_some()
        || side_rule(key).is_some()
        || catalogue().is_some_and(|rules| rules.guarded_use.binary_search(&id).is_ok())
}

fn catalogue() -> Option<&'static Catalogue> {
    static RULES: OnceLock<Option<Catalogue>> = OnceLock::new();
    RULES
        .get_or_init(|| {
            serde_json::from_str(include_str!(
                "../../../../tools/content-schema/native-gameplay/furniture-rules.json"
            ))
            .ok()
        })
        .as_ref()
}

/// Ordinary USE selects only the donor's independent USE action. In particular,
/// using a bookcase still reaches its container owner rather than rotating it.
pub fn transform_to(key: &str) -> Option<u32> {
    let id = key
        .strip_prefix("oteryn:item.tibia.i")?
        .parse::<u32>()
        .ok()?;
    let rules = &catalogue()?.r#use;
    rules
        .binary_search_by_key(&id, |row| row[0])
        .ok()
        .map(|at| rules[at][1])
}

/// Pinned ordinary portable actions exclude every House-only carpet and kit.
pub fn portable_use_to(key: &str) -> Option<u32> {
    let id = key
        .strip_prefix("oteryn:item.tibia.i")?
        .parse::<u32>()
        .ok()?;
    let rules = &catalogue()?.portable_use;
    rules
        .binary_search_by_key(&id, |r| r[0])
        .ok()
        .map(|at| rules[at][1])
}
pub fn is_portable_effect(key: &str) -> bool {
    key.strip_prefix("oteryn:item.tibia.i")
        .and_then(|s| s.parse::<u32>().ok())
        .is_some_and(|id| catalogue().is_some_and(|r| r.portable_side.binary_search(&id).is_ok()))
}
pub fn speech(key: &str) -> Option<&'static str> {
    let id = key
        .strip_prefix("oteryn:item.tibia.i")?
        .parse::<u32>()
        .ok()?;
    catalogue()?
        .use_speech
        .iter()
        .find(|(source, _)| *source == id)
        .map(|(_, text)| text.as_str())
}
pub fn plan_portable_use(
    native: &NativeGameplayState,
    used: &TypedDefinitionRef,
) -> Option<NoveltyPlan> {
    let mut plan = plan(native, used, portable_use_to(&used.production_key)?, "use")?;
    plan.effects["kind"] = json!("transform");
    plan.effects["portable_use"] = json!(true);
    plan.recipe = format!(
        "oteryn:novelty-item.furniture-use.{}.v1",
        used.production_key
    );
    Some(plan)
}
/// Native USE of an acquired source kit selects its source-authored furniture.
pub fn construction_to(key: &str) -> Option<u32> {
    let id = key
        .strip_prefix("oteryn:item.tibia.i")?
        .parse::<u32>()
        .ok()?;
    let all = catalogue()?;
    if let Ok(at) = all.carpet_placement.binary_search_by_key(&id, |row| row[0]) {
        return Some(all.carpet_placement[at][1]);
    }
    let rules = &all.construction;
    rules
        .binary_search_by_key(&id, |row| row[0])
        .ok()
        .map(|at| rules[at][1])
}

/// An explicit Rotate never selects the independent USE transition.
pub fn rotate_to(key: &str) -> Option<u32> {
    let id = key
        .strip_prefix("oteryn:item.tibia.i")?
        .parse::<u32>()
        .ok()?;
    let rules = &catalogue()?.rotate;
    rules
        .binary_search_by_key(&id, |row| row[0])
        .ok()
        .map(|at| rules[at][1])
}

pub fn plan_rotate(native: &NativeGameplayState, used: &TypedDefinitionRef) -> Option<NoveltyPlan> {
    plan(native, used, rotate_to(&used.production_key)?, "rotate")
}

/// The current native generation must independently admit both source and result.
/// The durable owner supplies exact custody and keeps the instance and reclaim provenance.
pub fn plan_house(native: &NativeGameplayState, used: &TypedDefinitionRef) -> Option<NoveltyPlan> {
    plan(native, used, transform_to(&used.production_key)?, "use")
}

fn plan(
    native: &NativeGameplayState,
    used: &TypedDefinitionRef,
    target: u32,
    action: &str,
) -> Option<NoveltyPlan> {
    let source = native
        .item_policy(&used.production_key, &used.revision_ref)?
        .record();
    let after = native.item_policy_for_source_id(target)?.record();
    if used.family != "Item"
        || source.production_definition.family != used.family
        || !source.admission.materializable
        || !after.admission.materializable
    {
        return None;
    }
    let mut effects = serde_json::to_value(crate::novelty_items::NoveltyEffects {
        kind: crate::novelty_items::NoveltyKind::HouseTransform,
        source_item_id: used
            .production_key
            .strip_prefix("oteryn:item.tibia.i")?
            .parse::<u32>()
            .ok()?,
        donor: "crystal-201189f8:items.xml;decay_to.lua;carpets.lua;bath_tub_drain.lua".into(),
        ground: None,
        cues: if action == "use" && is_golden_display(&used.production_key) {
            vec![crate::novelty_items::NoveltyCue {
                binding: "appearance:effect/donor_171".into(),
                effect_id: 171,
            }]
        } else {
            Vec::new()
        },
        speech: speech(&used.production_key).map(str::to_owned),
        private_speech: false,
        require_head: false,
        require_player_target: false,
        dice_value: None,
        costume: None,
        damage: None,
        achievement: None,
        achievement_progress: Vec::new(),
        timed_output: None,
    })
    .ok()?;
    if action == "rotate" {
        effects["house_rotate"] = json!(true);
    }
    Some(NoveltyPlan {
        recipe: format!("oteryn:house-furniture.{action}.{}.v1", used.production_key),
        outputs: vec![(
            TypedDefinitionRef {
                family: after.production_definition.family.clone(),
                production_key: after.production_definition.production_key.clone(),
                revision_ref: after.production_definition.revision_ref.clone(),
            },
            1,
        )],
        consume_source: false,
        effects,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn srv7_rotate_refuses_xml_only_pairs_without_official_affordance() {
        for id in [
            10194, 14002, 14003, 14004, 14005, 14006, 14007, 25103, 25104, 31955, 31956, 34300,
            34301, 35161, 35162, 42271, 42272, 48457,
        ] {
            assert_eq!(
                super::rotate_to(&format!("oteryn:item.tibia.i{id}")),
                None,
                "official appearance has no Rotate affordance: {id}"
            );
        }
        assert_eq!(super::rotate_to("oteryn:item.tibia.i9034"), Some(9039));
        assert_eq!(super::rotate_to("oteryn:item.tibia.i9039"), Some(9034));
        assert_eq!(super::rotate_to("oteryn:item.tibia.i9035"), Some(9040));
        assert_eq!(super::rotate_to("oteryn:item.tibia.i9040"), Some(9035));
    }
    #[test]
    fn srv7_decorations_retain_source_lamp_cycles_and_rotation() {
        assert_eq!(super::transform_to("oteryn:item.tibia.i2934"), Some(2935));
        assert_eq!(super::transform_to("oteryn:item.tibia.i2935"), Some(2934));
        assert_eq!(super::rotate_to("oteryn:item.tibia.i6355"), Some(6357));
        assert_eq!(super::rotate_to("oteryn:item.tibia.i2360"), Some(2361));
        assert_eq!(super::transform_to("oteryn:item.tibia.i6109"), None);
        assert_eq!(super::transform_to("oteryn:item.tibia.i23434"), Some(23436));
        assert_eq!(super::rotate_to("oteryn:item.tibia.i23434"), Some(23435));
        assert_eq!(super::transform_to("oteryn:item.tibia.i2487"), None);
        assert_eq!(super::transform_to("oteryn:item.tibia.i1628"), None);
    }
}

#[cfg(test)]
mod portable_census_tests {
    #[test]
    fn srv7_all_eighty_portable_source_uses_reach_channel_action_admission() {
        for id in [
            2911, 2912, 2914, 2915, 2917, 2918, 2920, 2921, 2922, 2923, 2924, 2925, 2977, 2978,
            3046, 3047, 5812, 5813, 6488, 6489, 9063, 9064, 9065, 9066, 20280, 20281, 22153, 22154,
            28915, 28917, 30362, 30363, 30364, 31674, 32760, 32761, 33331, 33333, 34264, 34266,
            34268, 34269, 34326, 35909, 36019, 36478, 36480, 36978, 36979, 36996, 36998, 37052,
            37053, 37054, 37056, 37061, 37063, 37111, 37113, 37543, 37544, 37545, 37580, 37581,
            37582, 37743, 37745, 37749, 37750, 38677, 38704, 39697, 39698, 39699, 39700, 39701,
            39702, 39757, 39759, 39761,
        ] {
            assert!(
                crate::novelty_items::is_ground_action(&format!("oteryn:item.tibia.i{id}")),
                "portable USE source{id} must also reach actual Ground action admission"
            );
            assert!(
                crate::novelty_items::is_novelty(&format!("oteryn:item.tibia.i{id}")),
                "pinned portable/no-House-guard USE source {id} must reach ordinary Channel action admission"
            );
        }
    }
    #[test]
    fn srv7_all_fifteen_portable_instruments_reach_channel_cue_admission() {
        for id in [
            2948, 2949, 2950, 2951, 2952, 2953, 2954, 2955, 2956, 2957, 2958, 2965, 2966, 3219,
            12602,
        ] {
            assert!(super::side_rule(&format!("oteryn:item.tibia.i{id}")).is_some());
            assert!(
                crate::novelty_items::is_ground_action(&format!("oteryn:item.tibia.i{id}")),
                "portable instrument{id} must also reach Ground cue admission"
            );
            assert!(
                crate::novelty_items::is_novelty(&format!("oteryn:item.tibia.i{id}")),
                "pinned portable instrument {id} must reach ordinary Channel cue action admission"
            );
        }
    }
}

#[cfg(test)]
mod golden_display_source_tests {
    #[test]
    fn srv7_golden_display_use_retains_all_sixteen_source_orientation_cycles() {
        // Crystal201189 golden_outfit_display.lua1–18. These are USE quality
        // changes; ordinary Rotate remains a separate orientation action.
        for cycle in [
            [31509, 31500, 31505, 31513],
            [31510, 31502, 31506, 31514],
            [31511, 31503, 31507, 31515],
            [31512, 31504, 31508, 31516],
        ] {
            for index in 0..cycle.len() {
                let source = format!("oteryn:item.tibia.i{}", cycle[index]);
                assert_eq!(
                    super::transform_to(&source),
                    Some(cycle[(index + 1) % cycle.len()]),
                    "source USE cycle for {source}"
                );
                assert!(
                    super::requires_house_preflight(&source),
                    "the durable full-Golden-Outfit gate must precede {source}"
                );
            }
        }
    }
}
