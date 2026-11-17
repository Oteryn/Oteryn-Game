// NX-1 focused staff release case; included in the original test module.
#[test]
fn nx1_staff_lost_activity_return_keeps_epoch_proof_and_forces_terminal_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let began = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| activity_scenario(true).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "lost Activity kick panicked")??;
    eprintln!(
        "NX-1 lost Activity kick PostgreSQL duration: {:?}",
        began.elapsed()
    );
    Ok(())
}
async fn record_activity_loss(
    h: &Harness,
    session: crate::foundation::GameSessionId,
) -> TestResult {
    use crate::foundation::*;
    let store = FreshAdmissionStore::from_root(h.root.clone());
    let (current, now) = store.current_session_at(session).await?;
    let history = store
        .activity_loss_origin(session)
        .await?
        .ok_or("Activity history")?;
    let ControlLossHistoryV1::ActivityOrigin { protection, .. } = &history else {
        return Err("Activity history kind".into());
    };
    let protection = *protection;
    let mut key = [0u8; 33];
    key[0] = 3;
    key[1..17].copy_from_slice(current.current_runtime_scope().world_id().as_bytes());
    let RuntimeScopeRefV1::Instance { instance_id, .. } = current.current_runtime_scope() else {
        return Err("Instance scope".into());
    };
    key[17..].copy_from_slice(&instance_id);
    let revision: String = sqlx::query_scalar(
        "SELECT source_revision::text FROM game_runtime_scope_assignments WHERE scope_key=$1",
    )
    .bind(key.as_slice())
    .fetch_one(&h.pool)
    .await?;
    let epoch = ControlLossEpochRefV1::new(1).map_err(debug)?;
    let observation = ControlLossObservationV1 {
        source_authority: current.current_runtime_scope(),
        source_revision: revision.parse()?,
        accepted_source_revision: revision.parse()?,
        decision_identity: epoch,
        accepted_decision_identity: epoch,
        observed_at: now,
        session: current,
        account_presence: AccountPresenceClaimV1::new(
            &crate::gameplay_transport::canonical_uuid(&id(40)),
            current.commit().character_id(),
        )
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
    let source = std::sync::Arc::new(crate::gameplay_transport::ChannelOwnedLossSource(
        observation,
    ));
    let mut flow = ControlLossFlowV1::begin(
        ControlLossAuthorizationV1::authorize(source.as_ref(), session, now).map_err(debug)?,
    );
    assert!(matches!(
        store
            .commit_fresh_loss(
                std::sync::Arc::new(flow.take_request().map_err(debug)?),
                source
            )
            .await?,
        ControlLossOutcomeV1::Committed { .. }
    ));
    Ok(())
}
