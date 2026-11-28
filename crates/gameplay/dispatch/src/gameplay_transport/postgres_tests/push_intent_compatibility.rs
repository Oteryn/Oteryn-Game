use super::TestResult;
use super::creature_push_fixtures::{LegacyPushCase, legacy_push_scenario};

#[test]
fn push_intent_15_moves_and_shares_cooldown_with_command_98_postgres() -> TestResult {
    run(LegacyPushCase::Selected)
}

#[test]
fn push_intent_15_requires_capability_11_postgres() -> TestResult {
    run(LegacyPushCase::Unselected)
}

fn run(case: LegacyPushCase) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || legacy_push_scenario(case))?
        .join()
        .map_err(|_| "legacy push scenario panicked")?
        .map_err(Into::into)
}
