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
    // PAR-I11 baking (content/craft/runtime-item-profiles.json): the cookie tray 8020 and the
    // cookie 3598, from Crystal ff7ede59 items.xml. Both have speed_bonus=0.
    ("CRAFT-2", 2),
    // ENERGY-RING-1 admits the inactive 3051 and active 3088 through the timed pipeline.
    ("ENERGY-RING-1", 2),
    // The sliver 37109 (tools/content-migration/forge_creature_profiles.py), speed_bonus=0.
    ("FORGECREATURE", 1),
    // The coin converters 23722/25719/28525/28526 (tools/content-migration/coin_converter_profiles.py).
    // All have speed_bonus=0.
    ("GOLDCONV", 4),
    // I05 admits the inactive and active dwarven-ring forms 3097/3099 (speed_bonus=0).
    ("I05", 2),
    // I05 skill rings: the inactive and active power 3050/3087, sword 3091/3094, axe 3092/3095
    // and club 3093/3096 forms (tools/content-migration/timed_item_profiles.py), speed_bonus=0.
    ("I05-SKILL-RINGS", 8),
    // I10a: the podiums 35973/35974, 38707 and 42367/42368
    // (tools/content-migration/podium_item_profiles.py). All have speed_bonus=0.
    ("I10A-PODIUM", 5),
    // I14: seven restore potions, two foods and the two larger flasks admitted from
    // digest-pinned consumption facts. They carry no equipment/speed qualification.
    ("I14", 11),
    // I14G: 3250, 7642, 23374, 23375 and 23535 are non-equipment consumables.
    // Crystal 201189f8 items.xml:8201,20873,46095,46100,46862 and Canary
    // 99902524 items.xml:8170,20782,45877,45882,46644 carry no speed attribute.
    ("I14G", 5),
    // 4 timed-item profiles.
    ("ITEM-TIMED-1", 4),
    // The wooden chairs 2358..=2361 (tools/content-migration/item_transform_profiles.py).
    // All have speed_bonus=0.
    ("ITEM-TRANSFORM-1", 4),
    // 23 training/exercise weapon profiles (tools/content-migration/loop_training_profiles.py):
    // 28552..=28557, 35279..=35290, 44065..=44067, 50294/50295. All have speed_bonus=0.
    ("LOOP-TRAIN-1", 23),
    // M06: the five door profiles 1628, 1646, 1647, 2967 and 2968
    // (tools/content-schema/native-gameplay/build_key_door_profiles.py). All have speed_bonus=0.
    ("M06-KEYS", 5),
    // M09: 28 gathering profiles from Crystal 201189f
    // (tools/content-schema/native-gameplay/build_gathering_item_profiles.py). All have speed_bonus=0.
    ("M09-GATHER", 28),
    // E09 mail (tools/content-migration/mail_item_profiles.py): the parcel 3503, stamped
    // parcel 3504 and stamped letter 3506 are new; the letter 3505 and label 3507 replace
    // their content-only profiles. None of them is equipment.
    ("MAIL-E09", 3),
    // The pick 3456, shiny stone 10310 and small diamond 3028.
    ("MINING-1", 3),
    // Monk shrine rewards 50267..=50269 and 50271..=50274 from
    // tools/content-migration/monk_reward_profiles.py. All have speed_bonus=0.
    ("MONKQUEST", 7),
    // The Gold Pouch 23721 and eight quivers (tools/content-migration/quick_loot_item_profiles.py).
    // All have speed_bonus=0.
    ("QUICKLOOT-1", 9),
    // Skull 3114 (tools/content-migration/quest_callback_profiles.py), from Canary
    // 47dfd51f items.xml and exact Crystal ff7ede59 bindings. speed_bonus=0.
    ("QUESTCB2", 1),
    // The quest-advancing reward chest items 6120 and 9019, carried from ready
    // reward_claims quest_transition placements (tools/content-migration/quest_callback_profiles.py).
    // Both have speed_bonus=0.
    ("QUESTCHEST-1", 2),
    // Item 7934 used by a world callback's item-on-target step
    // (tools/content-migration/quest_callback_profiles.py). speed_bonus=0.
    ("QUESTCB3", 1),
    // 19 read-only readable profiles (tools/content-migration/readable_item_profiles.py):
    // 2012..=2017, 2021..=2024, 2597, 2598, 2602, 2605, 6121, 6122, 10322, 12782, 12783.
    // None of them is equipment.
    ("READ-WRITE-1", 19),
    // 27 reward-bag profiles (tools/content-migration/reward_bag_profiles.py):
    // Bag You Covet 43895 and rewards 43864..=43887, 50146/50157 from Canary 47dfd51f
    // reward_bags.lua/items.xml/appearances.dat, with reward ids checked against Crystal ff7ede59.
    // 43884/43887 have speed_bonus=10; the other 25 profiles have speed_bonus=0.
    ("REWARD-BAG-1", 27),
    // 36 more reward-bag profiles once Bag You Desire and Primal Bag are playable (same
    // script and sources): 34082..=34099, 34109, 39147..=39149, 39151..=39154, 39177,
    // 39180, 39183, 39186, 39546, 50149, 50159, 50188, 50240, 50254.
    // 34097 has speed_bonus=15 and 34098 has speed_bonus=20; the other 34 have speed_bonus=0.
    ("REWARD-BAG-2", 36),
    // TIMEDITEMS2-1 admits the inactive and active time-ring forms.
    ("TIMEDITEMS2-1", 2),
    // W18: the 20 fluid containers 2873..=2877, 2879..=2885, 2893, 2901..=2903 and
    // 3477..=3480 (tools/content-migration/fluid_container_profiles.py). All have speed_bonus=0.
    ("W18-FLUID", 20),
    // M08/M09: the eleven qualified shovel, machete, rod/bait/water and target policies
    // in tool-item-profiles.json. The source importer records speed_bonus=0.
    ("TOOLS-PKG4", 11),
    // M08B: the pick and its stone, loose stone and loose ice piles 593/594, 606/615 and
    // 608/609 in tool-item-profiles.json (Canary 99902524). None of them is equipment.
    ("TOOLS-M08B", 6),
    // W19F: the 24 rust-removal and lottery-ticket profiles 3357..=3360, 3362, 3364,
    // 3370..=3372, 3377, 3381, 3382, 3557, 3558, 5957, 5958, 8063, 8894..=8899 and 9016
    // (tools/content-migration/rust_lottery_profiles.py). All have speed_bonus=0.
    ("W19F-RUST-LOTTERY", 24),
    // Decision 25 text profiles: 15 one-way sources, their fixed targets and
    // readable carriers. These add no equipment or speed qualification.
    ("WRITE-CONTENT-1", 92),
    // PKG1 Amber: the amber kusarigama 50239 (tools/content-migration/monk_reward_profiles.py,
    // Crystal ff7ede59 items.xml), a monk weapon with speed_bonus=0.
    ("PKG1-AMBER", 1),
    // GEMX I19 enchanting (tools/content-migration/enchanting_item_profiles.py): 108 shrine,
    // gem and enchantable/enchanted item profiles 146..=161, 660..=693, 761..=763, 774,
    // 779..=798, 801..=810, 3029, 3030, 3032, 3033, 3271, 3279, 3311, 3317, 3318, 3333,
    // 3342, 7380, 7383, 7384, 7389, 7392, 7402, 7406, 7415 and 8077..=8081.
    // 8080 (sparking rainbow shield) has speed_bonus=10; the other 107 have speed_bonus=0.
    ("GEMX-I19", 108),
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
            if matches!(
                record.production_definition.production_key.as_str(),
                "oteryn:item.tibia.i9215"
                    | "oteryn:item.tibia.i9216"
                    | "oteryn:item.tibia.i9217"
                    | "oteryn:item.tibia.i9221"
                    | "oteryn:item.tibia.i31268"
            ) {
                // WRITE-CONTENT-1 preserves these source equipment facts but
                // grants no mint/equip admission or speed qualification.
                assert!(matches!(
                    record.semantics.equipment,
                    ReferenceItemField::Known(_)
                ));
                assert!(!record.admission.materializable);
            } else {
                assert!(!matches!(
                    record.semantics.equipment,
                    ReferenceItemField::Known(_)
                ));
            }
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
        // I14 preserves unknown speed for the newly admitted non-equipment consumables.
        .chain(
            [236, 237, 239, 268, 283, 284, 3578, 3606, 7643, 7876, 23373]
                .map(|id| format!("oteryn:item.tibia.i{id}")),
        )
        // I14G preserves absent speed for the five additional non-equipment consumables.
        .chain([3250, 7642, 23374, 23375, 23535].map(|id| format!("oteryn:item.tibia.i{id}")))
        // READ-WRITE-1: the 19 readable profiles carry no speed attribute and are not equipment.
        .chain(
            [
                2012, 2013, 2014, 2015, 2016, 2017, 2021, 2022, 2023, 2024, 2597, 2598, 2602, 2605,
                6121, 6122, 10322, 12782, 12783,
            ]
            .map(|id| format!("oteryn:item.tibia.i{id}")),
        )
        .chain(
            [
                347, 401, 635, 636, 637, 638, 639, 640, 641, 642, 2813, 2814, 2815, 2817, 2818, 2819,
                2821, 2824, 2825, 2826, 2827, 2828, 2829, 2830, 2831, 2832, 2833, 2834, 2835,
                2838, 2839, 2840, 2841, 2842, 2843, 2844, 2845, 2846, 2847, 3212, 3505, 3507,
                4100, 4842, 4843, 5785, 5799, 5805, 5806, 5807, 5956, 6386, 6538, 7879, 8150,
                8151, 8155, 8156, 8737, 9206, 9207, 9208, 9209, 9210, 9211, 9212, 9213, 9214,
                9215, 9216, 9217, 9218, 9219, 9220, 9221, 9222, 9223, 9224, 9538, 9614, 15276,
                15633, 19072, 19073, 19074, 21445, 27932, 27933, 27934, 28572, 31266, 31268,
            ]
            .map(|id| format!("oteryn:item.tibia.i{id}")),
        )
        // MAIL-E09: the parcels and the stamped letter carry no speed attribute.
        .chain([3503, 3504, 3506].map(|id| format!("oteryn:item.tibia.i{id}")))
        .collect()
    );
    Ok(())
}

#[test]
fn equipment_effects_owner_time_ring_grants_speed_only_in_ring_slot() -> TestResult {
    let content = native(&input()?)?;
    for slot in 1..=10 {
        let equipment = snapshot(&content, "oteryn:item.tibia.i3090", slot);
        assert_eq!(
            speed_term(&equipment, &content),
            Some(if slot == 8 { 30 } else { 0 }),
            "slot {slot}"
        );
    }
    Ok(())
}

/// I05: custody keeps the inactive time ring 3053; worn, it takes the active 3090's +30 speed
/// (Crystal `items.xml`, `transformEquipTo` 3090, `speed` 30) in the ring slot only.
#[test]
fn equipment_effects_owner_inactive_time_ring_takes_worn_speed_only_in_ring_slot() -> TestResult {
    let content = native(&input()?)?;
    for slot in 1..=10 {
        let equipment = snapshot(&content, "oteryn:item.tibia.i3053", slot);
        assert_eq!(
            speed_term(&equipment, &content),
            Some(if slot == 8 { 30 } else { 0 }),
            "slot {slot}"
        );
    }
    Ok(())
}

/// I05: the active dwarven ring 3099 (Crystal `items.xml:7477-7489`, `suppressdrunk`) suppresses
/// drunkenness in the ring slot only.
#[test]
fn equipment_effects_owner_dwarven_ring_suppresses_drunk_only_in_ring_slot() -> TestResult {
    let content = native(&input()?)?;
    for slot in 1..=10 {
        let equipment = snapshot(&content, "oteryn:item.tibia.i3099", slot);
        let effects = equipment_effects(&equipment, &content).ok_or("effects")?;
        let expected: std::collections::BTreeSet<_> = if slot == 8 {
            [AbilitySuppression::Drunk].into_iter().collect()
        } else {
            Default::default()
        };
        assert_eq!(effects.suppressions, expected, "slot {slot}");
    }
    Ok(())
}

/// I05: custody keeps a skill ring's inactive form; worn, it takes the active form's skill in the
/// ring slot only (Crystal 201189f8 `items.xml:7339-7350`, power ring 3087 `skillfist` 6;
/// `:7422-7457`, sword/axe/club rings 3094/3095/3096 skill 4).
#[test]
fn equipment_effects_owner_inactive_skill_rings_take_worn_skill_only_in_ring_slot() -> TestResult {
    use oteryn_gameplay_content::content::item_abilities::AbilitySkill;
    let content = native(&input()?)?;
    for (base, skill, points) in [
        (3050, AbilitySkill::Fist, 6),
        (3091, AbilitySkill::Sword, 4),
        (3092, AbilitySkill::Axe, 4),
        (3093, AbilitySkill::Club, 4),
    ] {
        for slot in 1..=10 {
            let equipment = snapshot(&content, &format!("oteryn:item.tibia.i{base}"), slot);
            let effects = equipment_effects(&equipment, &content).ok_or("effects")?;
            let expected: std::collections::BTreeMap<_, _> = if slot == 8 {
                [(skill, points)].into_iter().collect()
            } else {
                Default::default()
            };
            assert_eq!(effects.skills, expected, "{base} slot {slot}");
        }
    }
    Ok(())
}

#[test]
fn equipment_effects_owner_reward_boots_grant_speed_only_in_feet() -> TestResult {
    let content = native(&input()?)?;
    for key in ["oteryn:item.tibia.i43884", "oteryn:item.tibia.i43887"] {
        for slot in 1..=10 {
            let equipment = snapshot(&content, key, slot);
            assert_eq!(
                speed_term(&equipment, &content),
                Some(if slot == 4 { 10 } else { 0 }),
                "{key} in durable slot {slot}"
            );
        }
    }
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
