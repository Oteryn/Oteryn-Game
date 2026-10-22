//! Tutorial CAS, replay, liveness and reload against the production SQLx migration.
use crate::bestiary_postgres_harness::{
    Harness, TestResult, configured_admin, debug, fence, runtime,
};
use crate::durability::character_tutorial::TutorialProgressOutcome;
use oteryn_gameplay_quests::tutorial::{TutorialProgress, TutorialStep};

#[test]
fn tutorial_progress_postgres_cas_conflicts_even_for_equal_state_and_reloads() -> TestResult {
    let admin = configured_admin().ok_or("tutorial progress requires routed PostgreSQL")?;
    runtime()?.block_on(async {
        let h = Harness::create(admin, "tutorial_cas", true).await?;
        let result = async {
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
            let read = h
                .root
                .character_tutorial_progress(&authority, &h.node, fence(1)?, None)
                .await
                .map_err(debug)?;
            assert_eq!(read, TutorialProgressOutcome::Read(None));
            let first = h
                .root
                .character_tutorial_progress(&authority, &h.node, fence(1)?, Some((0, progress)))
                .await
                .map_err(debug)?;
            let TutorialProgressOutcome::Written(stored) = first else {
                return Err("first CAS did not write".into());
            };
            assert_eq!(stored.version, 1);
            assert_eq!(
                h.root
                    .character_tutorial_progress(
                        &authority,
                        &h.node,
                        fence(1)?,
                        Some((0, progress))
                    )
                    .await
                    .map_err(debug)?,
                TutorialProgressOutcome::Conflict(Some(stored))
            );
            assert_eq!(
                h.root
                    .character_tutorial_progress(
                        &authority,
                        &h.node,
                        fence(1)?,
                        Some((1, progress))
                    )
                    .await
                    .map_err(debug)?,
                TutorialProgressOutcome::Unchanged(stored)
            );
            // Reopen the real runtime adapter; no in-memory progress can supply the read.
            let reopened =
                crate::durability::DurabilityRoot::connect_test_runtime(&h.database.url)?;
            assert!(reopened.maintain_ready_once().await?);
            let reopened_authority = reopened
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            assert_eq!(
                reopened
                    .character_tutorial_progress(&reopened_authority, &h.node, fence(1)?, None)
                    .await
                    .map_err(debug)?,
                TutorialProgressOutcome::Read(Some(stored))
            );
            let mut stale = fence(1)?;
            stale.connection_generation =
                crate::foundation::ConnectionGeneration::new(2).map_err(debug)?;
            for change in [None, Some((1, progress))] {
                assert!(
                    h.root
                        .character_tutorial_progress(&authority, &h.node, stale, change)
                        .await
                        .is_err()
                );
            }
            stale = fence(1)?;
            stale.character_lease_generation += 1;
            assert!(
                h.root
                    .character_tutorial_progress(&authority, &h.node, stale, None)
                    .await
                    .is_err()
            );
            stale = fence(1)?;
            stale.scope_ownership_generation =
                crate::foundation::ScopeOwnershipGeneration::new(2).map_err(debug)?;
            assert!(
                h.root
                    .character_tutorial_progress(&authority, &h.node, stale, Some((1, progress)))
                    .await
                    .is_err()
            );
            assert_eq!(h.root_revision().await?, "1");
            Ok(())
        }
        .await;
        h.cleanup().await?;
        result
    })
}
