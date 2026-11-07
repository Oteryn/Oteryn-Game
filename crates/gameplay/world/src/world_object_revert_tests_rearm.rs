/// `od9_bound`, re-bound under `scope` at `scope_generation` and opened by its own driver.
fn od9_open_in(
    clock: &ManualClock,
    key: &str,
    encounter: &str,
    scope: RuntimeScopeRefV1,
    scope_generation: ScopeOwnershipGeneration,
) -> TestResult<(ScopeRevertDriver<TestIssuer>, Od9, ChildOccurrenceRef)> {
    let mut od9 = od9_bound(key, encounter)?;
    let runtime = bind_in(
        &od9.content,
        &od9.anchor,
        &[&od9.create, &od9.remove],
        1,
        scope,
        scope_generation,
    )?;
    od9.runtimes.insert(od9.anchor.clone(), runtime);
    let mut driver = driver_in(clock, scope, scope_generation);
    let id = opened(od9_kill(
        &mut driver,
        &mut od9,
        key,
        &format!("{key}/death/1"),
    )?)?;
    Ok((driver, od9, id))
}

/// #1204 review (P1): `foreign` is an open teleporter at the same placement, incarnation,
/// content generation, state and revision as the one `driver` scheduled `id` for, but bound
/// under another scope or scope generation. Its re-arm is refused with `expected`, and
/// nothing changes on either side.
fn assert_foreign_re_arm_refused(
    driver: &mut ScopeRevertDriver<TestIssuer>,
    (key, od9, id): (&str, &Od9, &ChildOccurrenceRef),
    foreign: &Od9,
    expected: &WorldRuntimeError,
) -> TestResult {
    let runtime = foreign
        .runtimes
        .get(&foreign.anchor)
        .ok_or(fixture("foreign runtime"))?;
    let (minted, deadline, records) = (
        driver.issuer().minted,
        driver.next_deadline(),
        driver.record_count(),
    );
    let lifecycle = driver.lifecycle(id).cloned();
    match driver.rearm_open_create(
        runtime,
        &LoweredActionId::new(&od9.action)?,
        &TransitionKey::new(&od9.create)?,
        &TransitionEventOwner::new(key)?,
    ) {
        Err(RevertError::Runtime(ref error))
            if std::mem::discriminant(error) == std::mem::discriminant(expected) => {}
        other => return Err(format!("expected {expected:?}, got {other:?}").into()),
    }
    assert_eq!(driver.issuer().minted, minted);
    assert_eq!(driver.next_deadline(), deadline);
    assert_eq!(driver.record_count(), records);
    assert_eq!(driver.lifecycle(id).cloned(), lifecycle);
    assert_eq!(
        state_of(&od9.runtimes, od9.anchor.as_str())?,
        at(SEALED_ITEM, 1)
    );
    assert_eq!(
        state_of(&foreign.runtimes, foreign.anchor.as_str())?,
        at(SEALED_ITEM, 1)
    );
    Ok(())
}

#[test]
fn a_re_arm_with_a_runtime_from_another_scope_or_a_stale_generation_is_refused() -> TestResult {
    let other_scope =
        RuntimeScopeRefV1::channel(world(1)?, ChannelId::decode(&uuid_v7(4)).map_err(fixture)?);
    for (key, _, encounter) in OD9_SAMPLES {
        let clock = ManualClock::new(Moment::ZERO);

        // A runtime from a different scope: another channel of the same world.
        let (mut driver, od9, id) = od9_open_in(&clock, key, encounter, scope()?, generation(1)?)?;
        let (_, foreign, _) = od9_open_in(&clock, key, encounter, other_scope, generation(1)?)?;
        assert_foreign_re_arm_refused(
            &mut driver,
            (key, &od9, &id),
            &foreign,
            &WorldRuntimeError::StaleRuntimeScope,
        )?;

        // A runtime from a stale scope generation: bound at generation 1 while this driver
        // owns the scope at generation 2.
        let (mut driver, od9, id) = od9_open_in(&clock, key, encounter, scope()?, generation(2)?)?;
        let (_, stale, _) = od9_open_in(&clock, key, encounter, scope()?, generation(1)?)?;
        assert_foreign_re_arm_refused(
            &mut driver,
            (key, &od9, &id),
            &stale,
            &WorldRuntimeError::StaleScopeOwnershipGeneration,
        )?;

        // The driver's own runtime still re-arms its record.
        clock.advance(millis(60_000))?;
        let runtime = od9.runtimes.get(&od9.anchor).ok_or(fixture("runtime"))?;
        assert_eq!(
            driver.rearm_open_create(
                runtime,
                &LoweredActionId::new(&od9.action)?,
                &TransitionKey::new(&od9.create)?,
                &TransitionEventOwner::new(key)?,
            )?,
            id
        );
    }
    Ok(())
}

#[test]
fn a_re_kill_without_one_pending_revert_fails_closed_and_the_duke_keeps_its_no_op() -> TestResult {
    // Q2=b: an open teleporter whose revert is no longer PENDING re-arms nothing.
    let (key, _, encounter) = OD9_SAMPLES[0];
    let mut od9 = od9_bound(key, encounter)?;
    let clock = ManualClock::new(Moment::ZERO);
    let mut driver = driver(&clock, RevertDriverLimits::registered())?;
    let id = opened(od9_kill(
        &mut driver,
        &mut od9,
        key,
        &format!("{key}/death/1"),
    )?)?;
    driver
        .force_in_flight_for_test(&id)
        .ok_or(fixture("force IN_FLIGHT"))?;
    let minted = driver.issuer().minted;
    assert!(matches!(
        od9_kill(&mut driver, &mut od9, key, &format!("{key}/death/open")),
        Err(error) if matches!(
            error.downcast_ref::<RevertError>(),
            Some(RevertError::NoOpenRevert)
        )
    ));
    assert_eq!(driver.issuer().minted, minted);
    assert_eq!(driver.record_count(), 1);

    // Owner decision 3 still covers the duke's TRANSFORM teleporter: no CREATE edge opens it.
    let (content, revert) = duke_content(DUKE)?;
    let anchor = duke_anchor()?;
    let mut runtimes = BTreeMap::from([(
        anchor.clone(),
        bind_at(
            &content,
            &anchor,
            &[OPEN_TELEPORTER, &duke_rearm(), revert.as_str()],
            1,
        )?,
    )]);
    let mut duke_driver = self::driver(&clock, RevertDriverLimits::registered())?;
    scheduled(forward(
        &mut duke_driver,
        &mut runtimes,
        anchor.as_str(),
        DUKE_DEATH,
        DUKE_ACTION,
        OPEN_TELEPORTER,
    )?)?;
    let runtime = runtimes.get(&anchor).ok_or(fixture("runtime"))?;
    let action = LoweredActionId::new(DUKE_ACTION)?;
    assert_eq!(
        select_timed_forward(runtime, &action, &content.transitions)?,
        None
    );
    assert_eq!(
        select_open_create(runtime, &action, &content.transitions)?,
        None
    );

    // #1204 review (P1): the re-arm boundary refuses the open timed TRANSFORM edge itself, so
    // no caller can bypass the selector: nothing is minted and no deadline moves.
    let (minted, deadline, records) = (
        duke_driver.issuer().minted,
        duke_driver.next_deadline(),
        duke_driver.record_count(),
    );
    clock.advance(millis(60_000))?;
    assert!(matches!(
        duke_driver.rearm_open_create(
            runtime,
            &action,
            &TransitionKey::new(OPEN_TELEPORTER)?,
            &TransitionEventOwner::new(DUKE_DEATH)?,
        ),
        Err(RevertError::NotCreateTransition)
    ));
    assert_eq!(duke_driver.issuer().minted, minted);
    assert_eq!(duke_driver.next_deadline(), deadline);
    assert_eq!(duke_driver.record_count(), records);
    Ok(())
}
