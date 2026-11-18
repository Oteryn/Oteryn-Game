//! NX-8 P19: a current balance read is independent of retained mutation history.
use super::{TestResult, item_forge_fixtures::support};

#[test]
fn nx8_forge_postgres_current_dust_without_history_and_generation_fence() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("nx8-current-dust", || {
        support::scenario(support::Case::CurrentDust)
    })
}
