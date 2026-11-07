#[test]
fn terminal_replacement_rejects_same_world_generation_different_current_runtime_scope()
-> Result<(), ReconnectDurabilityErrorV1> {
    let candidate = candidate_record()?;
    let result = TerminalGameSessionReplacementAuthorizationV1::from_current_authority(
        ACCOUNT,
        Some(&AccountPresenceClaimV1::new(ACCOUNT, character(11)?)?),
        game_session(10)?,
        game_session(20)?,
        predecessor_snapshot(14)?,
        &candidate,
    );
    assert_eq!(result, Err(ReconnectDurabilityErrorV1::StaleAuthority));
    Ok(())
}

#[test]
fn final_revalidation_requires_complete_current_authority_facts()
-> Result<(), ReconnectDurabilityErrorV1> {
    let record = candidate_record()?;
    let exact_scope = record.identity().runtime_scope();
    let exact_connection = record.connection().predecessor();
    let exact_epoch = record.continuity().control_loss_epoch();
    let exact_proof = record.proof().clone();

    let mut exact_flow = prepared_flow(&record)?;
    let exact = current_authority(
        &record,
        exact_scope,
        exact_connection,
        exact_epoch,
        exact_proof.clone(),
    )?;
    assert!(exact_flow.authorize_commit(exact, 104).is_ok());

    let mut scope_flow = prepared_flow(&record)?;
    let changed_scope = current_authority(
        &record,
        RuntimeScopeRefV1::channel(record.identity().world_id(), channel(14)?),
        exact_connection,
        exact_epoch,
        exact_proof.clone(),
    )?;
    assert_eq!(
        scope_flow.authorize_commit(changed_scope, 104),
        Err(ReconnectDurabilityErrorV1::StaleAuthority)
    );

    let mut connection_flow = prepared_flow(&record)?;
    let changed_connection = current_authority(
        &record,
        exact_scope,
        ConnectionGeneration::new(9).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        exact_epoch,
        exact_proof.clone(),
    )?;
    assert_eq!(
        connection_flow.authorize_commit(changed_connection, 104),
        Err(ReconnectDurabilityErrorV1::StaleAuthority)
    );

    let mut epoch_flow = prepared_flow(&record)?;
    let changed_epoch = current_authority(
        &record,
        exact_scope,
        exact_connection,
        ControlLossEpochRefV1::new(4)?,
        exact_proof,
    )?;
    assert_eq!(
        epoch_flow.authorize_commit(changed_epoch, 104),
        Err(ReconnectDurabilityErrorV1::StaleAuthority)
    );

    let mut proof_flow = prepared_flow(&record)?;
    let changed_proof = current_authority(
        &record,
        exact_scope,
        exact_connection,
        exact_epoch,
        ReconnectProofV1::ReauthenticatedRecovery {
            recovery_grant_nonce: [0x56; 32],
        },
    )?;
    assert_eq!(
        proof_flow.authorize_commit(changed_proof, 104),
        Err(ReconnectDurabilityErrorV1::StaleAuthority)
    );
    Ok(())
}

#[test]
fn final_revalidation_requires_current_original_grace_deadline()
-> Result<(), ReconnectDurabilityErrorV1> {
    let record = candidate_record()?;
    let current = |original_grace_deadline: i64| {
        ReconnectCurrentAuthorityV1::from_current_facts(
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
            record.authority(),
            record.continuity().control_loss_epoch(),
            original_grace_deadline,
            record.proof().clone(),
            record.fnd02().clone(),
            record.compatibility().clone(),
            GameSessionState::Reconnectable,
            false,
            105,
        )
    };

    let exact_deadline = record.continuity().original_grace_deadline();
    let mut exact_flow = prepared_flow(&record)?;
    assert!(
        exact_flow
            .authorize_commit(current(exact_deadline)?, 104)
            .is_ok()
    );

    let mut shortened_flow = prepared_flow(&record)?;
    assert_eq!(
        shortened_flow.authorize_commit(current(exact_deadline - 1)?, 104),
        Err(ReconnectDurabilityErrorV1::StaleAuthority)
    );

    let snapshot = ReconnectDurableReconciliationSnapshotV2::new(
        record.clone(),
        ReconnectDurableOutcomeV2::Committed {
            current_generation: record.connection().candidate(),
            current_transport_ref: record.connection().transport_ref(),
        },
    );
    let mut budget = ReconnectAttemptBudgetV1::new(record.continuity().control_loss_epoch());
    budget.reserve(
        record.identity().reconnect_attempt_ref(),
        record.connection().transport_ref(),
    )?;
    let (mut reconciliation_flow, request) = ReconnectDurabilityFlowV2::begin(record.clone(), None);
    reconciliation_flow.accept_prepare_completion(
        ReconnectPrepareCompletionV2::for_request(
            &request,
            ReconnectPrepareDispositionV2::Ambiguous,
        ),
        &mut budget,
    )?;
    assert!(matches!(
        reconciliation_flow.accept_reconciliation(
            snapshot,
            current(exact_deadline)?,
            &mut budget,
        )?,
        ReconnectProjectionDecisionV2::InstallController { .. }
    ));
    Ok(())
}

#[test]
fn replacement_authorization_rejects_runtime_scope_record_substitution()
-> Result<(), ReconnectDurabilityErrorV1> {
    let candidate = candidate_record()?;
    let authorization = TerminalGameSessionReplacementAuthorizationV1::from_current_authority(
        ACCOUNT,
        Some(&AccountPresenceClaimV1::new(ACCOUNT, character(11)?)?),
        game_session(10)?,
        game_session(20)?,
        predecessor_snapshot(13)?,
        &candidate,
    )?;
    let substituted = candidate_record_for_channel(14)?;

    assert!(
        !crate::durability::replacement_authorization_matches_record(&authorization, &substituted,)
    );
    Ok(())
}

#[test]
fn replacement_reconciliation_requires_receipt_authorization_when_request_omits_it() -> TestResult {
    run_postgres_test(async {
        let (database, database_url) = migrated_database("unsigned_replacement_reconcile").await?;
        let now = unix_now()?;
        seed_current_actor_anchor(&database_url, 10, now).await?;
        let journal = AdmissionReconnectJournalV2::connect_runtime(&database_url).await?;
        let candidate = postgres_record(now, 20, 1, 0xa1, 7, ProtectionEntitlementV1::unused())
            .map_err(|_| "candidate record")?;
        let authorization = replacement_authorization(&candidate, 10, 7)
            .map_err(|_| "replacement authorization")?;
        let signed_request =
            ReconnectDurabilityFlowV2::begin(candidate.clone(), Some(authorization)).1;
        assert_eq!(
            journal.prepare(&signed_request).await?,
            ReconnectPrepareDispositionV2::Prepared
        );

        let unsigned_request = ReconnectDurabilityFlowV2::begin(candidate, None).1;
        assert!(matches!(
            journal.reconcile(&unsigned_request).await,
            Err(crate::durability::DurabilityError::InvalidStoredState)
        ));

        drop(journal);
        database.cleanup().await?;
        Ok(())
    })
}

#[test]
fn replacement_prepare_requires_receipt_authorization_when_request_omits_it() -> TestResult {
    run_postgres_test(async {
        let (database, database_url) = migrated_database("unsigned_replacement_prepare").await?;
        let now = unix_now()?;
        seed_current_actor_anchor(&database_url, 10, now).await?;
        let journal = AdmissionReconnectJournalV2::connect_runtime(&database_url).await?;
        let candidate = postgres_record(now, 20, 1, 0xa2, 7, ProtectionEntitlementV1::unused())
            .map_err(|_| "candidate record")?;
        let authorization = replacement_authorization(&candidate, 10, 7)
            .map_err(|_| "replacement authorization")?;
        let signed_request =
            ReconnectDurabilityFlowV2::begin(candidate.clone(), Some(authorization)).1;
        assert_eq!(
            journal.prepare(&signed_request).await?,
            ReconnectPrepareDispositionV2::Prepared
        );

        let unsigned_request = ReconnectDurabilityFlowV2::begin(candidate, None).1;
        assert!(matches!(
            journal.prepare(&unsigned_request).await,
            Err(crate::durability::DurabilityError::InvalidStoredState)
        ));

        drop(journal);
        database.cleanup().await?;
        Ok(())
    })
}

#[test]
fn legacy_prepare_rejects_receipt_backed_replacement_candidate() -> TestResult {
    run_postgres_test(async {
        let (database, database_url) = migrated_database("legacy_replacement_prepare").await?;
        let now = unix_now()?;
        seed_current_actor_anchor(&database_url, 10, now).await?;
        let journal = AdmissionReconnectJournalV2::connect_runtime(&database_url).await?;
        let candidate = postgres_record(now, 20, 1, 0xa3, 7, ProtectionEntitlementV1::unused())
            .map_err(|_| "candidate record")?;
        let authorization = replacement_authorization(&candidate, 10, 7)
            .map_err(|_| "replacement authorization")?;
        let signed_request =
            ReconnectDurabilityFlowV2::begin(candidate.clone(), Some(authorization)).1;
        assert_eq!(
            journal.prepare(&signed_request).await?,
            ReconnectPrepareDispositionV2::Prepared
        );

        let (_, legacy_request) = ReconnectDurabilityFlowV1::begin(candidate);
        assert!(matches!(
            journal.legacy().prepare(&legacy_request).await,
            Err(crate::durability::DurabilityError::InvalidStoredState)
        ));

        drop(journal);
        database.cleanup().await?;
        Ok(())
    })
}

#[test]
fn terminal_replacement_preserves_committed_predecessor_reconciliation() -> TestResult {
    run_postgres_test(async {
        let (database, database_url) = migrated_database("committed_predecessor_reconcile").await?;
        let now = unix_now()?;
        let journal = AdmissionReconnectJournalV2::connect_runtime(&database_url).await?;
        let fenced = ProtectionEntitlementV1::fenced(42).map_err(|_| "fenced protection")?;
        let predecessor_record =
            postgres_record(now, 10, 1, 0xb1, 7, fenced).map_err(|_| "predecessor record")?;
        let (mut predecessor_flow, predecessor_prepare) =
            ReconnectDurabilityFlowV1::begin(predecessor_record.clone());
        assert_eq!(
            journal.legacy().prepare(&predecessor_prepare).await?,
            ReconnectPrepareDispositionV1::Prepared
        );
        predecessor_flow
            .accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                &predecessor_prepare,
                ReconnectPrepareDispositionV1::Prepared,
            ))
            .map_err(|_| "predecessor prepare completion")?;
        let current = exact_current_authority(&predecessor_record, now)
            .map_err(|_| "predecessor current authority")?;
        let predecessor_commit = predecessor_flow
            .authorize_commit(current, now)
            .map_err(|_| "predecessor commit authorization")?;
        assert_eq!(
            journal.legacy().commit(&predecessor_commit).await?,
            ReconnectCommitDispositionV1::Committed
        );

        let candidate = postgres_record(
            now,
            20,
            2,
            0xb2,
            8,
            ProtectionEntitlementV1::fenced(42).map_err(|_| "candidate protection")?,
        )
        .map_err(|_| "candidate record")?;
        let authorization = replacement_authorization(&candidate, 10, 8)
            .map_err(|_| "replacement authorization")?;
        let replacement_request =
            ReconnectDurabilityFlowV2::begin(candidate, Some(authorization)).1;
        assert_eq!(
            journal.prepare(&replacement_request).await?,
            ReconnectPrepareDispositionV2::Prepared
        );

        assert_eq!(
            journal.legacy().reconcile(&predecessor_prepare).await?,
            ReconnectDurableReconciliationSnapshotV1::terminal(predecessor_record)
        );

        drop(journal);
        database.cleanup().await?;
        Ok(())
    })
}
