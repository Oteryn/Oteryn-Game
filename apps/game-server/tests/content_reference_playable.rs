use oteryn_game_server::content::*;
use oteryn_game_server::foundation::WorldId;

fn world_id() -> Result<WorldId, ContentError> {
    let bytes = [
        0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0x70, 0xcd, 0x8e, 0xf0, 0x12, 0x34, 0x56, 0x78,
        0x9a, 0xbc,
    ];
    WorldId::decode(&bytes)
        .map_err(|_| ContentError::InvalidArtifact("reference-playable test WorldId invalid"))
}

fn evidence(disposition: EvidenceDisposition) -> Result<EvidenceBindingRef, ContentError> {
    Ok(EvidenceBindingRef::new(
        ProductionAtom::new("reference manifest revision", "manifest-r4")?,
        ProductionKey::new("oteryn:reference.case.local-object")?,
        disposition,
    ))
}

fn qualified_footprint(
    members: Vec<FootprintCell>,
) -> Result<FootprintRelation, ContentError> {
    Ok(FootprintRelation::Qualified {
        members,
        evidence: evidence(EvidenceDisposition::Proven)?,
    })
}

fn source() -> Result<ReferencePlayableContentSource, ContentError> {
    let package_key = ProductionKey::new("oteryn:content.reference-playable")?;
    let package_revision = ProductionAtom::new("reference package revision", "package-r1")?;
    let package_manifest = PackageManifestBinding::new(
        package_key.clone(),
        package_revision.clone(),
        ProductionAtom::new("reference semantic schema", "schema-v1")?,
        ProductionAtom::new("reference licensing metadata", "license:project-owned-v1")?,
        Sha256HexDigest::new("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")?,
    );
    let provenance = package_manifest.package_provenance_digest()?;
    let content_lock = ContentLockBinding {
        revision_digest_token: ProductionAtom::new("reference content lock", "lock:reference-r1")?,
        entries: vec![ContentLockEntry::exact(
            package_key,
            package_revision,
            provenance,
        )],
    };

    let object_key = ProductionKey::new("oteryn:reference.object.local-door")?;
    let object_revision = DefinitionRevisionRef::new("definition-r1")?;
    let object_ref = TypedDefinitionRef::new(
        DefinitionFamily::LocalObject,
        object_key,
        object_revision,
    );
    let closed = ProductionKey::new("oteryn:reference.state.closed")?;
    let open = ProductionKey::new("oteryn:reference.state.open")?;

    let effect_ref = TypedDefinitionRef::new(
        DefinitionFamily::Effect,
        ProductionKey::new("oteryn:reference.effect.heal")?,
        DefinitionRevisionRef::new("definition-r1")?,
    );

    let placement_key = PlacementKey::new("oteryn:reference.placement.local-door")?;
    let coordinate_frame = CoordinateFrameRef::new("global-target-2026-07-28")?;
    let placement = PlacementRef {
        key: placement_key.clone(),
        map_revision: MapRevisionRef::new("map-r1")?,
        definition: object_ref.clone(),
        address: SpatialAddress {
            world_id: world_id()?,
            coordinate_frame: coordinate_frame.clone(),
            cell: LogicalCell {
                x: 33_572,
                y: 32_528,
                z: 7,
            },
            evidence: evidence(EvidenceDisposition::Proven)?,
        },
        presentation_footprint: qualified_footprint(vec![
            FootprintCell {
                dx: 0,
                dy: 0,
                dz: 0,
            },
            FootprintCell {
                dx: 0,
                dy: 1,
                dz: 0,
            },
        ])?,
        collision_footprint: qualified_footprint(vec![FootprintCell {
            dx: 0,
            dy: 0,
            dz: 0,
        }])?,
    };

    Ok(ReferencePlayableContentSource {
        profile_revision: ProductionAtom::new(
            "reference profile revision",
            REFERENCE_PLAYABLE_CONTENT_PROFILE_ID,
        )?,
        capability_profile: ProductionAtom::new(
            "reference capability profile",
            REFERENCE_PLAYABLE_CAPABILITY_PROFILE,
        )?,
        package_manifest,
        content_lock,
        world_id: world_id()?,
        coordinate_frame,
        definitions: vec![
            ReferenceDefinition {
                definition: object_ref.clone(),
                kind: ReferenceDefinitionKind::LocalObjectStates(vec![
                    closed.clone(),
                    open.clone(),
                ]),
                client_projection: ClientProjectionClass::ClientSafe,
            },
            ReferenceDefinition {
                definition: effect_ref,
                kind: ReferenceDefinitionKind::Effect(ReferenceEffectFamily::Heal),
                client_projection: ClientProjectionClass::ServerOnly,
            },
        ],
        placements: vec![placement],
        ordered_placements: vec![OrderedPlacementSet {
            field_key: ProductionKey::new("oteryn:reference.field.local-door")?,
            placement_keys: vec![placement_key],
            evidence: evidence(EvidenceDisposition::Proven)?,
        }],
        transitions: vec![TransitionBinding {
            key: TransitionKey::new("oteryn:reference.transition.open")?,
            definition: object_ref,
            source_state: closed,
            normalized_intent_family: ProductionKey::new(
                "oteryn:reference.intent.local-object-open",
            )?,
            target_state: open,
            owner_capability: OwnerCapabilityRequirement {
                capability_key: ProductionKey::new(
                    "oteryn:runtime.capability.local-object-transition",
                )?,
            },
            policy_guard_refs: vec![ProductionKey::new(
                "oteryn:reference.guard.local-object-open",
            )?],
        }],
    })
}

#[test]
fn valid_reference_model_links_and_preserves_successor_boundaries() -> Result<(), ContentError> {
    let canonical = link_reference_playable(source()?)?;
    assert_eq!(
        canonical.profile_revision.as_str(),
        REFERENCE_PLAYABLE_CONTENT_PROFILE_ID
    );
    assert_ne!(
        REFERENCE_PLAYABLE_CONTENT_PROFILE_ID,
        FIRST_PRODUCTION_PROFILE_ID
    );
    assert!(matches!(
        canonical.definitions[1].kind,
        ReferenceDefinitionKind::Effect(ReferenceEffectFamily::Heal)
    ));
    assert_ne!(
        canonical.placements[0].presentation_footprint,
        canonical.placements[0].collision_footprint
    );

    let client = canonical.client_safe_definitions();
    assert_eq!(client.len(), 1);
    assert_eq!(client[0].definition.family(), DefinitionFamily::LocalObject);
    assert!(matches!(
        client[0].kind,
        ClientSafeDefinitionKind::LocalObjectStates(_)
    ));
    Ok(())
}

#[test]
fn wrong_family_reference_fails_closed() -> Result<(), ContentError> {
    let mut candidate = source()?;
    let current = candidate.placements[0].definition.clone();
    candidate.placements[0].definition = TypedDefinitionRef::new(
        DefinitionFamily::Item,
        current.key().clone(),
        current.revision().clone(),
    );
    assert!(matches!(
        link_reference_playable(candidate),
        Err(ContentError::MissingReference { .. })
    ));
    Ok(())
}

#[test]
fn incompatible_definition_revision_fails_closed() -> Result<(), ContentError> {
    let mut candidate = source()?;
    let current = candidate.placements[0].definition.clone();
    candidate.placements[0].definition = TypedDefinitionRef::new(
        current.family(),
        current.key().clone(),
        DefinitionRevisionRef::new("definition-r2")?,
    );
    assert!(matches!(
        link_reference_playable(candidate),
        Err(ContentError::RevisionMismatch(
            "reference-playable definition revision"
        ))
    ));
    Ok(())
}

#[test]
fn unsafe_target_evidence_classes_fail_closed() -> Result<(), ContentError> {
    for disposition in [
        EvidenceDisposition::Unknown,
        EvidenceDisposition::Conflict,
        EvidenceDisposition::OtsHypothesisOnly,
        EvidenceDisposition::ObservedPostTarget,
    ] {
        let mut candidate = source()?;
        candidate.placements[0].address.evidence = evidence(disposition)?;
        assert!(
            link_reference_playable(candidate).is_err(),
            "{disposition:?} unexpectedly promoted"
        );
    }
    Ok(())
}

#[test]
fn derived_and_continuity_proven_evidence_can_remain_typed() -> Result<(), ContentError> {
    for disposition in [
        EvidenceDisposition::Derived,
        EvidenceDisposition::ObservedContinuityProven,
    ] {
        let mut candidate = source()?;
        candidate.placements[0].address.evidence = evidence(disposition)?;
        assert!(link_reference_playable(candidate).is_ok());
    }
    Ok(())
}

#[test]
fn duplicate_placement_key_rejects() -> Result<(), ContentError> {
    let mut candidate = source()?;
    candidate.placements.push(candidate.placements[0].clone());
    assert!(matches!(
        link_reference_playable(candidate),
        Err(ContentError::DuplicateKey(_))
    ));
    Ok(())
}

#[test]
fn source_enumeration_order_does_not_change_canonical_result() -> Result<(), ContentError> {
    let mut left = source()?;
    let mut second = left.placements[0].clone();
    second.key = PlacementKey::new("oteryn:reference.placement.local-door.second")?;
    second.address.cell.x += 1;
    left.ordered_placements[0]
        .placement_keys
        .push(second.key.clone());
    left.placements.push(second);

    let mut right = left.clone();
    right.definitions.reverse();
    right.placements.reverse();
    right.transitions.reverse();

    assert_eq!(
        link_reference_playable(left)?,
        link_reference_playable(right)?
    );
    Ok(())
}

#[test]
fn coordinate_frame_mismatch_rejects() -> Result<(), ContentError> {
    let mut candidate = source()?;
    candidate.placements[0].address.coordinate_frame =
        CoordinateFrameRef::new("legacy-ots-frame")?;
    assert!(matches!(
        link_reference_playable(candidate),
        Err(ContentError::InvalidArtifact(
            "reference-playable coordinate frame mismatch"
        ))
    ));
    Ok(())
}

#[test]
fn unresolved_collision_is_not_implicit_walkable_space() -> Result<(), ContentError> {
    let mut candidate = source()?;
    candidate.placements[0].collision_footprint =
        FootprintRelation::Unresolved(UnresolvedTargetField::Unknown);
    assert!(matches!(
        link_reference_playable(candidate),
        Err(ContentError::InvalidArtifact(
            "reference-playable footprint remains unresolved"
        ))
    ));
    Ok(())
}

#[test]
fn duplicate_footprint_member_rejects() -> Result<(), ContentError> {
    let mut candidate = source()?;
    let member = FootprintCell {
        dx: 0,
        dy: 0,
        dz: 0,
    };
    candidate.placements[0].collision_footprint =
        qualified_footprint(vec![member, member])?;
    assert!(matches!(
        link_reference_playable(candidate),
        Err(ContentError::InvalidArtifact(
            "reference-playable footprint contains duplicate member"
        ))
    ));
    Ok(())
}

#[test]
fn transition_requires_declared_local_object_state() -> Result<(), ContentError> {
    let mut candidate = source()?;
    candidate.transitions[0].target_state =
        ProductionKey::new("oteryn:reference.state.unavailable")?;
    assert!(matches!(
        link_reference_playable(candidate),
        Err(ContentError::MissingReference { .. })
    ));
    Ok(())
}

#[test]
fn wrong_successor_profile_and_capability_reject() -> Result<(), ContentError> {
    let mut wrong_profile = source()?;
    wrong_profile.profile_revision =
        ProductionAtom::new("reference profile revision", FIRST_PRODUCTION_PROFILE_ID)?;
    assert!(matches!(
        link_reference_playable(wrong_profile),
        Err(ContentError::RevisionMismatch(
            "reference-playable profile revision"
        ))
    ));

    let mut wrong_capability = source()?;
    wrong_capability.capability_profile =
        ProductionAtom::new("reference capability profile", "content:other-v1")?;
    assert!(matches!(
        link_reference_playable(wrong_capability),
        Err(ContentError::RevisionMismatch(
            "reference-playable capability profile"
        ))
    ));
    Ok(())
}

#[test]
fn content_lock_must_bind_exact_package_provenance() -> Result<(), ContentError> {
    let mut candidate = source()?;
    candidate.content_lock.entries[0].package_provenance_digest =
        Sha256HexDigest::new("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc")?;
    assert!(matches!(
        link_reference_playable(candidate),
        Err(ContentError::InvalidArtifact(
            "reference-playable Content Lock does not bind exact root package provenance"
        ))
    ));
    Ok(())
}

#[test]
fn ordered_placement_relation_rejects_unknown_or_duplicate_members() -> Result<(), ContentError> {
    let mut unknown = source()?;
    unknown.ordered_placements[0]
        .placement_keys
        .push(PlacementKey::new("oteryn:reference.placement.unknown")?);
    assert!(matches!(
        link_reference_playable(unknown),
        Err(ContentError::MissingReference { .. })
    ));

    let mut duplicate = source()?;
    let first = duplicate.ordered_placements[0].placement_keys[0].clone();
    duplicate.ordered_placements[0].placement_keys.push(first);
    assert!(matches!(
        link_reference_playable(duplicate),
        Err(ContentError::DuplicateKey(_))
    ));
    Ok(())
}
