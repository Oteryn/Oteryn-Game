#[test]
fn duke_teleporter_reverts_to_the_warzone_exit_after_its_authored_duration() -> TestResult {
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

    // Natural source state: no destination before the forward operation.
    assert_eq!(state_of(&runtimes, anchor.as_str())?, at(SEALED_ITEM, 0));
    let runtime = runtimes.get(&anchor).ok_or(fixture("runtime"))?;
    assert_eq!(destination_of(runtime), None);

    let forward = forward(
        &mut driver,
        &mut runtimes,
        anchor.as_str(),
        DUKE_DEATH,
        DUKE_ACTION,
        OPEN_TELEPORTER,
    )?;
    assert_eq!(forward.outcome.disposition(), "COMMITTED");
    let id = scheduled(forward)?;
    let runtime = runtimes.get(&anchor).ok_or(fixture("runtime"))?;
    assert_eq!(runtime.state_key().as_str(), OPEN_ITEM);
    assert_eq!(
        destination_of(runtime),
        Some(duke_destination("reward_destination")?.as_str())
    );
    let Some(RevertLifecycle::Pending(pending)) = driver.lifecycle(&id) else {
        return Err("revert record is not PENDING".into());
    };
    assert_eq!(pending.inverse(), &revert);
    assert_eq!(pending.expected_state().as_str(), OPEN_ITEM);
    assert_eq!(pending.expected_revision(), 1);

    clock.advance(millis(1_199_999))?;
    assert!(
        driver
            .wake(&mut runtimes, &BTreeSet::new())?
            .fired
            .is_empty()
    );
    clock.advance(millis(1))?;
    let report = driver.wake(&mut runtimes, &BTreeSet::new())?;
    let [(fired, outcome)] = report.fired.as_slice() else {
        return Err("expected exactly one fired revert".into());
    };
    assert_eq!(fired, &id);
    assert!(outcome.committed());

    // Post-revert variant: same rendered item, own state key, destination = warzone exit.
    let post_revert = format!("{DUKE_ACTION}/post-revert");
    assert_eq!(state_of(&runtimes, anchor.as_str())?, at(&post_revert, 2));
    let runtime = runtimes.get(&anchor).ok_or(fixture("runtime"))?;
    assert_eq!(
        destination_of(runtime),
        Some(duke_destination("warzone_exit")?.as_str())
    );
    assert_eq!(
        driver.lifecycle(&id),
        Some(&RevertLifecycle::Terminal(outcome.clone()))
    );

    // A duplicate presentation returns the TERMINAL outcome: no ordinal, no mutation.
    let minted = driver.issuer().minted;
    assert_eq!(
        driver.present(&id, &mut runtimes, &BTreeSet::new())?,
        RevertPresentation::Terminal(outcome.clone())
    );
    assert_eq!(driver.issuer().minted, minted);
    assert_eq!(state_of(&runtimes, anchor.as_str())?, at(&post_revert, 2));
    Ok(())
}

/// One duke death as the encounter owner delivers it: select the timed forward from the
/// teleporter's current state (D90) and, if there is one, commit it through the driver.
fn duke_kill(
    driver: &mut ScopeRevertDriver<TestIssuer>,
    runtimes: &mut BTreeMap<PlacementKey, LocalObjectRuntime>,
    content: &CanonicalReferencePlayableContent,
    death: &str,
) -> TestResult<Option<ForwardOutcome>> {
    let anchor = duke_anchor()?;
    let runtime = runtimes.get(&anchor).ok_or(fixture("runtime"))?;
    let action = LoweredActionId::new(DUKE_ACTION)?;
    let Some(transition) = select_timed_forward(runtime, &action, &content.transitions)? else {
        return Ok(None);
    };
    Ok(Some(forward(
        driver,
        runtimes,
        anchor.as_str(),
        death,
        DUKE_ACTION,
        transition.as_str(),
    )?))
}

#[test]
fn duke_teleporter_re_arms_on_every_kill_and_a_kill_while_open_is_a_no_op() -> TestResult {
    // Owner decision D90: after the timed revert lands on the post-revert variant, the next
    // kill reopens the teleporter through the lowered re-arm forward.
    let (content, revert) = duke_content(DUKE)?;
    let anchor = duke_anchor()?;
    let rearm = duke_rearm();
    let mut runtimes = BTreeMap::from([(
        anchor.clone(),
        bind_at(
            &content,
            &anchor,
            &[OPEN_TELEPORTER, &rearm, revert.as_str()],
            1,
        )?,
    )]);
    let clock = ManualClock::new(Moment::ZERO);
    let mut driver = driver(&clock, RevertDriverLimits::registered())?;
    let post_revert = format!("{DUKE_ACTION}/post-revert");
    let reward = duke_destination("reward_destination")?;
    let warzone = duke_destination("warzone_exit")?;
    let destination = |runtimes: &BTreeMap<PlacementKey, LocalObjectRuntime>| {
        runtimes
            .get(&anchor)
            .and_then(destination_of)
            .map(str::to_owned)
    };

    // Kill 1 from the natural state: A -> B, reward destination, one PENDING record.
    let kill_1 = duke_kill(&mut driver, &mut runtimes, &content, DUKE_DEATH)?
        .ok_or(fixture("kill 1 selected no forward"))?;
    assert_eq!(kill_1.outcome.disposition(), "COMMITTED");
    let first = scheduled(kill_1)?;
    assert_eq!(state_of(&runtimes, anchor.as_str())?, at(OPEN_ITEM, 1));
    assert_eq!(destination(&runtimes), Some(reward.as_str().to_owned()));

    // A kill while open selects no forward: no ordinal, no record, no mutation.
    let minted = driver.issuer().minted;
    assert!(
        duke_kill(
            &mut driver,
            &mut runtimes,
            &content,
            "canary:occurrence/the_duke_of_the_depths/death/open-1"
        )?
        .is_none()
    );
    assert_eq!(driver.issuer().minted, minted);
    assert_eq!(driver.record_count(), 1);
    assert_eq!(state_of(&runtimes, anchor.as_str())?, at(OPEN_ITEM, 1));

    // 1,200,000 ms later the revert B -> C lands on the warzone exit.
    clock.advance(millis(1_200_000))?;
    let report = driver.wake(&mut runtimes, &BTreeSet::new())?;
    let [(fired, first_outcome)] = report.fired.as_slice() else {
        return Err("expected the first revert to fire".into());
    };
    assert_eq!(fired, &first);
    assert!(first_outcome.committed());
    let first_outcome = first_outcome.clone();
    assert_eq!(state_of(&runtimes, anchor.as_str())?, at(&post_revert, 2));
    assert_eq!(destination(&runtimes), Some(warzone.as_str().to_owned()));

    // Kill 2 from the post-revert variant: the re-arm forward C -> B reopens the teleporter
    // to the reward destination and schedules a second, distinct revert of the same inverse.
    let runtime = runtimes.get(&anchor).ok_or(fixture("runtime"))?;
    let selected = select_timed_forward(
        runtime,
        &LoweredActionId::new(DUKE_ACTION)?,
        &content.transitions,
    )?;
    assert_eq!(
        selected.as_ref().map(TransitionKey::as_str),
        Some(rearm.as_str())
    );
    let kill_2 = duke_kill(
        &mut driver,
        &mut runtimes,
        &content,
        "canary:occurrence/the_duke_of_the_depths/death/2",
    )?
    .ok_or(fixture("kill 2 selected no forward"))?;
    assert_eq!(kill_2.outcome.disposition(), "COMMITTED");
    let second = scheduled(kill_2)?;
    assert_ne!(second, first);
    assert_eq!(state_of(&runtimes, anchor.as_str())?, at(OPEN_ITEM, 3));
    assert_eq!(destination(&runtimes), Some(reward.as_str().to_owned()));
    let Some(RevertLifecycle::Pending(pending)) = driver.lifecycle(&second) else {
        return Err("second revert record is not PENDING".into());
    };
    assert_eq!(pending.inverse(), &revert);
    assert_eq!(pending.expected_state().as_str(), OPEN_ITEM);
    assert_eq!(pending.expected_revision(), 3);

    // A kill while open again is a no-op, and the first revert still stands.
    let minted = driver.issuer().minted;
    assert!(
        duke_kill(
            &mut driver,
            &mut runtimes,
            &content,
            "canary:occurrence/the_duke_of_the_depths/death/open-2"
        )?
        .is_none()
    );
    assert_eq!(driver.issuer().minted, minted);
    assert_eq!(driver.record_count(), 2);
    assert_eq!(state_of(&runtimes, anchor.as_str())?, at(OPEN_ITEM, 3));
    assert_eq!(
        driver.lifecycle(&first),
        Some(&RevertLifecycle::Terminal(first_outcome.clone()))
    );

    // 1,200,000 ms after kill 2 the teleporter reverts to the warzone exit again.
    clock.advance(millis(1_199_999))?;
    assert!(
        driver
            .wake(&mut runtimes, &BTreeSet::new())?
            .fired
            .is_empty()
    );
    clock.advance(millis(1))?;
    let report = driver.wake(&mut runtimes, &BTreeSet::new())?;
    let [(fired, second_outcome)] = report.fired.as_slice() else {
        return Err("expected the second revert to fire".into());
    };
    assert_eq!(fired, &second);
    assert!(second_outcome.committed());
    assert_eq!(state_of(&runtimes, anchor.as_str())?, at(&post_revert, 4));
    assert_eq!(destination(&runtimes), Some(warzone.as_str().to_owned()));

    // Two distinct TERMINAL lifecycle records, one per kill.
    assert_eq!(driver.record_count(), 2);
    assert_eq!(
        driver.lifecycle(&first),
        Some(&RevertLifecycle::Terminal(first_outcome))
    );
    assert_eq!(
        driver.lifecycle(&second),
        Some(&RevertLifecycle::Terminal(second_outcome.clone()))
    );
    assert_eq!(driver.next_deadline(), None);
    Ok(())
}
