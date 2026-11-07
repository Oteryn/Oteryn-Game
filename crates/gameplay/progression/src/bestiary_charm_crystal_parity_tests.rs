//! Re-audit P01 parity: Bestiary kill stages, charm points and the charm catalogue/effects on
//! the production path (`canonical_bestiary_rows`, `bestiary_stage`, the embedded charm
//! catalogue, `derive_balance` and `effect_outcome`) against Crystal
//! `201189f871d23597b82caf88fd611454a2ddefb8` (`iobestiary.cpp`, `bestiary_charms.lua`),
//! checked against Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446`. The Global Bestiary table
//! (TibiaWiki `Template:Bestiary_Table`) is the authority for per-creature values; where Crystal
//! data disagrees with it the difference is pinned, not adopted.
#![allow(clippy::expect_used, clippy::panic)]

use std::collections::{BTreeMap, BTreeSet};

use super::{
    CharmAttackerFacts, CharmCatalogueRead, CharmEffect, CharmEffectOutcome, CharmHitSource,
    CharmHookEvent, CharmResource, effect_outcome,
};
use crate::bestiary::bestiary_stage;
use oteryn_gameplay_content::content::{
    ProjectEvidenceLimits, canonical_bestiary_rows, charm_source::CanonicalCharmCatalogue,
};
use oteryn_gameplay_domain::domain::bestiary::BestiaryRace;
use oteryn_gameplay_domain::domain::charm::{
    CharmCategory, CharmKey, CharmStage, PROMOTION_ECHOES, derive_balance,
};

/// The limits the production charm progression uses for the same source.
fn rows() -> Vec<(BestiaryRace, u32)> {
    canonical_bestiary_rows(ProjectEvidenceLimits {
        max_documents: 5,
        max_document_bytes: 2_500_000,
        max_total_bytes: 8_000_000,
        max_json_depth: 10,
        max_decoded_fields: 65_000,
        max_string_bytes: 1_000_000,
        max_locator_bytes: 128,
        max_locator_segments: 5,
        max_reference_records: 4096,
        max_import_records: 1,
        max_reimport_states: 1,
    })
    .expect("canonical Bestiary rows")
}

/// Crystal `IOBestiary::getKillStatus` over `[FirstUnlock, SecondUnlock, toUnlock]`: 1..=4.
fn crystal_kill_status(thresholds: [u32; 3], kills: u32) -> u8 {
    if kills < thresholds[0] {
        1
    } else if kills < thresholds[1] {
        2
    } else if kills < thresholds[2] {
        3
    } else {
        4
    }
}

fn thresholds(race: &BestiaryRace) -> [u32; 3] {
    race.kill_thresholds()
        .try_into()
        .expect("three Bestiary thresholds")
}

#[test]
fn every_canonical_row_follows_the_global_bestiary_table() {
    // (thresholds, charm points) of each difficulty: ordinary occurrences, then Very Rare.
    let table: BTreeMap<([u32; 3], u32), &str> = [
        (([5, 10, 25], 1), "harmless"),
        (([10, 100, 250], 5), "trivial"),
        (([25, 250, 500], 15), "easy"),
        (([50, 500, 1000], 25), "medium"),
        (([100, 1000, 2500], 50), "hard"),
        (([200, 2000, 5000], 100), "challenging"),
        (([2, 3, 5], 5), "harmless very rare"),
        (([2, 3, 5], 10), "trivial very rare"),
        (([2, 3, 5], 30), "easy very rare"),
        (([2, 3, 5], 50), "medium very rare"),
        (([2, 3, 5], 100), "hard very rare"),
        (([2, 3, 5], 200), "challenging very rare"),
    ]
    .into_iter()
    .collect();
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    let rows = rows();
    for (race, points) in &rows {
        let class = table
            .get(&(thresholds(race), *points))
            .unwrap_or_else(|| panic!("{} is off the Global table", race.key()));
        *counts.entry(class).or_default() += 1;
    }
    let expected: BTreeMap<&str, usize> = [
        ("harmless", 21),
        ("trivial", 46),
        ("easy", 163),
        ("medium", 278),
        ("hard", 215),
        ("challenging", 67),
        ("trivial very rare", 3),
        ("easy very rare", 7),
        ("medium very rare", 23),
    ]
    .into_iter()
    .collect();
    assert_eq!(counts, expected);
    assert_eq!(rows.len(), 823);
}

/// Creatures whose Crystal monster file disagrees with the Global table. Each Global value was
/// read from its TibiaWiki page; Crystal's is the recorded difference (40 of 819 shared races
/// differ in thresholds or points).
#[test]
fn wiki_verified_crystal_deviations_keep_the_global_values() {
    let rows = rows();
    for (key, global, global_points, crystal, crystal_points) in [
        ("corrupted_skeleton", [10, 100, 250], 5, [25, 250, 500], 15),
        ("infernal_frog", [50, 500, 1000], 25, [25, 250, 500], 15),
        ("gloom_maw", [100, 1000, 2500], 50, [50, 500, 2500], 50),
        ("bluebeak", [100, 1000, 2500], 50, [50, 500, 2500], 50),
        (
            "brinebrute_inferniarch",
            [100, 1000, 2500],
            50,
            [200, 2000, 2500],
            50,
        ),
        ("iks_ahpututu", [2, 3, 5], 50, [1, 2, 5], 50),
        ("chocolate_blob", [5, 10, 25], 1, [10, 100, 250], 5),
        ("infernoid_blob", [25, 250, 500], 15, [50, 500, 1000], 15),
        ("corrupted_ghost", [10, 100, 250], 5, [17, 175, 250], 5),
        ("bride_of_night", [2, 3, 5], 50, [1, 5, 5], 50),
    ] {
        let key = format!("oteryn:creature.{key}");
        let (race, points) = rows
            .iter()
            .find(|(race, _)| race.key() == key)
            .unwrap_or_else(|| panic!("{key} is not a Bestiary race"));
        assert_eq!(
            (thresholds(race), *points),
            (global, global_points),
            "{key}"
        );
        assert_ne!((global, global_points), (crystal, crystal_points), "{key}");
    }
}

#[test]
fn bestiary_stage_is_crystal_kill_status_minus_one_for_every_race() {
    for (race, _) in rows() {
        let thresholds = thresholds(&race);
        let mut kills: BTreeSet<u32> = [0, u32::MAX].into_iter().collect();
        for threshold in thresholds {
            kills.extend([threshold - 1, threshold, threshold + 1, threshold * 2]);
        }
        for kills in kills {
            assert_eq!(
                bestiary_stage(&race, kills).get() + 1,
                crystal_kill_status(thresholds, kills),
                "{} at {kills} kills",
                race.key()
            );
        }
        // Crystal keeps counting past `toUnlock`; Oteryn's count stops there. The stage, and
        // so every charm rule that reads it, is the same either way.
        let last = thresholds[2];
        assert_eq!(race.next_kill_count(last - 1), Some(last));
        assert_eq!(race.next_kill_count(last), None);
        assert_eq!(crystal_kill_status(thresholds, last + 1_000), 4);
    }
}

/// Crystal `bestiary_charms.lua`: key, category, stage costs, stage values in hundredths of a
/// percent (`chance`), the damage `percent` in hundredths and the condition duration.
#[allow(clippy::type_complexity)]
const CRYSTAL_CHARMS: [(
    &str,
    CharmCategory,
    [u32; 3],
    [u32; 3],
    Option<u32>,
    Option<u32>,
); 25] = {
    use CharmCategory::{Major, Minor};
    [
        (
            "adrenaline_burst",
            Minor,
            [100, 150, 225],
            [600, 900, 1200],
            None,
            Some(10_000),
        ),
        (
            "bless",
            Minor,
            [100, 150, 225],
            [600, 900, 1200],
            None,
            None,
        ),
        (
            "carnage",
            Major,
            [600, 900, 3000],
            [1000, 2000, 2200],
            Some(1500),
            None,
        ),
        (
            "cleanse",
            Minor,
            [100, 150, 225],
            [600, 900, 1200],
            None,
            None,
        ),
        (
            "cripple",
            Minor,
            [100, 150, 225],
            [600, 900, 1200],
            None,
            Some(10_000),
        ),
        (
            "curse",
            Major,
            [360, 540, 1800],
            [500, 1000, 1100],
            Some(500),
            None,
        ),
        (
            "divine_wrath",
            Major,
            [600, 900, 3000],
            [500, 1000, 1100],
            Some(500),
            None,
        ),
        (
            "dodge",
            Major,
            [240, 360, 1200],
            [500, 1000, 1100],
            None,
            None,
        ),
        (
            "enflame",
            Major,
            [400, 600, 2000],
            [500, 1000, 1100],
            Some(500),
            None,
        ),
        (
            "fatal_hold",
            Minor,
            [100, 150, 225],
            [3000, 4500, 6000],
            None,
            Some(30_000),
        ),
        (
            "freeze",
            Major,
            [320, 480, 1600],
            [500, 1000, 1100],
            Some(500),
            None,
        ),
        ("gut", Minor, [100, 150, 225], [600, 900, 1200], None, None),
        (
            "low_blow",
            Major,
            [800, 1200, 4000],
            [400, 800, 900],
            None,
            None,
        ),
        (
            "numb",
            Minor,
            [100, 150, 225],
            [600, 900, 1200],
            None,
            Some(10_000),
        ),
        (
            "overflux",
            Major,
            [600, 900, 3000],
            [500, 1000, 1100],
            Some(250),
            None,
        ),
        (
            "overpower",
            Major,
            [600, 900, 3000],
            [500, 1000, 1100],
            Some(500),
            None,
        ),
        (
            "parry",
            Major,
            [400, 600, 2000],
            [500, 1000, 1100],
            None,
            None,
        ),
        (
            "poison",
            Major,
            [240, 360, 1200],
            [500, 1000, 1100],
            Some(500),
            None,
        ),
        (
            "savage_blow",
            Major,
            [800, 1200, 4000],
            [2000, 4000, 4400],
            None,
            None,
        ),
        (
            "scavenge",
            Minor,
            [100, 150, 225],
            [6000, 9000, 12000],
            None,
            None,
        ),
        (
            "vampiric_embrace",
            Minor,
            [100, 150, 225],
            [160, 240, 320],
            None,
            None,
        ),
        (
            "void_inversion",
            Minor,
            [100, 150, 225],
            [2000, 3000, 4000],
            None,
            None,
        ),
        (
            "voids_call",
            Minor,
            [100, 150, 225],
            [80, 120, 160],
            None,
            None,
        ),
        (
            "wound",
            Major,
            [240, 360, 1200],
            [500, 1000, 1100],
            Some(500),
            None,
        ),
        (
            "zap",
            Major,
            [320, 480, 1600],
            [500, 1000, 1100],
            Some(500),
            None,
        ),
    ]
};

#[test]
fn embedded_charm_catalogue_equals_crystal() {
    let imported = CanonicalCharmCatalogue::embedded().expect("embedded charms");
    assert_eq!(
        imported.catalogue().definitions().count(),
        CRYSTAL_CHARMS.len()
    );
    for (name, category, costs, stages, percent, duration) in CRYSTAL_CHARMS {
        let key = format!("oteryn:charm.{name}");
        let rule = imported
            .catalogue()
            .get(&CharmKey::new(key.clone()).expect("charm key"))
            .unwrap_or_else(|_| panic!("{key} is not in the catalogue"));
        assert_eq!(
            (rule.category, rule.stage_costs),
            (category, costs),
            "{key}"
        );
        let definition = imported.charm(&key).expect("charm effect");
        let values = [1, 2, 3].map(|stage| definition.stage(stage).expect("stage").hundredths());
        assert_eq!(values, stages, "{key}");
        let effect_percent = match definition.effect() {
            CharmEffect::AttackProcDamage {
                percent_of_creature_max_health,
                damage_cap_level_multiplier,
                ..
            } => {
                assert_eq!(damage_cap_level_multiplier, 2, "{key}");
                Some(percent_of_creature_max_health.hundredths())
            }
            CharmEffect::KillAreaDamage {
                percent_of_creature_max_health,
                damage_cap_level_multiplier,
                ..
            } => {
                assert_eq!(damage_cap_level_multiplier, 6, "{key}");
                Some(percent_of_creature_max_health.hundredths())
            }
            CharmEffect::AttackProcResourceDamage {
                percent_of_own_maximum,
                damage_cap_percent_of_creature_max_health,
                ..
            } => {
                assert_eq!(damage_cap_percent_of_creature_max_health.hundredths(), 800);
                Some(percent_of_own_maximum.hundredths())
            }
            _ => None,
        };
        assert_eq!(effect_percent, percent, "{key}");
        let effect_duration = match definition.effect() {
            CharmEffect::ParalyseCreatureOnAttack { duration_ms }
            | CharmEffect::ParalyseCreatureAfterItsAttack { duration_ms }
            | CharmEffect::HasteAfterHit { duration_ms }
            | CharmEffect::PreventCreatureFlee { duration_ms } => Some(duration_ms),
            _ => None,
        };
        assert_eq!(effect_duration, duration, "{key}");
    }
}

/// Crystal `sendBuyCharmRune` action 0: a major tier `t` (0-based) costs `points[t]` and grants
/// `25t² + 25t + 50` Minor Charm Echoes; a minor tier is paid with echoes. Promotion grants 100.
#[test]
fn unlock_balance_equals_crystal_buy_charm_rune() {
    let imported = CanonicalCharmCatalogue::embedded().expect("embedded charms");
    let catalogue = imported.catalogue();
    for (name, category, costs, ..) in CRYSTAL_CHARMS {
        let key = CharmKey::new(format!("oteryn:charm.{name}")).expect("charm key");
        for reached in 1..=3_u8 {
            let unlocks =
                BTreeMap::from([(key.clone(), CharmStage::new(reached).expect("charm stage"))]);
            let paid: u32 = costs[..usize::from(reached)].iter().sum();
            let spent = u64::from(paid);
            let echoes: u64 = (0..u64::from(reached))
                .map(|tier| 25 * tier * tier + 25 * tier + 50)
                .sum();
            for promoted in [false, true] {
                let balance =
                    derive_balance(catalogue, &unlocks, [paid], promoted).expect("balance");
                let bonus = if promoted { PROMOTION_ECHOES } else { 0 };
                assert_eq!(PROMOTION_ECHOES, 100);
                // Crystal adds each completed entry's `bestiaryCharmsPoints` on crossing
                // `toUnlock`; the derived balance earns the sum of completed entries' points.
                assert_eq!(balance.points_earned, spent);
                match category {
                    CharmCategory::Major => {
                        assert_eq!(balance.points_spent, spent, "{name}");
                        assert_eq!(balance.echoes_earned, echoes + bonus, "{name}");
                        assert_eq!(balance.echoes_spent, 0);
                    }
                    CharmCategory::Minor => {
                        assert_eq!(balance.points_spent, 0);
                        assert_eq!(balance.echoes_earned, bonus);
                        assert_eq!(balance.echoes_spent, spent, "{name}");
                    }
                }
            }
        }
    }
}

fn committed_hit(level: u32, own_maximum: u64, creature_max_health: u64) -> CharmHookEvent {
    CharmHookEvent::CommittedHit {
        source: CharmHitSource::CharacterAttack,
        attacker: CharmAttackerFacts {
            level,
            max_health: own_maximum,
            max_mana: own_maximum,
        },
        creature_max_health,
        health_before: 10,
        health_after: 1,
    }
}

fn amount(outcome: Option<CharmEffectOutcome>) -> u64 {
    match outcome {
        Some(
            CharmEffectOutcome::ProcDamage { amount, .. }
            | CharmEffectOutcome::AreaDamageAroundKill { amount, .. },
        ) => amount,
        None => 0,
        Some(other) => panic!("not a damage outcome: {other:?}"),
    }
}

/// Crystal `parseOffensiveCharmCombat` ceils each operand in f64; Oteryn floors exactly
/// (CHARM-4). Recorded difference: Oteryn is never above Crystal and at most one below, and
/// equal whenever the percentage divides exactly or the level cap binds.
#[test]
fn proc_damage_is_crystal_floored_at_most_one_below() {
    let imported = CanonicalCharmCatalogue::embedded().expect("embedded charms");
    let mut differences = BTreeMap::<&str, usize>::new();
    let mut cases = 0_usize;
    for (name, _, _, _, percent, _) in CRYSTAL_CHARMS {
        let Some(percent) = percent else { continue };
        let definition = imported
            .charm(&format!("oteryn:charm.{name}"))
            .expect("charm effect");
        let crystal_percent = f64::from(percent) / 10_000.0;
        for level in [1_u32, 8, 50, 137, 400, 1000] {
            for health in (1_u64..=3000).chain([9_999, 123_457, 1_000_000]) {
                let own = health * 3 + 7;
                let stage = definition.stage(3).expect("stage");
                let oteryn = amount(effect_outcome(
                    definition.effect(),
                    stage,
                    &committed_hit(level, own, health),
                ));
                let level = f64::from(level);
                let crystal = match definition.effect() {
                    CharmEffect::AttackProcDamage { .. } => (level * 2.0)
                        .ceil()
                        .min((health as f64 * crystal_percent).ceil()),
                    CharmEffect::KillAreaDamage { .. } => {
                        (health as f64 * crystal_percent).ceil().min(level * 6.0)
                    }
                    CharmEffect::AttackProcResourceDamage { resource, .. } => {
                        assert!(matches!(
                            resource,
                            CharmResource::Health | CharmResource::Mana
                        ));
                        (health as f64 * 0.08)
                            .ceil()
                            .min((own as f64 * crystal_percent).ceil())
                    }
                    other => panic!("{name} is not a damage charm: {other:?}"),
                } as u64;
                cases += 1;
                assert!(
                    oteryn <= crystal && crystal <= oteryn + 1,
                    "{name} {health}"
                );
                if oteryn != crystal {
                    *differences.entry(name).or_default() += 1;
                }
            }
        }
    }
    assert_eq!(cases, 10 * 6 * 3003);
    assert!(!differences.is_empty());
    assert!(differences.values().all(|count| *count < 6 * 3003));
}
