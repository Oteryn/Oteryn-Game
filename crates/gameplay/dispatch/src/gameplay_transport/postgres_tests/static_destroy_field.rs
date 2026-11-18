//! Crystal 201189f8 data/scripts/runes/destroy_field_rune.lua:14-20.
use super::TestResult;

#[test]
fn destroy_field_rune_removes_the_static_fortress_field_and_spends_one_charge() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run_async(async move |_| {
        super::field_runes::static_field_case(admin).await
    })
}
