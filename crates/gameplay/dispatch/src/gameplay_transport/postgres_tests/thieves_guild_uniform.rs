use super::TestResult;

#[test]
fn fishnapping_grants_key_only_with_the_equipped_officer_hat_rank_and_sex() -> TestResult {
    super::thieves_guild_uniform_fixtures::run_case()
}
