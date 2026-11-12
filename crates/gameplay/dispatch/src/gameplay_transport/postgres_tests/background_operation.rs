//! Background arbitration must let a bounded multi-pass position write finish.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session, track};
use crate::durability::DurabilityRoot;

#[test]
fn background_position_batch_completes_both_passes_with_foreground_demand_postgres() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("background-position-operation", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(
                admin,
                "background_position_operation",
                true,
                false,
            )
            .await?;
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let dependencies = Dependencies::new()?;
            let owner = dependencies.owner(&h, &authority);
            dependencies.initialize(&owner).await;
            owner
                .runtime
                .lock()
                .await
                .initialize_first_entry_position(dependencies.actor)
                .map_err(debug)?;
            track(&owner, &dependencies)?;
            let foreground = h.root.foreground_demand()?;
            let until = tokio::time::Instant::now() + crate::durability::ROOT_RECOVERY_WINDOW;
            assert_eq!(
                DurabilityRoot::with_background_holder(owner.write_due_last_positions(true)).await,
                1,
                "foreground demand refuses a fresh background operation"
            );
            tokio::time::sleep_until(until).await;
            assert_eq!(
                DurabilityRoot::with_background_holder(owner.write_due_last_positions(true)).await,
                0,
                "one fairness grant must cover the fence read and position write"
            );
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_last_positions")
                    .fetch_one(&h.pool)
                    .await?,
                1
            );
            assert!(
                matches!(
                    h.root.try_issue_background_semantic_pass(),
                    Err(crate::durability::DurabilityError::RootUnavailable)
                ),
                "a completed operation must release its fairness grant"
            );
            drop(foreground);
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}
