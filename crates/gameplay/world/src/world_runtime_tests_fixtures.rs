fn synthetic_content(
    package_revision_value: &str,
) -> Result<CanonicalReferencePlayableContent, WorldRuntimeError> {
    let world_id = decode_world(1)?;
    let package_key = ProductionKey::new("oteryn:content.cw4-local-object")?;
    let package_revision =
        ProductionAtom::new("cw4 test package revision", package_revision_value)?;
    let package_manifest = PackageManifestBinding::new(
        package_key.clone(),
        package_revision.clone(),
        ProductionAtom::new("cw4 test schema", "schema-v1")?,
        ProductionAtom::new("cw4 test license", "license:project-owned-v1")?,
        Sha256HexDigest::new("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")?,
    );
    let provenance = package_manifest.package_provenance_digest()?;
    let content_lock = ContentLockBinding {
        revision_digest_token: ProductionAtom::new(
            "cw4 test content lock",
            &format!("lock:{package_revision_value}"),
        )?,
        entries: vec![ContentLockEntry::exact(
            package_key,
            package_revision,
            provenance,
        )],
    };

    let object_ref = TypedDefinitionRef::new(
        DefinitionFamily::LocalObject,
        ProductionKey::new("oteryn:reference.object.cw4-local")?,
        DefinitionRevisionRef::new("definition-r1")?,
    );
    let closed = ProductionKey::new("oteryn:reference.state.closed")?;
    let open = ProductionKey::new("oteryn:reference.state.open")?;
    let coordinate_frame = CoordinateFrameRef::new("global-target-2026-09-27")?;
    let capability = OwnerCapabilityRequirement {
        capability_key: ProductionKey::new("oteryn:runtime.capability.local-object-transition")?,
    };

    let mut canonical = link_reference_playable(ReferencePlayableContentSource {
        profile_revision: ProductionAtom::new(
            "cw4 test profile",
            REFERENCE_PLAYABLE_CONTENT_PROFILE_ID,
        )?,
        capability_profile: ProductionAtom::new(
            "cw4 test capability profile",
            REFERENCE_PLAYABLE_CAPABILITY_PROFILE,
        )?,
        package_manifest,
        content_lock,
        world_id,
        coordinate_frame: coordinate_frame.clone(),
        definitions: vec![ReferenceDefinition {
            definition: object_ref.clone(),
            kind: ReferenceDefinitionKind::LocalObjectStates(vec![
                LocalObjectStateDefinition {
                    key: closed.clone(),
                    collision: LocalObjectCollisionPresence::Present,
                    attribute_variant_of: None,
                    absent: false,
                },
                LocalObjectStateDefinition {
                    key: open.clone(),
                    collision: LocalObjectCollisionPresence::Absent,
                    attribute_variant_of: None,
                    absent: false,
                },
            ]),
            client_projection: ClientProjectionClass::ClientSafe,
        }],
        placements: vec![],
        ordered_placements: vec![],
        transitions: vec![
            TransitionBinding {
                key: TransitionKey::new(OPEN_TRANSITION)?,
                definition: object_ref.clone(),
                source_state: closed.clone(),
                normalized_intent_family: ProductionKey::new(
                    "oteryn:reference.intent.local-object-open",
                )?,
                target_state: open.clone(),
                owner_capability: capability.clone(),
                policy_guard_refs: vec![],
            },
            TransitionBinding {
                key: TransitionKey::new(CLOSE_TRANSITION)?,
                definition: object_ref.clone(),
                source_state: open.clone(),
                normalized_intent_family: ProductionKey::new(
                    "oteryn:reference.intent.local-object-close",
                )?,
                target_state: closed.clone(),
                owner_capability: capability,
                policy_guard_refs: vec![],
            },
        ],
    })?;

    // Protected CW3 currently has no accepted CONTENT_WORLD target-sensitive
    // evidence binding. The CW4 component is therefore a synthetic in-process
    // runtime proof only: its placement witness is deliberately unpromoted
    // and must never be interpreted as Reference target authority.
    let evidence = EvidenceBindingRef::new(
        ProductionAtom::new("reference manifest revision", "manifest-r0")?,
        ProductionKey::new("oteryn:cw4.local-object-placement")?,
        EvidenceDisposition::Unknown,
    );
    let collision_members = vec![
        FootprintCell {
            dx: 0,
            dy: 0,
            dz: 0,
        },
        FootprintCell {
            dx: 1,
            dy: 0,
            dz: 0,
        },
    ];
    let placement = |key: &str| -> Result<PlacementRef, WorldRuntimeError> {
        Ok(PlacementRef {
            key: PlacementKey::new(key)?,
            map_revision: MapRevisionRef::new("map-r1")?,
            definition: object_ref.clone(),
            address: crate::content::SpatialAddress {
                world_id,
                coordinate_frame: coordinate_frame.clone(),
                cell: LogicalCell {
                    x: 100,
                    y: 200,
                    z: 7,
                },
                evidence: evidence.clone(),
            },
            presentation_footprint: FootprintRelation::Qualified {
                members: collision_members.clone(),
                evidence: evidence.clone(),
            },
            collision_footprint: FootprintRelation::Qualified {
                members: collision_members.clone(),
                evidence: evidence.clone(),
            },
            // D38 W1b mechanical adaptation: both CW4 fixture placements author the same
            // starting state the runtime previously derived from the OPEN transition, so
            // existing CW4 test assertions (both instances start Closed) are unchanged.
            local_object_initial_state: Some(closed.clone()),
            local_object_state_attributes: BTreeMap::new(),
            local_object_revert_after_ms: BTreeMap::new(),
            local_object_event_transitions: BTreeMap::new(),
        })
    };
    canonical.placements = vec![placement(PLACEMENT_A)?, placement(PLACEMENT_B)?];
    Ok(canonical)
}

// D38 world-object overlay fixture: a four-state vocabulary (beyond the fixed two-state
// Open/Close pair) exercising the full TRANSFORM/CREATE/REMOVE/RETAG operation family this
// generalized runtime supports, plus a second, unrelated LocalObject definition used only to
// construct a transition this runtime must reject as foreign at bind time.
const WORLD_OBJECT_PLACEMENT: &str = "oteryn:reference.placement.world-object-overlay";
const CREATE_TRANSITION: &str = "oteryn:reference.transition.world-object-create";
const REMOVE_TRANSITION: &str = "oteryn:reference.transition.world-object-remove";
const RETAG_TRANSITION: &str = "oteryn:reference.transition.world-object-retag";
const TRANSFORM_TRANSITION: &str = "oteryn:reference.transition.world-object-transform";
const FOREIGN_TRANSITION: &str = "oteryn:reference.transition.world-object-foreign";

fn world_object_overlay_content(
    package_revision_value: &str,
) -> Result<CanonicalReferencePlayableContent, WorldRuntimeError> {
    let world_id = decode_world(1)?;
    let package_key = ProductionKey::new("oteryn:content.cw4-world-object-overlay")?;
    let package_revision =
        ProductionAtom::new("cw4 test package revision", package_revision_value)?;
    let package_manifest = PackageManifestBinding::new(
        package_key.clone(),
        package_revision.clone(),
        ProductionAtom::new("cw4 test schema", "schema-v1")?,
        ProductionAtom::new("cw4 test license", "license:project-owned-v1")?,
        Sha256HexDigest::new("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")?,
    );
    let provenance = package_manifest.package_provenance_digest()?;
    let content_lock = ContentLockBinding {
        revision_digest_token: ProductionAtom::new(
            "cw4 test content lock",
            &format!("world-object-lock:{package_revision_value}"),
        )?,
        entries: vec![ContentLockEntry::exact(
            package_key,
            package_revision,
            provenance,
        )],
    };

    let object_ref = TypedDefinitionRef::new(
        DefinitionFamily::LocalObject,
        ProductionKey::new("oteryn:reference.object.world-object-anchor")?,
        DefinitionRevisionRef::new("definition-r1")?,
    );
    let foreign_ref = TypedDefinitionRef::new(
        DefinitionFamily::LocalObject,
        ProductionKey::new("oteryn:reference.object.world-object-foreign")?,
        DefinitionRevisionRef::new("definition-r1")?,
    );
    let absent = ProductionKey::new("oteryn:reference.state.world-object-absent")?;
    let present = ProductionKey::new("oteryn:reference.state.world-object-present")?;
    let armed = ProductionKey::new("oteryn:reference.state.world-object-armed")?;
    let dormant = ProductionKey::new("oteryn:reference.state.world-object-dormant")?;
    let foreign_state_a = ProductionKey::new("oteryn:reference.state.foreign-a")?;
    let foreign_state_b = ProductionKey::new("oteryn:reference.state.foreign-b")?;
    let coordinate_frame = CoordinateFrameRef::new("global-target-2026-09-27")?;
    let capability = OwnerCapabilityRequirement {
        capability_key: ProductionKey::new("oteryn:runtime.capability.local-object-transition")?,
    };

    let mut canonical = link_reference_playable(ReferencePlayableContentSource {
        profile_revision: ProductionAtom::new(
            "cw4 test profile",
            REFERENCE_PLAYABLE_CONTENT_PROFILE_ID,
        )?,
        capability_profile: ProductionAtom::new(
            "cw4 test capability profile",
            REFERENCE_PLAYABLE_CAPABILITY_PROFILE,
        )?,
        package_manifest,
        content_lock,
        world_id,
        coordinate_frame: coordinate_frame.clone(),
        definitions: vec![
            ReferenceDefinition {
                definition: object_ref.clone(),
                kind: ReferenceDefinitionKind::LocalObjectStates(vec![
                    LocalObjectStateDefinition {
                        key: absent.clone(),
                        collision: LocalObjectCollisionPresence::Absent,
                        attribute_variant_of: None,
                        absent: false,
                    },
                    LocalObjectStateDefinition {
                        key: present.clone(),
                        collision: LocalObjectCollisionPresence::Present,
                        attribute_variant_of: None,
                        absent: false,
                    },
                    LocalObjectStateDefinition {
                        key: armed.clone(),
                        collision: LocalObjectCollisionPresence::Present,
                        attribute_variant_of: None,
                        absent: false,
                    },
                    LocalObjectStateDefinition {
                        key: dormant.clone(),
                        collision: LocalObjectCollisionPresence::Absent,
                        attribute_variant_of: None,
                        absent: false,
                    },
                ]),
                client_projection: ClientProjectionClass::ClientSafe,
            },
            ReferenceDefinition {
                definition: foreign_ref.clone(),
                kind: ReferenceDefinitionKind::LocalObjectStates(vec![
                    LocalObjectStateDefinition {
                        key: foreign_state_a.clone(),
                        collision: LocalObjectCollisionPresence::Absent,
                        attribute_variant_of: None,
                        absent: false,
                    },
                    LocalObjectStateDefinition {
                        key: foreign_state_b.clone(),
                        collision: LocalObjectCollisionPresence::Present,
                        attribute_variant_of: None,
                        absent: false,
                    },
                ]),
                client_projection: ClientProjectionClass::ClientSafe,
            },
        ],
        placements: vec![],
        ordered_placements: vec![],
        transitions: vec![
            TransitionBinding {
                key: TransitionKey::new(CREATE_TRANSITION)?,
                definition: object_ref.clone(),
                source_state: absent.clone(),
                normalized_intent_family: ProductionKey::new(
                    "oteryn:reference.intent.world-object-create",
                )?,
                target_state: present.clone(),
                owner_capability: capability.clone(),
                policy_guard_refs: vec![],
            },
            TransitionBinding {
                key: TransitionKey::new(REMOVE_TRANSITION)?,
                definition: object_ref.clone(),
                source_state: present.clone(),
                normalized_intent_family: ProductionKey::new(
                    "oteryn:reference.intent.world-object-remove",
                )?,
                target_state: absent.clone(),
                owner_capability: capability.clone(),
                policy_guard_refs: vec![],
            },
            TransitionBinding {
                key: TransitionKey::new(RETAG_TRANSITION)?,
                definition: object_ref.clone(),
                source_state: present.clone(),
                normalized_intent_family: ProductionKey::new(LOCAL_OBJECT_RETAG_INTENT_FAMILY)?,
                target_state: armed.clone(),
                owner_capability: capability.clone(),
                policy_guard_refs: vec![],
            },
            TransitionBinding {
                key: TransitionKey::new(TRANSFORM_TRANSITION)?,
                definition: object_ref.clone(),
                source_state: armed.clone(),
                normalized_intent_family: ProductionKey::new(
                    "oteryn:reference.intent.world-object-transform",
                )?,
                target_state: dormant.clone(),
                owner_capability: capability.clone(),
                policy_guard_refs: vec![],
            },
            TransitionBinding {
                key: TransitionKey::new(FOREIGN_TRANSITION)?,
                definition: foreign_ref,
                source_state: foreign_state_a,
                normalized_intent_family: ProductionKey::new(
                    "oteryn:reference.intent.world-object-foreign",
                )?,
                target_state: foreign_state_b,
                owner_capability: capability,
                policy_guard_refs: vec![],
            },
        ],
    })?;

    // Same disjointness rationale as `synthetic_content`'s placement witness: unpromoted,
    // synthetic-only evidence, never Reference target authority.
    let evidence = EvidenceBindingRef::new(
        ProductionAtom::new("reference manifest revision", "manifest-r0")?,
        ProductionKey::new("oteryn:cw4.world-object-placement")?,
        EvidenceDisposition::Unknown,
    );
    let collision_members = vec![
        FootprintCell {
            dx: 0,
            dy: 0,
            dz: 0,
        },
        FootprintCell {
            dx: 1,
            dy: 0,
            dz: 0,
        },
    ];
    canonical.placements = vec![PlacementRef {
        key: PlacementKey::new(WORLD_OBJECT_PLACEMENT)?,
        map_revision: MapRevisionRef::new("map-r1")?,
        definition: object_ref,
        address: crate::content::SpatialAddress {
            world_id,
            coordinate_frame,
            cell: LogicalCell {
                x: 300,
                y: 400,
                z: 3,
            },
            evidence: evidence.clone(),
        },
        presentation_footprint: FootprintRelation::Qualified {
            members: collision_members.clone(),
            evidence: evidence.clone(),
        },
        collision_footprint: FootprintRelation::Qualified {
            members: collision_members,
            evidence,
        },
        local_object_initial_state: Some(absent),
        local_object_state_attributes: BTreeMap::new(),
        local_object_revert_after_ms: BTreeMap::new(),
        local_object_event_transitions: BTreeMap::new(),
    }];
    Ok(canonical)
}

fn authority(
    session_seed: u8,
    channel_seed: u8,
    scope_generation: u64,
    connection_generation: u64,
) -> Result<
    (
        GameSessionAuthoritySnapshot<u64>,
        crate::foundation::GameSessionId,
        RuntimeScopeRefV1,
    ),
    WorldRuntimeError,
> {
    let world = decode_world(1)?;
    let channel = ChannelId::decode(&uuid_v7(channel_seed))
        .map_err(|_error: FoundationProtocolError| fixture_error("channel"))?;
    let character = CharacterId::decode(&uuid_v7(session_seed.wrapping_add(80)))
        .map_err(|_error: FoundationProtocolError| fixture_error("character"))?;
    let session = GameSessionId::decode(&uuid_v7(session_seed))
        .map_err(|_error: FoundationProtocolError| fixture_error("session"))?;
    let mut nonce = [0_u8; 32];
    nonce[31] = session_seed.max(1);
    let facts = FreshAdmissionFacts::new(nonce, character, world, channel, 1, scope_generation)
        .map_err(|_error| fixture_error("fresh admission"))?;
    let commit = FreshAdmissionCommit::from_facts(session, facts, 99_u64)
        .map_err(|_error| fixture_error("fresh commit"))?;
    let scope = RuntimeScopeRefV1::channel(world, channel);
    let connection_generation = ConnectionGeneration::new(connection_generation)
        .map_err(|_error: GenerationError| fixture_error("connection generation"))?;
    let character_lease =
        CharacterLease::new(character, 1).map_err(|_error| fixture_error("character lease"))?;
    let scope_generation = ScopeOwnershipGeneration::new(scope_generation)
        .map_err(|_error: GenerationError| fixture_error("scope generation"))?;
    let snapshot = GameSessionAuthoritySnapshot::new(
        commit,
        GameSessionState::Active,
        connection_generation,
        Some(99_u64),
        character_lease,
        scope_generation,
    );
    Ok((snapshot, session, scope))
}

fn runtime_for(
    content: &CanonicalReferencePlayableContent,
    scope: RuntimeScopeRefV1,
    placement: &str,
    scope_generation: u64,
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
        &PlacementKey::new(placement)?,
        1,
        &[
            TransitionKey::new(OPEN_TRANSITION)?,
            TransitionKey::new(CLOSE_TRANSITION)?,
        ],
    )
}

fn open_operation() -> Result<LocalObjectOperation, WorldRuntimeError> {
    Ok(LocalObjectOperation::new(TransitionKey::new(
        OPEN_TRANSITION,
    )?))
}

fn close_operation() -> Result<LocalObjectOperation, WorldRuntimeError> {
    Ok(LocalObjectOperation::new(TransitionKey::new(
        CLOSE_TRANSITION,
    )?))
}

fn command(
    runtime: &LocalObjectRuntime,
    session: GameSessionId,
    id: u64,
    connection_generation: u64,
    operation: LocalObjectOperation,
    expected_revision: u64,
) -> Result<LocalObjectCommand, WorldRuntimeError> {
    let command_id =
        CommandId::new(id).map_err(|_error: CommandIdError| fixture_error("command id"))?;
    let connection_generation = ConnectionGeneration::new(connection_generation)
        .map_err(|_error: GenerationError| fixture_error("connection generation"))?;
    Ok(LocalObjectCommand::new(
        CommandRef::new(session, command_id),
        connection_generation,
        runtime.placement_key().clone(),
        runtime.incarnation(),
        runtime.content_generation().clone(),
        operation,
        expected_revision,
    ))
}
