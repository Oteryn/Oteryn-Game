use super::TestResult;

#[test]
fn slice2_ordinary_death_schedules_exact_source_corpse_decay_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    super::qengine_rewards_fixtures::scenario(super::ordinary_corpse_slice2_fixtures::NAME)
}
