// Runs the unchanged native exercise only in the routed PostgreSQL target.
#[test]
fn npc_travel_native_postgres_replay_fences_rollback_and_reload()
-> crate::bestiary_postgres_harness::TestResult {
    // The canonical helper marks absent configuration PRE-ROUTING / NONCANONICAL.
    // Only the unchanged exercise below can produce the PostgreSQL completion marker.
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    crate::durability_npc_travel_postgres_fixture::npc_travel_native_postgres_replay_fences_rollback_and_reload()
}
