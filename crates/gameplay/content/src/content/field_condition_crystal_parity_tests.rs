//! C08 parity: movement-field damage recipes against Crystal `201189f8` (`data/items/items.xml`
//! `field` attributes, `ItemParse::parseFieldCombatDamage`, `ConditionDamage::addDamage` and
//! `generateDamageList`; the parser and both condition functions are identical in Canary
//! `47dfd51f`). The item specs below are recorded source data and the oracle is a test-only
//! restatement of that parser, not production code.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use super::*;
use oteryn_gameplay_domain::domain::condition::ConditionValues;

/// One `field` attribute: its item ids, element and ordered `key`/`value` children.
type Spec = (&'static [u32], &'static str, &'static [(&'static str, i32)]);

const FIRE_7_10: &[(&str, i32)] = &[("ticks", 10_000), ("count", 7), ("damage", 10)];
const POISON_RAMP: &[(&str, i32)] = &[("ticks", 5_000), ("start", 5), ("damage", 100)];

/// Every damaging `field` item in Crystal `201189f8` `items.xml` (the Oteryn-supported elements).
const CRYSTAL_FIELDS: &[Spec] = &[
    (&[105, 2121], "poison", POISON_RAMP),
    (
        &[1998, 1999, 2000],
        "fire",
        &[("ticks", 4_000), ("count", 2), ("damage", 20)],
    ),
    (
        &[2118],
        "fire",
        &[("ticks", 10_000), ("count", 7), ("damage", 20)],
    ),
    (&[2119, 2123, 2124, 10069, 10070, 10071], "fire", FIRE_7_10),
    (&[2120, 2125, 2131, 2132, 2133], "fire", &[]),
    (&[2134], "poison", &[]),
    (
        &[2122, 23366, 31380],
        "energy",
        &[("ticks", 10_000), ("damage", 25)],
    ),
    (
        &[2126],
        "energy",
        &[("ticks", 5_000), ("damage", 100), ("start", 5)],
    ),
    (
        &[
            2137, 2138, 7465, 7466, 7467, 7468, 7469, 7470, 7471, 7472, 7473,
        ],
        "fire",
        &[("damage", 300)],
    ),
    (&[2140], "fire", &[("damage", 0)]),
    (&[23365], "fire", &[("ticks", 10_000), ("damage", 25)]),
    (&[23367], "poison", &[("ticks", 10_000), ("damage", 25)]),
    (
        &[23368],
        "poison",
        &[("ticks", 5_000), ("damage", 100), ("start", 5)],
    ),
];
/// Crystal magic fields with no `field` attribute: they carry no damage condition.
const CRYSTAL_NON_DAMAGING: &[u32] = &[2128, 2129, 2130, 2135, 2136, 10181, 10182];

/// `ConditionDamage::generateDamageList`.
fn damage_list(amount: i32, start: i32) -> Vec<i32> {
    let mut list = Vec::new();
    let mut sum = 0_i32;
    for i in (1..=start).rev() {
        let med = ((start + 1 - i) * amount) / start;
        loop {
            sum += i;
            list.push(i);
            let x1 = (1.0_f64 - f64::from((sum as f32 + i as f32) / med as f32)).abs();
            let x2 = (1.0_f64 - f64::from(sum as f32 / med as f32)).abs();
            // C++ `do { ... } while (x1 < x2)`.
            if x1 < x2 {
                continue;
            }
            break;
        }
    }
    list
}

/// `parseFieldCombatDamage` then `addDamage` (interval at least the 1000 ms creature think),
/// as consecutive `(amount, interval)` runs.
fn crystal_entries(children: &[(&str, i32)]) -> Vec<FieldDamageEntry> {
    let (mut ticks, mut count, mut start) = (0_i32, 1_i32, 0_i32);
    let mut ticks_list = Vec::new();
    for &(key, value) in children {
        match key {
            "ticks" => ticks = value,
            "count" => count = value.max(1),
            "start" => start = value.max(0),
            "damage" => {
                let interval = ticks.max(1_000);
                if start == 0 {
                    ticks_list.extend(std::iter::repeat_n((value, interval), count as usize));
                }
                ticks_list.extend(damage_list(value, start).into_iter().map(|d| (d, interval)));
                start = 0;
            }
            _ => panic!("unrecorded key {key}"),
        }
    }
    let mut entries: Vec<FieldDamageEntry> = Vec::new();
    for (amount, interval) in ticks_list {
        let (amount, interval_ms) = (amount as u32, interval as u32);
        match entries.last_mut() {
            Some(last) if last.amount == amount && last.interval_ms == interval_ms => {
                last.repetitions += 1
            }
            _ => entries.push(FieldDamageEntry {
                amount,
                interval_ms,
                repetitions: 1,
            }),
        }
    }
    entries
}

fn element(name: &str) -> FieldElement {
    match name {
        "fire" => FieldElement::Fire,
        "energy" => FieldElement::Energy,
        "poison" => FieldElement::Poison,
        _ => panic!("unrecorded element {name}"),
    }
}

fn profiles(server: &str) -> Vec<QualifiedFieldCondition> {
    let catalog: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../tools/content-schema/spell-authoring/samples/native-field-profiles.json"
    ))
    .unwrap();
    catalog["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["source"]["server"] == server)
        .map(|r| serde_json::from_value(r.clone()).unwrap())
        .collect()
}

/// The field recipes the production Item content carries.
fn production_recipes() -> Vec<QualifiedFieldCondition> {
    fn walk(value: &serde_json::Value, out: &mut Vec<QualifiedFieldCondition>) {
        match value {
            serde_json::Value::Object(map) => {
                if let Some(recipe) = map.get("field_condition").filter(|v| !v.is_null()) {
                    out.push(serde_json::from_value(recipe.clone()).unwrap());
                }
                map.values().for_each(|v| walk(v, out));
            }
            serde_json::Value::Array(items) => items.iter().for_each(|v| walk(v, out)),
            _ => {}
        }
    }
    let content: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../content/items/definitions/spell-native-profiles.json"
    ))
    .unwrap();
    let mut out = Vec::new();
    walk(&content, &mut out);
    out
}

fn crystal_expected(item: u32) -> Option<(FieldElement, Vec<FieldDamageEntry>)> {
    CRYSTAL_FIELDS
        .iter()
        .find(|(ids, ..)| ids.contains(&item))
        .map(|(_, name, children)| (element(name), crystal_entries(children)))
}

/// The production consumer's damage steps for a recipe.
fn steps(recipe: &QualifiedFieldCondition) -> Option<(DotElement, Vec<FieldDamageEntry>)> {
    match recipe.condition_values().unwrap() {
        Some(ConditionValues::DamageSequence {
            element,
            steps,
            len,
            delayed,
        }) => {
            assert!(
                !delayed,
                "Crystal startCondition deals the first field tick at once"
            );
            Some((
                element,
                steps[..usize::from(len)]
                    .iter()
                    .map(|s| FieldDamageEntry {
                        amount: s.amount,
                        interval_ms: s.interval_ms,
                        repetitions: s.repetitions,
                    })
                    .collect(),
            ))
        }
        None => None,
        Some(other) => panic!("not a damage sequence: {other:?}"),
    }
}

fn expected_steps(item: u32) -> Option<(DotElement, Vec<FieldDamageEntry>)> {
    let (element, entries) = crystal_expected(item)?;
    let total: u32 = entries.iter().map(|e| e.amount * e.repetitions).sum();
    (total > 0).then(|| (element.condition_element(), entries))
}

#[test]
fn c08_crystal_field_recipes_equal_crystal_items_xml() {
    let crystal = profiles("crystal");
    for recipe in &crystal {
        let item = recipe.source.server_item_id;
        match (&recipe.condition, crystal_expected(item)) {
            (FieldConditionKind::Damage { element, entries }, Some(expected)) => {
                assert_eq!((*element, entries.clone()), expected, "item {item}");
                assert_eq!(steps(recipe), expected_steps(item), "item {item}");
            }
            (FieldConditionKind::None, None) => {
                assert!(CRYSTAL_NON_DAMAGING.contains(&item), "item {item}");
                assert_eq!(steps(recipe), None);
            }
            // Crystal's `agony` field (start 260 above damage 100) has no Oteryn element.
            (FieldConditionKind::Unsupported { source_element }, None) => {
                assert_eq!((item, source_element.as_str()), (43297, "agony"));
            }
            (condition, expected) => panic!("item {item}: {condition:?} vs {expected:?}"),
        }
    }
    let covered: usize = CRYSTAL_FIELDS.iter().map(|(ids, ..)| ids.len()).sum();
    assert_eq!(crystal.len(), covered + CRYSTAL_NON_DAMAGING.len() + 1);
}

/// The production recipes follow Canary; they equal Crystal on every item but the
/// Optional-PvP "neutral" field variants and Canary's yellow flame.
#[test]
fn c08_production_field_damage_equals_crystal_except_the_recorded_items() {
    const DIVERGENT: [u32; 5] = [2131, 2132, 2134, 2135, 21465];
    let recipes = production_recipes();
    assert!(recipes.len() >= 15, "{}", recipes.len());
    let mut divergent = Vec::new();
    for recipe in &recipes {
        assert_eq!(recipe.source.server, "canary");
        let item = recipe.source.server_item_id;
        if steps(recipe) == expected_steps(item) {
            assert!(!DIVERGENT.contains(&item), "item {item}");
        } else {
            divergent.push(item);
        }
    }
    divergent.sort_unstable();
    // Crystal: 2132/2134/2135 deal nothing, and 21465 is not a field; Canary damages.
    assert_eq!(divergent, [2132, 2134, 2135, 21465]);
    let canary = profiles("canary");
    for (item, expected) in [
        (2131, Some((DotElement::Fire, 20, 10_000, 7))),
        (2132, Some((DotElement::Fire, 10, 10_000, 7))),
        (2134, Some((DotElement::Poison, 100, 5_000, 1))),
        (2135, Some((DotElement::Energy, 25, 10_000, 1))),
        (21465, Some((DotElement::Fire, 10, 10_000, 7))),
    ] {
        let recipe = canary
            .iter()
            .find(|r| r.source.server_item_id == item)
            .unwrap();
        let got = steps(recipe).map(|(element, entries)| {
            assert_eq!(entries.len(), 1);
            let e = entries[0];
            (element, e.amount, e.interval_ms, e.repetitions)
        });
        assert_eq!(got, expected, "item {item}");
        assert_eq!(expected_steps(item), None, "item {item}");
    }
}

/// C07: decay timers are Item attributes, not the rune Lua combat callback.
/// Crystal 201189f871d23597b82caf88fd611454a2ddefb8 data/items/items.xml:
/// 105:33-44, 2118..2122:4253-4310, 2132..2135:4393-4420.
/// These values already match production; no Global/Crystal conflict is resolved here.
#[test]
fn c07_production_field_decay_equals_crystal_for_shared_field_items() {
    let document: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../content/items/definitions/spell-native-profiles.json"
    ))
    .unwrap();
    let items = document["records"].as_array().unwrap();
    for (item, duration_ms, target) in [
        (105, 248_000, None),
        (2118, 200_000, Some(2119)),
        (2119, 148_000, Some(2120)),
        (2120, 98_000, None),
        (2122, 98_000, None),
        (2132, 120_000, Some(2133)),
        (2133, 120_000, None),
        (2134, 120_000, None),
        (2135, 120_000, None),
    ] {
        let external_id = item.to_string();
        let item = items
            .iter()
            .find(|record| record["production_binding"]["external_id"] == external_id)
            .unwrap();
        let temporal = &item["semantics"]["temporal"]["value"];
        assert_eq!(temporal["duration"]["value"], duration_ms);
        assert_eq!(temporal["stop_duration"]["value"], false);
        let decay = &temporal["decay_target"];
        if let Some(target) = target {
            assert_eq!(decay["state"], "KNOWN");
            assert_eq!(
                decay["value"]["key"],
                format!("oteryn:item.tibia.i{target}")
            );
        } else {
            assert_eq!(decay["state"], "NOT_APPLICABLE");
        }
    }
}
