//! Captured security state cannot be dropped by a real scope successor.
use super::TestResult;
use super::fixture::{Harness, debug, id};
use crate::durability::activity_handoff::{
    ActivityHandoffOwner, ActivityReturnReason, ActivityReturnRequest,
};
use crate::durability::character_last_position::LastVitals;
use crate::durability::platform_streams::{PageOutcome, SecurityRevocation, StreamPage};
use crate::foundation::{AuthenticatedTransportRefV1, GameSessionId};
use std::sync::{Arc, Mutex};

type Stage = Arc<Mutex<&'static str>>;

#[test]
fn revocation_activity_successor_lineage_and_controller_fences_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let stage = Arc::new(Mutex::new("setup"));
    let observed = stage.clone();
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    std::thread::Builder::new()
        .name("revocation-lineage".into())
        .stack_size(256 << 20)
        .spawn(move || {
            let result = scenario(&stage).map_err(|e| e.to_string());
            let _ = send.send(result);
        })?;
    receive
        .recv_timeout(std::time::Duration::from_secs(120))
        .map_err(|e| {
            format!(
                "revocation lineage watchdog at {}: {e}",
                observed.lock().map(|stage| *stage).unwrap_or("stage lock")
            )
        })?
        .map_err(Into::into)
}

fn scenario(stage: &Stage) -> TestResult {
    let admin = crate::bestiary_postgres_harness::configured_admin()
        .ok_or("configured PostgreSQL admin")?;
    let runtime = crate::bestiary_postgres_harness::runtime()?;
    let h = runtime.block_on(Harness::create_with_admission_seed(
        admin,
        "revocation_lineage",
        true,
        false,
    ))?;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        runtime.block_on(case(&h, stage))
    }));
    *stage.lock().map_err(|_| "stage lock")? = "cleanup";
    let cleanup = runtime.block_on(h.cleanup());
    match result {
        Ok(result) => {
            result?;
            cleanup
        }
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

async fn case(h: &Harness, stage: &Stage) -> TestResult {
    *stage.lock().map_err(|_| "stage lock")? = "admission/handoff";
    let entered = super::activity_runtime::enter_fixture(h).await?;
    let current = entered.fence.game_session_id;
    let (generation,revoked):(Option<String>,bool)=sqlx::query_as("SELECT admission_security_generation::text,security_revoked FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid")
                    .bind(current.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        generation.as_deref(),
        Some("1"),
        "activity successor must retain its admission generation"
    );
    assert!(!revoked);
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let owner = ActivityHandoffOwner::new(h.root.clone());
    let candidate = owner
        .transfer_recovery(current)
        .await?
        .ok_or("current activity")?;
    let placement = owner
        .read_return_placement(&authority, &h.node, entered.fence)
        .await?
        .ok_or("return placement")?;
    let credential = super::house_transfer::RecoverySource::token(246)?;
    let kid = crate::foundation::fnd04_verifier::fresh_grant_signing_key_id(&credential)
        .ok_or("recovery key")?;
    entered
        .evidence
        .evidence
        .refresh_recovery(&h.root, &h.node, &candidate.current.account_id, &kid)
        .await
        .map_err(debug)?;
    let subject = crate::durability::recovery_evidence_composition::RecoveryEvidenceSubject::new(
        &candidate.current.account_id,
        &kid,
    )
    .map_err(debug)?;
    let facts = h
        .root
        .verify_registered_recovery(&h.node, &subject, &credential, &candidate.current)
        .await
        .map_err(debug)?
        .map_err(debug)?;
    let now = facts
        .verified_at()
        .checked_add(1)
        .ok_or("fixed decision clock")?;
    let _clock = super::preadmission::FrozenAdmissionClock::install(h, now).await?;
    *stage.lock().map_err(|_| "stage lock")? = "capture";
    assert!(matches!(
        h.root
            .apply_platform_stream_page(
                None,
                StreamPage::SecurityRevocations(vec![SecurityRevocation {
                    revocation_id: id(247),
                    account_id: candidate.account,
                    native_security_generation: 1,
                    cause: 3,
                    effective_at_ms: now * 1000
                }]),
                Some("v1:1".into())
            )
            .await
            .map_err(debug)?,
        PageOutcome::Advanced(_)
    ));
    *stage.lock().map_err(|_| "stage lock")? = "ordinary return";
    let returns: i64 = sqlx::query_scalar("SELECT count(*) FROM game_activity_runtime_returns")
        .fetch_one(&h.pool)
        .await?;
    let escape = owner
        .return_participant(
            &authority,
            &h.node,
            ActivityReturnRequest {
                return_id: id(248),
                handoff_id: entered.request.handoff_id,
                command_id: Some(2),
                source: entered.fence,
                destination_game_session_id: GameSessionId::decode(&id(249)).map_err(debug)?,
                destination_transport: Some(
                    AuthenticatedTransportRefV1::decode(&[249; 16]).map_err(debug)?,
                ),
                destination_generation: placement.destination_generation,
                reason: ActivityReturnReason::Exit,
                current_vitals: LastVitals::new(51, 9),
                current_rest: Some([2, 200, 8, 9]),
            },
        )
        .await;
    assert!(
        matches!(escape, Err(crate::durability::DurabilityError::Unavailable)),
        "normal return cannot escape a captured revocation"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_activity_runtime_returns")
            .fetch_one(&h.pool)
            .await?,
        returns
    );
    // The consumer flag is deliberately left off: captured lineage remains terminal.
    *stage.lock().map_err(|_| "stage lock")? = "controller recovery";
    let recovery = owner
        .recover_transfer(
            &authority,
            &h.node,
            candidate,
            facts,
            AuthenticatedTransportRefV1::decode(&[246; 16]).map_err(debug)?,
            std::sync::Arc::new(|_, _, _| true),
        )
        .await;
    assert!(
        matches!(
            recovery,
            Err(crate::durability::DurabilityError::Unavailable)
        ),
        "captured lineage cannot issue a replacement controller"
    );

    Ok(())
}
