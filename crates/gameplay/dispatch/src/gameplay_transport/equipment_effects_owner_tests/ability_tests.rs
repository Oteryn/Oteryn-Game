use super::*;

#[test]
fn equipment_effects_owner_regeneration_reads_worn_item_in_own_slot_only() -> TestResult {
    use oteryn_gameplay_progression::regeneration::EquipmentRegeneration;

    let modifier = |kind: &str, parameter: &str, value: u64| {
        serde_json::json!({"kind":kind,
          "parameter":{"state":"KNOWN","value":{"kind":parameter,"value":value}},
          "evaluation_phase":{"state":"UNKNOWN"},"priority":{"state":"UNKNOWN"},
          "target_domain":{"state":"UNKNOWN"}})
    };
    let content = native(&fixture_input(
        serde_json::json!([
            modifier("HEALTH_GAIN", "SIGNED_POINTS", 6),
            modifier("HEALTH_TICKS", "MILLISECONDS", 6_000),
            modifier("MANA_GAIN", "SIGNED_POINTS", 24),
            modifier("MANA_TICKS", "MILLISECONDS", 6_000),
        ]),
        Some(0),
    )?)?;
    let regeneration = EquipmentRegeneration {
        health_gain: 6,
        health_ticks_ms: 6_000,
        mana_gain: 24,
        mana_ticks_ms: 6_000,
    };
    for slot in 1..=10 {
        assert_eq!(
            equipped_regeneration(
                &snapshot(&content, "oteryn:item.tibia.i2854", slot),
                &content,
            ),
            Some(if slot == 9 {
                vec![(9, regeneration)]
            } else {
                vec![]
            }),
            "slot {slot}"
        );
    }

    let gain_only = native(&fixture_input(
        serde_json::json!([modifier("HEALTH_GAIN", "SIGNED_POINTS", 6)]),
        Some(0),
    )?)?;
    assert_eq!(
        equipped_regeneration(
            &snapshot(&gain_only, "oteryn:item.tibia.i2854", 9),
            &gain_only,
        ),
        Some(vec![])
    );
    let ordinary = native(&input()?)?;
    assert_eq!(
        equipped_regeneration(
            &snapshot(&ordinary, "oteryn:item.tibia.i2854", 9),
            &ordinary,
        ),
        Some(vec![])
    );
    Ok(())
}

/// The item profiles before any mechanic added its own; all of them are unchanged.
const ORIGINAL_PROFILES: usize = 432;

/// The item profiles each mechanic adds, one row per mechanic in name order, so a new mechanic
/// adds its own row instead of rewriting a shared total.
const PROFILE_ADDITIONS: &[(&str, usize)] = &[
    // M41 source operands: sculpture 31574, beak 52632 and corpse stages 52677..=52680.
    // Regenerated from m41_quest_item_profiles.IDS, independently of the profile union.
    ("M41", 6),
    // C08: Crystal items.xml:75219-75231 admits agony field 43297, with speed_bonus=0.
    ("C08", 1),
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
    // 53 exact Grizzly shop keys admitted from Crystal 201189f through
    // tools/content-migration/grizzly_shop_profiles.py. All have speed_bonus=0.
    // Twelve are already admitted by main's Task Board cohort; count each key once.
    ("GRIZZLY-SHOP", 41),
    // E12 captures 1425 bound Hireling items from Crystal 201189f.
    // Counted by production_definition.production_key against main d78ed6d6's
    // 2372 profiles: 564 already exist and 861 are new (hireling-item-profiles.json).
    // The old 909 count predates main's newer overlapping owner cohorts.
    // The native importer preserves shared attributes/richer ability facts;
    // every raw speed and the exact unknown-speed set remain checked below.
    ("HIRELING-E12", 861),
    // Count event-item-profiles.json's 328 production_definition.production_key values
    // minus the union of main d78ed6d6 and Hireling keys: 44 overlap, 284 are new.
    // The old 288 count from 3ae3372a5 predates the merged chest/shop overlap.
    // The exact source-key union is 2372 + 861 + 284 = 3517; no main key is missing.
    // Every raw speed and the exact unknown-speed set remain checked below.
    ("PKG2-CONTENT-UNION", 284),
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
    // C19b: Newhaven's 34017, 21348, 21400, 21350 and 50166 are source-qualified
    // by build_newhaven_starter_profiles.py (Canary 04b83b51 items.xml + official 15.30).
    // Each has known speed_bonus=0; preserve the exact unknown-speed set below.
    ("NEWHAVEN-C19B", 5),
    // P11: condition potions 7439/7440/7443/49271 from Crystal 201189f potions.lua:1-31
    // (tools/content-migration/condition_potion_profiles.py). All have speed_bonus=0.
    ("P11-CONDITION-POTIONS", 4),
    // P11: mechanical rod 9306 from Crystal 201189f items.xml:25156-25159
    // (tools/content-migration/gathering_item_profiles.py). It has speed_bonus=0.
    ("P11-MECHANICAL-FISHING", 1),
    // QNPCFX: tender venison 52636, source-qualified by qnpcfx_item_profiles.py
    // from Crystal 201189f items.xml:88039 and the pinned client; zero speed.
    ("QNPCFX", 1),
    // QUESTPC4: source-neutral sword52824; questpc4-stag-item-profiles.json.
    ("QUESTPC4-STAG-ITEMS", 1),
    ("QUESTPC4-TREASURE-REWARDS", 5),
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
    // W15: Holy Icon 9388 and 96 additional chest-reward profiles from Crystal 201189f
    // (tools/content-migration/quest_callback_profiles.py). Shop profile 7400 is counted
    // by GRIZZLY-SHOP above. Existing text carriers 2815/5956 are already counted.
    // All 97 additions have speed_bonus=0.
    // Ten also belong to cohorts added on main; W15 contributes 87 distinct new keys.
    ("QUESTCHEST-W15", 87),
    // ITEMHOLD-Q: captured wolf, dog collar, Monolith Kit, Bottled Dragon Breath
    // and Guardian Ring from itemhold_q{,_remaining}.py; the newly qualified
    // Ornamented Shield recipe also closes over lyre 2949 in quest_callback_profiles.py.
    // All six retain the source-qualified speed_bonus=0 (ITEMHOLD-Q review evidence).
    ("ITEMHOLD-Q", 6),
    // Item 7934 used by a world callback's item-on-target step
    // (tools/content-migration/quest_callback_profiles.py). speed_bonus=0.
    ("QUESTCB3", 1),
    // Batch 3 admits only the canonical demon helmet 3387 through quest_callback_profiles.py.
    // Crystal 201189f871d23597b82caf88fd611454a2ddefb8:data/items/items.xml:9970-9986
    // and data-global/startup/tables/chest.lua:806-811 confirm the single reward and
    // no speed attribute (speed_bonus=0); boots 3554 and shield 3420 already exist.
    ("QUESTS3-DEMON-HELMET", 1),
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
    // Task Board: 436 delivery definitions plus talisman 51978; five were already native.
    // Crystal 201189f XML/appearance and accepted canonical facts are qualified by
    // tools/content-migration/task_board_item_profiles.py; no existing cohort is removed.
    // The package adds 432 keys, but six are already counted by cohorts merged after the
    // draft base ae4188a9c: 3318, 3333, 3342, 7383 and 7402 by GEMX-I19 and 37110 by
    // W07-SOULPIT. The merged file holds each once, so this cohort counts 426 new keys.
    ("TASKBOARD-P04", 426),
    // TIMEDITEMS2-1 admits the inactive and active time-ring forms.
    ("TIMEDITEMS2-1", 2),
    // W06/W07: exact Crystal 201189f items.xml/appearances.dat capture in
    // soulpit-echo-item-profiles.json. All 845 profiles have qualified speed_bonus=0.
    // W06 contributes the 24 basic scrolls 53751..=53774. W07 contributes the 835
    // source catalogue minus 15 unbound IDs, plus Exalted Core 37110 (821 profiles).
    // P18 Forge shares that existing core profile and adds no duplicate.
    ("W06-ECHO", 24),
    ("W07-SOULPIT", 821),
    // W10-PRIMAL: generated by primal_item_profiles.py --refresh-cohort.
    // Crystal ff7ede59 data/items/items.xml:71977,73314-73335; Items 39176, 39527..=39530, speed_bonus=0.
    ("W10-PRIMAL", 5),
    // W15c debris 6475..=6483 and 6485/6486, captured by
    // tools/content-schema/native-gameplay/build_grizzly_debris_profiles.py from Crystal
    // 201189f items.xml:17891-17893 (no speed override). src/items/items.hpp:38 defaults
    // speed to zero; all eleven profiles preserve that qualified speed_bonus=0.
    ("W15C-GRIZZLY-DEBRIS", 11),
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
    // P06/P07: fifteen unrevealed vocation gems and two fragment inputs, from Crystal
    // 201189f (tools/content-migration/wheel_gem_item_profiles.py). All have speed_bonus=0.
    ("WHEEL-GEM", 17),
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
    // W22c Drome: the Crystal Cube and its twenty rewards 36723..=36742 and 36827 from
    // Crystal 201189f (tools/content-migration/drome_item_profiles.py). All have
    // speed_bonus=0. P15 Forge reuses the existing Exalted Core 37110 and adds none.
    ("W22C-DROME", 21),
    // W2V harvest/craft: 3134, 3651..=3653, 4848, 5462..=5466, 5470,
    // 9594/9596/9598, 10426..=10429 and 17151/17153. Each is source-qualified
    // by harvest-item-profiles.json from Crystal 201189f871d23597b82caf88fd611454a2ddefb8
    // data/items/items.xml and appearances.dat; all twenty have speed_bonus=0.
    ("W2V-HARVEST-CRAFT", 20),
    // Batch 2 callback Items: 2859, 3217, 18931, 19084, 19089 and 19129.
    // zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:
    // data/items/items.xml:5996,8015,38639,39126,39147,39279 has no speed override;
    // src/items/items.hpp:38 defaults ability speed to zero. The accepted
    // quest_callback_profiles.py and quests_2_emperor_item_profiles.py producers
    // retain that qualified zero for these six newly materializable profiles.
    ("QUESTS-2", 6),
    // M08-M10 sewer/fishing harvest: 953 (nail), 4609 (worked mechanical rod),
    // 9307, 21341 and 21342, added to tools/content-migration/harvest_item_profiles.py
    // IDS from Crystal 201189f871d23597b82caf88fd611454a2ddefb8 data/items/items.xml.
    // None has a speed override, so all five keep speed_bonus=0.
    ("M08-M10-SEWER-HARVEST", 5),
    // I14 special potions: antidote potion 7644 and magic shield potion 35563, added to
    // tools/content-migration/condition_potion_profiles.py IDS from Crystal potions.lua.
    // Neither has a speed override, so both keep speed_bonus=0.
    ("I14-POTIONS", 2),
    // SRVENC W02: thirteen new keys among the 22 captured Sugar Daddy corpse/loot
    // profiles (boss-reward-item-profiles.json; boss_reward_item_profiles.py).
    // Canary47df data-otservbr-global/monster/bosses/sugar_daddy.lua:21-25,76-132:
    // 45639..=45643, 48114, 48249..=48251, 48253, 48254, 48256 and corpse 48416.
    // Each retains captured speed_bonus=0; the raw-attribute loop and the exact
    // independently pinned unknown-speed set below remain unchanged.
    ("SRVENC-W02", 13),
    // SRVGAP-C1: eight pinned Canary starter profiles have no speed override.
    ("SRVGAP-C1", 8),
    // The pinned dispenser-item-profiles.json adds 22 source-qualified dispenser
    // definitions (25879..=25914, its exact source subset), all with speed_bonus=0.
    ("SRVGAP-DISPENSERS", 22),
    // Source crate disguise adds one qualified retained portable Item (11328).
    // Derived from the corpus-backed shared quest disguise descriptor, speed 0.
    ("SRVGAP-QUEST-DISGUISE", 1),
    // Crystal201189f XML: retained Shapechanger12251 and the exact eight dummy
    // IDs12047..=12053,12056. All nine are new zero-speed source profiles.
    ("M23-LAST-CREEP", 9),
    // srx1-potion-profiles.json adds 53162..=53164, each with speed_bonus=0.
    // These remain the same original SRX-1 cohorts. SEASON coalesces identical
    // dispenser facts into the base, so the supplement now carries only the
    // remaining overrides/new keys; the merged key inventory counts each once.
    ("SRX1-POTIONS", 3),
    // SV16 Echo catalysts 51588, 51589 and 54266, added to the W06/W07 overlay
    // (tools/content-migration/soulpit_echo_item_profiles.py EXTRA_IDS) from Crystal
    // 201189f871d23597b82caf88fd611454a2ddefb8 items.xml. All three keep speed_bonus=0.
    ("SV16-ECHO-CATALYSTS", 3),
    // WORLDCHG: 41 source-captured definitions; 12369 is already in ITEMHOLD-Q.
    ("WORLDCHG", 40),
    // SRV9: wedding ring 9585 and wedding outfit box 9586, Crystal 201189f8.
    ("SRV9-MARRIAGE", 2),
    // BEGIN GENERATED QREADY PROFILE ADDITIONS
    // Regenerate: python3 tools/contracts/update_equipment_profile_additions.py
    // tools/content-schema/native-gameplay/qready1-reward-item-profiles.json
    // Source-qualified zero speed: 3068, 3323, 3383, 3579, 3734, 4871.
    ("QREADY1", 6),
    // tools/content-migration/qready2_native_profiles.py
    // Source-qualified zero speed: 2822, 2823, 2848, 3310, 21982.
    ("QREADY2", 5),
    // tools/content-schema/native-gameplay/srv7-targuna-item-profiles.json
    // Source-qualified zero speed: 53073.
    ("SRV7-M14", 1),
    // END GENERATED QREADY PROFILE ADDITIONS
    // SEASON W18:2126 exact initialized House source profiles. Relative to the
    // frozen9668e0a04 base (including loadout/SRX updates),1959 keys are new.
    // Source: house-decoration-profiles.json, Crystal201189f items.xml and
    // both pinned appearance banks; no original key is removed or relabeled.
    ("SEASON-W18-HOUSE", 1959),
    // Merged main1ae39c7fb adds53 distinct QENGINE reward identities from
    // qengine-reward-item-profiles.json (pinned Canary items.xml/appearance).
    // The exact packet carries speed_bonus=0 for every record, checked below.
    ("QENGINE-REWARDS", 53),
    // W05d:48 canonical seasonal profiles;35 further keys are new after the
    // House cohort above. Source: seasonal-item-profiles.json (Santa, Ruprecht,
    // Cake and Halloween donor locators/hashes); all have qualified speed0.
    ("SEASON-W05D", 35),
];

// qengine_reward_item_profiles.py: 53 shared identities, seven QUESTPC4 tools,
// Juicy Roots 21291 (M12), and Cursed Crystal ingredients 21504/21505 (QN4).
const QENGINE_SOURCE_PROFILES: usize = 53 + 7 + 1 + 2;

#[test]
fn equipment_effects_owner_pinned_production_speed_matches_all_raw_attributes() -> TestResult {
    let input = input()?;
    let profiles: ItemProfilesDocument =
        serde_json::from_slice(&input.item_profiles.as_ref().ok_or("profiles")?.bytes)?;
    // QENGINE2 independently captures shared engine and portable quest Items.
    // QENGINE2 captures 63 source-qualified Items (qengine_reward_item_profiles.IDS).
    // QENGINE2 captures the closed shared, QUESTPC4, M12 and QN4 Item cohorts.
    // The producer's fixed IDS set is source-pinned to Canary 47dfd51f XML/client
    // data. All retain qualified zero speed; merged owner cohorts overlap.
    let proof: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../tools/content-schema/native-gameplay/qengine-reward-item-profiles.json"
    )))?;
    let additions = proof["records"]
        .as_array()
        .ok_or("QENGINE2 source profiles")?;
    let mut added_keys = std::collections::BTreeSet::new();
    for source in additions {
        let key = source["production_definition"]["production_key"]
            .as_str()
            .ok_or("QENGINE2 source key")?;
        assert!(
            added_keys.insert(key),
            "duplicate QENGINE2 source key {key}"
        );
        let profile = profiles
            .records
            .iter()
            .find(|row| row.production_definition.production_key == key)
            .ok_or_else(|| format!("missing QENGINE2 source profile {key}"))?;
        assert!(profile.admission.materializable, "{key}");
        assert_eq!(profile.attributes.speed_bonus, Some(0), "{key}");
    }
    assert_eq!(added_keys.len(), 63);
    assert_eq!(added_keys.len(), QENGINE_SOURCE_PROFILES);
    let npc_proof: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../tools/content-schema/native-gameplay/qnpcfx-item-profiles.json"
    )))?;
    let npc_additions = npc_proof["records"]
        .as_array()
        .ok_or("NPC source profiles")?;
    assert_eq!(npc_additions.len(), 1);
    assert_eq!(
        npc_additions[0]["production_definition"]["production_key"],
        "oteryn:item.tibia.i52636"
    );
    let venison = profiles
        .records
        .iter()
        .find(|row| row.production_definition.production_key == "oteryn:item.tibia.i52636")
        .ok_or("missing NPC venison profile")?;
    assert!(venison.admission.materializable);
    assert_eq!(venison.attributes.speed_bonus, Some(0));
    let world_change_proof: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../tools/content-schema/native-gameplay/world-change-item-profiles.json"
    )))?;
    let world_change_records = world_change_proof["records"]
        .as_array()
        .ok_or("world-change profiles")?;
    assert_eq!(world_change_records.len(), 41);
    for source in world_change_records {
        let key = source["production_definition"]["production_key"]
            .as_str()
            .ok_or("world-change key")?;
        let profile = profiles
            .records
            .iter()
            .find(|row| row.production_definition.production_key == key)
            .ok_or_else(|| format!("missing world-change source profile {key}"))?;
        assert!(profile.admission.materializable, "{key}");
        assert_eq!(profile.attributes.speed_bonus, Some(0), "{key}");
    }
    for id in [9585, 9586] {
        let key = format!("oteryn:item.tibia.i{id}");
        let profile = profiles
            .records
            .iter()
            .find(|row| row.production_definition.production_key == key)
            .ok_or_else(|| format!("missing marriage source profile {key}"))?;
        assert!(profile.admission.materializable, "{key}");
        assert_eq!(profile.attributes.speed_bonus, Some(0), "{key}");
    }

    assert_eq!(
        profiles.records.len(),
        ORIGINAL_PROFILES
            + PROFILE_ADDITIONS.iter().map(|(_, n)| n).sum::<usize>()
            + (additions.len() - 53)
            // Source-backed QUESTPC4 floor/callback cohorts add 26 + 23 keys.
            + 26
            + 23
    );
    // Independently replay the closed source packets' zero-speed facts;
    // the unchanged per-row raw-ability and unknown-speed checks below still
    // reject loss, invention or accidental overwrite of any prior owner rule.
    for (path, expected_count) in [
        ("house-decoration-profiles.json", 2126),
        ("seasonal-item-profiles.json", 48),
        ("qengine-reward-item-profiles.json", 63),
        ("qengine-reward-item-profiles.json", QENGINE_SOURCE_PROFILES),
    ] {
        let source: serde_json::Value = serde_json::from_slice(&std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../tools/content-schema/native-gameplay")
                .join(path),
        )?)?;
        let records = source["records"]
            .as_array()
            .ok_or("missing source Item profiles")?;
        assert_eq!(records.len(), expected_count, "{path}");
        for source in records {
            let key = source["production_definition"]["production_key"]
                .as_str()
                .ok_or("source Item key")?;
            assert_eq!(
                source["attributes"]["speed_bonus"],
                serde_json::json!(0),
                "source speed {path}:{key}"
            );
            let actual = profiles
                .records
                .iter()
                .find(|row| row.production_definition.production_key == key)
                .ok_or_else(|| format!("missing source-qualified SEASON profile {key}"))?;
            assert_eq!(actual.attributes.speed_bonus, Some(0), "{path}:{key}");
            assert!(actual.admission.materializable, "{path}:{key}");
        }
    }
    for id in [2949, 12369, 14156, 33330, 44179, 52732] {
        let key = format!("oteryn:item.tibia.i{id}");
        let record = profiles
            .records
            .iter()
            .find(|record| record.production_definition.production_key == key)
            .ok_or_else(|| format!("missing source-qualified ITEMHOLD-Q profile {key}"))?;
        assert!(record.admission.materializable, "{key}");
        assert_eq!(record.attributes.speed_bonus, Some(0), "{key}");
    }
    // SRX-1 qualifies zero speed for these 34 newly admitted writing items.
    // The producer preserves 2815/2821/3505/3507 under their existing owners;
    // those four therefore retain their unknown-speed assertions below.
    for id in [
        637, 638, 639, 640, 641, 642, 2598, 2605, 2813, 2814, 2817, 2818, 2819, 2824, 2825, 2826,
        2828, 2829, 2830, 2831, 2832, 2834, 2838, 2839, 2840, 2841, 2843, 2844, 2845, 2846, 2847,
        4842, 4843, 7879,
    ] {
        let key = format!("oteryn:item.tibia.i{id}");
        let record = profiles
            .records
            .iter()
            .find(|record| record.production_definition.production_key == key)
            .ok_or_else(|| format!("missing source-qualified writing profile {key}"))?;
        assert_eq!(record.attributes.speed_bonus, Some(0), "{key}");
        assert!(
            !matches!(record.semantics.equipment, ReferenceItemField::Known(_)),
            "writing profile {key} remains non-equipment",
        );
    }
    for id in [34017, 21348, 21400, 21350, 50166] {
        let key = format!("oteryn:item.tibia.i{id}");
        let record = profiles
            .records
            .iter()
            .find(|record| record.production_definition.production_key == key)
            .ok_or_else(|| format!("missing source-qualified Newhaven profile {key}"))?;
        assert_eq!(record.attributes.speed_bonus, Some(0), "{key}");
    }
    for id in [2859, 3217, 18931, 19084, 19089, 19129] {
        let key = format!("oteryn:item.tibia.i{id}");
        let record = profiles
            .records
            .iter()
            .find(|record| record.production_definition.production_key == key)
            .ok_or_else(|| format!("missing source-qualified batch 2 profile {key}"))?;
        assert_eq!(record.attributes.speed_bonus, Some(0), "{key}");
    }
    // This expected set comes from the independently pinned source catalogue and the
    // explicit Item admission gaps, never from the native profiles being tested.
    // These are known zero-speed Items, so the pre-existing unknown-speed set below
    // remains unchanged and continues to reject any accidental loss of qualification.
    let soulpit_echo_items: std::collections::BTreeSet<_> =
        crate::domain::soulpit::SOUL_CORE_SOURCE_IDS
            .iter()
            .copied()
            .filter(|id| *id != 47899 && !(53142..=53155).contains(id))
            .chain(std::iter::once(37110))
            .chain(53751..=53774)
            .map(|id| format!("oteryn:item.tibia.i{id}"))
            .collect();
    assert_eq!(soulpit_echo_items.len(), 845);
    for key in soulpit_echo_items {
        let record = profiles
            .records
            .iter()
            .find(|record| record.production_definition.production_key == key)
            .ok_or_else(|| format!("missing source-qualified W06/W07 profile {key}"))?;
        assert_eq!(record.attributes.speed_bonus, Some(0), "{key}");
    }
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
        // I05 keeps these three timed rings' equipment effects unhosted. E12
        // adds their physical Trader custody without granting equip admission;
        // the raw source attributes are still checked independently above.
        let expected = match record.production_definition.production_key.as_str() {
            "oteryn:item.tibia.i3049" | "oteryn:item.tibia.i3052" | "oteryn:item.tibia.i3098" => {
                None
            }
            _ => Some(0),
        };
        assert_eq!(
            speed_term(&equipment, &content),
            expected,
            "{}",
            record.production_definition.production_key
        );
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
        // READ-WRITE-1: retain unknown speed for the original readable cohort,
        // except 2598/2605 now explicitly qualified by SRX-1 writing above.
        .chain(
            [
                2012, 2013, 2014, 2015, 2016, 2017, 2021, 2022, 2023, 2024, 2597, 2602, 6121, 6122,
                10322, 12782, 12783,
            ]
            .map(|id| format!("oteryn:item.tibia.i{id}")),
        )
        .chain(
            [
                347, 401, 635, 636, 2815, 2821, 2827, 2833, 2835, 2842, 3212, 3505, 3507, 4100,
                5785, 5799, 5805, 5806, 5807, 5956, 6386, 6538, 8150, 8151, 8155, 8156, 8737, 9206,
                9207, 9208, 9209, 9210, 9211, 9212, 9213, 9214, 9215, 9216, 9217, 9218, 9219, 9220,
                9221, 9222, 9223, 9224, 9538, 9614, 15276, 15633, 19072, 19073, 19074, 21445,
                27932, 27933, 27934, 28572, 31266, 31268,
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
/// ring slot only. Monk Vocation Update 2025-04-08, TibiaWiki Power Ring history: +6 -> +4.
/// Crystal 201189f8 supplies the ring pairs (`items.xml:7339-7350`;
/// `:7422-7457`, sword/axe/club rings 3094/3095/3096 skill 4).
#[test]
fn equipment_effects_owner_inactive_skill_rings_take_worn_skill_only_in_ring_slot() -> TestResult {
    use oteryn_gameplay_content::content::item_abilities::AbilitySkill;
    let content = native(&input()?)?;
    for (base, skill, points) in [
        (3050, AbilitySkill::Fist, 4),
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
fn equipment_effects_owner_staff_native_profile_matches_current_monk_facts() -> TestResult {
    let content = native(&input()?)?;
    let policy = content
        .item_policy("oteryn:item.tibia.i3289", "definition-r1")
        .ok_or("staff native profile")?;
    let semantics = serde_json::to_value(&policy.record().semantics)?;
    let weapon = &semantics["weapon"]["value"];
    for (field, value) in [
        ("weapon_type", serde_json::json!("FIST")),
        ("attack", serde_json::json!(22)),
        ("defense", serde_json::json!(14)),
    ] {
        assert_eq!(
            weapon[field],
            serde_json::json!({"state": "KNOWN", "value": value})
        );
    }
    let patterns = &semantics["equipment"]["value"]["patterns"]["value"];
    assert_eq!(patterns.as_array().ok_or("staff patterns")?.len(), 1);
    for (field, value) in [
        ("vocations", serde_json::json!(["MONK"])),
        ("level", serde_json::json!(15)),
        ("primary_slot", serde_json::json!("WEAPON")),
        ("additional_reserved_slots", serde_json::json!(["SHIELD"])),
    ] {
        assert_eq!(
            patterns[0][field],
            serde_json::json!({"state": "KNOWN", "value": value})
        );
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
fn equipment_effects_owner_invisibility_reads_worn_item_in_own_slot_only() -> TestResult {
    let invisible = |value| {
        serde_json::json!({"kind":"INVISIBILITY",
          "parameter":{"state":"KNOWN","value":{"kind":"BOOLEAN","value":value}},
          "evaluation_phase":{"state":"UNKNOWN"},"priority":{"state":"UNKNOWN"},"target_domain":{"state":"UNKNOWN"}})
    };
    let content = native(&fixture_input(
        serde_json::json!([invisible(true)]),
        Some(0),
    )?)?;
    for slot in 1..=10 {
        assert_eq!(
            equipped_invisibility(
                &snapshot(&content, "oteryn:item.tibia.i2854", slot),
                &content
            ),
            Some(slot == 9),
            "slot {slot}"
        );
    }

    let content = native(&fixture_input(
        serde_json::json!([invisible(false)]),
        Some(0),
    )?)?;
    assert_eq!(
        equipped_invisibility(&snapshot(&content, "oteryn:item.tibia.i2854", 9), &content),
        Some(false)
    );

    let content = native(&input()?)?;
    assert_eq!(
        equipped_invisibility(&snapshot(&content, "oteryn:item.tibia.i2854", 9), &content),
        Some(false)
    );
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
