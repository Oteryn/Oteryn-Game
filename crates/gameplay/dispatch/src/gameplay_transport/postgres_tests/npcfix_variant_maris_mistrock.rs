//! Exact Crystal identity at the retained audited retail anchor.
#[test]
fn npcfix_variant_maris_mistrock_postgres_actor_identity() -> super::TestResult {
    super::npcfix_physical_endpoints::threaded_anchor((
        "oteryn:npc.maris_mistrock",
        "Maris",
        32638,
        31438,
        -7,
    ))
}
