use super::inquisition_coffin_recovery_fixtures::{FOREIGN_GENERATION, RESTART};
use super::{TestResult, quest_world_callbacks_fixtures};

#[test]
fn inquisition_coffin_uses_ordinary_count_for_foreign_generation_offerings() -> TestResult {
    run(FOREIGN_GENERATION)
}

#[test]
fn inquisition_coffin_recovers_paid_weakened_count_after_losing_pending_state() -> TestResult {
    run(RESTART)
}

#[test]
fn inquisition_coffin_refuses_fresh_use_after_weakened_count_dies() -> TestResult {
    run_case(RESTART, "inquisition-weakened-coffin#delivered")
}

#[test]
fn inquisition_side_coffin_refuses_fresh_use_after_weakened_count_dies() -> TestResult {
    run_case(FOREIGN_GENERATION, "inquisition-weakened-coffin-side#delivered")
}

#[test]
fn inquisition_coffins_recover_ordinary_count_after_losing_pending_state() -> TestResult {
    run_case(RESTART, "inquisition-weakened-coffin#ordinary_restart")?;
    run_case(
        FOREIGN_GENERATION,
        "inquisition-weakened-coffin-side#ordinary_restart",
    )
}

fn run(name: &'static str) -> TestResult {
    run_case(name, name)
}

fn run_case(name: &'static str, scenario: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(name, move || {
        let trigger = quest_world_callbacks_fixtures::callback(name)?
            .selector
            .position
            .ok_or("coffin")?;
        quest_world_callbacks_fixtures::scenario(scenario, trigger, false)
    })
}
