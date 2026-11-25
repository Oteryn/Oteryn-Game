use super::{
    TestResult, qengine_rewards_fixtures, quest_world_callbacks_fixtures as fixture,
    srvenc_source_deaths_fixtures::scenario,
};

#[test]
fn black_bert_sells_dust_of_arthei_for_exactly_four_thousand() -> TestResult {
    qengine_rewards_fixtures::scenario("qengine-rewards-black-bert-arthei")
}

#[test]
fn storkus_accepts_deposits_and_confirms_in_separate_steps() -> TestResult {
    qengine_rewards_fixtures::scenario("qengine-rewards-storkus-dust")
}

#[test]
fn nomad_ignores_shared_xp_disabled_in_the_world_owner() -> TestResult {
    scenario(
        stringify!(nomad_ignores_shared_xp_disabled_in_the_world_owner),
        "nomad",
    )
}

#[test]
fn inquisition_coffin_does_not_replace_a_live_count_after_losing_local_reservation() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("inquisition-live-count", || {
        let trigger = fixture::callback("inquisition-weakened-coffin")?
            .selector
            .position
            .ok_or("coffin")?;
        fixture::scenario(
            "inquisition-weakened-coffin#live_without_reservation",
            trigger,
            false,
        )
    })
}
