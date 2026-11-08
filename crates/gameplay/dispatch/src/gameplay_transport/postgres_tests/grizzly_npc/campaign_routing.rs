//! Reward refusal must precede the legacy Grizzly campaign explanation.
use super::*;

#[test]
fn questpc8_campaign_keyword_uses_reward_refusal_before_grizzly_dialogue() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED questpc8-campaign-routing");
    super::super::kill_reward_e2e::scenario_thread::run("questpc8-campaign-routing", || scenario(7))?;
    eprintln!("POSTGRES_CASE_COMPLETED questpc8-campaign-routing duration={:?}", started.elapsed());
    Ok(())
}
