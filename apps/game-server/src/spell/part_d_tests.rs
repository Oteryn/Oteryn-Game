//! Engine tests of part D (`OTERYN_SPELL_NATIVE_BEHAVIOURS_CANDIDATE_V1.md`): the healing rune
//! target rule of D.3 (`targeting.allowed_targets`), the weapon and shield requirements of D.4
//! (`needs_weapon`, `needs_shield`, the `shield_defense` formula input), and the rejection of the
//! Part D keys that have no implementation yet.

#![allow(clippy::expect_used)]

use std::collections::BTreeSet;

use oteryn_simulation_determinism::SemanticTimeMicros;
use serde_json::{Value, json};

use super::authoring::spell_from_bundle;
use super::plan::{CastPlanError, effect_plan};
use super::target::{AllowedTargets, CastTarget};
use super::*;
use crate::ability::{AbilityEngine, AbilityOccurrence, RevisionSet};

const RUNE: &str = include_str!(
    "../../../../tools/content-schema/spell-authoring/samples/starter-bundles/rune-sudden_death_rune/spell.json"
);
pub(super) const HEALING: &str = include_str!(
    "../../../../tools/content-schema/spell-authoring/samples/starter-bundles/instant-light_healing/dependencies.json"
);
const STRIKE: &str = include_str!(
    "../../../../tools/content-schema/spell-authoring/samples/starter-bundles/instant-energy_strike/spell.json"
);
const STRIKE_DEPENDENCIES: &str = include_str!(
    "../../../../tools/content-schema/spell-authoring/samples/starter-bundles/instant-energy_strike/dependencies.json"
);

pub(super) const CASTER: u64 = 1;
pub(super) const OTHER_PLAYER: u64 = 2;
pub(super) const SUMMON: u64 = 3;

fn var(name: &str) -> Value {
    json!({ "var": name })
}

fn constant(value: &str) -> Value {
    json!({ "const": value })
}

fn op(name: &str, left: Value, right: Value) -> Value {
    json!({ "op": name, "args": [left, right] })
}

/// `level / 5 + magic_level * factor + flat` (the Canary healing rune form, kept by S4).
fn healing_bound(factor: &str, flat: &str) -> Value {
    op(
        "add",
        op(
            "add",
            op("div", var("level"), constant("5")),
            op("mul", var("magic_level"), constant(factor)),
        ),
        constant(flat),
    )
}

/// Ultimate Healing Rune as the D.3 authoring shape: a heal and a paralysis removal, used on a
/// creature, `allowed_targets` self or own summons (Q7, the F reading), rune users without the
/// monks (Q8), healing group 1 s.
fn ultimate_healing_rune_bundle(allowed_targets: &str) -> (Value, Value) {
    let mut spell: Value = serde_json::from_str(RUNE).expect("rune spell");
    let body = &mut spell["spell"];
    body["identity"]["key"] = json!("candidate:spell/rune/ultimate_healing_rune");
    body["name"] = json!("ultimate healing rune");
    body["requirements"] = json!({
        "vocations": [
            "druid", "elder_druid", "elite_knight", "knight", "master_sorcerer", "paladin",
            "royal_paladin", "sorcerer"
        ],
        "level": 24,
        "premium": false,
        "learning_required": false
    });
    body["cooldown_ms"] = json!(1000);
    body["groups"] = json!([{ "group": "healing", "cooldown_ms": 1000 }]);
    body["targeting"]["aggressive"] = json!(false);
    body["targeting"]["allowed_targets"] = json!(allowed_targets);
    body["base_power"] = json!(250);
    body["rune"]["item"]["key"] = json!("candidate:item/3160");
    body["rune"]["charges"] = json!(1);
    body["rune"]["magic_level"] = json!(4);
    let mut dependencies: Value = serde_json::from_str(HEALING).expect("healing dependencies");
    body["execution"]["ability"] = json!({
        "family": "Ability",
        "key": dependencies["abilities"][0]["identity"]["key"],
        "revision": dependencies["abilities"][0]["identity"]["revision"]
    });
    dependencies["abilities"][0]["needs_target"] = json!(true);
    dependencies["formulas"][0]["minimum"] = healing_bound("7.3", "42");
    dependencies["formulas"][0]["maximum"] = healing_bound("12.4", "90");
    (spell, dependencies)
}

pub(super) fn ultimate_healing_rune() -> SpellDefinition {
    let (spell, dependencies) = ultimate_healing_rune_bundle("self_or_own_summons");
    spell_from_bundle(&spell, &dependencies).expect("Ultimate Healing Rune admitted")
}

/// `base_power * shielding_skill / 100 * shield_defense / 10 * spread` (D.4.1, Canary).
fn shield_bound(spread: &str) -> Value {
    op(
        "mul",
        op(
            "mul",
            op(
                "mul",
                var("base_power"),
                op("div", var("shielding_skill"), constant("100")),
            ),
            op("div", var("shield_defense"), constant("10")),
        ),
        constant(spread),
    )
}

/// Shield Bash as the D.4 authoring shape, without the debuff Effect (its condition type has no
/// owner yet): `needs_shield`, range 1, physical damage from the shield formula.
fn shield_bash_bundle(needs_shield: Option<bool>) -> (Value, Value) {
    let mut spell: Value = serde_json::from_str(STRIKE).expect("strike spell");
    let body = &mut spell["spell"];
    body["identity"]["key"] = json!("candidate:spell/shield_bash");
    body["name"] = json!("Shield Bash");
    body["words"] = json!("exori ico scu");
    body["requirements"] = json!({
        "vocations": ["elite_knight", "knight"],
        "level": 18,
        "premium": false,
        "learning_required": false
    });
    body["costs"] = json!({ "mana": 30, "soul": 0 });
    body["cooldown_ms"] = json!(4000);
    body["targeting"]["needs_target"] = json!(true);
    body["targeting"]["target_or_direction"] = json!(false);
    body["targeting"]["range_tiles"] = json!(1);
    body["base_power"] = json!(55);
    if let Some(needs_shield) = needs_shield {
        body["needs_shield"] = json!(needs_shield);
    }
    let mut dependencies: Value =
        serde_json::from_str(STRIKE_DEPENDENCIES).expect("strike dependencies");
    dependencies["effects"][0]["damage_type"] = json!("physical");
    dependencies["effects"][0]["mitigated_by"] = json!(["armor"]);
    let formula = &mut dependencies["formulas"][0];
    formula["inputs"] = json!("skill");
    formula["minimum"] = shield_bound("0.9");
    formula["maximum"] = shield_bound("1.1");
    (spell, dependencies)
}

fn shield_bash() -> SpellDefinition {
    let (spell, dependencies) = shield_bash_bundle(Some(true));
    spell_from_bundle(&spell, &dependencies).expect("Shield Bash admitted")
}

pub(super) fn caster(vocation: Vocation) -> CasterState {
    CasterState {
        harmony_multiplier: super::harmony::HarmonyMultiplier::ONE,
        vocation,
        level: 100,
        magic_level: 50,
        premium: false,
        mana: 1000,
        max_mana: 1000,
        soul: 100,
        learned: BTreeSet::new(),
        attack_skill: 10,
        attack_value: 7,
        attack_factor: 1.0,
        shielding_skill: 100,
        melee_weapon: false,
        shield_defense: None,
    }
}

fn knight_with_shield(defense: u32) -> CasterState {
    CasterState {
        shield_defense: Some(defense),
        ..caster(Vocation::Knight)
    }
}

pub(super) fn at_ms(millis: u64) -> SemanticTimeMicros {
    SemanticTimeMicros::from_micros(millis * 1000)
}

pub(super) fn lowest(minimum: i64, _: i64) -> i64 {
    minimum
}

pub(super) fn target(creature: u64, master: Option<u64>) -> CastTarget {
    CastTarget {
        caster: CASTER,
        creature,
        actor: format!("actor:{creature}"),
        master,
    }
}

pub(super) fn occurrence(id: &str) -> AbilityOccurrence {
    let revisions = RevisionSet::new(
        "ruleset:r1",
        "content:part-d",
        "world:r1",
        "formula:s5",
        "simulation:r1",
    )
    .expect("revisions");
    AbilityOccurrence::new(id, revisions).expect("occurrence")
}

/// D.3.4 test 1: Ultimate Healing Rune on self heals within the formula and removes paralysis.
#[test]
fn ultimate_healing_rune_on_self_heals_and_removes_paralysis() {
    let rune = ultimate_healing_rune();
    assert_eq!(rune.allowed_targets, AllowedTargets::SelfOrOwnSummons);
    assert!(matches!(rune.carrier, Carrier::Rune { item: 3160, .. }));
    let resolution = resolve_targeted_cast(
        &rune,
        &caster(Vocation::Druid),
        &Cooldowns::default(),
        at_ms(0),
        target(CASTER, None),
        &mut lowest,
    )
    .expect("cast on self");
    // level 100 / 5 + 50 * 7.3 + 42 = 427.
    assert_eq!(
        resolution.effects,
        vec![
            ResolvedEffect::Heal { magnitude: 427 },
            ResolvedEffect::RemoveCondition {
                condition: "paralyze".into()
            }
        ]
    );
    assert_eq!(
        resolution.cooldowns.group_ready_at("healing"),
        Some(at_ms(1000))
    );
    let plan = effect_plan(
        &rune,
        &resolution,
        "actor:1",
        Some("actor:1"),
        occurrence("cast:uhr-self"),
        "channel:test",
    )
    .expect("plan");
    let mut engine = AbilityEngine::new();
    assert!(
        engine
            .commit(plan.effects.expect("heal"))
            .expect("commit")
            .applied()
    );
    assert_eq!(engine.fixture_health("actor:1"), Some(427));
}

/// D.3.4 test 2: on another player the cast is refused and nothing is spent (the rune charge is
/// kept, since no cast resolution exists to consume it).
#[test]
fn ultimate_healing_rune_on_another_player_is_refused() {
    let rune = ultimate_healing_rune();
    for master in [None, Some(OTHER_PLAYER)] {
        assert_eq!(
            resolve_targeted_cast(
                &rune,
                &caster(Vocation::Druid),
                &Cooldowns::default(),
                at_ms(0),
                target(OTHER_PLAYER + 10, master),
                &mut lowest,
            ),
            Err(CastRejection::TargetNotAllowed)
        );
    }
    // Without the target facts a targeted cast cannot be checked, so it is refused.
    assert_eq!(
        resolve_cast(
            &rune,
            &caster(Vocation::Druid),
            &Cooldowns::default(),
            at_ms(0),
            true,
            &mut lowest
        ),
        Err(CastRejection::TargetFactsRequired)
    );
}

/// D.3.4 test 3: the caster's own summon is healed (Q7, the F reading); `self_only` (the Canary
/// reading) refuses it.
#[test]
fn ultimate_healing_rune_heals_the_casters_own_summon() {
    let rune = ultimate_healing_rune();
    let resolution = resolve_targeted_cast(
        &rune,
        &caster(Vocation::Druid),
        &Cooldowns::default(),
        at_ms(0),
        target(SUMMON, Some(CASTER)),
        &mut lowest,
    )
    .expect("cast on own summon");
    let plan = effect_plan(
        &rune,
        &resolution,
        "actor:1",
        Some("actor:3"),
        occurrence("cast:uhr-summon"),
        "channel:test",
    )
    .expect("plan");
    let heal = plan.effects.expect("heal");
    let mut engine = AbilityEngine::new();
    engine.commit(heal).expect("commit");
    assert_eq!(engine.fixture_health("actor:3"), Some(427));
    assert_eq!(engine.fixture_health("actor:1"), None);

    let (spell, dependencies) = ultimate_healing_rune_bundle("self_only");
    let self_only = spell_from_bundle(&spell, &dependencies).expect("self_only admitted");
    assert_eq!(
        resolve_targeted_cast(
            &self_only,
            &caster(Vocation::Druid),
            &Cooldowns::default(),
            at_ms(0),
            target(SUMMON, Some(CASTER)),
            &mut lowest,
        ),
        Err(CastRejection::TargetNotAllowed)
    );
}

/// D.3.4 test 4: used on a tile without a creature the rune has no target and is refused.
#[test]
fn ultimate_healing_rune_without_a_creature_is_refused() {
    assert_eq!(
        resolve_cast(
            &ultimate_healing_rune(),
            &caster(Vocation::Druid),
            &Cooldowns::default(),
            at_ms(0),
            false,
            &mut lowest
        ),
        Err(CastRejection::TargetRequired)
    );
}

/// D.3.4 test 5: the monk and the exalted monk cannot use Ultimate Healing Rune (Q8, F).
#[test]
fn ultimate_healing_rune_refuses_both_monk_vocations() {
    let rune = ultimate_healing_rune();
    for vocation in [Vocation::Monk, Vocation::ExaltedMonk] {
        assert_eq!(
            resolve_targeted_cast(
                &rune,
                &caster(vocation),
                &Cooldowns::default(),
                at_ms(0),
                target(CASTER, None),
                &mut lowest,
            ),
            Err(CastRejection::VocationCannotUse)
        );
    }
}

/// The plan of a cast checked against one target applies to that target only: a rune checked on
/// the caster cannot heal another player, and a rune cast without the target facts cannot plan
/// on a target at all.
#[test]
fn a_checked_cast_plans_only_on_its_checked_target() {
    let rune = ultimate_healing_rune();
    let resolution = resolve_targeted_cast(
        &rune,
        &caster(Vocation::Druid),
        &Cooldowns::default(),
        at_ms(0),
        target(CASTER, None),
        &mut lowest,
    )
    .expect("cast on self");
    let checked = resolution.target.as_ref().expect("checked target");
    assert_eq!((checked.creature(), checked.actor()), (CASTER, "actor:1"));
    for other in [Some("actor:2"), None] {
        assert_eq!(
            effect_plan(
                &rune,
                &resolution,
                "actor:1",
                other,
                occurrence("cast:uhr-mismatch"),
                "channel:test",
            ),
            Err(CastPlanError::TargetMismatch)
        );
    }

    // A spell without `allowed_targets` checked on a target keeps that target too.
    let (spell, dependencies) = ultimate_healing_rune_bundle("any");
    let any = spell_from_bundle(&spell, &dependencies).expect("any admitted");
    let resolution = resolve_targeted_cast(
        &any,
        &caster(Vocation::Druid),
        &Cooldowns::default(),
        at_ms(0),
        target(OTHER_PLAYER, None),
        &mut lowest,
    )
    .expect("cast on another player");
    assert_eq!(
        effect_plan(
            &any,
            &resolution,
            "actor:1",
            Some("actor:3"),
            occurrence("cast:any-mismatch"),
            "channel:test",
        ),
        Err(CastPlanError::TargetMismatch)
    );
    assert!(
        effect_plan(
            &any,
            &resolution,
            "actor:1",
            Some("actor:2"),
            occurrence("cast:any-match"),
            "channel:test",
        )
        .is_ok()
    );
}

/// The target rule comes after the cast checks: a cooling rune reports the cooldown first.
#[test]
fn the_target_rule_follows_the_cast_checks() {
    let rune = ultimate_healing_rune();
    let cast = resolve_targeted_cast(
        &rune,
        &caster(Vocation::Druid),
        &Cooldowns::default(),
        at_ms(0),
        target(CASTER, None),
        &mut lowest,
    )
    .expect("cast");
    assert!(matches!(
        resolve_targeted_cast(
            &rune,
            &caster(Vocation::Druid),
            &cast.cooldowns,
            at_ms(500),
            target(OTHER_PLAYER, None),
            &mut lowest,
        ),
        Err(CastRejection::GroupCooling { .. })
    ));
}

#[test]
fn allowed_targets_values_are_read_and_checked() {
    for (key, expected) in [
        ("any", AllowedTargets::Any),
        ("self_only", AllowedTargets::SelfOnly),
        ("self_or_own_summons", AllowedTargets::SelfOrOwnSummons),
    ] {
        let (spell, dependencies) = ultimate_healing_rune_bundle(key);
        let spell = spell_from_bundle(&spell, &dependencies).expect(key);
        assert_eq!(spell.allowed_targets, expected);
    }
    let (spell, dependencies) = ultimate_healing_rune_bundle("own_party");
    assert!(spell_from_bundle(&spell, &dependencies).is_err());
    // A chain picks its own creatures; the single-target rule cannot apply to it.
    let (spell, mut dependencies) = ultimate_healing_rune_bundle("self_only");
    dependencies["abilities"][0]["chain"] =
        json!({ "max_targets": 2, "range_tiles": 3, "backtracking": false });
    assert!(spell_from_bundle(&spell, &dependencies).is_err());
    dependencies["abilities"][0]
        .as_object_mut()
        .expect("ability")
        .remove("chain");
    assert!(spell_from_bundle(&spell, &dependencies).is_ok());
}

/// D.4.4 test 1: Shield Bash without a shield is refused; mana and cooldown are unchanged.
#[test]
fn shield_bash_without_a_shield_is_refused() {
    let bash = shield_bash();
    assert!(bash.needs_shield);
    assert_eq!(
        resolve_cast(
            &bash,
            &caster(Vocation::Knight),
            &Cooldowns::default(),
            at_ms(0),
            true,
            &mut lowest
        ),
        Err(CastRejection::ShieldRequired)
    );
    // The engine checks still come first.
    assert_eq!(
        resolve_cast(
            &bash,
            &CasterState {
                mana: 29,
                ..caster(Vocation::Knight)
            },
            &Cooldowns::default(),
            at_ms(0),
            true,
            &mut lowest
        ),
        Err(CastRejection::NotEnoughMana { required: 30 })
    );
    let cast = resolve_cast(
        &bash,
        &knight_with_shield(20),
        &Cooldowns::default(),
        at_ms(0),
        true,
        &mut lowest,
    )
    .expect("cast with a shield");
    assert_eq!(cast.mana_spent, 30);
    assert_eq!(cast.cooldowns.spell_ready_at(&bash.key), Some(at_ms(4000)));
}

/// D.4.4 test 2: defense 20, shielding 100, base power 55, flat 0: 99..121, average 110 before
/// armor.
#[test]
fn shield_bash_damage_follows_the_shield_formula() {
    let mut drawn = Vec::new();
    let cast = resolve_cast(
        &shield_bash(),
        &knight_with_shield(20),
        &Cooldowns::default(),
        at_ms(0),
        true,
        &mut |minimum, maximum| {
            drawn.push((minimum, maximum));
            minimum
        },
    )
    .expect("cast");
    assert_eq!(drawn, vec![(99, 121)]);
    assert_eq!((99 + 121) / 2, 110);
    assert_eq!(
        cast.effects,
        vec![ResolvedEffect::Damage {
            damage_type: "physical".into(),
            magnitude: 99
        }]
    );
}

/// A formula may read `shield_defense` only in a spell that needs a shield; `needs_shield` is
/// false when absent.
#[test]
fn shield_defense_needs_a_shield_spell() {
    for needs_shield in [None, Some(false)] {
        let (spell, dependencies) = shield_bash_bundle(needs_shield);
        let error = spell_from_bundle(&spell, &dependencies).expect_err("admitted");
        assert!(error.to_string().contains("shield_defense"), "{error}");
    }
    let (spell, dependencies): (Value, Value) = (
        serde_json::from_str(STRIKE).expect("strike"),
        serde_json::from_str(STRIKE_DEPENDENCIES).expect("strike dependencies"),
    );
    assert!(
        !spell_from_bundle(&spell, &dependencies)
            .expect("energy strike")
            .needs_shield
    );
}

/// D.4: `needs_weapon` refuses a caster without a melee weapon, after the vocation check and
/// before the premium check (Canary/Crystal `Spell::playerSpellCheck`).
#[test]
fn needs_weapon_refuses_a_caster_without_a_melee_weapon() {
    let (mut spell, dependencies) = shield_bash_bundle(Some(true));
    spell["spell"]["needs_weapon"] = json!(true);
    spell["spell"]["requirements"]["premium"] = json!(true);
    let spell = spell_from_bundle(&spell, &dependencies).expect("admitted");
    assert!(spell.needs_weapon);
    let knight = knight_with_shield(20);
    assert_eq!(
        resolve_cast(
            &spell,
            &knight,
            &Cooldowns::default(),
            at_ms(0),
            true,
            &mut lowest
        ),
        Err(CastRejection::WeaponRequired)
    );
    assert_eq!(
        resolve_cast(
            &spell,
            &CasterState {
                vocation: Vocation::Druid,
                ..knight.clone()
            },
            &Cooldowns::default(),
            at_ms(0),
            true,
            &mut lowest
        ),
        Err(CastRejection::VocationCannotUse)
    );
    let armed = CasterState {
        melee_weapon: true,
        ..knight
    };
    assert_eq!(
        resolve_cast(
            &spell,
            &armed,
            &Cooldowns::default(),
            at_ms(0),
            true,
            &mut lowest
        ),
        Err(CastRejection::PremiumRequired)
    );
    let premium = CasterState {
        premium: true,
        ..armed
    };
    assert!(
        resolve_cast(
            &spell,
            &premium,
            &Cooldowns::default(),
            at_ms(0),
            true,
            &mut lowest
        )
        .is_ok()
    );
}

/// S7/D13: the Part D `native_behavior` keys without an implementation stay rejected, as do the
/// family names.
#[test]
fn part_d_keys_without_a_runtime_stay_rejected() {
    for key in [
        "owned_field_buff",
        "delayed_strike",
        "tile_item_operation",
        "monster_ai_override",
        "delayed_or_repeated",
        "target_position",
        "target_default",
        "equipment_dependent",
        "extra_presentation_only",
        "other",
    ] {
        let (mut spell, dependencies) = shield_bash_bundle(Some(true));
        spell["spell"]["execution"] =
            json!({ "native_behavior": { "key": key, "parameters": {} } });
        let error = spell_from_bundle(&spell, &dependencies).expect_err(key);
        assert!(!error.to_string().is_empty(), "{key}: {error}");
    }
}
