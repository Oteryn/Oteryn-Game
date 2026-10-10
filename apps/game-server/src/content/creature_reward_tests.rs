#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::collections::HashMap;

use oteryn_simulation_determinism::ExactI64;
use serde_json::{Value, json};

use super::{
    CreatureRewardTable, LootTablesSection, NoSettlementReason, ProjectV2AuthoringProfileData,
};
use crate::combat::{LootDefinitionRef, LootSelectionAlgorithm};
use crate::content::native_gameplay::CreatureProfilesDocument;
use crate::domain::bestiary::BestiaryRace;

const RAT: &str = "canary:creature/rat";
const FAMILIAR: &str = "canary:creature/knight_familiar";
const REVISION: &str = "canary-47dfd51f";
const CORPSE: &str = "canary:item/5964";
const GOLD: &str = "oteryn:item/i3031";
const TABLE: &str = "oteryn:loot/rat";

fn creatures() -> CreatureProfilesDocument {
    serde_json::from_slice(include_bytes!(
        "../../../../tools/content-schema/native-gameplay/creature_profiles.json"
    ))
    .expect("sample creature profiles")
}

fn reference(family: &str, key: &str) -> Value {
    json!({"family": family, "key": key, "revision": REVISION})
}

fn section() -> Value {
    json!({
        "schema": "OTERYN_NATIVE_LOOT_TABLES/v1",
        "creature_loot": [
            {"creature": reference("Creature", RAT), "loot": reference("Loot", TABLE)},
            {"creature": reference("Creature", FAMILIAR), "loot": null},
        ],
        "tables": [{
            "identity": reference("Loot", TABLE),
            "algorithm": "IndependentBernoulliPpm",
            "entries": [{"item": reference("Item", GOLD), "min_count": 1, "max_count": 3,
                         "probability_ppm": 500_000}],
        }],
        "items": [
            {"item": reference("Item", CORPSE), "materializable": true, "container_capacity": 16},
            {"item": reference("Item", GOLD), "materializable": true, "container_capacity": null},
        ],
    })
}

fn decode(value: &Value) -> Result<LootTablesSection, crate::content::ContentError> {
    LootTablesSection::decode(&serde_json::to_vec(value).unwrap(), &creatures())
}

fn build(value: &Value) -> CreatureRewardTable {
    build_with(&creatures(), value)
}

fn build_with(creatures: &CreatureProfilesDocument, value: &Value) -> CreatureRewardTable {
    let section: LootTablesSection = serde_json::from_value(value.clone()).unwrap();
    CreatureRewardTable::build(creatures, &section, &HashMap::new())
}

fn reason(table: &CreatureRewardTable, creature: &str) -> NoSettlementReason {
    table.row(creature).expect_err("refused row")
}

fn set_experience(creatures: &mut CreatureProfilesDocument, experience: Option<u64>) {
    let ProjectV2AuthoringProfileData::Creature(profile) = &mut creatures.records[0].profile.data
    else {
        panic!("rat is a Creature profile");
    };
    profile.experience = experience;
}

#[test]
fn builds_the_bound_row() {
    let section = decode(&section()).expect("valid section");
    let rat = CreatureRewardTable::build(&creatures(), &section, &HashMap::new())
        .row(RAT)
        .expect("rat row");
    assert_eq!(rat.xp_amount, ExactI64::new(5));
    assert_eq!(
        rat.corpse_item,
        LootDefinitionRef::new("Item", CORPSE, REVISION)
    );
    assert_eq!(
        rat.loot_table_ref,
        LootDefinitionRef::new("Loot", TABLE, REVISION)
    );
    assert_eq!(
        rat.loot_table.algorithm,
        LootSelectionAlgorithm::IndependentBernoulliPpm
    );
    assert_eq!(rat.loot_table.entries.len(), 1);
    assert_eq!(
        rat.loot_table.entries[0].item,
        LootDefinitionRef::new("Item", GOLD, REVISION)
    );
    assert_eq!(rat.loot_table.entries[0].probability_ppm, Some(500_000));
    assert_eq!(rat.race, None);
}

#[test]
fn null_loot_mints_the_corpse_with_an_empty_table() {
    let mut value = section();
    value["creature_loot"][0]["loot"] = Value::Null;
    value["tables"] = json!([]);
    let section = decode(&value).expect("null binding decodes");
    let rat = CreatureRewardTable::build(&creatures(), &section, &HashMap::new())
        .row(RAT)
        .expect("rat row");
    assert!(rat.loot_table.entries.is_empty());
    assert_eq!(rat.loot_table_ref, rat.corpse_item);
}

#[test]
fn bound_table_absent_is_loot_table_missing() {
    let mut value = section();
    value["tables"] = json!([]);
    decode(&value).expect("a missing table is a row refusal, not a pin refusal");
    assert_eq!(
        reason(&build(&value), RAT),
        NoSettlementReason::LootTableMissing
    );
}

#[test]
fn unbound_creature_is_no_loot_binding() {
    let mut value = section();
    value["creature_loot"].as_array_mut().unwrap().remove(0);
    let table = build(&value);
    assert_eq!(reason(&table, RAT), NoSettlementReason::NoLootBinding);
    assert_eq!(
        reason(&CreatureRewardTable::from_pin(&creatures(), None), RAT),
        NoSettlementReason::NoLootBinding
    );
}

#[test]
fn profile_without_corpse_is_corpse_item_missing() {
    assert_eq!(
        reason(&build(&section()), FAMILIAR),
        NoSettlementReason::CorpseItemMissing
    );
}

#[test]
fn corpse_must_be_an_admitted_materializable_bounded_container() {
    for facts in [
        json!({"materializable": false, "container_capacity": 16}),
        json!({"materializable": true, "container_capacity": null}),
        json!({"materializable": true, "container_capacity": 0}),
        json!({"materializable": true, "container_capacity": 17}),
    ] {
        let mut value = section();
        value["items"][0]["materializable"] = facts["materializable"].clone();
        value["items"][0]["container_capacity"] = facts["container_capacity"].clone();
        assert_eq!(
            reason(&build(&value), RAT),
            NoSettlementReason::CorpseItemInadmissible,
            "{facts}"
        );
    }
    let mut value = section();
    value["items"].as_array_mut().unwrap().remove(0);
    assert_eq!(
        reason(&build(&value), RAT),
        NoSettlementReason::CorpseItemInadmissible
    );
    let mut value = section();
    value["items"][0]["container_capacity"] = json!(1);
    build(&value).row(RAT).expect("capacity 1 is admitted");
}

#[test]
fn entry_item_must_be_admitted_and_materializable() {
    let mut value = section();
    value["items"][1]["materializable"] = json!(false);
    assert_eq!(
        reason(&build(&value), RAT),
        NoSettlementReason::LootItemInadmissible
    );
    let mut value = section();
    value["items"].as_array_mut().unwrap().remove(1);
    assert_eq!(
        reason(&build(&value), RAT),
        NoSettlementReason::LootItemInadmissible
    );
}

#[test]
fn experience_bounds() {
    let mut creatures = creatures();
    set_experience(&mut creatures, Some(u64::MAX));
    assert_eq!(
        reason(&build_with(&creatures, &section()), RAT),
        NoSettlementReason::XpOutOfRange
    );
    set_experience(&mut creatures, Some(i64::MAX as u64));
    let row = build_with(&creatures, &section()).row(RAT).unwrap();
    assert_eq!(row.xp_amount, ExactI64::new(i64::MAX));
    for experience in [None, Some(0)] {
        set_experience(&mut creatures, experience);
        let row = build_with(&creatures, &section()).row(RAT).unwrap();
        assert_eq!(row.xp_amount, ExactI64::new(0));
    }
}

#[test]
fn race_requires_the_exact_definition_revision() {
    let section: LootTablesSection = serde_json::from_value(section()).unwrap();
    // The sample keys carry `/`, which a Bestiary race key refuses; the lookup is by
    // creature key, so a valid race key stands in.
    let race = BestiaryRace::new("canary:creature.rat", REVISION, vec![1, 2]).unwrap();
    let races = HashMap::from([(RAT.to_owned(), race.clone())]);
    let row = CreatureRewardTable::build(&creatures(), &section, &races)
        .row(RAT)
        .unwrap();
    assert_eq!(row.race, Some(race));
    let other = BestiaryRace::new("canary:creature.rat", "canary-other", vec![1]).unwrap();
    let races = HashMap::from([(RAT.to_owned(), other)]);
    let row = CreatureRewardTable::build(&creatures(), &section, &races)
        .row(RAT)
        .unwrap();
    assert_eq!(row.race, None);
}

#[test]
fn decode_refuses_malformed_sections() {
    let mut cases: Vec<(&str, Value)> = Vec::new();
    let mut value = section();
    value["creature_loot"].as_array_mut().unwrap().remove(1);
    cases.push(("pinned creature without binding", value));
    let mut value = section();
    value["creature_loot"]
        .as_array_mut()
        .unwrap()
        .push(json!({"creature": reference("Creature", "canary:creature/dragon"), "loot": null}));
    cases.push(("binding for an unpinned creature", value));
    let mut value = section();
    value["creature_loot"][1]["creature"]["revision"] = json!("canary-other");
    cases.push(("binding for another revision", value));
    let mut value = section();
    let row = value["creature_loot"][1].clone();
    value["creature_loot"].as_array_mut().unwrap().push(row);
    cases.push(("duplicate binding", value));
    let mut value = section();
    value["creature_loot"][0]["loot"] = Value::Null;
    cases.push(("unreferenced table", value));
    let mut value = section();
    let table = value["tables"][0].clone();
    value["tables"].as_array_mut().unwrap().push(table);
    cases.push(("duplicate table", value));
    let mut value = section();
    value["tables"][0]["algorithm"] = json!("Unknown");
    cases.push(("unknown algorithm", value));
    let mut value = section();
    let item = value["items"][1].clone();
    value["items"].as_array_mut().unwrap().push(item);
    cases.push(("duplicate Item facts", value));
    let mut value = section();
    value["schema"] = json!("OTERYN_NATIVE_LOOT_TABLES/v2");
    cases.push(("schema", value));
    let mut value = section();
    value["extra"] = json!(true);
    cases.push(("unknown field", value));
    let mut value = section();
    value["creature_loot"][0]["loot"]["family"] = json!("Item");
    cases.push(("binding family", value));
    for (case, value) in cases {
        assert!(decode(&value).is_err(), "{case}");
    }
}

#[test]
fn reasons_have_stable_names() {
    for (reason, name) in [
        (NoSettlementReason::NoLootBinding, "no_loot_binding"),
        (NoSettlementReason::LootTableMissing, "loot_table_missing"),
        (NoSettlementReason::CorpseItemMissing, "corpse_item_missing"),
        (
            NoSettlementReason::CorpseItemInadmissible,
            "corpse_item_inadmissible",
        ),
        (
            NoSettlementReason::LootItemInadmissible,
            "loot_item_inadmissible",
        ),
        (NoSettlementReason::XpOutOfRange, "xp_out_of_range"),
    ] {
        assert_eq!(reason.as_str(), name);
    }
}

/// CP D929: the production pin's rat row refuses while its table names cheese
/// `i3607`, which is not materializable (OTV2-20261007-d3-8-cheese).
#[test]
fn the_production_rat_row_refuses_its_inadmissible_cheese() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let creatures: CreatureProfilesDocument = serde_json::from_slice(
        &std::fs::read(root.join("content/creatures/definitions/spell-native-profiles.json"))
            .unwrap(),
    )
    .expect("production creature profiles");
    let bytes =
        std::fs::read(root.join("tools/content-schema/native-gameplay/loot-tables.json")).unwrap();
    let section = LootTablesSection::decode(&bytes, &creatures).expect("production loot section");
    let table = CreatureRewardTable::build(&creatures, &section, &HashMap::new());
    assert_eq!(
        reason(&table, "oteryn:creature.rat"),
        NoSettlementReason::LootItemInadmissible
    );
}
