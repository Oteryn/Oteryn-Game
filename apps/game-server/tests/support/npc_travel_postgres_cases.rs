// Runs the unchanged native exercise only in the routed PostgreSQL target.
#[test]
fn npc_travel_native_postgres_replay_fences_rollback_and_reload()
-> crate::bestiary_postgres_harness::TestResult {
    crate::durability::npc_travel::npc_travel_postgres::npc_travel_native_postgres_replay_fences_rollback_and_reload()
}
