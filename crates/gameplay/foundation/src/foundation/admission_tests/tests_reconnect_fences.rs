#[test]
fn healthy_binding_cannot_be_preempted_by_reconnect() -> Result<(), AdmissionError> {
    let mut authority = test_authority();
    admit(&mut authority, 2, 200, 100u64)?;
    assert_eq!(
        authority.prepare_reconnect(
            ReconnectAttemptRef::new(1)?,
            ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?,
            200u64,
            7,
            11
        ),
        Err(AdmissionError::IncumbentHealthy)
    );
    Ok(())
}

#[test]
fn prepare_has_no_authority_and_commit_fences_predecessor() -> Result<(), AdmissionError> {
    let mut authority = test_authority();
    admit(&mut authority, 3, 300, 100u64)?;
    lose_current(&mut authority)?;
    let attempt = ReconnectAttemptRef::new(9)?;
    let first = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
    let candidate = authority.prepare_reconnect(attempt, first, 200u64, 7, 11)?;
    assert_eq!(candidate.get(), 2);
    assert_eq!(
        authority
            .current()
            .ok_or(AdmissionError::Terminal)?
            .connection_generation()
            .get(),
        1
    );
    let committed = authority.commit_reconnect(attempt, 200u64, 7, 11)?;
    assert_eq!(committed.get(), 2);
    assert!(
        !authority
            .current()
            .ok_or(AdmissionError::Terminal)?
            .accepts_generation(first)
    );
    Ok(())
}

#[test]
fn control_loss_lost_response_reconciles_on_exact_retry() -> Result<(), AdmissionError> {
    let journal = TestReconnectAttemptJournal::default();
    let mut authority = AdmissionAuthority::new(journal.clone());
    admit(&mut authority, 97, 9700, 100u64)?;
    let generation_one = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
    journal.fail_next_control_loss_response_after_commit();

    assert_eq!(
        authority.mark_unexpected_control_loss(100u64, generation_one),
        Err(AdmissionError::ReconciliationUnavailable)
    );
    assert_eq!(authority.current_transport(), None);
    assert_eq!(
        authority.prepare_reconnect(
            ReconnectAttemptRef::new(970)?,
            generation_one,
            200u64,
            7,
            11,
        ),
        Err(AdmissionError::ReconciliationUnavailable)
    );

    assert_eq!(
        authority.mark_unexpected_control_loss(100u64, generation_one)?,
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
fn runtime_scope_lost_response_reconciles_authoritative_generation() -> Result<(), AdmissionError> {
    let journal = TestReconnectAttemptJournal::default();
    let mut authority = AdmissionAuthority::new(journal.clone());
    admit(&mut authority, 98, 9800, 100u64)?;
    let generation_eleven =
        ScopeOwnershipGeneration::new(11).map_err(|_| AdmissionError::InvalidFacts)?;
    let old_stamp = {
        let session = authority.current.as_mut().ok_or(AdmissionError::Terminal)?;
        let ordinal = session
            .runtime_scope
            .accept_input(generation_eleven)
            .map_err(|_| AdmissionError::StaleRuntime)?;
        session.runtime_scope.stamp(ordinal)
    };
    journal.fail_next_runtime_scope_response_after_commit();

    assert_eq!(
        authority.observe_runtime_ownership_generation(12),
        Err(AdmissionError::ReconciliationUnavailable)
    );
    assert_eq!(
        authority
            .current()
            .map(GameSession::runtime_scope_generation),
        Some(generation_eleven)
    );
    assert!(
        !authority
            .current()
            .ok_or(AdmissionError::Terminal)?
            .runtime_scope
            .accepts_stamp(old_stamp)
    );

    authority.observe_runtime_ownership_generation(12)?;
    assert_eq!(
        authority
            .current()
            .map(GameSession::runtime_scope_generation),
        Some(ScopeOwnershipGeneration::new(12).map_err(|_| AdmissionError::InvalidFacts)?)
    );
    Ok(())
}

#[test]
fn control_loss_is_authoritative_without_test_only_resynchronization() -> Result<(), AdmissionError>
{
    let mut authority = test_authority();
    admit(&mut authority, 96, 9600, 100u64)?;
    let generation_one = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;

    assert_eq!(
        authority.mark_unexpected_control_loss(100u64, generation_one)?,
        ControlLossDisposition::Applied
    );

    let attempt = ReconnectAttemptRef::new(96)?;
    let generation_two = authority.prepare_reconnect(attempt, generation_one, 200u64, 7, 11)?;
    assert_eq!(
        authority.commit_reconnect(attempt, 200u64, 7, 11)?,
        generation_two
    );
    Ok(())
}

#[test]
fn reconnect_commit_revalidates_authoritative_lease_inside_atomic_journal_boundary()
-> Result<(), AdmissionError> {
    let mut authority = test_authority();
    admit(&mut authority, 92, 9200, 100u64)?;
    lose_current(&mut authority)?;
    let attempt = ReconnectAttemptRef::new(92)?;
    let predecessor = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
    let candidate = authority.prepare_reconnect(attempt, predecessor, 200u64, 7, 11)?;
    let (game_session_id, character_id, scope_generation) = {
        let session = authority.current().ok_or(AdmissionError::Terminal)?;
        (
            session.game_session_id(),
            session.character_id(),
            session.runtime_scope_generation(),
        )
    };
    authority.reconnect_attempts.set_authoritative_fences(
        game_session_id,
        CharacterLease {
            character_id,
            generation: 8,
        },
        scope_generation,
    );

    assert_eq!(
        authority.commit_reconnect(attempt, 200u64, 7, 11),
        Err(AdmissionError::StaleLease)
    );
    assert_eq!(
        authority
            .current()
            .ok_or(AdmissionError::Terminal)?
            .connection_generation(),
        predecessor
    );
    assert!(authority.prepared.is_none());
    assert_eq!(
        authority
            .reconnect_attempts
            .lookup(game_session_id, attempt)?,
        Some(ReconnectAttemptDisposition::TerminallySuperseded)
    );
    assert_eq!(candidate.get(), 2);
    Ok(())
}

#[test]
fn reconnect_commit_revalidates_authoritative_runtime_inside_atomic_journal_boundary()
-> Result<(), AdmissionError> {
    let mut authority = test_authority();
    admit(&mut authority, 93, 9300, 100u64)?;
    lose_current(&mut authority)?;
    let attempt = ReconnectAttemptRef::new(93)?;
    let predecessor = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
    authority.prepare_reconnect(attempt, predecessor, 200u64, 7, 11)?;
    let (game_session_id, character_lease) = {
        let session = authority.current().ok_or(AdmissionError::Terminal)?;
        (session.game_session_id(), session.character_lease())
    };
    let newer_scope =
        ScopeOwnershipGeneration::new(12).map_err(|_| AdmissionError::InvalidFacts)?;
    authority.reconnect_attempts.set_authoritative_fences(
        game_session_id,
        character_lease,
        newer_scope,
    );

    assert_eq!(
        authority.commit_reconnect(attempt, 200u64, 7, 11),
        Err(AdmissionError::StaleRuntime)
    );
    assert_eq!(
        authority
            .current()
            .ok_or(AdmissionError::Terminal)?
            .connection_generation(),
        predecessor
    );
    assert!(authority.prepared.is_none());
    assert_eq!(
        authority
            .reconnect_attempts
            .lookup(game_session_id, attempt)?,
        Some(ReconnectAttemptDisposition::TerminallySuperseded)
    );
    Ok(())
}

#[test]
fn reconnect_commit_revalidates_authoritative_session_binding_inside_atomic_journal_boundary()
-> Result<(), AdmissionError> {
    let mut authority = test_authority();
    admit(&mut authority, 94, 9400, 100u64)?;
    lose_current(&mut authority)?;
    let attempt = ReconnectAttemptRef::new(94)?;
    let predecessor = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
    authority.prepare_reconnect(attempt, predecessor, 200u64, 7, 11)?;
    let (game_session_id, character_lease, scope_generation) = {
        let session = authority.current().ok_or(AdmissionError::Terminal)?;
        (
            session.game_session_id(),
            session.character_lease(),
            session.runtime_scope_generation(),
        )
    };
    authority.reconnect_attempts.set_authoritative_fences(
        game_session_id,
        character_lease,
        scope_generation,
    );
    authority.reconnect_attempts.set_authoritative_session(
        game_session_id,
        GameSessionState::Active,
        ConnectionGeneration::new(2).map_err(|_| AdmissionError::InvalidFacts)?,
        Some(300u64),
    );

    assert_eq!(
        authority.commit_reconnect(attempt, 200u64, 7, 11),
        Err(AdmissionError::StaleConnection)
    );
    assert_eq!(
        authority
            .current()
            .ok_or(AdmissionError::Terminal)?
            .connection_generation(),
        predecessor
    );
    Ok(())
}

#[test]
fn reconnect_commit_rejects_recovered_controller_without_generation_change()
-> Result<(), AdmissionError> {
    let mut authority = test_authority();
    admit(&mut authority, 95, 9500, 100u64)?;
    lose_current(&mut authority)?;
    let attempt = ReconnectAttemptRef::new(95)?;
    let predecessor = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
    authority.prepare_reconnect(attempt, predecessor, 200u64, 7, 11)?;
    let game_session_id = authority
        .current()
        .ok_or(AdmissionError::Terminal)?
        .game_session_id();
    authority.reconnect_attempts.set_authoritative_session(
        game_session_id,
        GameSessionState::Active,
        predecessor,
        Some(300u64),
    );

    assert_eq!(
        authority.commit_reconnect(attempt, 200u64, 7, 11),
        Err(AdmissionError::StaleConnection)
    );
    assert_eq!(
        authority
            .current()
            .ok_or(AdmissionError::Terminal)?
            .connection_generation(),
        predecessor
    );
    assert!(authority.prepared.is_none());
    Ok(())
}

#[test]
fn reconnect_commit_rejects_terminal_transition_inside_atomic_journal_boundary()
-> Result<(), AdmissionError> {
    let mut authority = test_authority();
    admit(&mut authority, 96, 9600, 100u64)?;
    lose_current(&mut authority)?;
    let attempt = ReconnectAttemptRef::new(96)?;
    let predecessor = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
    authority.prepare_reconnect(attempt, predecessor, 200u64, 7, 11)?;
    let game_session_id = authority
        .current()
        .ok_or(AdmissionError::Terminal)?
        .game_session_id();
    authority.reconnect_attempts.set_authoritative_session(
        game_session_id,
        GameSessionState::Terminal,
        predecessor,
        None,
    );

    assert_eq!(
        authority.commit_reconnect(attempt, 200u64, 7, 11),
        Err(AdmissionError::Terminal)
    );
    assert_eq!(
        authority
            .current()
            .ok_or(AdmissionError::Terminal)?
            .connection_generation(),
        predecessor
    );
    assert!(authority.prepared.is_none());
    Ok(())
}

#[test]
fn stale_lease_or_runtime_generation_fails_before_switch() -> Result<(), AdmissionError> {
    let mut authority = test_authority();
    admit(&mut authority, 4, 400, 100u64)?;
    lose_current(&mut authority)?;
    let attempt = ReconnectAttemptRef::new(10)?;
    let first = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
    authority.prepare_reconnect(attempt, first, 200u64, 7, 11)?;
    assert_eq!(
        authority.commit_reconnect(attempt, 200u64, 8, 11),
        Err(AdmissionError::StaleLease)
    );
    assert_eq!(
        authority
            .current()
            .ok_or(AdmissionError::Terminal)?
            .connection_generation()
            .get(),
        1
    );
    Ok(())
}

#[test]
fn mismatched_commit_does_not_destroy_prepared_candidate() -> Result<(), AdmissionError> {
    let mut authority = test_authority();
    admit(&mut authority, 6, 600, 100u64)?;
    lose_current(&mut authority)?;
    let prepared_attempt = ReconnectAttemptRef::new(12)?;
    let wrong_attempt = ReconnectAttemptRef::new(13)?;
    let first = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
    let candidate = authority.prepare_reconnect(prepared_attempt, first, 200u64, 7, 11)?;
    assert_eq!(
        authority.commit_reconnect(wrong_attempt, 200u64, 7, 11),
        Err(AdmissionError::AttemptMismatch)
    );
    assert_eq!(
        authority.commit_reconnect(prepared_attempt, 200u64, 7, 11)?,
        candidate
    );
    Ok(())
}

#[test]
fn reconnect_commit_requires_exact_prepared_authenticated_transport() -> Result<(), AdmissionError>
{
    let mut authority = test_authority();
    admit(&mut authority, 9, 900, 100u64)?;
    lose_current(&mut authority)?;
    let attempt = ReconnectAttemptRef::new(22)?;
    let first = ConnectionGeneration::new(1).map_err(|_| AdmissionError::InvalidFacts)?;
    let candidate = authority.prepare_reconnect(attempt, first, 200u64, 7, 11)?;
    assert_eq!(
        authority.commit_reconnect(attempt, 201u64, 7, 11),
        Err(AdmissionError::AttemptMismatch)
    );
    assert_eq!(
        authority.commit_reconnect(attempt, 200u64, 7, 11)?,
        candidate
    );
    assert_eq!(authority.current_transport(), Some(200u64));
    Ok(())
}
