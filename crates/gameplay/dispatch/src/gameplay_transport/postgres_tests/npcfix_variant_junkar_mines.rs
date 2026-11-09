//! Exact Crystal identity at the retained audited retail anchor.
#[test]
fn npcfix_variant_junkar_mines_postgres_actor_identity() -> super::TestResult {
    super::npcfix_physical_endpoints::threaded_anchor((
        "oteryn:npc.junkar_mines",
        "Junkar",
        32555,
        32068,
        -10,
    ))
}
