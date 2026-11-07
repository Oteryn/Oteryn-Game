// USE-WIRE-V1 selection kernel and `attempt_use` (#162 5868482467, M2b). SEAM_EVIDENCE:
// world_runtime.rs use-selection kernel unit coverage.

#[test]
fn use_selection_kernel_fails_closed_on_an_ambiguous_current_state() -> Result<(), WorldRuntimeError>
{
    let content = world_object_overlay_content("wo-r1")?;
    let (_authority, _session, scope) = authority(90, 18, 1, 1)?;
    let mut runtime =
        world_object_runtime_for(&content, scope, 1, &world_object_all_transitions()?)?;
    let empty = BTreeSet::new();

    // "absent" binds exactly one outgoing transition (CREATE): a unique candidate commits.
    assert_eq!(
        runtime.attempt_use(0, &empty)?,
        LocalObjectUseOutcome::Committed {
            state: ProductionKey::new("oteryn:reference.state.world-object-present")?,
            revision: 1,
        }
    );

    // "present" binds two outgoing transitions (REMOVE, RETAG): ambiguous fails closed
    // before any state read that would leak which one a real selection would have picked,
    // and makes no mutation.
    assert_eq!(
        runtime.attempt_use(1, &empty)?,
        LocalObjectUseOutcome::Rejected
    );
    assert_eq!(runtime.revision(), 1);
    assert_eq!(
        runtime.state_key().as_str(),
        "oteryn:reference.state.world-object-present"
    );
    Ok(())
}

#[test]
fn use_selection_kernel_fails_closed_with_no_candidate_from_a_terminal_state()
-> Result<(), WorldRuntimeError> {
    let mut content = world_object_overlay_content("wo-r1")?;
    let placement = content
        .placements
        .iter_mut()
        .find(|placement| placement.key.as_str() == WORLD_OBJECT_PLACEMENT)
        .ok_or(fixture_error("world object placement"))?;
    placement.local_object_initial_state = Some(ProductionKey::new(
        "oteryn:reference.state.world-object-dormant",
    )?);
    let (_authority, _session, scope) = authority(91, 19, 1, 1)?;
    let mut runtime =
        world_object_runtime_for(&content, scope, 1, &world_object_all_transitions()?)?;
    let empty = BTreeSet::new();

    // "dormant" binds no outgoing transition among the four bound here: zero candidates
    // fails closed as NothingToUse, and makes no mutation.
    assert_eq!(
        runtime.attempt_use(0, &empty)?,
        LocalObjectUseOutcome::NothingToUse
    );
    assert_eq!(runtime.revision(), 0);
    assert_eq!(
        runtime.state_key().as_str(),
        "oteryn:reference.state.world-object-dormant"
    );
    Ok(())
}

#[test]
fn stale_revision_wins_over_an_ambiguous_current_state() -> Result<(), WorldRuntimeError> {
    let content = world_object_overlay_content("wo-r1")?;
    let (_authority, _session, scope) = authority(93, 21, 1, 1)?;
    let mut runtime =
        world_object_runtime_for(&content, scope, 1, &world_object_all_transitions()?)?;
    let empty = BTreeSet::new();

    // Reach "present" (revision 1), which binds two outgoing transitions (REMOVE, RETAG):
    // selecting from here alone would be Ambiguous/Rejected.
    assert_eq!(
        runtime.attempt_use(0, &empty)?,
        LocalObjectUseOutcome::Committed {
            state: ProductionKey::new("oteryn:reference.state.world-object-present")?,
            revision: 1,
        }
    );

    // Codex 5869579920: a stale `expected_revision` (the caller's last-known revision 0,
    // now behind the current revision 1) must report STALE_STATE, never the Rejected an
    // ambiguous *current* state would otherwise produce, and must not mutate.
    assert_eq!(
        runtime.attempt_use(0, &empty)?,
        LocalObjectUseOutcome::StaleState
    );
    assert_eq!(runtime.revision(), 1);
    assert_eq!(
        runtime.state_key().as_str(),
        "oteryn:reference.state.world-object-present"
    );
    Ok(())
}

#[test]
fn stale_revision_wins_over_a_terminal_current_state() -> Result<(), WorldRuntimeError> {
    let mut content = world_object_overlay_content("wo-r1")?;
    let placement = content
        .placements
        .iter_mut()
        .find(|placement| placement.key.as_str() == WORLD_OBJECT_PLACEMENT)
        .ok_or(fixture_error("world object placement"))?;
    placement.local_object_initial_state = Some(ProductionKey::new(
        "oteryn:reference.state.world-object-dormant",
    )?);
    let (_authority, _session, scope) = authority(94, 22, 1, 1)?;
    let mut runtime =
        world_object_runtime_for(&content, scope, 1, &world_object_all_transitions()?)?;
    let empty = BTreeSet::new();

    // Codex 5869579920: "dormant" binds no outgoing transition (would select NothingToUse),
    // but a stale `expected_revision` (7, never reached: the runtime starts and stays at 0)
    // must still report STALE_STATE, never NothingToUse, and must not mutate.
    assert_eq!(
        runtime.attempt_use(7, &empty)?,
        LocalObjectUseOutcome::StaleState
    );
    assert_eq!(runtime.revision(), 0);
    assert_eq!(
        runtime.state_key().as_str(),
        "oteryn:reference.state.world-object-dormant"
    );
    Ok(())
}

#[test]
fn attempt_use_commits_the_unique_transition_and_reports_stale_and_occupied()
-> Result<(), WorldRuntimeError> {
    let content = synthetic_content("package-r1")?;
    let (_authority, _session, scope) = authority(92, 20, 1, 1)?;
    let mut runtime = runtime_for(&content, scope, PLACEMENT_A, 1)?;
    let empty = BTreeSet::new();

    // Stale: `expected_revision` does not match the runtime's current revision (0).
    assert_eq!(
        runtime.attempt_use(7, &empty)?,
        LocalObjectUseOutcome::StaleState
    );
    assert_eq!(runtime.revision(), 0);

    // Closed -> open: the target state's collision is Absent, so this commits regardless
    // of `occupied_cells`.
    assert_eq!(
        runtime.attempt_use(0, &empty)?,
        LocalObjectUseOutcome::Committed {
            state: ProductionKey::new("oteryn:reference.state.open")?,
            revision: 1,
        }
    );
    assert!(runtime.blocking_cells().is_empty());

    // Open -> closed: the target state's collision is Present, so a currently occupied
    // footprint refuses the transition and makes no mutation.
    let occupied = runtime.collision_cells().clone();
    assert_eq!(
        runtime.attempt_use(1, &occupied)?,
        LocalObjectUseOutcome::Occupied
    );
    assert_eq!(runtime.revision(), 1);
    assert_eq!(runtime.state_key().as_str(), "oteryn:reference.state.open");

    // The same close now succeeds once the footprint is clear.
    assert_eq!(
        runtime.attempt_use(1, &empty)?,
        LocalObjectUseOutcome::Committed {
            state: ProductionKey::new("oteryn:reference.state.closed")?,
            revision: 2,
        }
    );
    assert_eq!(runtime.blocking_cells(), runtime.collision_cells());
    Ok(())
}

// Owner decision D91: the native entry door names no event origin, so both of its edges stay
// `PlayerUse`: USE opens and closes it as before, and a scope operation cannot run either.
#[test]
fn native_entry_door_edges_stay_player_use_and_scope_operations_refuse_them()
-> Result<(), Box<dyn Error>> {
    use crate::content::accepted as door_accepted;
    let (_authority, _session, scope) = authority(93, 20, 1, 1)?;
    let room = crate::content::qualify_native_entry_room(scope.world_id())
        .map_err(|error| format!("{error:?}"))?;
    let door_content = room.door().clone();
    let generation = ScopeOwnershipGeneration::new(1).map_err(|error| format!("{error:?}"))?;
    let fence = ScopeContentGenerationFence::for_activation(
        scope,
        generation,
        ReferenceContentGeneration::from_content(&door_content)?,
    );
    let mut door = bind_native_entry_door(&door_content, &fence, scope, generation)?;
    let open = TransitionKey::new(door_accepted::DOOR_OPEN_TRANSITION)?;
    let close = TransitionKey::new(door_accepted::DOOR_CLOSE_TRANSITION)?;
    assert_eq!(door.transition_origin(&open), TransitionOrigin::PlayerUse);
    assert_eq!(door.transition_origin(&close), TransitionOrigin::PlayerUse);

    let scope_open = ScopeLocalObjectOperation::new(
        door.placement_key().clone(),
        door.incarnation(),
        door.content_generation().clone(),
        LocalObjectOperation::new(open),
        door.revision(),
        TransitionEventOwner::new("oteryn:encounter/any-event")?,
    );
    assert!(matches!(
        door.apply_scope_operation(
            scope,
            generation,
            &scope_open,
            &BTreeSet::new(),
            None,
            |_| Ok::<(), std::convert::Infallible>(()),
        ),
        Err(WorldRuntimeError::InvalidBinding(
            "player-use transition is not a scope operation"
        ))
    ));
    assert_eq!(door.revision(), 0);

    let empty = BTreeSet::new();
    assert!(matches!(
        door.attempt_use(0, &empty)?,
        LocalObjectUseOutcome::Committed { revision: 1, .. }
    ));
    assert!(matches!(
        door.attempt_use(1, &empty)?,
        LocalObjectUseOutcome::Committed { revision: 2, .. }
    ));
    Ok(())
}
