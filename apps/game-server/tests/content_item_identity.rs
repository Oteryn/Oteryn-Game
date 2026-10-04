//! ITEM-ID-1 (decision `A12-ITEM-IDENTITY-TIBIA-ID-V1`): the alias table, the semantic key
//! constants and the fail-closed key switch.

#![allow(clippy::expect_used, clippy::panic)]

use oteryn_game_server::content::{
    DefinitionIdentityDocument, ItemStackDocument, ProjectDraft, ProjectReferenceRecord,
    ProjectV2DefinitionRef, ProjectV2Draft, ProjectV2EditorEntry, ProjectV2Family, ProjectV2State,
    ProjectionDocument, ReferenceItemField,
    item_identity::{
        APPEARANCE_ONLY_ITEM_IDS, ItemIdentityError, ItemKeyAliasTable, RetiredItemKey,
        apply_tibia_id_key_rule, apply_tibia_id_key_rule_with_appearance_items,
        is_canonical_item_key, semantic, tibia_item_key,
    },
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const ALIASES: &[u8] = include_bytes!("../../../content/items/aliases.json");
const REFERENCE: &[u8] = include_bytes!("../../../content/world/definitions/reference.json");

/// Each retired named key and the constant that replaces it (A12 §4.3, 64 CW2-B1 keys plus
/// the R7-P04 gold coin).
const SEMANTIC_ITEM_KEYS: [(&str, &str); 65] = [
    ("oteryn:item.ammunition.arrow", semantic::AMMUNITION_ARROW),
    ("oteryn:item.armor.plate_armor", semantic::ARMOR_PLATE_ARMOR),
    (
        "oteryn:item.charged.might_ring",
        semantic::CHARGED_MIGHT_RING,
    ),
    (
        "oteryn:item.charged.sacred_tree_amulet",
        semantic::CHARGED_SACRED_TREE_AMULET,
    ),
    (
        "oteryn:item.charged.stone_skin_amulet",
        semantic::CHARGED_STONE_SKIN_AMULET,
    ),
    (
        "oteryn:item.consumable.potion.great_health",
        semantic::CONSUMABLE_POTION_GREAT_HEALTH,
    ),
    (
        "oteryn:item.consumable.potion.health",
        semantic::CONSUMABLE_POTION_HEALTH,
    ),
    (
        "oteryn:item.consumable.potion.mana",
        semantic::CONSUMABLE_POTION_MANA,
    ),
    (
        "oteryn:item.consumable.potion.strong_mana",
        semantic::CONSUMABLE_POTION_STRONG_MANA,
    ),
    (
        "oteryn:item.consumable.sudden_death_rune",
        semantic::CONSUMABLE_SUDDEN_DEATH_RUNE,
    ),
    (
        "oteryn:item.container.book_backpack",
        semantic::CONTAINER_BOOK_BACKPACK,
    ),
    (
        "oteryn:item.container.deepling_backpack",
        semantic::CONTAINER_DEEPLING_BACKPACK,
    ),
    ("oteryn:item.container.fur_bag", semantic::CONTAINER_FUR_BAG),
    (
        "oteryn:item.container.golden_backpack",
        semantic::CONTAINER_GOLDEN_BACKPACK,
    ),
    (
        "oteryn:item.container.moon_backpack",
        semantic::CONTAINER_MOON_BACKPACK,
    ),
    (
        "oteryn:item.container.pillow_backpack",
        semantic::CONTAINER_PILLOW_BACKPACK,
    ),
    (
        "oteryn:item.container.pirate_backpack",
        semantic::CONTAINER_PIRATE_BACKPACK,
    ),
    (
        "oteryn:item.container.pirate_bag",
        semantic::CONTAINER_PIRATE_BAG,
    ),
    (
        "oteryn:item.currency.crystal_coin",
        semantic::CURRENCY_CRYSTAL_COIN,
    ),
    (
        "oteryn:item.currency.gold_coin",
        semantic::CURRENCY_GOLD_COIN,
    ),
    (
        "oteryn:item.currency.platinum_coin",
        semantic::CURRENCY_PLATINUM_COIN,
    ),
    ("oteryn:item.decor.candlestick", semantic::DECOR_CANDLESTICK),
    ("oteryn:item.decor.flask.brown", semantic::DECOR_FLASK_BROWN),
    (
        "oteryn:item.decor.oil_lamp.small",
        semantic::DECOR_OIL_LAMP_SMALL,
    ),
    ("oteryn:item.decor.panpipes", semantic::DECOR_PANPIPES),
    ("oteryn:item.decor.piggy_bank", semantic::DECOR_PIGGY_BANK),
    (
        "oteryn:item.decor.pillow.small_blue",
        semantic::DECOR_PILLOW_SMALL_BLUE,
    ),
    ("oteryn:item.decor.teddy_bear", semantic::DECOR_TEDDY_BEAR),
    ("oteryn:item.decor.tome.blue", semantic::DECOR_TOME_BLUE),
    ("oteryn:item.decor.tome.purple", semantic::DECOR_TOME_PURPLE),
    ("oteryn:item.decor.tome.red", semantic::DECOR_TOME_RED),
    ("oteryn:item.decor.vase", semantic::DECOR_VASE),
    (
        "oteryn:item.equipment.crown_helmet",
        semantic::EQUIPMENT_CROWN_HELMET,
    ),
    (
        "oteryn:item.equipment.crusader_helmet",
        semantic::EQUIPMENT_CRUSADER_HELMET,
    ),
    (
        "oteryn:item.equipment.glacier_robe",
        semantic::EQUIPMENT_GLACIER_ROBE,
    ),
    (
        "oteryn:item.equipment.knight_armor",
        semantic::EQUIPMENT_KNIGHT_ARMOR,
    ),
    (
        "oteryn:item.equipment.magma_monocle",
        semantic::EQUIPMENT_MAGMA_MONOCLE,
    ),
    (
        "oteryn:item.equipment.plate_legs",
        semantic::EQUIPMENT_PLATE_LEGS,
    ),
    (
        "oteryn:item.equipment.platinum_amulet",
        semantic::EQUIPMENT_PLATINUM_AMULET,
    ),
    (
        "oteryn:item.equipment.steel_helmet",
        semantic::EQUIPMENT_STEEL_HELMET,
    ),
    (
        "oteryn:item.equipment.steel_shield",
        semantic::EQUIPMENT_STEEL_SHIELD,
    ),
    (
        "oteryn:item.equipment.terra_legs",
        semantic::EQUIPMENT_TERRA_LEGS,
    ),
    (
        "oteryn:item.equipment.wooden_shield",
        semantic::EQUIPMENT_WOODEN_SHIELD,
    ),
    (
        "oteryn:item.material.gem.small_enchanted_amethyst",
        semantic::MATERIAL_GEM_SMALL_ENCHANTED_AMETHYST,
    ),
    (
        "oteryn:item.material.gem.small_enchanted_emerald",
        semantic::MATERIAL_GEM_SMALL_ENCHANTED_EMERALD,
    ),
    (
        "oteryn:item.material.gem.small_enchanted_ruby",
        semantic::MATERIAL_GEM_SMALL_ENCHANTED_RUBY,
    ),
    (
        "oteryn:item.material.gem.small_enchanted_sapphire",
        semantic::MATERIAL_GEM_SMALL_ENCHANTED_SAPPHIRE,
    ),
    ("oteryn:item.material.worm", semantic::MATERIAL_WORM),
    (
        "oteryn:item.physical.crystal.ice.flawless",
        semantic::PHYSICAL_CRYSTAL_ICE_FLAWLESS,
    ),
    ("oteryn:item.physical.marlin", semantic::PHYSICAL_MARLIN),
    ("oteryn:item.physical.nail", semantic::PHYSICAL_NAIL),
    (
        "oteryn:item.physical.soil.glimmering",
        semantic::PHYSICAL_SOIL_GLIMMERING,
    ),
    (
        "oteryn:item.physical.soil.natural",
        semantic::PHYSICAL_SOIL_NATURAL,
    ),
    ("oteryn:item.weapon.axe.fire", semantic::WEAPON_AXE_FIRE),
    ("oteryn:item.weapon.axe.hand", semantic::WEAPON_AXE_HAND),
    ("oteryn:item.weapon.axe.knight", semantic::WEAPON_AXE_KNIGHT),
    (
        "oteryn:item.weapon.blade.dagger",
        semantic::WEAPON_BLADE_DAGGER,
    ),
    (
        "oteryn:item.weapon.club.battle_hammer",
        semantic::WEAPON_CLUB_BATTLE_HAMMER,
    ),
    ("oteryn:item.weapon.club.mace", semantic::WEAPON_CLUB_MACE),
    (
        "oteryn:item.weapon.club.skull_staff",
        semantic::WEAPON_CLUB_SKULL_STAFF,
    ),
    ("oteryn:item.weapon.ranged.bow", semantic::WEAPON_RANGED_BOW),
    (
        "oteryn:item.weapon.ranged.crossbow",
        semantic::WEAPON_RANGED_CROSSBOW,
    ),
    (
        "oteryn:item.weapon.sword.bright",
        semantic::WEAPON_SWORD_BRIGHT,
    ),
    ("oteryn:item.weapon.sword.fire", semantic::WEAPON_SWORD_FIRE),
    (
        "oteryn:item.weapon.wand.snakebite_rod",
        semantic::WEAPON_WAND_SNAKEBITE_ROD,
    ),
];

fn table() -> ItemKeyAliasTable {
    ItemKeyAliasTable::parse(ALIASES).expect("committed alias table")
}

fn content_item_keys() -> BTreeSet<String> {
    let reference: Value = serde_json::from_slice(REFERENCE).expect("reference JSON");
    reference["records"]
        .as_array()
        .expect("records")
        .iter()
        .filter(|record| record["identity"]["family"] == "Item")
        .map(|record| record["identity"]["key"].as_str().expect("key").to_owned())
        .collect()
}

#[test]
fn alias_table_is_total_and_resolves_the_d149_and_r7_p04_cases() {
    let table = table();
    assert_eq!(table.len(), 38_562);
    assert_eq!(
        table.resolve("oteryn:item.registry.i00002921"),
        table.resolve("oteryn:item.currency.gold_coin")
    );
    assert!(matches!(
        table.resolve("oteryn:item.registry.i00000001"),
        Some(RetiredItemKey::WithoutSuccessor { .. })
    ));
    assert_eq!(table.resolve(semantic::CURRENCY_GOLD_COIN), None);
    let mut drifted = ALIASES.to_vec();
    drifted[0] = b' ';
    assert_eq!(
        ItemKeyAliasTable::parse(&drifted),
        Err(ItemIdentityError::AliasTableBytes)
    );
}

#[test]
fn semantic_constants_resolve_to_existing_tibia_keys() {
    let table = table();
    let content = content_item_keys();
    assert_eq!(
        SEMANTIC_ITEM_KEYS
            .iter()
            .map(|(retired, _)| retired)
            .collect::<BTreeSet<_>>()
            .len(),
        65
    );
    for (retired, constant) in SEMANTIC_ITEM_KEYS {
        assert_eq!(
            table.resolve(retired),
            Some(&RetiredItemKey::Alias {
                target: (*constant).to_owned()
            }),
            "{retired}"
        );
        assert!(content.contains(constant), "{constant}");
    }
}

#[test]
fn content_names_only_canonical_item_keys() {
    let table = table();
    let content = content_item_keys();
    assert_eq!(content.len(), 34_032);
    for key in &content {
        assert!(is_canonical_item_key(key), "{key}");
        assert!(key.starts_with("oteryn:item.tibia.i"), "{key}");
        assert_eq!(table.resolve(key), None, "{key}");
    }
    assert!(is_canonical_item_key("oteryn:item.oteryn.event_token"));
    for key in [
        "oteryn:item.tibia.i0",
        "oteryn:item.tibia.i03031",
        "oteryn:item.tibia.i",
        "oteryn:item.oteryn.Event",
        "oteryn:item.oteryn._x",
        "oteryn:item.currency.gold_coin",
    ] {
        assert!(!is_canonical_item_key(key), "{key}");
    }
    assert_eq!(tibia_item_key(0), None);
    assert_eq!(
        tibia_item_key(3031).as_deref(),
        Some(semantic::CURRENCY_GOLD_COIN)
    );
}

#[test]
fn snowball_current_appearance_has_only_an_identity_record() {
    let reference: Value = serde_json::from_slice(REFERENCE).expect("reference JSON");
    let snowball = reference["records"]
        .as_array()
        .expect("reference records")
        .iter()
        .find(|record| record["identity"]["key"] == "oteryn:item.tibia.i53855")
        .expect("admitted Snowball identity");
    assert_eq!(
        snowball,
        &serde_json::to_value(item("oteryn:item.tibia.i53855")).expect("identity-only Item")
    );
}

fn item(key: &str) -> ProjectReferenceRecord {
    ProjectReferenceRecord::Item {
        identity: DefinitionIdentityDocument {
            family: "Item".to_owned(),
            key: key.to_owned(),
            revision: "definition-r1".to_owned(),
        },
        client_projection: ProjectionDocument::ClientSafe,
        materializable: false,
        stack_class: ItemStackDocument::Unknown,
        semantics: Default::default(),
    }
}

fn editor(key: &str) -> ProjectV2EditorEntry {
    ProjectV2EditorEntry {
        target: ProjectV2DefinitionRef {
            family: ProjectV2Family::Item,
            key: key.to_owned(),
            revision: "definition-r1".to_owned(),
        },
        display_name: "x".to_owned(),
        description: String::new(),
        categories: Vec::new(),
        notes: Vec::new(),
        aliases: Vec::new(),
        tags: Vec::new(),
    }
}

fn draft(records: &[&str], editors: &[&str]) -> ProjectV2Draft {
    ProjectV2Draft {
        core: ProjectDraft {
            project_revision: "r".to_owned(),
            package_key: "oteryn:content.test".to_owned(),
            semantic_schema_version: "reference-schema-v1".to_owned(),
            licensing_metadata: "PENDING".to_owned(),
            world_id: "0123456789ab70cd8ef0123456789abc".to_owned(),
            coordinate_frame: "f".to_owned(),
            records: records.iter().map(|key| item(key)).collect(),
            imports: Vec::new(),
            metadata: Vec::new(),
        },
        state: ProjectV2State {
            editor: editors.iter().map(|key| editor(key)).collect(),
            ..ProjectV2State::default()
        },
    }
}

fn source_ids() -> BTreeMap<String, u64> {
    BTreeMap::from([
        ("oteryn:item.registry.i00000001".to_owned(), 1),
        ("oteryn:item.registry.i00002921".to_owned(), 3031),
        ("oteryn:item.registry.i00003031".to_owned(), 3147),
    ])
}

#[test]
fn key_switch_rekeys_by_rule_removes_d149_and_rewrites_references() {
    let table = table();
    let mut switched = draft(
        &[
            "oteryn:item.registry.i00000001",
            "oteryn:item.registry.i00002921",
            "oteryn:item.registry.i00003031",
        ],
        &["oteryn:item.currency.gold_coin", "oteryn:item.tibia.i3147"],
    );
    let switch = apply_tibia_id_key_rule(&mut switched, &table, &source_ids()).expect("switch");
    assert_eq!(switch.item_records, 2);
    assert_eq!(switch.removed_without_successor, 1);
    assert_eq!(
        switched,
        draft(
            &["oteryn:item.tibia.i3031", "oteryn:item.tibia.i3147"],
            &["oteryn:item.tibia.i3031", "oteryn:item.tibia.i3147"],
        )
    );
}

#[test]
fn key_switch_fails_closed() {
    let table = table();
    let run = |records: &[&str], editors: &[&str], ids: BTreeMap<String, u64>| {
        apply_tibia_id_key_rule(&mut draft(records, editors), &table, &ids)
    };
    let records = [
        "oteryn:item.registry.i00000001",
        "oteryn:item.registry.i00002921",
    ];
    assert_eq!(
        run(&records, &["oteryn:item.registry.i00000001"], source_ids()),
        Err(ItemIdentityError::RetiredReference(
            "oteryn:item.registry.i00000001".to_owned()
        ))
    );
    assert_eq!(
        run(&records, &["oteryn:item.tibia.i3147"], source_ids()),
        Err(ItemIdentityError::DanglingReference(
            "oteryn:item.tibia.i3147".to_owned()
        ))
    );
    // The rule is the record's own source id; an alias that disagrees fails.
    let mut wrong = source_ids();
    wrong.insert("oteryn:item.registry.i00002921".to_owned(), 3032);
    assert_eq!(
        run(&records, &[], wrong),
        Err(ItemIdentityError::RuleMismatch(
            "oteryn:item.registry.i00002921".to_owned()
        ))
    );
    // Both historical keys of Crystal 3031 cannot be two records.
    assert_eq!(
        run(
            &[
                "oteryn:item.registry.i00002921",
                "oteryn:item.currency.gold_coin"
            ],
            &[],
            BTreeMap::from([
                ("oteryn:item.registry.i00002921".to_owned(), 3031),
                ("oteryn:item.currency.gold_coin".to_owned(), 3031),
            ]),
        ),
        Err(ItemIdentityError::RuleMismatch(
            "oteryn:item.currency.gold_coin".to_owned()
        ))
    );
    let mut untyped = draft(&records, &[]);
    untyped.state.editor.push(ProjectV2EditorEntry {
        tags: vec!["oteryn:item.registry.i00002921".to_owned()],
        ..editor("oteryn:item.registry.i00002921")
    });
    assert_eq!(
        apply_tibia_id_key_rule(&mut untyped, &table, &source_ids()),
        Err(ItemIdentityError::UntypedItemKey(
            "oteryn:item.registry.i00002921".to_owned()
        ))
    );
}

fn appearance_items() -> Vec<ProjectReferenceRecord> {
    APPEARANCE_ONLY_ITEM_IDS
        .iter()
        .map(|id| item(&format!("oteryn:item.tibia.i{id}")))
        .collect()
}

#[test]
fn appearance_extension_preserves_exact_historical_baseline_output() {
    let table = table();
    let historical = [
        "oteryn:item.registry.i00000001",
        "oteryn:item.registry.i00002921",
    ];
    let mut expected = draft(&historical, &["oteryn:item.currency.gold_coin"]);
    let baseline = apply_tibia_id_key_rule(&mut expected, &table, &source_ids()).expect("baseline");
    expected.core.records.extend(appearance_items());
    let mut actual = draft(&historical, &["oteryn:item.currency.gold_coin"]);
    let switch = apply_tibia_id_key_rule_with_appearance_items(
        &mut actual,
        &table,
        &source_ids(),
        appearance_items(),
    )
    .expect("verified appearance extension");
    assert_eq!(actual, expected);
    assert_eq!(
        serde_json::to_vec(&actual.core.records).expect("actual"),
        serde_json::to_vec(&expected.core.records).expect("expected")
    );
    assert_eq!(switch.item_records, baseline.item_records + 61);
    assert_eq!(
        switch.removed_without_successor,
        baseline.removed_without_successor
    );
}

#[test]
fn appearance_extension_resolves_the_new_creature_stage_i44048_reference() {
    let table = table();
    let mut candidate = draft(
        &["oteryn:item.registry.i00002921"],
        &["oteryn:item.tibia.i44048"],
    );
    assert_eq!(
        apply_tibia_id_key_rule(&mut candidate.clone(), &table, &source_ids()),
        Err(ItemIdentityError::DanglingReference(
            "oteryn:item.tibia.i44048".to_owned()
        ))
    );
    let switch = apply_tibia_id_key_rule_with_appearance_items(
        &mut candidate,
        &table,
        &source_ids(),
        appearance_items(),
    )
    .expect("accepted appearance reference now closes");
    assert_eq!(switch.item_records, 62);
    assert_eq!(
        candidate.state.editor[0].target.key,
        "oteryn:item.tibia.i44048"
    );
}

#[test]
fn appearance_extension_rejects_identity_and_gameplay_substitution() {
    let table = table();
    for mutant in 0..7 {
        let mut record = item("oteryn:item.tibia.i44048");
        let ProjectReferenceRecord::Item {
            identity,
            client_projection,
            materializable,
            stack_class,
            semantics,
        } = &mut record
        else {
            panic!("Item fixture")
        };
        match mutant {
            0 => identity.revision = "definition-r2".to_owned(),
            1 => identity.family = "Creature".to_owned(),
            2 => identity.key = "oteryn:item.tibia.i044048".to_owned(),
            3 => *materializable = true,
            4 => *stack_class = ItemStackDocument::NonStackable,
            5 => semantics.temporal = ReferenceItemField::NotApplicable,
            6 => *client_projection = ProjectionDocument::ServerOnly,
            _ => unreachable!(),
        }
        assert!(
            apply_tibia_id_key_rule_with_appearance_items(
                &mut draft(&[], &[]),
                &table,
                &BTreeMap::new(),
                vec![record],
            )
            .is_err(),
            "mutant {mutant}"
        );
    }
    let non_item = ProjectReferenceRecord::Formula {
        identity: DefinitionIdentityDocument {
            family: "Formula".to_owned(),
            key: "oteryn:formula.example".to_owned(),
            revision: "definition-r1".to_owned(),
        },
    };
    assert_eq!(
        apply_tibia_id_key_rule_with_appearance_items(
            &mut draft(&[], &[]),
            &table,
            &BTreeMap::new(),
            vec![non_item],
        ),
        Err(ItemIdentityError::AliasTable(
            "appearance record is not an Item"
        ))
    );
}

#[test]
fn appearance_extension_rejects_outside_cohort_missing_retired_duplicate_and_bound_ids() {
    let table = table();
    // 3031 is a current appearance, but not in the accepted appearance-only cohort.
    for id in [3031, 48296, 53161] {
        assert!(
            apply_tibia_id_key_rule_with_appearance_items(
                &mut draft(&[], &[]),
                &table,
                &BTreeMap::new(),
                vec![item(&format!("oteryn:item.tibia.i{id}"))],
            )
            .is_err(),
            "id {id}"
        );
    }
    assert!(
        apply_tibia_id_key_rule_with_appearance_items(
            &mut draft(&[], &[]),
            &table,
            &BTreeMap::new(),
            vec![
                item("oteryn:item.tibia.i44048"),
                item("oteryn:item.tibia.i44048")
            ],
        )
        .is_err()
    );
    assert!(
        apply_tibia_id_key_rule_with_appearance_items(
            &mut draft(&[], &[]),
            &table,
            &BTreeMap::from([("source:already-bound".to_owned(), 44048)]),
            vec![item("oteryn:item.tibia.i44048")],
        )
        .is_err()
    );
}

#[test]
fn appearance_extension_keeps_historical_protected_failures() {
    let table = table();
    assert_eq!(
        apply_tibia_id_key_rule_with_appearance_items(
            &mut draft(
                &["oteryn:item.registry.i00000001"],
                &["oteryn:item.registry.i00000001"]
            ),
            &table,
            &source_ids(),
            appearance_items(),
        ),
        Err(ItemIdentityError::RetiredReference(
            "oteryn:item.registry.i00000001".to_owned()
        ))
    );
    assert_eq!(
        apply_tibia_id_key_rule_with_appearance_items(
            &mut draft(&["oteryn:item.tibia.i44048"], &[]),
            &table,
            &source_ids(),
            appearance_items(),
        ),
        Err(ItemIdentityError::RuleMismatch(
            "oteryn:item.tibia.i44048".to_owned()
        ))
    );
    let mut untyped = draft(&[], &[]);
    untyped.state.editor.push(ProjectV2EditorEntry {
        tags: vec!["oteryn:item.tibia.i44048".to_owned()],
        ..editor("oteryn:item.tibia.i44048")
    });
    assert_eq!(
        apply_tibia_id_key_rule_with_appearance_items(
            &mut untyped,
            &table,
            &source_ids(),
            appearance_items(),
        ),
        Err(ItemIdentityError::UntypedItemKey(
            "oteryn:item.tibia.i44048".to_owned()
        ))
    );
}
