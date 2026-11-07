// §10.4 (OD9): the pre-authored `CREATE` teleporters.
const OD9_SAMPLES: [(&str, &str, &str); 2] = [
    (
        "canary:encounter/death_priest_shargon",
        "shargon_exit",
        include_str!(
            "../../../../tools/content-schema/encounter-authoring/samples/death_priest_shargon/encounter.json"
        ),
    ),
    (
        "canary:encounter/the_ravager",
        "ravager_exit",
        include_str!(
            "../../../../tools/content-schema/encounter-authoring/samples/the_ravager/encounter.json"
        ),
    ),
];

/// One OD9 sample bound at its anchor from the lowered tables, as `od9_bound` returns it.
struct Od9 {
    content: CanonicalReferencePlayableContent,
    runtimes: BTreeMap<PlacementKey, LocalObjectRuntime>,
    anchor: PlacementKey,
    action: String,
    absent: String,
    create: String,
    remove: String,
}

/// One OD9 sample lowered, linked and bound at its anchor from the lowered absent start, with
/// its destination marker injected.
fn od9_bound(key: &str, encounter: &str) -> TestResult<Od9> {
    let mut source = source_with(
        vec![state(SEALED_ITEM, LocalObjectCollisionPresence::Absent)?],
        vec![],
        "lock:od9-r1",
    )?;
    let anchors = BTreeMap::from([("exit_teleporter".to_owned(), object_definition()?)]);
    let lowered = lower_map_item_transforms(encounter, &source, &anchors)?;
    lowered.apply_to_source(&mut source)?;
    let mut content = link_reference_playable(source)?;
    let anchor = anchor_placement_key(key, "exit_teleporter")?;
    let tables = lowered
        .placements
        .get(&anchor)
        .ok_or(fixture("lowered anchor tables"))?;
    let mut placement = placement_at(&content, anchor.clone(), object_definition()?, 100)?;
    placement.local_object_initial_state = tables.initial_state.clone();
    placement.local_object_state_attributes = tables.state_attributes.clone();
    placement.local_object_revert_after_ms = tables.revert_after_ms.clone();
    placement.local_object_event_transitions = tables.event_transitions.clone();
    let mut placements = vec![placement];
    for (x, marker) in (200..).zip(&lowered.destination_markers) {
        placements.push(placement_at(
            &content,
            marker.clone(),
            marker_definition()?,
            x,
        )?);
    }
    content.placements = placements;
    let rule = key.rsplit('/').next().ok_or(fixture("encounter key"))?;
    let action = format!("{key}/{rule}_death/0");
    let (create, remove) = (format!("{action}/create"), format!("{action}/remove"));
    let runtime = bind_at(&content, &anchor, &[&create, &remove], 1)?;
    Ok(Od9 {
        content,
        runtimes: BTreeMap::from([(anchor.clone(), runtime)]),
        anchor,
        absent: format!("{action}/absent"),
        action,
        create,
        remove,
    })
}

/// What one boss death did to the teleporter.
#[derive(Debug)]
enum Od9Kill {
    /// The create committed (absent -> present) and scheduled its revert.
    Opened(ForwardOutcome),
    /// Owner decision Q2=b: the teleporter was open; its revert was re-armed.
    Rearmed(ChildOccurrenceRef),
}

/// One boss death as the encounter owner delivers it: from absent, select and commit the
/// create through the driver; while open, re-arm the pending revert (owner decision Q2=b).
fn od9_kill(
    driver: &mut ScopeRevertDriver<TestIssuer>,
    od9: &mut Od9,
    owner: &str,
    death: &str,
) -> TestResult<Option<Od9Kill>> {
    let runtime = od9
        .runtimes
        .get_mut(&od9.anchor)
        .ok_or(fixture("runtime"))?;
    let action = LoweredActionId::new(&od9.action)?;
    let owner = TransitionEventOwner::new(owner)?;
    if let Some(transition) = select_timed_forward(runtime, &action, &od9.content.transitions)? {
        let operation = operation_owned(runtime, transition.as_str(), runtime.revision(), &owner)?;
        return Ok(Some(Od9Kill::Opened(driver.apply_forward(
            runtime,
            &forward_child(death, None)?,
            &revisions()?,
            &action,
            &operation,
            &BTreeSet::new(),
        )?)));
    }
    let Some(create) = select_open_create(runtime, &action, &od9.content.transitions)? else {
        return Ok(None);
    };
    Ok(Some(Od9Kill::Rearmed(
        driver.rearm_open_create(runtime, &action, &create, &owner)?,
    )))
}

fn opened(kill: Option<Od9Kill>) -> TestResult<ChildOccurrenceRef> {
    match kill {
        Some(Od9Kill::Opened(forward)) => {
            assert_eq!(forward.outcome.disposition(), "COMMITTED");
            Ok(scheduled(forward)?)
        }
        other => Err(format!("expected the create to commit, got {other:?}").into()),
    }
}

/// `(is_absent, destination, blocks nothing)` of the bound teleporter.
fn od9_view(od9: &Od9) -> Option<(bool, Option<String>, bool)> {
    od9.runtimes.get(&od9.anchor).map(|runtime| {
        (
            runtime.is_absent(),
            destination_of(runtime).map(str::to_owned),
            runtime.blocking_cells().is_empty(),
        )
    })
}

#[test]
fn od9_create_teleporter_opens_a_re_kill_re_arms_its_revert_and_it_re_opens_after_closing()
-> TestResult {
    for (key, exit, encounter) in OD9_SAMPLES {
        let mut od9 = od9_bound(key, encounter)?;
        let anchor = od9.anchor.clone();
        let absent = od9.absent.clone();
        let exit = Some(anchor_placement_key(key, exit)?.as_str().to_owned());
        let clock = ManualClock::new(Moment::ZERO);
        let mut driver = driver(&clock, RevertDriverLimits::registered())?;
        let state = |od9: &Od9| state_of(&od9.runtimes, anchor.as_str());

        // Before the first kill: bound, absent, non-colliding, no destination.
        assert_eq!(state(&od9)?, at(&absent, 0));
        assert_eq!(od9_view(&od9), Some((true, None, true)));

        // Kill 1 at t=0: create commits absent -> present with the destination, one record.
        let first = opened(od9_kill(
            &mut driver,
            &mut od9,
            key,
            &format!("{key}/death/1"),
        )?)?;
        assert_eq!(state(&od9)?, at(SEALED_ITEM, 1));
        assert_eq!(od9_view(&od9), Some((false, exit.clone(), true)));
        let Some(RevertLifecycle::Pending(pending)) = driver.lifecycle(&first) else {
            return Err("revert record is not PENDING".into());
        };
        assert_eq!(pending.inverse().as_str(), od9.remove);
        assert_eq!(pending.expected_state().as_str(), SEALED_ITEM);
        assert_eq!(pending.expected_revision(), 1);

        // Re-kill at t=4 min while open (owner decision Q2=b): the same record is re-armed
        // to the full 5 min from now. One ordinal for the accepted input; no second record,
        // no second object and no mutation of the teleporter.
        clock.advance(millis(240_000))?;
        let minted = driver.issuer().minted;
        let deadline = driver.next_deadline();
        let Some(Od9Kill::Rearmed(rearmed)) =
            od9_kill(&mut driver, &mut od9, key, &format!("{key}/death/open"))?
        else {
            return Err("a kill while open did not re-arm the revert".into());
        };
        assert_eq!(rearmed, first);
        assert_eq!(driver.issuer().minted, minted + 1);
        assert_eq!(driver.record_count(), 1);
        assert_ne!(driver.next_deadline(), deadline);
        assert_eq!(state(&od9)?, at(SEALED_ITEM, 1));
        let Some(RevertLifecycle::Pending(pending)) = driver.lifecycle(&first) else {
            return Err("re-armed record is not PENDING".into());
        };
        assert_eq!(pending.expected_revision(), 1);

        // At t=5 min it stays open; it closes at t=4+5 min, not at 5 min.
        clock.advance(millis(60_000))?;
        assert!(
            driver
                .wake(&mut od9.runtimes, &BTreeSet::new())?
                .fired
                .is_empty()
        );
        assert_eq!(state(&od9)?, at(SEALED_ITEM, 1));
        clock.advance(millis(239_999))?;
        assert!(
            driver
                .wake(&mut od9.runtimes, &BTreeSet::new())?
                .fired
                .is_empty()
        );
        clock.advance(millis(1))?;
        let report = driver.wake(&mut od9.runtimes, &BTreeSet::new())?;
        let [(fired, first_outcome)] = report.fired.as_slice() else {
            return Err("expected the re-armed revert to fire at 4+5 min".into());
        };
        assert_eq!(fired, &first);
        assert!(first_outcome.committed());
        let first_outcome = first_outcome.clone();
        assert_eq!(state(&od9)?, at(&absent, 2));
        assert_eq!(od9_view(&od9), Some((true, None, true)));

        // A kill after it closed re-opens it normally: the create again from the natural
        // absent state (D90: no re-arm edge), with its own new record.
        let runtime = od9.runtimes.get(&anchor).ok_or(fixture("runtime"))?;
        let selected = select_timed_forward(
            runtime,
            &LoweredActionId::new(&od9.action)?,
            &od9.content.transitions,
        )?;
        assert_eq!(
            selected.as_ref().map(TransitionKey::as_str),
            Some(od9.create.as_str())
        );
        let second = opened(od9_kill(
            &mut driver,
            &mut od9,
            key,
            &format!("{key}/death/2"),
        )?)?;
        assert_ne!(second, first);
        assert_eq!(state(&od9)?, at(SEALED_ITEM, 3));
        assert_eq!(od9_view(&od9), Some((false, exit.clone(), true)));
        clock.advance(millis(299_999))?;
        assert!(
            driver
                .wake(&mut od9.runtimes, &BTreeSet::new())?
                .fired
                .is_empty()
        );
        clock.advance(millis(1))?;
        let report = driver.wake(&mut od9.runtimes, &BTreeSet::new())?;
        let [(fired, second_outcome)] = report.fired.as_slice() else {
            return Err("expected the second revert to fire".into());
        };
        assert_eq!(fired, &second);
        assert!(second_outcome.committed());
        let second_outcome = second_outcome.clone();
        assert_eq!(state(&od9)?, at(&absent, 4));

        // Two distinct TERMINAL records, one per opening kill.
        assert_eq!(driver.record_count(), 2);
        assert_eq!(
            driver.lifecycle(&first),
            Some(&RevertLifecycle::Terminal(first_outcome))
        );
        assert_eq!(
            driver.lifecycle(&second),
            Some(&RevertLifecycle::Terminal(second_outcome))
        );
        assert_eq!(driver.next_deadline(), None);
    }
    Ok(())
}

#[test]
fn od9_create_edges_refuse_use_session_and_foreign_owners() -> TestResult {
    let not_bound = "command names a transition this local-object runtime does not bind";
    for (key, _, encounter) in OD9_SAMPLES {
        let mut od9 = od9_bound(key, encounter)?;
        let anchor = od9.anchor.clone();
        let absent = od9.absent.clone();
        let clock = ManualClock::new(Moment::ZERO);
        let mut driver = driver(&clock, RevertDriverLimits::registered())?;
        let foreign = TransitionEventOwner::new(DUKE_KEY)?;
        let mut ingress = CommandIngress::new();

        // Absent: USE selects nothing, and a session command naming create is refused.
        let runtime = od9.runtimes.get_mut(&anchor).ok_or(fixture("runtime"))?;
        assert_eq!(
            runtime.attempt_use(0, &BTreeSet::new())?,
            LocalObjectUseOutcome::NothingToUse
        );
        assert!(matches!(
            apply_session(runtime, &mut ingress, 1, &od9.create)?,
            Err(WorldRuntimeError::InvalidBinding(reason)) if reason == not_bound
        ));

        // A foreign owner's create is refused before any ordinal, record or mutation.
        let foreign_create = operation_owned(runtime, &od9.create, 0, &foreign)?;
        assert!(matches!(
            driver.apply_forward(
                runtime,
                &forward_child(&format!("{key}/death/1"), None)?,
                &revisions()?,
                &LoweredActionId::new(&od9.action)?,
                &foreign_create,
                &BTreeSet::new(),
            ),
            Err(RevertError::Runtime(WorldRuntimeError::EventOwnerMismatch))
        ));
        assert_eq!(driver.issuer().minted, 0);
        assert_eq!(driver.record_count(), 0);
        assert_eq!(state_of(&od9.runtimes, anchor.as_str())?, at(&absent, 0));

        // Open: USE cannot select the remove edge, a session command naming it is refused,
        // and a foreign owner's remove is refused; nothing mutates.
        let id = opened(od9_kill(
            &mut driver,
            &mut od9,
            key,
            &format!("{key}/death/1"),
        )?)?;
        let runtime = od9.runtimes.get_mut(&anchor).ok_or(fixture("runtime"))?;
        // Q2=b re-arm: another owner's re-kill is refused (D91) before any ordinal, and the
        // deadline is unchanged.
        let (minted, deadline) = (driver.issuer().minted, driver.next_deadline());
        let create = TransitionKey::new(&od9.create)?;
        let action = LoweredActionId::new(&od9.action)?;
        assert!(matches!(
            driver.rearm_open_create(runtime, &action, &create, &foreign),
            Err(RevertError::Runtime(WorldRuntimeError::EventOwnerMismatch))
        ));
        assert_eq!(driver.issuer().minted, minted);
        assert_eq!(driver.next_deadline(), deadline);
        assert_eq!(
            runtime.attempt_use(1, &BTreeSet::new())?,
            LocalObjectUseOutcome::NothingToUse
        );
        assert!(matches!(
            apply_session(runtime, &mut ingress, 2, &od9.remove)?,
            Err(WorldRuntimeError::InvalidBinding(reason)) if reason == not_bound
        ));
        assert_eq!(ingress.outstanding(), 0);
        let foreign_remove = operation_owned(runtime, &od9.remove, 1, &foreign)?;
        assert!(matches!(
            runtime.apply_scope_operation(
                scope()?,
                generation(1)?,
                &foreign_remove,
                &BTreeSet::new(),
                None,
                |_| Ok::<(), Infallible>(()),
            ),
            Err(WorldRuntimeError::EventOwnerMismatch)
        ));
        assert_eq!(
            state_of(&od9.runtimes, anchor.as_str())?,
            at(SEALED_ITEM, 1)
        );

        // The driver still reverts on the create's own owner.
        clock.advance(millis(300_000))?;
        let report = driver.wake(&mut od9.runtimes, &BTreeSet::new())?;
        let [(fired, outcome)] = report.fired.as_slice() else {
            return Err("expected one fired revert".into());
        };
        assert_eq!(fired, &id);
        assert!(outcome.committed());
        assert_eq!(state_of(&od9.runtimes, anchor.as_str())?, at(&absent, 2));
    }
    Ok(())
}
