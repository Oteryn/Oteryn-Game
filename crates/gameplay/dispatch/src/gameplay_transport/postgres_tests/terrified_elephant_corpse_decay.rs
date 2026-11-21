//! Native Terrified Elephant death schedules the complete source corpse chain.
use super::TestResult;

#[test]
fn terrified_elephant_corpse_decay_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    super::qengine_rewards_fixtures::scenario(super::terrified_elephant_corpse_decay_fixtures::NAME)
}
