//! Verify both retained Captain Waverider anchors without inventing a name repair.
#[test]
fn npcfix_captain_waverider_postgres_both_actor_identities() -> super::TestResult {
    for (x, y) in [(32043, 32931), (32347, 32859)] {
        super::npcfix_physical_endpoints::threaded_anchor((
            "oteryn:npc.captain_waverider",
            "Captain Waverider",
            x,
            y,
            -7,
        ))?;
    }
    Ok(())
}
