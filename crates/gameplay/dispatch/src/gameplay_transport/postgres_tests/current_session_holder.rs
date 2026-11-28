//! W20b: the first canonical lookup after admission can meet a scheduled pass.
use super::TestResult;
use super::fixture::{Harness, debug, id};
use super::fresh_session_fixtures::commit_current_session;
use super::kill_reward_e2e_fixtures::scenario_thread::run;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::{DurabilityError, DurabilityRoot};
use crate::foundation::{GameSessionId, GameSessionState};
use std::future::{Future, poll_fn};
use std::task::Poll;

#[test]
fn current_session_reads_new_admission_on_first_call_with_busy_holder_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run("current-session-busy-holder", move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
                let h =
                    Harness::create_with_admission_seed(admin, "current_session_busy", true, false)
                        .await?;
                let request = commit_current_session(&h).await?;
                let session = request.binding().candidate_session;
                let store = FreshAdmissionStore::from_root(h.root.clone());
                let held = h.root.try_issue_background_semantic_pass()?;
                let mut lookup = Box::pin(DurabilityRoot::with_holder_owner(
                    store.current_session(session),
                ));
                let first = poll_fn(|cx| Poll::Ready(lookup.as_mut().poll(cx))).await;
                assert!(
                    first.is_pending(),
                    "the first lookup must wait for the holder: {first:?}"
                );
                held.run(|_, _| Box::pin(async { Ok(()) })).await?;

                // A scheduler can try to reacquire before the suspended lookup is polled again.
                assert!(matches!(
                    h.root.try_issue_background_semantic_pass(),
                    Err(DurabilityError::RootUnavailable)
                ));
                let current = lookup.await?;
                assert_eq!(current.current_game_session_id(), session);
                assert_eq!(current.session_state(), GameSessionState::Active);
                h.root
                    .try_issue_background_semantic_pass()?
                    .run(|_, _| Box::pin(async { Ok(()) }))
                    .await?;
                h.cleanup().await
            })
    })
}

#[test]
fn current_session_returns_root_unavailable_for_unknown_session_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run("current-session-unknown", move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
                let h = Harness::create_with_admission_seed(
                    admin,
                    "current_session_unknown",
                    false,
                    false,
                )
                .await?;
                let unknown = GameSessionId::decode(&id(250)).map_err(debug)?;
                let store = FreshAdmissionStore::from_root(h.root.clone());
                let result = store.current_session(unknown).await;
                assert!(
                    matches!(result, Err(DurabilityError::RootUnavailable)),
                    "{result:?}"
                );
                h.cleanup().await
            })
    })
}
