#[test]
fn terminal_replacement_authorization_rejects_candidate_loss_epoch_mismatch() {
    let record = candidate_record(20, ACCOUNT, 11, 12, 7, 9, 10, 1).expect("record");
    let mismatched_continuity = ReconnectContinuityV1::new(
        ControlLossEpochRefV1::new(4).expect("epoch"),
        record.continuity().original_grace_deadline(),
        record.continuity().prepared_deadline(),
        record.continuity().protection_entitlement(),
    )
    .expect("continuity");
    let mismatched = ReconnectDurabilityRecordV1::new(
        record.identity().clone(),
        record.connection(),
        record.authority(),
        mismatched_continuity,
        record.proof().clone(),
        record.fnd02().clone(),
        record.compatibility().clone(),
    )
    .expect("mismatched record");

    assert!(
        authorize(
            predecessor_snapshot(GameSessionState::Terminal, None, 10).expect("snapshot"),
            &mismatched,
            game_session(10).expect("predecessor"),
            game_session(20).expect("candidate"),
        )
        .is_err()
    );
}

#[test]
fn terminal_replacement_authorization_rejects_candidate_original_deadline_mismatch() {
    let record = candidate_record(20, ACCOUNT, 11, 12, 7, 9, 10, 1).expect("record");
    let mismatched_continuity = ReconnectContinuityV1::new(
        record.continuity().control_loss_epoch(),
        record.continuity().original_grace_deadline() + 1,
        record.continuity().prepared_deadline(),
        record.continuity().protection_entitlement(),
    )
    .expect("continuity");
    let mismatched = ReconnectDurabilityRecordV1::new(
        record.identity().clone(),
        record.connection(),
        record.authority(),
        mismatched_continuity,
        record.proof().clone(),
        record.fnd02().clone(),
        record.compatibility().clone(),
    )
    .expect("mismatched record");

    assert!(
        authorize(
            predecessor_snapshot(GameSessionState::Terminal, None, 10).expect("snapshot"),
            &mismatched,
            game_session(10).expect("predecessor"),
            game_session(20).expect("candidate"),
        )
        .is_err()
    );
}

#[test]
fn terminal_replacement_authorization_requires_terminal_transportless_predecessor() {
    let candidate = candidate_record(20, ACCOUNT, 11, 12, 7, 9, 11, 1).expect("candidate");
    let predecessor = game_session(10).expect("predecessor");
    let candidate_id = game_session(20).expect("candidate id");
    assert!(
        authorize(
            predecessor_snapshot(
                GameSessionState::Active,
                Some(AuthenticatedTransportRefV1::decode(&[0x70; 16]).expect("transport"),),
                11,
            )
            .expect("snapshot"),
            &candidate,
            predecessor,
            candidate_id,
        )
        .is_err()
    );
    assert!(
        authorize(
            predecessor_snapshot(GameSessionState::Reconnectable, None, 11).expect("snapshot"),
            &candidate,
            predecessor,
            candidate_id,
        )
        .is_err()
    );
    assert!(
        authorize(
            predecessor_snapshot(
                GameSessionState::Terminal,
                Some(AuthenticatedTransportRefV1::decode(&[0x70; 16]).expect("transport"),),
                11,
            )
            .expect("snapshot"),
            &candidate,
            predecessor,
            candidate_id,
        )
        .is_err()
    );
    assert!(
        authorize(
            predecessor_snapshot(GameSessionState::Terminal, None, 11).expect("snapshot"),
            &candidate,
            predecessor,
            candidate_id,
        )
        .is_ok()
    );
}

#[test]
fn terminal_replacement_authorization_carries_current_scope_not_only_committed_scope() {
    let candidate = candidate_record(20, ACCOUNT, 11, 12, 7, 9, 11, 1).expect("candidate");
    let authorization = authorize(
        predecessor_snapshot(GameSessionState::Terminal, None, 11).expect("snapshot"),
        &candidate,
        game_session(10).expect("predecessor"),
        game_session(20).expect("candidate"),
    )
    .expect("terminal authorization");
    assert_eq!(
        authorization
            .predecessor_current_scope_ownership_generation()
            .get(),
        11
    );
}

#[test]
fn terminal_replacement_authorization_rejects_predecessor_session_mismatch() {
    let candidate = candidate_record(20, ACCOUNT, 11, 12, 7, 9, 11, 1).expect("candidate");
    assert!(
        authorize(
            predecessor_snapshot(GameSessionState::Terminal, None, 11).expect("snapshot"),
            &candidate,
            game_session(99).expect("wrong predecessor"),
            game_session(20).expect("candidate"),
        )
        .is_err()
    );
}

#[test]
fn terminal_replacement_authorization_rejects_predecessor_connection_generation_mismatch() {
    let candidate = candidate_record(20, ACCOUNT, 11, 12, 6, 9, 11, 1).expect("candidate");
    assert!(
        authorize(
            predecessor_snapshot(GameSessionState::Terminal, None, 11).expect("snapshot"),
            &candidate,
            game_session(10).expect("predecessor"),
            game_session(20).expect("candidate"),
        )
        .is_err()
    );
}

#[test]
fn terminal_replacement_authorization_rejects_predecessor_lease_generation_mismatch() {
    let candidate = candidate_record(20, ACCOUNT, 11, 12, 7, 10, 11, 1).expect("candidate");
    assert!(
        authorize(
            predecessor_snapshot(GameSessionState::Terminal, None, 11).expect("snapshot"),
            &candidate,
            game_session(10).expect("predecessor"),
            game_session(20).expect("candidate"),
        )
        .is_err()
    );
}

#[test]
fn terminal_replacement_authorization_rejects_candidate_session_mismatch() {
    let candidate = candidate_record(20, ACCOUNT, 11, 12, 7, 9, 11, 1).expect("candidate");
    assert!(
        authorize(
            predecessor_snapshot(GameSessionState::Terminal, None, 11).expect("snapshot"),
            &candidate,
            game_session(10).expect("predecessor"),
            game_session(21).expect("wrong candidate"),
        )
        .is_err()
    );
}

#[test]
fn terminal_replacement_authorization_rejects_candidate_account_mismatch() {
    let candidate = candidate_record(20, OTHER_ACCOUNT, 11, 12, 7, 9, 11, 1).expect("candidate");
    assert!(
        authorize(
            predecessor_snapshot(GameSessionState::Terminal, None, 11).expect("snapshot"),
            &candidate,
            game_session(10).expect("predecessor"),
            game_session(20).expect("candidate"),
        )
        .is_err()
    );
}

#[test]
fn terminal_replacement_authorization_rejects_candidate_character_mismatch() {
    let candidate = candidate_record(20, ACCOUNT, 99, 12, 7, 9, 11, 1).expect("candidate");
    assert!(
        authorize(
            predecessor_snapshot(GameSessionState::Terminal, None, 11).expect("snapshot"),
            &candidate,
            game_session(10).expect("predecessor"),
            game_session(20).expect("candidate"),
        )
        .is_err()
    );
}

#[test]
fn terminal_replacement_authorization_rejects_candidate_world_mismatch() {
    let candidate = candidate_record(20, ACCOUNT, 11, 99, 7, 9, 11, 1).expect("candidate");
    assert!(
        authorize(
            predecessor_snapshot(GameSessionState::Terminal, None, 11).expect("snapshot"),
            &candidate,
            game_session(10).expect("predecessor"),
            game_session(20).expect("candidate"),
        )
        .is_err()
    );
}

#[test]
fn generic_v1_existing_terminal_requires_typed_same_attempt_reconciliation() {
    let record = candidate_record(20, ACCOUNT, 11, 12, 7, 9, 10, 1).expect("record");
    let (mut flow, request) = ReconnectDurabilityFlowV1::begin(record);
    assert_eq!(
        flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
            &request,
            ReconnectPrepareDispositionV1::ExistingTerminal,
        ))
        .expect("completion"),
        ReconnectPrepareActionV1::ReconcileSameAttempt
    );
    assert_eq!(
        flow.phase(),
        ReconnectDurabilityPhaseV1::ReconciliationRequired
    );
}

#[test]
fn v2_direct_existing_terminal_collision_marks_budget_and_respects_capacity() {
    let record = candidate_record(20, ACCOUNT, 11, 12, 7, 9, 10, 1).expect("record");
    let attempt = record.identity().reconnect_attempt_ref();
    let transport = record.connection().transport_ref();
    let mut budget = ReconnectAttemptBudgetV1::new(record.continuity().control_loss_epoch());
    assert_eq!(
        budget.reserve(attempt, transport).expect("reserve"),
        ReconnectAttemptReservationV1::New
    );
    let (mut flow, request) = ReconnectDurabilityFlowV2::begin(record, None);
    flow.accept_prepare_completion(
        ReconnectPrepareCompletionV2::for_request(
            &request,
            ReconnectPrepareDispositionV2::ExistingTerminal {
                disposition: ReconnectDurableTerminalDispositionV1::TransportRefCollision,
            },
        ),
        &mut budget,
    )
    .expect("typed replay");
    assert!(budget.replacement_allowed_after_collision(attempt));
}

#[test]
fn v2_direct_existing_terminal_noncollision_never_unlocks_fresh_attempt() {
    for disposition in [
        ReconnectDurableTerminalDispositionV1::ConcurrentPrepared,
        ReconnectDurableTerminalDispositionV1::StaleAuthority,
    ] {
        let record = candidate_record(20, ACCOUNT, 11, 12, 7, 9, 10, 1).expect("record");
        let attempt = record.identity().reconnect_attempt_ref();
        let transport = record.connection().transport_ref();
        let mut budget = ReconnectAttemptBudgetV1::new(record.continuity().control_loss_epoch());
        budget.reserve(attempt, transport).expect("reserve");
        let (mut flow, request) = ReconnectDurabilityFlowV2::begin(record, None);
        flow.accept_prepare_completion(
            ReconnectPrepareCompletionV2::for_request(
                &request,
                ReconnectPrepareDispositionV2::ExistingTerminal { disposition },
            ),
            &mut budget,
        )
        .expect("typed replay");
        assert!(!budget.replacement_allowed_after_collision(attempt));
    }
}

#[test]
fn v2_reconciliation_preserves_all_terminal_dispositions_and_collision_only_remint() {
    for (disposition, allows_replacement) in [
        (
            ReconnectDurableTerminalDispositionV1::TransportRefCollision,
            true,
        ),
        (
            ReconnectDurableTerminalDispositionV1::ConcurrentPrepared,
            false,
        ),
        (ReconnectDurableTerminalDispositionV1::StaleAuthority, false),
    ] {
        let record = candidate_record(20, ACCOUNT, 11, 12, 7, 9, 10, 1).expect("record");
        let attempt = record.identity().reconnect_attempt_ref();
        let transport = record.connection().transport_ref();
        let mut budget = ReconnectAttemptBudgetV1::new(record.continuity().control_loss_epoch());
        budget.reserve(attempt, transport).expect("reserve");
        let (mut flow, request) = ReconnectDurabilityFlowV2::begin(record.clone(), None);
        flow.accept_prepare_completion(
            ReconnectPrepareCompletionV2::for_request(
                &request,
                ReconnectPrepareDispositionV2::Ambiguous,
            ),
            &mut budget,
        )
        .expect("ambiguous");
        let snapshot = ReconnectDurableReconciliationSnapshotV2::new(
            record.clone(),
            ReconnectDurableOutcomeV2::Terminal { disposition },
        );
        let decision = flow
            .accept_reconciliation(
                snapshot,
                exact_current_authority(&record, 105).expect("current authority"),
                &mut budget,
            )
            .expect("typed reconciliation");
        assert_eq!(decision.terminal_disposition(), Some(disposition));
        assert_eq!(
            budget.replacement_allowed_after_collision(attempt),
            allows_replacement
        );
    }
}
