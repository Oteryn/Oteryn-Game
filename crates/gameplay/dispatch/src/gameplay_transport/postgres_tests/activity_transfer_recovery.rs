//! Credential-verified lost-ack recovery against the real activity/session owners.
use super::activity_runtime::enter_fixture;
use super::fixture::{Harness, debug};
use super::{TestResult, reset};
use crate::durability::activity_handoff::{ActivityHandoffOwner, ActivityTransferRecovery};
use crate::durability::fresh_admission::{CurrentCapabilityCheck, FreshAdmissionStore};
use crate::durability::recovery_evidence_composition::RecoveryEvidenceSubject;
use crate::foundation::fnd04_verifier::{
    VerifiedRecoveryDurabilityFactsV2, fresh_grant_signing_key_id,
};
use crate::foundation::{AuthenticatedTransportRefV1, GameSessionState, ScopeOwnershipGeneration};
use crate::gameplay_transport::item_fence_of;
use std::sync::Arc;

#[test]
fn activity_transfer_recovery_postgres_nonce_transport_and_first_input() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("ACTIVITY_TRANSFER_RECOVERY POSTGRES_CASE_SKIPPED: no admin URL");
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "activity transfer recovery fixture panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    let Some(admin) = crate::bestiary_postgres_harness::configured_admin() else {
        eprintln!("ACTIVITY_TRANSFER_RECOVERY POSTGRES_CASE_SKIPPED: no configured admin");
        return Ok(());
    };
    let runtime = crate::bestiary_postgres_harness::runtime()?;
    let h = runtime.block_on(Harness::create_with_admission_seed(
        admin,
        "native_activity_transfer_recovery",
        true,
        false,
    ))?;
    let result =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| runtime.block_on(case(&h))));
    let cleanup = runtime.block_on(h.cleanup());
    match result {
        Ok(result) => {
            result?;
            cleanup
        }
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

async fn verified(
    h: &Harness,
    evidence: &super::house_transfer::RecoverySource,
    candidate: &ActivityTransferRecovery,
    nonce: u8,
) -> TestResult<VerifiedRecoveryDurabilityFactsV2> {
    let token = super::house_transfer::RecoverySource::token(nonce)?;
    let key = fresh_grant_signing_key_id(&token).ok_or("fixture signing key")?;
    evidence
        .evidence
        .refresh_recovery(&h.root, &h.node, &candidate.current.account_id, &key)
        .await
        .map_err(debug)?;
    let subject =
        RecoveryEvidenceSubject::new(&candidate.current.account_id, &key).map_err(debug)?;
    h.root
        .verify_registered_recovery(&h.node, &subject, &token, &candidate.current)
        .await
        .map_err(debug)?
        .map_err(debug)
}

async fn counts(h: &Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_activity_transfer_recoveries),(SELECT count(*) FROM game_durability_recovery_grant_consumptions),(SELECT count(*) FROM game_durability_transport_ref_reservations)")
        .fetch_one(&h.pool).await?)
}

async fn case(h: &Harness) -> TestResult {
    reset();
    let entered = enter_fixture(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let owner = ActivityHandoffOwner::new(h.root.clone());
    let store = FreshAdmissionStore::from_root(h.root.clone());
    let source = entered.request.participants[0].source.game_session_id;
    let candidate = owner
        .transfer_recovery(source)
        .await?
        .ok_or("retained successor routing")?;
    assert_eq!(candidate.current_session, entered.fence.game_session_id);
    assert_eq!(candidate.source_session, source);
    assert_eq!(candidate.runtime_scope, entered.fence.runtime_scope);
    assert_eq!(candidate.selected_capabilities, vec![117]);
    assert!(!candidate.started && !candidate.has_owning_loss);
    let transport = AuthenticatedTransportRefV1::decode(&[91; 16]).map_err(debug)?;
    let capabilities: CurrentCapabilityCheck = Arc::new(|_, _, _| true);
    let before = counts(h).await?;

    // Canonical credential data alone cannot replace a stale current scope or
    // widen the negotiated selection recorded by the source connection.
    let mut wrong = candidate.clone();
    wrong.scope_generation =
        ScopeOwnershipGeneration::new(candidate.scope_generation.get() + 1).map_err(debug)?;
    assert!(
        owner
            .recover_transfer(
                &authority,
                &h.node,
                wrong,
                verified(h, &entered.evidence, &candidate, 7).await?,
                transport,
                capabilities.clone()
            )
            .await
            .is_err()
    );
    let mut wrong = candidate.clone();
    wrong.selected_capabilities.push(225);
    assert!(
        owner
            .recover_transfer(
                &authority,
                &h.node,
                wrong,
                verified(h, &entered.evidence, &candidate, 7).await?,
                transport,
                capabilities.clone()
            )
            .await
            .is_err()
    );
    assert!(
        owner
            .recover_transfer(
                &authority,
                &h.node,
                candidate.clone(),
                verified(h, &entered.evidence, &candidate, 7).await?,
                transport,
                Arc::new(|_, _, _| false)
            )
            .await
            .is_err()
    );
    assert_eq!(counts(h).await?, before);

    let recovered = owner
        .recover_transfer(
            &authority,
            &h.node,
            candidate.clone(),
            verified(h, &entered.evidence, &candidate, 7).await?,
            transport,
            capabilities.clone(),
        )
        .await?;
    assert_eq!(recovered.connection_generation, 2);
    assert_eq!(recovered.current_session, candidate.current_session);
    let after = counts(h).await?;
    assert_eq!(after, (before.0 + 1, before.1 + 1, before.2 + 1));
    // Simulate an unknown commit acknowledgement: keep the original generation-1
    // candidate and repeat exactly the same single-use nonce and transport.
    let replay = owner
        .recover_transfer(
            &authority,
            &h.node,
            candidate.clone(),
            verified(h, &entered.evidence, &candidate, 7).await?,
            transport,
            capabilities.clone(),
        )
        .await?;
    assert_eq!(replay.connection_generation, 2);
    assert_eq!(counts(h).await?, after);
    assert!(
        owner
            .recover_transfer(
                &authority,
                &h.node,
                candidate.clone(),
                verified(h, &entered.evidence, &candidate, 7).await?,
                AuthenticatedTransportRefV1::decode(&[92; 16]).map_err(debug)?,
                capabilities.clone()
            )
            .await
            .is_err()
    );
    assert_eq!(counts(h).await?, after);

    let current = store.current_session_at(candidate.current_session).await?.0;
    assert_eq!(current.session_state(), GameSessionState::Active);
    assert_eq!(current.current_connection_generation().get(), 2);
    assert_eq!(current.current_transport(), Some(transport));
    assert!(current.current_control_loss_epoch().is_none());
    assert!(current.current_original_grace_deadline().is_none());
    assert_eq!(current.commit().game_session_id(), source);
    assert_eq!(
        current.current_character_lease().generation(),
        entered.fence.character_lease_generation
    );
    let ordinary_loss_rows: i64 = sqlx::query_scalar("SELECT count(*) FROM game_durability_admission_lifecycle_receipts WHERE substring(operation_key FROM 1 FOR 14)=convert_to('owning-loss-v1','UTF8')")
        .fetch_one(&h.pool).await?;
    assert_eq!(
        ordinary_loss_rows, 0,
        "lost acknowledgement is not a fabricated control loss"
    );
    let attempt_count: i16 = sqlx::query_scalar("SELECT attempt_count FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid")
        .bind(candidate.current_session.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(attempt_count, 0, "ordinary loss-epoch budget is untouched");

    // The old transport/generation cannot even mark the new stream as started.
    assert!(
        owner
            .note_transfer_input(
                &h.node,
                entered.fence,
                entered.request.participants[0]
                    .destination_transport
                    .ok_or("fixture transport")?
            )
            .await
            .is_err()
    );
    assert!(
        !owner
            .transfer_recovery(candidate.current_session)
            .await?
            .ok_or("current projection")?
            .started
    );
    let fence = item_fence_of(current).ok_or("recovered current fence")?;
    owner.note_transfer_input(&h.node, fence, transport).await?;
    let started = owner
        .transfer_recovery(candidate.current_session)
        .await?
        .ok_or("started projection remains readable")?;
    assert!(started.started);
    assert_eq!(
        started.selected_capabilities,
        candidate.selected_capabilities
    );
    assert!(
        owner
            .recover_transfer(
                &authority,
                &h.node,
                started.clone(),
                verified(h, &entered.evidence, &started, 8).await?,
                AuthenticatedTransportRefV1::decode(&[93; 16]).map_err(debug)?,
                capabilities
            )
            .await
            .is_err()
    );
    assert_eq!(counts(h).await?, after);
    eprintln!("ACTIVITY_TRANSFER_RECOVERY POSTGRES_CASE_COMPLETED");
    Ok(())
}
