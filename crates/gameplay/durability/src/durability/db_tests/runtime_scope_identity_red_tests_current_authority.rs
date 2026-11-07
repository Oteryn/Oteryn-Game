#[test]
fn terminal_replacement_requires_independent_current_character_world_eligibility()
-> Result<(), ReconnectDurabilityErrorV1> {
    let candidate = candidate_record()?;
    assert!(
        TerminalGameSessionReplacementAuthorizationV1::from_current_authority(
            ACCOUNT,
            Some(&AccountPresenceClaimV1::new(ACCOUNT, character(11)?)?),
            game_session(10)?,
            game_session(20)?,
            predecessor_snapshot_with_world_eligibility(
                13,
                Some(CharacterWorldEligibilityClaimV1::new(
                    character(11)?,
                    world(12)?,
                )),
            )?,
            &candidate,
        )
        .is_ok()
    );

    for current_eligibility in [
        None,
        Some(CharacterWorldEligibilityClaimV1::new(
            character(11)?,
            world(99)?,
        )),
        Some(CharacterWorldEligibilityClaimV1::new(
            character(99)?,
            world(12)?,
        )),
    ] {
        assert_eq!(
            TerminalGameSessionReplacementAuthorizationV1::from_current_authority(
                ACCOUNT,
                Some(&AccountPresenceClaimV1::new(ACCOUNT, character(11)?)?),
                game_session(10)?,
                game_session(20)?,
                predecessor_snapshot_with_world_eligibility(13, current_eligibility)?,
                &candidate,
            ),
            Err(ReconnectDurabilityErrorV1::StaleAuthority)
        );
    }
    Ok(())
}

#[test]
fn terminal_replacement_authorization_requires_current_account_presence_at_prepare()
-> Result<(), ReconnectDurabilityErrorV1> {
    let candidate = candidate_record()?;
    let exact_presence = AccountPresenceClaimV1::new(ACCOUNT, character(11)?)?;
    let reassigned_presence = AccountPresenceClaimV1::new(ACCOUNT, character(99)?)?;

    assert!(
        TerminalGameSessionReplacementAuthorizationV1::from_current_authority(
            ACCOUNT,
            Some(&exact_presence),
            game_session(10)?,
            game_session(20)?,
            predecessor_snapshot(13)?,
            &candidate,
        )
        .is_ok()
    );

    for current_presence in [None, Some(&reassigned_presence)] {
        assert_eq!(
            TerminalGameSessionReplacementAuthorizationV1::from_current_authority(
                ACCOUNT,
                current_presence,
                game_session(10)?,
                game_session(20)?,
                predecessor_snapshot(13)?,
                &candidate,
            ),
            Err(ReconnectDurabilityErrorV1::StaleAuthority)
        );
    }
    Ok(())
}

#[test]
fn v2_final_revalidation_requires_current_world_and_live_candidate_binding()
-> Result<(), ReconnectDurabilityErrorV1> {
    let record = candidate_record()?;
    let exact_world = Some(CharacterWorldEligibilityClaimV1::new(
        record.identity().character_id(),
        record.identity().world_id(),
    ));
    let exact_candidate = ReconnectCandidateBindingV1::new(
        record.identity().game_session_id(),
        record.identity().reconnect_attempt_ref(),
        record.connection().candidate(),
        record.connection().transport_ref(),
        record.continuity().prepared_deadline(),
    )?;
    let exact_scope = record.identity().runtime_scope();
    let exact_connection = record.connection().predecessor();
    let exact_epoch = record.continuity().control_loss_epoch();
    let exact_proof = record.proof().clone();

    let mut exact_flow = prepared_flow(&record)?;
    assert!(
        exact_flow
            .authorize_commit(
                current_authority_with_bindings(
                    &record,
                    exact_world,
                    Some(exact_candidate),
                    exact_scope,
                    exact_connection,
                    exact_epoch,
                    exact_proof.clone(),
                    105,
                )?,
                104,
            )
            .is_ok()
    );

    let mut missing_world_flow = prepared_flow(&record)?;
    assert_eq!(
        missing_world_flow.authorize_commit(
            current_authority_with_bindings(
                &record,
                None,
                Some(exact_candidate),
                exact_scope,
                exact_connection,
                exact_epoch,
                exact_proof.clone(),
                105,
            )?,
            104,
        ),
        Err(ReconnectDurabilityErrorV1::StaleAuthority)
    );

    let mut reassigned_world_flow = prepared_flow(&record)?;
    assert_eq!(
        reassigned_world_flow.authorize_commit(
            current_authority_with_bindings(
                &record,
                Some(CharacterWorldEligibilityClaimV1::new(
                    character(11)?,
                    world(99)?,
                )),
                Some(exact_candidate),
                exact_scope,
                exact_connection,
                exact_epoch,
                exact_proof.clone(),
                105,
            )?,
            104,
        ),
        Err(ReconnectDurabilityErrorV1::StaleAuthority)
    );

    let mut missing_candidate_flow = prepared_flow(&record)?;
    assert_eq!(
        missing_candidate_flow.authorize_commit(
            current_authority_with_bindings(
                &record,
                exact_world,
                None,
                exact_scope,
                exact_connection,
                exact_epoch,
                exact_proof.clone(),
                105,
            )?,
            104,
        ),
        Err(ReconnectDurabilityErrorV1::StaleAuthority)
    );

    let rebound_candidate = ReconnectCandidateBindingV1::new(
        record.identity().game_session_id(),
        record.identity().reconnect_attempt_ref(),
        record.connection().candidate(),
        AuthenticatedTransportRefV1::decode(&[0x72; 16])
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        record.continuity().prepared_deadline(),
    )?;
    let mut rebound_candidate_flow = prepared_flow(&record)?;
    assert_eq!(
        rebound_candidate_flow.authorize_commit(
            current_authority_with_bindings(
                &record,
                exact_world,
                Some(rebound_candidate),
                exact_scope,
                exact_connection,
                exact_epoch,
                exact_proof.clone(),
                105,
            )?,
            104,
        ),
        Err(ReconnectDurabilityErrorV1::StaleAuthority)
    );

    let changed_attempt_candidate = ReconnectCandidateBindingV1::new(
        record.identity().game_session_id(),
        ReconnectAttemptRef::new(2).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        record.connection().candidate(),
        record.connection().transport_ref(),
        record.continuity().prepared_deadline(),
    )?;
    let mut changed_attempt_flow = prepared_flow(&record)?;
    assert_eq!(
        changed_attempt_flow.authorize_commit(
            current_authority_with_bindings(
                &record,
                exact_world,
                Some(changed_attempt_candidate),
                exact_scope,
                exact_connection,
                exact_epoch,
                exact_proof,
                105,
            )?,
            104,
        ),
        Err(ReconnectDurabilityErrorV1::StaleAuthority)
    );
    Ok(())
}

#[test]
fn v2_committed_reconciliation_requires_current_world_and_live_candidate_binding()
-> Result<(), ReconnectDurabilityErrorV1> {
    let record = candidate_record()?;
    let snapshot = ReconnectDurableReconciliationSnapshotV2::new(
        record.clone(),
        ReconnectDurableOutcomeV2::Committed {
            current_generation: record.connection().candidate(),
            current_transport_ref: record.connection().transport_ref(),
        },
    );
    let reconcile = |current_character_world_eligibility: Option<
        CharacterWorldEligibilityClaimV1,
    >,
                     current_candidate: Option<ReconnectCandidateBindingV1>| {
        let mut budget = ReconnectAttemptBudgetV1::new(record.continuity().control_loss_epoch());
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
        flow.accept_reconciliation(
            snapshot.clone(),
            current_authority_with_bindings(
                &record,
                current_character_world_eligibility,
                current_candidate,
                record.identity().runtime_scope(),
                record.connection().predecessor(),
                record.continuity().control_loss_epoch(),
                record.proof().clone(),
                105,
            )?,
            &mut budget,
        )
    };

    let exact_world = Some(CharacterWorldEligibilityClaimV1::new(
        record.identity().character_id(),
        record.identity().world_id(),
    ));
    let exact_candidate = Some(ReconnectCandidateBindingV1::new(
        record.identity().game_session_id(),
        record.identity().reconnect_attempt_ref(),
        record.connection().candidate(),
        record.connection().transport_ref(),
        record.continuity().prepared_deadline(),
    )?);
    assert!(matches!(
        reconcile(exact_world, exact_candidate)?,
        ReconnectProjectionDecisionV2::InstallController { .. }
    ));
    assert_eq!(
        reconcile(None, exact_candidate),
        Err(ReconnectDurabilityErrorV1::ReconciliationMismatch)
    );
    assert_eq!(
        reconcile(exact_world, None),
        Err(ReconnectDurabilityErrorV1::ReconciliationMismatch)
    );
    Ok(())
}

#[test]
fn v2_terminal_reconciliation_survives_scope_advance_but_committed_stays_fenced()
-> Result<(), ReconnectDurabilityErrorV1> {
    let record = candidate_record()?;
    let advanced_authority = ReconnectCurrentAuthorityV1::from_current_facts(
        &record,
        Some(AccountPresenceClaimV1::new(
            record.identity().account_id(),
            record.identity().character_id(),
        )?),
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
        ReconnectAuthorityFenceV1::new(
            record.authority().character_lease_generation(),
            ScopeOwnershipGeneration::new(11)
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        )?,
        record.continuity().control_loss_epoch(),
        record.continuity().original_grace_deadline(),
        record.proof().clone(),
        record.fnd02().clone(),
        record.compatibility().clone(),
        GameSessionState::Reconnectable,
        false,
        105,
    )?;

    let reconciliation_flow = || {
        let mut budget = ReconnectAttemptBudgetV1::new(record.continuity().control_loss_epoch());
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
        Ok::<_, ReconnectDurabilityErrorV1>((flow, budget))
    };

    let (mut terminal_flow, mut terminal_budget) = reconciliation_flow()?;
    let terminal_snapshot = ReconnectDurableReconciliationSnapshotV2::new(
        record.clone(),
        ReconnectDurableOutcomeV2::Terminal {
            disposition: ReconnectDurableTerminalDispositionV1::TransportRefCollision,
        },
    );
    assert_eq!(
        terminal_flow.accept_reconciliation(
            terminal_snapshot,
            advanced_authority.clone(),
            &mut terminal_budget,
        )?,
        ReconnectProjectionDecisionV2::Terminal {
            disposition: ReconnectDurableTerminalDispositionV1::TransportRefCollision,
        }
    );
    assert!(
        terminal_budget
            .replacement_allowed_after_collision(record.identity().reconnect_attempt_ref())
    );

    let (mut committed_flow, mut committed_budget) = reconciliation_flow()?;
    let committed_snapshot = ReconnectDurableReconciliationSnapshotV2::new(
        record.clone(),
        ReconnectDurableOutcomeV2::Committed {
            current_generation: record.connection().candidate(),
            current_transport_ref: record.connection().transport_ref(),
        },
    );
    assert_eq!(
        committed_flow.accept_reconciliation(
            committed_snapshot,
            advanced_authority,
            &mut committed_budget,
        ),
        Err(ReconnectDurabilityErrorV1::ReconciliationMismatch)
    );
    Ok(())
}
