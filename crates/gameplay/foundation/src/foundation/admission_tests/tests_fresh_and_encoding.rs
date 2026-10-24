#[test]
fn reconnect_attempt_ref_has_stable_durable_encoding() -> Result<(), AdmissionError> {
    let attempt = ReconnectAttemptRef::new(0x0102_0304_0506_0708)?;
    let encoded = attempt.to_be_bytes();
    assert_eq!(encoded, [1, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(ReconnectAttemptRef::decode(&encoded)?, attempt);
    assert_eq!(
        ReconnectAttemptRef::decode(&[0u8; 8]),
        Err(AdmissionError::InvalidFacts)
    );
    let short = [1u8; 7];
    assert_eq!(
        ReconnectAttemptRef::decode(&short),
        Err(AdmissionError::InvalidFacts)
    );
    Ok(())
}

#[test]
fn fresh_admission_replay_key_has_stable_durable_encoding() -> Result<(), AdmissionError> {
    let facts = facts(0x0102_0304_0506_0708)?;
    let key = facts.replay_key();
    let encoded = key.to_bytes();
    assert_eq!(encoded.len(), 33);
    assert_eq!(encoded[0], FRESH_ADMISSION_REPLAY_KEY_TAG);
    assert_eq!(FreshAdmissionReplayKey::decode(&encoded)?, key);
    assert_eq!(key.trusted_issuer(), PRE_ADMISSION_TRUSTED_ISSUER);
    assert_eq!(key.profile(), PRE_ADMISSION_PROFILE);

    let mut wrong_tag = encoded;
    wrong_tag[0] = 0xff;
    assert_eq!(
        FreshAdmissionReplayKey::decode(&wrong_tag),
        Err(AdmissionError::InvalidFacts)
    );
    let mut zero_nonce = [0u8; 33];
    zero_nonce[0] = FRESH_ADMISSION_REPLAY_KEY_TAG;
    assert_eq!(
        FreshAdmissionReplayKey::decode(&zero_nonce),
        Err(AdmissionError::InvalidFacts)
    );
    assert_eq!(
        FreshAdmissionReplayKey::decode(&encoded[..32]),
        Err(AdmissionError::InvalidFacts)
    );
    Ok(())
}

#[test]
fn reconnect_journal_serializes_distinct_prepares_per_session() -> Result<(), AdmissionError> {
    let journal = TestReconnectAttemptJournal::default();
    let mut authority = AdmissionAuthority::new(journal.clone());
    admit(&mut authority, 9000, 9000, 100u64)?;
    lose_current(&mut authority)?;
    let session = authority
        .current()
        .ok_or(AdmissionError::Terminal)?
        .game_session_id();
    let generation_two = ConnectionGeneration::new(2).map_err(|_| AdmissionError::InvalidFacts)?;
    let attempt_b = ReconnectAttemptRef::new(5)?;
    let attempt_c = ReconnectAttemptRef::new(u64::MAX)?;

    assert_eq!(
        journal.claim_prepared(
            session,
            attempt_b,
            reconnect_binding_for_test(1, 2, 200u64)?
        )?,
        ReconnectAttemptClaim::Claimed
    );
    assert_eq!(
        journal.claim_prepared(
            session,
            attempt_c,
            reconnect_binding_for_test(1, 2, 300u64)?
        )?,
        ReconnectAttemptClaim::RejectedConcurrent
    );
    assert_eq!(
        journal.lookup(session, attempt_c)?,
        Some(ReconnectAttemptDisposition::TerminallySuperseded)
    );
    assert_eq!(
        journal.lookup(session, attempt_b)?,
        Some(ReconnectAttemptDisposition::Prepared {
            candidate_generation: generation_two,
        })
    );
    Ok(())
}

#[test]
fn fresh_identity_replay_history_is_not_retained_in_session_authority() {
    let authority = test_authority();
    assert_eq!(authority.retained_fresh_identity_history(), 0);
}

#[test]
fn fresh_admission_commits_generation_one_and_nonce_replay_fails() -> Result<(), AdmissionError> {
    let journal = TestReconnectAttemptJournal::default();
    let mut authority = AdmissionAuthority::new(journal.clone());
    let session = admit(&mut authority, 1, 100, 100u64)?;
    assert_eq!(session.connection_generation().get(), 1);
    assert_eq!(session.character_lease().generation(), 7);
    authority.terminate_current()?;
    assert_eq!(
        authority.commit_fresh(facts(1)?, 100u64, || game_session_id(101)),
        Err(AdmissionError::GrantReplayed)
    );
    Ok(())
}

#[test]
fn fresh_admission_reconciliation_cannot_bind_two_transports_to_one_session()
-> Result<(), AdmissionError> {
    let journal = TestReconnectAttemptJournal::default();
    let mut first_authority = AdmissionAuthority::new(journal.clone());
    let mut second_authority = AdmissionAuthority::new(journal.clone());
    admit(&mut first_authority, 95, 9500, 100u64)?;
    assert_eq!(first_authority.current_transport(), Some(100));

    assert_eq!(
        second_authority.commit_fresh(facts(95)?, 200u64, || game_session_id(9501)),
        Err(AdmissionError::GrantReplayed)
    );
    assert!(second_authority.current().is_none());
    assert_eq!(second_authority.current_transport(), None);
    Ok(())
}

#[test]
fn fresh_admission_process_recovery_cannot_revive_terminal_session() -> Result<(), AdmissionError> {
    let journal = TestReconnectAttemptJournal::default();
    let mut authority = AdmissionAuthority::new(journal.clone());
    admit(&mut authority, 91, 901, 100u64)?;
    authority.terminate_current()?;
    drop(authority);

    let mut recovered_authority = AdmissionAuthority::new(journal.clone());
    assert_eq!(
        recovered_authority.commit_fresh(facts(91)?, 101u64, || game_session_id(902)),
        Err(AdmissionError::GrantReplayed)
    );
    assert!(recovered_authority.current().is_none());
    Ok(())
}

#[test]
fn fresh_admission_process_recovery_cannot_rollback_reconnected_active_generation()
-> Result<(), AdmissionError> {
    let journal = TestReconnectAttemptJournal::default();
    let mut authority = AdmissionAuthority::new(journal.clone());
    admit(&mut authority, 94, 9400, 100u64)?;
    lose_current(&mut authority)?;
    let attempt = ReconnectAttemptRef::new(94)?;
    let first = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
    let generation_two = authority.prepare_reconnect(attempt, first, 200u64, 7, 11)?;
    authority.commit_reconnect(attempt, 200u64, 7, 11)?;
    assert_eq!(generation_two.get(), 2);
    drop(authority);

    let mut recovered_authority = AdmissionAuthority::new(journal.clone());
    assert_eq!(
        recovered_authority.commit_fresh(facts(94)?, 300u64, || game_session_id(9401)),
        Err(AdmissionError::GrantReplayed)
    );
    assert!(recovered_authority.current().is_none());
    Ok(())
}

#[test]
fn fresh_admission_lost_commit_response_can_reconstruct_the_same_session()
-> Result<(), AdmissionError> {
    let journal = TestReconnectAttemptJournal::default();
    let mut authority = AdmissionAuthority::new(journal.clone());
    let admission = facts(90)?;
    let session_id = game_session_id(900)?;
    journal.fail_next_fresh_response_after_commit();

    let lost_response = authority.commit_fresh(admission, 100u64, || Ok(session_id));
    assert_eq!(
        lost_response.map(|_| ()),
        Err(AdmissionError::ReconciliationUnavailable)
    );
    assert!(authority.current().is_none());

    let recovered = authority.commit_fresh(admission, 100u64, || game_session_id(901))?;
    assert_eq!(recovered.game_session_id(), session_id);
    assert_eq!(recovered.connection_generation().get(), 1);
    assert_eq!(authority.current_transport(), Some(100));
    Ok(())
}
