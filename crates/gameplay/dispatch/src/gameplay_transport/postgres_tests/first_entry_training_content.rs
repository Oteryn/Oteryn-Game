//! Distinguish a training content rejection from a transient busy holder at first entry.
use super::fixture::{Harness, debug};
use super::kill_reward_e2e_fixtures::scenario_thread::run_async as run;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::connection::FirstEntryOutcome;

#[test]
fn first_entry_training_content_mismatch_is_not_holder_contention_postgres() -> TestResult {
    run(async |admin| {
        let started = std::time::Instant::now();
        let h = Harness::create_with_admission_seed(admin, "entry_training_content", true, false)
            .await?;
        let result: TestResult = async {
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let deps = Dependencies::new()?;
            let (incompatible, _) = super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                deps.admitted.world_id,
                "s3b-content-1",
            )?;
            let (compatible, _) = super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
            let mut owner = deps.owner(&h, &authority);
            deps.initialize(&owner).await;
            let current =
                crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                    .current_session(deps.admitted.game_session_id)
                    .await?;
            let mut admitted = deps.admitted;
            admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
            owner.active_generation = incompatible.active();
            // The holder is idle, with no competitors. Retrying an incompatible formula
            // cannot turn a semantic rejection into a successful admission settlement.
            for _ in 0..2 {
                h.root.await_ready().await?;
                assert!(h.root.is_ready());
                admitted.first_entry = FirstEntryOutcome::Positioned;
                owner.admit_loop_training(&mut admitted).await;
                assert_eq!(admitted.first_entry, FirstEntryOutcome::RefusedUnavailable);
            }
            let markers: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_character_training_markers")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(markers, 0, "incompatible training must write nothing");

            // Rebinding the fixture's authored formula to its Character interpretation
            // uses the unchanged durable context guard and real admission owner.
            owner.active_generation = compatible.active();
            admitted.first_entry = FirstEntryOutcome::Positioned;
            h.root.await_ready().await?;
            owner.admit_loop_training(&mut admitted).await;
            assert_eq!(admitted.first_entry, FirstEntryOutcome::Positioned);
            let markers: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_character_training_markers")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(markers, 1, "compatible admission actually settled training");
            eprintln!(
                "PG_EXECUTED first_entry_training_content mismatched=2 settled=1 elapsed_ms={}",
                started.elapsed().as_millis()
            );
            Ok(())
        }
        .await;
        let cleanup = h.cleanup().await;
        result?;
        cleanup
    })
}
