//! PVP-1b expiry through the production composed Channel owner.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};

#[test]
fn pvp_world_cleanup_uses_the_current_composed_channel_owner() -> TestResult {
    run(async |admin| {
        reset();
        let harness =
            Harness::create_with_admission_seed(admin, "pvp_world_cycle", true, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let owner = dependencies.owner(&harness, &authority);
        // Seed an already-expired state. Production cleanup must use the real
        // scope/recovery/node proofs, no privileged cleanup or invented session.
        sqlx::query(
            "INSERT INTO game_character_pvp_state (character_id, world_id, skull, skull_until) \
             VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, clock_timestamp() - interval '1 second')",
        ).bind(id(41).as_slice()).bind(id(42).as_slice()).execute(&harness.pool).await?;
        let cleaned = owner
            .drain_source_pvp_expiry()
            .await
            .ok_or("current owner cleanup refused")?;
        assert_eq!(
            (
                cleaned.skulls,
                cleaned.points,
                cleaned.marks,
                cleaned.ledger_rows
            ),
            (1, 0, 0, 0)
        );
        assert_eq!(
            owner
                .drain_source_pvp_expiry()
                .await
                .ok_or("idempotent pass refused")?
                .skulls,
            0
        );
        let normalized: bool = sqlx::query_scalar("SELECT skull = 0 AND skull_until IS NULL AND revision = 2 FROM game_character_pvp_state")
            .fetch_one(&harness.pool).await?;
        assert!(normalized);
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
