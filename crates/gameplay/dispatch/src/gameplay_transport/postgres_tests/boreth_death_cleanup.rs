use super::TestResult;
use super::boreth_death_cleanup_fixtures::{NAME, UNBOUND, run};

#[test]
fn boreth_projected_death_removes_only_its_adds_inside_the_source_rectangle() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run(NAME)
}

#[test]
fn boreth_projected_death_leaves_adds_alone_without_a_binding() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run(UNBOUND)
}
