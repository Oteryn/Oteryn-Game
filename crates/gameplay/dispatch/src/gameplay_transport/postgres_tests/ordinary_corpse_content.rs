use super::TestResult;

#[test]
fn ordinary_corpse_content_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    super::qengine_rewards_fixtures::scenario(super::ordinary_corpse_content_fixtures::NAME)
}
