//! Grace expiry polls staff decisions before every release attempt. A busy holder at that
//! poll must be waited for, not end the attempt: under steady holder use a one-shot poll
//! lost every time and the lost session was never released (node boot world=1).
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::connection::{
    ControlLossResult, FreshAdmissionAuthority, GraceExpiryResult,
};
use std::time::Duration;

#[test]
fn staff_release_poll_waits_for_busy_holder_on_lost_session_postgres() -> TestResult {
    run_scenario(Duration::from_millis(150))
}

#[test]
fn staff_release_poll_waits_through_lifecycle_holder_contention_postgres() -> TestResult {
    run_scenario(Duration::from_millis(1_500))
}

fn run_scenario(hold_duration: Duration) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(hold_duration).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "staff release holder fixture panicked")??;
    Ok(())
}

fn scenario(hold_duration: Duration) -> TestResult {
    run(async move |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "staff_release_holder_busy", true, false)
                .await?;
        prepare_current_session(&h).await?;
        h.seed_pvp_target().await?;
        crate::gameplay_transport::monk_save::postgres_tests::appearance_change_postgres_fixtures::observer_visibility_tests::admit(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let owner = deps.owner(&h, &authority);
        deps.initialize(&owner).await;
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        super::track(&owner, &deps)?;
        owner
            .states
            .command_ingress
            .admit(deps.admitted.game_session_id);
        assert_eq!(
            owner
                .lose_control(deps.admitted, std::time::Duration::ZERO, None)
                .await,
            ControlLossResult::Recorded
        );
        // The other owner's pass holds the single holder before the poll starts.
        let holder = owner.root.try_issue_semantic_pass().map_err(debug)?;
        let (polled, ()) = tokio::join!(owner.release_staff_session(deps.admitted), async {
            tokio::time::sleep(hold_duration).await;
            drop(holder);
        });
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        assert_eq!(
            polled,
            GraceExpiryResult::NotApplicable,
            "the staff poll should wait for the holder and observe no staff action"
        );
        Ok(())
    })
}
