//! Reuses the last-position owner's harness and record capture; mounted once there.
use super::*;
use crate::durability::character_tutorial::TutorialProgressOutcome;
use oteryn_gameplay_quests::tutorial::{ISLET_TEMPLE, TutorialProgress, TutorialStep};

#[test]
fn tutorial_islet_placement_survives_restart_and_consumes_only_after_placement() -> TestResult {
    configured_admin().ok_or("tutorial placement requires routed PostgreSQL")?;
    run("tutorial_islet_placement", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let progress = TutorialProgress {
            step: TutorialStep::Islet,
            hunting: false,
            corruptor: false,
        };
        assert!(
            matches!(h.root.character_tutorial_progress(&authority, &h.node, fence(1)?, Some((0, progress))).await.map_err(debug)?, TutorialProgressOutcome::Written(stored) if stored.pending_islet)
        );
        drop(authority);
        let reopened = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(reopened.maintain_ready_once().await?);
        let authority = reopened
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        assert!(
            matches!(reopened.character_tutorial_progress(&authority, &h.node, fence(1)?, None).await.map_err(debug)?, TutorialProgressOutcome::Read(Some(stored)) if stored.pending_islet)
        );
        let mut placed = FinalLastPosition {
            record: record(ISLET_TEMPLE.0),
            write_sequence: 10,
        };
        placed.record.cell = LastPositionCell {
            x: ISLET_TEMPLE.0,
            y: ISLET_TEMPLE.1,
            floor: ISLET_TEMPLE.2,
        };
        let mut stale = fence(1)?;
        stale.connection_generation = ConnectionGeneration::new(2).map_err(debug)?;
        assert!(
            reopened
                .complete_tutorial_islet_placement(&authority, &h.node, stale, placed)
                .await
                .is_err()
        );
        assert_eq!(row(h).await?, None);
        let mut wrong = placed;
        wrong.record.cell.x += 20;
        assert!(
            reopened
                .complete_tutorial_islet_placement(&authority, &h.node, fence(1)?, wrong)
                .await
                .is_err()
        );
        assert!(
            reopened
                .complete_tutorial_islet_placement(&authority, &h.node, fence(1)?, placed)
                .await
                .map_err(debug)?
        );
        assert!(
            !reopened
                .complete_tutorial_islet_placement(&authority, &h.node, fence(1)?, placed)
                .await
                .map_err(debug)?
        );
        assert!(
            matches!(reopened.character_tutorial_progress(&authority, &h.node, fence(1)?, None).await.map_err(debug)?, TutorialProgressOutcome::Read(Some(stored)) if !stored.pending_islet && stored.version == 2 && stored.progress == progress)
        );
        assert_eq!(
            reopened
                .read_character_last_position(&authority, &h.node, fence(1)?)
                .await
                .map_err(debug)?
                .ok_or("missing placed position")?
                .record,
            placed.record
        );
        assert_eq!(h.root_revision().await?, "1");
        Ok(())
    })
}
