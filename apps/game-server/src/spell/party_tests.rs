//! Engine tests of part C (`OTERYN_SPELL_NATIVE_BEHAVIOURS_CANDIDATE_V1.md`): the `party_buff`
//! tests of C.3, the Cancel Magic Shield tests of C.5, and the rejection of the Part C keys that
//! have no implementation yet.

#![allow(clippy::expect_used)]

use std::collections::BTreeSet;

use oteryn_simulation_determinism::SemanticTimeMicros;
use serde_json::{Value, json};

use super::authoring::spell_from_bundle;
use super::chain::{ChainCreature, TilePosition};
use super::party::{PartyWorld, SoloParty};
use super::plan::{CastPlanError, effect_plan, party_plans};
use super::*;
use crate::ability::{AbilityOccurrence, RevisionSet};

const SPELL: &str = include_str!(
    "../../../../tools/content-schema/spell-authoring/samples/starter-bundles/instant-cure_poison/spell.json"
);
const DEPENDENCIES: &str = include_str!(
    "../../../../tools/content-schema/spell-authoring/samples/starter-bundles/instant-cure_poison/dependencies.json"
);

const CASTER: u64 = 1;
const REGENERATION: &str = "candidate:spell/heal_party/effect-regeneration";

/// The radius-3 circle of C.3 step 2 (rows of 3, 5, 7, 7, 7, 5 and 3 tiles).
fn circle() -> Value {
    json!([
        "..xxx..", ".xxxxx.", "xxxxxxx", "xxxCxxx", "xxxxxxx", ".xxxxx.", "..xxx.."
    ])
}

fn scaled(base: u32) -> Value {
    json!({ "mode": "scaled", "base": base, "falloff": 0.9, "rounding": "up" })
}

fn parameters(mana: Value) -> Value {
    json!({
        "area": circle(),
        "same_floor": true,
        "min_affected": 2,
        "requires_party": true,
        "mana": mana,
        "effect": { "family": "Effect", "key": REGENERATION, "revision": "test" }
    })
}

/// Heal Party as the C.3 authoring shape: `party_buff`, `costs.mana` 0, own and support cooldowns
/// 2 s; each member gets regeneration 20 HP every 2 s for 120 s.
fn heal_party_bundle(parameters: Value, cooldown_ms: u32) -> (Value, Value) {
    let mut spell: Value = serde_json::from_str(SPELL).expect("spell");
    let body = &mut spell["spell"];
    body["identity"]["key"] = json!("candidate:spell/heal_party");
    body["name"] = json!("Heal Party");
    body["words"] = json!("utura mas sio");
    body["requirements"] = json!({
        "vocations": ["druid", "elder_druid"],
        "level": 32,
        "premium": true,
        "learning_required": false
    });
    body["costs"] = json!({ "mana": 0, "soul": 0 });
    body["cooldown_ms"] = json!(cooldown_ms);
    body["groups"] = json!([{ "group": "support", "cooldown_ms": 2000 }]);
    body["execution"] =
        json!({ "native_behavior": { "key": "party_buff", "parameters": parameters } });
    let dependencies = json!({
        "abilities": [],
        "effects": [{
            "identity": { "key": REGENERATION, "revision": "test" },
            "operation": "condition",
            "duration_ms": 120000,
            "condition": {
                "type": "regeneration",
                "lifetime": "fixed_duration",
                "regeneration": { "health_gain": 20, "health_interval_ms": 2000 },
                "buff_spell": true
            }
        }],
        "formulas": []
    });
    (spell, dependencies)
}

fn heal_party() -> SpellDefinition {
    let (spell, dependencies) = heal_party_bundle(parameters(scaled(120)), 2000);
    spell_from_bundle(&spell, &dependencies).expect("Heal Party admitted")
}

fn druid(mana: u32) -> CasterState {
    CasterState {
        vocation: Vocation::ElderDruid,
        level: 100,
        magic_level: 50,
        premium: true,
        mana,
        max_mana: 5000,
        soul: 100,
        learned: BTreeSet::new(),
        attack_skill: 10,
        attack_value: 7,
        attack_factor: 1.0,
        shielding_skill: 10,
    }
}

fn at_ms(millis: u64) -> SemanticTimeMicros {
    SemanticTimeMicros::from_micros(millis * 1000)
}

fn lowest(minimum: i64, _: i64) -> i64 {
    minimum
}

fn tile(x: i32, y: i32, floor: i16) -> TilePosition {
    TilePosition { x, y, floor }
}

fn member(id: u64, position: TilePosition) -> ChainCreature {
    ChainCreature {
        id,
        actor: format!("actor:{id}"),
        position,
    }
}

/// A party whose caster stands at (0, 0) on floor 7.
struct Party {
    caster: ChainCreature,
    members: Vec<ChainCreature>,
}

impl Party {
    fn new() -> Self {
        let caster = member(CASTER, tile(0, 0, 7));
        Self {
            caster: caster.clone(),
            members: vec![caster],
        }
    }

    fn with(mut self, id: u64, position: TilePosition) -> Self {
        self.members.push(member(id, position));
        self
    }

    /// `count` further members next to the caster.
    fn around(self, count: u64) -> Self {
        (0..count).fold(self, |party, index| {
            let x = i32::try_from(index % 3).expect("x") - 1;
            let y = i32::try_from(index / 3).expect("y") - 1;
            party.with(10 + index, tile(x, y, 7))
        })
    }
}

impl PartyWorld for Party {
    fn caster(&self) -> &ChainCreature {
        &self.caster
    }

    fn party(&self) -> Option<&[ChainCreature]> {
        Some(&self.members)
    }
}

fn cast(
    spell: &SpellDefinition,
    mana: u32,
    cooldowns: &Cooldowns,
    now: SemanticTimeMicros,
    world: &dyn PartyWorld,
) -> Result<CastResolution, CastRejection> {
    resolve_party_cast(spell, &druid(mana), cooldowns, now, world, &mut lowest)
}

fn affected(resolution: &CastResolution) -> Vec<u64> {
    resolution
        .party
        .iter()
        .map(|member| member.creature)
        .collect()
}

fn occurrence(id: &str) -> AbilityOccurrence {
    let revisions = RevisionSet::new(
        "ruleset:r1",
        "content:party",
        "world:r1",
        "formula:s5",
        "simulation:r1",
    )
    .expect("revisions");
    AbilityOccurrence::new(id, revisions).expect("occurrence")
}

/// C.3 test 1: without a party, or alone in the area, the cast fails and spends nothing.
#[test]
fn a_caster_without_party_members_in_range_is_refused() {
    let spell = heal_party();
    let none = Cooldowns::default();
    let solo = SoloParty(member(CASTER, tile(0, 0, 7)));
    assert_eq!(
        cast(&spell, 5000, &none, at_ms(0), &solo),
        Err(CastRejection::NoPartyMembers)
    );
    let alone = Party::new().with(2, tile(10, 0, 7));
    assert_eq!(
        cast(&spell, 5000, &none, at_ms(0), &alone),
        Err(CastRejection::NoPartyMembers)
    );
    // A party the world lists without the caster is not the caster's party.
    let foreign = Party {
        caster: member(CASTER, tile(0, 0, 7)),
        members: vec![member(2, tile(1, 0, 7)), member(3, tile(0, 1, 7))],
    };
    assert_eq!(
        cast(&spell, 5000, &none, at_ms(0), &foreign),
        Err(CastRejection::NoPartyMembers)
    );
    // The ordinary cast checks come first.
    let mut cooling = none.clone();
    cooling.groups.insert("support".into(), at_ms(1));
    assert!(matches!(
        cast(&spell, 5000, &cooling, at_ms(0), &solo),
        Err(CastRejection::GroupCooling { .. })
    ));
}

/// C.3 test 2: three members in the area cost 292 mana (291.6 rounded up); each gets the
/// regeneration Effect, and the cooldowns start.
#[test]
fn heal_party_with_three_members_costs_292_and_buffs_each() {
    let spell = heal_party();
    let party = Party::new().with(2, tile(1, 1, 7)).with(3, tile(-2, 2, 7));
    let resolution = cast(&spell, 5000, &Cooldowns::default(), at_ms(0), &party).expect("cast");
    assert_eq!(resolution.mana_spent, 292);
    assert_eq!(affected(&resolution), vec![1, 2, 3]);
    assert!(resolution.effects.is_empty() && resolution.chain.is_empty());
    for member in &resolution.party {
        assert_eq!(
            member.effects,
            vec![ResolvedEffect::Unresolved {
                operation: "condition".into(),
                effect: REGENERATION.into()
            }]
        );
    }
    assert_eq!(
        resolution.cooldowns.spell_ready_at(&spell.key),
        Some(at_ms(2000))
    );
    assert_eq!(
        resolution.cooldowns.group_ready_at("support"),
        Some(at_ms(2000))
    );

    let plans = party_plans(
        &resolution,
        "actor:1",
        &occurrence("cast:1"),
        "channel:test",
    )
    .expect("party plans");
    assert_eq!(
        plans
            .iter()
            .map(|plan| plan.member.as_str())
            .collect::<Vec<_>>(),
        vec!["actor:1", "actor:2", "actor:3"]
    );
    assert!(plans.iter().all(|plan| plan.plan.effects.is_none()));
    assert_eq!(
        effect_plan(
            &spell,
            &resolution,
            "actor:1",
            None,
            occurrence("cast:2"),
            "channel:test"
        ),
        Err(CastPlanError::WrongPlanKind)
    );
}

/// C.3 test 3 and the F1182737 table: X = 2..10 members cost 216, 292, 350, 394, 426, 447, 460,
/// 465, 465; ten members pay less per member than two.
#[test]
fn scaled_mana_follows_the_formula_rounded_up() {
    let spell = heal_party();
    let expected = [216, 292, 350, 394, 426, 447, 460, 465, 465];
    let mut per_member = Vec::new();
    for (further, cost) in (1..=9).zip(expected) {
        let party = Party::new().around(further);
        let resolution = cast(&spell, 5000, &Cooldowns::default(), at_ms(0), &party).expect("cast");
        assert_eq!(resolution.mana_spent, cost, "{} members", further + 1);
        assert_eq!(
            resolution.party.len(),
            usize::try_from(further + 1).expect("len")
        );
        per_member.push(f64::from(resolution.mana_spent) / (further as f64 + 1.0));
    }
    // 465 / 10 = 46.5 per member, below 216 / 2 = 108.
    assert_eq!(per_member.first(), Some(&108.0));
    assert_eq!(per_member.last(), Some(&46.5));
}

/// C.3 test 4: the radius-3 circle on the caster's floor.
#[test]
fn the_area_is_the_radius_three_circle_on_the_caster_floor() {
    let spell = heal_party();
    let party = Party::new()
        .with(2, tile(3, 1, 7))
        .with(3, tile(3, 2, 7))
        .with(4, tile(3, 3, 7))
        .with(5, tile(0, 0, 6))
        .with(6, tile(-1, -3, 7))
        .with(7, tile(0, 4, 7));
    let resolution = cast(&spell, 5000, &Cooldowns::default(), at_ms(0), &party).expect("cast");
    assert_eq!(affected(&resolution), vec![1, 2, 6]);
    // Three affected members: members outside the area are not counted.
    assert_eq!(resolution.mana_spent, 292);
    assert!(matches!(&spell.execution, Execution::PartyBuff(buff) if buff.area.len() == 37));
}

/// C.3 test 5, the engine part: a recast after the 2 s cooldown gives each member the same
/// regeneration Effect again; replacing the running condition is the condition owner's.
#[test]
fn a_recast_after_the_cooldown_gives_the_same_effect_again() {
    let spell = heal_party();
    let party = Party::new().with(2, tile(1, 0, 7));
    let first = cast(&spell, 5000, &Cooldowns::default(), at_ms(0), &party).expect("cast");
    assert!(matches!(
        cast(&spell, 5000, &first.cooldowns, at_ms(1999), &party),
        Err(CastRejection::GroupCooling { .. })
    ));
    let again = cast(&spell, 5000, &first.cooldowns, at_ms(60_000), &party).expect("recast");
    assert_eq!(again.party, first.party);
}

/// C.3 test 6: mana just below the computed cost refuses the cast; no member is affected.
#[test]
fn mana_below_the_computed_cost_is_refused() {
    let spell = heal_party();
    let party = Party::new().with(2, tile(1, 1, 7)).with(3, tile(-2, 2, 7));
    assert_eq!(
        cast(&spell, 291, &Cooldowns::default(), at_ms(0), &party),
        Err(CastRejection::NotEnoughMana { required: 292 })
    );
    assert!(cast(&spell, 292, &Cooldowns::default(), at_ms(0), &party).is_ok());
}

/// C.3 test 7 with the fixed mode: 75 mana for 2 or 6 members, own cooldown 300 s.
#[test]
fn fixed_mana_does_not_scale_and_the_own_cooldown_holds() {
    let (spell, dependencies) =
        heal_party_bundle(parameters(json!({ "mode": "fixed", "base": 75 })), 300_000);
    let spell = spell_from_bundle(&spell, &dependencies).expect("fixed party buff");
    for further in [1, 5] {
        let party = Party::new().around(further);
        let resolution = cast(&spell, 5000, &Cooldowns::default(), at_ms(0), &party).expect("cast");
        assert_eq!(resolution.mana_spent, 75);
        let recast = cast(&spell, 5000, &resolution.cooldowns, at_ms(299_999), &party);
        assert_eq!(
            recast,
            Err(CastRejection::SpellCooling {
                ready_at: at_ms(300_000)
            })
        );
    }
}

/// A party buff resolved without party facts, or as a chain cast, fails closed.
#[test]
fn a_party_buff_needs_the_party_facts() {
    let spell = heal_party();
    assert_eq!(
        resolve_cast(
            &spell,
            &druid(5000),
            &Cooldowns::default(),
            at_ms(0),
            false,
            &mut lowest
        ),
        Err(CastRejection::PartyWorldRequired)
    );
}

#[test]
fn the_reader_admits_only_the_accepted_party_buff_values() {
    let reject = |mutate: &dyn Fn(&mut Value, &mut Value)| {
        let (mut spell, mut dependencies) = heal_party_bundle(parameters(scaled(120)), 2000);
        mutate(&mut spell, &mut dependencies);
        spell_from_bundle(&spell, &dependencies).err()
    };
    let parameter = |name: &'static str, value: Value| {
        move |spell: &mut Value, _: &mut Value| {
            spell["spell"]["execution"]["native_behavior"]["parameters"][name] = value.clone();
        }
    };
    assert!(reject(&|_, _| {}).is_none());
    assert!(reject(&|spell, _| spell["spell"]["costs"]["mana"] = json!(120)).is_some());
    assert!(reject(&parameter("same_floor", json!(false))).is_some());
    assert!(reject(&parameter("requires_party", json!(false))).is_some());
    assert!(reject(&parameter("min_affected", json!(0))).is_some());
    assert!(reject(&parameter("area", json!(["xxx", "xcx", "xx"]))).is_some());
    assert!(reject(&parameter("area", json!(["xCx", "xCx"]))).is_some());
    assert!(reject(&parameter("area", json!(["xxx"]))).is_some());
    assert!(
        reject(&parameter(
            "mana",
            scaled(120).as_object().map_or(json!(null), |m| {
                let mut m = m.clone();
                m.insert("rounding".into(), json!("down"));
                Value::Object(m)
            })
        ))
        .is_some()
    );
    assert!(
        reject(&parameter(
            "mana",
            json!({ "mode": "scaled", "base": 120, "falloff": 0.905, "rounding": "up" })
        ))
        .is_some()
    );
    assert!(
        reject(&parameter(
            "mana",
            json!({ "mode": "fixed", "base": 75, "falloff": 0.9 })
        ))
        .is_some()
    );
    assert!(reject(&parameter("spell_range", json!(4))).is_some());
    assert!(
        reject(&|_, dependencies| dependencies["effects"] = json!([])).is_some(),
        "the member Effect must exist"
    );
}

/// S7/D13: the Part C keys without an implementation stay rejected, as does any unknown key.
#[test]
fn part_c_keys_without_a_runtime_stay_rejected() {
    for key in [
        "familiar_summon",
        "acquire_summon",
        "stance_toggle",
        "familiar",
        "summons_share_condition",
        "stance",
        "conditional_self_state",
        "party",
        "unresolved",
    ] {
        let (mut spell, dependencies) = heal_party_bundle(parameters(scaled(120)), 2000);
        spell["spell"]["execution"]["native_behavior"]["key"] = json!(key);
        assert!(
            spell_from_bundle(&spell, &dependencies).is_err(),
            "{key} must be rejected"
        );
    }
}

/// C.5 tests 1 and 2: Cancel Magic Shield is a plain Ability `remove_condition` magic_shield on
/// the caster (no native key). The cast succeeds with or without a shield (Canary; Q23 open).
#[test]
fn cancel_magic_shield_removes_the_shield_condition() {
    let mut spell: Value = serde_json::from_str(SPELL).expect("spell");
    let mut dependencies: Value = serde_json::from_str(DEPENDENCIES).expect("dependencies");
    let body = &mut spell["spell"];
    body["name"] = json!("Cancel Magic Shield");
    body["words"] = json!("exana vita");
    body["requirements"] = json!({
        "vocations": ["druid", "elder_druid", "sorcerer", "master_sorcerer"],
        "level": 14,
        "premium": true,
        "learning_required": false
    });
    body["costs"] = json!({ "mana": 50, "soul": 0 });
    body["cooldown_ms"] = json!(2000);
    body["groups"] = json!([{ "group": "support", "cooldown_ms": 2000 }]);
    dependencies["effects"][0]["removed_condition"] = json!("magic_shield");
    let spell = spell_from_bundle(&spell, &dependencies).expect("Cancel Magic Shield");
    let resolution = resolve_cast(
        &spell,
        &druid(50),
        &Cooldowns::default(),
        at_ms(0),
        false,
        &mut lowest,
    )
    .expect("cast");
    assert_eq!(resolution.mana_spent, 50);
    assert_eq!(
        resolution.effects,
        vec![ResolvedEffect::RemoveCondition {
            condition: "magic_shield".into()
        }]
    );
    assert_eq!(
        resolution.cooldowns.spell_ready_at(&spell.key),
        Some(at_ms(2000))
    );
    assert_eq!(
        resolve_cast(
            &spell,
            &druid(49),
            &Cooldowns::default(),
            at_ms(0),
            false,
            &mut lowest
        ),
        Err(CastRejection::NotEnoughMana { required: 50 })
    );
}
