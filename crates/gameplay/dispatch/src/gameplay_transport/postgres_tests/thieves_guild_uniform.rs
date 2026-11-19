use super::TestResult;

#[test]
fn fishnapping_grants_key_only_with_the_equipped_officer_hat_rank_and_sex() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::thieves_guild_uniform_fixtures::run_case()
}
