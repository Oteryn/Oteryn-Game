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

fn run(name: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(name, move || {
        let trigger = quest_world_callbacks_fixtures::callback(name)?
            .selector
            .position
            .ok_or("coffin")?;
        quest_world_callbacks_fixtures::scenario(name, trigger, false)
    })
}
