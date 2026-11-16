#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! Corpse wave S1 (`tools/content-schema/native-gameplay/creature_corpse_items.py`): every
//! selected spawned Newhaven creature settles its kill on the production content: its corpse
//! Item and loot table bind, and the row grants exactly the profile's experience.

use crate::content::creature_reward::{CreatureRewardTable, LootTablesSection};
use crate::content::native_gameplay::CreatureProfilesDocument;
use crate::combat::LootDefinitionRef;
use serde_json::Value;
use std::collections::HashMap;

const S1: [&str; 11] = [
    "dark_carnisylvan",
    "ghost",
    "hulking_carnisylvan",
    "hunter",
    "muglex_clan_assassin",
    "muglex_clan_footman",
    "mummy",
    "poisonous_carnisylvan",
    "smuggler",
    "stone_golem",
    "wild_warrior",
];

fn read(path: &str) -> Vec<u8> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::read(root.join(path)).unwrap()
}

fn reference(value: &Value) -> LootDefinitionRef {
    LootDefinitionRef::new(
        value["family"].as_str().unwrap(),
        value["key"].as_str().unwrap(),
        value["revision"].as_str().unwrap(),
    )
}

#[test]
fn every_wave_s1_creature_settles_with_its_profile_experience_and_bound_loot() {
    let profiles = read("content/creatures/definitions/spell-native-profiles.json");
    let loot = read("tools/content-schema/native-gameplay/loot-tables.json");
    let creatures: CreatureProfilesDocument = serde_json::from_slice(&profiles).unwrap();
    let section = LootTablesSection::decode(&loot, &creatures).expect("production loot section");
    let table = CreatureRewardTable::build(&creatures, &section, &HashMap::new());

    let profiles: Value = serde_json::from_slice(&profiles).unwrap();
    let loot: Value = serde_json::from_slice(&loot).unwrap();
    for name in S1 {
        let key = format!("oteryn:creature.{name}");
        let profile = profiles["records"]
            .as_array()
            .unwrap()
            .iter()
            .map(|record| &record["profile"])
            .find(|profile| profile["target"]["key"] == key.as_str())
            .unwrap_or_else(|| panic!("{key} has a production profile"));
        let data = &profile["data"]["profile"];
        let experience = data["experience"].as_i64().unwrap();
        assert!(experience > 0, "{key} grants experience");
        let binding = &loot["creature_loot"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["creature"]["key"] == key.as_str())
            .unwrap_or_else(|| panic!("{key} has a loot binding"))["loot"];
        let entries = loot["tables"]
            .as_array()
            .unwrap()
            .iter()
            .find(|table| table["identity"] == *binding)
            .unwrap_or_else(|| panic!("{key} loot table exists"))["entries"]
            .as_array()
            .unwrap()
            .len();
        assert!(entries > 0, "{key} has loot");

        let row = table
            .row(&key)
            .unwrap_or_else(|reason| panic!("{key} settles: {reason:?}"));
        assert_eq!(row.xp_amount.get(), experience, "{key} experience");
        assert_eq!(
            row.corpse_item,
            reference(&data["details"]["corpse_item"]),
            "{key} corpse"
        );
        assert_eq!(row.loot_table_ref, reference(binding), "{key} loot table");
        assert_eq!(row.loot_table.entries.len(), entries, "{key} loot entries");
    }
}
