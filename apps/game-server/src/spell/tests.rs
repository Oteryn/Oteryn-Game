#![allow(clippy::expect_used)]

use std::collections::BTreeSet;

use oteryn_simulation_determinism::SemanticTimeMicros;
use serde_json::Value;

use super::authoring::spell_from_bundle;
use super::formula::level_base_damage_healing;
use super::plan::{CastPlanError, effect_plan};
use super::*;
use crate::ability::{AbilityEngine, AbilityOccurrence, RevisionSet};

macro_rules! bundle {
    ($dir:literal) => {
        (
            include_str!(concat!(
                "../../../../tools/content-schema/spell-authoring/samples/starter-bundles/",
                $dir,
                "/spell.json"
            )),
            include_str!(concat!(
                "../../../../tools/content-schema/spell-authoring/samples/starter-bundles/",
                $dir,
                "/dependencies.json"
            )),
        )
    };
}

const STARTER: [(&str, &str); 9] = [
    bundle!("instant-light_healing"),
    bundle!("instant-intense_healing"),
    bundle!("instant-ice_strike"),
    bundle!("instant-energy_strike"),
    bundle!("instant-cure_poison"),
    bundle!("instant-sudden_death_rune"),
    bundle!("instant-great_fireball_rune"),
    bundle!("rune-sudden_death_rune"),
    bundle!("rune-great_fireball_rune"),
];

fn book() -> SpellBook {
    let spells = STARTER
        .iter()
        .map(|(spell, deps)| {
            let spell: Value = serde_json::from_str(spell).expect("spell.json");
            let deps: Value = serde_json::from_str(deps).expect("dependencies.json");
            spell_from_bundle(&spell, &deps).expect("starter bundle converts")
        })
        .collect();
    SpellBook::new(spells).expect("starter spells are unique")
}

fn caster(vocation: Vocation, level: u32, magic_level: u32, mana: u32) -> CasterState {
    CasterState {
        harmony_multiplier: super::harmony::HarmonyMultiplier::ONE,
        vocation,
        level,
        magic_level,
        premium: false,
        mana,
        max_mana: mana,
        soul: 100,
        learned: BTreeSet::new(),
        attack_skill: 10,
        attack_value: 7,
        attack_factor: 1.0,
        shielding_skill: 10,
        melee_weapon: false,
        shield_defense: None,
    }
}

fn at(millis: u64) -> SemanticTimeMicros {
    SemanticTimeMicros::from_micros(millis * 1000)
}

fn lowest(minimum: i64, _: i64) -> i64 {
    minimum
}

fn highest(_: i64, maximum: i64) -> i64 {
    maximum
}

/// Official scaling table (Tibia news of 2022-10-17, patch 13.05.12657).
fn official_level_bonus(level: u32) -> i64 {
    if level <= 500 {
        return i64::from(level / 5);
    }
    let (mut total, mut threshold, mut step, mut span) = (100_i64, 500_u32, 6_u32, 600_u32);
    while level > threshold + span {
        total += i64::from(span / step);
        threshold += span;
        step += 1;
        span += 100;
    }
    total + i64::from((level - threshold) / step)
}

#[test]
fn level_curve_matches_the_official_table() {
    for level in 0..=20_000 {
        assert_eq!(
            level_base_damage_healing(level),
            official_level_bonus(level),
            "level {level}"
        );
    }
    assert_eq!(
        level_base_damage_healing(800),
        150,
        "the news example: level 800 gives 150"
    );
}

#[test]
fn spoken_words_find_the_spell() {
    let book = book();
    let spoken = book.spoken("  EXURA ").expect("exura");
    assert_eq!(spoken.spell.name, "Light Healing");
    assert_eq!(spoken.parameter, None);
    assert!(book.spoken("exura gran").is_some());
    assert!(
        book.spoken("exura sio \"Bubble").is_none(),
        "Heal Friend is not in the starter set"
    );
    assert!(book.spoken("hello exura").is_none());
    assert_eq!(
        book.rune(3155).map(|spell| spell.key.as_str()),
        Some("candidate:spell/rune/sudden_death_rune")
    );
}

#[test]
fn light_healing_heals_within_the_formula_bounds_and_starts_cooldowns() {
    let book = book();
    let spell = book.spoken("exura").expect("exura").spell;
    let druid = caster(Vocation::Druid, 8, 0, 20);
    let low = resolve_cast(
        spell,
        &druid,
        &Cooldowns::default(),
        at(0),
        false,
        &mut lowest,
    )
    .expect("cast");
    let high = resolve_cast(
        spell,
        &druid,
        &Cooldowns::default(),
        at(0),
        false,
        &mut highest,
    )
    .expect("cast");
    // Level 8 contributes 1: minimum 1 + 0 * 1.4 + 8, maximum 1 + 0 * 1.795 + 11.
    assert_eq!(low.effects[0], ResolvedEffect::Heal { magnitude: 9 });
    assert_eq!(high.effects[0], ResolvedEffect::Heal { magnitude: 12 });
    assert_eq!(
        low.effects[1],
        ResolvedEffect::RemoveCondition {
            condition: "paralyze".into()
        }
    );
    assert_eq!((low.mana_spent, low.soul_spent), (20, 0));
    assert_eq!(low.cooldowns.group_ready_at("healing"), Some(at(1000)));
    assert_eq!(low.cooldowns.spell_ready_at(&spell.key), Some(at(1000)));

    let again = resolve_cast(spell, &druid, &low.cooldowns, at(999), false, &mut lowest);
    assert_eq!(
        again,
        Err(CastRejection::GroupCooling {
            group: "healing".into(),
            ready_at: at(1000)
        })
    );
    assert!(resolve_cast(spell, &druid, &low.cooldowns, at(1000), false, &mut lowest).is_ok());
}

#[test]
fn cast_checks_follow_the_engine_order() {
    let book = book();
    let spell = book.spoken("exura").expect("exura").spell;
    let none = Cooldowns::default();
    let reject =
        |state: CasterState| resolve_cast(spell, &state, &none, at(0), false, &mut lowest).err();
    assert_eq!(
        reject(caster(Vocation::Druid, 7, 0, 0)),
        Some(CastRejection::LevelTooLow { required: 8 })
    );
    assert_eq!(
        reject(caster(Vocation::Druid, 8, 0, 19)),
        Some(CastRejection::NotEnoughMana { required: 20 })
    );
    assert_eq!(
        reject(caster(Vocation::Knight, 8, 0, 20)),
        Some(CastRejection::VocationCannotUse)
    );
    assert_eq!(reject(caster(Vocation::ExaltedMonk, 8, 0, 20)), None);
}

#[test]
fn ice_strike_damage_uses_the_level_curve_and_needs_nothing_else() {
    let book = book();
    let spell = book.spoken("exori frigo").expect("exori frigo").spell;
    let sorcerer = caster(Vocation::MasterSorcerer, 100, 50, 100);
    let none = Cooldowns::default();
    let low = resolve_cast(spell, &sorcerer, &none, at(0), false, &mut lowest).expect("cast");
    let high = resolve_cast(spell, &sorcerer, &none, at(0), false, &mut highest).expect("cast");
    // B(100) = 20: 20 + 50 * 1.403 + 8 = 98.15 and 20 + 50 * 2.203 + 13 = 143.15, truncated.
    assert_eq!(
        low.effects,
        vec![ResolvedEffect::Damage {
            damage_type: "ice".into(),
            magnitude: 98
        }]
    );
    assert_eq!(
        high.effects,
        vec![ResolvedEffect::Damage {
            damage_type: "ice".into(),
            magnitude: 143
        }]
    );
    assert_eq!(spell.range_tiles, Some(3));
}

#[test]
fn sudden_death_rune_needs_magic_level_and_uses_base_power() {
    let book = book();
    let rune = book.rune(3155).expect("sudden death rune");
    let none = Cooldowns::default();
    let weak = caster(Vocation::Knight, 100, 14, 0);
    assert_eq!(
        resolve_cast(rune, &weak, &none, at(0), true, &mut lowest),
        Err(CastRejection::MagicLevelTooLow { required: 15 })
    );
    let mage = caster(Vocation::Sorcerer, 100, 50, 0);
    assert_eq!(
        resolve_cast(rune, &mage, &none, at(0), false, &mut lowest),
        Err(CastRejection::TargetRequired)
    );
    let low = resolve_cast(rune, &mage, &none, at(0), true, &mut lowest).expect("cast");
    let high = resolve_cast(rune, &mage, &none, at(0), true, &mut highest).expect("cast");
    // P150, buckets70: F20 + floor((1 ± 70/150/2) * (150/25*ML50 + 150/4)).
    // The independent sorcerer level100/ML50 reference returns 278..436. F is outside variation.
    assert_eq!(
        low.effects,
        vec![ResolvedEffect::Damage {
            damage_type: "death".into(),
            magnitude: 278
        }]
    );
    assert_eq!(
        high.effects,
        vec![ResolvedEffect::Damage {
            damage_type: "death".into(),
            magnitude: 436
        }]
    );
    assert_eq!(low.mana_spent, 0);
}

#[test]
fn conjuring_spell_makes_runes_from_a_blank_rune() {
    let book = book();
    let spell = book
        .spoken("adori gran mort")
        .expect("adori gran mort")
        .spell;
    let sorcerer = caster(Vocation::Sorcerer, 45, 15, 985);
    let cast = resolve_cast(
        spell,
        &sorcerer,
        &Cooldowns::default(),
        at(0),
        false,
        &mut lowest,
    )
    .expect("cast");
    assert_eq!(
        cast.effects,
        vec![ResolvedEffect::Conjure {
            reagent: Some(3147),
            result: 3155,
            count: 3
        }]
    );
    assert_eq!((cast.mana_spent, cast.soul_spent), (985, 5));
    let mut tired = caster(Vocation::Sorcerer, 45, 15, 985);
    tired.soul = 4;
    assert_eq!(
        resolve_cast(
            spell,
            &tired,
            &Cooldowns::default(),
            at(0),
            false,
            &mut lowest
        ),
        Err(CastRejection::NotEnoughSoul { required: 5 })
    );
}

#[test]
fn every_starter_formula_is_valid_over_the_level_grid() {
    let book = book();
    for (spell, _) in STARTER {
        let key =
            serde_json::from_str::<Value>(spell).expect("spell.json")["spell"]["identity"]["key"]
                .clone();
        let definition = book
            .spells
            .iter()
            .find(|s| key == s.key.as_str())
            .expect("loaded");
        let Execution::Effects(effects) = &definition.execution else {
            continue;
        };
        for effect in effects {
            let (SpellEffect::Damage { formula, .. } | SpellEffect::Heal { formula }) = effect
            else {
                continue;
            };
            for level in [1, 8, 50, 100, 500, 1000, 2500] {
                for magic_level in [0, 10, 50, 130] {
                    let inputs = FormulaInputs {
                        level,
                        magic_level,
                        base_power: definition.base_power,
                        attack_skill: 10,
                        attack_value: 7,
                        attack_factor: 1.0,
                        shielding_skill: 10,
                        shield_defense: None,
                    };
                    formula.bounds(&inputs).expect("valid bounds");
                }
            }
        }
    }
}

#[test]
fn uniform_draw_stays_in_bounds() {
    for decision in [0, 1, 7, u64::MAX] {
        let value = uniform_draw(decision, 9, 12);
        assert!((9..=12).contains(&value));
    }
    assert_eq!(uniform_draw(5, 4, 4), 4);
}

#[test]
fn spell_book_rejects_duplicate_words() {
    let book = book();
    let mut spells = book.spells.clone();
    spells.push(SpellDefinition {
        key: "candidate:spell/copy".into(),
        ..spells[0].clone()
    });
    assert!(matches!(
        SpellBook::new(spells),
        Err(SpellBookError::Words(_))
    ));
}

fn occurrence(id: &str) -> AbilityOccurrence {
    let revisions = RevisionSet::new(
        "ruleset:r1",
        "content:starter",
        "world:r1",
        "formula:s5",
        "simulation:r1",
    )
    .expect("revisions");
    AbilityOccurrence::new(id, revisions).expect("occurrence")
}

#[test]
fn a_cast_becomes_an_ability_plan_that_the_engine_commits() {
    let book = book();
    let mut engine = AbilityEngine::new();

    let exura = book.spoken("exura").expect("exura").spell;
    let druid = caster(Vocation::Druid, 8, 0, 20);
    let healed = resolve_cast(
        exura,
        &druid,
        &Cooldowns::default(),
        at(0),
        false,
        &mut highest,
    )
    .expect("cast");
    let plan = effect_plan(
        exura,
        &healed,
        "actor:druid",
        None,
        occurrence("cast:1"),
        "channel:test",
    )
    .expect("plan");
    assert_eq!(
        plan.side_effects,
        vec![ResolvedEffect::RemoveCondition {
            condition: "paralyze".into()
        }]
    );
    let receipt = engine.commit(plan.effects.expect("heal")).expect("commit");
    assert!(receipt.applied());
    assert_eq!(engine.fixture_health("actor:druid"), Some(12));

    let strike = book.spoken("exori frigo").expect("exori frigo").spell;
    let sorcerer = caster(Vocation::Sorcerer, 100, 50, 100);
    let hit = resolve_cast(
        strike,
        &sorcerer,
        &Cooldowns::default(),
        at(0),
        true,
        &mut lowest,
    )
    .expect("cast");
    assert_eq!(
        effect_plan(
            strike,
            &hit,
            "actor:sorcerer",
            None,
            occurrence("cast:2"),
            "channel:test"
        ),
        Err(CastPlanError::MissingTarget)
    );
    let plan = effect_plan(
        strike,
        &hit,
        "actor:sorcerer",
        Some("actor:rat"),
        occurrence("cast:2"),
        "channel:test",
    )
    .expect("plan");
    engine
        .commit(plan.effects.clone().expect("damage"))
        .expect("commit");
    assert_eq!(engine.fixture_health("actor:rat"), Some(-98));
    // The same occurrence commits once.
    let again = engine
        .commit(plan.effects.expect("damage"))
        .expect("idempotent");
    assert!(!again.applied());
    assert_eq!(engine.fixture_health("actor:rat"), Some(-98));
}

#[test]
fn a_conjure_has_no_ability_effects() {
    let book = book();
    let spell = book
        .spoken("adori gran mort")
        .expect("adori gran mort")
        .spell;
    let sorcerer = caster(Vocation::Sorcerer, 45, 15, 985);
    let cast = resolve_cast(
        spell,
        &sorcerer,
        &Cooldowns::default(),
        at(0),
        false,
        &mut lowest,
    )
    .expect("cast");
    let plan = effect_plan(
        spell,
        &cast,
        "actor:sorcerer",
        None,
        occurrence("cast:3"),
        "channel:test",
    )
    .expect("plan");
    assert_eq!(plan.effects, None);
    assert_eq!(plan.side_effects, cast.effects);
}

#[test]
fn a_wheel_spell_retains_its_requirement_and_refuses_missing_owner_facts() {
    let (spell, dependencies) = STARTER[0];
    let mut spell: Value = serde_json::from_str(spell).expect("spell");
    let dependencies: Value = serde_json::from_str(dependencies).expect("dependencies");
    spell["spell"]["requirements"]["wheel_unlock"] = Value::Bool(true);
    let admitted = spell_from_bundle(&spell, &dependencies).expect("typed Wheel requirement");
    assert_eq!(
        resolve_cast(
            &admitted,
            &caster(Vocation::Druid, 300, 100, 10000),
            &Cooldowns::default(),
            at(0),
            false,
            &mut |_, _| panic!("draw before Wheel proof")
        ),
        Err(CastRejection::WheelUnlockRequired)
    );
    spell["spell"]["requirements"]["wheel_unlock"] = Value::from("yes");
    assert!(spell_from_bundle(&spell, &dependencies).is_err());
    spell["spell"]["requirements"]["wheel_unlock"] = Value::Null;
    assert!(spell_from_bundle(&spell, &dependencies).is_err());
    spell["spell"]["requirements"]["wheel_unlock"] = Value::Bool(false);
    assert!(spell_from_bundle(&spell, &dependencies).is_ok());
}

#[test]
fn a_positional_spell_retains_its_requirement_and_refuses_missing_owner_facts() {
    let (spell, dependencies) = STARTER[0];
    let mut spell: Value = serde_json::from_str(spell).expect("spell");
    let dependencies: Value = serde_json::from_str(dependencies).expect("dependencies");
    spell["spell"]["targeting"]["cast_at_position"] = Value::Bool(true);
    let admitted = spell_from_bundle(&spell, &dependencies).expect("typed position requirement");
    assert_eq!(
        resolve_cast(
            &admitted,
            &caster(Vocation::Druid, 300, 100, 10000),
            &Cooldowns::default(),
            at(0),
            false,
            &mut |_, _| panic!("draw before position proof")
        ),
        Err(CastRejection::OperationalFactsRequired)
    );
    spell["spell"]["targeting"]["cast_at_position"] = Value::Null;
    assert!(spell_from_bundle(&spell, &dependencies).is_err());
    spell["spell"]["targeting"]["cast_at_position"] = Value::Bool(false);
    assert!(spell_from_bundle(&spell, &dependencies).is_ok());
    spell["spell"]["targeting"]["aim_at_target"] = Value::Bool(true);
    assert!(spell_from_bundle(&spell, &dependencies).is_ok());
}

#[test]
fn a_chain_spell_is_admitted_and_needs_the_world_facts() {
    let (spell, dependencies) = STARTER[0];
    let spell: Value = serde_json::from_str(spell).expect("spell");
    let mut dependencies: Value = serde_json::from_str(dependencies).expect("dependencies");
    assert!(spell_from_bundle(&spell, &dependencies).is_ok());
    for ability in dependencies["abilities"].as_array_mut().expect("abilities") {
        ability["chain"] = serde_json::json!({
            "max_targets": 4,
            "range_tiles": 4,
            "backtracking": false,
        });
    }
    let chained = spell_from_bundle(&spell, &dependencies).expect("chain admitted");
    let chain = chained.chain.as_ref().expect("chain");
    assert_eq!((chain.max_targets, chain.initial_range_tiles), (4, 4));
    // Cast without the world facts, a chain would silently hit one creature.
    assert_eq!(
        resolve_cast(
            &chained,
            &caster(Vocation::Druid, 8, 0, 20),
            &Cooldowns::default(),
            at(0),
            false,
            &mut lowest
        ),
        Err(CastRejection::ChainWorldRequired)
    );
}

/// S26 with SPELL-D8 §8.2 H-live: the runtime actor owns Harmony, so a monk-only builder or
/// spender is admitted with its role; an unknown role or a non-monk vocation still fails closed.
#[test]
fn a_harmony_role_is_admitted_only_for_monk_spells() {
    let (spell, dependencies) = STARTER[0];
    let dependencies: Value = serde_json::from_str(dependencies).expect("dependencies");
    let with = |role: &str, vocations: Value| {
        let mut spell: Value = serde_json::from_str(spell).expect("spell");
        spell["spell"]["harmony_role"] = Value::String(role.into());
        spell["spell"]["requirements"]["vocations"] = vocations;
        spell_from_bundle(&spell, &dependencies)
    };
    let monks = || serde_json::json!(["monk", "exalted_monk"]);
    assert_eq!(
        with("builder", monks()).expect("builder").harmony_role,
        Some(HarmonyRole::Builder)
    );
    assert_eq!(
        with("spender", monks()).expect("spender").harmony_role,
        Some(HarmonyRole::Spender)
    );
    let unknown = with("focus", monks()).expect_err("unknown role admitted");
    assert!(unknown.to_string().contains("harmony_role"), "{unknown}");
    let foreign =
        with("builder", serde_json::json!(["monk", "druid"])).expect_err("druid builder admitted");
    assert!(foreign.to_string().contains("monks only"), "{foreign}");
    let plain: Value = serde_json::from_str(spell).expect("spell");
    assert_eq!(
        spell_from_bundle(&plain, &dependencies)
            .expect("plain")
            .harmony_role,
        None
    );
}

/// §A.2 step 3b: a spender's damage bounds are each multiplied and truncated before the roll;
/// a builder's and any other spell's are not.
#[test]
fn only_a_spender_scales_its_damage_bounds_by_the_harmony_multiplier() {
    let book = book();
    let base = book.spoken("exori frigo").expect("exori frigo").spell;
    let multiplier = harmony::HarmonyMultiplier::new(100, 3, false, true).expect("multiplier");
    let sorcerer = CasterState {
        harmony_multiplier: multiplier,
        ..caster(Vocation::MasterSorcerer, 100, 50, 100)
    };
    let none = Cooldowns::default();
    let bounds = |role: Option<HarmonyRole>| {
        let spell = SpellDefinition {
            harmony_role: role,
            ..base.clone()
        };
        let low = resolve_cast(&spell, &sorcerer, &none, at(0), false, &mut lowest).expect("cast");
        let high =
            resolve_cast(&spell, &sorcerer, &none, at(0), false, &mut highest).expect("cast");
        let magnitude = |effects: &[ResolvedEffect]| match effects {
            [ResolvedEffect::Damage { magnitude, .. }] => Some(*magnitude),
            _ => None,
        };
        (magnitude(&low.effects), magnitude(&high.effects))
    };
    assert_eq!(bounds(None), (Some(98), Some(143)));
    assert_eq!(bounds(Some(HarmonyRole::Builder)), (Some(98), Some(143)));
    // B = 7 + 0.005 * 100 = 7.5 percent, x4 at three charges: m = 1.3 exactly.
    assert_eq!(multiplier.apply(98), 127);
    assert_eq!(
        bounds(Some(HarmonyRole::Spender)),
        (Some(multiplier.apply(98)), Some(multiplier.apply(143)))
    );
}
