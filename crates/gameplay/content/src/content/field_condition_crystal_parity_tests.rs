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
fn c07_owner_global_field_entry_and_residual_recipes() {
    let mut covered = std::collections::BTreeSet::new();
    for recipe in production_recipes().into_iter().chain(profiles("canary")) {
        let item = recipe.source.server_item_id;
        let (element, interval, runs): (_, _, &[(u32, u32)]) = match item {
            2118 | 2119 | 2132 => (DotElement::Fire, 9_000, &[(20, 1), (10, 7)]),
            2122 | 2135 => (DotElement::Energy, 11_000, &[(30, 1), (25, 1)]),
            105 | 2134 => (
                DotElement::Poison,
                4_000,
                &[(5, 6), (4, 5), (3, 7), (2, 10), (1, 19)],
            ),
            // Owner Global PKG1R4: 2123 has a separate 19 entry hit, then
            // seven Burning ticks of 9 every 9 s (unchanged from PKG1R3).
            // PKG1R6 owner 2026-10-30, 0% resistance, 1998 big flames:
            // 17:43:03 entry 19, then seven Burning ticks of 9 through 17:44:06.
            1998 | 2123 => (DotElement::Fire, 9_000, &[(19, 1), (9, 7)]),
            // Owner Global PKG1R4: 2124 has six Burning ticks of 9 every
            // 9 s, first on step-in, with no separate 19 entry hit.
            // PKG1R6, 0% resistance, first tick on step-in: 1999 medium
            // 17:46:45..17:47:30, 2000 embers 17:36:18..17:37:03.
            1999 | 2000 | 2124 => (DotElement::Fire, 9_000, &[(9, 6)]),
            2120 | 2125 | 2133 => {
                assert_eq!(steps(&recipe), None, "harmless final fire {item}");
                continue;
            }
            10069..=10071 | 2126 | 2137 | 2138 | 7465..=7473 | 2140 => {
                assert_eq!(
                    steps(&recipe),
                    expected_steps(item),
                    "excluded hazard {item}"
                );
                continue;
            }
            21465 => {
                assert_eq!(
                    steps(&recipe),
                    Some((
                        DotElement::Fire,
                        vec![FieldDamageEntry {
                            amount: 10,
                            interval_ms: 10_000,
                            repetitions: 7,
                        }]
                    )),
                    "excluded yellow flame"
                );
                continue;
            }
            _ => continue,
        };
        covered.insert(item);
        assert_eq!(
            recipe.total_damage().unwrap(),
            runs.iter()
                .map(|(amount, count)| amount * count)
                .sum::<u32>(),
            "total damage {item}"
        );
        let expected = runs
            .iter()
            .map(|&(amount, repetitions)| FieldDamageEntry {
                amount,
                interval_ms: interval,
                repetitions,
            })
            .collect();
        assert_eq!(steps(&recipe), Some((element, expected)), "item {item}");
    }
    assert_eq!(
        covered.into_iter().collect::<Vec<_>>(),
        [
            105, 1998, 1999, 2000, 2118, 2119, 2122, 2123, 2124, 2132, 2134, 2135
        ]
    );
}

/// PKG1R4: 10069..10071 remain unmeasured and retain main's recipes.
/// PKG1R6 lifts the campfire exclusion using the owner's measured stages.
#[test]
fn c07_pkg1r4_unmeasured_fields_retain_main_recipes() {
    let mut covered = std::collections::BTreeSet::new();
    for recipe in production_recipes().into_iter().chain(profiles("canary")) {
        let item = recipe.source.server_item_id;
        let (amount, interval_ms, repetitions) = match item {
            10069..=10071 => (10, 10_000, 7),
            // Owner Global PKG1R4: 2125 causes zero damage on entry and
            // no Burning ticks (no damage interval), matching main/Crystal.
            2125 => {
                covered.insert(item);
                assert_eq!(recipe.total_damage().unwrap(), 0);
                assert_eq!(steps(&recipe), None);
                continue;
            }
            _ => continue,
        };
        covered.insert(item);
        let expected = Some((
            DotElement::Fire,
            vec![FieldDamageEntry {
                amount,
                interval_ms,
                repetitions,
            }],
        ));
        assert_eq!(steps(&recipe), expected, "main recipe {item}");
        assert_eq!(steps(&recipe), expected_steps(item), "Crystal {item}");
    }
    assert_eq!(
        covered.into_iter().collect::<Vec<_>>(),
        [2125, 10069, 10070, 10071]
    );
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

/// Production retains the recorded Canary exceptions and C07-2 owner-approved
/// Global rune recipes (PKG1R3 owner table: 2118/2119/2132 entry 20 +
/// seven ticks of 10 at 9 s; 2122/2135 entry 30 + one tick of 25 at 11 s;
/// 105/2134 entry 5 + 5x5, 5x4, 7x3, 10x2, 19x1 at 4 s).
/// Owner Global PKG1R4 measurements override Crystal for map fire:
/// 2123 separate entry 19 + seven Burning ticks of 9 every 9 s;
/// 2124 six Burning ticks of 9 every 9 s, first on step-in, no separate entry.
/// PKG1R6 owner 2026-10-30 campfire mapping and Global measurements:
/// (0% resistance) 1998 big flames: 17:43:03 entry 19 + seven ticks of 9
/// through 17:44:06; 1999 medium: six ticks of 9, 17:46:45..17:47:30;
/// 2000 embers: six ticks of 9, 17:36:18..17:37:03.
/// All Burning intervals are 9 s; 1999/2000 first tick on step-in, no entry hit.
/// Unmeasured 10069..10071 retain main's recipe, without a Global override.
/// Exact Global recipes are asserted separately; raw Crystal parity stays above.
#[test]
fn c08_production_field_damage_equals_crystal_except_the_recorded_items() {
    const DIVERGENT: &[u32] = &[
        105, 1998, 1999, 2000, 2118, 2119, 2122, 2123, 2124, 2132, 2134, 2135, 21465,
    ];
    let recipes = production_recipes();
    let canary = profiles("canary");
    assert!(recipes.len() >= 15, "{}", recipes.len());
    let mut divergent = Vec::new();
    // Campfires are qualified but absent from the current production Item export.
    // Check their runtime lowering too, without broadening content admission.
    for recipe in recipes.iter().chain(
        canary
            .iter()
            .filter(|r| matches!(r.source.server_item_id, 1998..=2000)),
    ) {
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
    assert_eq!(divergent, DIVERGENT);
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
        // Safe rune products now lower to the separately asserted Global recipe;
        // preserve these donor assertions without changing the pinned capture.
        let got = if matches!(item, 2132 | 2134 | 2135) {
            match &recipe.condition {
                FieldConditionKind::Damage { element, entries } if !entries.is_empty() => {
                    Some((element.condition_element(), entries.clone()))
                }
                _ => None,
            }
        } else {
            steps(recipe)
        }
        .map(|(element, entries)| {
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
/// Owner-approved Global exception (2026-10-30): 2122 lasts 100 s, not Crystal's 98 s.
/// Two owner measurements and TibiaWiki Energy_Field_(Field) revision 1011133 win.
/// Every other shared timer remains Crystal parity.
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
        (2122, 100_000, None),
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
