#[test]
fn open_close_then_replay_open_preserves_current_closed_state() -> Result<(), WorldRuntimeError> {
    let content = synthetic_content("package-r1")?;
    let (authority_a, session_a, scope) = authority(10, 3, 1, 1)?;
    let (authority_b, session_b, _) = authority(11, 3, 1, 1)?;
    let mut runtime = runtime_for(&content, scope, PLACEMENT_A, 1)?;
    let mut ingress_a = CommandIngress::new();
    let mut ingress_b = CommandIngress::new();
    let empty = BTreeSet::new();

    let open = command(&runtime, session_a, 1, 1, open_operation()?, 0)?;
    let opened = runtime.apply(&authority_a, &open, &mut ingress_a, &empty)?;
    assert_eq!(opened.disposition(), DISPOSITION_COMMITTED);
    assert_eq!(opened.revision(), 1);
    assert!(runtime.blocking_cells().is_empty());

    let close = command(&runtime, session_b, 1, 1, close_operation()?, 1)?;
    let closed = runtime.apply(&authority_b, &close, &mut ingress_b, &empty)?;
    assert_eq!(closed.disposition(), DISPOSITION_COMMITTED);
    assert_eq!(closed.revision(), 2);
    assert_eq!(runtime.blocking_cells(), runtime.collision_cells());

    let (stale_authority_a, _, _) = authority(10, 3, 1, 2)?;
    assert!(matches!(
        runtime.apply(&stale_authority_a, &open, &mut ingress_a, &empty),
        Err(WorldRuntimeError::StaleConnectionGeneration)
    ));
    assert_eq!(runtime.revision(), 2);

    let replay = runtime.apply(&authority_a, &open, &mut ingress_a, &empty)?;
    assert!(replay.replayed());
    assert_eq!(replay.disposition(), DISPOSITION_COMMITTED);
    assert_eq!(replay.state(), "oteryn:reference.state.open");
    assert_eq!(replay.revision(), 1);
    assert_eq!(
        runtime.state_key().as_str(),
        "oteryn:reference.state.closed"
    );
    assert_eq!(runtime.revision(), 2);
    Ok(())
}

#[test]
fn authored_open_initial_state_starts_unblocked_then_close_commits_and_blocks()
-> Result<(), WorldRuntimeError> {
    let mut content = synthetic_content("package-r1")?;
    let placement_a = content
        .placements
        .iter_mut()
        .find(|placement| placement.key.as_str() == PLACEMENT_A)
        .ok_or(fixture_error("placement a"))?;
    placement_a.local_object_initial_state =
        Some(ProductionKey::new("oteryn:reference.state.open")?);

    let (authority, session, scope) = authority(40, 6, 1, 1)?;
    let mut runtime = runtime_for(&content, scope, PLACEMENT_A, 1)?;
    assert_eq!(runtime.state_key().as_str(), "oteryn:reference.state.open");
    assert!(runtime.blocking_cells().is_empty());

    let mut ingress = CommandIngress::new();
    let empty = BTreeSet::new();
    let close = command(&runtime, session, 1, 1, close_operation()?, 0)?;
    let closed = runtime.apply(&authority, &close, &mut ingress, &empty)?;
    assert_eq!(closed.disposition(), DISPOSITION_COMMITTED);
    assert_eq!(closed.state(), "oteryn:reference.state.closed");
    assert_eq!(runtime.blocking_cells(), runtime.collision_cells());
    assert!(!runtime.blocking_cells().is_empty());
    Ok(())
}

#[test]
fn later_command_cannot_commit_ahead_of_earlier_pending_across_objects()
-> Result<(), WorldRuntimeError> {
    let content = synthetic_content("package-r1")?;
    let (authority, session, scope) = authority(20, 4, 1, 1)?;
    let mut runtime_a = runtime_for(&content, scope, PLACEMENT_A, 1)?;
    let mut runtime_b = runtime_for(&content, scope, PLACEMENT_B, 1)?;
    let mut ingress = CommandIngress::new();
    let first = command(&runtime_a, session, 1, 1, open_operation()?, 0)?;
    let first_semantic = runtime_a.command_semantic_identity(&first)?;
    assert_eq!(
        ingress.reserve(first.command_ref().command_id(), first_semantic),
        IngressDecision::Reserved(first.command_ref().command_id())
    );

    let second = command(&runtime_b, session, 2, 1, open_operation()?, 0)?;
    let before = runtime_b.blocking_cells().clone();
    assert!(matches!(
        runtime_b.apply(&authority, &second, &mut ingress, &BTreeSet::new()),
        Err(WorldRuntimeError::CommandLifecycle(
            CommandLifecycleError::TerminalOutOfOrder { .. }
        ))
    ));
    assert_eq!(runtime_b.revision(), 0);
    assert_eq!(runtime_b.blocking_cells(), &before);

    assert!(matches!(
        runtime_b.apply(&authority, &second, &mut ingress, &BTreeSet::new()),
        Err(WorldRuntimeError::PendingOriginal)
    ));
    assert_eq!(runtime_b.revision(), 0);

    let first_result =
        runtime_a.resume_pending(&authority, &first, &mut ingress, &BTreeSet::new())?;
    assert_eq!(first_result.disposition(), DISPOSITION_COMMITTED);
    assert_eq!(runtime_a.revision(), 1);

    let second_result =
        runtime_b.resume_pending(&authority, &second, &mut ingress, &BTreeSet::new())?;
    assert_eq!(second_result.disposition(), DISPOSITION_COMMITTED);
    assert_eq!(runtime_b.revision(), 1);
    assert!(runtime_b.blocking_cells().is_empty());
    assert_eq!(ingress.outstanding(), 0);
    Ok(())
}

#[test]
fn binding_with_no_transitions_is_rejected_before_runtime_creation() -> Result<(), WorldRuntimeError>
{
    let content = synthetic_content("package-r1")?;
    let (_authority, _session, scope) = authority(22, 4, 1, 1)?;
    let scope_generation = ScopeOwnershipGeneration::new(1)
        .map_err(|_error: GenerationError| fixture_error("scope generation"))?;
    let active_content = ScopeContentGenerationFence::for_test(
        scope,
        scope_generation,
        ReferenceContentGeneration::from_content(&content)?,
    );
    assert!(matches!(
        LocalObjectRuntime::bind(
            &content,
            &active_content,
            scope,
            scope_generation,
            &PlacementKey::new(PLACEMENT_A)?,
            1,
            &[],
        ),
        Err(WorldRuntimeError::InvalidBinding(
            "local-object runtime requires at least one bound D38 world-object operation"
        ))
    ));
    Ok(())
}

#[test]
fn binding_a_foreign_definition_transition_is_rejected_before_runtime_creation()
-> Result<(), WorldRuntimeError> {
    let content = world_object_overlay_content("wo-r1")?;
    let (_authority, _session, scope) = authority(51, 7, 1, 1)?;
    let scope_generation = ScopeOwnershipGeneration::new(1)
        .map_err(|_error: GenerationError| fixture_error("scope generation"))?;
    let active_content = ScopeContentGenerationFence::for_test(
        scope,
        scope_generation,
        ReferenceContentGeneration::from_content(&content)?,
    );
    assert!(matches!(
        LocalObjectRuntime::bind(
            &content,
            &active_content,
            scope,
            scope_generation,
            &PlacementKey::new(WORLD_OBJECT_PLACEMENT)?,
            1,
            &[TransitionKey::new(FOREIGN_TRANSITION)?],
        ),
        Err(WorldRuntimeError::InvalidBinding(
            "bound transition does not target the selected placement definition"
        ))
    ));
    Ok(())
}

#[test]
fn invalid_active_authority_is_rejected_before_ingress_or_mutation() -> Result<(), WorldRuntimeError>
{
    let content = synthetic_content("package-r1")?;
    let (authority, session, scope) = authority(23, 4, 1, 1)?;
    let invalid = GameSessionAuthoritySnapshot::from_current_facts(
        authority.commit(),
        GameSessionState::Active,
        authority.current_connection_generation(),
        None,
        authority.current_character_lease(),
        authority.current_character_world_eligibility(),
        authority.current_runtime_scope(),
        authority.current_scope_generation(),
    )
    .map_err(|_error| fixture_error("invalid current authority fixture"))?;
    let mut runtime = runtime_for(&content, scope, PLACEMENT_A, 1)?;
    let command = command(&runtime, session, 1, 1, open_operation()?, 0)?;
    let before_state = runtime.state_key().clone();
    let before_blocking = runtime.blocking_cells().clone();
    let mut ingress = CommandIngress::new();

    assert!(matches!(
        runtime.apply(&invalid, &command, &mut ingress, &BTreeSet::new()),
        Err(WorldRuntimeError::InvalidSessionAuthority(
            "active GameSession has no current authenticated transport"
        ))
    ));
    assert_eq!(ingress.outstanding(), 0);
    assert_eq!(runtime.state_key(), &before_state);
    assert_eq!(runtime.revision(), 0);
    assert_eq!(runtime.blocking_cells(), &before_blocking);
    Ok(())
}

#[test]
fn authored_capability_cannot_grant_runtime_authority() -> Result<(), WorldRuntimeError> {
    let mut content = synthetic_content("package-r1")?;
    let unsupported = OwnerCapabilityRequirement {
        capability_key: ProductionKey::new("oteryn:runtime.capability.unowned-transition")?,
    };
    for transition in &mut content.transitions {
        transition.owner_capability = unsupported.clone();
    }
    let (_authority, _session, scope) = authority(21, 4, 1, 1)?;
    assert!(matches!(
        runtime_for(&content, scope, PLACEMENT_A, 1),
        Err(WorldRuntimeError::InvalidBinding(
            "transition capability is not supported by this runtime profile"
        ))
    ));
    Ok(())
}

#[test]
fn content_generation_binding_stays_within_foundation_component_bound()
-> Result<(), WorldRuntimeError> {
    let content = synthetic_content("package-r1")?;
    let generation = ReferenceContentGeneration::from_content(&content)?;
    let mut changed_lock = content.clone();
    changed_lock.content_lock.revision_digest_token =
        ProductionAtom::new("cw4 test content lock", "lock:changed")?;
    let changed_generation = ReferenceContentGeneration::from_content(&changed_lock)?;
    assert_eq!(generation.as_str().len(), 64);
    assert_eq!(changed_generation.as_str().len(), 64);
    assert_ne!(generation, changed_generation);
    Ok(())
}

#[test]
fn live_scope_rejects_different_content_generation_before_second_runtime_creation()
-> Result<(), WorldRuntimeError> {
    let content_a = synthetic_content("package-r1")?;
    let content_b = synthetic_content("package-r2")?;
    let (_authority, _session, scope) = authority(24, 4, 1, 1)?;
    let scope_generation = ScopeOwnershipGeneration::new(1)
        .map_err(|_error: GenerationError| fixture_error("scope generation"))?;
    let active_content = ScopeContentGenerationFence::for_test(
        scope,
        scope_generation,
        ReferenceContentGeneration::from_content(&content_a)?,
    );

    let runtime_a = LocalObjectRuntime::bind(
        &content_a,
        &active_content,
        scope,
        scope_generation,
        &PlacementKey::new(PLACEMENT_A)?,
        1,
        &[
            TransitionKey::new(OPEN_TRANSITION)?,
            TransitionKey::new(CLOSE_TRANSITION)?,
        ],
    )?;
    let before_state = runtime_a.state_key().clone();
    let before_blocking = runtime_a.blocking_cells().clone();

    let error = match LocalObjectRuntime::bind(
        &content_b,
        &active_content,
        scope,
        scope_generation,
        &PlacementKey::new(PLACEMENT_B)?,
        1,
        &[
            TransitionKey::new(OPEN_TRANSITION)?,
            TransitionKey::new(CLOSE_TRANSITION)?,
        ],
    ) {
        Ok(_runtime) => {
            return Err(fixture_error(
                "second Content generation unexpectedly activated in live scope",
            ));
        }
        Err(error) => error,
    };
    assert!(matches!(
        error,
        WorldRuntimeError::InvalidBinding(
            "different Content generation cannot activate while runtime scope is live"
        )
    ));
    assert_eq!(runtime_a.state_key(), &before_state);
    assert_eq!(runtime_a.revision(), 0);
    assert_eq!(runtime_a.blocking_cells(), &before_blocking);
    Ok(())
}

#[test]
fn forged_proven_target_evidence_is_rejected_before_runtime_creation()
-> Result<(), WorldRuntimeError> {
    let mut content = synthetic_content("package-r1")?;
    let forged = EvidenceBindingRef::new(
        ProductionAtom::new("reference manifest revision", "manifest-r1")?,
        ProductionKey::new(
            "oteryn:reference.case.ability_combat.light_healing.self_heal_semantics.v1",
        )?,
        EvidenceDisposition::Proven,
    );
    let placement = content
        .placements
        .first_mut()
        .ok_or(WorldRuntimeError::InvalidBinding(
            "CW4 test fixture has no placement",
        ))?;
    placement.address.evidence = forged;

    let (_authority, _session, scope) = authority(25, 4, 1, 1)?;
    assert!(matches!(
        runtime_for(&content, scope, PLACEMENT_A, 1),
        Err(WorldRuntimeError::InvalidBinding(
            "synthetic CW4 placement cannot claim Reference target promotion"
        ))
    ));
    Ok(())
}

#[test]
fn invalid_content_lock_is_rejected_by_cw3_linker_before_runtime_creation()
-> Result<(), WorldRuntimeError> {
    let mut content = synthetic_content("package-r1")?;
    let entry =
        content
            .content_lock
            .entries
            .first_mut()
            .ok_or(WorldRuntimeError::InvalidBinding(
                "CW4 test fixture has no Content Lock entry",
            ))?;
    entry.package_revision = ProductionAtom::new("cw4 test package revision", "package-corrupted")?;

    let (_authority, _session, scope) = authority(26, 4, 1, 1)?;
    assert!(matches!(
        runtime_for(&content, scope, PLACEMENT_A, 1),
        Err(WorldRuntimeError::Content(ContentError::InvalidArtifact(
            "reference-playable Content Lock does not bind exact root package provenance"
        )))
    ));
    Ok(())
}

#[test]
fn stale_scope_or_session_authority_fails_before_ingress_or_mutation()
-> Result<(), WorldRuntimeError> {
    let content = synthetic_content("package-r1")?;
    let (_authority, session, scope) = authority(30, 5, 1, 1)?;
    let mut runtime = runtime_for(&content, scope, PLACEMENT_A, 1)?;
    let command = command(&runtime, session, 1, 1, open_operation()?, 0)?;
    let mut ingress = CommandIngress::new();

    let (stale_scope, _, _) = authority(30, 5, 2, 1)?;
    assert!(matches!(
        runtime.apply(&stale_scope, &command, &mut ingress, &BTreeSet::new()),
        Err(WorldRuntimeError::StaleScopeOwnershipGeneration)
    ));
    assert_eq!(
        ingress.next_command_id(),
        Some(CommandId::new(1).map_err(|_error: CommandIdError| fixture_error("command id"))?)
    );
    assert_eq!(runtime.revision(), 0);

    let (wrong_session_authority, _, _) = authority(31, 5, 1, 1)?;
    assert!(matches!(
        runtime.apply(
            &wrong_session_authority,
            &command,
            &mut ingress,
            &BTreeSet::new()
        ),
        Err(WorldRuntimeError::StaleGameSession)
    ));
    assert_eq!(runtime.revision(), 0);
    Ok(())
}

#[test]
fn same_placement_in_two_channels_has_independent_overlay_state() -> Result<(), WorldRuntimeError> {
    let content = synthetic_content("package-r1")?;
    let (authority_a, session_a, scope_a) = authority(40, 6, 1, 1)?;
    let (_authority_b, _session_b, scope_b) = authority(41, 7, 1, 1)?;
    let mut runtime_a = runtime_for(&content, scope_a, PLACEMENT_A, 1)?;
    let runtime_b = runtime_for(&content, scope_b, PLACEMENT_A, 1)?;
    let before_b = runtime_b.blocking_cells().clone();
    let mut ingress_a = CommandIngress::new();
    let open = command(&runtime_a, session_a, 1, 1, open_operation()?, 0)?;

    runtime_a.apply(&authority_a, &open, &mut ingress_a, &BTreeSet::new())?;
    assert!(runtime_a.blocking_cells().is_empty());
    assert_eq!(
        runtime_b.state_key().as_str(),
        "oteryn:reference.state.closed"
    );
    assert_eq!(runtime_b.revision(), 0);
    assert_eq!(runtime_b.blocking_cells(), &before_b);
    Ok(())
}

#[test]
fn occupied_multicell_close_is_atomic_and_open_removes_only_own_contribution()
-> Result<(), WorldRuntimeError> {
    let content = synthetic_content("package-r1")?;
    let (authority, session, scope) = authority(50, 8, 1, 1)?;
    let mut runtime_a = runtime_for(&content, scope, PLACEMENT_A, 1)?;
    let runtime_b = runtime_for(&content, scope, PLACEMENT_B, 1)?;
    let mut ingress = CommandIngress::new();

    let open = command(&runtime_a, session, 1, 1, open_operation()?, 0)?;
    runtime_a.apply(&authority, &open, &mut ingress, &BTreeSet::new())?;
    assert!(runtime_a.blocking_cells().is_empty());
    assert_eq!(runtime_b.blocking_cells().len(), 2);

    let occupied_cell = runtime_a.collision_cells().iter().next().copied().ok_or(
        WorldRuntimeError::InvalidBinding("test fixture has no collision cell"),
    )?;
    let occupied = BTreeSet::from([occupied_cell]);
    let close = command(&runtime_a, session, 2, 1, close_operation()?, 1)?;
    let rejected = runtime_a.apply(&authority, &close, &mut ingress, &occupied)?;
    assert_eq!(rejected.disposition(), DISPOSITION_OCCUPIED);
    assert_eq!(
        runtime_a.state_key().as_str(),
        "oteryn:reference.state.open"
    );
    assert!(runtime_a.blocking_cells().is_empty());

    let close_after_leave = command(&runtime_a, session, 3, 1, close_operation()?, 1)?;
    let committed = runtime_a.apply(
        &authority,
        &close_after_leave,
        &mut ingress,
        &BTreeSet::new(),
    )?;
    assert_eq!(committed.disposition(), DISPOSITION_COMMITTED);
    assert_eq!(runtime_a.blocking_cells(), runtime_a.collision_cells());
    assert_eq!(runtime_a.blocking_cells().len(), 2);
    Ok(())
}

#[test]
fn changed_duplicate_conflicts_without_reexecuting_transition() -> Result<(), WorldRuntimeError> {
    let content = synthetic_content("package-r1")?;
    let (authority, session, scope) = authority(60, 9, 1, 1)?;
    let mut runtime = runtime_for(&content, scope, PLACEMENT_A, 1)?;
    let mut ingress = CommandIngress::new();
    let open = command(&runtime, session, 1, 1, open_operation()?, 0)?;
    runtime.apply(&authority, &open, &mut ingress, &BTreeSet::new())?;

    let changed = command(&runtime, session, 1, 1, close_operation()?, 1)?;
    assert!(matches!(
        runtime.apply(&authority, &changed, &mut ingress, &BTreeSet::new()),
        Err(WorldRuntimeError::ConflictChangedInput)
    ));
    assert_eq!(runtime.state_key().as_str(), "oteryn:reference.state.open");
    assert_eq!(runtime.revision(), 1);
    Ok(())
}

#[test]
fn changed_content_generation_on_duplicate_conflicts_without_replay()
-> Result<(), WorldRuntimeError> {
    let content = synthetic_content("package-r1")?;
    let mut changed_content = synthetic_content("package-r1")?;
    changed_content.content_lock.revision_digest_token =
        ProductionAtom::new("cw4 test content lock", "lock:changed-duplicate")?;
    let (authority, session, scope) = authority(69, 10, 1, 1)?;
    let mut runtime = runtime_for(&content, scope, PLACEMENT_A, 1)?;
    let mut ingress = CommandIngress::new();
    let first = command(&runtime, session, 1, 1, open_operation()?, 0)?;
    runtime.apply(&authority, &first, &mut ingress, &BTreeSet::new())?;

    let mut changed = first.clone();
    changed.content_generation = ReferenceContentGeneration::from_content(&changed_content)?;
    assert!(matches!(
        runtime.apply(&authority, &changed, &mut ingress, &BTreeSet::new()),
        Err(WorldRuntimeError::ConflictChangedInput)
    ));
    assert_eq!(runtime.state_key().as_str(), "oteryn:reference.state.open");
    assert_eq!(runtime.revision(), 1);
    assert!(runtime.blocking_cells().is_empty());
    Ok(())
}

#[test]
fn different_content_generation_is_terminally_rejected_while_scope_stays_live()
-> Result<(), WorldRuntimeError> {
    let content = synthetic_content("package-r1")?;
    let mut other_content = synthetic_content("package-r1")?;
    other_content.content_lock.revision_digest_token =
        ProductionAtom::new("cw4 test content lock", "lock:other-generation")?;
    let (authority, session, scope) = authority(70, 10, 1, 1)?;
    let mut runtime = runtime_for(&content, scope, PLACEMENT_A, 1)?;
    let mut ingress = CommandIngress::new();
    let mut command = command(&runtime, session, 1, 1, open_operation()?, 0)?;
    command.content_generation = ReferenceContentGeneration::from_content(&other_content)?;

    let result = runtime.apply(&authority, &command, &mut ingress, &BTreeSet::new())?;
    assert_eq!(result.disposition(), DISPOSITION_BINDING_MISMATCH);
    assert_eq!(
        runtime.state_key().as_str(),
        "oteryn:reference.state.closed"
    );
    assert_eq!(runtime.revision(), 0);
    assert_eq!(runtime.blocking_cells(), runtime.collision_cells());
    Ok(())
}

#[test]
fn ingress_capacity_failure_leaves_gameplay_and_spatial_state_unchanged()
-> Result<(), WorldRuntimeError> {
    let content = synthetic_content("package-r1")?;
    let (authority, session, scope) = authority(71, 11, 1, 1)?;
    let mut runtime = runtime_for(&content, scope, PLACEMENT_A, 1)?;
    let mut ingress = CommandIngress::new();
    let count = u64::try_from(MAX_OUTSTANDING_COMMANDS)
        .map_err(|_error| fixture_error("outstanding command count"))?;
    for raw in 1..=count {
        let command_id =
            CommandId::new(raw).map_err(|_error: CommandIdError| fixture_error("command id"))?;
        let semantic = CommandSemanticIdentity::new(
            NormalizedSemanticIntentIdentity::new(
                "oteryn:capacity.placement",
                raw,
                "oteryn:capacity.intent",
                0,
            )?,
            RetainedBindingIdentity::new(
                "oteryn:capacity.generation",
                "oteryn:capacity.transition",
            )?,
        );
        assert_eq!(
            ingress.reserve(command_id, semantic),
            IngressDecision::Reserved(command_id)
        );
    }
    let next = count
        .checked_add(1)
        .ok_or(WorldRuntimeError::InvalidBinding(
            "test command count overflow",
        ))?;
    let open = command(&runtime, session, next, 1, open_operation()?, 0)?;
    let before = runtime.blocking_cells().clone();
    assert!(matches!(
        runtime.apply(&authority, &open, &mut ingress, &BTreeSet::new()),
        Err(WorldRuntimeError::IngressCapacityExceeded { .. })
    ));
    assert_eq!(runtime.revision(), 0);
    assert_eq!(runtime.blocking_cells(), &before);
    Ok(())
}
