//! Resume retains holder preference before Activity/House routing selects a scope.
use super::fixture::{Harness, debug, id};
use super::kill_reward_e2e::scenario_thread::run_async as run;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::{DurabilityError, ROOT_RECOVERY_WINDOW};
use crate::foundation::{AuthenticatedTransportRefV1, GameSessionState};
use crate::gameplay_transport::connection::{
    AdmissionRefusal, FreshAdmissionAuthority, ResumeAttempt,
};
use std::future::{Future, poll_fn};
use std::task::Poll;
use std::time::Duration;

#[test]
fn resume_keeps_bounded_holder_preference_and_releases_it_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "resume_holder_priority", true, false)
                .await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let owner = dependencies.owner(&harness, &authority);
        let session = dependencies.admitted.game_session_id;
        let store = FreshAdmissionStore::from_root(harness.root.clone());
        let (before, _) = store.current_session_at(session).await?;
        assert_eq!(before.session_state(), GameSessionState::Active);
        let receipts_before: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_durability_fresh_admission_receipts")
                .fetch_one(&harness.pool)
                .await?;
        let candidate = AuthenticatedTransportRefV1::decode(&id(247)).map_err(debug)?;
        // The real routing reads must complete before ordinary resume refuses
        // this still-active session. No recovery credential is accepted here.
        let attempt = || ResumeAttempt {
            game_session_id: session,
            recovery_material: b"unread-for-active-session",
            transport: candidate,
            last_applied_server_sequence: 0,
            supported_capabilities: &[],
        };

        let held = owner.root.try_issue_background_semantic_pass()?;
        let mut resuming = Box::pin(owner.resume(attempt()));
        assert!(
            poll_fn(|cx| Poll::Ready(resuming.as_mut().poll(cx)))
                .await
                .is_pending()
        );
        assert!(owner.runtime.try_lock().is_ok());
        assert!(owner.states.spell_states.try_lock().is_ok());
        held.run(|_, _| Box::pin(async { Ok(()) })).await?;
        assert!(owner.root.is_ready());
        // No repoll of resume after return: its first Activity lookup, before
        // resume_session_for_scope, must already own foreground preference.
        for _ in 0..16 {
            assert!(matches!(
                owner.root.try_issue_background_semantic_pass(),
                Err(DurabilityError::RootUnavailable)
            ));
        }
        assert!(matches!(resuming.await, Err(AdmissionRefusal::Rejected)));
        owner
            .root
            .try_issue_background_semantic_pass()?
            .run(|_, _| Box::pin(async { Ok(()) }))
            .await?;

        let held = owner.root.try_issue_semantic_pass()?;
        let mut cancelled = Box::pin(owner.resume(attempt()));
        assert!(
            poll_fn(|cx| Poll::Ready(cancelled.as_mut().poll(cx)))
                .await
                .is_pending()
        );
        held.run(|_, _| Box::pin(async { Ok(()) })).await?;
        assert!(owner.root.is_ready());
        assert!(matches!(
            owner.root.try_issue_background_semantic_pass(),
            Err(DurabilityError::RootUnavailable)
        ));
        drop(cancelled);
        owner
            .root
            .try_issue_background_semantic_pass()?
            .run(|_, _| Box::pin(async { Ok(()) }))
            .await?;

        // An unavailable first routing read consumes the original root budget;
        // it cannot renew that budget by entering another resume retry round.
        let held = owner.root.try_acquire_ready()?;
        let started = tokio::time::Instant::now();
        let result = tokio::time::timeout(
            ROOT_RECOVERY_WINDOW + Duration::from_secs(1),
            Box::pin(owner.resume(attempt())),
        )
        .await?;
        assert!(matches!(result, Err(AdmissionRefusal::Unavailable)));
        assert!(
            started.elapsed()
                >= ROOT_RECOVERY_WINDOW
                    - crate::gameplay_transport::fresh_admission_backoff(&session)
        );
        drop(held);
        owner.root.await_ready().await?;
        owner
            .root
            .try_issue_background_semantic_pass()?
            .run(|_, _| Box::pin(async { Ok(()) }))
            .await?;

        let (after, _) = store.current_session_at(session).await?;
        assert_eq!(after.session_state(), before.session_state());
        assert_eq!(
            after.current_connection_generation(),
            before.current_connection_generation()
        );
        assert_eq!(after.current_transport(), before.current_transport());
        assert_eq!(
            after.current_control_loss_epoch(),
            before.current_control_loss_epoch()
        );
        assert_eq!(
            after.current_original_grace_deadline(),
            before.current_original_grace_deadline()
        );
        let receipts_after: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_durability_fresh_admission_receipts")
                .fetch_one(&harness.pool)
                .await?;
        assert_eq!(receipts_after, receipts_before);

        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
