//! W22a: the real auto-attack kill drains large and nested loot plans through SQL custody guards.
use super::w22a_loot_fixtures::{Case};

use super::TestResult;

use super::kill_reward_e2e_fixtures::{scenario, scenario_thread};


use super::w22a_loot_fixtures::CASE;

#[test]
fn w22a_configured_loot_rate_scales_the_table_used_by_death_settlement() -> TestResult {
    // Crystal 201189f8 `data/libs/functions/functions.lua:68-70`: RATE_LOOT multiplies the
    // effective chance. This is the same helper called by kill_reward.rs before PG settlement.
    let mut table = crate::combat::LootTableDefinition {
        algorithm: crate::combat::LootSelectionAlgorithm::IndependentBernoulliPpm,
        entries: vec![crate::combat::LootTableEntry {
            item: crate::combat::LootDefinitionRef::new(
                "Item",
                "oteryn:item.tibia.i3031",
                "definition-r1",
            ),
            min_count: 1,
            max_count: 1,
            probability_ppm: Some(400_000),
            details: Default::default(),
        }],
    };
    oteryn_gameplay_items::loot_plan::apply_configured_loot_rate(&mut table, 200);
    assert_eq!(table.entries[0].probability_ppm, Some(800_000));
    Ok(())
}

#[test]
fn w22a_more_than_sixteen_entries_drop_into_the_kill_corpse() -> TestResult {
    scenario_thread::run("w22a-large", || {
        CASE.set(Some(Case::Large));
        scenario()
    })
}
#[test]
fn w22a_nested_container_drop_mints_its_children_inside_the_container() -> TestResult {
    scenario_thread::run("w22a-nested", || {
        CASE.set(Some(Case::Nested));
        scenario()
    })
}
#[test]
fn w22a_charged_loot_with_a_subtype_mints_with_its_full_charges() -> TestResult {
    scenario_thread::run("w22a-charged-subtype", || {
        CASE.set(Some(Case::ChargedSubtype));
        scenario()
    })
}
#[test]
fn w22a_fluid_loot_with_a_subtype_mints_the_filled_container() -> TestResult {
    scenario_thread::run("w22a-fluid-subtype", || {
        CASE.set(Some(Case::FluidSubtype));
        scenario()
    })
}
#[test]
fn w22b_gut_widens_creature_product_rolls_on_the_real_kill_path() -> TestResult {
    scenario_thread::run("w22b-gut", || {
        CASE.set(Some(Case::Gut));
        scenario()
    })
}
#[test]
fn w22c_wealth_duplex_rolls_the_kill_loot_once_more() -> TestResult {
    scenario_thread::run("w22c-duplex-on", || {
        CASE.set(Some(Case::DuplexOn));
        scenario()
    })
}
#[test]
fn w22c_a_kill_without_wealth_duplex_rolls_its_loot_once() -> TestResult {
    scenario_thread::run("w22c-duplex-off", || {
        CASE.set(Some(Case::DuplexOff));
        scenario()
    })
}
#[test]
fn w22b_wealth_duplex_ignores_wall_clock_time_spent_offline() -> TestResult {
    scenario_thread::run("w22b-duplex-offline", || {
        CASE.set(Some(Case::DuplexOfflineIgnored));
        scenario()
    })
}
#[test]
fn w22b_wealth_duplex_ends_after_one_online_hour() -> TestResult {
    scenario_thread::run("w22b-duplex-online-spent", || {
        CASE.set(Some(Case::DuplexOnlineSpent));
        scenario()
    })
}
#[test]
fn w22a_mint_into_a_container_outside_the_kill_corpse_is_refused() -> TestResult {
    scenario_thread::run("w22a-outside", || {
        CASE.set(Some(Case::Outside));
        scenario()
    })
}
