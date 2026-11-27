use oteryn_gameplay_content::content::{
    creature_reward::{CreatureRewardTable, LootTablesSection},
    native_gameplay::CreatureProfilesDocument,
};
use oteryn_simulation_determinism::ExactI64;

#[test]
fn poison_spider_reward_admits_exact_corpse_and_xp() -> Result<(), Box<dyn std::error::Error>> {
    let creatures: CreatureProfilesDocument = serde_json::from_slice(include_bytes!(
        "../../../../content/creatures/definitions/spell-native-profiles.json"
    ))?;
    let loot = LootTablesSection::decode(
        include_bytes!("../../../../tools/content-schema/native-gameplay/loot-tables.json"),
        &creatures,
    )?;
    let rewards = CreatureRewardTable::from_pin(&creatures, Some(&loot));
    let row = rewards
        .row("oteryn:creature.poison_spider")
        .map_err(|reason| format!("Poison Spider reward refused: {reason:?}"))?;
    let corpse = row
        .corpse_item
        .as_ref()
        .ok_or("missing Poison Spider corpse")?;
    assert_eq!(corpse.family, "Item");
    assert_eq!(corpse.production_key, "oteryn:item.tibia.i5974");
    assert_eq!(corpse.revision_ref, "definition-r1");
    assert_eq!(row.xp_amount, ExactI64::new(22));
    assert!(!row.loot_table.entries.is_empty());
    Ok(())
}
