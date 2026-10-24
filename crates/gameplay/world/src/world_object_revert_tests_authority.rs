fn nested_forward(depth: usize) -> Result<ChildOccurrenceRef, InteractionError> {
    let mut occurrence = forward_child("oteryn:occurrence/nested", None)?;
    for _ in 1..depth {
        occurrence = ChildOccurrenceRef::for_child(
            &occurrence,
            "encounter:cascade",
            "encounter:anchor",
            "transform",
            None,
            &revisions()?,
        )?;
    }
    Ok(occurrence)
}

fn forward_with(
    driver: &mut ScopeRevertDriver<TestIssuer>,
    runtimes: &mut BTreeMap<PlacementKey, LocalObjectRuntime>,
    occurrence: &ChildOccurrenceRef,
    revisions: &SemanticRevisionContext,
) -> TestResult<Result<ForwardOutcome, RevertError>> {
    let runtime = runtimes
        .get_mut(&PlacementKey::new(WALL_A)?)
        .ok_or(fixture("runtime"))?;
    let crack = operation(runtime, CRACK, runtime.revision())?;
    Ok(driver.apply_forward(
        runtime,
        occurrence,
        revisions,
        &LoweredActionId::new(WALL_ACTION)?,
        &crack,
        &BTreeSet::new(),
    ))
}

#[test]
fn over_deep_or_over_large_occurrences_are_refused_before_commit() -> TestResult {
    // #1144 P2 4125881422: WOBJ-RL-06/07 bound what one record retains.
    let content = wall_content(&[(CRACK, WALL_ACTION, 1_000)], &[(WALL_A, 100)], "lock:r1")?;
    let clock = ManualClock::new(Moment::ZERO);
    let long = format!(
        "encounter:{}",
        "x".repeat(MAX_REVERT_OCCURRENCE_RENDERED_BYTES)
    );
    let too_large_forward = ChildOccurrenceRef::for_root(
        &RootSourceOccurrenceRef::new("oteryn:occurrence/large")?,
        "encounter:map_item",
        &long,
        "transform",
        None,
        &revisions()?,
    )?;
    let too_large_revisions = SemanticRevisionContext::new(&long, "ruleset:r1", "sim:v1")?;
    let cases = [
        (
            nested_forward(MAX_REVERT_OCCURRENCE_DEPTH)?,
            revisions()?,
            "OccurrenceTooDeep",
        ),
        (too_large_forward, revisions()?, "OccurrenceTooLarge"),
        (
            forward_child("oteryn:occurrence/wall/1", None)?,
            too_large_revisions,
            "OccurrenceTooLarge",
        ),
    ];
    for (occurrence, context, expected) in cases {
        let mut runtimes = walls(&content, &[WALL_A])?;
        let mut driver = driver(&clock, RevertDriverLimits::registered())?;
        let refused = forward_with(&mut driver, &mut runtimes, &occurrence, &context)?;
        assert!(
            matches!(
                (&refused, expected),
                (Err(RevertError::OccurrenceTooDeep), "OccurrenceTooDeep")
                    | (Err(RevertError::OccurrenceTooLarge), "OccurrenceTooLarge")
            ),
            "{expected}: {refused:?}"
        );
        // Nothing minted, committed or recorded.
        assert_eq!(driver.issuer().minted, 0);
        assert_eq!(driver.record_count(), 0);
        assert_eq!(state_of(&runtimes, WALL_A)?, at(WALL_SEALED, 0));
    }

    // The deepest admitted forward (one below the revert bound) still schedules.
    let mut runtimes = walls(&content, &[WALL_A])?;
    let mut driver = driver(&clock, RevertDriverLimits::registered())?;
    let deepest = nested_forward(MAX_REVERT_OCCURRENCE_DEPTH - 1)?;
    let admitted = forward_with(&mut driver, &mut runtimes, &deepest, &revisions()?)??;
    assert_eq!(
        scheduled(admitted)?.ancestry_depth(),
        MAX_REVERT_OCCURRENCE_DEPTH
    );
    Ok(())
}

#[test]
fn use_on_the_open_duke_teleporter_cannot_select_its_revert_and_an_early_event_revert_is_stale()
-> TestResult {
    // Owner decision D91 gap regression: the open teleporter's only edge from its current
    // state is the event-owned `/revert` inverse, so USE selects nothing and a session
    // command naming it is refused; nothing mutates. §7's one path for an intervening
    // change stays defensive: an early event-owned scope revert makes the timer reach
    // `prepare` and terminalize STALE_STATE; it never reverts a second time.
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
    let clock = ManualClock::new(Moment::ZERO);
    let mut driver = driver(&clock, RevertDriverLimits::registered())?;
    let id = scheduled(forward(
        &mut driver,
        &mut runtimes,
        anchor.as_str(),
        DUKE_DEATH,
        DUKE_ACTION,
        OPEN_TELEPORTER,
    )?)?;
    let runtime = runtimes.get_mut(&anchor).ok_or(fixture("runtime"))?;
    let post_revert = format!("{DUKE_ACTION}/post-revert");
    assert_eq!(
        runtime.attempt_use(1, &BTreeSet::new())?,
        LocalObjectUseOutcome::NothingToUse
    );
    let mut ingress = CommandIngress::new();
    assert!(matches!(
        apply_session(runtime, &mut ingress, 1, revert.as_str())?,
        Err(WorldRuntimeError::InvalidBinding(
            "command names a transition this local-object runtime does not bind"
        ))
    ));
    assert_eq!(ingress.outstanding(), 0);
    assert_eq!(state_of(&runtimes, anchor.as_str())?, at(OPEN_ITEM, 1));

    let runtime = runtimes.get_mut(&anchor).ok_or(fixture("runtime"))?;
    assert_eq!(apply_plain(runtime, revert.as_str())?, "COMMITTED");
    assert_eq!(state_of(&runtimes, anchor.as_str())?, at(&post_revert, 2));
    clock.advance(millis(1_200_000))?;
    let report = driver.wake(&mut runtimes, &BTreeSet::new())?;
    let [(fired, outcome)] = report.fired.as_slice() else {
        return Err("expected one fired revert".into());
    };
    assert_eq!(fired, &id);
    assert_eq!(disposition(outcome), Some("STALE_STATE"));
    assert_eq!(state_of(&runtimes, anchor.as_str())?, at(&post_revert, 2));
    let runtime = runtimes.get(&anchor).ok_or(fixture("runtime"))?;
    assert_eq!(
        destination_of(runtime),
        Some(duke_destination("warzone_exit")?.as_str())
    );
    Ok(())
}

#[test]
fn scope_operations_commit_only_their_own_owners_event_edges() -> TestResult {
    // Owner decision D91 (#1187 P1): the executing event names its owner; another owner's
    // forward or inverse is refused before any ordinal, record or mutation.
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
    let clock = ManualClock::new(Moment::ZERO);
    let mut driver = driver(&clock, RevertDriverLimits::registered())?;
    let foreign = TransitionEventOwner::new("oteryn:encounter/another-boss")?;

    // A foreign-owner forward.
    let runtime = runtimes.get_mut(&anchor).ok_or(fixture("runtime"))?;
    let foreign_forward = operation_owned(runtime, OPEN_TELEPORTER, 0, &foreign)?;
    let refused = driver.apply_forward(
        runtime,
        &forward_child(DUKE_DEATH, None)?,
        &revisions()?,
        &LoweredActionId::new(DUKE_ACTION)?,
        &foreign_forward,
        &BTreeSet::new(),
    );
    assert!(matches!(
        refused,
        Err(RevertError::Runtime(WorldRuntimeError::EventOwnerMismatch))
    ));
    assert!(matches!(
        runtime.apply_scope_operation(
            scope()?,
            generation(1)?,
            &foreign_forward,
            &BTreeSet::new(),
            None,
            |_| Ok::<(), Infallible>(()),
        ),
        Err(WorldRuntimeError::EventOwnerMismatch)
    ));
    assert_eq!(driver.issuer().minted, 0);
    assert_eq!(driver.record_count(), 0);
    assert_eq!(state_of(&runtimes, anchor.as_str())?, at(SEALED_ITEM, 0));

    // The correct owner commits the forward and schedules its revert.
    let id = scheduled(forward(
        &mut driver,
        &mut runtimes,
        anchor.as_str(),
        DUKE_DEATH,
        DUKE_ACTION,
        OPEN_TELEPORTER,
    )?)?;
    assert_eq!(state_of(&runtimes, anchor.as_str())?, at(OPEN_ITEM, 1));

    // A foreign-owner inverse.
    let runtime = runtimes.get_mut(&anchor).ok_or(fixture("runtime"))?;
    let foreign_inverse = operation_owned(runtime, revert.as_str(), 1, &foreign)?;
    assert!(matches!(
        runtime.apply_scope_operation(
            scope()?,
            generation(1)?,
            &foreign_inverse,
            &BTreeSet::new(),
            None,
            |_| Ok::<(), Infallible>(()),
        ),
        Err(WorldRuntimeError::EventOwnerMismatch)
    ));
    assert_eq!(state_of(&runtimes, anchor.as_str())?, at(OPEN_ITEM, 1));

    // The driver fires the inverse on the forward's own owner, and it commits.
    clock.advance(millis(1_200_000))?;
    let report = driver.wake(&mut runtimes, &BTreeSet::new())?;
    let [(fired, outcome)] = report.fired.as_slice() else {
        return Err("expected one fired revert".into());
    };
    assert_eq!(fired, &id);
    assert_eq!(disposition(outcome), Some("COMMITTED"));
    let post_revert = format!("{DUKE_ACTION}/post-revert");
    assert_eq!(state_of(&runtimes, anchor.as_str())?, at(&post_revert, 2));
    Ok(())
}
