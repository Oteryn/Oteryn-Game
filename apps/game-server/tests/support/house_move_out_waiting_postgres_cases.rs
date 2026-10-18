//! NOT MOUNTED: D1349 gate #2137 + #2140, real composed House access owner.
//! The earlier case replaced game_house_access locally. That substitute is removed;
//! qualify this case only after installing the actual owner schemas/current fixture.
//! These retained assertions exercise real commit/census behavior once that seam lands.
use super::*;
use crate::durability::house_scope_handoff::{HouseCommitOutcome, HouseEntryCommit};

#[test]
fn actual_committed_house_session_blocks_after_reservation_is_released() -> TestResult {
    run("hm_canonical", async |h| {
        setup(h, 1).await?;
        past_notice(h).await?;
        prepare_entry(h).await?;
        // Requires the real composed game_house_access from #2137 and #2140.
        // Its owner schemas/current fixture must be installed when this case is mounted.
        // No test-only replacement of the native access owner is permitted.
        let seal = h.recovery.seal_current().map_err(debug)?;
        let auth = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        assert!(matches!(
            h.root
                .commit_house_entry(
                    &auth,
                    &h.node,
                    HouseEntryCommit {
                        handoff_id: id(60),
                        destination_game_session_id: GameSessionId::decode(&id(61))
                            .map_err(debug)?,
                        committed_at: 200
                    }
                )
                .await
                .map_err(debug)?,
            HouseCommitOutcome::Committed(_)
        ));
        apply(h, freeze()?).await.map_err(debug)?;
        assert!(matches!(apply(h, step(0)?).await, Err(Error::Occupied)));
        // Boundary-specific negative operator: trusted DB-owner fixture releases just the
        // retained reservation, with native transition guards active. The actual canonical
        // house session produced above remains live. This is not an exit/placement claim.
        sqlx::query("UPDATE game_house_scope_handoffs SET state=3,released_at=201 WHERE handoff_id=encode($1,'hex')::uuid").bind(id(60).as_slice()).execute(&h.pool).await?;
        let reservations: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_house_scope_handoffs WHERE state IN(1,2)",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(reservations, 0);
        assert!(matches!(apply(h, step(0)?).await, Err(Error::Occupied)));
        assert_eq!(counts(h).await?, (1, 1, 0, 0));
        Ok(())
    })
}
