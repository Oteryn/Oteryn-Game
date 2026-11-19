use super::{TestResult, thalas_greeting_fixtures};

#[test]
fn thalas_greeting_requires_current_poison_and_advances_treasure_with_teleport() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    thalas_greeting_fixtures::thalas_greeting_requires_current_poison_and_advances_treasure_with_teleport_scenario()
}
