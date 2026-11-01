//! Actual activity session loss and inherited offline return through the same owners.
use super::TestResult;
use super::activity_runtime::prepare_activity_fixture;
use super::fixture::{Harness, debug, id};
use super::house_transfer::RecoverySource;
use crate::durability::activity_handoff::{
    ActivityEnterOutcome, ActivityHandoffOwner, ActivityReturnOutcome, ActivityReturnReason,
    ActivityReturnRequest, ActivityTransferRecovery,
};
use crate::durability::fresh_admission::{
    FreshAdmissionStore, FreshLossReconciliation, decode_fresh_loss, encode_fresh_loss,
};
use crate::durability::recovery_evidence_composition::RecoveryEvidenceSubject;
use crate::foundation::fnd04_verifier::{
    VerifiedRecoveryDurabilityFactsV2, fresh_grant_signing_key_id,
};
use crate::foundation::*;
use crate::gameplay_transport::{ChannelOwnedLossSource, item_fence_of};
use std::sync::Arc;

// The control assignment schema uses tag 1 for Channel and tag 3 for Instance
// (0247); admission/runtime wire keys use a different Instance tag. This local
// fixture encoding must not widen the production writer's private API.
fn test_assignment_scope_key(scope: RuntimeScopeRefV1) -> [u8; 33] {
    let mut key = [0; 33];
    key[1..17].copy_from_slice(scope.world_id().as_bytes());
    match scope {
        RuntimeScopeRefV1::Channel { channel_id, .. } => {
            key[0] = 1;
            key[17..].copy_from_slice(channel_id.as_bytes());
        }
        RuntimeScopeRefV1::Instance { instance_id, .. } => {
            key[0] = 3;
            key[17..].copy_from_slice(&instance_id);
        }
    }
    key
}

#[test]
fn activity_origin_postgres_actual_session_loss_and_offline_recovery() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("ACTIVITY_ORIGIN_LOSS POSTGRES_CASE_SKIPPED: no admin URL");
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "activity origin fixture panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    let Some(admin) = crate::bestiary_postgres_harness::configured_admin() else {
        eprintln!("ACTIVITY_ORIGIN_LOSS POSTGRES_CASE_SKIPPED: no configured admin");
        return Ok(());
    };
    let runtime = crate::bestiary_postgres_harness::runtime()?;
    for recovered in [false, true] {
        let h = runtime.block_on(Harness::create_with_admission_seed(
            admin.clone(),
            if recovered {
                "activity_recovered_control_loss"
            } else {
                "activity_offline_return_recovery"
            },
            true,
            false,
        ))?;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            runtime.block_on(case(&h, recovered))
        }));
        let cleanup = runtime.block_on(h.cleanup());
        match result {
            Ok(result) => {
                result?;
                cleanup?;
            }
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }
    eprintln!("ACTIVITY_ORIGIN_LOSS POSTGRES_CASE_COMPLETED");
    Ok(())
}

async fn verified(
    h: &Harness,
    evidence: &RecoverySource,
    candidate: &ActivityTransferRecovery,
    nonce: u8,
) -> TestResult<VerifiedRecoveryDurabilityFactsV2> {
    let token = RecoverySource::token(nonce)?;
    let key = fresh_grant_signing_key_id(&token).ok_or("signing key")?;
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

async fn case(h: &Harness, recover_before_loss: bool) -> TestResult {
    let (evidence, mut request) = prepare_activity_fixture(h).await?;
    // Authored test policy, not a production timer override. One millisecond
    // permits a real server timeout without sleeping through a 60-second room.
    request.time_limit_ms = 1;
    let source_session = request.participants[0].source.game_session_id;
    let session = request.participants[0].destination_game_session_id;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let owner = ActivityHandoffOwner::new(h.root.clone());
    assert!(matches!(
        owner.enter(&authority, &h.node, request.clone()).await?,
        ActivityEnterOutcome::Entered(_)
    ));
    let store = FreshAdmissionStore::from_root(h.root.clone());
    if recover_before_loss {
        let candidate = owner
            .transfer_recovery(source_session)
            .await?
            .ok_or("lost-ack candidate")?;
        owner
            .recover_transfer(
                &authority,
                &h.node,
                candidate.clone(),
                verified(h, &evidence, &candidate, 17).await?,
                AuthenticatedTransportRefV1::decode(&[94; 16]).map_err(debug)?,
                Arc::new(|_, _, _| true),
            )
            .await?;
    }
    let (current, now) = store.current_session_at(session).await?;
    let history = store
        .activity_loss_origin(session)
        .await?
        .ok_or("activity loss provenance")?;
    assert!(
        matches!(&history, ControlLossHistoryV1::ActivityOrigin { admission_id, .. } if *admission_id==request.handoff_id)
    );
    let ControlLossHistoryV1::ActivityOrigin { protection, .. } = &history else {
        return Err("activity protection".into());
    };
    let protection = *protection;
    let account: String = sqlx::query_scalar("SELECT account_id::text FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid")
        .bind(session.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    let revision: String = sqlx::query_scalar(
        "SELECT source_revision::text FROM game_runtime_scope_assignments WHERE scope_key=$1",
    )
    .bind(test_assignment_scope_key(current.current_runtime_scope()).as_slice())
    .fetch_one(&h.pool)
    .await?;
    let epoch = ControlLossEpochRefV1::new(1).map_err(debug)?;
    // The sealed owning observer is the production source type. Its placement
    // is isolated authored fixture data; session/lease/transport/claims and
    // assignment are all read from the real committed activity.
    let observation = ControlLossObservationV1 {
        source_authority: current.current_runtime_scope(),
        source_revision: revision.parse()?,
        accepted_source_revision: revision.parse()?,
        decision_identity: epoch,
        accepted_decision_identity: epoch,
        observed_at: now,
        session: current,
        account_presence: AccountPresenceClaimV1::new(&account, current.commit().character_id())
            .map_err(debug)?,
        placement_identity: id(181),
        placement_revision: 1,
        actor_present: true,
        runtime_ready: true,
        cause: ControlLossCauseV1::AuthoritativeUnexpectedLoss,
        loss_epoch: epoch,
        loss_origin: now,
        original_grace_deadline: now + crate::gameplay_transport::SAME_SESSION_GRACE_SECONDS,
        history,
        protection,
    };
    let mut wrong = observation.clone();
    wrong.history = ControlLossHistoryV1::ActivityOrigin {
        admission_id: id(182),
        protection,
    };
    let wrong_source = Arc::new(ChannelOwnedLossSource(wrong));
    let mut flow = ControlLossFlowV1::begin(
        ControlLossAuthorizationV1::authorize(wrong_source.as_ref(), session, now)
            .map_err(debug)?,
    );
    assert_eq!(
        store
            .commit_fresh_loss(Arc::new(flow.take_request().map_err(debug)?), wrong_source)
            .await?,
        ControlLossOutcomeV1::Rejected
    );
    assert_eq!(store.current_session_at(session).await?.0, current);
    // The pure observer can carry a syntactically valid alternative. The SQL
    // owner must independently compare the immutable handoff continuation.
    let mut wrong = observation.clone();
    wrong.protection.usage = RecoveryProtectionUseV1::Unused {
        entitlement_generation: 2,
    };
    wrong.history = ControlLossHistoryV1::ActivityOrigin {
        admission_id: request.handoff_id,
        protection: wrong.protection,
    };
    let wrong_source = Arc::new(ChannelOwnedLossSource(wrong));
    let mut flow = ControlLossFlowV1::begin(
        ControlLossAuthorizationV1::authorize(wrong_source.as_ref(), session, now)
            .map_err(debug)?,
    );
    assert_eq!(
        store
            .commit_fresh_loss(Arc::new(flow.take_request().map_err(debug)?), wrong_source)
            .await?,
        ControlLossOutcomeV1::Rejected
    );
    assert_eq!(store.current_session_at(session).await?.0, current);
    assert!(sqlx::query("UPDATE game_durability_reconnect_sessions SET activity_recovery_protection=$2 WHERE game_session_id=encode($1,'hex')::uuid")
        .bind(session.as_bytes().as_slice()).bind([0_u8; 20].as_slice()).execute(&h.pool).await.is_err(), "retained protection is immutable");
    let loss_source = Arc::new(ChannelOwnedLossSource(observation));
    let mut flow = ControlLossFlowV1::begin(
        ControlLossAuthorizationV1::authorize(loss_source.as_ref(), session, now).map_err(debug)?,
    );
    let loss_request = Arc::new(flow.take_request().map_err(debug)?);
    let operation = loss_request.operation().clone();
    let encoded = encode_fresh_loss(&operation)?;
    assert_eq!(decode_fresh_loss(&encoded, current.commit())?, operation);
    let committed = store
        .commit_fresh_loss(loss_request.clone(), loss_source.clone())
        .await?;
    assert!(matches!(committed, ControlLossOutcomeV1::Committed { .. }));
    assert_eq!(
        store.commit_fresh_loss(loss_request, loss_source).await?,
        committed
    );
    assert!(matches!(
        store.reconcile_fresh_loss(&operation).await?,
        FreshLossReconciliation::Committed { .. }
    ));
    let retained = store
        .owning_loss(session, epoch)
        .await?
        .ok_or("actual activity owning loss")?;
    assert_eq!(retained.0, operation);
    assert!(
        store.owning_loss(source_session, epoch).await?.is_none(),
        "original fresh login is historical, not the lost controller"
    );
    let lost = store.current_session_at(session).await?.0;
    assert_eq!(lost.session_state(), GameSessionState::Reconnectable);
    assert_eq!(lost.current_game_session_id(), session);
    assert_eq!(lost.commit().game_session_id(), source_session);
    assert_eq!(
        lost.current_connection_generation().get(),
        if recover_before_loss { 2 } else { 1 }
    );
    assert!(
        owner
            .transfer_recovery(session)
            .await?
            .ok_or("loss projection")?
            .has_owning_loss
    );
    if recover_before_loss {
        return Ok(());
    }

    // Native timeout may return a player while its controller is lost. It
    // preserves the original grace and issues no fictional owning-loss receipt
    // for the fresh origin-Channel successor.
    let returned_session = GameSessionId::decode(&id(183)).map_err(debug)?;
    let return_request = ActivityReturnRequest {
        return_id: id(184),
        handoff_id: request.handoff_id,
        command_id: None,
        source: item_fence_of(lost).ok_or("lost current fence")?,
        destination_game_session_id: returned_session,
        destination_transport: None,
        destination_generation: request.participants[0].source.scope_ownership_generation,
        reason: ActivityReturnReason::Timeout,
        current_vitals: request.participants[0].source_position.vitals,
        current_rest: request.participants[0].source_position.rest,
    };
    let returned = owner
        .return_participant(&authority, &h.node, return_request)
        .await?;
    assert!(
        matches!(returned, ActivityReturnOutcome::Returned(_)),
        "{returned:?}"
    );
    let offline = store.current_session_at(returned_session).await?.0;
    assert_eq!(offline.session_state(), GameSessionState::Reconnectable);
    assert_eq!(
        offline.current_original_grace_deadline(),
        lost.current_original_grace_deadline()
    );
    assert!(store.owning_loss(returned_session, epoch).await?.is_none());
    let candidate = owner
        .transfer_recovery(source_session)
        .await?
        .ok_or("entry and timeout return lineage")?;
    assert_eq!(candidate.current_session, returned_session);
    assert!(!candidate.started && !candidate.has_owning_loss);
    assert_eq!(
        candidate.selected_capabilities,
        request.participants[0].selected_capabilities
    );
    let recovered = owner
        .recover_transfer(
            &authority,
            &h.node,
            candidate.clone(),
            verified(h, &evidence, &candidate, 18).await?,
            AuthenticatedTransportRefV1::decode(&[95; 16]).map_err(debug)?,
            Arc::new(|_, _, _| true),
        )
        .await?;
    assert_eq!(recovered.current_session, returned_session);
    assert_eq!(recovered.connection_generation, 2);
    let active = store.current_session_at(returned_session).await?.0;
    assert_eq!(active.session_state(), GameSessionState::Active);
    assert!(active.current_control_loss_epoch().is_none());
    assert!(active.current_original_grace_deadline().is_none());
    assert!(
        matches!(store.activity_loss_origin(returned_session).await?, Some(ControlLossHistoryV1::ActivityOrigin { admission_id, protection: inherited }) if admission_id==id(184) && inherited==protection)
    );
    // The original immutable receipt still owns the old Instance loss/grace.
    assert_eq!(
        store
            .owning_loss(session, epoch)
            .await?
            .ok_or("retained old loss")?
            .0,
        operation
    );
    Ok(())
}
