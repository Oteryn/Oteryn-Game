#![allow(clippy::expect_used, clippy::panic)]

use oteryn_game_server::content::*;

const B1_EVIDENCE: &[u8] = include_bytes!(
    "../../../docs/agents/evidence/OTV2-20260919-content-world-cw2-b1-item-identity-catalog.json"
);
const SECTION_ENTRY_LEN: usize = 48;
const MANIFEST_ENTRY: usize = 24;
const INDEX_ENTRY: usize = MANIFEST_ENTRY + SECTION_ENTRY_LEN;
const BODY_ENTRY: usize = INDEX_ENTRY + SECTION_ENTRY_LEN;

fn limits() -> ProjectEvidenceLimits {
    ProjectEvidenceLimits {
        max_documents: 6,
        max_document_bytes: 32_768,
        max_total_bytes: 65_536,
        max_json_depth: 20,
        max_decoded_fields: 1_024,
        max_string_bytes: 32_768,
        max_locator_bytes: 160,
        max_locator_segments: 8,
        max_reference_records: 1,
        max_import_records: 1,
        max_reimport_states: 2,
    }
}

fn canonical_documents() -> CanonicalProjectDocuments {
    let imported = protected_cw2_b1_vase_import(B1_EVIDENCE).expect("protected one-Item import");
    CanonicalProjectDocuments::from_draft(
        ProjectDraft {
            project_revision: "project-r1".to_owned(),
            package_key: "oteryn:content.world-project".to_owned(),
            semantic_schema_version: "reference-schema-v1".to_owned(),
            licensing_metadata: "PENDING".to_owned(),
            world_id: "0123456789ab70cd8ef0123456789abc".to_owned(),
            coordinate_frame: "global-target-2026-07-28".to_owned(),
            records: vec![imported.record],
            imports: vec![imported.batch],
            metadata: Vec::new(),
        },
        limits(),
    )
    .expect("canonical one-Item project")
}

fn linked_from_enumeration(
    reverse: bool,
) -> Result<CanonicalReferencePlayableContent, ProjectError> {
    let canonical = canonical_documents();
    let mut documents = canonical
        .documents()
        .iter()
        .map(|(locator, bytes)| (locator.clone(), bytes.clone()))
        .collect::<Vec<_>>();
    if reverse {
        documents.reverse();
    }
    ProjectSnapshot::new(documents, limits())?
        .parse(limits())?
        .link()
}

fn compiled() -> CompiledReferencePlayableContent {
    let linked = linked_from_enumeration(false).expect("linked one-Item project");
    compile_reference_playable(&linked).expect("compiled Reference artifact pair")
}

fn full_family_limits() -> ProjectEvidenceLimits {
    ProjectEvidenceLimits {
        max_documents: 8,
        max_document_bytes: 96_000_000,
        max_total_bytes: 160_000_000,
        max_json_depth: 24,
        max_decoded_fields: 2_110_000,
        max_string_bytes: 96_000_000,
        max_locator_bytes: 160,
        max_locator_segments: 8,
        max_reference_records: CW2_B1_FULL_ITEM_FAMILY_COUNT,
        max_import_records: CW2_B1_FULL_ITEM_FAMILY_COUNT,
        max_reimport_states: CW2_B1_FULL_ITEM_FAMILY_COUNT,
    }
}

type NamedItemSemantics = (&'static str, ReferenceItemSemantics, String);

fn typed_family_linked(
    cases: Vec<(&'static str, ReferenceItemSemantics)>,
) -> Result<(CanonicalReferencePlayableContent, Vec<NamedItemSemantics>), Box<dyn std::error::Error>>
{
    let mut imported = protected_cw2_b1_full_item_family_import(B1_EVIDENCE)?;
    let mut selected = Vec::with_capacity(cases.len());
    let mut cases = cases.into_iter();
    for record in &mut imported.records {
        let ProjectReferenceRecord::Item {
            identity,
            semantics,
            ..
        } = record
        else {
            continue;
        };
        let Some((name, value)) = cases.next() else {
            break;
        };
        *semantics = value.clone();
        selected.push((name, value, identity.key.clone()));
    }
    assert!(cases.next().is_none(), "full family carries all Item cases");
    let canonical = CanonicalProjectDocuments::from_draft(
        ProjectDraft {
            project_revision: "project-r1".to_owned(),
            package_key: "oteryn:content.world-project".to_owned(),
            semantic_schema_version: "reference-schema-v1".to_owned(),
            licensing_metadata: "PENDING".to_owned(),
            world_id: "0123456789ab70cd8ef0123456789abc".to_owned(),
            coordinate_frame: "global-target-2026-07-28".to_owned(),
            records: imported.records,
            imports: vec![imported.batch],
            metadata: Vec::new(),
        },
        full_family_limits(),
    )?;
    let linked = canonical
        .into_snapshot(full_family_limits())?
        .parse(full_family_limits())?
        .link()?;
    Ok((linked, selected))
}

fn promoted_family_linked() -> Result<CanonicalReferencePlayableContent, Box<dyn std::error::Error>>
{
    let promoted = protected_cw2_b1_promoted_item_family_import(B1_EVIDENCE)?;
    assert_eq!(
        promoted.promoted_fields,
        ITEM_SEMANTIC_PROMOTION_FIELD_COUNT
    );
    assert_eq!(promoted.promoted_items, ITEM_SEMANTIC_PROMOTION_ITEM_COUNT);
    let canonical = CanonicalProjectDocuments::from_draft(
        ProjectDraft {
            project_revision: "project-r1".to_owned(),
            package_key: "oteryn:content.world-project".to_owned(),
            semantic_schema_version: "reference-schema-v1".to_owned(),
            licensing_metadata: "PENDING".to_owned(),
            world_id: "0123456789ab70cd8ef0123456789abc".to_owned(),
            coordinate_frame: "global-target-2026-07-28".to_owned(),
            records: promoted.family.records,
            imports: vec![promoted.family.batch],
            metadata: Vec::new(),
        },
        full_family_limits(),
    )?;
    Ok(canonical
        .into_snapshot(full_family_limits())?
        .parse(full_family_limits())?
        .link()?)
}

fn promoted_atom_count(semantics: &ReferenceItemSemantics) -> usize {
    use ReferenceItemField::Known;
    let mut count = 0_usize;
    if let Known(value) = &semantics.presentation {
        count += usize::from(matches!(&value.name, Known(_)));
    }
    if let Known(value) = &semantics.weapon {
        count += usize::from(matches!(&value.attack, Known(_)));
        count += usize::from(matches!(&value.defense, Known(_)));
        count += usize::from(matches!(&value.extra_defense, Known(_)));
        count += usize::from(matches!(&value.range, Known(_)));
        count += usize::from(matches!(&value.hit_chance, Known(_)));
    }
    if let Known(value) = &semantics.protection {
        count += usize::from(matches!(&value.armor, Known(_)));
    }
    if let Known(value) = &semantics.charges {
        count += usize::from(matches!(&value.count, Known(_)));
    }
    if let Known(value) = &semantics.container {
        count += usize::from(matches!(&value.capacity, Known(_)));
    }
    count
}

#[test]
fn protected_semantic_promotion_round_trips_exact_69_atoms_through_artifact_v4_server_and_client()
-> Result<(), Box<dyn std::error::Error>> {
    let linked = promoted_family_linked()?;
    assert_eq!(linked.definitions.len(), CW2_B1_FULL_ITEM_FAMILY_COUNT);

    let promoted_items = linked
        .definitions
        .iter()
        .filter(|definition| {
            matches!(
                &definition.kind,
                ReferenceDefinitionKind::Item(item) if promoted_atom_count(&item.semantics) > 0
            )
        })
        .count();
    let promoted_fields = linked
        .definitions
        .iter()
        .map(|definition| match &definition.kind {
            ReferenceDefinitionKind::Item(item) => promoted_atom_count(&item.semantics),
            _ => 0,
        })
        .sum::<usize>();
    assert_eq!(promoted_items, ITEM_SEMANTIC_PROMOTION_ITEM_COUNT);
    assert_eq!(promoted_fields, ITEM_SEMANTIC_PROMOTION_FIELD_COUNT);

    let compiled = compile_reference_playable(&linked)?;
    let repeated = compile_reference_playable(&linked)?;
    assert_eq!(compiled.server_artifact, repeated.server_artifact);
    assert_eq!(compiled.client_artifact, repeated.client_artifact);

    let server = load_reference_playable_artifact(
        &compiled.server_artifact,
        ReferenceArtifactProjection::ServerAuthoritative,
    )?;
    let client = load_reference_playable_artifact(
        &compiled.client_artifact,
        ReferenceArtifactProjection::ClientSafe,
    )?;
    assert_eq!(
        server.artifact_profile_id(),
        "OTERYN_REFERENCE_PLAYABLE_ARTIFACT/v4"
    );
    assert_eq!(
        client.artifact_profile_id(),
        "OTERYN_REFERENCE_PLAYABLE_ARTIFACT/v4"
    );

    let mut server_promoted = 0_usize;
    let mut client_promoted = 0_usize;
    for definition in &linked.definitions {
        let ReferenceDefinitionKind::Item(item) = &definition.kind else {
            continue;
        };
        let expected = &item.semantics;
        let server_item = server
            .lookup_server_item(&definition.definition)?
            .expect("server Item from complete promoted family");
        let client_item = client
            .lookup_client_item(&definition.definition)?
            .expect("client Item from complete promoted family");
        assert_eq!(server_item.semantics, *expected);
        assert_eq!(client_item.semantics, expected.client_projection());
        server_promoted += promoted_atom_count(&server_item.semantics);
        client_promoted += promoted_atom_count(&client_item.semantics);
    }
    assert_eq!(server_promoted, ITEM_SEMANTIC_PROMOTION_FIELD_COUNT);
    assert_eq!(client_promoted, ITEM_SEMANTIC_PROMOTION_FIELD_COUNT);
    Ok(())
}

fn item_target() -> ReferenceItemTarget {
    ReferenceItemTarget::new(CW2_B1_VASE_KEY, CW2_B1_VASE_REVISION).expect("typed target")
}

fn presentation() -> ReferenceItemPresentation {
    ReferenceItemPresentation {
        name: ReferenceItemField::Known("typed item".to_owned()),
        description: ReferenceItemField::Known("synthetic schema witness".to_owned()),
    }
}

#[test]
fn typed_item_v4_representative_families_round_trip_through_project_and_both_projections()
-> Result<(), Box<dyn std::error::Error>> {
    use ReferenceItemField::{Conflict, Known, NotApplicable, Unknown};
    let capabilities = std::array::from_fn(|index| match index % 4 {
        0 => Unknown,
        1 => NotApplicable,
        2 => Conflict,
        _ => Known(false),
    });
    let classification = ReferenceItemClassification {
        item_type: Known(ReferenceItemType::Rune),
        capabilities: Known(capabilities),
    };
    let equipment = ReferenceItemEquipment {
        patterns: Known(vec![ReferenceEquipmentPattern {
            pattern_id: 1,
            primary_slot: Known(ReferenceEquipmentSlot::Weapon),
            additional_reserved_slots: Known(vec![ReferenceEquipmentSlot::Shield]),
            mutually_exclusive_groups: Known(vec![ReferenceItemGroupKey::new(
                "oteryn:equipment-group.two-handed",
            )?]),
            vocations: Known(vec![ReferenceBaseVocation::Paladin]),
            level: Known(20),
            compatibility_rule: Unknown,
        }]),
    };
    let target = item_target();
    let mut cases = Vec::new();

    let melee = ReferenceItemSemantics {
        presentation: Known(presentation()),
        classification: Known(classification.clone()),
        weapon: Known(ReferenceItemWeapon {
            weapon_type: Known(ReferenceWeaponType::Sword),
            attack: Known(ReferenceSignedPoints(42)),
            defense: Known(ReferenceSignedPoints(20)),
            extra_defense: Known(ReferenceSignedPoints(0)),
            range: Known(ReferenceCells(1)),
            hit_chance: Known(ReferenceRationalPercent::new(1, 1)?),
            max_hit_chance: Known(ReferenceRationalPercent::new(1, 1)?),
            ammunition: NotApplicable,
            elemental: Known(Vec::new()),
        }),
        ..Default::default()
    };
    cases.push(("melee_weapon", melee));

    let distance = ReferenceItemSemantics {
        presentation: Known(presentation()),
        weapon: Known(ReferenceItemWeapon {
            weapon_type: Known(ReferenceWeaponType::Distance),
            attack: Known(ReferenceSignedPoints(35)),
            defense: Unknown,
            extra_defense: Unknown,
            range: Known(ReferenceCells(7)),
            hit_chance: Known(ReferenceRationalPercent::new(9, 10)?),
            max_hit_chance: Known(ReferenceRationalPercent::new(1, 1)?),
            ammunition: Known(ReferenceAmmoType::Arrow),
            elemental: Unknown,
        }),
        ..Default::default()
    };
    cases.push(("distance_weapon", distance));
    cases.push((
        "armor_equipment",
        ReferenceItemSemantics {
            presentation: Known(presentation()),
            equipment: Known(equipment.clone()),
            protection: Known(ReferenceItemProtection {
                armor: Known(ReferenceSignedPoints(12)),
                resistances: Unknown,
            }),
            ..Default::default()
        },
    ));
    cases.push((
        "container",
        ReferenceItemSemantics {
            presentation: Known(presentation()),
            container: Known(ReferenceItemContainer {
                capacity: Known(20),
            }),
            ..Default::default()
        },
    ));
    cases.push((
        "charges_consumable",
        ReferenceItemSemantics {
            charges: Known(ReferenceItemCharges { count: Known(5) }),
            temporal: Known(ReferenceItemTemporal {
                consumption_mode: Known(ReferenceTemporalMode::AuthoritativeActiveTimeBudget),
                duration: Known(ReferenceMilliseconds(60_000)),
                stop_duration: Known(false),
                decay_target: Unknown,
            }),
            ..Default::default()
        },
    ));
    cases.push((
        "rune_use_item",
        ReferenceItemSemantics {
            classification: Known(classification.clone()),
            use_transform: Known(ReferenceItemUseTransform {
                targets: (1..=10)
                    .map(|kind| {
                        Ok(ReferenceTransformTarget {
                            kind: ReferenceTransformKind::from_wire(kind)?,
                            target: if kind == ReferenceTransformKind::Use.wire() {
                                Known(target.clone())
                            } else {
                                Unknown
                            },
                        })
                    })
                    .collect::<Result<Vec<_>, ContentError>>()?,
            }),
            ..Default::default()
        },
    ));
    cases.push((
        "material_loot",
        ReferenceItemSemantics {
            presentation: Known(presentation()),
            classification: Conflict,
            physical: Known(ReferenceItemPhysical {
                weight: Known(0),
                movable: Known(false),
                pickupable: Known(true),
            }),
            ..Default::default()
        },
    ));
    cases.push((
        "presentation_only",
        ReferenceItemSemantics {
            presentation: Known(presentation()),
            classification: Known(classification),
            ..Default::default()
        },
    ));
    cases.push((
        "resistance_modifiers",
        ReferenceItemSemantics {
            protection: Known(ReferenceItemProtection {
                armor: Known(ReferenceSignedPoints(0)),
                resistances: Known(vec![ReferenceResistance {
                    kind: ReferenceResistanceKind::Fire,
                    percent: Known(ReferenceRationalPercent::new(1, 10)?),
                }]),
            }),
            skill_modifiers: Known(ReferenceItemSkillModifiers {
                modifiers: Known(vec![ReferenceModifierBinding {
                    kind: ReferenceSkillModifierKind::CriticalHitChance,
                    target_domain: Unknown,
                    evaluation_phase: Unknown,
                    priority: Known(0),
                    parameter: Known(ReferenceModifierParameter::RationalPercent(
                        ReferenceRationalPercent::new(1, 100)?,
                    )),
                }]),
            }),
            ..Default::default()
        },
    ));
    cases.push((
        "imbuement_slots",
        ReferenceItemSemantics {
            equipment: Known(equipment),
            imbuement: Known(ReferenceItemImbuement {
                slot_count: Known(3),
                allowed_family_tiers: Known(vec![ReferenceImbuementAllowance {
                    family: ReferenceImbuementFamily::CriticalHit,
                    tier: ReferenceImbuementTier::Three,
                }]),
                excluded_families: Known(Vec::new()),
            }),
            ..Default::default()
        },
    ));
    cases.push((
        "transform_decay_target",
        ReferenceItemSemantics {
            temporal: Known(ReferenceItemTemporal {
                consumption_mode: Known(ReferenceTemporalMode::DurableAbsoluteDeadline),
                duration: Known(ReferenceMilliseconds(0)),
                stop_duration: Known(false),
                decay_target: Known(target),
            }),
            ..Default::default()
        },
    ));

    assert_eq!(cases.len(), 11);
    let (linked, cases) = typed_family_linked(cases)?;
    assert_eq!(linked.definitions.len(), CW2_B1_FULL_ITEM_FAMILY_COUNT);
    let compiled = compile_reference_playable(&linked)?;
    let repeated = compile_reference_playable(&linked)?;
    assert_eq!(compiled.server_artifact, repeated.server_artifact);
    assert_eq!(compiled.client_artifact, repeated.client_artifact);
    let server = load_reference_playable_artifact(
        &compiled.server_artifact,
        ReferenceArtifactProjection::ServerAuthoritative,
    )?;
    let client = load_reference_playable_artifact(
        &compiled.client_artifact,
        ReferenceArtifactProjection::ClientSafe,
    )?;
    for (name, semantics, key) in cases {
        let identity = linked
            .definitions
            .iter()
            .find(|definition| definition.definition.key().as_str() == key)
            .map(|definition| &definition.definition)
            .expect(name);
        assert_eq!(
            server.artifact_profile_id(),
            "OTERYN_REFERENCE_PLAYABLE_ARTIFACT/v4",
            "{name}"
        );
        assert_eq!(
            server.lookup_server_item(identity)?.expect(name).semantics,
            semantics,
            "{name}"
        );
        assert_eq!(
            client.lookup_client_item(identity)?.expect(name).semantics,
            semantics.client_projection(),
            "{name}",
        );
    }

    let mut below = linked.clone();
    below.definitions.pop();
    assert!(matches!(
        compile_reference_playable(&below),
        Err(ContentError::LimitExceeded {
            resource: "Reference playable definitions",
            actual: 38_156,
            limit: 38_157,
        })
    ));
    let mut above = linked.clone();
    above
        .definitions
        .push(above.definitions.last().expect("last definition").clone());
    assert!(matches!(
        compile_reference_playable(&above),
        Err(ContentError::LimitExceeded {
            resource: "Reference playable definitions",
            actual: 38_158,
            limit: 38_157,
        })
    ));

    for projection in [
        ReferenceArtifactProjection::ServerAuthoritative,
        ReferenceArtifactProjection::ClientSafe,
    ] {
        let source = match projection {
            ReferenceArtifactProjection::ServerAuthoritative => &compiled.server_artifact,
            ReferenceArtifactProjection::ClientSafe => &compiled.client_artifact,
        };
        for invalid_count in [38_156, 38_158] {
            let mut malformed = source.clone();
            write_u32(&mut malformed, INDEX_ENTRY + 12, invalid_count);
            write_u32(&mut malformed, BODY_ENTRY + 12, invalid_count);
            assert!(matches!(
                load_reference_playable_artifact(&malformed, projection),
                Err(ContentError::InvalidArtifact(
                    "Reference artifact section cardinality mismatch"
                ))
            ));
        }
    }
    Ok(())
}

fn read_u32(bytes: &[u8], offset: usize) -> usize {
    usize::try_from(u32::from_be_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("four byte field"),
    ))
    .expect("u32 fits usize")
}

fn write_u16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}

fn decode_digest(value: &str) -> [u8; 32] {
    fn nibble(value: u8) -> u8 {
        match value {
            b'0'..=b'9' => value - b'0',
            b'a'..=b'f' => value - b'a' + 10,
            unexpected => panic!("unexpected digest byte {unexpected}"),
        }
    }

    let mut decoded = [0_u8; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        decoded[index] = (nibble(pair[0]) << 4) | nibble(pair[1]);
    }
    decoded
}

fn reseal_whole(bytes: &mut [u8]) {
    let payload_end = read_u32(bytes, 20);
    let digest = decode_digest(&world_project_sha256(&bytes[..payload_end]));
    bytes[payload_end..payload_end + 32].copy_from_slice(&digest);
}

fn reseal_section(bytes: &mut [u8], entry: usize) {
    let offset = read_u32(bytes, entry + 4);
    let length = read_u32(bytes, entry + 8);
    let digest = decode_digest(&world_project_sha256(&bytes[offset..offset + length]));
    bytes[entry + 16..entry + SECTION_ENTRY_LEN].copy_from_slice(&digest);
    reseal_whole(bytes);
}

fn assert_rejected(bytes: &[u8], projection: ReferenceArtifactProjection) {
    assert!(load_reference_playable_artifact(bytes, projection).is_err());
}

#[test]
fn protected_project_links_compiles_loads_and_stages_with_semantic_equivalence()
-> Result<(), Box<dyn std::error::Error>> {
    let linked = linked_from_enumeration(false)?;
    let compiled = compile_reference_playable(&linked)?;
    assert_eq!(compiled.server_artifact.len(), 788);
    assert_eq!(compiled.client_artifact.len(), 776);
    assert_eq!(
        compiled.server_digest(),
        [
            199, 192, 106, 212, 18, 62, 17, 244, 10, 33, 134, 27, 72, 253, 155, 193, 227, 163, 120,
            28, 30, 255, 26, 229, 213, 63, 221, 149, 98, 111, 20, 125,
        ]
    );
    assert_eq!(
        compiled.client_digest(),
        [
            164, 20, 224, 125, 140, 136, 44, 180, 114, 134, 89, 86, 211, 126, 223, 187, 35, 87,
            101, 223, 236, 254, 54, 158, 25, 54, 29, 149, 66, 144, 186, 252,
        ]
    );
    assert!(compiled.server_artifact.len() <= REFERENCE_PLAYABLE_MAX_SERVER_ARTIFACT_BYTES);
    assert!(compiled.client_artifact.len() <= REFERENCE_PLAYABLE_MAX_CLIENT_ARTIFACT_BYTES);
    assert!(
        compiled.server_artifact.len() + compiled.client_artifact.len()
            <= REFERENCE_PLAYABLE_MAX_GENERATION_PAIR_BYTES
    );
    assert_eq!(
        compiled.expectation().package_provenance_digest(),
        linked
            .package_manifest
            .package_provenance_digest()?
            .as_str()
    );

    let identity = linked.definitions[0].definition.clone();
    let server = load_reference_playable_artifact(
        &compiled.server_artifact,
        ReferenceArtifactProjection::ServerAuthoritative,
    )?;
    let client = load_reference_playable_artifact(
        &compiled.client_artifact,
        ReferenceArtifactProjection::ClientSafe,
    )?;
    assert_eq!(server.indexed_identity(), &identity);
    assert_eq!(client.indexed_identity(), &identity);

    let authoritative = server
        .lookup_server_item(&identity)?
        .expect("authoritative Item lookup");
    assert_eq!(
        authoritative.physical_class,
        ReferenceItemPhysicalClass::Physical
    );
    assert!(authoritative.materializable);
    assert_eq!(
        authoritative.stack_class,
        ReferenceItemStackClass::NonStackable
    );
    assert_eq!(
        authoritative.legal_destinations,
        [ReferenceItemDestination::CharacterInventory]
    );
    let projected = client
        .lookup_client_item(&identity)?
        .expect("client Item lookup");
    assert_eq!(projected.physical_class, authoritative.physical_class);
    assert_eq!(projected.stack_class, authoritative.stack_class);

    let missing = TypedDefinitionRef::new(
        DefinitionFamily::Item,
        ProductionKey::new("oteryn:item.decor.missing")?,
        DefinitionRevisionRef::new("definition-r1")?,
    );
    assert_eq!(server.lookup_server_item(&missing)?, None);
    assert_eq!(client.lookup_client_item(&missing)?, None);

    let staged = stage_reference_playable(
        &compiled.server_artifact,
        &compiled.client_artifact,
        compiled.expectation(),
    )?;
    assert_eq!(staged.identity().item_identity(), &identity);
    assert_eq!(
        staged.identity().package_provenance_digest(),
        compiled.expectation().package_provenance_digest()
    );
    assert_eq!(
        staged.identity().server_artifact_digest(),
        compiled.server_digest()
    );
    assert_eq!(
        staged.identity().client_artifact_digest(),
        compiled.client_digest()
    );

    let controller = ContentActivationController::new();
    assert!(controller.staged_identity().is_none());
    assert!(controller.active().is_none());
    assert!(!controller.is_ready());
    Ok(())
}

#[test]
fn compile_is_repeatable_and_input_enumeration_independent()
-> Result<(), Box<dyn std::error::Error>> {
    let forward = compile_reference_playable(&linked_from_enumeration(false)?)?;
    let repeated = compile_reference_playable(&linked_from_enumeration(false)?)?;
    let reversed = compile_reference_playable(&linked_from_enumeration(true)?)?;
    assert_eq!(forward.server_artifact, repeated.server_artifact);
    assert_eq!(forward.client_artifact, repeated.client_artifact);
    assert_eq!(forward.server_artifact, reversed.server_artifact);
    assert_eq!(forward.client_artifact, reversed.client_artifact);
    assert_eq!(forward.expectation(), reversed.expectation());
    Ok(())
}

#[test]
fn typed_semantic_identity_survives_section_table_reordering()
-> Result<(), Box<dyn std::error::Error>> {
    let compiled = compiled();
    let expected_identity = load_reference_playable_artifact(
        &compiled.server_artifact,
        ReferenceArtifactProjection::ServerAuthoritative,
    )?
    .indexed_identity()
    .clone();
    let mut reordered = compiled.server_artifact.clone();
    let first: [u8; SECTION_ENTRY_LEN] = reordered
        [MANIFEST_ENTRY..MANIFEST_ENTRY + SECTION_ENTRY_LEN]
        .try_into()
        .expect("manifest entry");
    let last: [u8; SECTION_ENTRY_LEN] = reordered[BODY_ENTRY..BODY_ENTRY + SECTION_ENTRY_LEN]
        .try_into()
        .expect("body entry");
    reordered[MANIFEST_ENTRY..MANIFEST_ENTRY + SECTION_ENTRY_LEN].copy_from_slice(&last);
    reordered[BODY_ENTRY..BODY_ENTRY + SECTION_ENTRY_LEN].copy_from_slice(&first);
    reseal_whole(&mut reordered);

    let loaded = load_reference_playable_artifact(
        &reordered,
        ReferenceArtifactProjection::ServerAuthoritative,
    )?;
    assert_eq!(loaded.indexed_identity(), &expected_identity);
    assert!(loaded.lookup_server_item(&expected_identity)?.is_some());
    Ok(())
}

#[test]
fn malformed_sections_corruption_cross_profile_and_trailing_bytes_fail_closed() {
    let compiled = compiled();
    let projection = ReferenceArtifactProjection::ServerAuthoritative;

    let mut duplicate = compiled.server_artifact.clone();
    write_u16(&mut duplicate, INDEX_ENTRY, 1);
    reseal_whole(&mut duplicate);
    assert_rejected(&duplicate, projection);

    let mut missing = compiled.server_artifact.clone();
    write_u16(&mut missing, MANIFEST_ENTRY, 99);
    write_u16(&mut missing, MANIFEST_ENTRY + 2, 0);
    reseal_whole(&mut missing);
    assert_rejected(&missing, projection);

    let mut unknown = compiled.server_artifact.clone();
    write_u16(&mut unknown, INDEX_ENTRY, 99);
    reseal_whole(&mut unknown);
    assert_rejected(&unknown, projection);

    let body_offset = read_u32(&compiled.server_artifact, BODY_ENTRY + 4);
    let mut overlap = compiled.server_artifact.clone();
    write_u32(
        &mut overlap,
        BODY_ENTRY + 4,
        u32::try_from(body_offset - 1).expect("body offset"),
    );
    reseal_whole(&mut overlap);
    assert_rejected(&overlap, projection);

    let mut gap = compiled.server_artifact.clone();
    write_u32(
        &mut gap,
        BODY_ENTRY + 4,
        u32::try_from(body_offset + 1).expect("body offset"),
    );
    reseal_whole(&mut gap);
    assert_rejected(&gap, projection);

    let mut section_corrupt = compiled.server_artifact.clone();
    section_corrupt[body_offset] ^= 0x01;
    reseal_whole(&mut section_corrupt);
    assert!(matches!(
        load_reference_playable_artifact(&section_corrupt, projection),
        Err(ContentError::IntegrityMismatch(
            "Reference artifact section"
        ))
    ));

    let mut whole_corrupt = compiled.server_artifact.clone();
    let last = whole_corrupt.len() - 1;
    whole_corrupt[last] ^= 0x01;
    assert!(matches!(
        load_reference_playable_artifact(&whole_corrupt, projection),
        Err(ContentError::IntegrityMismatch("Reference artifact"))
    ));

    let mut cross_profile = compiled.server_artifact.clone();
    cross_profile[..8].copy_from_slice(b"OTFPC01\0");
    reseal_whole(&mut cross_profile);
    assert!(matches!(
        load_reference_playable_artifact(&cross_profile, projection),
        Err(ContentError::InvalidMagic)
    ));

    let mut trailing = compiled.server_artifact.clone();
    trailing.push(0);
    assert_rejected(&trailing, projection);
}

#[test]
fn size_max_plus_one_and_encoded_length_overflow_values_reject_before_decode() {
    let server_at_max = vec![0_u8; REFERENCE_PLAYABLE_MAX_SERVER_ARTIFACT_BYTES];
    assert!(matches!(
        load_reference_playable_artifact(
            &server_at_max,
            ReferenceArtifactProjection::ServerAuthoritative,
        ),
        Err(ContentError::InvalidMagic)
    ));
    let server_oversized = vec![0_u8; REFERENCE_PLAYABLE_MAX_SERVER_ARTIFACT_BYTES + 1];
    assert!(matches!(
        load_reference_playable_artifact(
            &server_oversized,
            ReferenceArtifactProjection::ServerAuthoritative,
        ),
        Err(ContentError::LimitExceeded {
            resource: "Reference artifact bytes",
            actual,
            limit: REFERENCE_PLAYABLE_MAX_SERVER_ARTIFACT_BYTES,
        }) if actual == REFERENCE_PLAYABLE_MAX_SERVER_ARTIFACT_BYTES + 1
    ));
    let client_at_max = vec![0_u8; REFERENCE_PLAYABLE_MAX_CLIENT_ARTIFACT_BYTES];
    assert!(matches!(
        load_reference_playable_artifact(&client_at_max, ReferenceArtifactProjection::ClientSafe,),
        Err(ContentError::InvalidMagic)
    ));
    let client_oversized = vec![0_u8; REFERENCE_PLAYABLE_MAX_CLIENT_ARTIFACT_BYTES + 1];
    assert!(matches!(
        load_reference_playable_artifact(
            &client_oversized,
            ReferenceArtifactProjection::ClientSafe,
        ),
        Err(ContentError::LimitExceeded {
            resource: "Reference artifact bytes",
            actual,
            limit: REFERENCE_PLAYABLE_MAX_CLIENT_ARTIFACT_BYTES,
        }) if actual == REFERENCE_PLAYABLE_MAX_CLIENT_ARTIFACT_BYTES + 1
    ));

    let compiled = compiled();
    let mut payload_overflow = compiled.server_artifact.clone();
    write_u32(&mut payload_overflow, 20, u32::MAX);
    assert_rejected(
        &payload_overflow,
        ReferenceArtifactProjection::ServerAuthoritative,
    );

    let mut range_overflow = compiled.server_artifact.clone();
    write_u32(&mut range_overflow, BODY_ENTRY + 4, u32::MAX);
    write_u32(&mut range_overflow, BODY_ENTRY + 8, u32::MAX);
    reseal_whole(&mut range_overflow);
    assert_rejected(
        &range_overflow,
        ReferenceArtifactProjection::ServerAuthoritative,
    );
}

#[test]
fn client_body_is_a_positive_allowlist_and_pair_mismatch_rejects() {
    let compiled = compiled();
    let identity = load_reference_playable_artifact(
        &compiled.client_artifact,
        ReferenceArtifactProjection::ClientSafe,
    )
    .expect("client load")
    .indexed_identity()
    .clone();
    let client = load_reference_playable_artifact(
        &compiled.client_artifact,
        ReferenceArtifactProjection::ClientSafe,
    )
    .expect("client load");
    let projected = client
        .lookup_client_item(&identity)
        .expect("client lookup")
        .expect("client Item");
    assert_eq!(
        projected.physical_class,
        ReferenceItemPhysicalClass::Physical
    );
    assert_eq!(projected.stack_class, ReferenceItemStackClass::NonStackable);

    let mut server_as_client = compiled.server_artifact.clone();
    server_as_client[12] = 2;
    reseal_whole(&mut server_as_client);
    assert!(
        stage_reference_playable(
            &compiled.server_artifact,
            &server_as_client,
            compiled.expectation(),
        )
        .is_err()
    );

    let mut client_body = compiled.client_artifact.clone();
    let body_offset = read_u32(&client_body, BODY_ENTRY + 4);
    client_body[body_offset] = 2;
    reseal_section(&mut client_body, BODY_ENTRY);
    assert!(
        stage_reference_playable(
            &compiled.server_artifact,
            &client_body,
            compiled.expectation(),
        )
        .is_err()
    );
}

#[test]
fn exact_public_limits_remain_the_owner_accepted_one_item_values() {
    assert_eq!(REFERENCE_PLAYABLE_MAX_SERVER_ARTIFACT_BYTES, 8_715);
    assert_eq!(REFERENCE_PLAYABLE_MAX_CLIENT_ARTIFACT_BYTES, 8_675);
    assert_eq!(REFERENCE_PLAYABLE_MAX_GENERATION_PAIR_BYTES, 17_390);
    assert_eq!(
        OTERYN_REFERENCE_PLAYABLE_ARTIFACT_PROFILE_ID,
        "OTERYN_REFERENCE_PLAYABLE_ARTIFACT/v1"
    );
}
