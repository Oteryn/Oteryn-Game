//! One LOGOUT_INTENT contends with the real durability holder during release.
use super::fixture::{Harness, debug};
use super::{Dependencies, TestResult, kill_reward_e2e_fixtures, prepare_current_session, track};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::GameSessionState;
use crate::gameplay_transport::condition_persistence::BUSY_LOGOUT_CONDITION_READS;
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, LogoutReply};
use oteryn_protocol_oteryn::logout::LogoutResult;
use std::cell::Cell;

#[test]
fn logout_accepts_first_request_after_one_busy_holder_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario(admin, 1, LogoutResult::Accepted, GameSessionState::Terminal)
}

#[test]
fn logout_returns_busy_after_release_retry_bound_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario(
        admin,
        u32::MAX,
        LogoutResult::Busy,
        GameSessionState::Active,
    )
}

fn scenario(
    admin: String,
    busy_reads: u32,
    expected: LogoutResult,
    expected_state: GameSessionState,
) -> TestResult {
    kill_reward_e2e_fixtures::scenario_thread::run_async(async move |_| {
        let h =
            Harness::create_with_admission_seed(&admin, "logout_busy_holder", true, false).await?;
        prepare_current_session(&h).await?;
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
        track(&owner, &deps)?;

        let remaining = Cell::new(busy_reads);
        let (reply, remaining) = BUSY_LOGOUT_CONDITION_READS
            .scope(remaining, async {
                let reply = owner.logout_command(deps.admitted, 1).await;
                (reply, BUSY_LOGOUT_CONDITION_READS.with(Cell::get))
            })
            .await;
        assert_eq!(reply, LogoutReply::Result(expected));
        let injections = busy_reads - remaining;
        match expected {
            LogoutResult::Accepted => assert_eq!(injections, 1),
            LogoutResult::Busy => assert!(
                (2..=crate::gameplay_transport::RECONCILE_ATTEMPTS).contains(&injections),
                "expected a bounded series of release attempts, got {injections}"
            ),
            LogoutResult::InFight { .. } => unreachable!(),
        }
        let store = FreshAdmissionStore::from_root(h.root.clone());
        assert_eq!(
            store
                .current_session(deps.admitted.game_session_id)
                .await?
                .session_state(),
            expected_state
        );
        if expected == LogoutResult::Busy {
            // The exhausted request returned its fence, so a later request can release.
            assert_eq!(
                owner.logout_command(deps.admitted, 2).await,
                LogoutReply::Result(LogoutResult::Accepted)
            );
        }
        drop(owner);
        drop(seal);
        h.cleanup().await
    })
}
