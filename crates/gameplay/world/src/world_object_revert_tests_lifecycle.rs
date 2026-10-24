#[test]
fn a_due_record_fires_once_mints_one_ordinal_and_restores_the_literal_source() -> TestResult {
    let content = wall_content(
        &[(CRACK, WALL_ACTION, 300_000)],
        &[(WALL_A, 100)],
        "lock:r1",
    )?;
    let mut runtimes = walls(&content, &[WALL_A])?;
    let clock = ManualClock::new(Moment::ZERO);
    let mut driver = driver(&clock, RevertDriverLimits::registered())?;
    let id = scheduled(forward(
        &mut driver,
        &mut runtimes,
        WALL_A,
        "oteryn:occurrence/wall/1",
        WALL_ACTION,
        CRACK,
    )?)?;
    assert_eq!(driver.issuer().minted, 1);
    // Not yet due: presenting it early runs nothing.
    assert_eq!(
        driver.present(&id, &mut runtimes, &BTreeSet::new())?,
        RevertPresentation::NotDue
    );
    clock.advance(millis(300_000))?;
    let first = driver.present(&id, &mut runtimes, &BTreeSet::new())?;
    let second = driver.present(&id, &mut runtimes, &BTreeSet::new())?;
    assert_eq!(first, second);
    assert!(matches!(&first, RevertPresentation::Terminal(outcome) if outcome.committed()));
    assert_eq!(driver.issuer().minted, 2);
    assert_eq!(state_of(&runtimes, WALL_A)?, at(WALL_SEALED, 2));
    assert!(
        driver
            .wake(&mut runtimes, &BTreeSet::new())?
            .fired
            .is_empty()
    );
    Ok(())
}

#[test]
fn an_in_flight_duplicate_converges_without_an_ordinal_or_prepare() -> TestResult {
    let content = wall_content(&[(CRACK, WALL_ACTION, 1_000)], &[(WALL_A, 100)], "lock:r1")?;
    let mut runtimes = walls(&content, &[WALL_A])?;
    let clock = ManualClock::new(Moment::ZERO);
    let mut driver = driver(&clock, RevertDriverLimits::registered())?;
    let id = scheduled(forward(
        &mut driver,
        &mut runtimes,
        WALL_A,
        "oteryn:occurrence/wall/1",
        WALL_ACTION,
        CRACK,
    )?)?;
    clock.advance(millis(1_000))?;
    driver
        .force_in_flight_for_test(&id)
        .ok_or(fixture("force in flight"))?;
    // Even with the scope generation moved, an IN_FLIGHT record never re-runs fences.
    driver.issuer_mut_for_test().generation = generation(2)?;
    assert_eq!(
        driver.present(&id, &mut runtimes, &BTreeSet::new())?,
        RevertPresentation::InFlight
    );
    assert_eq!(driver.issuer().minted, 1);
    assert_eq!(state_of(&runtimes, WALL_A)?, at(WALL_CRACKED, 1));
    assert!(
        driver
            .wake(&mut runtimes, &BTreeSet::new())?
            .fired
            .is_empty()
    );
    Ok(())
}

#[derive(Clone, Copy)]
enum FenceChange {
    ScopeGeneration,
    Incarnation,
    ContentGeneration,
}

#[test]
fn pending_fences_reject_without_prepare_and_duplicates_return_the_first_outcome() -> TestResult {
    let timed = [(CRACK, WALL_ACTION, 1_000)];
    let content = wall_content(&timed, &[(WALL_A, 100)], "lock:r1")?;
    let other_generation = wall_content(&timed, &[(WALL_A, 100)], "lock:r2")?;
    let wall = PlacementKey::new(WALL_A)?;
    for (change, fence) in [
        (FenceChange::ScopeGeneration, RevertFence::ScopeGeneration),
        (FenceChange::Incarnation, RevertFence::Incarnation),
        (
            FenceChange::ContentGeneration,
            RevertFence::ContentGeneration,
        ),
    ] {
        let mut runtimes = walls(&content, &[WALL_A])?;
        let clock = ManualClock::new(Moment::ZERO);
        let mut driver = driver(&clock, RevertDriverLimits::registered())?;
        let id = scheduled(forward(
            &mut driver,
            &mut runtimes,
            WALL_A,
            "oteryn:occurrence/wall/1",
            WALL_ACTION,
            CRACK,
        )?)?;
        match change {
            FenceChange::ScopeGeneration => {
                driver.issuer_mut_for_test().generation = generation(2)?;
            }
            FenceChange::Incarnation => {
                runtimes.insert(wall.clone(), bind_at(&content, &wall, &[CRACK, MEND], 2)?);
            }
            FenceChange::ContentGeneration => {
                runtimes.insert(
                    wall.clone(),
                    bind_at(&other_generation, &wall, &[CRACK, MEND], 1)?,
                );
            }
        }
        let before = state_of(&runtimes, WALL_A)?;
        clock.advance(millis(1_000))?;
        let report = driver.wake(&mut runtimes, &BTreeSet::new())?;
        assert_eq!(
            report.fired,
            vec![(id.clone(), RevertOutcome::Fenced(fence))]
        );
        // No ordinal minted and no prepare/mutation for a fenced record.
        assert_eq!(driver.issuer().minted, 1);
        assert_eq!(state_of(&runtimes, WALL_A)?, before);
        assert_eq!(
            driver.present(&id, &mut runtimes, &BTreeSet::new())?,
            RevertPresentation::Terminal(RevertOutcome::Fenced(fence))
        );
    }
    Ok(())
}

#[test]
fn a_terminal_duplicate_returns_the_first_outcome_after_the_target_is_replaced() -> TestResult {
    let content = wall_content(&[(CRACK, WALL_ACTION, 1_000)], &[(WALL_A, 100)], "lock:r1")?;
    let mut runtimes = walls(&content, &[WALL_A])?;
    let clock = ManualClock::new(Moment::ZERO);
    let mut driver = driver(&clock, RevertDriverLimits::registered())?;
    let id = scheduled(forward(
        &mut driver,
        &mut runtimes,
        WALL_A,
        "oteryn:occurrence/wall/1",
        WALL_ACTION,
        CRACK,
    )?)?;
    clock.advance(millis(1_000))?;
    let report = driver.wake(&mut runtimes, &BTreeSet::new())?;
    let [(_, first)] = report.fired.as_slice() else {
        return Err("expected one fired revert".into());
    };
    assert!(first.committed());
    let wall = PlacementKey::new(WALL_A)?;
    runtimes.insert(wall.clone(), bind_at(&content, &wall, &[CRACK, MEND], 2)?);
    assert_eq!(
        driver.present(&id, &mut runtimes, &BTreeSet::new())?,
        RevertPresentation::Terminal(first.clone())
    );
    assert_eq!(driver.issuer().minted, 2);
    assert_eq!(state_of(&runtimes, WALL_A)?, at(WALL_SEALED, 0));
    Ok(())
}

#[test]
fn distinct_occurrences_keep_distinct_records_and_an_intervening_change_is_stale() -> TestResult {
    let content = wall_content(&[(CRACK, WALL_ACTION, 1_000)], &[(WALL_A, 100)], "lock:r1")?;
    let mut runtimes = walls(&content, &[WALL_A])?;
    let clock = ManualClock::new(Moment::ZERO);
    let mut driver = driver(&clock, RevertDriverLimits::registered())?;
    let first = scheduled(forward(
        &mut driver,
        &mut runtimes,
        WALL_A,
        "oteryn:occurrence/wall/1",
        WALL_ACTION,
        CRACK,
    )?)?;
    // Something else mends the wall, then the same transform fires again.
    let wall = PlacementKey::new(WALL_A)?;
    let runtime = runtimes.get_mut(&wall).ok_or(fixture("runtime"))?;
    assert_eq!(apply_plain(runtime, MEND)?, "COMMITTED");
    clock.advance(millis(10))?;
    let second = scheduled(forward(
        &mut driver,
        &mut runtimes,
        WALL_A,
        "oteryn:occurrence/wall/2",
        WALL_ACTION,
        CRACK,
    )?)?;
    assert_ne!(first, second);
    assert_eq!(driver.record_count(), 2);

    clock.advance(millis(1_000))?;
    let report = driver.wake(&mut runtimes, &BTreeSet::new())?;
    let [(first_id, first_outcome), (second_id, second_outcome)] = report.fired.as_slice() else {
        return Err("expected two fired reverts".into());
    };
    assert_eq!((first_id, second_id), (&first, &second));
    // The first record meets a changed object: STALE_STATE through `prepare`, no overwrite.
    assert_eq!(disposition(first_outcome), Some("STALE_STATE"));
    assert!(second_outcome.committed());
    assert_eq!(state_of(&runtimes, WALL_A)?, at(WALL_SEALED, 4));
    Ok(())
}

#[test]
fn distinct_forward_children_derive_distinct_canonical_revert_children() -> TestResult {
    // #1144 P1 4125535254: the record key is the canonical nested `ChildOccurrenceRef`, so
    // forward children differing in any discriminator (here only the ordinal) never share a
    // record.
    let content = wall_content(
        &[(CRACK, WALL_ACTION, 1_000)],
        &[(WALL_A, 100), (WALL_B, 300)],
        "lock:r1",
    )?;
    let mut runtimes = walls(&content, &[WALL_A, WALL_B])?;
    let clock = ManualClock::new(Moment::ZERO);
    let mut driver = driver(&clock, RevertDriverLimits::registered())?;
    let action = LoweredActionId::new(WALL_ACTION)?;
    let mut ids = Vec::new();
    for (wall, ordinal) in [(WALL_A, Some(0)), (WALL_B, Some(1))] {
        let key = PlacementKey::new(wall)?;
        let runtime = runtimes.get_mut(&key).ok_or(fixture("runtime"))?;
        let crack = operation(runtime, CRACK, 0)?;
        let forward = forward_child("oteryn:occurrence/boss-death", ordinal)?;
        let outcome = driver.apply_forward(
            runtime,
            &forward,
            &revisions()?,
            &action,
            &crack,
            &BTreeSet::new(),
        )?;
        let id = scheduled(outcome)?;
        // Derived through the canonical constructor: nested one level below the forward
        // child, deterministic, with the bound inverse as its edge.
        assert_eq!(id.ancestry_depth(), 2);
        assert_eq!(
            id,
            revert_child_occurrence(
                &forward,
                &action,
                &key,
                &TransitionKey::new(MEND)?,
                &revisions()?
            )?
        );
        ids.push(id);
    }
    assert_ne!(ids.first(), ids.get(1));
    assert_eq!(driver.record_count(), 2);
    Ok(())
}

#[test]
fn use_and_session_commands_cannot_commit_an_event_owned_transition() -> TestResult {
    // #1144 P1 4125535249, subsumed by owner decision D91: a timed transition is event-owned,
    // and USE and `apply` refuse every event-owned edge, exactly like an unbound one.
    let timed = wall_content(&[(CRACK, WALL_ACTION, 1_000)], &[(WALL_A, 100)], "lock:r1")?;
    let mut runtimes = walls(&timed, &[WALL_A])?;
    let wall = PlacementKey::new(WALL_A)?;
    let runtime = runtimes.get_mut(&wall).ok_or(fixture("runtime"))?;
    assert_eq!(
        runtime.attempt_use(0, &BTreeSet::new())?,
        LocalObjectUseOutcome::NothingToUse
    );
    let mut ingress = CommandIngress::new();
    assert!(matches!(
        apply_session(runtime, &mut ingress, 1, CRACK)?,
        Err(WorldRuntimeError::InvalidBinding(
            "command names a transition this local-object runtime does not bind"
        ))
    ));
    assert_eq!(ingress.outstanding(), 0);
    assert_eq!(state_of(&runtimes, WALL_A)?, at(WALL_SEALED, 0));

    // D91 replaces carry-over (b): the untimed but event-owned inverse is neither
    // USE-selectable nor session-invocable once the forward has committed.
    let clock = ManualClock::new(Moment::ZERO);
    let mut driver = driver(&clock, RevertDriverLimits::registered())?;
    scheduled(forward(
        &mut driver,
        &mut runtimes,
        WALL_A,
        "oteryn:occurrence/wall/1",
        WALL_ACTION,
        CRACK,
    )?)?;
    let runtime = runtimes.get_mut(&wall).ok_or(fixture("runtime"))?;
    assert_eq!(
        runtime.attempt_use(1, &BTreeSet::new())?,
        LocalObjectUseOutcome::NothingToUse
    );
    assert!(matches!(
        apply_session(runtime, &mut ingress, 1, MEND)?,
        Err(WorldRuntimeError::InvalidBinding(
            "command names a transition this local-object runtime does not bind"
        ))
    ));
    assert_eq!(ingress.outstanding(), 0);
    assert_eq!(state_of(&runtimes, WALL_A)?, at(WALL_CRACKED, 1));

    // The duke teleporter cannot be opened by USE from its natural state.
    let (content, revert) = duke_content(DUKE)?;
    let anchor = duke_anchor()?;
    let mut duke = bind_at(
        &content,
        &anchor,
        &[OPEN_TELEPORTER, &duke_rearm(), revert.as_str()],
        1,
    )?;
    assert_eq!(
        duke.attempt_use(0, &BTreeSet::new())?,
        LocalObjectUseOutcome::NothingToUse
    );
    assert_eq!(
        (duke.state_key().as_str(), duke.revision()),
        (SEALED_ITEM, 0)
    );
    Ok(())
}

#[test]
fn untimed_use_and_session_commands_are_unchanged() -> TestResult {
    let untimed = wall_content(&[], &[(WALL_A, 100), (WALL_B, 300)], "lock:r1")?;
    let mut runtimes = walls(&untimed, &[WALL_A, WALL_B])?;
    let runtime = runtimes
        .get_mut(&PlacementKey::new(WALL_A)?)
        .ok_or(fixture("runtime"))?;
    assert_eq!(
        runtime.attempt_use(0, &BTreeSet::new())?,
        LocalObjectUseOutcome::Committed {
            state: ProductionKey::new(WALL_CRACKED)?,
            revision: 1,
        }
    );
    let runtime = runtimes
        .get_mut(&PlacementKey::new(WALL_B)?)
        .ok_or(fixture("runtime"))?;
    let mut ingress = CommandIngress::new();
    assert_eq!(
        apply_session(runtime, &mut ingress, 1, CRACK)??,
        "COMMITTED"
    );
    assert_eq!(state_of(&runtimes, WALL_B)?, at(WALL_CRACKED, 1));
    Ok(())
}

#[test]
fn a_scope_operation_without_the_scheduling_capability_refuses_a_timed_transition() -> TestResult {
    // #1144 P1 4125881398: only the driver holds the capability, so no other crate caller
    // can commit a timed transition without its lifecycle record.
    let content = wall_content(&[(CRACK, WALL_ACTION, 1_000)], &[(WALL_A, 100)], "lock:r1")?;
    let mut runtimes = walls(&content, &[WALL_A])?;
    let wall = PlacementKey::new(WALL_A)?;
    let runtime = runtimes.get_mut(&wall).ok_or(fixture("runtime"))?;
    let refused = apply_plain(runtime, CRACK)
        .err()
        .and_then(|error| error.downcast::<WorldRuntimeError>().ok());
    assert!(matches!(
        refused.as_deref(),
        Some(WorldRuntimeError::InvalidBinding(
            "timed transition commits only through the revert scheduler"
        ))
    ));
    assert_eq!(state_of(&runtimes, WALL_A)?, at(WALL_SEALED, 0));

    // The driver commits it (with its record); an untimed operation needs no capability.
    let clock = ManualClock::new(Moment::ZERO);
    let mut driver = driver(&clock, RevertDriverLimits::registered())?;
    scheduled(forward(
        &mut driver,
        &mut runtimes,
        WALL_A,
        "oteryn:occurrence/wall/1",
        WALL_ACTION,
        CRACK,
    )?)?;
    let runtime = runtimes.get_mut(&wall).ok_or(fixture("runtime"))?;
    assert_eq!(apply_plain(runtime, MEND)?, "COMMITTED");
    assert_eq!(state_of(&runtimes, WALL_A)?, at(WALL_SEALED, 2));
    Ok(())
}
