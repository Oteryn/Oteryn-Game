//! Real command6116 read projection plus unchanged paid writer/replay and fencing proof.
use super::{
    TestResult,
    item_forge_fixtures::support::{self, Case},
};

#[test]
fn forge_preview_postgres_coherent_quote_readonly_paid_replay_and_stale_fence() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("forge-preview", || {
        support::scenario(Case::Preview)
    })
}

#[test]
fn forge_browse_activity_holder_refusal_decodes_and_replays_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("forge-browse-busy", || {
        support::scenario(Case::PreviewBusyHolder)
    })
}
