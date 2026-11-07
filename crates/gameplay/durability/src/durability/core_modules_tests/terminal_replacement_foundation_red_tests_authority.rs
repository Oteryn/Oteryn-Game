use crate::foundation::{
    AccountPresenceClaimV1, AuthenticatedTransportRefV1, AuthorityEvidenceFenceV1, ChannelId,
    CharacterId, CharacterLease, CharacterWorldEligibilityClaimV1, CommandId, ConnectionGeneration,
    ControlLossEpochRefV1, Fnd02ReconciliationFenceV1, FreshAdmissionCommit, FreshAdmissionFacts,
    GameSessionAuthoritySnapshot, GameSessionId, GameSessionState, PendingCommandDispositionV1,
    PendingCommandReconciliationV1, ProtectionEntitlementV1, ReconnectAttemptBudgetV1,
    ReconnectAttemptRef, ReconnectAttemptReservationV1, ReconnectAuthorityFenceV1,
    ReconnectCandidateBindingV1, ReconnectCompatibilityEvidenceV1, ReconnectConnectionFenceV1,
    ReconnectContinuityV1, ReconnectCurrentAuthorityV1, ReconnectDurabilityErrorV1,
    ReconnectDurabilityFlowV1, ReconnectDurabilityFlowV2, ReconnectDurabilityPhaseV1,
    ReconnectDurabilityRecordV1, ReconnectDurableOutcomeV2,
    ReconnectDurableReconciliationSnapshotV1, ReconnectDurableReconciliationSnapshotV2,
    ReconnectDurableTerminalDispositionV1, ReconnectIdentityV1, ReconnectPrepareActionV1,
    ReconnectPrepareCompletionV1, ReconnectPrepareCompletionV2, ReconnectPrepareDispositionV1,
    ReconnectPrepareDispositionV2, ReconnectProjectionDecisionV1, ReconnectProjectionDecisionV2,
    ReconnectProofV1, RuntimeScopeRefV1, ScopeOwnershipGeneration, StateDomainRevisionV1,
    TerminalGameSessionReplacementAuthorizationV1, WorldId,
};

const ACCOUNT: &str = "123e4567-e89b-12d3-a456-426614174000";
const OTHER_ACCOUNT: &str = "123e4567-e89b-12d3-a456-426614174001";

fn exact_current_authority(
    record: &ReconnectDurabilityRecordV1,
    observed_at: i64,
) -> Result<ReconnectCurrentAuthorityV1, ReconnectDurabilityErrorV1> {
    current_authority(
        record,
        Some(AccountPresenceClaimV1::new(
            record.identity().account_id(),
            record.identity().character_id(),
        )?),
        observed_at,
    )
}

fn current_authority(
    record: &ReconnectDurabilityRecordV1,
    current_account_presence: Option<AccountPresenceClaimV1>,
    observed_at: i64,
) -> Result<ReconnectCurrentAuthorityV1, ReconnectDurabilityErrorV1> {
    ReconnectCurrentAuthorityV1::from_current_facts(
        record,
        current_account_presence,
        Some(CharacterWorldEligibilityClaimV1::new(
            record.identity().character_id(),
            record.identity().world_id(),
        )),
        Some(ReconnectCandidateBindingV1::new(
            record.identity().game_session_id(),
            record.identity().reconnect_attempt_ref(),
            record.connection().candidate(),
            record.connection().transport_ref(),
            record.continuity().prepared_deadline(),
        )?),
        record.identity().runtime_scope(),
        record.connection().predecessor(),
        record.authority(),
        record.continuity().control_loss_epoch(),
        record.continuity().original_grace_deadline(),
        record.proof().clone(),
        record.fnd02().clone(),
        record.compatibility().clone(),
        GameSessionState::Reconnectable,
        false,
        observed_at,
    )
}

#[test]
fn v1_committed_reconciliation_requires_complete_current_authority() {
    let record = candidate_record(20, ACCOUNT, 11, 12, 7, 9, 10, 1).expect("record");
    let committed = ReconnectDurableReconciliationSnapshotV1::committed(record.clone());

    let (mut exact_flow, exact_request) = ReconnectDurabilityFlowV1::begin(record.clone());
    exact_flow
        .accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
            &exact_request,
            ReconnectPrepareDispositionV1::Ambiguous,
        ))
        .expect("ambiguous prepare enters reconciliation");
    let exact = exact_current_authority(&record, 105).expect("exact current authority");
    assert_eq!(
        exact_flow.accept_reconciliation(committed.clone(), exact),
        Ok(ReconnectProjectionDecisionV1::InstallController {
            generation: record.connection().candidate(),
            transport_ref: record.connection().transport_ref(),
        })
    );

    let (mut stale_flow, stale_request) = ReconnectDurabilityFlowV1::begin(record.clone());
    stale_flow
        .accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
            &stale_request,
            ReconnectPrepareDispositionV1::Ambiguous,
        ))
        .expect("ambiguous prepare enters reconciliation");
    let stale = current_authority(&record, None, 104).expect("stale current authority");
    assert_eq!(
        stale_flow.accept_reconciliation(committed, stale),
        Err(ReconnectDurabilityErrorV1::ReconciliationMismatch)
    );
}

fn uuid_v7(raw: u64) -> [u8; 16] {
    let mut value = [0_u8; 16];
    value[8..].copy_from_slice(&raw.to_be_bytes());
    value[6] = 0x70;
    value[8] = (value[8] & 0x3f) | 0x80;
    value
}

fn game_session(raw: u64) -> Result<GameSessionId, ReconnectDurabilityErrorV1> {
    GameSessionId::decode(&uuid_v7(raw)).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)
}

fn character(raw: u64) -> Result<CharacterId, ReconnectDurabilityErrorV1> {
    CharacterId::decode(&uuid_v7(raw)).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)
}

fn world(raw: u64) -> Result<WorldId, ReconnectDurabilityErrorV1> {
    WorldId::decode(&uuid_v7(raw)).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)
}

fn channel(raw: u64) -> Result<ChannelId, ReconnectDurabilityErrorV1> {
    ChannelId::decode(&uuid_v7(raw)).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)
}

fn candidate_record(
    session_raw: u64,
    account_id: &str,
    character_raw: u64,
    world_raw: u64,
    predecessor_generation: u64,
    lease_generation: u64,
    scope_generation: u64,
    attempt: u64,
) -> Result<ReconnectDurabilityRecordV1, ReconnectDurabilityErrorV1> {
    candidate_record_with_evidence(
        session_raw,
        account_id,
        character_raw,
        world_raw,
        predecessor_generation,
        lease_generation,
        scope_generation,
        attempt,
        100,
        101,
    )
}

#[allow(clippy::too_many_arguments)]
fn candidate_record_with_evidence(
    session_raw: u64,
    account_id: &str,
    character_raw: u64,
    world_raw: u64,
    predecessor_generation: u64,
    lease_generation: u64,
    scope_generation: u64,
    attempt: u64,
    platform_source_observed_at: i64,
    trust_source_observed_at: i64,
) -> Result<ReconnectDurabilityRecordV1, ReconnectDurabilityErrorV1> {
    let world_id = world(world_raw)?;
    let identity = ReconnectIdentityV1::new(
        game_session(session_raw)?,
        ReconnectAttemptRef::new(attempt).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        account_id,
        character(character_raw)?,
        world_id,
        RuntimeScopeRefV1::channel(world_id, channel(13)?),
    )?;
    let connection = ReconnectConnectionFenceV1::new(
        ConnectionGeneration::new(predecessor_generation)
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        ConnectionGeneration::new(predecessor_generation + 1)
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        AuthenticatedTransportRefV1::decode(&[0x71; 16])
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
    )?;
    let authority = ReconnectAuthorityFenceV1::new(
        lease_generation,
        ScopeOwnershipGeneration::new(scope_generation)
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
        vec![
            PendingCommandReconciliationV1::new(
                CommandId::new(1).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
                PendingCommandDispositionV1::PendingOriginal,
            ),
            PendingCommandReconciliationV1::new(
                CommandId::new(2).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
                PendingCommandDispositionV1::TerminalOutcomeRetained,
            ),
        ],
        41,
        vec![
            StateDomainRevisionV1::new(1, 4)?,
            StateDomainRevisionV1::new(2, 7)?,
        ],
    )?;
    let platform = AuthorityEvidenceFenceV1::new(
        "platform-security",
        "reconnect",
        "account",
        "sec:17",
        "decision:sec:17",
        platform_source_observed_at,
    )?;
    let trust = AuthorityEvidenceFenceV1::new(
        "proof-trust",
        "reconnect",
        "recovery-key",
        "trust:21",
        "decision:trust:21",
        trust_source_observed_at,
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

fn prepared_v1_flow(record: &ReconnectDurabilityRecordV1) -> ReconnectDurabilityFlowV1 {
    let (mut flow, request) = ReconnectDurabilityFlowV1::begin(record.clone());
    flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
        &request,
        ReconnectPrepareDispositionV1::Prepared,
    ))
    .expect("prepare completion");
    flow
}

fn prepared_v2_flow(record: &ReconnectDurabilityRecordV1) -> ReconnectDurabilityFlowV2 {
    let mut budget = ReconnectAttemptBudgetV1::new(record.continuity().control_loss_epoch());
    budget
        .reserve(
            record.identity().reconnect_attempt_ref(),
            record.connection().transport_ref(),
        )
        .expect("reserve");
    let (mut flow, request) = ReconnectDurabilityFlowV2::begin(record.clone(), None);
    flow.accept_prepare_completion(
        ReconnectPrepareCompletionV2::for_request(
            &request,
            ReconnectPrepareDispositionV2::Prepared,
        ),
        &mut budget,
    )
    .expect("prepare completion");
    flow
}

#[test]
fn commit_authorization_rejects_future_authenticated_evidence_and_accepts_equal_timestamps() {
    let exact = candidate_record_with_evidence(20, ACCOUNT, 11, 12, 7, 9, 10, 1, 105, 105)
        .expect("exact record");
    assert!(
        prepared_v1_flow(&exact)
            .authorize_commit(exact_current_authority(&exact, 105).expect("current"), 105)
            .is_ok()
    );
    assert!(
        prepared_v2_flow(&exact)
            .authorize_commit(exact_current_authority(&exact, 105).expect("current"), 105)
            .is_ok()
    );

    for (platform_at, trust_at) in [(106, 105), (105, 106)] {
        let record =
            candidate_record_with_evidence(20, ACCOUNT, 11, 12, 7, 9, 10, 1, platform_at, trust_at)
                .expect("future-evidence record");
        assert_eq!(
            prepared_v1_flow(&record)
                .authorize_commit(exact_current_authority(&record, 105).expect("current"), 105,),
            Err(ReconnectDurabilityErrorV1::StaleAuthority)
        );
        assert_eq!(
            prepared_v2_flow(&record)
                .authorize_commit(exact_current_authority(&record, 105).expect("current"), 105,),
            Err(ReconnectDurabilityErrorV1::StaleAuthority)
        );
    }
}

#[test]
fn committed_reconciliation_rejects_authority_observed_before_authenticated_evidence() {
    for (platform_at, trust_at) in [(106, 105), (105, 106)] {
        let record =
            candidate_record_with_evidence(20, ACCOUNT, 11, 12, 7, 9, 10, 1, platform_at, trust_at)
                .expect("future-evidence record");
        let committed = ReconnectDurableReconciliationSnapshotV1::committed(record.clone());
        let (mut flow, request) = ReconnectDurabilityFlowV1::begin(record.clone());
        flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
            &request,
            ReconnectPrepareDispositionV1::Ambiguous,
        ))
        .expect("ambiguous prepare");
        assert_eq!(
            flow.accept_reconciliation(
                committed,
                exact_current_authority(&record, 105).expect("current"),
            ),
            Err(ReconnectDurabilityErrorV1::ReconciliationMismatch)
        );
    }
}

fn predecessor_snapshot(
    state: GameSessionState,
    current_transport: Option<AuthenticatedTransportRefV1>,
    current_scope: u64,
) -> Result<GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>, ReconnectDurabilityErrorV1> {
    let facts =
        FreshAdmissionFacts::new([0x44; 32], character(11)?, world(12)?, channel(13)?, 9, 10)
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
    let initial_transport = AuthenticatedTransportRefV1::decode(&[0x70; 16])
        .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
    let commit = FreshAdmissionCommit::from_facts(game_session(10)?, facts, initial_transport)
        .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
    GameSessionAuthoritySnapshot::from_current_facts(
        commit,
        state,
        ConnectionGeneration::new(7).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        current_transport,
        CharacterLease::new(character(11)?, 9)
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        Some(CharacterWorldEligibilityClaimV1::new(
            character(11)?,
            world(12)?,
        )),
        RuntimeScopeRefV1::channel(world(12)?, channel(13)?),
        ScopeOwnershipGeneration::new(current_scope)
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
    )?
    .with_control_loss_continuity(ControlLossEpochRefV1::new(3)?, 120)
}

fn authorize(
    snapshot: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
    candidate: &ReconnectDurabilityRecordV1,
    expected_predecessor: GameSessionId,
    expected_candidate: GameSessionId,
) -> Result<TerminalGameSessionReplacementAuthorizationV1, ReconnectDurabilityErrorV1> {
    TerminalGameSessionReplacementAuthorizationV1::from_current_authority(
        ACCOUNT,
        Some(&AccountPresenceClaimV1::new(
            candidate.identity().account_id(),
            candidate.identity().character_id(),
        )?),
        expected_predecessor,
        expected_candidate,
        snapshot,
        candidate,
    )
}
