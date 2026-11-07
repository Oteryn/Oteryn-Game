#[test]
fn v2_replacement_root_ready_miss_is_fail_closed_without_connect() {
    use std::net::{IpAddr, Ipv4Addr};

    let candidate = candidate_record(20, ACCOUNT, 11, 12, 7, 9, 10, 1).expect("candidate record");
    let authorization = authorize(
        predecessor_snapshot(GameSessionState::Terminal, None, 10).expect("snapshot"),
        &candidate,
        game_session(10).expect("predecessor session"),
        game_session(20).expect("candidate session"),
    )
    .expect("replacement authorization");
    let request = ReconnectDurabilityFlowV2::begin(candidate, Some(authorization)).1;

    let root = super::DurabilityRoot::new(
        super::DurabilityRootConfig::new_with_isolated_test_ledger(
            IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7)),
            5432,
            "db.example",
            "oteryn",
            "explicit",
            "test-secret",
            b"-----BEGIN CERTIFICATE-----\nAA==\n-----END CERTIFICATE-----\n",
        )
        .expect("root config"),
    )
    .expect("durability root");
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime")
        .block_on(async {
            let journal = super::AdmissionReconnectJournalV2::from_root(root.clone());

            assert!(matches!(
                journal.prepare(&request).await,
                Err(super::DurabilityError::RootUnavailable)
            ));
            assert!(!root.is_ready());
            assert!(root.has_ready_demand());
            tokio::task::yield_now().await;
            assert!(
                !root.is_ready(),
                "V2 replacement ready miss must not establish a connection in the background"
            );
        });
    drop(root);
}

#[test]
fn v2_final_revalidation_accepts_external_current_facts_and_rejects_changed_authority() {
    let record = candidate_record(20, ACCOUNT, 11, 12, 7, 9, 10, 1).expect("record");
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

    let exact_current = ReconnectCurrentAuthorityV1::from_current_facts(
        &record,
        Some(
            AccountPresenceClaimV1::new(
                record.identity().account_id(),
                record.identity().character_id(),
            )
            .expect("presence"),
        ),
        Some(CharacterWorldEligibilityClaimV1::new(
            record.identity().character_id(),
            record.identity().world_id(),
        )),
        Some(
            ReconnectCandidateBindingV1::new(
                record.identity().game_session_id(),
                record.identity().reconnect_attempt_ref(),
                record.connection().candidate(),
                record.connection().transport_ref(),
                record.continuity().prepared_deadline(),
            )
            .expect("candidate binding"),
        ),
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
        105,
    )
    .expect("exact current authority");
    assert!(flow.authorize_commit(exact_current, 104).is_ok());

    let mut changed_budget =
        ReconnectAttemptBudgetV1::new(record.continuity().control_loss_epoch());
    changed_budget
        .reserve(
            record.identity().reconnect_attempt_ref(),
            record.connection().transport_ref(),
        )
        .expect("changed reserve");
    let (mut changed_flow, changed_request) =
        ReconnectDurabilityFlowV2::begin(record.clone(), None);
    changed_flow
        .accept_prepare_completion(
            ReconnectPrepareCompletionV2::for_request(
                &changed_request,
                ReconnectPrepareDispositionV2::Prepared,
            ),
            &mut changed_budget,
        )
        .expect("changed prepare completion");
    let changed_authority = ReconnectAuthorityFenceV1::new(
        record.authority().character_lease_generation() + 1,
        record.authority().scope_ownership_generation(),
    )
    .expect("changed authority");
    let changed_current = ReconnectCurrentAuthorityV1::from_current_facts(
        &record,
        Some(
            AccountPresenceClaimV1::new(
                record.identity().account_id(),
                record.identity().character_id(),
            )
            .expect("presence"),
        ),
        Some(CharacterWorldEligibilityClaimV1::new(
            record.identity().character_id(),
            record.identity().world_id(),
        )),
        Some(
            ReconnectCandidateBindingV1::new(
                record.identity().game_session_id(),
                record.identity().reconnect_attempt_ref(),
                record.connection().candidate(),
                record.connection().transport_ref(),
                record.continuity().prepared_deadline(),
            )
            .expect("candidate binding"),
        ),
        record.identity().runtime_scope(),
        record.connection().predecessor(),
        changed_authority,
        record.continuity().control_loss_epoch(),
        record.continuity().original_grace_deadline(),
        record.proof().clone(),
        record.fnd02().clone(),
        record.compatibility().clone(),
        GameSessionState::Reconnectable,
        false,
        105,
    )
    .expect("changed current authority");

    assert_eq!(
        changed_flow.authorize_commit(changed_current, 104),
        Err(ReconnectDurabilityErrorV1::StaleAuthority)
    );
}

#[test]
fn v2_final_authority_revalidation_requires_current_account_presence() {
    let record = candidate_record(20, ACCOUNT, 11, 12, 7, 9, 10, 1).expect("record");
    let exact_presence = AccountPresenceClaimV1::new(
        record.identity().account_id(),
        record.identity().character_id(),
    )
    .expect("account presence");
    let reassigned_presence =
        AccountPresenceClaimV1::new(ACCOUNT, character(99).expect("character"))
            .expect("reassigned account presence");

    let current = |presence| {
        ReconnectCurrentAuthorityV1::from_current_facts(
            &record,
            presence,
            Some(CharacterWorldEligibilityClaimV1::new(
                record.identity().character_id(),
                record.identity().world_id(),
            )),
            Some(
                ReconnectCandidateBindingV1::new(
                    record.identity().game_session_id(),
                    record.identity().reconnect_attempt_ref(),
                    record.connection().candidate(),
                    record.connection().transport_ref(),
                    record.continuity().prepared_deadline(),
                )
                .expect("candidate binding"),
            ),
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
            105,
        )
        .expect("current authority")
    };
    let flow = || {
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
    };

    assert!(
        flow()
            .authorize_commit(current(Some(exact_presence.clone())), 104)
            .is_ok()
    );
    assert_eq!(
        flow().authorize_commit(current(None), 104),
        Err(ReconnectDurabilityErrorV1::StaleAuthority)
    );
    assert_eq!(
        flow().authorize_commit(current(Some(reassigned_presence.clone())), 104),
        Err(ReconnectDurabilityErrorV1::StaleAuthority)
    );

    let committed = ReconnectDurableReconciliationSnapshotV2::new(
        record.clone(),
        ReconnectDurableOutcomeV2::Committed {
            current_generation: record.connection().candidate(),
            current_transport_ref: record.connection().transport_ref(),
        },
    );
    let reconcile = |presence| {
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
                ReconnectPrepareDispositionV2::Ambiguous,
            ),
            &mut budget,
        )
        .expect("prepare completion");
        flow.accept_reconciliation(committed.clone(), current(presence), &mut budget)
    };
    assert!(matches!(
        reconcile(Some(exact_presence)),
        Ok(ReconnectProjectionDecisionV2::InstallController { .. })
    ));
    assert_eq!(
        reconcile(None),
        Err(ReconnectDurabilityErrorV1::ReconciliationMismatch)
    );
    assert_eq!(
        reconcile(Some(reassigned_presence)),
        Err(ReconnectDurabilityErrorV1::ReconciliationMismatch)
    );
}
