use super::{TestResult, item_look_fixtures as item_look};

#[test]
fn fenced_look_dispatch_shows_crystal_arena_goblet_inscriptions() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(
        !admin.is_empty(),
        "PostgreSQL route must be nonempty when configured"
    );
    item_look::large_stack(item_look::scenario_with_goblets)
}
