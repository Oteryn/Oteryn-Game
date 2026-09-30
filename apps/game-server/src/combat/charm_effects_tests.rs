//! CHARM-4 charm effect engine tests: every catalogue charm and effect type, the fail-closed
//! matrix, deterministic replayable trigger rolls, and one negative per invariant.
#![allow(clippy::expect_used)]

use std::collections::BTreeSet;
use std::mem::discriminant;

use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};

use super::*;

const CHARACTER: [u8; 16] = [0x11; 16];
const OTHER_CHARACTER: [u8; 16] = [0x22; 16];
const RACE: &str = "oteryn:creature.rotworm";
const OTHER_RACE: &str = "oteryn:creature.cave_rat";

fn pct(hundredths: u32) -> CharmPercent {
    CharmPercent::from_hundredths(hundredths).expect("valid percent")
}

fn stages(values: [u32; 3]) -> [CharmPercent; 3] {
    values.map(pct)
}

fn define(
    key: &str,
    category: CharmCategory,
    values: [u32; 3],
    effect: CharmEffect,
) -> CharmDefinition {
    CharmDefinition::new(key, category, effect.stage_value(), stages(values), effect)
        .expect("valid definition")
}

/// Elemental procs: element damage, resistances apply, not reduced by armor (#1293 d89f30f3).
const fn elemental(element: CharmElement) -> CharmDamageKind {
    CharmDamageKind {
        element,
        ignores_resistances: false,
        reduced_by_armor: false,
    }
}
/// Overpower, Overflux: shown as physical, ignore resistances, not reduced by armor.
const RESOURCE_KIND: CharmDamageKind = CharmDamageKind {
    element: CharmElement::Physical,
    ignores_resistances: true,
    reduced_by_armor: false,
};
/// Carnage: physical, resistances apply (wiki), reduced by armor.
const CARNAGE_KIND: CharmDamageKind = CharmDamageKind {
    element: CharmElement::Physical,
    ignores_resistances: false,
    reduced_by_armor: true,
};
/// Parry: shown as physical, ignores resistances, reduced by armor.
const PARRY_KIND: CharmDamageKind = CharmDamageKind {
    element: CharmElement::Physical,
    ignores_resistances: true,
    reduced_by_armor: true,
};

fn proc_damage(element: CharmElement) -> CharmEffect {
    CharmEffect::AttackProcDamage {
        damage: elemental(element),
        percent_of_creature_max_health: pct(500),
        damage_cap_level_multiplier: 2,
    }
}

fn resource_damage(resource: CharmResource, percent: u32) -> CharmEffect {
    CharmEffect::AttackProcResourceDamage {
        damage: RESOURCE_KIND,
        resource,
        percent_of_own_maximum: pct(percent),
        damage_cap_percent_of_creature_max_health: pct(800),
    }
}

/// The 25 charms of the candidate catalogue (#1293 `samples/charms-candidate.json` at
/// d89f30f3, TibiaWiki values per owner answer 5a), percent values in hundredths.
fn catalogue_charms() -> Vec<CharmDefinition> {
    use CharmCategory::{Major, Minor};
    use CharmElement::{Death, Earth, Energy, Fire, Holy, Ice, Physical};
    let major_proc = [500, 1000, 1100];
    let minor = [600, 900, 1200];
    vec![
        define(
            "oteryn:charm.adrenaline_burst",
            Minor,
            minor,
            CharmEffect::HasteAfterHit {
                duration_ms: 10_000,
            },
        ),
        define(
            "oteryn:charm.bless",
            Minor,
            minor,
            CharmEffect::DeathLossReduction,
        ),
        define(
            "oteryn:charm.carnage",
            Major,
            [1000, 2000, 2200],
            CharmEffect::KillAreaDamage {
                damage: CARNAGE_KIND,
                percent_of_creature_max_health: pct(1500),
                damage_cap_level_multiplier: 6,
            },
        ),
        define(
            "oteryn:charm.cleanse",
            Minor,
            minor,
            CharmEffect::CleanseAfterHit,
        ),
        define(
            "oteryn:charm.cripple",
            Minor,
            minor,
            CharmEffect::ParalyseCreatureOnAttack {
                duration_ms: 10_000,
            },
        ),
        define("oteryn:charm.curse", Major, major_proc, proc_damage(Death)),
        define(
            "oteryn:charm.divine_wrath",
            Major,
            major_proc,
            proc_damage(Holy),
        ),
        define(
            "oteryn:charm.dodge",
            Major,
            major_proc,
            CharmEffect::DodgeAttack,
        ),
        define("oteryn:charm.enflame", Major, major_proc, proc_damage(Fire)),
        define(
            "oteryn:charm.fatal_hold",
            Minor,
            [3000, 4500, 6000],
            CharmEffect::PreventCreatureFlee {
                duration_ms: 30_000,
            },
        ),
        define("oteryn:charm.freeze", Major, major_proc, proc_damage(Ice)),
        define(
            "oteryn:charm.gut",
            Minor,
            minor,
            CharmEffect::CreatureProductBonus,
        ),
        define(
            "oteryn:charm.low_blow",
            Major,
            [400, 800, 900],
            CharmEffect::CriticalHitChance,
        ),
        define(
            "oteryn:charm.numb",
            Minor,
            minor,
            CharmEffect::ParalyseCreatureAfterItsAttack {
                duration_ms: 10_000,
            },
        ),
        define(
            "oteryn:charm.overflux",
            Major,
            major_proc,
            resource_damage(CharmResource::Mana, 250),
        ),
        define(
            "oteryn:charm.overpower",
            Major,
            major_proc,
            resource_damage(CharmResource::Health, 500),
        ),
        define(
            "oteryn:charm.parry",
            Major,
            major_proc,
            CharmEffect::ReflectDamageTaken { damage: PARRY_KIND },
        ),
        define("oteryn:charm.poison", Major, major_proc, proc_damage(Earth)),
        define(
            "oteryn:charm.savage_blow",
            Major,
            [2000, 4000, 4400],
            CharmEffect::CriticalExtraDamage,
        ),
        define(
            "oteryn:charm.scavenge",
            Minor,
            [6000, 9000, 12_000],
            CharmEffect::SkinningChanceBonus,
        ),
        define(
            "oteryn:charm.vampiric_embrace",
            Minor,
            [160, 240, 320],
            CharmEffect::LifeLeech,
        ),
        define(
            "oteryn:charm.void_inversion",
            Minor,
            [2000, 3000, 4000],
            CharmEffect::ManaDrainInversion,
        ),
        define(
            "oteryn:charm.voids_call",
            Minor,
            [80, 120, 160],
            CharmEffect::ManaLeech,
        ),
        define(
            "oteryn:charm.wound",
            Major,
            major_proc,
            proc_damage(Physical),
        ),
        define("oteryn:charm.zap", Major, major_proc, proc_damage(Energy)),
    ]
}

/// The same charm with a 100% stage-3 trigger chance, so a test exercises the effect itself.
fn always_triggering(definition: &CharmDefinition) -> CharmDefinition {
    let effect = definition.effect();
    let values = match effect.stage_value() {
        CharmStageValue::TriggerChancePercent => [1, 2, CHARM_PERCENT_HUNDREDTHS_WHOLE],
        CharmStageValue::EffectPercent => definition.stages.map(CharmPercent::hundredths),
    };
    define(definition.key(), definition.category(), values, effect)
}

struct Catalogue(Vec<CharmDefinition>);

impl CharmCatalogueRead for Catalogue {
    fn charm(&self, key: &str) -> Option<&CharmDefinition> {
        self.0.iter().find(|definition| definition.key() == key)
    }
}

/// CHARM-3 test double: one character's assignments by race.
struct State {
    character: [u8; 16],
    by_race: Vec<(&'static str, Vec<CharmAssignment>)>,
    unavailable: bool,
}

impl State {
    fn with(race: &'static str, assigned: &[(&str, u8)]) -> Self {
        Self {
            character: CHARACTER,
            by_race: vec![(
                race,
                assigned
                    .iter()
                    .map(|(key, stage)| CharmAssignment {
                        charm_key: (*key).to_owned(),
                        unlocked_stage: *stage,
                    })
                    .collect(),
            )],
            unavailable: false,
        }
    }
}

impl CharmStateRead for State {
    fn character(&self) -> [u8; 16] {
        self.character
    }

    fn assignments_for_race(
        &self,
        race_key: &str,
    ) -> Result<Vec<CharmAssignment>, CharmStateUnavailable> {
        if self.unavailable {
            return Err(CharmStateUnavailable);
        }
        Ok(self
            .by_race
            .iter()
            .filter(|(race, _)| *race == race_key)
            .flat_map(|(_, assigned)| assigned.iter().cloned())
            .collect())
    }
}

const ROOT: GameplayDecisionRoot = GameplayDecisionRoot::from_bytes([7; 32]);

fn occurrence(seed: u32) -> DecisionOccurrenceId {
    let mut bytes = [0_u8; 16];
    bytes[..4].copy_from_slice(&seed.to_be_bytes());
    DecisionOccurrenceId::from_bytes(bytes)
}

const ATTACKER: CharmAttackerFacts = CharmAttackerFacts {
    level: 1_000,
    max_health: 4_000,
    max_mana: 10_000,
};

fn hit(creature_max_health: u64, before: i64, after: i64) -> CharmHookEvent {
    CharmHookEvent::CommittedHit {
        source: CharmHitSource::CharacterAttack,
        attacker: ATTACKER,
        creature_max_health,
        health_before: before,
        health_after: after,
    }
}

const NON_LETHAL: CharmHookEvent = CharmHookEvent::CommittedHit {
    source: CharmHitSource::CharacterAttack,
    attacker: ATTACKER,
    creature_max_health: 10_000,
    health_before: 10_000,
    health_after: 9_000,
};
const LETHAL: CharmHookEvent = CharmHookEvent::CommittedHit {
    source: CharmHitSource::CharacterAttack,
    attacker: ATTACKER,
    creature_max_health: 10_000,
    health_before: 100,
    health_after: 0,
};

fn input(event: CharmHookEvent, seed: u32) -> CharmEvaluationInput<'static> {
    CharmEvaluationInput {
        character: CHARACTER,
        race_key: RACE,
        decision_root: &ROOT,
        occurrence: occurrence(seed),
        event,
    }
}

fn evaluate(
    catalogue: &Catalogue,
    state: &State,
    event: CharmHookEvent,
    seed: u32,
) -> Result<Vec<CharmOutcome>, CharmEvaluationError> {
    evaluate_charm_hook(&input(event, seed), state, catalogue)
}

fn single(catalogue: &Catalogue, key: &str, stage: u8, event: CharmHookEvent) -> CharmResult {
    let outcomes = evaluate(catalogue, &State::with(RACE, &[(key, stage)]), event, 1)
        .expect("evaluation succeeds");
    assert_eq!(outcomes.len(), 1, "{key}: {outcomes:?}");
    assert_eq!(outcomes[0].charm_key, key);
    assert_eq!(outcomes[0].stage, stage);
    outcomes[0].result
}

fn forced_catalogue() -> Catalogue {
    Catalogue(catalogue_charms().iter().map(always_triggering).collect())
}

fn event_for(hook: CharmHook) -> CharmHookEvent {
    match hook {
        CharmHook::AttackDamageCalculation => CharmHookEvent::AttackDamageCalculation,
        CharmHook::AttackHit => NON_LETHAL,
        CharmHook::CreatureKilled => LETHAL,
        CharmHook::IncomingCreatureAttack => CharmHookEvent::IncomingCreatureAttack,
        CharmHook::IncomingCreatureHit => CharmHookEvent::IncomingCreatureHit { base_damage: 50 },
        CharmHook::IncomingManaDrain => CharmHookEvent::IncomingManaDrain { mana_drained: 30 },
        CharmHook::CreatureLoot => CharmHookEvent::CreatureLoot,
        CharmHook::Skinning => CharmHookEvent::Skinning,
        CharmHook::CharacterDeath => CharmHookEvent::CharacterDeath,
    }
}

#[test]
fn charm_effect_catalogue_has_all_25_charms_and_every_effect_type() {
    let charms = catalogue_charms();
    assert_eq!(charms.len(), 25);
    let keys: BTreeSet<_> = charms.iter().map(CharmDefinition::key).collect();
    assert_eq!(keys.len(), 25);
    let majors = charms
        .iter()
        .filter(|charm| charm.category() == CharmCategory::Major)
        .count();
    assert_eq!((majors, 25 - majors), (14, 11));
    let types: Vec<_> = charms
        .iter()
        .map(|charm| discriminant(&charm.effect()))
        .fold(Vec::new(), |mut seen, kind| {
            if !seen.contains(&kind) {
                seen.push(kind);
            }
            seen
        });
    // `charm.schema.json` `effect`: 3 damage shapes, 4 timed and 11 parameterless types.
    assert_eq!(types.len(), 18);
}

#[test]
fn charm_effect_hook_and_fail_closed_matrix_covers_every_charm() {
    use CharmHook as H;
    use CharmMissingSystem as M;
    let expected: [(&str, H, Option<M>); 25] = [
        (
            "adrenaline_burst",
            H::IncomingCreatureHit,
            Some(M::HasteCondition),
        ),
        ("bless", H::CharacterDeath, Some(M::DeathLossCharmInput)),
        ("carnage", H::CreatureKilled, Some(M::AreaTargetResolver)),
        ("cleanse", H::IncomingCreatureHit, Some(M::ConditionCleanse)),
        ("cripple", H::AttackHit, Some(M::ParalysisCondition)),
        ("curse", H::AttackHit, None),
        ("divine_wrath", H::AttackHit, None),
        (
            "dodge",
            H::IncomingCreatureAttack,
            Some(M::IncomingCreatureDamage),
        ),
        ("enflame", H::AttackHit, None),
        ("fatal_hold", H::AttackHit, Some(M::CreatureFlee)),
        ("freeze", H::AttackHit, None),
        ("gut", H::CreatureLoot, Some(M::CreatureProductLoot)),
        ("low_blow", H::AttackDamageCalculation, Some(M::CriticalHit)),
        ("numb", H::IncomingCreatureHit, Some(M::ParalysisCondition)),
        ("overflux", H::AttackHit, None),
        ("overpower", H::AttackHit, None),
        (
            "parry",
            H::IncomingCreatureHit,
            Some(M::IncomingCreatureDamage),
        ),
        ("poison", H::AttackHit, None),
        (
            "savage_blow",
            H::AttackDamageCalculation,
            Some(M::CriticalHit),
        ),
        ("scavenge", H::Skinning, Some(M::Skinning)),
        ("vampiric_embrace", H::AttackHit, Some(M::Leech)),
        ("void_inversion", H::IncomingManaDrain, Some(M::ManaDrain)),
        ("voids_call", H::AttackHit, Some(M::Leech)),
        ("wound", H::AttackHit, None),
        ("zap", H::AttackHit, None),
    ];
    let catalogue = forced_catalogue();
    for (name, hook, missing) in expected {
        let key = format!("oteryn:charm.{name}");
        let definition = catalogue.charm(&key).expect("catalogue charm");
        assert_eq!(definition.effect().hook(), hook, "{name}");
        assert_eq!(definition.effect().missing_system(), missing, "{name}");
        // At its own hook, with a certain trigger, the charm applies exactly when its system
        // exists and otherwise fails closed with that system named.
        match (single(&catalogue, &key, 3, event_for(hook)), missing) {
            (CharmResult::Applied(CharmEffectOutcome::ProcDamage { amount, .. }), None) => {
                assert!(amount > 0, "{name}");
            }
            (CharmResult::FailedClosed { reason, .. }, Some(missing)) => {
                assert_eq!(reason, missing, "{name}");
            }
            (other, _) => unreachable!("{name}: {other:?}"),
        }
    }
}

#[test]
fn charm_effect_attack_proc_damage_is_percent_of_max_health_capped_at_twice_level() {
    let catalogue = forced_catalogue();
    let wound = "oteryn:charm.wound";
    let proc = |attacker: CharmAttackerFacts, max_health: u64| {
        single(
            &catalogue,
            wound,
            3,
            CharmHookEvent::CommittedHit {
                source: CharmHitSource::CharacterAttack,
                attacker,
                creature_max_health: max_health,
                health_before: i64::try_from(max_health).expect("fits"),
                health_after: 1,
            },
        )
    };
    let damage = |amount| {
        CharmResult::Applied(CharmEffectOutcome::ProcDamage {
            damage: elemental(CharmElement::Physical),
            amount,
        })
    };
    // 5% of 10,000 is 500, below the cap of 2 × 1,000.
    assert_eq!(proc(ATTACKER, 10_000), damage(500));
    // Level 100 caps it at 200.
    let low = CharmAttackerFacts {
        level: 100,
        ..ATTACKER
    };
    assert_eq!(proc(low, 10_000), damage(200));
    // Exactly at the cap, and floor rounding: 5% of 1,999 is 99.95.
    assert_eq!(
        proc(
            CharmAttackerFacts {
                level: 250,
                ..ATTACKER
            },
            10_000
        ),
        damage(500)
    );
    assert_eq!(proc(ATTACKER, 1_999), damage(99));
    // 5% of 19 floors to 0: nothing applies.
    assert_eq!(proc(ATTACKER, 19), CharmResult::ZeroMagnitude);
    // The element follows the charm.
    assert_eq!(
        single(&catalogue, "oteryn:charm.divine_wrath", 3, NON_LETHAL),
        CharmResult::Applied(CharmEffectOutcome::ProcDamage {
            damage: elemental(CharmElement::Holy),
            amount: 500,
        })
    );
}

#[test]
fn charm_effect_resource_proc_is_percent_of_own_maximum_capped_at_8_percent_of_creature() {
    let catalogue = forced_catalogue();
    let run = |key, attacker, max_health| {
        single(
            &catalogue,
            key,
            3,
            CharmHookEvent::CommittedHit {
                source: CharmHitSource::CharacterAttack,
                attacker,
                creature_max_health: max_health,
                health_before: 100,
                health_after: 50,
            },
        )
    };
    let damage = |amount| {
        CharmResult::Applied(CharmEffectOutcome::ProcDamage {
            damage: RESOURCE_KIND,
            amount,
        })
    };
    // Overpower: 5% of 4,000 health is 200; 8% of 100,000 is 8,000.
    assert_eq!(
        run("oteryn:charm.overpower", ATTACKER, 100_000),
        damage(200)
    );
    // Capped at 8% of a 1,000-health creature.
    assert_eq!(run("oteryn:charm.overpower", ATTACKER, 1_000), damage(80));
    // Overflux: 2.5% of 10,000 mana is 250.
    assert_eq!(run("oteryn:charm.overflux", ATTACKER, 100_000), damage(250));
    // It reads mana, not health.
    let no_mana = CharmAttackerFacts {
        max_mana: 0,
        ..ATTACKER
    };
    assert_eq!(
        run("oteryn:charm.overflux", no_mana, 100_000),
        CharmResult::ZeroMagnitude
    );
    assert_eq!(run("oteryn:charm.overpower", no_mana, 100_000), damage(200));
}

#[test]
fn charm_effect_lethal_hit_runs_the_kill_hook_and_no_attack_proc() {
    let catalogue = forced_catalogue();
    let state = State::with(
        RACE,
        &[
            ("oteryn:charm.carnage", 3),
            ("oteryn:charm.vampiric_embrace", 2),
        ],
    );
    let outcomes = evaluate(&catalogue, &state, LETHAL, 1).expect("evaluates");
    assert_eq!(outcomes.len(), 2);
    // Hook order: the attack-hit leech first, then the kill.
    assert_eq!(outcomes[0].hook, CharmHook::AttackHit);
    assert_eq!(
        outcomes[0].result,
        CharmResult::FailedClosed {
            reason: CharmMissingSystem::Leech,
            evaluated: CharmEffectOutcome::LifeLeechBonus { percent: pct(240) },
        }
    );
    assert_eq!(outcomes[1].hook, CharmHook::CreatureKilled);
    assert_eq!(
        outcomes[1].result,
        CharmResult::FailedClosed {
            reason: CharmMissingSystem::AreaTargetResolver,
            // 15% of the killed creature's 10,000 maximum health.
            evaluated: CharmEffectOutcome::AreaDamageAroundKill {
                damage: CARNAGE_KIND,
                amount: 1_500,
            },
        }
    );
    // Capped at 6 × level: level 100 caps the 1,500 at 600.
    let low_level = CharmHookEvent::CommittedHit {
        source: CharmHitSource::CharacterAttack,
        attacker: CharmAttackerFacts {
            level: 100,
            ..ATTACKER
        },
        creature_max_health: 10_000,
        health_before: 100,
        health_after: 0,
    };
    assert_eq!(
        single(&catalogue, "oteryn:charm.carnage", 3, low_level),
        CharmResult::FailedClosed {
            reason: CharmMissingSystem::AreaTargetResolver,
            evaluated: CharmEffectOutcome::AreaDamageAroundKill {
                damage: CARNAGE_KIND,
                amount: 600,
            },
        }
    );
    // A proc on the creature the hit killed has no target, and no roll is made.
    assert_eq!(
        single(&catalogue, "oteryn:charm.wound", 3, LETHAL),
        CharmResult::NoLivingTarget
    );
    assert_eq!(
        single(&catalogue, "oteryn:charm.cripple", 3, LETHAL),
        CharmResult::NoLivingTarget
    );
    // Negative: a non-lethal hit does not run the kill hook.
    let state = State::with(RACE, &[("oteryn:charm.carnage", 3)]);
    assert_eq!(evaluate(&catalogue, &state, NON_LETHAL, 1), Ok(Vec::new()));
}

#[test]
fn charm_effect_effect_percent_charms_use_the_unlocked_stage_value_without_a_roll() {
    let catalogue = Catalogue(catalogue_charms());
    for (stage, hundredths) in [(1, 160), (2, 240), (3, 320)] {
        // No draw is made, so every occurrence gives the same result.
        for seed in 0..32 {
            let outcomes = evaluate(
                &catalogue,
                &State::with(RACE, &[("oteryn:charm.vampiric_embrace", stage)]),
                NON_LETHAL,
                seed,
            )
            .expect("evaluates");
            assert_eq!(
                outcomes[0].result,
                CharmResult::FailedClosed {
                    reason: CharmMissingSystem::Leech,
                    evaluated: CharmEffectOutcome::LifeLeechBonus {
                        percent: pct(hundredths),
                    },
                }
            );
        }
    }
    assert_eq!(
        single(
            &catalogue,
            "oteryn:charm.bless",
            3,
            CharmHookEvent::CharacterDeath
        ),
        CharmResult::FailedClosed {
            reason: CharmMissingSystem::DeathLossCharmInput,
            evaluated: CharmEffectOutcome::DeathLossReduction { percent: pct(1200) },
        }
    );
}

#[test]
fn charm_effect_incoming_hit_effects_carry_the_event_magnitude() {
    let catalogue = forced_catalogue();
    assert_eq!(
        single(
            &catalogue,
            "oteryn:charm.parry",
            3,
            CharmHookEvent::IncomingCreatureHit { base_damage: 321 }
        ),
        CharmResult::FailedClosed {
            reason: CharmMissingSystem::IncomingCreatureDamage,
            evaluated: CharmEffectOutcome::ReflectDamage {
                damage: PARRY_KIND,
                amount: 321,
            },
        }
    );
    assert_eq!(
        single(
            &catalogue,
            "oteryn:charm.void_inversion",
            3,
            CharmHookEvent::IncomingManaDrain { mana_drained: 77 }
        ),
        CharmResult::FailedClosed {
            reason: CharmMissingSystem::ManaDrain,
            evaluated: CharmEffectOutcome::InvertManaDrain { mana_gained: 77 },
        }
    );
    assert_eq!(
        single(
            &catalogue,
            "oteryn:charm.numb",
            3,
            CharmHookEvent::IncomingCreatureHit { base_damage: 1 }
        ),
        CharmResult::FailedClosed {
            reason: CharmMissingSystem::ParalysisCondition,
            evaluated: CharmEffectOutcome::ParalyseCreature {
                duration_ms: 10_000
            },
        }
    );
}

#[test]
fn charm_effect_applies_only_at_its_own_hook() {
    let catalogue = forced_catalogue();
    let state = State::with(
        RACE,
        &[("oteryn:charm.parry", 3), ("oteryn:charm.cripple", 3)],
    );
    for hook in [
        CharmHook::AttackDamageCalculation,
        CharmHook::AttackHit,
        CharmHook::CreatureKilled,
        CharmHook::IncomingCreatureAttack,
        CharmHook::IncomingCreatureHit,
        CharmHook::IncomingManaDrain,
        CharmHook::CreatureLoot,
        CharmHook::Skinning,
        CharmHook::CharacterDeath,
    ] {
        let outcomes = evaluate(&catalogue, &state, event_for(hook), 1).expect("evaluates");
        let keys: Vec<_> = outcomes
            .iter()
            .map(|outcome| outcome.charm_key.as_str())
            .collect();
        let expected: &[&str] = match hook {
            CharmHook::IncomingCreatureHit => &["oteryn:charm.parry"],
            // The lethal event also runs the attack-hit hook (with no living target).
            CharmHook::AttackHit | CharmHook::CreatureKilled => &["oteryn:charm.cripple"],
            _ => &[],
        };
        assert_eq!(keys, expected, "{hook:?}");
    }
}

#[test]
fn charm_effect_applies_only_against_the_assigned_race() {
    let catalogue = forced_catalogue();
    let state = State::with(RACE, &[("oteryn:charm.wound", 3)]);
    let mut other = input(NON_LETHAL, 1);
    other.race_key = OTHER_RACE;
    assert_eq!(
        evaluate_charm_hook(&other, &state, &catalogue),
        Ok(Vec::new())
    );
    assert_eq!(
        evaluate(&catalogue, &state, NON_LETHAL, 1)
            .expect("evaluates")
            .len(),
        1
    );
}

#[test]
fn charm_effect_rejects_charm_state_of_another_character() {
    let catalogue = forced_catalogue();
    let mut state = State::with(RACE, &[("oteryn:charm.wound", 3)]);
    state.character = OTHER_CHARACTER;
    assert_eq!(
        evaluate(&catalogue, &state, NON_LETHAL, 1),
        Err(CharmEvaluationError::StateForOtherCharacter)
    );
}

#[test]
fn charm_effect_fails_closed_when_the_charm_state_is_unavailable() {
    let catalogue = forced_catalogue();
    let mut state = State::with(RACE, &[("oteryn:charm.wound", 3)]);
    state.unavailable = true;
    assert_eq!(
        evaluate(&catalogue, &state, NON_LETHAL, 1),
        Err(CharmEvaluationError::StateUnavailable)
    );
}

#[test]
fn charm_effect_rejects_a_stage_outside_1_to_3() {
    let catalogue = forced_catalogue();
    for stage in [0, CHARM_STAGE_MAX + 1, u8::MAX] {
        assert_eq!(
            evaluate(
                &catalogue,
                &State::with(RACE, &[("oteryn:charm.wound", stage)]),
                NON_LETHAL,
                1
            ),
            Err(CharmEvaluationError::InvalidStage),
            "stage {stage}"
        );
    }
}

#[test]
fn charm_effect_rejects_an_unknown_charm() {
    let catalogue = forced_catalogue();
    assert_eq!(
        evaluate(
            &catalogue,
            &State::with(RACE, &[("oteryn:charm.unknown", 1)]),
            NON_LETHAL,
            1
        ),
        Err(CharmEvaluationError::UnknownCharm(
            "oteryn:charm.unknown".to_owned()
        ))
    );
}

#[test]
fn charm_effect_rejects_more_assignments_on_a_race_than_one_per_category() {
    let catalogue = forced_catalogue();
    let too_many = State::with(
        RACE,
        &[
            ("oteryn:charm.wound", 1),
            ("oteryn:charm.gut", 1),
            ("oteryn:charm.bless", 1),
        ],
    );
    assert_eq!(
        evaluate(&catalogue, &too_many, NON_LETHAL, 1),
        Err(CharmEvaluationError::TooManyAssignments)
    );
    let two_majors = State::with(RACE, &[("oteryn:charm.wound", 1), ("oteryn:charm.zap", 1)]);
    assert_eq!(
        evaluate(&catalogue, &two_majors, NON_LETHAL, 1),
        Err(CharmEvaluationError::DuplicateCategory)
    );
    let twice = State::with(
        RACE,
        &[("oteryn:charm.wound", 1), ("oteryn:charm.wound", 2)],
    );
    assert_eq!(
        evaluate(&catalogue, &twice, NON_LETHAL, 1),
        Err(CharmEvaluationError::DuplicateCharm)
    );
    // Positive: one major and one minor on one race.
    let pair = State::with(
        RACE,
        &[("oteryn:charm.wound", 1), ("oteryn:charm.cripple", 1)],
    );
    assert_eq!(
        evaluate(&catalogue, &pair, NON_LETHAL, 1)
            .expect("evaluates")
            .len(),
        2
    );
}

#[test]
fn charm_effect_rejects_inconsistent_event_facts() {
    let catalogue = forced_catalogue();
    let state = State::with(RACE, &[("oteryn:charm.wound", 3)]);
    assert!(evaluate(&catalogue, &state, NON_LETHAL, 1).is_ok());
    let with = |change: fn(&mut CharmAttackerFacts, &mut u64, &mut i64, &mut i64)| {
        let (mut attacker, mut max, mut before, mut after) = (ATTACKER, 10_000, 10_000, 9_000);
        change(&mut attacker, &mut max, &mut before, &mut after);
        CharmHookEvent::CommittedHit {
            source: CharmHitSource::CharacterAttack,
            attacker,
            creature_max_health: max,
            health_before: before,
            health_after: after,
        }
    };
    let invalid = [
        with(|attacker, _, _, _| attacker.level = 0),
        with(|attacker, _, _, _| attacker.max_health = 0),
        with(|_, max, _, _| *max = 0),
        with(|_, _, before, after| (*before, *after) = (0, 0)),
        with(|_, _, _, after| *after = 10_000),
        with(|_, _, _, after| *after = 10_001),
        with(|_, _, _, after| *after = -1),
        with(|_, _, before, _| *before = -5),
        with(|_, max, _, _| *max = 9_999),
        CharmHookEvent::IncomingCreatureHit { base_damage: 0 },
        CharmHookEvent::IncomingManaDrain { mana_drained: 0 },
    ];
    for event in invalid {
        assert_eq!(
            evaluate(&catalogue, &state, event, 1),
            Err(CharmEvaluationError::InvalidEventFacts),
            "{event:?}"
        );
    }
}

#[test]
fn charm_effect_definition_rejects_catalogue_rule_violations() {
    let wound = proc_damage(CharmElement::Physical);
    let major = CharmCategory::Major;
    let chance = CharmStageValue::TriggerChancePercent;
    assert!(CharmDefinition::new("k", major, chance, stages([500, 1000, 1100]), wound).is_ok());
    let cases = [
        (
            CharmDefinition::new("", major, chance, stages([500, 1000, 1100]), wound),
            CharmDefinitionError::EmptyKey,
        ),
        (
            CharmDefinition::new(
                "k",
                major,
                CharmStageValue::EffectPercent,
                stages([500, 1000, 1100]),
                wound,
            ),
            CharmDefinitionError::StageValueMismatch,
        ),
        (
            CharmDefinition::new("k", major, chance, stages([500, 1000, 1000]), wound),
            CharmDefinitionError::StageValuesNotIncreasing,
        ),
        (
            CharmDefinition::new("k", major, chance, stages([1100, 1000, 1200]), wound),
            CharmDefinitionError::StageValuesNotIncreasing,
        ),
        (
            CharmDefinition::new("k", major, chance, stages([500, 1000, 10_001]), wound),
            CharmDefinitionError::TriggerChanceAbove100,
        ),
        (
            CharmDefinition::new(
                "k",
                major,
                chance,
                stages([500, 1000, 1100]),
                CharmEffect::AttackProcDamage {
                    damage: elemental(CharmElement::Physical),
                    percent_of_creature_max_health: pct(500),
                    damage_cap_level_multiplier: 0,
                },
            ),
            CharmDefinitionError::InvalidParameter,
        ),
        (
            CharmDefinition::new(
                "k",
                major,
                chance,
                stages([1000, 2000, 2200]),
                CharmEffect::KillAreaDamage {
                    damage: CARNAGE_KIND,
                    percent_of_creature_max_health: pct(1500),
                    damage_cap_level_multiplier: 0,
                },
            ),
            CharmDefinitionError::InvalidParameter,
        ),
        (
            CharmDefinition::new(
                "k",
                CharmCategory::Minor,
                chance,
                stages([600, 900, 1200]),
                CharmEffect::HasteAfterHit { duration_ms: 0 },
            ),
            CharmDefinitionError::InvalidParameter,
        ),
    ];
    for (result, error) in cases {
        assert_eq!(result, Err(error));
    }
    // An effect percent may exceed 100% (Scavenge 120%); a percent is 0 < p <= 1000%.
    assert!(
        CharmDefinition::new(
            "k",
            CharmCategory::Minor,
            CharmStageValue::EffectPercent,
            stages([6000, 9000, 12_000]),
            CharmEffect::SkinningChanceBonus,
        )
        .is_ok()
    );
    assert_eq!(CharmPercent::from_hundredths(0), None);
    assert_eq!(
        CharmPercent::from_hundredths(CHARM_PERCENT_HUNDREDTHS_MAX + 1),
        None
    );
    assert!(CharmPercent::from_hundredths(CHARM_PERCENT_HUNDREDTHS_MAX).is_some());
}

/// The roll is exactly the simulation-determinism decision draw of the event's occurrence.
fn expected_trigger(seed: u32, hook: CharmHook, category: CharmCategory, chance: u32) -> bool {
    let draw = deterministic_decision_u64(
        &ROOT,
        occurrence(seed),
        "oteryn.charm.trigger.v1",
        trigger_draw_index(hook, category),
    )
    .expect("valid purpose");
    ((u128::from(draw) * 10_000) >> 64) < u128::from(chance)
}

#[test]
fn charm_effect_trigger_rolls_are_simulation_determinism_draws() {
    let catalogue = Catalogue(catalogue_charms());
    let state = State::with(
        RACE,
        &[("oteryn:charm.wound", 1), ("oteryn:charm.cripple", 3)],
    );
    let mut wound_triggers = 0_u32;
    for seed in 0..10_000 {
        let outcomes = evaluate(&catalogue, &state, NON_LETHAL, seed).expect("evaluates");
        let wound = expected_trigger(seed, CharmHook::AttackHit, CharmCategory::Major, 500);
        let cripple = expected_trigger(seed, CharmHook::AttackHit, CharmCategory::Minor, 1200);
        assert_eq!(outcomes[0].charm_key, "oteryn:charm.wound");
        assert_eq!(
            outcomes[0].result != CharmResult::NotTriggered,
            wound,
            "seed {seed}"
        );
        assert_eq!(
            outcomes[1].result != CharmResult::NotTriggered,
            cripple,
            "seed {seed}"
        );
        wound_triggers += u32::from(wound);
    }
    // 5% of 10,000 deterministic occurrences.
    assert!((400..=600).contains(&wound_triggers), "{wound_triggers}");
}

#[test]
fn charm_effect_replay_of_an_occurrence_reproduces_every_roll() {
    let catalogue = Catalogue(catalogue_charms());
    let state = State::with(
        RACE,
        &[
            ("oteryn:charm.overpower", 2),
            ("oteryn:charm.fatal_hold", 1),
        ],
    );
    let reversed = State::with(
        RACE,
        &[
            ("oteryn:charm.fatal_hold", 1),
            ("oteryn:charm.overpower", 2),
        ],
    );
    let committed = OwnerDamageResult {
        applied: true,
        health_before: 10_000,
        health_after: 9_000,
    };
    // The carrier's idempotent replay returns the prior result with `applied: false`.
    let replayed = OwnerDamageResult {
        applied: false,
        ..committed
    };
    let mut mask = 0_u64;
    for seed in 0..32 {
        let first = evaluate(
            &catalogue,
            &state,
            CharmHookEvent::committed_hit(
                &committed,
                CharmHitSource::CharacterAttack,
                ATTACKER,
                10_000,
            ),
            seed,
        );
        let replay = evaluate(
            &catalogue,
            &reversed,
            CharmHookEvent::committed_hit(
                &replayed,
                CharmHitSource::CharacterAttack,
                ATTACKER,
                10_000,
            ),
            seed,
        );
        assert_eq!(first, replay, "seed {seed}");
        let outcomes = first.expect("evaluates");
        for (bit, outcome) in outcomes.iter().enumerate() {
            if outcome.result != CharmResult::NotTriggered {
                mask |= 1 << (seed * 2 + u32::try_from(bit).expect("small"));
            }
        }
    }
    // Pinned so that a change of purpose, draw index or roll mapping, which would make old
    // replays diverge, fails here.
    assert_eq!(mask, CHARM_REPLAY_PIN);
    // Another decision root draws differently.
    let other_root = GameplayDecisionRoot::from_bytes([8; 32]);
    let differs = (0..64).any(|seed| {
        let mut other = input(NON_LETHAL, seed);
        other.decision_root = &other_root;
        evaluate_charm_hook(&other, &state, &catalogue)
            != evaluate(&catalogue, &state, NON_LETHAL, seed)
    });
    assert!(differs);
}

const CHARM_REPLAY_PIN: u64 = 0x2400_2a80_2821_8008;

#[test]
fn charm_effect_draw_indices_are_distinct_per_hook_and_category() {
    let hooks = [
        CharmHook::AttackDamageCalculation,
        CharmHook::AttackHit,
        CharmHook::CreatureKilled,
        CharmHook::IncomingCreatureAttack,
        CharmHook::IncomingCreatureHit,
        CharmHook::IncomingManaDrain,
        CharmHook::CreatureLoot,
        CharmHook::Skinning,
        CharmHook::CharacterDeath,
    ];
    let indices: BTreeSet<u64> = hooks
        .iter()
        .flat_map(|hook| {
            [CharmCategory::Major, CharmCategory::Minor]
                .map(|category| trigger_draw_index(*hook, category))
        })
        .collect();
    assert_eq!(indices.len(), 18);
    assert_eq!(
        trigger_draw_index(CharmHook::AttackHit, CharmCategory::Major),
        2
    );
    assert_eq!(
        trigger_draw_index(CharmHook::CreatureKilled, CharmCategory::Major),
        4
    );
    assert!(
        CHARM_TRIGGER_PURPOSE.len() <= oteryn_simulation_determinism::MAX_DECISION_PURPOSE_BYTES
    );
    assert_eq!(hundredths_roll(0), 0);
    assert_eq!(hundredths_roll(u64::MAX), 9_999);
}

#[test]
fn charm_effect_full_trigger_chance_always_triggers_and_low_chance_follows_the_draw() {
    let catalogue = forced_catalogue();
    let zap = State::with(RACE, &[("oteryn:charm.zap", 3)]);
    for seed in 0..256 {
        let outcomes = evaluate(&catalogue, &zap, hit(10_000, 10_000, 1), seed).expect("evaluates");
        assert!(matches!(outcomes[0].result, CharmResult::Applied(_)));
        // Stage 1 of the forced charm is 0.01%.
        let low = evaluate(
            &catalogue,
            &State::with(RACE, &[("oteryn:charm.zap", 1)]),
            NON_LETHAL,
            seed,
        )
        .expect("evaluates");
        assert_eq!(
            low[0].result != CharmResult::NotTriggered,
            expected_trigger(seed, CharmHook::AttackHit, CharmCategory::Major, 1)
        );
    }
}

#[test]
fn charm_effect_charm_damage_never_chains_into_another_charm() {
    let catalogue = forced_catalogue();
    // A kill by Carnage (or any charm damage) triggers no Carnage, proc or leech.
    let state = State::with(
        RACE,
        &[
            ("oteryn:charm.carnage", 3),
            ("oteryn:charm.vampiric_embrace", 3),
        ],
    );
    for event in [LETHAL, NON_LETHAL] {
        let CharmHookEvent::CommittedHit {
            attacker,
            creature_max_health,
            health_before,
            health_after,
            ..
        } = event
        else {
            unreachable!("committed hits")
        };
        let from_charm = CharmHookEvent::committed_hit(
            &OwnerDamageResult {
                applied: true,
                health_before,
                health_after,
            },
            CharmHitSource::CharmDamage,
            attacker,
            creature_max_health,
        );
        assert_eq!(evaluate(&catalogue, &state, from_charm, 1), Ok(Vec::new()));
        // Positive: the character's own attack does run them.
        assert!(
            !evaluate(&catalogue, &state, event, 1)
                .expect("evaluates")
                .is_empty()
        );
    }
}

#[test]
fn charm_effect_auto_attack_off_its_main_target_triggers_no_charm() {
    let catalogue = forced_catalogue();
    for charm in ["oteryn:charm.wound", "oteryn:charm.carnage"] {
        let state = State::with(RACE, &[(charm, 3)]);
        for event in [LETHAL, NON_LETHAL] {
            let CharmHookEvent::CommittedHit {
                attacker,
                creature_max_health,
                health_before,
                health_after,
                ..
            } = event
            else {
                unreachable!("committed hits")
            };
            let off_target = CharmHookEvent::committed_hit(
                &OwnerDamageResult {
                    applied: true,
                    health_before,
                    health_after,
                },
                CharmHitSource::CharacterAutoAttackOffTarget,
                attacker,
                creature_max_health,
            );
            assert_eq!(evaluate(&catalogue, &state, off_target, 1), Ok(Vec::new()));
        }
        // Positive: the same hit on the main target (or by a spell or rune) does run it.
        let event = if charm == "oteryn:charm.carnage" {
            LETHAL
        } else {
            NON_LETHAL
        };
        assert!(
            !evaluate(&catalogue, &state, event, 1)
                .expect("evaluates")
                .is_empty()
        );
    }
    // Low Blow applies before the damage of every hit, so area ammunition keeps it on every
    // creature it hits.
    let state = State::with(RACE, &[("oteryn:charm.low_blow", 3)]);
    let low_blow = evaluate(
        &catalogue,
        &state,
        CharmHookEvent::AttackDamageCalculation,
        1,
    )
    .expect("evaluates");
    assert_eq!(low_blow.len(), 1);
    assert_eq!(low_blow[0].charm_key, "oteryn:charm.low_blow");
}
