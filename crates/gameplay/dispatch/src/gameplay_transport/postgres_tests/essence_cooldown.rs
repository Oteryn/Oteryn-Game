use super::{TestResult, essence_cooldown_fixtures};

#[test]
fn essence_staging_portal_returns_character_until_boss_cooldown_expires() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    essence_cooldown_fixtures::essence_staging_portal_returns_character_until_boss_cooldown_expires_scenario()
}
