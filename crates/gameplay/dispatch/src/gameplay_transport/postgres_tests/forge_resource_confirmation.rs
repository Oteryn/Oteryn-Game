//! Additive Resource quote variants exercise actual6116 dispatch against the existing fenced fixture.
use super::{
    TestResult,
    item_forge_fixtures::support::{self, Case},
};

#[test]
fn forge_resource_confirmation_postgres_quotes_are_real_and_readonly() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("forge-resource-confirmation", || {
        support::scenario(Case::Preview)
    })
}
