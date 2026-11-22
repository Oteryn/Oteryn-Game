use super::TestResult;
use super::fixture::{debug, id};

#[test]
fn afflicted_source_creatures_admit_all_quest_drops_in_native_corpses() -> TestResult {
    let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, _) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
    let native = controller
        .active()
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native gameplay")?;
    for (creature, corpse, items) in [
        (
            "feverish_citizen",
            18114,
            &[
                (12551, 16_600),
                (12552, 17_200),
                (12553, 22_900),
                (12554, 20_060),
                (12555, 20_600),
                (12556, 22_900),
            ][..],
        ),
        ("swamp_troll", 6018, &[(12517, 21_600)][..]),
    ] {
        let key = format!("oteryn:creature.{creature}");
        let row = native.reward_table().row(&key).map_err(debug)?;
        let corpse = format!("oteryn:item.tibia.i{corpse}");
        assert_eq!(
            row.corpse_item
                .as_ref()
                .map(|item| item.production_key.as_str()),
            Some(corpse.as_str()),
        );
        for (id, probability) in items {
            let reference = native
                .reward_table()
                .admitted_source_item(*id)
                .ok_or("quest loot item must admit corpse custody")?;
            assert!(
                row.loot_table
                    .entries
                    .iter()
                    .any(|entry| entry.item == *reference
                        && entry.min_count == 1
                        && entry.max_count == 1
                        && entry.probability_ppm == Some(*probability))
            );
        }
    }
    Ok(())
}

#[test]
fn afflicted_collects_source_cloth_and_pouch_from_native_corpses() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::qengine_rewards_fixtures::scenario(super::afflicted_loot_fixtures::HUNT)
}
