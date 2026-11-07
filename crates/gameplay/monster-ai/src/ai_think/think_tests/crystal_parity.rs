//! W01 (Crystal re-audit 2026-10-28): the production native creature records, projected by
//! `native_profile::project_native_record` and run by the think engine, against Crystal
//! `201189f8` `data-crystal/monster/**.lua` and `Monster::isFleeing` (`monster.cpp:1001`), which
//! Canary `47dfd51f` repeats (`monster.cpp:1460`).

use super::super::native_profile::project_native_record;
use super::{Facts, at, player, rat, think};
use crate::ai_think::CreatureAiState;
use crate::content::native_gameplay::{CreatureProfileRecord, CreatureProfilesDocument};
use crate::content::{ProjectV2ChangeTarget, ProjectV2StrategyWeights};

fn production_records() -> Vec<CreatureProfileRecord> {
    let document: CreatureProfilesDocument = serde_json::from_slice(include_bytes!(
        "../../../../../../content/creatures/definitions/spell-native-profiles.json"
    ))
    .expect("production native creature profiles");
    document.records
}

fn record<'a>(records: &'a [CreatureProfileRecord], key: &str) -> &'a CreatureProfileRecord {
    records
        .iter()
        .find(|record| record.profile.target.key == format!("oteryn:creature.{key}"))
        .expect("production creature record")
}

/// One Crystal monster file's targeting block.
struct CrystalTargeting {
    key: &'static str,
    run_health: u64,
    change_target: (u64, u32),
    strategies: (u32, u32, u32, u32),
    static_attack_percent: u32,
    target_distance: u16,
    /// `{ name = "melee", interval, chance, minDamage = 0, maxDamage = -max }`.
    melee: (u64, u32, u32),
}

const CRYSTAL: [CrystalTargeting; 3] = [
    // mammals/rat.lua:38-59, :84
    CrystalTargeting {
        key: "rat",
        run_health: 5,
        change_target: (4000, 0),
        strategies: (100, 0, 0, 0),
        static_attack_percent: 90,
        target_distance: 1,
        melee: (2000, 100, 8),
    },
    // mammals/cave_rat.lua:38-59, :87
    CrystalTargeting {
        key: "cave_rat",
        run_health: 3,
        change_target: (4000, 0),
        strategies: (100, 0, 0, 0),
        static_attack_percent: 90,
        target_distance: 1,
        melee: (2000, 100, 10),
    },
    // dragons/dragon.lua:43-67, :112
    CrystalTargeting {
        key: "dragon",
        run_health: 300,
        change_target: (4000, 10),
        strategies: (70, 10, 10, 10),
        static_attack_percent: 80,
        target_distance: 1,
        melee: (2000, 100, 120),
    },
];

#[test]
fn production_targeting_matches_crystal_monster_files() {
    let records = production_records();
    for crystal in &CRYSTAL {
        let profile = project_native_record(record(&records, crystal.key)).expect("projects");
        let targeting = profile.targeting();
        assert_eq!(targeting.flee_health, crystal.run_health, "{}", crystal.key);
        assert_eq!(
            targeting.change_target,
            Some(ProjectV2ChangeTarget {
                interval_ms: crystal.change_target.0,
                chance_ppm: crystal.change_target.1 * 10_000,
            }),
            "{}",
            crystal.key
        );
        let (nearest, damage, health, random) = crystal.strategies;
        assert_eq!(
            targeting.strategy_weights,
            Some(ProjectV2StrategyWeights {
                nearest,
                damage,
                health,
                random,
            }),
            "{}",
            crystal.key
        );
        assert_eq!(
            targeting.static_attack_chance_ppm,
            crystal.static_attack_percent * 10_000,
            "{}",
            crystal.key
        );
        assert_eq!(
            targeting.target_distance_tiles, crystal.target_distance,
            "{}",
            crystal.key
        );
    }
}

#[test]
fn production_melee_matches_crystal_where_qualified_and_the_rat_has_none() {
    let records = production_records();
    for crystal in &CRYSTAL {
        let melee = record(&records, crystal.key).monster_melee.as_ref();
        let (interval_ms, chance_percent, maximum) = crystal.melee;
        if crystal.key == "rat" {
            // DIFF (recorded): Crystal's rat bites 0..8 every 2000 ms; the production rat
            // record carries no qualified melee binding, so wild melee is disabled for it.
            assert!(melee.is_none());
            continue;
        }
        let melee = melee.expect("qualified melee");
        assert_eq!(
            (
                melee.interval_ms,
                melee.chance_ppm,
                melee.minimum,
                melee.maximum
            ),
            (interval_ms, chance_percent * 10_000, 0, maximum),
            "{}",
            crystal.key
        );
    }
}

#[test]
fn production_rat_flees_at_crystal_run_health_and_not_above() {
    // `isFleeing`: `getHealth() <= runAwayHealth` (no challenge, no fatal hold, not a summon).
    let profile =
        project_native_record(record(&production_records(), "rat")).expect("rat projects");
    let facts = Facts::new();
    let players = [player(10, at(102, 100))];
    for (health, fleeing) in [(1, true), (5, true), (6, false), (20, false)] {
        let mut state = CreatureAiState::new(rat(), profile.clone(), at(100, 100));
        let report = think(&mut state, 1, &facts.at(at(100, 100), health, &players, 0));
        assert_eq!(report.fleeing, fleeing, "health {health}");
    }
}

#[test]
fn production_rat_never_changes_target_with_crystal_chance_zero() {
    let profile =
        project_native_record(record(&production_records(), "rat")).expect("rat projects");
    let facts = Facts::new();
    let players = [player(10, at(101, 100)), player(11, at(99, 100))];
    let mut state = CreatureAiState::new(rat(), profile, at(100, 100));
    for sequence in 1..=12 {
        let report = think(
            &mut state,
            sequence,
            &facts.at(at(100, 100), 20, &players, sequence * 1_000_000),
        );
        assert!(!report.target_changed);
        assert!(report.target.is_some());
    }
}
