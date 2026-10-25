use super::*;

/// The item profiles before any mechanic added its own; all of them are unchanged.
const ORIGINAL_PROFILES: usize = 432;

/// The item profiles each mechanic adds, one row per mechanic in name order, so a new mechanic
/// adds its own row instead of rewriting a shared total.
const PROFILE_ADDITIONS: &[(&str, usize)] = &[
    // 22 cooking profiles from Canary items.xml/appearances.dat at
    // 99902524e052f37574194466c2949c576e4ab269 (content/craft/runtime-item-profiles.json):
    // millstones 1943..=1946; ovens 2535/2537/2539/2541; reed 30975/31686;
    // trays 3464/8198; flour/dough/wheat 3603..=3605; cake 6276/6277;
    // chocolate 8018/8019; garlic 8194/8196/8199. All have speed_bonus=0.
    ("CRAFT-1", 22),
    // ENERGY-RING-1 admits the inactive 3051 and active 3088 through the timed pipeline.
    ("ENERGY-RING-1", 2),
    // 4 timed-item profiles.
    ("ITEM-TIMED-1", 4),
    // 23 training/exercise weapon profiles (tools/content-migration/loop_training_profiles.py):
    // 28552..=28557, 35279..=35290, 44065..=44067, 50294/50295. All have speed_bonus=0.
    ("LOOP-TRAIN-1", 23),
    // The pick 3456, shiny stone 10310 and small diamond 3028.
    ("MINING-1", 3),
    // 19 read-only readable profiles (tools/content-migration/readable_item_profiles.py):
    // 2012..=2017, 2021..=2024, 2597, 2598, 2602, 2605, 6121, 6122, 10322, 12782, 12783.
    // None of them is equipment.
    ("READ-WRITE-1", 19),
];

#[test]
fn equipment_effects_owner_pinned_production_speed_matches_all_raw_attributes() -> TestResult {
    let input = input()?;
    let profiles: ItemProfilesDocument =
        serde_json::from_slice(&input.item_profiles.as_ref().ok_or("profiles")?.bytes)?;
    assert_eq!(
        profiles.records.len(),
        ORIGINAL_PROFILES + PROFILE_ADDITIONS.iter().map(|(_, n)| n).sum::<usize>()
    );
    let content = native(&input)?;
    let mut unknown_speed = std::collections::BTreeSet::new();
    for record in profiles.records {
        let abilities = item_ability_profile(&record.semantics)?;
        let sum = abilities.abilities.iter().try_fold(0_i32, |sum, ability| {
            sum.checked_add(match ability {
                ItemAbility::Speed(value) => *value,
                _ => 0,
            })
        });
        if record.attributes.speed_bonus.is_some() {
            assert_eq!(
                sum, record.attributes.speed_bonus,
                "{}",
                record.production_definition.production_key
            );
        } else {
            assert!(!matches!(
                record.semantics.equipment,
                ReferenceItemField::Known(_)
            ));
            unknown_speed.insert(record.production_definition.production_key.clone());
        }
        let equipment = snapshot(&content, &record.production_definition.production_key, 10);
        assert_eq!(speed_term(&equipment, &content), Some(0));
    }
    assert_eq!(
        unknown_speed,
        [
            "oteryn:item.tibia.i266".into(),
            "oteryn:item.tibia.i285".into(),
            "oteryn:item.tibia.i3028".into(),
            "oteryn:item.tibia.i3456".into(),
            "oteryn:item.tibia.i10310".into(),
        ]
        .into_iter()
        // READ-WRITE-1: the 19 readable profiles carry no speed attribute and are not equipment.
        .chain(
            [
                2012, 2013, 2014, 2015, 2016, 2017, 2021, 2022, 2023, 2024, 2597, 2598, 2602, 2605,
                6121, 6122, 10322, 12782, 12783,
            ]
            .map(|id| format!("oteryn:item.tibia.i{id}")),
        )
        .collect()
    );
    Ok(())
}

#[test]
fn equipment_effects_owner_potions_in_ammo_and_hands_preserve_contributing_speed() -> TestResult {
    let boosted = fixture_input(
        serde_json::json!([
            {"kind":"SPEED","parameter":{"state":"KNOWN","value":{"kind":"SIGNED_POINTS","value":30}},"evaluation_phase":{"state":"UNKNOWN"},"priority":{"state":"UNKNOWN"},"target_domain":{"state":"UNKNOWN"}}
        ]),
        Some(30),
    )?;
    for (input, expected_speed) in [(input()?, 0), (boosted, 30)] {
        let content = native(&input)?;
        for potion in ["oteryn:item.tibia.i266", "oteryn:item.tibia.i285"] {
            for slot in [10, 5, 6] {
                let mut equipment = snapshot(&content, "oteryn:item.tibia.i2854", 9);
                let mut carried = snapshot(&content, potion, slot);
                carried.items[0].item_instance_id = [3; 16];
                equipment.items.extend(carried.items);
                assert_eq!(
                    speed_term(&equipment, &content),
                    Some(expected_speed),
                    "{potion} in slot {slot} beside speed {expected_speed}"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn equipment_effects_owner_rejects_mismatch_unknown_policy_and_invalid_slots() -> TestResult {
    let content = native(&input()?)?;
    let mut equipment = snapshot(&content, "oteryn:item.tibia.i2854", 9);
    assert!(equipment_effects(&equipment, &content).is_some());
    equipment.content_digest = [0; 32];
    assert!(equipment_effects(&equipment, &content).is_none());
    equipment.content_digest = content.source_digest();
    equipment.items[0].slot = 11;
    assert!(equipment_effects(&equipment, &content).is_none());
    equipment.items[0].slot = 9;
    equipment.items[0].definition.revision_ref = "unknown".into();
    assert!(speed_term(&equipment, &content).is_none());
    equipment.items.clear();
    assert_eq!(speed_term(&equipment, &content), Some(0));
    Ok(())
}

#[test]
fn equipment_effects_owner_uses_primary_slot_and_extra_never_suppresses() -> TestResult {
    let input = fixture_input(
        serde_json::json!([
            {"kind":"SPEED","parameter":{"state":"KNOWN","value":{"kind":"SIGNED_POINTS","value":30}},"evaluation_phase":{"state":"UNKNOWN"},"priority":{"state":"UNKNOWN"},"target_domain":{"state":"UNKNOWN"}},
            {"kind":"SUPPRESS_DROWN","parameter":{"state":"KNOWN","value":{"kind":"BOOLEAN","value":true}},"evaluation_phase":{"state":"UNKNOWN"},"priority":{"state":"UNKNOWN"},"target_domain":{"state":"UNKNOWN"}}
        ]),
        Some(30),
    )?;
    let content = native(&input)?;
    let mut equipment = snapshot(&content, "oteryn:item.tibia.i2854", 9);
    assert_eq!(speed_term(&equipment, &content), Some(30));
    assert_eq!(
        equipment_effects(&equipment, &content)
            .ok_or("effects")?
            .suppressions,
        [AbilitySuppression::Drown].into_iter().collect()
    );
    for slot in [1, 2, 3, 4, 5, 6, 7, 8, 10] {
        equipment.items[0].slot = slot;
        let effects = equipment_effects(&equipment, &content).ok_or("effects")?;
        assert_eq!(effects.speed, 0);
        assert!(effects.suppressions.is_empty());
    }
    Ok(())
}

#[test]
fn equipment_effects_owner_preserves_unknown_speed_and_rejects_ability_errors() -> TestResult {
    let content = native(&fixture_input(serde_json::json!([]), None)?)?;
    let equipment = snapshot(&content, "oteryn:item.tibia.i2854", 9);
    assert!(speed_term(&equipment, &content).is_none());
    let content = native(&fixture_input(
        serde_json::json!([
            {"kind":"SPEED","parameter":{"state":"UNKNOWN"},"evaluation_phase":{"state":"UNKNOWN"},"priority":{"state":"UNKNOWN"},"target_domain":{"state":"UNKNOWN"}}
        ]),
        Some(0),
    )?)?;
    let equipment = snapshot(&content, "oteryn:item.tibia.i2854", 9);
    assert!(equipment_effects(&equipment, &content).is_none());
    Ok(())
}
