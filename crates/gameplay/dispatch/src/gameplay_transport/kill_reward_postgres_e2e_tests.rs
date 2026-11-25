//! Kill → reward end to end on PostgreSQL: a held auto-attack target is driven by the real
//! `drain_auto_attacks` to a lethal swing, which queues the kill, and the real
//! `drain_kill_rewards` settles it durably (XP row and a linked corpse with its loot) through
//! the composed owner. The only fabricated inputs are the clock (`clock_origin` in the past, then
//! advanced between swings) and the session fixture; the content is the committed production
//! manifest's generation (the production rat: 5 XP, `oteryn:loot.creature.rat`).
//!
//! Nested under the monk-save PostgreSQL tests to reuse their fresh-admission fixture.
#![allow(clippy::expect_used, clippy::unwrap_used)]
use super::kill_reward_e2e_fixtures::{
    scenario, scenario_thread, scenario_with_charms, scenario_with_creature,
    scenario_with_creature_at_range, scenario_with_creature_charmed,
};

use super::TestResult;
use super::fixture::{debug, id, run};

#[test]
fn auto_attack_kill_settles_xp_and_a_linked_corpse_with_loot_on_postgres() -> TestResult {
    let started = std::time::Instant::now();
    let result = scenario_thread::run("kill-reward-e2e", scenario);
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_some() {
        eprintln!(
            "CHANNEL_NATIVE_CORPSE POSTGRES_CASE_COMPLETED elapsed_ms={} success={}",
            started.elapsed().as_millis(),
            result.is_ok()
        );
    }
    result
}

#[test]
fn accepted_auto_attack_damages_and_mints_corpse_when_bounty_read_is_blocked() -> TestResult {
    scenario_thread::run(
        "kill-reward-blocked-bounty",
        super::kill_reward_e2e_fixtures::scenario_with_blocked_bounty_read,
    )
}

#[test]
fn charm_wire_missing_snapshot_preserves_combat_and_assignment_changes_damage_and_dodge()
-> TestResult {
    scenario_thread::run("charm-wire-combat", || {
        let mut plain = Vec::new();
        let mut wound = Vec::new();
        let mut dodge = Vec::new();
        let mut parry = Vec::new();
        scenario_with_charms(None, Some(("", &mut plain)))?;
        scenario_with_charms(None, Some(("wound", &mut wound)))?;
        scenario_with_charms(None, Some(("dodge", &mut dodge)))?;
        scenario_with_charms(None, Some(("parry", &mut parry)))?;
        // Route-less execution is handled by the shared harness; it supplies no samples.
        if plain.is_empty() {
            return Ok(());
        }
        assert_eq!(wound.len(), plain.len());
        assert_eq!(dodge.len(), plain.len());
        assert_eq!(parry.len(), plain.len());
        assert!(
            plain.iter().any(|(outgoing, _)| *outgoing > 0),
            "no snapshot still deals damage: {plain:?}"
        );
        assert!(
            plain.iter().any(|(_, incoming)| *incoming > 0),
            "no snapshot still takes damage: {plain:?}"
        );
        assert!(
            wound.iter().zip(&plain).any(|(a, b)| a.0 > b.0),
            "assigned Wound adds damage"
        );
        assert!(
            dodge.iter().zip(&plain).any(|(a, b)| a.1 < b.1),
            "assigned Dodge cancels damage: dodge={dodge:?}, plain={plain:?}"
        );
        assert!(
            parry.iter().zip(&plain).any(|(a, b)| a.0 > b.0),
            "assigned Parry reflects creature damage"
        );
        assert!(wound.iter().zip(&plain).all(|(a, b)| a.0 >= b.0));
        assert!(dodge.iter().zip(&plain).all(|(a, b)| a.1 <= b.1));
        Ok(())
    })
}

#[test]
fn summer_radiant_all_six_execute_attacks_through_the_production_owner() -> TestResult {
    scenario_thread::run("summer-radiant-attacks", || {
        for name in [
            "acolyte",
            "inquisitor",
            "paragon",
            "templar",
            "warden",
            "zealot",
        ] {
            scenario_with_creature(None, false, Some(name), false)?;
        }
        Ok(())
    })
}

#[test]
fn summer_acolyte_physical_spell_hits_at_range_through_the_production_owner() -> TestResult {
    scenario_thread::run("summer-acolyte-physical-spell", || {
        scenario_with_creature_at_range(None, false, Some("acolyte"), false, true, false)
    })
}

#[test]
fn summer_acolyte_ice_area_hits_multiple_players_through_the_production_owner() -> TestResult {
    scenario_thread::run("summer-acolyte-ice-area", || {
        scenario_with_creature_at_range(None, false, Some("acolyte"), false, true, true)
    })
}

#[test]
fn summer_paragon_holy_target_hits_at_range_through_the_production_owner() -> TestResult {
    scenario_thread::run("summer-paragon-holy-target", || {
        scenario_with_creature_at_range(None, false, Some("paragon"), false, true, false)
    })
}

#[test]
fn summer_paragon_fire_area_hits_multiple_players_through_the_production_owner() -> TestResult {
    scenario_thread::run("summer-paragon-fire-area", || {
        scenario_with_creature_at_range(None, false, Some("paragon"), false, true, true)
    })
}

#[test]
fn summer_templar_energy_area_hits_multiple_players_through_the_production_owner() -> TestResult {
    scenario_thread::run("summer-templar-energy-area", || {
        scenario_with_creature_at_range(None, false, Some("templar"), false, true, true)
    })
}

#[test]
fn summer_templar_holy_target_hits_at_range_through_the_production_owner() -> TestResult {
    scenario_thread::run("summer-templar-holy-target", || {
        scenario_with_creature_at_range(None, false, Some("templar"), false, true, false)
    })
}

#[test]
fn summer_radiant_all_six_kills_settle_and_decay_on_postgres() -> TestResult {
    scenario_thread::run("summer-radiant-settlements", || {
        for name in [
            "acolyte",
            "inquisitor",
            "paragon",
            "templar",
            "warden",
            "zealot",
        ] {
            scenario_with_creature(None, false, Some(name), true)?;
        }
        Ok(())
    })
}

#[test]
fn summer_corpse_stages_recover_under_a_new_process_and_owner_generation() -> TestResult {
    scenario_thread::run("summer-corpse-handoff", || {
        scenario_with_creature(Some(true), false, Some("warden"), true)
    })
}

#[test]
fn boss_spawn1_respawn_death_and_replay_through_real_dispatch_postgres() -> TestResult {
    scenario_thread::run("boss-spawn1-respawn", || {
        scenario_with_creature_charmed(
            None,
            false,
            None,
            false,
            false,
            false,
            (
                None,
                Some(oteryn_gameplay_content::content::BossPolicy::Respawn {
                    min_s: 30,
                    max_s: 60,
                }),
                None,
            ),
        )
    })
}

#[test]
fn boss_spawn1_at_reset_death_and_replay_through_real_dispatch_postgres() -> TestResult {
    scenario_thread::run("boss-spawn1-at-reset", || {
        scenario_with_creature_charmed(
            None,
            false,
            None,
            false,
            false,
            false,
            (
                None,
                Some(oteryn_gameplay_content::content::BossPolicy::AtReset {
                    chance: oteryn_gameplay_content::content::BossChance {
                        numerator: 1,
                        denominator: 1,
                    },
                }),
                None,
            ),
        )
    })
}

#[test]
fn w13_boosted_boss_kill_rerolls_equipment_but_not_its_unique_drop_postgres() -> TestResult {
    scenario_thread::run("w13-boosted-boss-unique", || {
        super::w22a_loot_fixtures::boosted_boss_unique_case();
        scenario_with_creature_charmed(
            None,
            false,
            None,
            false,
            false,
            false,
            (None, None, Some("boss")),
        )
    })
}

#[test]
fn daily_boost_real_kill_xp_loot_and_unselected_race_postgres() -> TestResult {
    scenario_thread::run("daily-boost-rewards", || {
        for target in ["creature", "boss", "other"] {
            scenario_with_creature_charmed(
                None,
                false,
                None,
                false,
                false,
                false,
                (None, None, Some(target)),
            )?;
        }
        Ok(())
    })
}

#[path = "kill_reward_postgres_fixture/summer_settlement.rs"]
mod summer_settlement;
