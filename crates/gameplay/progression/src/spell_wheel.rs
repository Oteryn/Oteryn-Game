//! Pure Wheel stage projections for spell access and magnitude observations.
use crate::wheel::{WheelAllocation, WheelRuleset, WheelVocationRules};
use std::collections::BTreeMap;

/// Derive the closed spell and perk aliases from a current eligible allocation.
pub fn project_wheel_stages(
    ruleset: &WheelRuleset,
    allocation: &WheelAllocation,
    vocation: &str,
    level: u32,
) -> Option<BTreeMap<String, u8>> {
    let promoted = matches!(
        vocation,
        "elder_druid" | "elite_knight" | "exalted_monk" | "royal_paladin" | "master_sorcerer"
    );
    let rules = crate::wheel::eligibility(ruleset, vocation, level, promoted).ok()?;
    let stages = crate::wheel::WheelStages::derive(ruleset, allocation, vocation, level, promoted);
    let mut value: BTreeMap<_, _> = rules
        .revelations
        .iter()
        .map(|key| (key.clone(), stages.revelation_stage(key)))
        .collect();
    for key in rules.augment_targets.keys() {
        value.insert(key.clone(), stages.augment_stage(key));
    }
    for target in rules.augment_targets.values().flatten() {
        value.insert(target.clone(), stages.spell_augment_stage(target));
    }
    // Closed aliases from the existing Wheel spell ABI and admitted native profile labels.
    // No case folding or inferred key authorizes an arbitrary perk.
    for (label, key) in [
        ("Executioner's Throw", "executioner_s_throw"),
        ("Combat Mastery", "combat_mastery"),
        ("Divine Grenade", "divine_grenade"),
        ("Divine Empowerment", "divine_empowerment"),
        ("Blessing of the Grove", "blessing_of_the_grove"),
        ("Twin Burst", "twin_bursts"),
        ("Beam Mastery", "beam_mastery"),
        ("Lord of Destruction", "lord_of_destruction"),
        ("Spiritual Outburst", "spiritual_outburst"),
        ("Ascetic", "ascetic"),
        ("Avatar of Steel", "avatar_of_steel"),
        ("Avatar of Light", "avatar_of_light"),
        ("Avatar of Nature", "avatar_of_nature"),
        ("Avatar of Storm", "avatar_of_storm"),
        ("Avatar of Balance", "avatar_of_balance"),
        ("avatar of steel", "avatar_of_steel"),
        ("avatar of light", "avatar_of_light"),
        ("avatar of nature", "avatar_of_nature"),
        ("avatar of storm", "avatar_of_storm"),
        ("avatar of balance", "avatar_of_balance"),
        ("energy wave", "Energy Wave"),
        ("front sweep", "Front Sweep"),
        ("mass healing", "Mass Healing"),
        ("strong ice wave", "Strong Ice Wave"),
    ] {
        if let Some(stage) = value.get(key).copied() {
            value.insert(label.to_owned(), stage);
        }
    }
    Some(value)
}

/// Map the four revelation keys to the source profile's domain colours.
pub fn revelation_colours(
    rules: &WheelVocationRules,
    stages: &BTreeMap<String, u8>,
) -> Option<BTreeMap<String, u8>> {
    let mut colours = BTreeMap::new();
    for (colour, key) in ["green", "red", "purple", "blue"]
        .iter()
        .zip(&rules.revelations)
    {
        let stage = stages.get(key).copied()?;
        colours.insert((*colour).to_owned(), stage);
    }
    Some(colours)
}
