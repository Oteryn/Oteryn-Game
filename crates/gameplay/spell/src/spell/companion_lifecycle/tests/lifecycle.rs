/// #2199 Codex P1 4301237612 / 4301237626: both actor ends (logout and the end of the
/// in-fight deadline) close the row with this session's clean-end marker, which the durable
/// clean-end proof accepts; with no physical familiar a waiting clean row keeps its stored
/// time and only an open row that already ended here stores zero.
#[test]
fn actor_end_close_marks_clean_end_and_keeps_waiting_time()
-> Result<(), CompanionExecutionError> {
    use crate::durability::character_familiar::{
        FamiliarCleanEndProof, clean_end_marker_requirement,
    };
    let (old_writer, current) = writers();
    let transport = crate::foundation::AuthenticatedTransportRefV1::decode(&[9; 16])
        .map_err(|_| CompanionExecutionError::InvalidOwnerFacts)?;
    let cases = [
        // a live familiar: its actual timer remaining wins
        (
            lifecycle(&current, 900_000, true, None),
            Some(41_000),
            41_000,
        ),
        // waiting to return (no free tile at login): stored time survives the end
        (
            lifecycle(&current, 90_000, false, Some(&old_writer)),
            None,
            90_000,
        ),
        // an acquisition whose familiar already ended in this runtime
        (lifecycle(&current, 900_000, true, None), None, 0),
    ];
    for (before, physical, expected) in cases {
        let mut after = before.clone();
        close_familiar_lifecycle_at_actor_end(&mut after, &current, physical)?;
        assert_eq!(after.familiar_remaining_ms, expected);
        assert!(!after.open);
        assert_eq!(after.writer, current);
        assert_eq!(
            after.clean_end_game_session_id.as_deref(),
            Some(current.game_session_id.as_str())
        );
        for proof in [
            FamiliarCleanEndProof::GraceExpiry(1),
            FamiliarCleanEndProof::Logout(transport),
        ] {
            let required = clean_end_marker_requirement(
                before.clean_end_game_session_id.as_deref(),
                after.clean_end_game_session_id.as_deref(),
                &current.game_session_id,
                Some(proof),
            );
            assert!(matches!(required, Ok(Some(p)) if p == proof));
        }
        // F-I11: after its own clean end the session writes nothing more.
        let mut again = after.clone();
        assert_eq!(
            close_familiar_lifecycle_at_actor_end(&mut again, &current, physical),
            Err(CompanionExecutionError::InvalidOwnerFacts)
        );
        assert_eq!(again, after);
    }
    Ok(())
}

/// #2199 Codex P1 4301237622: a Login that finds no free tile carries the previous
/// session's clean row (marker, stored waiting time) as the ordinary write accepts it, so
/// the cooldown and the delayed return are kept; this session's own marker is refused.
#[test]
fn no_room_login_carries_foreign_clean_row() -> Result<(), CompanionExecutionError> {
    use crate::durability::character_familiar::clean_end_marker_requirement;
    let (old_writer, current) = writers();
    let before = lifecycle(&old_writer, 90_000, false, Some(&old_writer));
    let mut after = before.clone();
    prepare_familiar_login_lifecycle(&mut after, &current)?;
    settle_familiar_event_lifecycle(&mut after, &current, &FamiliarEvent::Login, None)?;
    assert_eq!(
        after,
        FamiliarLifecycleV2 {
            writer: current.clone(),
            ..before.clone()
        }
    );
    assert!(familiar_waits_to_return(&after));
    assert!(matches!(
        clean_end_marker_requirement(
            before.clean_end_game_session_id.as_deref(),
            after.clean_end_game_session_id.as_deref(),
            &current.game_session_id,
            None,
        ),
        Ok(None)
    ));
    // The later delayed return (Advance) acquires: open, marker cleared, new lifetime.
    let mut returned = after.clone();
    settle_familiar_event_lifecycle(
        &mut returned,
        &current,
        &FamiliarEvent::Advance,
        Some(90_000),
    )?;
    assert!(returned.open);
    assert_eq!(returned.clean_end_game_session_id, None);
    assert!(matches!(
        clean_end_marker_requirement(
            after.clean_end_game_session_id.as_deref(),
            None,
            &current.game_session_id,
            None,
        ),
        Ok(None)
    ));
    let mut own = lifecycle(&current, 90_000, false, Some(&current));
    assert_eq!(
        settle_familiar_event_lifecycle(&mut own, &current, &FamiliarEvent::Login, None),
        Err(CompanionExecutionError::InvalidOwnerFacts)
    );
    Ok(())
}

/// #2199 Codex P2 4302737307: a due expiry closes the open row with no time left, as a
/// familiar death does, so an expired familiar never leaves an open row with positive
/// time. A write that ends nothing (Advance) keeps the open row and its time.
#[test]
fn familiar_expiry_closes_the_open_lifecycle_like_death() -> Result<(), CompanionExecutionError>
{
    let (_, current) = writers();
    for event in [FamiliarEvent::Expiry, FamiliarEvent::FamiliarDeath] {
        let mut row = lifecycle(&current, 900_000, true, None);
        settle_familiar_event_lifecycle(&mut row, &current, &event, None)?;
        assert_eq!(row, lifecycle(&current, 0, false, None));
    }
    let mut row = lifecycle(&current, 900_000, true, None);
    settle_familiar_event_lifecycle(&mut row, &current, &FamiliarEvent::Advance, None)?;
    assert_eq!(row, lifecycle(&current, 900_000, true, None));
    Ok(())
}

/// #2199 Codex P1 4301955863: an open row from a prior session or runtime that Login has
/// not reconciled blocks Cast and Advance; this lifecycle's own open row (a same-runtime
/// reconnect at another connection generation) and any closed row do not.
#[test]
fn foreign_open_lifecycle_is_unreconciled_until_login() -> Result<(), CompanionExecutionError> {
    let (old_writer, current) = writers();
    assert!(familiar_lifecycle_unreconciled(
        &lifecycle(&old_writer, 90_000, true, None),
        &current
    ));
    let moved = FamiliarLifecycleWriter {
        scope_ownership_generation: 5,
        ..current.clone()
    };
    assert!(familiar_lifecycle_unreconciled(
        &lifecycle(&moved, 90_000, true, None),
        &current
    ));
    let reconnect = FamiliarLifecycleWriter {
        connection_generation: 2,
        ..current.clone()
    };
    assert!(!familiar_lifecycle_unreconciled(
        &lifecycle(&reconnect, 90_000, true, None),
        &current
    ));
    assert!(!familiar_lifecycle_unreconciled(
        &lifecycle(&old_writer, 90_000, false, Some(&old_writer)),
        &current
    ));
    assert!(!familiar_lifecycle_unreconciled(
        &lifecycle(&old_writer, 0, false, None),
        &current
    ));
    // The waiting-to-return check alone does not catch it; the unreconciled check
    // refuses that cast before payment.
    assert!(!familiar_waits_to_return(&lifecycle(
        &old_writer,
        90_000,
        true,
        None
    )));
    // Login is the reconciliation: afterwards the row is closed and no longer blocks.
    let mut reconciled = lifecycle(&old_writer, 90_000, true, None);
    prepare_familiar_login_lifecycle(&mut reconciled, &current)?;
    assert!(!familiar_lifecycle_unreconciled(&reconciled, &current));
    assert!(!familiar_waits_to_return(&reconciled));
    Ok(())
}

/// #2199 Codex P2 4301955868: a Login or Advance that changes nothing but the D1126
/// conversion of an absent projection keeps it absent, so no row or revision is written.
#[test]
fn no_op_event_keeps_absent_projection_absent() -> Result<(), CompanionExecutionError> {
    let (_, current) = writers();
    let absent = DurableFamiliarState::default();
    for event in [FamiliarEvent::Login, FamiliarEvent::Advance] {
        let mut after = absent
            .for_next_write(None, &current)
            .map_err(|_| CompanionExecutionError::InvalidOwnerFacts)?;
        let converted = after
            .lifecycle
            .as_mut()
            .ok_or(CompanionExecutionError::InvalidOwnerFacts)?;
        if event == FamiliarEvent::Login {
            prepare_familiar_login_lifecycle(converted, &current)?;
        }
        settle_familiar_event_lifecycle(converted, &current, &event, None)?;
        assert_ne!(after, absent);
        let mut kept = after.clone();
        keep_unconverted_projection_on_no_op(&absent, &mut kept, &event);
        assert_eq!(kept, absent);
        // A real mutation keeps the conversion, which every new write requires.
        let mut changed = after.clone();
        changed.selected_look = 7;
        let expected = changed.clone();
        keep_unconverted_projection_on_no_op(&absent, &mut changed, &event);
        assert_eq!(changed, expected);
        assert!(changed.lifecycle.is_some());
    }
    // A Cast always writes, and a stored v2 row is never reverted.
    let mut cast = absent
        .for_next_write(None, &current)
        .map_err(|_| CompanionExecutionError::InvalidOwnerFacts)?;
    let expected = cast.clone();
    keep_unconverted_projection_on_no_op(&absent, &mut cast, &FamiliarEvent::Cast);
    assert_eq!(cast, expected);
    let (old_writer, _) = writers();
    let stored = DurableFamiliarState {
        lifecycle: Some(lifecycle(&old_writer, 0, false, None)),
        ..DurableFamiliarState::default()
    };
    let mut next = stored.clone();
    settle_familiar_event_lifecycle(
        next.lifecycle
            .as_mut()
            .ok_or(CompanionExecutionError::InvalidOwnerFacts)?,
        &current,
        &FamiliarEvent::Login,
        None,
    )?;
    let expected = next.clone();
    keep_unconverted_projection_on_no_op(&stored, &mut next, &FamiliarEvent::Login);
    assert_eq!(next, expected);
    Ok(())
}

/// #2199 Codex P1 4301237630 (FAMILIARS0 §4 check 4): a stored familiar waiting to return
/// refuses a cast; an open acquisition or a row with no stored time does not wait.
#[test]
fn waiting_stored_familiar_blocks_cast() {
    let (old_writer, current) = writers();
    assert!(familiar_waits_to_return(&lifecycle(
        &current,
        90_000,
        false,
        Some(&old_writer)
    )));
    assert!(familiar_waits_to_return(&lifecycle(
        &current, 1, false, None
    )));
    assert!(!familiar_waits_to_return(&lifecycle(
        &current, 90_000, true, None
    )));
    assert!(!familiar_waits_to_return(&lifecycle(
        &current,
        0,
        false,
        Some(&old_writer)
    )));
}

/// #2199 Codex P2 4302041461: a runtime return admits only the row this session's Login
/// left waiting, at its epoch, so the stored familiar returns once and is never doubled
/// or adopted from another lifecycle.
#[test]
fn stored_return_admits_only_this_lifecycles_waiting_row() {
    let (old_writer, current) = writers();
    let row = |lifecycle: Option<FamiliarLifecycleV2>, epoch: u64| DurableFamiliarState {
        lifecycle,
        lifecycle_epoch: epoch,
        ..DurableFamiliarState::default()
    };
    // Login left it waiting: clean, stored time, this writer, the previous session's marker.
    let waiting = row(
        Some(lifecycle(&current, 90_000, false, Some(&old_writer))),
        3,
    );
    assert!(familiar_return_admissible(&waiting, None, &current, 3));
    // A reconnect of the same lifecycle at another connection generation still returns it.
    let reconnect = FamiliarLifecycleWriter {
        connection_generation: 2,
        ..current.clone()
    };
    assert!(familiar_return_admissible(&waiting, None, &reconnect, 3));
    // A different epoch: the return already committed (or another row replaced it).
    assert!(!familiar_return_admissible(&waiting, None, &current, 2));
    assert!(!familiar_return_admissible(&waiting, None, &current, 4));
    // The return committed: the row is open with its scheduled lifetime.
    let open = row(Some(lifecycle(&current, 90_000, true, None)), 4);
    assert!(!familiar_return_admissible(&open, None, &current, 4));
    // No stored time, a row Login has not reconciled, or this session's own clean end.
    let empty = row(Some(lifecycle(&current, 0, false, None)), 3);
    assert!(!familiar_return_admissible(&empty, None, &current, 3));
    let foreign = row(
        Some(lifecycle(&old_writer, 90_000, false, Some(&old_writer))),
        3,
    );
    assert!(!familiar_return_admissible(&foreign, None, &current, 3));
    let ended = row(Some(lifecycle(&current, 90_000, false, Some(&current))), 3);
    assert!(!familiar_return_admissible(&ended, None, &current, 3));
    // A legacy row that Login kept unconverted returns through the same D1126 conversion
    // as its next write; without its checkpoint it is unknown and refused.
    let legacy = DurableFamiliarState {
        saved_expiry_unix: 2_000,
        lifecycle_epoch: 3,
        ..DurableFamiliarState::default()
    };
    assert!(familiar_return_admissible(
        &legacy,
        Some(1_000_000),
        &current,
        3
    ));
    assert!(!familiar_return_admissible(&legacy, None, &current, 3));
    assert!(!familiar_return_admissible(
        &legacy,
        Some(2_000_000),
        &current,
        3
    ));
}

/// #2199 Codex P2 4302041461: only a Login stopped by the source spawn-room refusal leaves
/// a return waiting; a created familiar or a Login without that refusal does not.
#[test]
fn only_a_no_room_login_leaves_a_return_waiting() {
    let refusal = FamiliarAction::CreationRefusal {
        message: "no room".into(),
        poff: true,
    };
    assert!(login_left_familiar_waiting(
        None,
        &[FamiliarAction::RegisterAdvance, refusal]
    ));
    assert!(!login_left_familiar_waiting(
        None,
        &[FamiliarAction::RegisterAdvance]
    ));
    assert!(!login_left_familiar_waiting(None, &[]));
}
