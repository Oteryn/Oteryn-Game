//! Native Grizzly progression for the three selected task/boss pairs.
//! The chosen campaign adds its own once-only 10,000 XP and 5,000 gold completion.
//! No task counters, boss victories, points or quest receipts are seeded.

use super::super::{TestResult, grizzly_npc_fixtures::scenario};
#[test]
fn questpc8_hunting_campaign_native_deaths_portals_reports_and_rewards() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::super::kill_reward_e2e_fixtures::scenario_thread::run("questpc8-hunting-campaign", || {
        scenario(5)
    })?;
    println!(
        "POSTGRES_CASE_COMPLETED questpc8_hunting_campaign_native_deaths_portals_reports_and_rewards duration={:?}",
        started.elapsed()
    );
    Ok(())
}
