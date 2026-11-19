//! Existing admitted metadata and real fenced6116 dispatcher; outer fixture compares all assets.
use super::{
    TestResult,
    item_forge_fixtures::support::{self, Case},
};

#[test]
fn exaltation_postgres_real_effect_descriptions_are_readonly_and_handle_fenced() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("exaltation", || support::scenario(Case::Preview))
}
