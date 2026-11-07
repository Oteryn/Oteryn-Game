use super::*;

fn uuid_v7(raw: u64) -> [u8; 16] {
    let mut out = [0_u8; 16];
    out[8..].copy_from_slice(&raw.to_be_bytes());
    out[6] = 0x70;
    out[8] = (out[8] & 0x3f) | 0x80;
    out
}

fn sample_record(
    attempt_raw: u64,
    transport_byte: u8,
) -> Result<ReconnectDurabilityRecordV1, ReconnectDurabilityErrorV1> {
    let game_session_id = GameSessionId::decode(&uuid_v7(10))
        .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
    let character_id = CharacterId::decode(&uuid_v7(11))
        .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
    let world_id = WorldId::decode(&uuid_v7(12))
        .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
    let channel_id = ChannelId::decode(&uuid_v7(13))
        .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
    let identity = ReconnectIdentityV1::new(
        game_session_id,
        ReconnectAttemptRef::new(attempt_raw)
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        "123e4567-e89b-12d3-a456-426614174000",
        character_id,
        world_id,
        RuntimeScopeRefV1::channel(world_id, channel_id),
    )?;
    let connection = ReconnectConnectionFenceV1::new(
        ConnectionGeneration::new(7)
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        ConnectionGeneration::new(8)
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        AuthenticatedTransportRefV1::decode(&[transport_byte; 16])
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
    )?;
    let authority = ReconnectAuthorityFenceV1::new(
        9,
        ScopeOwnershipGeneration::new(10)
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
    )?;
    let continuity = ReconnectContinuityV1::new(
        ControlLossEpochRefV1::new(3)?,
        120,
        115,
        ProtectionEntitlementV1::unused(),
    )?;
    let fnd02 = Fnd02ReconciliationFenceV1::new(
        CommandId::new(3).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        vec![],
        41,
        vec![],
    )?;
    let platform = AuthorityEvidenceFenceV1::new(
        "platform-security",
        "reconnect",
        "account",
        "sec:17",
        "decision:sec:17",
        100,
    )?;
    let trust = AuthorityEvidenceFenceV1::new(
        "proof-trust",
        "reconnect",
        "recovery-key",
        "trust:21",
        "decision:trust:21",
        101,
    )?;
    let compatibility = ReconnectCompatibilityEvidenceV1::new(
        1,
        1,
        "rules:1",
        "content:2",
        "map:3",
        "world:4",
        12,
        platform,
        trust,
        Some(110),
    )?;
    ReconnectDurabilityRecordV1::new(
        identity,
        connection,
        authority,
        continuity,
        ReconnectProofV1::ReauthenticatedRecovery {
            recovery_grant_nonce: [0x55; 32],
        },
        fnd02,
        compatibility,
    )
}

#[test]
fn direct_and_reconciled_v2_prepared_paths_preserve_commit_progression(
) -> Result<(), ReconnectDurabilityErrorV1> {
    let direct_record = sample_record(20, 20)?;
    let mut direct_budget =
        ReconnectAttemptBudgetV1::new(direct_record.continuity().control_loss_epoch());
    direct_budget.reserve(
        direct_record.identity().reconnect_attempt_ref(),
        direct_record.connection().transport_ref(),
    )?;
    let (mut direct_flow, direct_request) =
        ReconnectDurabilityFlowV2::begin(direct_record.clone(), None);
    assert_eq!(
        direct_flow.accept_prepare_completion(
            ReconnectPrepareCompletionV2::for_request(
                &direct_request,
                ReconnectPrepareDispositionV2::Prepared,
            ),
            &mut direct_budget,
        )?,
        ReconnectPrepareActionV2::AwaitFinalRevalidation
    );
    let direct_commit = direct_flow.authorize_commit(
        ReconnectCurrentAuthorityV1::from_record(&direct_record, 105)?,
        104,
    )?;
    assert_eq!(direct_flow.phase(), ReconnectDurabilityPhaseV1::PendingCommit);
    assert_eq!(
        direct_flow.accept_commit_completion(ReconnectCommitCompletionV1::for_request(
            &direct_commit,
            ReconnectCommitDispositionV1::Ambiguous,
        ))?,
        ReconnectCommitActionV1::ReconcileSameAttempt
    );

    let reconciled_record = sample_record(21, 21)?;
    let mut reconciled_budget =
        ReconnectAttemptBudgetV1::new(reconciled_record.continuity().control_loss_epoch());
    reconciled_budget.reserve(
        reconciled_record.identity().reconnect_attempt_ref(),
        reconciled_record.connection().transport_ref(),
    )?;
    let (mut reconciled_flow, reconciled_request) =
        ReconnectDurabilityFlowV2::begin(reconciled_record.clone(), None);
    assert_eq!(
        reconciled_flow.accept_prepare_completion(
            ReconnectPrepareCompletionV2::for_request(
                &reconciled_request,
                ReconnectPrepareDispositionV2::Ambiguous,
            ),
            &mut reconciled_budget,
        )?,
        ReconnectPrepareActionV2::ReconcileSameAttempt
    );
    assert_eq!(
        reconciled_flow.accept_reconciliation(
            ReconnectDurableReconciliationSnapshotV2::new(
                reconciled_record.clone(),
                ReconnectDurableOutcomeV2::Prepared,
            ),
            ReconnectCurrentAuthorityV1::from_record(&reconciled_record, 105)?,
            &mut reconciled_budget,
        )?,
        ReconnectProjectionDecisionV2::AwaitFinalRevalidation
    );
    let reconciled_commit = reconciled_flow.authorize_commit(
        ReconnectCurrentAuthorityV1::from_record(&reconciled_record, 105)?,
        104,
    )?;
    assert_eq!(
        reconciled_flow.accept_commit_completion(ReconnectCommitCompletionV1::for_request(
            &reconciled_commit,
            ReconnectCommitDispositionV1::Committed,
        ))?,
        ReconnectCommitActionV1::ReconcileSameAttempt
    );
    assert_eq!(
        reconciled_flow.phase(),
        ReconnectDurabilityPhaseV1::ReconciliationRequired
    );
    Ok(())
}

#[test]
fn v2_committed_reconciliation_revalidates_complete_current_authority(
) -> Result<(), ReconnectDurabilityErrorV1> {
    let record = sample_record(31, 31)?;
    let snapshot = ReconnectDurableReconciliationSnapshotV2::new(
        record.clone(),
        ReconnectDurableOutcomeV2::Committed {
            current_generation: record.connection().candidate(),
            current_transport_ref: record.connection().transport_ref(),
        },
    );
    let reconcile = |current: ReconnectCurrentAuthorityV1| {
        let mut budget =
            ReconnectAttemptBudgetV1::new(record.continuity().control_loss_epoch());
        budget.reserve(
            record.identity().reconnect_attempt_ref(),
            record.connection().transport_ref(),
        )?;
        let (mut flow, request) = ReconnectDurabilityFlowV2::begin(record.clone(), None);
        flow.accept_prepare_completion(
            ReconnectPrepareCompletionV2::for_request(
                &request,
                ReconnectPrepareDispositionV2::Ambiguous,
            ),
            &mut budget,
        )?;
        flow.accept_reconciliation(snapshot.clone(), current, &mut budget)
    };

    assert!(matches!(
        reconcile(ReconnectCurrentAuthorityV1::from_record(&record, 105)?)?,
        ReconnectProjectionDecisionV2::InstallController { .. }
    ));
    assert_eq!(
        reconcile(ReconnectCurrentAuthorityV1::from_record(&record, 106)?),
        Err(ReconnectDurabilityErrorV1::ReconciliationMismatch)
    );
    let mut stale = ReconnectCurrentAuthorityV1::from_record(&record, 105)?;
    stale.session_state = GameSessionState::Terminal;
    assert_eq!(
        reconcile(stale),
        Err(ReconnectDurabilityErrorV1::ReconciliationMismatch)
    );
    Ok(())
}
