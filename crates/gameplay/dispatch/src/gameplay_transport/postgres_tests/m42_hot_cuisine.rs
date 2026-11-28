//! M42 proves the existing cooking writer against exact native fluid custody.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn m42_utc_cooking_refuses_wrong_fluid_and_commits_exact_typed_recipe() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("m42-hot-typed", || {
        super::quest_world_callbacks_fixtures::scenario(
            "m42-hot-typed",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    });
    eprintln!(
        "M42_HOT POSTGRES_CASE_COMPLETED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn m42_final_menu_and_culinary_master_use_real_typed_receipts() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("m42-hot-final", || {
        super::quest_world_callbacks_fixtures::scenario(
            "m42-hot-final",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    });
    eprintln!(
        "M42_HOT_FINAL POSTGRES_CASE_COMPLETED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn m42_upstairs_cookbooks_require_the_door_and_mint_readable_copies() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("m42-hot-cookbooks", || {
        super::quest_world_callbacks_fixtures::scenario(
            "m42-hot-cookbooks",
            Position { x: 33066, y: 32529, floor: -5 },
            false,
        )
    })
}
