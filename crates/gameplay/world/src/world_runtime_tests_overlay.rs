fn world_object_runtime_for(
    content: &CanonicalReferencePlayableContent,
    scope: RuntimeScopeRefV1,
    scope_generation: u64,
    transition_keys: &[TransitionKey],
) -> Result<LocalObjectRuntime, WorldRuntimeError> {
    let scope_generation = ScopeOwnershipGeneration::new(scope_generation)
        .map_err(|_error: GenerationError| fixture_error("scope generation"))?;
    let active_content = ScopeContentGenerationFence::for_test(
        scope,
        scope_generation,
        ReferenceContentGeneration::from_content(content)?,
    );
    LocalObjectRuntime::bind(
        content,
        &active_content,
        scope,
        scope_generation,
        &PlacementKey::new(WORLD_OBJECT_PLACEMENT)?,
        1,
        transition_keys,
    )
}

fn world_object_all_transitions() -> Result<Vec<TransitionKey>, WorldRuntimeError> {
    Ok(vec![
        TransitionKey::new(CREATE_TRANSITION)?,
        TransitionKey::new(REMOVE_TRANSITION)?,
        TransitionKey::new(RETAG_TRANSITION)?,
        TransitionKey::new(TRANSFORM_TRANSITION)?,
    ])
}

#[test]
fn create_retag_and_transform_drive_blocking_from_the_target_states_own_collision()
-> Result<(), WorldRuntimeError> {
    let content = world_object_overlay_content("wo-r1")?;
    let (authority, session, scope) = authority(80, 12, 1, 1)?;
    let mut runtime =
        world_object_runtime_for(&content, scope, 1, &world_object_all_transitions()?)?;
    let mut ingress = CommandIngress::new();
    let empty = BTreeSet::new();

    assert_eq!(
        runtime.state_key().as_str(),
        "oteryn:reference.state.world-object-absent"
    );
    assert!(runtime.blocking_cells().is_empty());

    // CREATE: absent -> present, an anchor materializing into its fixed, pre-authored
    // footprint (no dynamic geometry: `collision_cells` was reserved at bind time).
    let create = command(
        &runtime,
        session,
        1,
        1,
        LocalObjectOperation::new(TransitionKey::new(CREATE_TRANSITION)?),
        0,
    )?;
    let created = runtime.apply(&authority, &create, &mut ingress, &empty)?;
    assert_eq!(created.disposition(), DISPOSITION_COMMITTED);
    assert_eq!(
        created.state(),
        "oteryn:reference.state.world-object-present"
    );
    assert_eq!(runtime.blocking_cells(), runtime.collision_cells());

    // RETAG: present -> armed, same collision class (an interaction-binding rearm only);
    // blocking is unaffected by the identity change.
    let retag = command(
        &runtime,
        session,
        2,
        1,
        LocalObjectOperation::new(TransitionKey::new(RETAG_TRANSITION)?),
        1,
    )?;
    let retagged = runtime.apply(&authority, &retag, &mut ingress, &empty)?;
    assert_eq!(retagged.disposition(), DISPOSITION_COMMITTED);
    assert_eq!(
        retagged.state(),
        "oteryn:reference.state.world-object-armed"
    );
    assert_eq!(runtime.blocking_cells(), runtime.collision_cells());

    // TRANSFORM: armed -> dormant, an arbitrary from/to move outside the CREATE/REMOVE/RETAG
    // naming, landing on a collision-Absent state; this is also the >2-state case the fixed
    // Open/Close pair could not represent.
    let transform = command(
        &runtime,
        session,
        3,
        1,
        LocalObjectOperation::new(TransitionKey::new(TRANSFORM_TRANSITION)?),
        2,
    )?;
    let transformed = runtime.apply(&authority, &transform, &mut ingress, &empty)?;
    assert_eq!(transformed.disposition(), DISPOSITION_COMMITTED);
    assert_eq!(
        transformed.state(),
        "oteryn:reference.state.world-object-dormant"
    );
    assert!(runtime.blocking_cells().is_empty());
    Ok(())
}

#[test]
fn create_then_remove_round_trips_and_replay_of_create_does_not_reexecute()
-> Result<(), WorldRuntimeError> {
    let content = world_object_overlay_content("wo-r1")?;
    let (authority_a, session_a, scope) = authority(81, 13, 1, 1)?;
    let (authority_b, session_b, _) = authority(82, 13, 1, 1)?;
    let mut runtime = world_object_runtime_for(
        &content,
        scope,
        1,
        &[
            TransitionKey::new(CREATE_TRANSITION)?,
            TransitionKey::new(REMOVE_TRANSITION)?,
        ],
    )?;
    let mut ingress_a = CommandIngress::new();
    let mut ingress_b = CommandIngress::new();
    let empty = BTreeSet::new();

    let create = command(
        &runtime,
        session_a,
        1,
        1,
        LocalObjectOperation::new(TransitionKey::new(CREATE_TRANSITION)?),
        0,
    )?;
    let created = runtime.apply(&authority_a, &create, &mut ingress_a, &empty)?;
    assert_eq!(created.disposition(), DISPOSITION_COMMITTED);
    assert_eq!(created.revision(), 1);
    assert_eq!(runtime.blocking_cells(), runtime.collision_cells());

    let remove = command(
        &runtime,
        session_b,
        1,
        1,
        LocalObjectOperation::new(TransitionKey::new(REMOVE_TRANSITION)?),
        1,
    )?;
    let removed = runtime.apply(&authority_b, &remove, &mut ingress_b, &empty)?;
    assert_eq!(removed.disposition(), DISPOSITION_COMMITTED);
    assert_eq!(removed.revision(), 2);
    assert!(runtime.blocking_cells().is_empty());

    let replay = runtime.apply(&authority_a, &create, &mut ingress_a, &empty)?;
    assert!(replay.replayed());
    assert_eq!(replay.disposition(), DISPOSITION_COMMITTED);
    assert_eq!(
        replay.state(),
        "oteryn:reference.state.world-object-present"
    );
    assert_eq!(replay.revision(), 1);
    assert_eq!(
        runtime.state_key().as_str(),
        "oteryn:reference.state.world-object-absent"
    );
    assert_eq!(runtime.revision(), 2);
    Ok(())
}

#[test]
fn create_onto_an_occupied_footprint_is_rejected_and_leaves_no_partial_state()
-> Result<(), WorldRuntimeError> {
    let content = world_object_overlay_content("wo-r1")?;
    let (authority, session, scope) = authority(83, 14, 1, 1)?;
    let mut runtime = world_object_runtime_for(
        &content,
        scope,
        1,
        &[TransitionKey::new(CREATE_TRANSITION)?],
    )?;
    let mut ingress = CommandIngress::new();
    let occupied_cell = runtime.collision_cells().iter().next().copied().ok_or(
        WorldRuntimeError::InvalidBinding("test fixture has no collision cell"),
    )?;
    let occupied = BTreeSet::from([occupied_cell]);

    let create = command(
        &runtime,
        session,
        1,
        1,
        LocalObjectOperation::new(TransitionKey::new(CREATE_TRANSITION)?),
        0,
    )?;
    let rejected = runtime.apply(&authority, &create, &mut ingress, &occupied)?;
    assert_eq!(rejected.disposition(), DISPOSITION_OCCUPIED);
    assert_eq!(
        runtime.state_key().as_str(),
        "oteryn:reference.state.world-object-absent"
    );
    assert_eq!(runtime.revision(), 0);
    assert!(runtime.blocking_cells().is_empty());

    let create_after_clear = command(
        &runtime,
        session,
        2,
        1,
        LocalObjectOperation::new(TransitionKey::new(CREATE_TRANSITION)?),
        0,
    )?;
    let committed = runtime.apply(
        &authority,
        &create_after_clear,
        &mut ingress,
        &BTreeSet::new(),
    )?;
    assert_eq!(committed.disposition(), DISPOSITION_COMMITTED);
    assert_eq!(runtime.blocking_cells(), runtime.collision_cells());
    Ok(())
}

#[test]
fn retag_across_collision_classes_is_rejected_by_the_content_layer_before_runtime_creation()
-> Result<(), WorldRuntimeError> {
    let mut content = world_object_overlay_content("wo-r1")?;
    let ReferenceDefinitionKind::LocalObjectStates(states) = &mut content.definitions[0].kind
    else {
        return Err(fixture_error("world-object anchor definition"));
    };
    for state in states.iter_mut() {
        if state.key.as_str() == "oteryn:reference.state.world-object-armed" {
            state.collision = LocalObjectCollisionPresence::Absent;
        }
    }

    let (_authority, _session, scope) = authority(84, 15, 1, 1)?;
    assert!(matches!(
        world_object_runtime_for(&content, scope, 1, &world_object_all_transitions()?),
        Err(WorldRuntimeError::Content(ContentError::InvalidArtifact(
            "reference-playable RETAG transition must preserve local-object collision class"
        )))
    ));
    Ok(())
}

#[test]
fn operation_naming_an_unbound_transition_fails_closed_before_ingress_or_mutation()
-> Result<(), WorldRuntimeError> {
    let content = world_object_overlay_content("wo-r1")?;
    let (authority, session, scope) = authority(85, 16, 1, 1)?;
    let mut runtime = world_object_runtime_for(
        &content,
        scope,
        1,
        &[TransitionKey::new(CREATE_TRANSITION)?],
    )?;
    let mut ingress = CommandIngress::new();
    let unbound = command(
        &runtime,
        session,
        1,
        1,
        LocalObjectOperation::new(TransitionKey::new(REMOVE_TRANSITION)?),
        0,
    )?;
    assert!(matches!(
        runtime.apply(&authority, &unbound, &mut ingress, &BTreeSet::new()),
        Err(WorldRuntimeError::InvalidBinding(
            "command names a transition this local-object runtime does not bind"
        ))
    ));
    assert_eq!(ingress.outstanding(), 0);
    assert_eq!(runtime.revision(), 0);
    assert!(runtime.blocking_cells().is_empty());
    Ok(())
}

#[test]
fn fresh_bind_after_scope_restart_restores_the_authored_initial_state()
-> Result<(), WorldRuntimeError> {
    let content = world_object_overlay_content("wo-r1")?;
    let (authority, session, scope) = authority(86, 17, 1, 1)?;
    let all = world_object_all_transitions()?;
    let mut runtime = world_object_runtime_for(&content, scope, 1, &all)?;
    let mut ingress = CommandIngress::new();
    let create = command(
        &runtime,
        session,
        1,
        1,
        LocalObjectOperation::new(TransitionKey::new(CREATE_TRANSITION)?),
        0,
    )?;
    runtime.apply(&authority, &create, &mut ingress, &BTreeSet::new())?;
    assert_eq!(
        runtime.state_key().as_str(),
        "oteryn:reference.state.world-object-present"
    );
    assert!(!runtime.blocking_cells().is_empty());

    // D38 W2: world-object overlay state is scope-ephemeral. No persistence exists for it, so
    // a scope restart is exactly a fresh `bind()` from the same immutable placements,
    // independent of any earlier runtime instance or its mutations.
    let restarted = world_object_runtime_for(&content, scope, 1, &all)?;
    assert_eq!(
        restarted.state_key().as_str(),
        "oteryn:reference.state.world-object-absent"
    );
    assert_eq!(restarted.revision(), 0);
    assert!(restarted.blocking_cells().is_empty());
    Ok(())
}
