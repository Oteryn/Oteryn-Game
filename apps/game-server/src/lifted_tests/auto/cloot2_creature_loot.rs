use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::content::creature_reward::{
    LootTablesSection, NoSettlementReason, SectionLootEntry, SectionRef,
};
use crate::content::native_gameplay::test_support::try_activate_input;
use crate::content::native_gameplay::{CreatureProfilesDocument, NativeGameplayInput};
use crate::foundation::WorldId;

const CREATURES: &str =
    include_str!("../../../../../tools/content-schema/native-gameplay/cloot2-creatures.txt");
const ITEMS: &str =
    include_str!("../../../../../tools/content-schema/native-gameplay/cloot2-item-ids.json");

fn check_entries(
    creature: &str,
    entries: &[SectionLootEntry],
    admitted: &HashSet<&SectionRef>,
    unavailable: &mut Vec<String>,
) {
    for entry in entries {
        if !admitted.contains(&entry.item) {
            unavailable.push(format!("{creature}/{}", entry.item.key));
        }
        check_entries(creature, &entry.children, admitted, unavailable);
    }
}

#[test]
fn middle_creature_loot_slice_loads_all_yields()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(check_middle_creature_loot)?
        .join()
        .map_err(|_| "creature loot loader thread panicked")?
}

fn check_middle_creature_loot() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let profiles: CreatureProfilesDocument = serde_json::from_slice(&std::fs::read(
        root.join("content/creatures/definitions/spell-native-profiles.json"),
    )?)?;
    let section = LootTablesSection::decode(
        &std::fs::read(root.join("tools/content-schema/native-gameplay/loot-tables.json"))?,
        &profiles,
    )?;
    let admitted: HashSet<_> = section
        .items
        .iter()
        .filter(|fact| fact.materializable)
        .map(|fact| &fact.item)
        .collect();
    let bindings: HashMap<_, _> = section
        .creature_loot
        .iter()
        .map(|row| (row.creature.key.as_str(), row.loot.as_ref()))
        .collect();
    let tables: HashMap<_, _> = section
        .tables
        .iter()
        .map(|row| (&row.identity, &row.entries))
        .collect();
    let creatures: Vec<_> = CREATURES.lines().collect();
    assert_eq!(creatures.len(), 323);
    assert!(creatures.windows(2).all(|pair| pair[0] < pair[1]));

    let mut unavailable = Vec::new();
    for &creature in &creatures {
        let table = bindings[creature].ok_or_else(|| format!("{creature} has no loot binding"))?;
        check_entries(creature, tables[table], &admitted, &mut unavailable);
    }
    assert!(
        unavailable.is_empty(),
        "{} unavailable yields, beginning with {:?}",
        unavailable.len(),
        &unavailable[..unavailable.len().min(12)]
    );

    let world = WorldId::decode(&[
        1, 147, 79, 16, 124, 2, 112, 1, 128, 91, 59, 17, 34, 51, 68, 1,
    ])?;
    let input = NativeGameplayInput::from_manifest(&root.join("content/spells.manifest.json"))?;
    let (controller, _) = try_activate_input(world, &input)?;
    let state = controller
        .active()
        .and_then(|generation| generation.native_gameplay())
        .ok_or("activated native gameplay")?;
    let rewards = state.reward_table();
    for item in serde_json::from_str::<Vec<u32>>(ITEMS)? {
        let expected = format!("oteryn:item.tibia.i{item}");
        let definition = rewards
            .admitted_source_item(item)
            .ok_or(expected.as_str())?;
        assert_eq!(definition.production_key, expected);
        let policy = state
            .item_policy(&expected, &definition.revision_ref)
            .ok_or(expected.as_str())?;
        assert!(policy.record().admission.materializable, "{expected}");
    }

    let replacements = [
        ("muglex_clan_chief", 266, 547_200, 1, 2),
        ("muglex_clan_chief", 268, 188_700, 1, 3),
        ("megasylvan_yselda", 49271, 56_604, 2, 18),
    ];
    for (creature, item, probability, min, max) in replacements {
        let key = format!("oteryn:creature.{creature}");
        let table = bindings[key.as_str()].ok_or("missing corrected loot binding")?;
        let expected = format!("oteryn:item.tibia.i{item}");
        let entry = tables[table]
            .iter()
            .find(|entry| entry.item.key == expected)
            .ok_or(expected)?;
        assert_eq!(
            (entry.probability_ppm, entry.min_count, entry.max_count),
            (Some(probability), min, max)
        );
    }

    for creature in creatures {
        match rewards.row(creature) {
            Ok(row) => {
                let table = bindings[creature].ok_or("missing loot binding")?;
                assert_eq!(
                    row.loot_table.entries.len(),
                    tables[table].len(),
                    "{creature}"
                );
            }
            Err(
                NoSettlementReason::CorpseItemMissing | NoSettlementReason::CorpseItemInadmissible,
            ) => {
                // Body admission is separate from loot admission.
            }
            Err(reason) => return Err(format!("{creature} reward refused: {reason:?}").into()),
        }
    }
    Ok(())
}
