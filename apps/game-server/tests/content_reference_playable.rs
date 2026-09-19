use oteryn_game_server::content::*;
use oteryn_game_server::foundation::WorldId;

fn world_id() -> Result<WorldId, ContentError> {
    let bytes = [
        0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0x70, 0xcd, 0x8e, 0xf0, 0x12, 0x34, 0x56, 0x78, 0x9a,
        0xbc,
    ];
    WorldId::decode(&bytes)
        .map_err(|_| ContentError::InvalidArtifact("reference-playable test WorldId invalid"))
}

fn accepted_case_key() -> Result<ProductionKey, ContentError> {
    ProductionKey::new("oteryn:reference.case.ability_combat.light_healing.self_heal_semantics.v1")
}

fn accepted_case_binding() -> Result<EvidenceBindingRef, ContentError> {
    EvidenceBindingRef::from_accepted_case(accepted_case_key()?)
}

fn qualified_footprint(
    members: Vec<FootprintCell>,
    evidence: EvidenceBindingRef,
) -> FootprintRelation {
    FootprintRelation::Qualified { members, evidence }
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
    let object_ref = TypedDefinitionRef::new(
        DefinitionFamily::LocalObject,
        object_key,
        DefinitionRevisionRef::new("definition-r1")?,
    );
    let closed = ProductionKey::new("oteryn:reference.state.closed")?;
    let open = ProductionKey::new("oteryn:reference.state.open")?;

    let effect_ref = TypedDefinitionRef::new(
        DefinitionFamily::Effect,
        ProductionKey::new("oteryn:reference.effect.heal")?,
        DefinitionRevisionRef::new("definition-r1")?,
    );

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
        coordinate_frame: CoordinateFrameRef::new("global-target-2026-07-28")?,
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
        placements: vec![],
        ordered_placements: vec![],
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

fn source_with_typed_item(
    client_projection: ClientProjectionClass,
) -> Result<ReferencePlayableContentSource, ContentError> {
    let mut candidate = source()?;
    candidate.definitions.push(ReferenceDefinition {
        definition: TypedDefinitionRef::new(
            DefinitionFamily::Item,
            ProductionKey::new("oteryn:reference.item.cw3-b1-probe")?,
            DefinitionRevisionRef::new("definition-r1")?,
        ),
        kind: ReferenceDefinitionKind::Item(ReferenceItemDefinition {
            physical_class: ReferenceItemPhysicalClass::Physical,
            materializable: true,
            stack_class: ReferenceItemStackClass::StackCapable,
            legal_destinations: vec![ReferenceItemDestination::CharacterInventory],
        }),
        client_projection,
    });
    Ok(candidate)
}

fn typed_item_definition(
    definitions: &[ReferenceDefinition],
) -> Result<&ReferenceDefinition, ContentError> {
    definitions
        .iter()
        .find(|definition| definition.definition.family() == DefinitionFamily::Item)
        .ok_or(ContentError::InvalidArtifact(
            "reference-playable typed item probe missing",
        ))
}

fn typed_item_definition_mut(
    source: &mut ReferencePlayableContentSource,
) -> Result<&mut ReferenceDefinition, ContentError> {
    source
        .definitions
        .iter_mut()
        .find(|definition| definition.definition.family() == DefinitionFamily::Item)
        .ok_or(ContentError::InvalidArtifact(
            "reference-playable typed item probe missing",
        ))
}

fn typed_item_kind_mut(
    source: &mut ReferencePlayableContentSource,
) -> Result<&mut ReferenceItemDefinition, ContentError> {
    match &mut typed_item_definition_mut(source)?.kind {
        ReferenceDefinitionKind::Item(item) => Ok(item),
        _ => Err(ContentError::InvalidArtifact(
            "reference-playable typed item probe changed kind",
        )),
    }
}

fn source_with_target_claim(
    evidence: EvidenceBindingRef,
) -> Result<ReferencePlayableContentSource, ContentError> {
    let mut candidate = source()?;
    let definition = candidate.definitions[0].definition.clone();
    let placement_key = PlacementKey::new("oteryn:reference.placement.local-door")?;
    let placement = PlacementRef {
        key: placement_key.clone(),
        map_revision: MapRevisionRef::new("map-r1")?,
        definition,
        address: SpatialAddress {
            world_id: candidate.world_id,
            coordinate_frame: candidate.coordinate_frame.clone(),
            cell: LogicalCell {
                x: 33_572,
                y: 32_528,
                z: 7,
            },
            evidence: evidence.clone(),
        },
        presentation_footprint: qualified_footprint(
            vec![
                FootprintCell {
                    dx: 0,
                    dy: 1,
                    dz: 0,
                },
                FootprintCell {
                    dx: 0,
                    dy: 0,
                    dz: 0,
                },
            ],
            evidence.clone(),
        ),
        collision_footprint: qualified_footprint(
            vec![FootprintCell {
                dx: 0,
                dy: 0,
                dz: 0,
            }],
            evidence.clone(),
        ),
    };
    candidate.placements.push(placement);
    Ok(candidate)
}

fn source_with_ordered_target_claim(
    evidence: EvidenceBindingRef,
) -> Result<ReferencePlayableContentSource, ContentError> {
    let mut candidate = source_with_target_claim(evidence.clone())?;
    candidate.ordered_placements.push(OrderedPlacementSet {
        field_key: ProductionKey::new("oteryn:reference.field.local-door")?,
        placement_keys: vec![candidate.placements[0].key.clone()],
        evidence,
    });
    Ok(candidate)
}

#[test]
fn valid_reference_model_without_unproven_target_claims_links() -> Result<(), ContentError> {
    let canonical = link_reference_playable(source()?)?;
    assert_eq!(
        canonical.profile_revision.as_str(),
        REFERENCE_PLAYABLE_CONTENT_PROFILE_ID
    );
    assert_ne!(
        REFERENCE_PLAYABLE_CONTENT_PROFILE_ID,
        FIRST_PRODUCTION_PROFILE_ID
    );
    assert!(canonical.placements.is_empty());
    assert!(canonical.ordered_placements.is_empty());
    assert!(matches!(
        canonical.definitions[1].kind,
        ReferenceDefinitionKind::Effect(ReferenceEffectFamily::Heal)
    ));

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
fn typed_item_static_semantics_link_deterministically() -> Result<(), ContentError> {
    let left_source = source_with_typed_item(ClientProjectionClass::ServerOnly)?;
    let mut right_source = left_source.clone();
    right_source.definitions.reverse();

    let left = link_reference_playable(left_source)?;
    let right = link_reference_playable(right_source)?;
    assert_eq!(left, right);

    let definition = typed_item_definition(&left.definitions)?;
    assert_eq!(
        definition.definition.key().as_str(),
        "oteryn:reference.item.cw3-b1-probe"
    );
    assert_eq!(definition.definition.revision().as_str(), "definition-r1");
    let ReferenceDefinitionKind::Item(item) = &definition.kind else {
        return Err(ContentError::InvalidArtifact(
            "reference-playable typed item probe changed kind",
        ));
    };
    assert_eq!(item.physical_class, ReferenceItemPhysicalClass::Physical);
    assert!(item.materializable);
    assert_eq!(item.stack_class, ReferenceItemStackClass::StackCapable);
    assert_eq!(
        item.legal_destinations,
        vec![ReferenceItemDestination::CharacterInventory]
    );
    Ok(())
}

#[test]
fn item_family_requires_typed_supported_capabilities() -> Result<(), ContentError> {
    let mut undeclared = source()?;
    undeclared.definitions.push(ReferenceDefinition {
        definition: TypedDefinitionRef::new(
            DefinitionFamily::Item,
            ProductionKey::new("oteryn:reference.item.cw3-b1-undeclared")?,
            DefinitionRevisionRef::new("definition-r1")?,
        ),
        kind: ReferenceDefinitionKind::Generic,
        client_projection: ClientProjectionClass::ServerOnly,
    });
    assert!(matches!(
        link_reference_playable(undeclared),
        Err(ContentError::InvalidArtifact(
            "reference-playable item requires typed static item semantics"
        ))
    ));

    let mut missing_destination = source_with_typed_item(ClientProjectionClass::ServerOnly)?;
    typed_item_kind_mut(&mut missing_destination)?
        .legal_destinations
        .clear();
    assert!(matches!(
        link_reference_playable(missing_destination),
        Err(ContentError::InvalidArtifact(
            "reference-playable materializable item requires CharacterInventory destination capability"
        ))
    ));

    let mut impossible_destination = source_with_typed_item(ClientProjectionClass::ServerOnly)?;
    typed_item_kind_mut(&mut impossible_destination)?.materializable = false;
    assert!(matches!(
        link_reference_playable(impossible_destination),
        Err(ContentError::InvalidArtifact(
            "reference-playable non-materializable item cannot declare CharacterInventory destination capability"
        ))
    ));
    Ok(())
}

#[test]
fn item_kind_wrong_family_fails_closed() -> Result<(), ContentError> {
    let mut candidate = source_with_typed_item(ClientProjectionClass::ServerOnly)?;
    let definition = typed_item_definition_mut(&mut candidate)?;
    definition.definition = TypedDefinitionRef::new(
        DefinitionFamily::Creature,
        definition.definition.key().clone(),
        definition.definition.revision().clone(),
    );
    assert!(matches!(
        link_reference_playable(candidate),
        Err(ContentError::InvalidArtifact(
            "reference-playable definition kind does not match definition family"
        ))
    ));
    Ok(())
}

#[test]
fn item_client_projection_omits_server_legality_fields() -> Result<(), ContentError> {
    let canonical =
        link_reference_playable(source_with_typed_item(ClientProjectionClass::ClientSafe)?)?;
    let full_definition = typed_item_definition(&canonical.definitions)?;
    let ReferenceDefinitionKind::Item(full_item) = &full_definition.kind else {
        return Err(ContentError::InvalidArtifact(
            "reference-playable typed item probe changed kind",
        ));
    };
    assert!(full_item.materializable);
    assert_eq!(
        full_item.legal_destinations,
        vec![ReferenceItemDestination::CharacterInventory]
    );

    let client_definition = canonical
        .client_safe_definitions()
        .into_iter()
        .find(|definition| definition.definition.family() == DefinitionFamily::Item)
        .ok_or(ContentError::InvalidArtifact(
            "reference-playable client typed item probe missing",
        ))?;
    assert_eq!(
        client_definition.kind,
        ClientSafeDefinitionKind::Item(ClientSafeItemDefinition {
            physical_class: ReferenceItemPhysicalClass::Physical,
            stack_class: ReferenceItemStackClass::StackCapable,
        })
    );
    Ok(())
}

#[test]
fn wrong_family_reference_fails_closed() -> Result<(), ContentError> {
    let mut candidate = source()?;
    let current = candidate.transitions[0].definition.clone();
    candidate.transitions[0].definition = TypedDefinitionRef::new(
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
    let current = candidate.transitions[0].definition.clone();
    candidate.transitions[0].definition = TypedDefinitionRef::new(
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
fn accepted_manifest_classification_cannot_be_overridden_by_caller() -> Result<(), ContentError> {
    let actual = accepted_case_binding()?;
    let forged_disposition = if actual.disposition() == EvidenceDisposition::Proven {
        EvidenceDisposition::Unknown
    } else {
        EvidenceDisposition::Proven
    };
    let forged = EvidenceBindingRef::new(
        actual.manifest_revision().clone(),
        actual.case_key().clone(),
        forged_disposition,
    );
    assert!(matches!(
        link_reference_playable(source_with_target_claim(forged)?),
        Err(ContentError::InvalidArtifact(
            "reference-playable evidence disposition does not match accepted manifest"
        ))
    ));
    Ok(())
}

#[test]
fn accepted_manifest_nonpromotable_case_stays_fail_closed() -> Result<(), ContentError> {
    let actual = accepted_case_binding()?;
    if actual.disposition() != EvidenceDisposition::Proven {
        assert!(matches!(
            link_reference_playable(source_with_target_claim(actual)?),
            Err(ContentError::InvalidArtifact(
                "target-sensitive Reference claim lacks promotable evidence"
            ))
        ));
    }
    Ok(())
}

#[test]
fn stale_manifest_revision_fails_closed() -> Result<(), ContentError> {
    let actual = accepted_case_binding()?;
    let stale = EvidenceBindingRef::new(
        ProductionAtom::new("reference manifest revision", "manifest-r999")?,
        actual.case_key().clone(),
        actual.disposition(),
    );
    assert!(matches!(
        link_reference_playable(source_with_target_claim(stale)?),
        Err(ContentError::RevisionMismatch(
            "reference-playable evidence manifest revision"
        ))
    ));
    Ok(())
}

#[test]
fn missing_manifest_case_fails_closed() -> Result<(), ContentError> {
    let actual = accepted_case_binding()?;
    let missing = EvidenceBindingRef::new(
        actual.manifest_revision().clone(),
        ProductionKey::new("oteryn:reference.case.content_world.missing.v1")?,
        EvidenceDisposition::Proven,
    );
    assert!(matches!(
        link_reference_playable(source_with_target_claim(missing)?),
        Err(ContentError::MissingReference { .. })
    ));
    Ok(())
}

#[test]
fn duplicate_placement_key_rejects_before_evidence_promotion() -> Result<(), ContentError> {
    let mut candidate = source_with_target_claim(accepted_case_binding()?)?;
    candidate.placements.push(candidate.placements[0].clone());
    assert!(matches!(
        link_reference_playable(candidate),
        Err(ContentError::DuplicateKey(_))
    ));
    Ok(())
}

#[test]
fn source_enumeration_order_does_not_change_canonical_result() -> Result<(), ContentError> {
    let left = source()?;
    let mut right = left.clone();
    right.definitions.reverse();
    right.transitions.reverse();

    assert_eq!(
        link_reference_playable(left)?,
        link_reference_playable(right)?
    );
    Ok(())
}

#[test]
fn coordinate_frame_mismatch_rejects_before_evidence_promotion() -> Result<(), ContentError> {
    let mut candidate = source_with_target_claim(accepted_case_binding()?)?;
    candidate.placements[0].address.coordinate_frame = CoordinateFrameRef::new("legacy-ots-frame")?;
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
    let mut candidate = source_with_target_claim(accepted_case_binding()?)?;
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
fn duplicate_footprint_member_rejects_before_evidence_promotion() -> Result<(), ContentError> {
    let evidence = accepted_case_binding()?;
    let mut candidate = source_with_target_claim(evidence.clone())?;
    let member = FootprintCell {
        dx: 0,
        dy: 0,
        dz: 0,
    };
    candidate.placements[0].collision_footprint =
        qualified_footprint(vec![member, member], evidence);
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
fn ordered_placement_relation_rejects_unknown_or_duplicate_members_before_evidence()
-> Result<(), ContentError> {
    let mut unknown = source_with_ordered_target_claim(accepted_case_binding()?)?;
    unknown.ordered_placements[0]
        .placement_keys
        .push(PlacementKey::new("oteryn:reference.placement.unknown")?);
    assert!(matches!(
        link_reference_playable(unknown),
        Err(ContentError::MissingReference { .. })
    ));

    let mut duplicate = source_with_ordered_target_claim(accepted_case_binding()?)?;
    let first = duplicate.ordered_placements[0].placement_keys[0].clone();
    duplicate.ordered_placements[0].placement_keys.push(first);
    assert!(matches!(
        link_reference_playable(duplicate),
        Err(ContentError::DuplicateKey(_))
    ));
    Ok(())
}
