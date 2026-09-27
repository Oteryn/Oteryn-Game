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
    let formula_ref = TypedDefinitionRef::new(
        DefinitionFamily::Formula,
        ProductionKey::new("oteryn:reference.formula.project-owned-heal")?,
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
                kind: ReferenceDefinitionKind::Effect(ReferenceEffectDefinition {
                    family: ReferenceEffectFamily::Heal,
                    formula: formula_ref.clone(),
                }),
                client_projection: ClientProjectionClass::ServerOnly,
            },
            ReferenceDefinition {
                definition: formula_ref,
                kind: ReferenceDefinitionKind::Formula(ReferenceFormulaDefinition),
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
            semantics: Default::default(),
        }),
        client_projection,
    });
    Ok(candidate)
}

fn source_with_ability_effect_formula() -> Result<ReferencePlayableContentSource, ContentError> {
    let mut candidate = source()?;
    let heal = candidate.definitions[1].definition.clone();
    let damage_formula = TypedDefinitionRef::new(
        DefinitionFamily::Formula,
        ProductionKey::new("oteryn:reference.formula.project-owned-damage")?,
        DefinitionRevisionRef::new("definition-r1")?,
    );
    let damage = TypedDefinitionRef::new(
        DefinitionFamily::Effect,
        ProductionKey::new("oteryn:reference.effect.project-owned-damage")?,
        DefinitionRevisionRef::new("definition-r1")?,
    );
    let ability = TypedDefinitionRef::new(
        DefinitionFamily::Ability,
        ProductionKey::new("oteryn:reference.ability.project-owned-closure")?,
        DefinitionRevisionRef::new("definition-r1")?,
    );
    candidate.definitions.extend([
        ReferenceDefinition {
            definition: damage_formula.clone(),
            kind: ReferenceDefinitionKind::Formula(ReferenceFormulaDefinition),
            client_projection: ClientProjectionClass::ServerOnly,
        },
        ReferenceDefinition {
            definition: damage.clone(),
            kind: ReferenceDefinitionKind::Effect(ReferenceEffectDefinition {
                family: ReferenceEffectFamily::Damage,
                formula: damage_formula,
            }),
            client_projection: ClientProjectionClass::ClientSafe,
        },
        ReferenceDefinition {
            definition: ability,
            kind: ReferenceDefinitionKind::Ability(ReferenceAbilityDefinition {
                effects: vec![heal.clone(), damage, heal],
            }),
            client_projection: ClientProjectionClass::ServerOnly,
        },
    ]);
    Ok(candidate)
}

fn ability_kind_mut(
    source: &mut ReferencePlayableContentSource,
) -> Result<&mut ReferenceAbilityDefinition, ContentError> {
    source
        .definitions
        .iter_mut()
        .find_map(|definition| match &mut definition.kind {
            ReferenceDefinitionKind::Ability(ability) => Some(ability),
            _ => None,
        })
        .ok_or(ContentError::InvalidArtifact("structural ability missing"))
}

fn damage_effect_kind_mut(
    source: &mut ReferencePlayableContentSource,
) -> Result<&mut ReferenceEffectDefinition, ContentError> {
    source
        .definitions
        .iter_mut()
        .find_map(|definition| match &mut definition.kind {
            ReferenceDefinitionKind::Effect(effect)
                if effect.family == ReferenceEffectFamily::Damage =>
            {
                Some(effect)
            }
            _ => None,
        })
        .ok_or(ContentError::InvalidArtifact(
            "structural damage effect missing",
        ))
}

fn source_with_creature_loot() -> Result<ReferencePlayableContentSource, ContentError> {
    let mut candidate = source_with_typed_item(ClientProjectionClass::ClientSafe)?;

    let second_item = TypedDefinitionRef::new(
        DefinitionFamily::Item,
        ProductionKey::new("oteryn:reference.item.cw3-b2-secondary-probe")?,
        DefinitionRevisionRef::new("definition-r1")?,
    );
    candidate.definitions.push(ReferenceDefinition {
        definition: second_item.clone(),
        kind: ReferenceDefinitionKind::Item(ReferenceItemDefinition {
            physical_class: ReferenceItemPhysicalClass::Physical,
            materializable: true,
            stack_class: ReferenceItemStackClass::NonStackable,
            legal_destinations: vec![ReferenceItemDestination::CharacterInventory],
            semantics: Default::default(),
        }),
        client_projection: ClientProjectionClass::ServerOnly,
    });

    let presentation = TypedDefinitionRef::new(
        DefinitionFamily::Presentation,
        ProductionKey::new("oteryn:reference.presentation.cw3-b2-creature-probe")?,
        DefinitionRevisionRef::new("definition-r1")?,
    );
    candidate.definitions.push(ReferenceDefinition {
        definition: presentation.clone(),
        kind: ReferenceDefinitionKind::Generic,
        client_projection: ClientProjectionClass::ClientSafe,
    });

    let behavior = TypedDefinitionRef::new(
        DefinitionFamily::Behavior,
        ProductionKey::new("oteryn:reference.behavior.cw3-b2-creature-probe")?,
        DefinitionRevisionRef::new("definition-r1")?,
    );
    candidate.definitions.push(ReferenceDefinition {
        definition: behavior.clone(),
        kind: ReferenceDefinitionKind::Generic,
        client_projection: ClientProjectionClass::ServerOnly,
    });

    let primary_item = candidate
        .definitions
        .iter()
        .find(|definition| {
            definition.definition.family() == DefinitionFamily::Item
                && definition.definition.key().as_str() == "oteryn:reference.item.cw3-b1-probe"
        })
        .ok_or(ContentError::InvalidArtifact(
            "reference-playable primary item probe missing",
        ))?
        .definition
        .clone();

    let loot = TypedDefinitionRef::new(
        DefinitionFamily::Loot,
        ProductionKey::new("oteryn:reference.loot.cw3-b2-creature-probe")?,
        DefinitionRevisionRef::new("definition-r1")?,
    );
    candidate.definitions.push(ReferenceDefinition {
        definition: loot.clone(),
        kind: ReferenceDefinitionKind::Loot(ReferenceLootDefinition {
            algorithm: ReferenceLootSelectionAlgorithm::IndependentBernoulliPpm,
            entries: vec![
                ReferenceLootEntry {
                    item: second_item,
                    min_count: 1,
                    max_count: 1,
                    probability_ppm: Some(125_000),
                },
                ReferenceLootEntry {
                    item: primary_item,
                    min_count: 1,
                    max_count: 3,
                    probability_ppm: Some(750_000),
                },
            ],
        }),
        client_projection: ClientProjectionClass::ServerOnly,
    });

    candidate.definitions.push(ReferenceDefinition {
        definition: TypedDefinitionRef::new(
            DefinitionFamily::Creature,
            ProductionKey::new("oteryn:reference.creature.cw3-b2-probe")?,
            DefinitionRevisionRef::new("definition-r1")?,
        ),
        kind: ReferenceDefinitionKind::Creature(ReferenceCreatureDefinition {
            presentation,
            behavior,
            loot: Some(loot),
        }),
        client_projection: ClientProjectionClass::ClientSafe,
    });

    Ok(candidate)
}

fn typed_creature_kind_mut(
    source: &mut ReferencePlayableContentSource,
) -> Result<&mut ReferenceCreatureDefinition, ContentError> {
    let definition = source
        .definitions
        .iter_mut()
        .find(|definition| definition.definition.family() == DefinitionFamily::Creature)
        .ok_or(ContentError::InvalidArtifact(
            "reference-playable typed creature probe missing",
        ))?;
    match &mut definition.kind {
        ReferenceDefinitionKind::Creature(creature) => Ok(creature),
        _ => Err(ContentError::InvalidArtifact(
            "reference-playable typed creature probe changed kind",
        )),
    }
}

fn typed_loot_definition_mut(
    source: &mut ReferencePlayableContentSource,
) -> Result<&mut ReferenceDefinition, ContentError> {
    source
        .definitions
        .iter_mut()
        .find(|definition| definition.definition.family() == DefinitionFamily::Loot)
        .ok_or(ContentError::InvalidArtifact(
            "reference-playable typed loot probe missing",
        ))
}

fn typed_loot_kind_mut(
    source: &mut ReferencePlayableContentSource,
) -> Result<&mut ReferenceLootDefinition, ContentError> {
    match &mut typed_loot_definition_mut(source)?.kind {
        ReferenceDefinitionKind::Loot(loot) => Ok(loot),
        _ => Err(ContentError::InvalidArtifact(
            "reference-playable typed loot probe changed kind",
        )),
    }
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
        ReferenceDefinitionKind::Effect(ReferenceEffectDefinition {
            family: ReferenceEffectFamily::Heal,
            ..
        })
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
fn typed_creature_and_loot_semantics_link_deterministically() -> Result<(), ContentError> {
    let left_source = source_with_creature_loot()?;
    let mut right_source = left_source.clone();
    right_source.definitions.reverse();
    typed_loot_kind_mut(&mut right_source)?.entries.reverse();

    let left = link_reference_playable(left_source)?;
    let right = link_reference_playable(right_source)?;
    assert_eq!(left, right);

    let creature = left
        .definitions
        .iter()
        .find(|definition| definition.definition.family() == DefinitionFamily::Creature)
        .ok_or(ContentError::InvalidArtifact(
            "reference-playable typed creature probe missing",
        ))?;
    let ReferenceDefinitionKind::Creature(creature) = &creature.kind else {
        return Err(ContentError::InvalidArtifact(
            "reference-playable typed creature probe changed kind",
        ));
    };
    assert_eq!(
        creature.presentation.family(),
        DefinitionFamily::Presentation
    );
    assert_eq!(creature.behavior.family(), DefinitionFamily::Behavior);
    assert_eq!(
        creature
            .loot
            .as_ref()
            .ok_or(ContentError::InvalidArtifact(
                "reference-playable typed creature loot probe missing",
            ))?
            .family(),
        DefinitionFamily::Loot
    );

    let loot = left
        .definitions
        .iter()
        .find(|definition| definition.definition.family() == DefinitionFamily::Loot)
        .ok_or(ContentError::InvalidArtifact(
            "reference-playable typed loot probe missing",
        ))?;
    let ReferenceDefinitionKind::Loot(loot) = &loot.kind else {
        return Err(ContentError::InvalidArtifact(
            "reference-playable typed loot probe changed kind",
        ));
    };
    assert_eq!(
        loot.algorithm,
        ReferenceLootSelectionAlgorithm::IndependentBernoulliPpm
    );
    assert_eq!(loot.entries.len(), 2);
    assert_eq!(
        loot.entries[0].item.key().as_str(),
        "oteryn:reference.item.cw3-b1-probe"
    );
    assert_eq!(loot.entries[0].probability_ppm, Some(750_000));
    Ok(())
}

#[test]
fn creature_and_loot_families_require_typed_kinds() -> Result<(), ContentError> {
    let mut creature = source()?;
    creature.definitions.push(ReferenceDefinition {
        definition: TypedDefinitionRef::new(
            DefinitionFamily::Creature,
            ProductionKey::new("oteryn:reference.creature.cw3-b2-undeclared")?,
            DefinitionRevisionRef::new("definition-r1")?,
        ),
        kind: ReferenceDefinitionKind::Generic,
        client_projection: ClientProjectionClass::ServerOnly,
    });
    assert!(matches!(
        link_reference_playable(creature),
        Err(ContentError::InvalidArtifact(
            "reference-playable creature requires typed static creature semantics"
        ))
    ));

    let mut loot = source()?;
    loot.definitions.push(ReferenceDefinition {
        definition: TypedDefinitionRef::new(
            DefinitionFamily::Loot,
            ProductionKey::new("oteryn:reference.loot.cw3-b2-undeclared")?,
            DefinitionRevisionRef::new("definition-r1")?,
        ),
        kind: ReferenceDefinitionKind::Generic,
        client_projection: ClientProjectionClass::ServerOnly,
    });
    assert!(matches!(
        link_reference_playable(loot),
        Err(ContentError::InvalidArtifact(
            "reference-playable loot requires typed loot table semantics"
        ))
    ));
    Ok(())
}

#[test]
fn creature_typed_references_fail_closed() -> Result<(), ContentError> {
    let mut missing = source_with_creature_loot()?;
    typed_creature_kind_mut(&mut missing)?.presentation = TypedDefinitionRef::new(
        DefinitionFamily::Presentation,
        ProductionKey::new("oteryn:reference.presentation.missing")?,
        DefinitionRevisionRef::new("definition-r1")?,
    );
    assert!(matches!(
        link_reference_playable(missing),
        Err(ContentError::MissingReference { .. })
    ));

    let mut wrong_family = source_with_creature_loot()?;
    let behavior = typed_creature_kind_mut(&mut wrong_family)?.behavior.clone();
    typed_creature_kind_mut(&mut wrong_family)?.behavior = TypedDefinitionRef::new(
        DefinitionFamily::Presentation,
        behavior.key().clone(),
        behavior.revision().clone(),
    );
    assert!(matches!(
        link_reference_playable(wrong_family),
        Err(ContentError::InvalidArtifact(
            "reference-playable creature behavior reference must target Behavior"
        ))
    ));

    let mut wrong_revision = source_with_creature_loot()?;
    let loot = typed_creature_kind_mut(&mut wrong_revision)?
        .loot
        .clone()
        .ok_or(ContentError::InvalidArtifact(
            "reference-playable typed creature loot probe missing",
        ))?;
    typed_creature_kind_mut(&mut wrong_revision)?.loot = Some(TypedDefinitionRef::new(
        DefinitionFamily::Loot,
        loot.key().clone(),
        DefinitionRevisionRef::new("definition-r2")?,
    ));
    assert!(matches!(
        link_reference_playable(wrong_revision),
        Err(ContentError::RevisionMismatch(
            "reference-playable definition revision"
        ))
    ));
    Ok(())
}

#[test]
fn loot_entries_resolve_only_to_typed_items() -> Result<(), ContentError> {
    let mut wrong_family = source_with_creature_loot()?;
    let item = typed_loot_kind_mut(&mut wrong_family)?.entries[0]
        .item
        .clone();
    typed_loot_kind_mut(&mut wrong_family)?.entries[0].item = TypedDefinitionRef::new(
        DefinitionFamily::Creature,
        item.key().clone(),
        item.revision().clone(),
    );
    assert!(matches!(
        link_reference_playable(wrong_family),
        Err(ContentError::InvalidArtifact(
            "reference-playable loot entry must target Item"
        ))
    ));

    let mut missing = source_with_creature_loot()?;
    typed_loot_kind_mut(&mut missing)?.entries[0].item = TypedDefinitionRef::new(
        DefinitionFamily::Item,
        ProductionKey::new("oteryn:reference.item.missing")?,
        DefinitionRevisionRef::new("definition-r1")?,
    );
    assert!(matches!(
        link_reference_playable(missing),
        Err(ContentError::MissingReference { .. })
    ));

    let mut wrong_revision = source_with_creature_loot()?;
    let item = typed_loot_kind_mut(&mut wrong_revision)?.entries[0]
        .item
        .clone();
    typed_loot_kind_mut(&mut wrong_revision)?.entries[0].item = TypedDefinitionRef::new(
        DefinitionFamily::Item,
        item.key().clone(),
        DefinitionRevisionRef::new("definition-r2")?,
    );
    assert!(matches!(
        link_reference_playable(wrong_revision),
        Err(ContentError::RevisionMismatch(
            "reference-playable definition revision"
        ))
    ));
    Ok(())
}

#[test]
fn loot_algorithm_probability_and_count_domains_fail_closed() -> Result<(), ContentError> {
    let mut missing_probability = source_with_creature_loot()?;
    typed_loot_kind_mut(&mut missing_probability)?.entries[0].probability_ppm = None;
    assert!(matches!(
        link_reference_playable(missing_probability),
        Err(ContentError::InvalidArtifact(
            "reference-playable Bernoulli loot entry requires explicit probability_ppm"
        ))
    ));

    let mut oversized_probability = source_with_creature_loot()?;
    typed_loot_kind_mut(&mut oversized_probability)?.entries[0].probability_ppm =
        Some(REFERENCE_LOOT_PROBABILITY_PPM_SCALE + 1);
    assert!(matches!(
        link_reference_playable(oversized_probability),
        Err(ContentError::InvalidArtifact(
            "reference-playable loot probability_ppm exceeds one million"
        ))
    ));

    let mut zero_count = source_with_creature_loot()?;
    typed_loot_kind_mut(&mut zero_count)?.entries[0].min_count = 0;
    assert!(matches!(
        link_reference_playable(zero_count),
        Err(ContentError::InvalidArtifact(
            "reference-playable loot entry requires a positive ordered count range"
        ))
    ));

    let mut reversed_count = source_with_creature_loot()?;
    typed_loot_kind_mut(&mut reversed_count)?.entries[0].min_count = 3;
    typed_loot_kind_mut(&mut reversed_count)?.entries[0].max_count = 2;
    assert!(matches!(
        link_reference_playable(reversed_count),
        Err(ContentError::InvalidArtifact(
            "reference-playable loot entry requires a positive ordered count range"
        ))
    ));

    let mut guaranteed = source_with_creature_loot()?;
    typed_loot_kind_mut(&mut guaranteed)?.algorithm =
        ReferenceLootSelectionAlgorithm::GuaranteedEntries;
    for entry in &mut typed_loot_kind_mut(&mut guaranteed)?.entries {
        entry.probability_ppm = None;
    }
    let guaranteed = link_reference_playable(guaranteed)?;
    let guaranteed_loot = guaranteed
        .definitions
        .iter()
        .find(|definition| definition.definition.family() == DefinitionFamily::Loot)
        .ok_or(ContentError::InvalidArtifact(
            "reference-playable typed loot probe missing",
        ))?;
    assert!(matches!(
        guaranteed_loot.kind,
        ReferenceDefinitionKind::Loot(ReferenceLootDefinition {
            algorithm: ReferenceLootSelectionAlgorithm::GuaranteedEntries,
            ..
        })
    ));

    let mut mixed_guaranteed = source_with_creature_loot()?;
    typed_loot_kind_mut(&mut mixed_guaranteed)?.algorithm =
        ReferenceLootSelectionAlgorithm::GuaranteedEntries;
    assert!(matches!(
        link_reference_playable(mixed_guaranteed),
        Err(ContentError::InvalidArtifact(
            "reference-playable guaranteed loot entry cannot also declare probability_ppm"
        ))
    ));

    let mut weighted = source_with_creature_loot()?;
    typed_loot_kind_mut(&mut weighted)?.algorithm =
        ReferenceLootSelectionAlgorithm::WeightedSingleSelection;
    assert!(matches!(
        link_reference_playable(weighted),
        Err(ContentError::InvalidArtifact(
            "reference-playable weighted loot entries require separately accepted typed weight semantics"
        ))
    ));

    let mut empty_weighted = source_with_creature_loot()?;
    let empty_weighted_loot = typed_loot_kind_mut(&mut empty_weighted)?;
    empty_weighted_loot.algorithm = ReferenceLootSelectionAlgorithm::WeightedSingleSelection;
    empty_weighted_loot.entries.clear();
    assert!(matches!(
        link_reference_playable(empty_weighted),
        Err(ContentError::InvalidArtifact(
            "reference-playable weighted loot entries require separately accepted typed weight semantics"
        ))
    ));

    let mut nested = source_with_creature_loot()?;
    typed_loot_kind_mut(&mut nested)?.algorithm = ReferenceLootSelectionAlgorithm::NestedGroups;
    assert!(matches!(
        link_reference_playable(nested),
        Err(ContentError::InvalidArtifact(
            "reference-playable nested loot groups require separately accepted typed group references"
        ))
    ));

    let mut empty_nested = source_with_creature_loot()?;
    let empty_nested_loot = typed_loot_kind_mut(&mut empty_nested)?;
    empty_nested_loot.algorithm = ReferenceLootSelectionAlgorithm::NestedGroups;
    empty_nested_loot.entries.clear();
    assert!(matches!(
        link_reference_playable(empty_nested),
        Err(ContentError::InvalidArtifact(
            "reference-playable nested loot groups require separately accepted typed group references"
        ))
    ));
    Ok(())
}

#[test]
fn creature_projection_exposes_presentation_only_and_loot_stays_server_only()
-> Result<(), ContentError> {
    let canonical = link_reference_playable(source_with_creature_loot()?)?;
    let client = canonical.client_safe_definitions();
    assert!(
        !client
            .iter()
            .any(|definition| definition.definition.family() == DefinitionFamily::Loot)
    );

    let client_creature = client
        .iter()
        .find(|definition| definition.definition.family() == DefinitionFamily::Creature)
        .ok_or(ContentError::InvalidArtifact(
            "reference-playable client creature probe missing",
        ))?;
    let ClientSafeDefinitionKind::Creature(client_creature) = &client_creature.kind else {
        return Err(ContentError::InvalidArtifact(
            "reference-playable client creature probe changed kind",
        ));
    };
    assert_eq!(
        client_creature.presentation.family(),
        DefinitionFamily::Presentation
    );

    let full_creature = canonical
        .definitions
        .iter()
        .find(|definition| definition.definition.family() == DefinitionFamily::Creature)
        .ok_or(ContentError::InvalidArtifact(
            "reference-playable typed creature probe missing",
        ))?;
    let ReferenceDefinitionKind::Creature(full_creature) = &full_creature.kind else {
        return Err(ContentError::InvalidArtifact(
            "reference-playable typed creature probe changed kind",
        ));
    };
    assert_eq!(full_creature.behavior.family(), DefinitionFamily::Behavior);
    assert_eq!(
        full_creature
            .loot
            .as_ref()
            .ok_or(ContentError::InvalidArtifact(
                "reference-playable typed creature loot probe missing",
            ))?
            .family(),
        DefinitionFamily::Loot
    );

    let mut loot_leak = source_with_creature_loot()?;
    typed_loot_definition_mut(&mut loot_leak)?.client_projection =
        ClientProjectionClass::ClientSafe;
    assert!(matches!(
        link_reference_playable(loot_leak),
        Err(ContentError::InvalidArtifact(
            "reference-playable loot selection authority must remain server-only"
        ))
    ));

    let mut server_only_presentation = source_with_creature_loot()?;
    server_only_presentation
        .definitions
        .iter_mut()
        .find(|definition| definition.definition.family() == DefinitionFamily::Presentation)
        .ok_or(ContentError::InvalidArtifact(
            "reference-playable presentation probe missing",
        ))?
        .client_projection = ClientProjectionClass::ServerOnly;
    assert!(matches!(
        link_reference_playable(server_only_presentation),
        Err(ContentError::InvalidArtifact(
            "reference-playable client-safe creature requires client-safe presentation target"
        ))
    ));
    Ok(())
}

#[test]
fn cw3_b2_model_tests_do_not_promote_cw2_source_identity() -> Result<(), ContentError> {
    let canonical = link_reference_playable(source_with_creature_loot()?)?;
    for definition in &canonical.definitions {
        assert!(
            definition
                .definition
                .key()
                .as_str()
                .starts_with("oteryn:reference.")
        );
        assert!(
            !definition
                .definition
                .key()
                .as_str()
                .starts_with("oteryn:item.")
        );
        if let ReferenceDefinitionKind::Loot(loot) = &definition.kind {
            for entry in &loot.entries {
                assert!(entry.item.key().as_str().starts_with("oteryn:reference."));
                assert!(!entry.item.key().as_str().starts_with("oteryn:item."));
            }
        }
    }
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

#[test]
fn structural_ability_effect_formula_chain_links_without_rewriting_authored_effects()
-> Result<(), ContentError> {
    let left_source = source_with_ability_effect_formula()?;
    let mut right_source = left_source.clone();
    right_source.definitions.reverse();
    let left = link_reference_playable(left_source)?;
    let right = link_reference_playable(right_source)?;
    assert_eq!(left, right);

    let ability = left
        .definitions
        .iter()
        .find_map(|definition| match &definition.kind {
            ReferenceDefinitionKind::Ability(ability) => Some(ability),
            _ => None,
        })
        .ok_or(ContentError::InvalidArtifact("structural ability missing"))?;
    assert_eq!(ability.effects.len(), 3);
    assert_eq!(ability.effects[0], ability.effects[2]);
    assert_eq!(
        ability.effects[0].key().as_str(),
        "oteryn:reference.effect.heal"
    );
    assert_eq!(
        ability.effects[1].key().as_str(),
        "oteryn:reference.effect.project-owned-damage"
    );
    Ok(())
}

#[test]
fn both_structural_edges_reject_wrong_family_missing_and_stale_revision() -> Result<(), ContentError>
{
    let mut wrong_ability_family = source_with_ability_effect_formula()?;
    ability_kind_mut(&mut wrong_ability_family)?.effects[0] = TypedDefinitionRef::new(
        DefinitionFamily::Formula,
        ProductionKey::new("oteryn:reference.formula.project-owned-heal")?,
        DefinitionRevisionRef::new("definition-r1")?,
    );
    assert!(matches!(
        link_reference_playable(wrong_ability_family),
        Err(ContentError::InvalidArtifact(
            "reference-playable ability effect reference must target Effect"
        ))
    ));

    let mut missing_effect = source_with_ability_effect_formula()?;
    ability_kind_mut(&mut missing_effect)?.effects[0] = TypedDefinitionRef::new(
        DefinitionFamily::Effect,
        ProductionKey::new("oteryn:reference.effect.project-owned-missing")?,
        DefinitionRevisionRef::new("definition-r1")?,
    );
    assert!(matches!(
        link_reference_playable(missing_effect),
        Err(ContentError::MissingReference { .. })
    ));

    let mut stale_effect = source_with_ability_effect_formula()?;
    let effect_key = ability_kind_mut(&mut stale_effect)?.effects[0]
        .key()
        .clone();
    ability_kind_mut(&mut stale_effect)?.effects[0] = TypedDefinitionRef::new(
        DefinitionFamily::Effect,
        effect_key,
        DefinitionRevisionRef::new("definition-r2")?,
    );
    assert!(matches!(
        link_reference_playable(stale_effect),
        Err(ContentError::RevisionMismatch(
            "reference-playable definition revision"
        ))
    ));

    let mut wrong_formula_family = source_with_ability_effect_formula()?;
    damage_effect_kind_mut(&mut wrong_formula_family)?.formula = TypedDefinitionRef::new(
        DefinitionFamily::Item,
        ProductionKey::new("oteryn:reference.item.project-owned-formula-mismatch")?,
        DefinitionRevisionRef::new("definition-r1")?,
    );
    assert!(matches!(
        link_reference_playable(wrong_formula_family),
        Err(ContentError::InvalidArtifact(
            "reference-playable effect formula reference must target Formula"
        ))
    ));

    let mut missing_formula = source_with_ability_effect_formula()?;
    damage_effect_kind_mut(&mut missing_formula)?.formula = TypedDefinitionRef::new(
        DefinitionFamily::Formula,
        ProductionKey::new("oteryn:reference.formula.project-owned-missing")?,
        DefinitionRevisionRef::new("definition-r1")?,
    );
    assert!(matches!(
        link_reference_playable(missing_formula),
        Err(ContentError::MissingReference { .. })
    ));

    let mut stale_formula = source_with_ability_effect_formula()?;
    let formula_key = damage_effect_kind_mut(&mut stale_formula)?
        .formula
        .key()
        .clone();
    damage_effect_kind_mut(&mut stale_formula)?.formula = TypedDefinitionRef::new(
        DefinitionFamily::Formula,
        formula_key,
        DefinitionRevisionRef::new("definition-r2")?,
    );
    assert!(matches!(
        link_reference_playable(stale_formula),
        Err(ContentError::RevisionMismatch(
            "reference-playable definition revision"
        ))
    ));
    Ok(())
}

#[test]
fn typed_structural_families_cannot_use_generic_or_client_authority() -> Result<(), ContentError> {
    for family in [
        DefinitionFamily::Ability,
        DefinitionFamily::Effect,
        DefinitionFamily::Formula,
    ] {
        let mut candidate = source()?;
        candidate.definitions.push(ReferenceDefinition {
            definition: TypedDefinitionRef::new(
                family,
                ProductionKey::new(match family {
                    DefinitionFamily::Ability => "oteryn:reference.ability.project-owned-generic",
                    DefinitionFamily::Effect => "oteryn:reference.effect.project-owned-generic",
                    DefinitionFamily::Formula => "oteryn:reference.formula.project-owned-generic",
                    _ => return Err(ContentError::InvalidArtifact("unexpected family")),
                })?,
                DefinitionRevisionRef::new("definition-r1")?,
            ),
            kind: ReferenceDefinitionKind::Generic,
            client_projection: ClientProjectionClass::ServerOnly,
        });
        assert!(matches!(
            link_reference_playable(candidate),
            Err(ContentError::InvalidArtifact(_))
        ));
    }

    let mut client_ability = source_with_ability_effect_formula()?;
    client_ability
        .definitions
        .iter_mut()
        .find(|definition| definition.definition.family() == DefinitionFamily::Ability)
        .ok_or(ContentError::InvalidArtifact("structural ability missing"))?
        .client_projection = ClientProjectionClass::ClientSafe;
    assert!(matches!(
        link_reference_playable(client_ability),
        Err(ContentError::InvalidArtifact(
            "reference-playable ability structure must remain server-only"
        ))
    ));

    let mut client_formula = source_with_ability_effect_formula()?;
    client_formula
        .definitions
        .iter_mut()
        .find(|definition| definition.definition.family() == DefinitionFamily::Formula)
        .ok_or(ContentError::InvalidArtifact("structural formula missing"))?
        .client_projection = ClientProjectionClass::ClientSafe;
    assert!(matches!(
        link_reference_playable(client_formula),
        Err(ContentError::InvalidArtifact(
            "reference-playable formula endpoint must remain server-only"
        ))
    ));
    Ok(())
}

#[test]
fn client_safe_effect_projection_exposes_only_the_effect_family() -> Result<(), ContentError> {
    let canonical = link_reference_playable(source_with_ability_effect_formula()?)?;
    let client = canonical.client_safe_definitions();
    assert!(client.iter().any(|definition| matches!(
        definition.kind,
        ClientSafeDefinitionKind::Effect(ReferenceEffectFamily::Damage)
    )));
    assert!(!client.iter().any(|definition| matches!(
        definition.definition.family(),
        DefinitionFamily::Ability | DefinitionFamily::Formula
    )));
    Ok(())
}
