#[test]
fn grant_nonce_preserves_all_thirty_two_bytes() -> Result<(), AdmissionError> {
    let facts = FreshAdmissionFacts::for_test(7, 7, 11)?;
    assert_eq!(facts.grant_nonce.len(), 32);
    Ok(())
}

#[test]
fn committed_reconnect_attempt_cannot_be_reprepared() -> Result<(), AdmissionError> {
    let mut authority = test_authority();
    admit(&mut authority, 8, 800, 100u64)?;
    lose_current(&mut authority)?;
    let attempt = ReconnectAttemptRef::new(21)?;
    let first = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
    let second = authority.prepare_reconnect(attempt, first, 200u64, 7, 11)?;
    assert_eq!(authority.commit_reconnect(attempt, 200u64, 7, 11)?, second);
    lose_current(&mut authority)?;
    assert_eq!(
        authority.prepare_reconnect(attempt, second, 200u64, 7, 11),
        Err(AdmissionError::StaleConnection)
    );
    assert_eq!(
        authority
            .current()
            .ok_or(AdmissionError::Terminal)?
            .connection_generation(),
        second
    );
    Ok(())
}

#[test]
fn delayed_predecessor_loss_cannot_drop_reconnected_controller() -> Result<(), AdmissionError> {
    let mut authority = test_authority();
    admit(&mut authority, 29, 2900, 100u64)?;
    lose_current(&mut authority)?;
    let generation_one = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
    let attempt = ReconnectAttemptRef::new(29)?;
    let generation_two = authority.prepare_reconnect(attempt, generation_one, 200u64, 7, 11)?;
    authority.commit_reconnect(attempt, 200u64, 7, 11)?;

    // Models a delayed close/liveness callback from predecessor transport 100/generation 1.
    assert_eq!(
        authority.mark_unexpected_control_loss(100u64, generation_one)?,
        ControlLossDisposition::StaleIgnored
    );
    assert_eq!(authority.current_transport(), Some(200));
    assert_eq!(
        authority.current().map(GameSession::state),
        Some(GameSessionState::Active)
    );
    assert_eq!(
        authority.current().map(GameSession::connection_generation),
        Some(generation_two)
    );
    assert_eq!(
        authority.mark_unexpected_control_loss(200u64, generation_two)?,
        ControlLossDisposition::Applied
    );
    assert_eq!(authority.current_transport(), None);
    assert_eq!(
        authority.current().map(GameSession::state),
        Some(GameSessionState::Reconnectable)
    );
    Ok(())
}

#[test]
fn lost_current_transport_cannot_replay_committed_success() -> Result<(), AdmissionError> {
    let mut authority = test_authority();
    admit(&mut authority, 14, 1400, 100u64)?;
    lose_current(&mut authority)?;
    let attempt = ReconnectAttemptRef::new(28)?;
    let generation_one = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
    authority.prepare_reconnect(attempt, generation_one, 200u64, 7, 11)?;
    authority.commit_reconnect(attempt, 200u64, 7, 11)?;
    lose_current(&mut authority)?;
    assert_eq!(authority.current_transport(), None);
    assert_eq!(
        authority.commit_reconnect(attempt, 200u64, 7, 11),
        Err(AdmissionError::StaleConnection)
    );
    Ok(())
}

#[test]
fn superseded_committed_attempt_cannot_replay_success() -> Result<(), AdmissionError> {
    let mut authority = test_authority();
    admit(&mut authority, 13, 1300, 100u64)?;
    lose_current(&mut authority)?;
    let attempt_a = ReconnectAttemptRef::new(26)?;
    let generation_one = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
    let generation_two = authority.prepare_reconnect(attempt_a, generation_one, 200u64, 7, 11)?;
    authority.commit_reconnect(attempt_a, 200u64, 7, 11)?;

    lose_current(&mut authority)?;
    let attempt_b = ReconnectAttemptRef::new(27)?;
    let generation_three = authority.prepare_reconnect(attempt_b, generation_two, 300u64, 7, 11)?;
    authority.commit_reconnect(attempt_b, 300u64, 7, 11)?;
    assert_eq!(generation_three.get(), 3);

    assert_eq!(
        authority.commit_reconnect(attempt_a, 200u64, 7, 11),
        Err(AdmissionError::StaleConnection)
    );
    assert_eq!(
        authority.prepare_reconnect(attempt_a, generation_three, 200u64, 7, 11),
        Err(AdmissionError::StaleConnection)
    );
    Ok(())
}

#[test]
fn terminal_session_rejects_committed_attempt_replay() -> Result<(), AdmissionError> {
    let mut authority = test_authority();
    admit(&mut authority, 10, 1000, 100u64)?;
    lose_current(&mut authority)?;
    let attempt = ReconnectAttemptRef::new(23)?;
    let first = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
    authority.prepare_reconnect(attempt, first, 200u64, 7, 11)?;
    authority.commit_reconnect(attempt, 200u64, 7, 11)?;
    authority.terminate_current()?;
    assert_eq!(authority.current_transport(), None);
    assert_eq!(
        authority.commit_reconnect(attempt, 200u64, 7, 11),
        Err(AdmissionError::Terminal)
    );
    Ok(())
}

#[test]
fn reconnect_revalidates_updatable_runtime_ownership_generation() -> Result<(), AdmissionError> {
    let mut authority = test_authority();
    admit(&mut authority, 11, 1100, 100u64)?;
    lose_current(&mut authority)?;
    authority.observe_runtime_ownership_generation(12)?;
    let attempt = ReconnectAttemptRef::new(24)?;
    let first = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
    assert_eq!(
        authority.prepare_reconnect(attempt, first, 200u64, 7, 11),
        Err(AdmissionError::StaleRuntime)
    );
    let candidate = authority.prepare_reconnect(attempt, first, 200u64, 7, 12)?;
    assert_eq!(
        authority.commit_reconnect(attempt, 200u64, 7, 12)?,
        candidate
    );
    Ok(())
}

#[test]
fn runtime_owner_change_supersedes_prepared_reconnect() -> Result<(), AdmissionError> {
    let mut authority = test_authority();
    admit(&mut authority, 12, 1200, 100u64)?;
    lose_current(&mut authority)?;
    let attempt = ReconnectAttemptRef::new(25)?;
    let first = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
    authority.prepare_reconnect(attempt, first, 200u64, 7, 11)?;
    authority.observe_runtime_ownership_generation(12)?;
    assert_eq!(
        authority.commit_reconnect(attempt, 200u64, 7, 12),
        Err(AdmissionError::StaleConnection)
    );
    Ok(())
}

#[test]
fn reconnect_attempt_history_is_external_and_old_refs_stay_terminal() -> Result<(), AdmissionError>
{
    let journal = TestReconnectAttemptJournal::default();
    let mut authority = AdmissionAuthority::new(journal.clone());
    admit(&mut authority, 50, 5000, 100u64)?;
    let mut predecessor = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
    let first_attempt = ReconnectAttemptRef::new(50)?;

    for (raw_attempt, transport) in [(50u64, 200u64), (51, 201), (52, 202)] {
        lose_current(&mut authority)?;
        let attempt = ReconnectAttemptRef::new(raw_attempt)?;
        let candidate = authority.prepare_reconnect(attempt, predecessor, transport, 7, 11)?;
        authority.commit_reconnect(attempt, transport, 7, 11)?;
        predecessor = candidate;
    }

    assert!(authority.prepared.is_none());
    assert_eq!(journal.len(), 3);
    lose_current(&mut authority)?;
    assert_eq!(
        authority.prepare_reconnect(first_attempt, predecessor, 300u64, 7, 11),
        Err(AdmissionError::StaleConnection)
    );
    Ok(())
}

#[test]
fn superseded_prepared_attempt_cannot_be_reused_after_runtime_owner_change()
-> Result<(), AdmissionError> {
    let mut authority = test_authority();
    admit(&mut authority, 51, 5100, 100u64)?;
    lose_current(&mut authority)?;
    let attempt = ReconnectAttemptRef::new(60)?;
    let predecessor = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
    authority.prepare_reconnect(attempt, predecessor, 200u64, 7, 11)?;
    authority.observe_runtime_ownership_generation(12)?;
    assert_eq!(
        authority.prepare_reconnect(attempt, predecessor, 200u64, 7, 12),
        Err(AdmissionError::StaleConnection)
    );
    Ok(())
}

#[test]
fn superseded_prepare_retry_keeps_terminal_outcome_while_newer_prepare_is_pending()
-> Result<(), AdmissionError> {
    let mut authority = test_authority();
    admit(&mut authority, 70, 7000, 100u64)?;
    let generation_one = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;

    lose_current(&mut authority)?;
    let attempt_a = ReconnectAttemptRef::new(70)?;
    let generation_two = authority.prepare_reconnect(attempt_a, generation_one, 200u64, 7, 11)?;
    authority.commit_reconnect(attempt_a, 200u64, 7, 11)?;

    lose_current(&mut authority)?;
    let attempt_b = ReconnectAttemptRef::new(71)?;
    let generation_three = authority.prepare_reconnect(attempt_b, generation_two, 300u64, 7, 11)?;

    assert_eq!(
        authority.prepare_reconnect(attempt_a, generation_two, 200u64, 7, 11),
        Err(AdmissionError::StaleConnection)
    );
    assert_eq!(
        authority.commit_reconnect(attempt_b, 300u64, 7, 11)?,
        generation_three
    );
    Ok(())
}

#[test]
fn superseded_commit_retry_keeps_terminal_outcome_while_newer_prepare_is_pending()
-> Result<(), AdmissionError> {
    let mut authority = test_authority();
    admit(&mut authority, 71, 7100, 100u64)?;
    let generation_one = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;

    lose_current(&mut authority)?;
    let attempt_a = ReconnectAttemptRef::new(80)?;
    let generation_two = authority.prepare_reconnect(attempt_a, generation_one, 200u64, 7, 11)?;
    authority.commit_reconnect(attempt_a, 200u64, 7, 11)?;

    lose_current(&mut authority)?;
    let attempt_b = ReconnectAttemptRef::new(81)?;
    let generation_three = authority.prepare_reconnect(attempt_b, generation_two, 300u64, 7, 11)?;

    assert_eq!(
        authority.commit_reconnect(attempt_a, 200u64, 7, 11),
        Err(AdmissionError::StaleConnection)
    );
    assert_eq!(
        authority.commit_reconnect(attempt_b, 300u64, 7, 11)?,
        generation_three
    );
    Ok(())
}

#[test]
fn opaque_reconnect_ref_value_cannot_poison_future_attempts() -> Result<(), AdmissionError> {
    let mut authority = test_authority();
    admit(&mut authority, 73, 7300, 100u64)?;
    lose_current(&mut authority)?;
    let generation_one = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
    let winning_attempt = ReconnectAttemptRef::new(40)?;
    let poisoned_value = ReconnectAttemptRef::new(u64::MAX)?;
    let generation_two =
        authority.prepare_reconnect(winning_attempt, generation_one, 200u64, 7, 11)?;

    assert_eq!(
        authority.prepare_reconnect(poisoned_value, generation_one, 300u64, 7, 11),
        Err(AdmissionError::StaleConnection)
    );
    assert_eq!(
        authority.commit_reconnect(winning_attempt, 200u64, 7, 11)?,
        generation_two
    );

    lose_current(&mut authority)?;
    let later_opaque_attempt = ReconnectAttemptRef::new(7)?;
    let generation_three =
        authority.prepare_reconnect(later_opaque_attempt, generation_two, 400u64, 7, 11)?;
    assert_eq!(
        authority.commit_reconnect(later_opaque_attempt, 400u64, 7, 11)?,
        generation_three
    );
    Ok(())
}

#[test]
fn rejected_concurrent_reconnect_ref_is_terminally_retired() -> Result<(), AdmissionError> {
    let mut authority = test_authority();
    admit(&mut authority, 72, 7200, 100u64)?;
    lose_current(&mut authority)?;
    let generation_one = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
    let attempt_b = ReconnectAttemptRef::new(90)?;
    let attempt_c = ReconnectAttemptRef::new(91)?;
    let generation_two = authority.prepare_reconnect(attempt_b, generation_one, 200u64, 7, 11)?;

    assert_eq!(
        authority.prepare_reconnect(attempt_c, generation_one, 300u64, 7, 11),
        Err(AdmissionError::StaleConnection)
    );
    assert_eq!(
        authority.commit_reconnect(attempt_b, 200u64, 7, 11)?,
        generation_two
    );
    lose_current(&mut authority)?;
    assert_eq!(
        authority.prepare_reconnect(attempt_c, generation_two, 300u64, 7, 11),
        Err(AdmissionError::StaleConnection)
    );
    Ok(())
}
#[test]
fn lost_commit_response_reconciliation_is_idempotent() -> Result<(), AdmissionError> {
    let mut authority = test_authority();
    admit(&mut authority, 5, 500, 100u64)?;
    lose_current(&mut authority)?;
    let attempt = ReconnectAttemptRef::new(11)?;
    let first_generation =
        ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
    authority.prepare_reconnect(attempt, first_generation, 200u64, 7, 11)?;
    let first = authority.commit_reconnect(attempt, 200u64, 7, 11)?;
    assert_eq!(
        authority.commit_reconnect(attempt, 200u64, 7, 11),
        Ok(first)
    );
    assert_eq!(
        authority
            .current()
            .ok_or(AdmissionError::Terminal)?
            .connection_generation(),
        first
    );
    Ok(())
}
#[test]
fn game_session_id_is_never_reused_and_rejection_does_not_consume_nonce()
-> Result<(), AdmissionError> {
    let journal = TestReconnectAttemptJournal::default();
    let mut authority = AdmissionAuthority::new(journal.clone());
    admit(&mut authority, 20, 2000, 100u64)?;
    authority.terminate_current()?;
    assert_eq!(
        authority.commit_fresh(facts(21)?, 101u64, || game_session_id(2000)),
        Err(AdmissionError::InvalidFacts)
    );
    authority.commit_fresh(facts(21)?, 102u64, || game_session_id(2001))?;
    Ok(())
}

#[test]
fn fresh_admission_accepts_only_semantic_foundation_ids() -> Result<(), AdmissionError> {
    let mut raw = [0u8; 16];
    raw[6] = 0x70;
    raw[8] = 0x80;
    raw[15] = 1;
    let map = |_| AdmissionError::InvalidFacts;
    let mut nonce = [0u8; 32];
    nonce[31] = 1;
    FreshAdmissionFacts::new(
        nonce,
        CharacterId::decode(&raw).map_err(map)?,
        WorldId::decode(&raw).map_err(map)?,
        ChannelId::decode(&raw).map_err(map)?,
        1,
        1,
    )?;
    Ok(())
}
#[test]
fn game_session_id_is_issued_only_after_fresh_admission_preconditions_pass()
-> Result<(), AdmissionError> {
    use std::cell::Cell;

    let journal = TestReconnectAttemptJournal::default();
    let mut authority = AdmissionAuthority::new(journal.clone());
    let issue_calls = Cell::new(0u32);
    let first = facts(40)?;
    authority.commit_fresh(first, 100u64, || {
        issue_calls.set(issue_calls.get() + 1);
        game_session_id(4000)
    })?;
    assert_eq!(issue_calls.get(), 1);

    assert_eq!(
        authority.commit_fresh(facts(41)?, 101u64, || {
            issue_calls.set(issue_calls.get() + 1);
            game_session_id(4100)
        }),
        Err(AdmissionError::IncumbentHealthy)
    );
    assert_eq!(issue_calls.get(), 1);

    authority.terminate_current()?;
    assert_eq!(
        authority.commit_fresh(facts(40)?, 102u64, || {
            issue_calls.set(issue_calls.get() + 1);
            game_session_id(4200)
        }),
        Err(AdmissionError::GrantReplayed)
    );
    assert_eq!(issue_calls.get(), 1);
    Ok(())
}

#[test]
fn rejected_fresh_admission_never_issues_session_id_and_keeps_nonce_retryable()
-> Result<(), AdmissionError> {
    use std::cell::Cell;

    let journal = TestReconnectAttemptJournal::default();
    let mut authority = AdmissionAuthority::new(journal.clone());
    admit(&mut authority, 30, 3000, 100u64)?;
    let issue_calls = Cell::new(0u32);
    assert_eq!(
        authority.commit_fresh(facts(31)?, 101u64, || {
            issue_calls.set(issue_calls.get() + 1);
            game_session_id(3001)
        }),
        Err(AdmissionError::IncumbentHealthy)
    );
    assert_eq!(issue_calls.get(), 0);

    authority.terminate_current()?;
    authority.commit_fresh(facts(31)?, 102u64, || {
        issue_calls.set(issue_calls.get() + 1);
        game_session_id(3001)
    })?;
    assert_eq!(issue_calls.get(), 1);
    Ok(())
}
