use super::TestResult;

#[test]
fn classic_katana_hidden_lever_opens_and_recloses_native_reward_door() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("katana-hidden-lever", || {
        super::quest_world_callbacks_fixtures::scenario(
            "katana-hidden-lever",
            super::classic_katana_lever_fixtures::LEVER,
            false,
        )
    })
}
