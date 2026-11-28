//! Lane qh: registered authored bindings driven by literal USE on an unmodified native map crop.
use super::qh_authored_cases_fixtures::{
    AMBUSH, BARE, DIG, DIRT, NO_PICK, SMALL_SAPPHIRE, TOP_OF_THE_CITY, TOP_OF_THE_CITY_CRATE,
    WRONG_TOOL,
};
use super::{TestResult, quest_world_callbacks_fixtures};
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn qh_top_of_the_city_pick_digs_into_tunnel_postgres() -> TestResult {
    run_dig(DIG)
}

#[test]
fn qh_top_of_the_city_bare_dirt_use_refuses_postgres() -> TestResult {
    run_dig(BARE)
}

#[test]
fn qh_top_of_the_city_wrong_tool_refuses_postgres() -> TestResult {
    run_dig(WRONG_TOOL)
}

#[test]
fn qh_top_of_the_city_pick_use_without_pick_refuses_postgres() -> TestResult {
    run_dig(NO_PICK)
}

fn run_dig(name: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run(name, move || {
        quest_world_callbacks_fixtures::scenario(name, DIRT, false)
    });
    eprintln!(
        "QH_POSTGRES_CASE {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn qh_small_sapphire_premium_chest_mints_bag_once_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("qh-small-sapphire", || {
        // The scenario starts one tile west of this position, beside the chest.
        quest_world_callbacks_fixtures::scenario(
            SMALL_SAPPHIRE,
            Position {
                x: 32963,
                y: 32430,
                floor: -7,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED qh_small_sapphire_premium_chest_mints_bag_once_postgres duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn qh_top_of_the_city_npc_chain_and_cellar_crate_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("qh-top-of-the-city", || {
        // The story starts at Iriana; the trigger below identifies the cellar crate.
        quest_world_callbacks_fixtures::scenario(TOP_OF_THE_CITY, TOP_OF_THE_CITY_CRATE, false)
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED qh_top_of_the_city_npc_chain_and_cellar_crate_postgres duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn qh_jimmy_greeting_requires_three_assassin_deaths_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(AMBUSH, || {
        quest_world_callbacks_fixtures::scenario(AMBUSH, TOP_OF_THE_CITY_CRATE, false)
    })
}
