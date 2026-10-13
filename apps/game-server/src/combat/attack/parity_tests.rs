//! ATTACK-PARITY-1b: the auto-attack bounds against the pinned TibiaTools expression and the
//! TibiaPal fixtures (ATTACK-PARITY-1 packet 1.3, 1.4, 2.2).
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use super::*;
use crate::spell::formula::FormulaInputs;
use serde_json::Value;

const FIXTURE: &str =
    include_str!("../../../../../content/combat/parity/tibiapal_auto_attack_v1.json");
const STATS_PACKET: &str =
    include_str!("../../../../../docs/agents/evidence/OTV2-20260930-item-stats-promotion-v2.json");

/// Weapon rows whose Oteryn `weapon.attack` is unknown or differs from the catalogue attack
/// (packet 1.4): `(item key, oteryn attack or none, tibiatools attack)`. Empty: all 15 selected
/// weapons agree. The test keeps the list exact in both directions.
const ATTACK_MISMATCHES: &[(&str, Option<i64>, i64)] = &[];

/// Rows whose exact uniform mean differs from `raw.avg` (packet 1.4). Under the uniform draw
/// this is every row: TibiaPal's `avg` is `flat + v`, the engine's mean `flat + 1.25 v`. The test
/// fails if a row stops being a residual, so the attack values keep `PARITY_PENDING` until
/// ATTACK-DIST-0 decides the distribution.
const MEAN_RESIDUAL_ROWS: usize = 1536;

fn table() -> &'static AttackConstants {
    AttackConstants::checked_in().expect("the checked-in constants table parses")
}

fn fixture() -> Value {
    serde_json::from_str(FIXTURE).expect("the fixture parses")
}

fn int(value: &Value) -> i64 {
    value.as_i64().expect("an integer")
}

/// Oteryn `weapon.attack` per item key from the ITEM-SEM-2b-1 stats packet.
fn oteryn_attack(key: &str) -> Option<i64> {
    let packet: Value = serde_json::from_str(STATS_PACKET).expect("the stats packet parses");
    packet["promotions"]
        .as_array()
        .expect("promotions")
        .iter()
        .find(|row| row["field_path"] == "weapon.attack" && row["item_key"] == key)
        .map(|row| int(&row["typed_value"]["value"]))
}

fn highest_attack() -> i64 {
    let packet: Value = serde_json::from_str(STATS_PACKET).expect("the stats packet parses");
    packet["promotions"]
        .as_array()
        .expect("promotions")
        .iter()
        .filter(|row| row["field_path"] == "weapon.attack")
        .map(|row| int(&row["typed_value"]["value"]))
        .max()
        .expect("weapon attacks")
}

/// The transcription of tibiatools `a1d368906c` `damage.ts:13-25` and `character-state.ts:73-75`
/// (fists: attack 7, no monk factor). Doubles in the same operation order as the source.
fn oracle(level: i64, skill: i64, attack: i64) -> (i64, i64, i64) {
    let step = (((2.0 * level as f64 + 2025.0).sqrt() + 5.0) / 10.0).floor();
    let flat = step * 100.0 - 450.0 + ((level as f64 + 1000.0) / step - 50.0 * step).floor();
    let v = (6.0 * attack as f64 / 5.0).floor() * (skill as f64 + 4.0) / 28.0;
    (
        (flat + v / 2.0).floor() as i64,
        (flat + v).floor() as i64,
        (flat + 2.0 * v).floor() as i64,
    )
}

fn inputs(level: u32, skill: u32, attack: u32) -> FormulaInputs {
    FormulaInputs {
        level,
        magic_level: 0,
        base_power: None,
        attack_skill: skill,
        attack_value: attack,
        attack_factor: table().fight_mode(FightMode::Offensive).attack_factor,
        shielding_skill: 0,
        shield_defense: None,
    }
}

#[test]
fn offensive_is_the_unit_factor_the_fixtures_bind() {
    assert_eq!(table().fight_mode(FightMode::Offensive).attack_factor, 1.0);
    let fixture = fixture();
    for row in fixture["rows"].as_array().unwrap() {
        assert_eq!(row["oteryn_fight_mode"], "OFFENSIVE");
        assert_eq!(row["oteryn_attack_factor"], 1.0);
        assert!(
            !["monk"].contains(&row["request"]["stats"]["vocation"].as_str().unwrap()),
            "no monk row"
        );
    }
}

#[test]
fn selected_weapons_have_the_catalogue_attack_or_are_listed() {
    let fixture = fixture();
    let mut mismatches = Vec::new();
    for weapon in fixture["grid"]["weapons"].as_array().unwrap() {
        let key = weapon["oteryn_item_key"].as_str().unwrap();
        let expected = int(&weapon["tibiatools_attack"]);
        let ours = oteryn_attack(key);
        if ours != Some(expected) {
            mismatches.push((key.to_owned(), ours, expected));
        }
    }
    let listed: Vec<_> = ATTACK_MISMATCHES
        .iter()
        .map(|(key, ours, theirs)| ((*key).to_owned(), *ours, *theirs))
        .collect();
    assert_eq!(mismatches, listed);
}

#[test]
fn engine_equals_the_pinned_expression_over_the_offline_range() {
    let fist = player_fist_formula(table());
    let melee = player_melee_formula(table());
    let fixture = fixture();
    let levels: Vec<i64> = fixture["grid"]["levels"]
        .as_array()
        .unwrap()
        .iter()
        .map(int)
        .collect();
    let highest = highest_attack();
    for level in levels {
        for skill in 10..=130_i64 {
            let (min, _, max) = oracle(level, skill, 7);
            let given = inputs(level as u32, skill as u32, 7);
            assert_eq!(fist.bounds(&given), Ok((min, max)), "fists {level} {skill}");
            for attack in 0..=highest {
                let (min, _, max) = oracle(level, skill, attack);
                let given = inputs(level as u32, skill as u32, attack as u32);
                assert_eq!(
                    melee.bounds(&given),
                    Ok((min, max)),
                    "attack {attack} {level} {skill}"
                );
            }
        }
    }
}

#[test]
fn min_and_max_equal_every_fixture_row_and_the_mean_residuals_are_exact() {
    let fixture = fixture();
    let fist = player_fist_formula(table());
    let melee = player_melee_formula(table());
    let attacks: std::collections::BTreeMap<i64, i64> = fixture["grid"]["weapons"]
        .as_array()
        .unwrap()
        .iter()
        .map(|weapon| {
            (
                int(&weapon["tibiatools_id"]),
                int(&weapon["tibiatools_attack"]),
            )
        })
        .collect();
    let rows = fixture["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1536);
    let mut residuals = 0;
    for row in rows {
        let stats = &row["request"]["stats"];
        let (level, skill) = (int(&stats["level"]) as u32, int(&stats["skill"]) as u32);
        let weapon = int(&row["request"]["weapon"]["id"]);
        let (formula, attack) = if weapon == 1 {
            (&fist, 7)
        } else {
            (&melee, attacks[&weapon] as u32)
        };
        let (min, max) = formula.bounds(&inputs(level, skill, attack)).unwrap();
        let key = row["key"].as_str().unwrap();
        assert_eq!(min, int(&row["raw"]["min"]), "{key} min");
        assert_eq!(max, int(&row["raw"]["max"]), "{key} max");
        // The draw is uniform over [min, max]: its exact mean is (min + max) / 2.
        if (min + max).div_euclid(2) != int(&row["raw"]["avg"]) {
            residuals += 1;
        }
    }
    assert_eq!(residuals, MEAN_RESIDUAL_ROWS);
}

#[test]
fn attack_values_stay_pending_while_the_mean_residuals_stand() {
    // Every row has a mean residual (above), so no attack value may be labelled matched.
    let table: Value = serde_json::from_str(super::constants::CHECKED_IN_JSON).unwrap();
    assert_eq!(table["parity"], "PARITY_PENDING");
    for (value, parity) in table["parity_by_value"].as_object().unwrap() {
        assert_eq!(parity, "PARITY_PENDING", "{value}");
    }
}
