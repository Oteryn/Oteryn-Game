use super::*;

fn uuid_v7(raw: u64) -> [u8; 16] {
    let mut out = [0u8; 16];
    out[8..].copy_from_slice(&raw.to_be_bytes());
    out[6] = 0x70;
    out[8] = (out[8] & 0x3f) | 0x80;
    out
}

fn sample_record(attempt_raw: u64, transport_byte: u8) -> Result<ReconnectDurabilityRecordV1, ReconnectDurabilityErrorV1> {
    let game_session_id = GameSessionId::decode(&uuid_v7(10)).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
    let character_id = CharacterId::decode(&uuid_v7(11)).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
    let world_id = WorldId::decode(&uuid_v7(12)).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
    let channel_id = ChannelId::decode(&uuid_v7(13)).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
    let identity = ReconnectIdentityV1::new(game_session_id, ReconnectAttemptRef::new(attempt_raw).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?, "123e4567-e89b-12d3-a456-426614174000", character_id, world_id, RuntimeScopeRefV1::channel(world_id, channel_id))?;
    let predecessor = ConnectionGeneration::new(7).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
    let candidate = ConnectionGeneration::new(8).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
    let transport_ref = AuthenticatedTransportRefV1::decode(&[transport_byte; 16]).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
    let connection = ReconnectConnectionFenceV1::new(predecessor, candidate, transport_ref)?;
    let authority = ReconnectAuthorityFenceV1::new(9, ScopeOwnershipGeneration::new(10).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?)?;
    let continuity = ReconnectContinuityV1::new(ControlLossEpochRefV1::new(3)?, 120, 115, ProtectionEntitlementV1::unused())?;
    let fnd02 = Fnd02ReconciliationFenceV1::new(CommandId::new(3).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?, vec![PendingCommandReconciliationV1::new(CommandId::new(1).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?, PendingCommandDispositionV1::PendingOriginal), PendingCommandReconciliationV1::new(CommandId::new(2).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?, PendingCommandDispositionV1::TerminalOutcomeRetained)], 41, vec![StateDomainRevisionV1::new(1, 4)?, StateDomainRevisionV1::new(2, 7)?])?;
    let platform = AuthorityEvidenceFenceV1::new("platform-security", "reconnect", "account", "sec:17", "decision:sec:17", 100)?;
    let trust = AuthorityEvidenceFenceV1::new("proof-trust", "reconnect", "recovery-key", "trust:21", "decision:trust:21", 101)?;
    let compatibility = ReconnectCompatibilityEvidenceV1::new(1, 1, "rules:1", "content:2", "map:3", "world:4", 12, platform, trust, Some(110))?;
    ReconnectDurabilityRecordV1::new(identity, connection, authority, continuity, ReconnectProofV1::ReauthenticatedRecovery { recovery_grant_nonce: [0x55; 32] }, fnd02, compatibility)
}

#[test]
fn authenticated_transport_ref_v1_is_exact_nonzero_16_bytes() -> Result<(), AdmissionError> {
    let encoded = [0xA5u8; 16];
    let transport_ref = AuthenticatedTransportRefV1::decode(&encoded)?;
    assert_eq!(transport_ref.to_bytes(), encoded);
    assert_eq!(AuthenticatedTransportRefV1::decode(&[0u8; 16]), Err(AdmissionError::InvalidFacts));
    assert_eq!(AuthenticatedTransportRefV1::decode(&[0xA5u8; 15]), Err(AdmissionError::InvalidFacts));
    Ok(())
}

#[test]
fn same_attempt_ref_is_immutable_and_ninth_attempt_fails_before_allocation() -> Result<(), ReconnectDurabilityErrorV1> {
    let mut budget = ReconnectAttemptBudgetV1::new(ControlLossEpochRefV1::new(9)?);
    let first_attempt = ReconnectAttemptRef::new(1).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
    let first_ref = AuthenticatedTransportRefV1::decode(&[1u8; 16]).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
    let changed_ref = AuthenticatedTransportRefV1::decode(&[2u8; 16]).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
    assert_eq!(budget.reserve(first_attempt, first_ref)?, ReconnectAttemptReservationV1::New);
    assert_eq!(budget.reserve(first_attempt, first_ref)?, ReconnectAttemptReservationV1::Existing);
    assert_eq!(budget.reserve(first_attempt, changed_ref), Err(ReconnectDurabilityErrorV1::IdempotencyConflict));
    for raw in 2u64..=8 {
        let byte = u8::try_from(raw).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        assert_eq!(budget.reserve(ReconnectAttemptRef::new(raw).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?, AuthenticatedTransportRefV1::decode(&[byte; 16]).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?)?, ReconnectAttemptReservationV1::New);
    }
    assert_eq!(budget.reserve(ReconnectAttemptRef::new(9).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?, AuthenticatedTransportRefV1::decode(&[9u8; 16]).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?), Err(ReconnectDurabilityErrorV1::AttemptCapacityExceeded));
    assert_eq!(budget.distinct_attempts(), 8);
    Ok(())
}

#[test]
fn collision_allows_only_new_attempt_under_capacity_and_never_same_attempt_remint() -> Result<(), ReconnectDurabilityErrorV1> {
    let mut budget = ReconnectAttemptBudgetV1::new(ControlLossEpochRefV1::new(1)?);
    let attempt = ReconnectAttemptRef::new(1).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
    let transport_ref = AuthenticatedTransportRefV1::decode(&[1u8; 16]).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
    budget.reserve(attempt, transport_ref)?;
    budget.accept_prepare_completion(attempt, transport_ref, ReconnectPrepareDispositionV1::RejectedTransportRefCollision)?;
    assert!(budget.replacement_allowed_after_collision(attempt));
    assert_eq!(budget.reserve(attempt, AuthenticatedTransportRefV1::decode(&[2u8; 16]).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?), Err(ReconnectDurabilityErrorV1::IdempotencyConflict));
    assert_eq!(budget.reserve(ReconnectAttemptRef::new(2).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?, AuthenticatedTransportRefV1::decode(&[2u8; 16]).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?)?, ReconnectAttemptReservationV1::New);
    let mut exhausted = ReconnectAttemptBudgetV1::new(ControlLossEpochRefV1::new(2)?);
    for raw in 1u64..=8 {
        let byte = u8::try_from(raw).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        exhausted.reserve(ReconnectAttemptRef::new(raw).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?, AuthenticatedTransportRefV1::decode(&[byte; 16]).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?)?;
    }
    exhausted.accept_prepare_completion(ReconnectAttemptRef::new(8).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?, AuthenticatedTransportRefV1::decode(&[8u8; 16]).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?, ReconnectPrepareDispositionV1::RejectedTransportRefCollision)?;
    assert!(!exhausted.replacement_allowed_after_collision(ReconnectAttemptRef::new(8).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?));
    Ok(())
}

#[test]
fn record_preserves_complete_authority_reconciliation_and_security_evidence() -> Result<(), ReconnectDurabilityErrorV1> {
    let record = sample_record(3, 3)?;
    assert_eq!(record.version(), 1);
    assert_eq!(record.identity().account_id(), "123e4567-e89b-12d3-a456-426614174000");
    assert!(matches!(record.identity().runtime_scope(), RuntimeScopeRefV1::Channel { .. }));
    assert_eq!(record.connection().predecessor().get(), 7);
    assert_eq!(record.connection().candidate().get(), 8);
    assert_eq!(record.authority().character_lease_generation(), 9);
    assert_eq!(record.continuity().control_loss_epoch().get(), 3);
    assert_eq!(record.fnd02().pending().len(), 2);
    assert_eq!(record.fnd02().domain_revisions().len(), 2);
    assert_eq!(record.compatibility().protocol_major(), 1);
    assert_eq!(record.compatibility().transport_profile(), 1);
    assert_eq!(record.compatibility().account_security_generation(), 12);
    assert_eq!(record.compatibility().credential_expiration(), Some(110));
    assert_eq!(record.compatibility().platform_security_evidence().source_observed_at(), 100);
    assert_eq!(record.compatibility().proof_trust_evidence().source_observed_at(), 101);
    Ok(())
}

#[test]
fn prepare_unavailable_retries_same_request_and_ambiguous_requires_reconciliation() -> Result<(), ReconnectDurabilityErrorV1> {
    let record = sample_record(4, 4)?;
    let (mut unavailable_flow, request) = ReconnectDurabilityFlowV1::begin(record.clone());
    assert_eq!(unavailable_flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(&request, ReconnectPrepareDispositionV1::Unavailable))?, ReconnectPrepareActionV1::RetrySameRequest(request.clone()));
    assert_eq!(unavailable_flow.phase(), ReconnectDurabilityPhaseV1::PendingPrepare);
    let (mut ambiguous_flow, ambiguous_request) = ReconnectDurabilityFlowV1::begin(record);
    assert_eq!(ambiguous_flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(&ambiguous_request, ReconnectPrepareDispositionV1::Ambiguous))?, ReconnectPrepareActionV1::ReconcileSameAttempt);
    assert_eq!(ambiguous_flow.phase(), ReconnectDurabilityPhaseV1::ReconciliationRequired);
    Ok(())
}

#[test]
fn ambiguous_same_attempt_reconciliation_projects_terminal() -> Result<(), ReconnectDurabilityErrorV1> {
    let record = sample_record(7, 7)?;
    let (mut flow, request) = ReconnectDurabilityFlowV1::begin(record.clone());
    assert_eq!(flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(&request, ReconnectPrepareDispositionV1::Ambiguous))?, ReconnectPrepareActionV1::ReconcileSameAttempt);
    let snapshot = ReconnectDurableReconciliationSnapshotV1::terminal(record.clone());
    assert_eq!(snapshot.record, record);
    assert_eq!(snapshot.durable_state, DurableReconnectStateV1::Terminal);
    assert_eq!(snapshot.current_generation, None);
    assert_eq!(snapshot.current_transport_ref, None);
    assert_eq!(flow.accept_reconciliation(snapshot, ReconnectCurrentAuthorityV1::from_record(&record, 105)?)?, ReconnectProjectionDecisionV1::Terminal);
    assert_eq!(flow.phase(), ReconnectDurabilityPhaseV1::Terminal);
    Ok(())
}

#[test]
fn prepared_completion_requires_fresh_complete_revalidation_before_commit() -> Result<(), ReconnectDurabilityErrorV1> {
    let record = sample_record(5, 5)?;
    let (mut flow, request) = ReconnectDurabilityFlowV1::begin(record.clone());
    assert_eq!(flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(&request, ReconnectPrepareDispositionV1::Prepared))?, ReconnectPrepareActionV1::AwaitFinalRevalidation);
    let current = ReconnectCurrentAuthorityV1::from_record(&record, 105)?;
    let commit = flow.authorize_commit(current, 104)?;
    assert_eq!(commit.authorization().authorization_deadline(), 105);
    assert_eq!(flow.phase(), ReconnectDurabilityPhaseV1::PendingCommit);
    let (mut stale_flow, stale_request) = ReconnectDurabilityFlowV1::begin(record.clone());
    stale_flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(&stale_request, ReconnectPrepareDispositionV1::Prepared))?;
    let mut stale = ReconnectCurrentAuthorityV1::from_record(&record, 105)?;
    stale.account_security_generation = stale.account_security_generation.checked_add(1).ok_or(ReconnectDurabilityErrorV1::InvalidRecord)?;
    assert_eq!(stale_flow.authorize_commit(stale, 104), Err(ReconnectDurabilityErrorV1::StaleAuthority));
    Ok(())
}

#[test]
fn ambiguous_commit_installs_controller_only_after_exact_committed_reconciliation() -> Result<(), ReconnectDurabilityErrorV1> {
    let record = sample_record(6, 6)?;
    let (mut flow, request) = ReconnectDurabilityFlowV1::begin(record.clone());
    flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(&request, ReconnectPrepareDispositionV1::Prepared))?;
    let commit_request = flow.authorize_commit(ReconnectCurrentAuthorityV1::from_record(&record, 105)?, 104)?;
    assert_eq!(flow.accept_commit_completion(ReconnectCommitCompletionV1::for_request(&commit_request, ReconnectCommitDispositionV1::Ambiguous))?, ReconnectCommitActionV1::ReconcileSameAttempt);
    assert_eq!(flow.phase(), ReconnectDurabilityPhaseV1::ReconciliationRequired);
    assert_eq!(flow.accept_reconciliation(ReconnectDurableReconciliationSnapshotV1::committed(record.clone()), ReconnectCurrentAuthorityV1::from_record(&record, 105)?)?, ReconnectProjectionDecisionV1::InstallController { generation: record.connection().candidate(), transport_ref: record.connection().transport_ref() });
    let (mut mismatch_flow, mismatch_request) = ReconnectDurabilityFlowV1::begin(record.clone());
    mismatch_flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(&mismatch_request, ReconnectPrepareDispositionV1::Ambiguous))?;
    let mut mismatch = ReconnectDurableReconciliationSnapshotV1::committed(record.clone());
    mismatch.current_transport_ref = Some(AuthenticatedTransportRefV1::decode(&[7u8; 16]).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?);
    assert_eq!(mismatch_flow.accept_reconciliation(mismatch, ReconnectCurrentAuthorityV1::from_record(&record, 105)?), Err(ReconnectDurabilityErrorV1::ReconciliationMismatch));
    Ok(())
}
