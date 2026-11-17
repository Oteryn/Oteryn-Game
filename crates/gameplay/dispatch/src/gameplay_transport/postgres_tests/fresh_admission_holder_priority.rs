//! The real fresh-admission read keeps foreground preference until its owner ends.
use super::fixture::{Harness, debug, id};
use super::kill_reward_e2e::scenario_thread::run_async as run;
use super::{Dependencies, TestResult};
use crate::durability::{DurabilityError, ROOT_RECOVERY_WINDOW};
use crate::foundation::{AuthenticatedTransportRefV1, CharacterId, GameSessionId};
use crate::gameplay_transport::connection::{
    AdmissionRefusal, FreshAdmissionAttempt, FreshAdmissionAuthority,
};
use base64::Engine as _;
use std::future::{Future, poll_fn};
use std::task::Poll;
use std::time::Duration;

#[test]
fn fresh_admission_keeps_bounded_holder_preference_and_releases_it_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "fresh_holder_priority", true, false)
                .await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let owner = dependencies.owner(&harness, &authority);
        let before = admission_rows(&harness).await?;

        // A syntactically valid selector reaches the first real Character read.
        // The Character is absent, so this tests arbitration and the unchanged
        // refusal, not a successful grant or the Platform evidence exchange.
        let header = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(br#"{"alg":"Ed25519","kid":"fresh-1","typ":"oteryn-admission+jwt"}"#);
        let material = format!("{header}.e30.AA");
        assert_eq!(
            crate::foundation::fnd04_verifier::fresh_grant_signing_key_id(&material).as_deref(),
            Some("fresh-1")
        );
        let attempt = FreshAdmissionAttempt {
            character_id: CharacterId::decode(&id(245)).map_err(debug)?,
            admission_material: material.as_bytes(),
            game_session_id: GameSessionId::decode(&id(246)).map_err(debug)?,
            transport: AuthenticatedTransportRefV1::decode(&id(247)).map_err(debug)?,
            supported_capabilities: &[],
        };

        // The first read is genuinely busy. Return the holder and try the
        // scheduled checkout before polling admission again: no artificial gap.
        let held = owner.root.try_issue_background_semantic_pass()?;
        let mut admitting = Box::pin(owner.admit(attempt));
        assert!(
            poll_fn(|cx| Poll::Ready(admitting.as_mut().poll(cx)))
                .await
                .is_pending()
        );
        assert!(owner.runtime.try_lock().is_ok());
        assert!(owner.states.spell_states.try_lock().is_ok());
        held.run(|_, _| Box::pin(async { Ok(()) })).await?;
        assert!(owner.root.is_ready());
        for _ in 0..16 {
            assert!(matches!(
                owner.root.try_issue_background_semantic_pass(),
                Err(DurabilityError::RootUnavailable)
            ));
        }
        assert!(matches!(admitting.await, Err(AdmissionRefusal::Rejected)));
        owner
            .root
            .try_issue_background_semantic_pass()?
            .run(|_, _| Box::pin(async { Ok(()) }))
            .await?;
        assert_eq!(admission_rows(&harness).await?, before);

        // Cancellation at the same pre-write boundary releases the demand.
        let held = owner.root.try_issue_semantic_pass()?;
        let mut cancelled = Box::pin(owner.admit(attempt));
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
        assert_eq!(admission_rows(&harness).await?, before);

        // Keep a raw checkout through every retry. Its lifetime has no semantic
        // pass deadline; admission still ends within its original five-second
        // window, rather than renewing that window on each busy read.
        let held = owner.root.try_acquire_ready()?;
        let started = tokio::time::Instant::now();
        let result = tokio::time::timeout(
            ROOT_RECOVERY_WINDOW + Duration::from_secs(1),
            Box::pin(owner.admit(attempt)),
        )
        .await?;
        assert!(matches!(result, Err(AdmissionRefusal::Unavailable)));
        assert!(
            started.elapsed()
                >= ROOT_RECOVERY_WINDOW
                    - crate::gameplay_transport::fresh_admission_backoff(&attempt.game_session_id)
        );
        assert!(owner.runtime.try_lock().is_ok());
        assert!(owner.states.spell_states.try_lock().is_ok());
        drop(held);
        owner.root.await_ready().await?;
        owner
            .root
            .try_issue_background_semantic_pass()?
            .run(|_, _| Box::pin(async { Ok(()) }))
            .await?;
        assert_eq!(admission_rows(&harness).await?, before);

        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

async fn admission_rows(harness: &Harness) -> TestResult<(i64, i64)> {
    Ok(sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_durability_fresh_admission_receipts), \
                (SELECT count(*) FROM game_durability_reconnect_sessions)",
    )
    .fetch_one(&harness.pool)
    .await?)
}
