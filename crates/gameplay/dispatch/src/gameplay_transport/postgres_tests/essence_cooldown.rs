use super::{TestResult, essence_cooldown_fixtures};

#[test]
fn essence_staging_portal_returns_character_until_boss_cooldown_expires() -> TestResult {
    essence_cooldown_fixtures::essence_staging_portal_returns_character_until_boss_cooldown_expires_scenario()
}
