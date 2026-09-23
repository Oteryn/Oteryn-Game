#![allow(clippy::expect_used)]

use oteryn_game_server::content::*;
use serde_json::{Value, json};

fn limits() -> ProjectEvidenceLimits {
    ProjectEvidenceLimits {
        max_documents: 12,
        max_document_bytes: 16_384,
        max_total_bytes: 65_536,
        max_json_depth: 20,
        max_decoded_fields: 1_024,
        max_string_bytes: 32_768,
        max_locator_bytes: 160,
        max_locator_segments: 8,
        max_reference_records: 32,
        max_import_records: 8,
        max_reimport_states: 8,
    }
}

fn core() -> ProjectDraft {
    ProjectDraft {
        project_revision: "project-r1".into(),
        package_key: "oteryn:content.world-project".into(),
        semantic_schema_version: "reference-schema-v1".into(),
        licensing_metadata: "license:project-owned-v1".into(),
        world_id: "0123456789ab70cd8ef0123456789abc".into(),
        coordinate_frame: "global-target-2026-07-28".into(),
        records: vec![ProjectReferenceRecord::Generic {
            identity: DefinitionIdentityDocument {
                family: "Presentation".into(),
                key: "oteryn:reference.presentation.courier".into(),
                revision: "definition-r1".into(),
            },
            client_projection: ProjectionDocument::ClientSafe,
        }],
        imports: vec![],
        metadata: vec![],
    }
}

fn identity(key: &str) -> ProjectV2Identity {
    ProjectV2Identity {
        key: format!("oteryn:content.{key}"),
        revision: "definition-r1".into(),
    }
}

fn reference(family: ProjectV2Family, key: &str) -> ProjectV2DefinitionRef {
    ProjectV2DefinitionRef {
        family,
        key: key.into(),
        revision: "definition-r1".into(),
    }
}

fn candidate() -> ProjectV2Draft {
    let npc = reference(ProjectV2Family::Npc, "oteryn:content.npc.courier");
    let presentation = reference(
        ProjectV2Family::Presentation,
        "oteryn:reference.presentation.courier",
    );
    let asset = ProjectV2AssetRef {
        key: "oteryn:asset.courier".into(),
        revision: "asset-r1".into(),
    };
    ProjectV2Draft {
        core: core(),
        state: ProjectV2State {
            // Deliberately unordered: the writer normalizes canonical identity order.
            declarations: vec![
                ProjectV2Declaration::Npc {
                    identity: identity("npc.courier"),
                    presentation: Some(presentation.clone()),
                    behavior: None,
                    dialogue: Some(reference(
                        ProjectV2Family::Dialogue,
                        "oteryn:content.dialogue.courier",
                    )),
                    services: vec![reference(
                        ProjectV2Family::Service,
                        "oteryn:content.service.courier",
                    )],
                    fields: vec![],
                },
                ProjectV2Declaration::WorldObject {
                    identity: identity("object.sign"),
                    presentation: Some(presentation.clone()),
                    fields: vec![],
                },
                ProjectV2Declaration::Dialogue {
                    identity: identity("dialogue.courier"),
                    fields: vec![ProjectV2CandidateField {
                        field_path: "oteryn:source.dialogue-text".into(),
                        value: ProjectV2CandidateValue::Text("hello".into()),
                    }],
                },
                ProjectV2Declaration::Service {
                    identity: identity("service.courier"),
                    offers: vec![],
                    recipes: vec![],
                    fields: vec![],
                },
                ProjectV2Declaration::Interaction {
                    identity: identity("interaction.courier"),
                    fields: vec![],
                },
                ProjectV2Declaration::Quest {
                    identity: identity("quest.courier"),
                    fields: vec![ProjectV2CandidateField {
                        field_path: "oteryn:source.stage-id".into(),
                        value: ProjectV2CandidateValue::SourceId(11),
                    }],
                },
                ProjectV2Declaration::Transition {
                    identity: identity("transition.courier"),
                    fields: vec![],
                },
                ProjectV2Declaration::House {
                    identity: identity("house.courier"),
                    fields: vec![],
                },
                ProjectV2Declaration::Encounter {
                    identity: identity("encounter.courier"),
                    fields: vec![],
                },
            ],
            item_authoring: vec![],
            authoring_profiles: vec![],
            worlds: vec![ProjectV2World {
                key: "oteryn:world.reference".into(),
                world_id: core().world_id,
                coordinate_frame: core().coordinate_frame,
                bounds: ProjectV2Bounds {
                    min_x: 0,
                    min_y: 0,
                    max_x_exclusive: 256,
                    max_y_exclusive: 256,
                },
                floors: vec![7],
            }],
            placements: vec![ProjectV2Placement {
                key: "oteryn:placement.courier".into(),
                world: "oteryn:world.reference".into(),
                map_revision: "map-r1".into(),
                definition: npc.clone(),
                area: None,
                document: None,
                parent_placement: None,
                coordinate_frame: "global-target-2026-07-28".into(),
                x: 100,
                y: 200,
                floor: 7,
                presentation_order: ProjectV2PresentationOrder { plane: 0, order: 0 },
                disposition: ProjectV2Disposition::CandidateOnly,
            }],
            appearance_bindings: vec![ProjectV2AppearanceBinding {
                presentation: presentation.clone(),
                asset: asset.clone(),
                disposition: ProjectV2Disposition::CandidateOnly,
            }],
            assets: vec![ProjectV2Asset {
                identity: asset,
                sha256: "97fbfe027f93834bfaef365e4271dbb56b479ba29528e3f00a1b046aae0a7491".into(),
            }],
            sources: vec![],
            editor: vec![ProjectV2EditorEntry {
                target: npc,
                display_name: "Courier".into(),
                description: "Editor only".into(),
                categories: vec!["npc".into()],
                notes: vec!["Candidate".into()],
                aliases: vec!["Messenger".into(), "Courier".into()],
                tags: vec!["oteryn:editor.npc".into()],
            }],
        },
    }
}

fn canonical(value: &Value) -> Vec<u8> {
    let mut bytes = serde_json::to_vec(value).expect("encode JSON");
    bytes.push(b'\n');
    bytes
}

fn rebind_manifest_and_lock(
    documents: &mut std::collections::BTreeMap<String, Vec<u8>>,
    manifest: &Value,
) {
    let manifest_bytes = canonical(manifest);
    let package = PackageManifestBinding::new(
        ProductionKey::new(manifest["package_key"].as_str().expect("package key")).expect("key"),
        ProductionAtom::new(
            "revision",
            manifest["package_revision"].as_str().expect("revision"),
        )
        .expect("revision"),
        ProductionAtom::new(
            "schema",
            manifest["semantic_schema_version"]
                .as_str()
                .expect("schema"),
        )
        .expect("schema"),
        ProductionAtom::new(
            "license",
            manifest["licensing_metadata"].as_str().expect("license"),
        )
        .expect("license"),
        Sha256HexDigest::new(&world_project_sha256(&manifest_bytes)).expect("digest"),
    );
    let mut lock: Value = serde_json::from_slice(&documents["content.lock.json"]).expect("lock");
    lock["entries"][0]["package_provenance_digest"] = json!(
        package
            .package_provenance_digest()
            .expect("provenance")
            .as_str()
    );
    let lock_bytes = canonical(&lock);
    let mut root: Value = serde_json::from_slice(&documents["project.json"]).expect("root");
    root["manifest_sha256"] = json!(world_project_sha256(&manifest_bytes));
    root["content_lock_sha256"] = json!(world_project_sha256(&lock_bytes));
    documents.insert("manifest.json".into(), manifest_bytes);
    documents.insert("content.lock.json".into(), lock_bytes);
    documents.insert("project.json".into(), canonical(&root));
}

#[test]
fn v1_migrates_explicitly_and_retains_its_original_six_document_wire_format() {
    let original = CanonicalProjectDocuments::from_draft(core(), limits()).expect("v1 documents");
    assert_eq!(original.documents().len(), 6);
    let parsed = ProjectSnapshot::new(original.documents().clone(), limits())
        .expect("admit v1")
        .parse(limits())
        .expect("parse v1");
    assert!(parsed.v2().is_none());
    assert_eq!(
        parsed
            .canonical_documents(limits())
            .expect("v1 rewrite")
            .documents(),
        original.documents()
    );

    let migrated = CanonicalProjectDocuments::from_v2_draft(parsed.migrate_to_v2(), limits())
        .expect("explicit migration");
    assert_eq!(migrated.documents().len(), 11);
    let v2 = ProjectSnapshot::new(migrated.documents().clone(), limits())
        .expect("admit v2")
        .parse(limits())
        .expect("parse v2");
    assert_eq!(v2.v2(), Some(&ProjectV2State::default()));
    assert_eq!(
        v2.canonical_documents(limits())
            .expect("v2 rewrite")
            .documents(),
        migrated.documents()
    );
    assert_eq!(
        v2.lower_reference_source()
            .expect("reference projection")
            .definitions
            .len(),
        1
    );
}

#[test]
fn all_v2_declarative_families_round_trip_without_lowering_candidates() {
    let documents =
        CanonicalProjectDocuments::from_v2_draft(candidate(), limits()).expect("v2 documents");
    let project = ProjectSnapshot::new(documents.documents().clone(), limits())
        .expect("admit v2")
        .parse(limits())
        .expect("parse v2");
    let state = project.v2().expect("v2 state");
    assert_eq!(state.declarations.len(), 9);
    assert_eq!(state.placements.len(), 1);
    assert_eq!(state.worlds[0].floors, [7]);
    assert_eq!(state.editor[0].aliases, ["Courier", "Messenger"]);
    assert_eq!(
        project
            .canonical_documents(limits())
            .expect("rewrite")
            .documents(),
        documents.documents()
    );
    let reference = project
        .lower_reference_source()
        .expect("reference projection");
    assert_eq!(reference.definitions.len(), 1);
    assert!(reference.placements.is_empty());
    assert!(reference.transitions.is_empty());
    project
        .link()
        .expect("server-authoritative Reference linker still works");
}

#[test]
fn typed_references_and_author_aliases_fail_closed() {
    let mut wrong_family = candidate();
    if let ProjectV2Declaration::Npc { dialogue, .. } = &mut wrong_family.state.declarations[0] {
        dialogue.as_mut().expect("dialogue").family = ProjectV2Family::Service;
    }
    assert!(CanonicalProjectDocuments::from_v2_draft(wrong_family, limits()).is_err());

    let mut missing_revision = candidate();
    missing_revision.state.placements[0].definition.revision = "missing-r1".into();
    assert!(CanonicalProjectDocuments::from_v2_draft(missing_revision, limits()).is_err());

    let mut duplicate_alias = candidate();
    duplicate_alias.state.editor[0]
        .aliases
        .push("Courier".into());
    assert!(CanonicalProjectDocuments::from_v2_draft(duplicate_alias, limits()).is_err());

    let mut invalid_tag = candidate();
    invalid_tag.state.editor[0].tags.push("quest-reward".into());
    assert!(CanonicalProjectDocuments::from_v2_draft(invalid_tag, limits()).is_err());

    let mut duplicate_field = candidate();
    if let ProjectV2Declaration::Dialogue { fields, .. } =
        &mut duplicate_field.state.declarations[2]
    {
        fields.push(fields[0].clone());
    }
    assert!(CanonicalProjectDocuments::from_v2_draft(duplicate_field, limits()).is_err());

    let mut orphan_source = candidate();
    orphan_source.state.sources.push(ProjectV2Source {
        key: "oteryn:source.reference".into(),
        import_batch_id: "absent".into(),
        revision: "source-r1".into(),
        sha256: "97fbfe027f93834bfaef365e4271dbb56b479ba29528e3f00a1b046aae0a7491".into(),
        evidence: ProjectV2EvidenceClass::Unknown,
    });
    assert!(CanonicalProjectDocuments::from_v2_draft(orphan_source, limits()).is_err());
}

#[test]
fn spatial_bounds_floors_and_presentation_order_follow_the_native_contract() {
    let mut out_of_bounds = candidate();
    out_of_bounds.state.placements[0].x = 256;
    assert!(CanonicalProjectDocuments::from_v2_draft(out_of_bounds, limits()).is_err());

    let mut unknown_floor = candidate();
    unknown_floor.state.placements[0].floor = 6;
    assert!(CanonicalProjectDocuments::from_v2_draft(unknown_floor, limits()).is_err());

    let mut unsorted_floors = candidate();
    unsorted_floors.state.worlds[0].floors = vec![7, 6];
    assert!(CanonicalProjectDocuments::from_v2_draft(unsorted_floors, limits()).is_err());

    let mut duplicate_order = candidate();
    let mut second = duplicate_order.state.placements[0].clone();
    second.key = "oteryn:placement.second".into();
    duplicate_order.state.placements.push(second);
    assert!(CanonicalProjectDocuments::from_v2_draft(duplicate_order, limits()).is_err());
}

#[test]
fn v2_inventory_digest_and_bounded_document_count_are_enforced() {
    let documents =
        CanonicalProjectDocuments::from_v2_draft(candidate(), limits()).expect("v2 documents");
    let mut changed = documents.documents().clone();
    changed
        .get_mut("assets/catalog.json")
        .expect("asset document")
        .push(b' ');
    assert!(
        ProjectSnapshot::new(changed, limits())
            .expect("admit")
            .parse(limits())
            .is_err()
    );

    let mut too_few = limits();
    too_few.max_documents = 10;
    assert!(CanonicalProjectDocuments::from_v2_draft(candidate(), too_few).is_err());
}

#[test]
fn v2_role_and_strict_schema_reject_undeclared_authority() {
    let documents =
        CanonicalProjectDocuments::from_v2_draft(candidate(), limits()).expect("v2 documents");
    let mut unknown_role = documents.documents().clone();
    let mut manifest: Value =
        serde_json::from_slice(&unknown_role["manifest.json"]).expect("manifest");
    let role = manifest["documents"]
        .as_array_mut()
        .expect("inventory")
        .iter_mut()
        .find(|entry| entry["role"] == "declarative-definitions")
        .expect("role");
    role["role"] = json!("executable-definitions");
    rebind_manifest_and_lock(&mut unknown_role, &manifest);
    assert!(
        ProjectSnapshot::new(unknown_role, limits())
            .expect("admit")
            .parse(limits())
            .is_err()
    );

    let mut unknown_field = documents.documents().clone();
    let path = "worlds/world.json";
    let mut world: Value = serde_json::from_slice(&unknown_field[path]).expect("world");
    world["placements"][0]["runtime_instance"] = json!("implicit");
    let bytes = canonical(&world);
    unknown_field.insert(path.into(), bytes.clone());
    let mut manifest: Value =
        serde_json::from_slice(&unknown_field["manifest.json"]).expect("manifest");
    let entry = manifest["documents"]
        .as_array_mut()
        .expect("inventory")
        .iter_mut()
        .find(|entry| entry["locator"] == path)
        .expect("entry");
    entry["byte_length"] = json!(bytes.len());
    entry["sha256"] = json!(world_project_sha256(&bytes));
    rebind_manifest_and_lock(&mut unknown_field, &manifest);
    assert!(
        ProjectSnapshot::new(unknown_field, limits())
            .expect("admit")
            .parse(limits())
            .is_err()
    );
}

#[test]
fn relocating_one_managed_document_keeps_v2_semantic_identity() {
    let original =
        CanonicalProjectDocuments::from_v2_draft(candidate(), limits()).expect("v2 documents");
    let mut moved = original.documents().clone();
    let declaration_bytes = moved
        .remove("definitions/declarations.json")
        .expect("declarations");
    moved.insert("definitions/moved.json".into(), declaration_bytes);
    let mut manifest: Value = serde_json::from_slice(&moved["manifest.json"]).expect("manifest");
    let entry = manifest["documents"]
        .as_array_mut()
        .expect("inventory")
        .iter_mut()
        .find(|entry| entry["role"] == "declarative-definitions")
        .expect("role");
    entry["locator"] = json!("definitions/moved.json");
    rebind_manifest_and_lock(&mut moved, &manifest);
    let parsed = ProjectSnapshot::new(moved, limits())
        .expect("admit moved project")
        .parse(limits())
        .expect("parse moved project");
    assert_eq!(
        parsed
            .canonical_documents(limits())
            .expect("canonical rewrite")
            .documents(),
        original.documents()
    );
}

fn item_candidate() -> ProjectV2Draft {
    let mut draft = candidate();
    draft.core.records.extend([
        ProjectReferenceRecord::Formula {
            identity: DefinitionIdentityDocument {
                family: "Formula".into(),
                key: "oteryn:reference.formula.item-alpha".into(),
                revision: "definition-r1".into(),
            },
        },
        ProjectReferenceRecord::Effect {
            identity: DefinitionIdentityDocument {
                family: "Effect".into(),
                key: "oteryn:reference.effect.item-alpha".into(),
                revision: "definition-r1".into(),
            },
            client_projection: ProjectionDocument::ServerOnly,
            effect_family: EffectFamilyDocument::Damage,
            formula: DefinitionReferenceDocument {
                family: "Formula".into(),
                key: "oteryn:reference.formula.item-alpha".into(),
                revision: "definition-r1".into(),
            },
        },
        ProjectReferenceRecord::Ability {
            identity: DefinitionIdentityDocument {
                family: "Ability".into(),
                key: "oteryn:reference.ability.item-alpha".into(),
                revision: "definition-r1".into(),
            },
            effects: vec![DefinitionReferenceDocument {
                family: "Effect".into(),
                key: "oteryn:reference.effect.item-alpha".into(),
                revision: "definition-r1".into(),
            }],
        },
        ProjectReferenceRecord::Item {
            identity: DefinitionIdentityDocument {
                family: "Item".into(),
                key: "oteryn:reference.item.weapon-alpha".into(),
                revision: "definition-r1".into(),
            },
            client_projection: ProjectionDocument::ClientSafe,
            materializable: false,
            stack_class: ItemStackDocument::Unknown,
            semantics: ReferenceItemSemantics::default(),
        },
    ]);
    draft
        .state
        .declarations
        .push(ProjectV2Declaration::Interaction {
            identity: identity("interaction.item-alpha"),
            fields: vec![],
        });
    draft
        .state
        .declarations
        .push(ProjectV2Declaration::Service {
            identity: identity("service.item-alpha"),
            offers: vec![ProjectV2ServiceOffer {
                item: reference(ProjectV2Family::Item, "oteryn:reference.item.weapon-alpha"),
                direction: ProjectV2ServiceOfferDirection::SellToPlayer,
                unit_price: 125_000,
                currency: None,
            }],
            recipes: vec![],
            fields: vec![],
        });
    draft.state.item_authoring.push(ProjectV2ItemAuthoring {
        item: reference(ProjectV2Family::Item, "oteryn:reference.item.weapon-alpha"),
        presentation: Some(reference(
            ProjectV2Family::Presentation,
            "oteryn:reference.presentation.courier",
        )),
        document: None,
        taxonomy: Some(ProjectV2ItemTaxonomy {
            primary: "Weapons".into(),
            secondary: Some("Fist".into()),
            tertiary: Some("Monk".into()),
        }),
        forge: Some(ProjectV2ItemForgeProfile {
            classification: 4,
            max_tier: 10,
        }),
        proficiency: Some(ProjectV2WeaponProficiencyProfile {
            levels: vec![
                ProjectV2ProficiencyLevel {
                    level: 2,
                    perks: vec![ProjectV2AugmentBinding {
                        key: "oteryn:augment.proficiency.second".into(),
                        target: ProjectV2AugmentTarget::Ability {
                            ability: reference(
                                ProjectV2Family::Ability,
                                "oteryn:reference.ability.item-alpha",
                            ),
                        },
                        effect: Some(reference(
                            ProjectV2Family::Effect,
                            "oteryn:reference.effect.item-alpha",
                        )),
                        rank_values: vec![ProjectV2AugmentRankValue {
                            rank: 1,
                            value: ProjectV2AugmentValue::RationalPercent(ProjectV2ExactRatio {
                                numerator: 3,
                                denominator: 100,
                            }),
                        }],
                        fields: vec![],
                    }],
                },
                ProjectV2ProficiencyLevel {
                    level: 1,
                    perks: vec![ProjectV2AugmentBinding {
                        key: "oteryn:augment.proficiency.first".into(),
                        target: ProjectV2AugmentTarget::AutoAttack,
                        effect: None,
                        rank_values: vec![ProjectV2AugmentRankValue {
                            rank: 1,
                            value: ProjectV2AugmentValue::SignedPoints(2),
                        }],
                        fields: vec![],
                    }],
                },
            ],
            shaping: Some(ProjectV2PerkShaping {
                max_rank: 10,
                replace_slots: 2,
                refine_enabled: true,
                reshape_enabled: true,
                clear_enabled: true,
                lunar_ascension_enabled: true,
                cost_service: Some(reference(
                    ProjectV2Family::Service,
                    "oteryn:content.service.item-alpha",
                )),
            }),
        }),
        augments: vec![ProjectV2AugmentBinding {
            key: "oteryn:augment.base.item-alpha".into(),
            target: ProjectV2AugmentTarget::OffensiveRune,
            effect: None,
            rank_values: vec![],
            fields: vec![ProjectV2CandidateField {
                field_path: "oteryn:source.augment-note".into(),
                value: ProjectV2CandidateValue::Text("source-only".into()),
            }],
        }],
        on_use_interactions: vec![reference(
            ProjectV2Family::Interaction,
            "oteryn:content.interaction.item-alpha",
        )],
        use_ability: Some(reference(
            ProjectV2Family::Ability,
            "oteryn:reference.ability.item-alpha",
        )),
        required_magic_level: Some(15),
        consumable: Some(ProjectV2ItemConsumableProfile {
            edible: true,
            regeneration_seconds: Some(60),
        }),
        use_observation: Some(ProjectV2ItemUseObservation {
            damage: Some(ProjectV2ItemDamageObservation::Range { min: 60, max: 80 }),
            damage_type: Some("Energy".into()),
            mana_cost: Some(13),
        }),
        lifecycle: Some(ProjectV2ItemLifecycle {
            enchantable: Some(true),
            destructible: Some(false),
            enchant_interactions: vec![reference(
                ProjectV2Family::Interaction,
                "oteryn:content.interaction.item-alpha",
            )],
            destroy_interactions: vec![],
        }),
        source_lifecycle: Some(ProjectV2ItemSourceLifecycle {
            implemented: Some("Summer Update 2026".into()),
            removed: None,
        }),
    });
    draft.state.editor.push(ProjectV2EditorEntry {
        target: reference(ProjectV2Family::Item, "oteryn:reference.item.weapon-alpha"),
        display_name: "Weapon Alpha".into(),
        description: "Editor-only Item metadata".into(),
        categories: vec!["weapon".into()],
        notes: vec!["TibiaWiki crosswalk candidate".into()],
        aliases: vec!["Alpha Weapon".into()],
        tags: vec!["oteryn:editor.item".into()],
    });
    draft
}

#[test]
fn modern_item_authoring_and_shop_relations_round_trip_without_runtime_lowering() {
    let documents =
        CanonicalProjectDocuments::from_v2_draft(item_candidate(), limits()).expect("v2 documents");
    let parsed = ProjectSnapshot::new(documents.documents().clone(), limits())
        .expect("admit v2")
        .parse(limits())
        .expect("parse v2");
    let state = parsed.v2().expect("v2 state");
    assert_eq!(state.item_authoring.len(), 1);
    let item = &state.item_authoring[0];
    assert_eq!(item.forge.expect("forge").classification, 4);
    assert_eq!(item.forge.expect("forge").max_tier, 10);
    assert_eq!(
        item.use_observation
            .as_ref()
            .and_then(|value| value.mana_cost),
        Some(13)
    );
    assert_eq!(
        item.proficiency
            .as_ref()
            .expect("proficiency")
            .levels
            .iter()
            .map(|level| level.level)
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    let service = state
        .declarations
        .iter()
        .find_map(|declaration| match declaration {
            ProjectV2Declaration::Service { offers, .. } if !offers.is_empty() => Some(offers),
            _ => None,
        })
        .expect("typed service offer");
    assert_eq!(service[0].unit_price, 125_000);
    assert_eq!(
        parsed
            .canonical_documents(limits())
            .expect("rewrite")
            .documents(),
        documents.documents()
    );

    let reference = parsed
        .lower_reference_source()
        .expect("reference projection");
    assert_eq!(reference.definitions.len(), 5);
}

#[test]
fn item_authoring_rejects_mutable_instance_and_reverse_relationship_fields() {
    let documents =
        CanonicalProjectDocuments::from_v2_draft(item_candidate(), limits()).expect("v2 documents");
    for forbidden in [
        ("current_forge_tier", json!(3)),
        ("proficiency_xp", json!(25_000)),
        ("npcprice", json!(125_000)),
        ("droppedby", json!(["Demon"])),
    ] {
        let mut changed = documents.documents().clone();
        let path = "definitions/declarations.json";
        let mut declarations: Value = serde_json::from_slice(&changed[path]).expect("declarations");
        declarations["item_authoring"][0][forbidden.0] = forbidden.1;
        let bytes = canonical(&declarations);
        changed.insert(path.into(), bytes.clone());
        let mut manifest: Value =
            serde_json::from_slice(&changed["manifest.json"]).expect("manifest");
        let entry = manifest["documents"]
            .as_array_mut()
            .expect("inventory")
            .iter_mut()
            .find(|entry| entry["locator"] == path)
            .expect("entry");
        entry["byte_length"] = json!(bytes.len());
        entry["sha256"] = json!(world_project_sha256(&bytes));
        rebind_manifest_and_lock(&mut changed, &manifest);
        assert!(
            ProjectSnapshot::new(changed, limits())
                .expect("admit")
                .parse(limits())
                .is_err(),
            "forbidden Item authoring field admitted: {}",
            forbidden.0
        );
    }
}

#[test]
fn protected_empty_v2_declarations_wire_remains_unchanged_without_item_authoring() {
    let documents =
        CanonicalProjectDocuments::from_v2_draft(candidate(), limits()).expect("v2 documents");
    let declarations: Value =
        serde_json::from_slice(&documents.documents()["definitions/declarations.json"])
            .expect("declarations");
    assert!(declarations.get("item_authoring").is_none());
    assert!(declarations.get("authoring_profiles").is_none());
    let parsed = ProjectSnapshot::new(documents.documents().clone(), limits())
        .expect("admit")
        .parse(limits())
        .expect("parse");
    assert_eq!(
        parsed
            .canonical_documents(limits())
            .expect("rewrite")
            .documents(),
        documents.documents()
    );
}

fn wiki_coverage_candidate() -> ProjectV2Draft {
    let mut draft = item_candidate();
    draft.core.records.extend([
        ProjectReferenceRecord::Generic {
            identity: DefinitionIdentityDocument {
                family: "Behavior".into(),
                key: "oteryn:reference.behavior.wiki-alpha".into(),
                revision: "definition-r1".into(),
            },
            client_projection: ProjectionDocument::ServerOnly,
        },
        ProjectReferenceRecord::Creature {
            identity: DefinitionIdentityDocument {
                family: "Creature".into(),
                key: "oteryn:reference.creature.wiki-alpha".into(),
                revision: "definition-r1".into(),
            },
            client_projection: ProjectionDocument::ClientSafe,
            presentation: DefinitionReferenceDocument {
                family: "Presentation".into(),
                key: "oteryn:reference.presentation.courier".into(),
                revision: "definition-r1".into(),
            },
            behavior: DefinitionReferenceDocument {
                family: "Behavior".into(),
                key: "oteryn:reference.behavior.wiki-alpha".into(),
                revision: "definition-r1".into(),
            },
            loot: None,
        },
    ]);

    let interaction = reference(
        ProjectV2Family::Interaction,
        "oteryn:content.interaction.item-alpha",
    );
    let area = reference(ProjectV2Family::Area, "oteryn:content.area.thais");
    let document = reference(
        ProjectV2Family::Document,
        "oteryn:content.document.library-alpha",
    );
    let achievement = reference(
        ProjectV2Family::Achievement,
        "oteryn:content.achievement.alpha",
    );
    let presentation = reference(
        ProjectV2Family::Presentation,
        "oteryn:reference.presentation.courier",
    );
    let item = reference(ProjectV2Family::Item, "oteryn:reference.item.weapon-alpha");

    draft.state.declarations.extend([
        ProjectV2Declaration::Area {
            identity: identity("area.thais"),
            parent: None,
            fields: vec![],
        },
        ProjectV2Declaration::Document {
            identity: identity("document.library-alpha"),
            document_type: ProjectV2DocumentType::Book,
            title: Some("Oteryn Library Alpha".into()),
            author: Some("Oteryn".into()),
            language: Some("en".into()),
            content: vec!["Project-owned readable content.".into()],
            fields: vec![],
        },
        ProjectV2Declaration::Achievement {
            identity: identity("achievement.alpha"),
            presentation: Some(presentation.clone()),
            source_id: Some(1),
            degree: Some(2),
            points: Some(3),
            secret: Some(false),
            premium: Some(false),
            unlock_interactions: vec![interaction.clone()],
            fields: vec![],
        },
        ProjectV2Declaration::Outfit {
            identity: identity("outfit.alpha"),
            presentations: vec![presentation.clone()],
            premium: Some(true),
            acquisition_interactions: vec![interaction.clone()],
            fields: vec![],
        },
        ProjectV2Declaration::Mount {
            identity: identity("mount.alpha"),
            presentation: Some(presentation),
            speed_bonus: Some(10),
            premium: Some(true),
            taming_item: Some(item.clone()),
            acquisition_interactions: vec![interaction.clone()],
            fields: vec![],
        },
        ProjectV2Declaration::Charm {
            identity: identity("charm.alpha"),
            charm_type: ProjectV2CharmType::Major,
            ranks: vec![ProjectV2CharmRank {
                rank: 1,
                points_cost: 100,
                chance: Some(ProjectV2ExactRatio {
                    numerator: 1,
                    denominator: 10,
                }),
                effect: Some(reference(
                    ProjectV2Family::Effect,
                    "oteryn:reference.effect.item-alpha",
                )),
                fields: vec![],
            }],
            fields: vec![],
        },
    ]);

    let service = draft
        .state
        .declarations
        .iter_mut()
        .find_map(|declaration| match declaration {
            ProjectV2Declaration::Service {
                identity, recipes, ..
            } if identity.key == "oteryn:content.service.item-alpha" => Some(recipes),
            _ => None,
        })
        .expect("item service");
    service.push(ProjectV2ServiceRecipe {
        key: "oteryn:recipe.weapon-alpha".into(),
        inputs: vec![ProjectV2ItemQuantity {
            item: item.clone(),
            quantity: 2,
        }],
        outputs: vec![ProjectV2ItemQuantity {
            item: item.clone(),
            quantity: 1,
        }],
        fee: Some(500),
        currency: None,
    });

    draft.state.item_authoring[0].document = Some(document.clone());

    draft.state.authoring_profiles.extend([
        ProjectV2AuthoringProfile {
            target: reference(
                ProjectV2Family::Creature,
                "oteryn:reference.creature.wiki-alpha",
            ),
            data: ProjectV2AuthoringProfileData::Creature(ProjectV2CreatureAuthoring {
                health: Some(1_000),
                experience: Some(500),
                speed: Some(220),
                armor: Some(30),
                mitigation: Some(ProjectV2ExactRatio {
                    numerator: 1,
                    denominator: 20,
                }),
                resistances: vec![ProjectV2Resistance {
                    damage_type: "Fire".into(),
                    percent: ProjectV2ExactRatio {
                        numerator: 1,
                        denominator: 5,
                    },
                }],
                immunities: vec!["Paralysis".into()],
                pushable: Some(false),
                pushes_objects: Some(true),
                pass_through: Some(false),
                abilities: vec![reference(
                    ProjectV2Family::Ability,
                    "oteryn:reference.ability.item-alpha",
                )],
                bestiary: Some(ProjectV2BestiaryProfile {
                    difficulty: "Medium".into(),
                    occurrence: Some("Common".into()),
                    kill_thresholds: vec![5, 50, 500],
                    charm_points: 25,
                }),
                bosstiary: Some(ProjectV2BosstiaryProfile {
                    category: "Bane".into(),
                    prowess_kills: 5,
                    expertise_kills: 20,
                    mastery_kills: 60,
                    boss_points: 5,
                }),
                familiar: Some(ProjectV2FamiliarProfile {
                    vocation: Some("Monk".into()),
                    summon_ability: Some(reference(
                        ProjectV2Family::Ability,
                        "oteryn:reference.ability.item-alpha",
                    )),
                    duration_seconds: Some(900),
                    mana_cost: Some(300),
                    owner_speed_bonus: Some(10),
                }),
                fields: vec![],
            }),
        },
        ProjectV2AuthoringProfile {
            target: reference(
                ProjectV2Family::Ability,
                "oteryn:reference.ability.item-alpha",
            ),
            data: ProjectV2AuthoringProfileData::Ability(ProjectV2AbilityAuthoring {
                incantation: Some("exori alpha".into()),
                vocations: vec!["Monk".into()],
                required_level: Some(30),
                group: Some("Attack".into()),
                cooldown_ms: Some(8_000),
                group_cooldown_ms: Some(2_000),
                premium: Some(true),
                mana_cost: Some(150),
                base_power: Some(85),
                range: Some(7),
                damage_type: Some("Physical".into()),
                acquisition_interactions: vec![interaction.clone()],
                augments: vec![],
                fields: vec![],
            }),
        },
        ProjectV2AuthoringProfile {
            target: reference(ProjectV2Family::Quest, "oteryn:content.quest.courier"),
            data: ProjectV2AuthoringProfileData::Quest(ProjectV2QuestAuthoring {
                required_level: Some(20),
                premium: Some(false),
                repeatable: Some(false),
                prerequisites: vec![],
                reward_items: vec![ProjectV2ItemQuantity {
                    item: item.clone(),
                    quantity: 1,
                }],
                reward_achievements: vec![achievement],
                encounters: vec![reference(
                    ProjectV2Family::Encounter,
                    "oteryn:content.encounter.courier",
                )],
                fields: vec![],
            }),
        },
        ProjectV2AuthoringProfile {
            target: reference(ProjectV2Family::House, "oteryn:content.house.courier"),
            data: ProjectV2AuthoringProfileData::House(ProjectV2HouseAuthoring {
                area: Some(area.clone()),
                size_sqm: Some(120),
                rent_amount: Some(10_000),
                rent_currency: None,
                beds: Some(2),
                floors: Some(2),
                rooms: Some(4),
                player_ownable: Some(true),
                streets: vec!["Harbour Lane".into()],
                fields: vec![],
            }),
        },
        ProjectV2AuthoringProfile {
            target: reference(
                ProjectV2Family::Encounter,
                "oteryn:content.encounter.courier",
            ),
            data: ProjectV2AuthoringProfileData::Encounter(ProjectV2EncounterAuthoring {
                encounter_type: ProjectV2EncounterType::Raid,
                scope: ProjectV2EncounterScope::World,
                areas: vec![area.clone()],
                cooldown_seconds: Some(3_600),
                repeatable: Some(true),
                interactions: vec![interaction.clone()],
                fields: vec![],
            }),
        },
        ProjectV2AuthoringProfile {
            target: reference(ProjectV2Family::WorldObject, "oteryn:content.object.sign"),
            data: ProjectV2AuthoringProfileData::WorldObject(ProjectV2WorldObjectAuthoring {
                area: Some(area.clone()),
                interactions: vec![interaction],
                transitions: vec![reference(
                    ProjectV2Family::Transition,
                    "oteryn:content.transition.courier",
                )],
                document: Some(document.clone()),
                fields: vec![],
            }),
        },
    ]);

    let parent = ProjectV2Placement {
        key: "oteryn:placement.library-shelf".into(),
        world: "oteryn:world.reference".into(),
        map_revision: "map-r1".into(),
        definition: reference(ProjectV2Family::WorldObject, "oteryn:content.object.sign"),
        area: Some(area.clone()),
        document: None,
        parent_placement: None,
        coordinate_frame: "global-target-2026-07-28".into(),
        x: 101,
        y: 200,
        floor: 7,
        presentation_order: ProjectV2PresentationOrder { plane: 0, order: 1 },
        disposition: ProjectV2Disposition::CandidateOnly,
    };
    let child = ProjectV2Placement {
        key: "oteryn:placement.library-book".into(),
        world: "oteryn:world.reference".into(),
        map_revision: "map-r1".into(),
        definition: item,
        area: Some(area),
        document: Some(document),
        parent_placement: Some(parent.key.clone()),
        coordinate_frame: "global-target-2026-07-28".into(),
        x: 101,
        y: 200,
        floor: 7,
        presentation_order: ProjectV2PresentationOrder { plane: 1, order: 0 },
        disposition: ProjectV2Disposition::CandidateOnly,
    };
    draft.state.placements.extend([child, parent]);
    draft
}

#[test]
fn wiki_wide_static_authoring_round_trips_without_runtime_lowering() {
    let documents = CanonicalProjectDocuments::from_v2_draft(wiki_coverage_candidate(), limits())
        .expect("wiki-wide v2 documents");
    let parsed = ProjectSnapshot::new(documents.documents().clone(), limits())
        .expect("admit wiki-wide v2")
        .parse(limits())
        .expect("parse wiki-wide v2");
    let state = parsed.v2().expect("v2 state");

    assert_eq!(state.authoring_profiles.len(), 6);
    assert_eq!(
        state
            .declarations
            .iter()
            .filter(|declaration| matches!(declaration, ProjectV2Declaration::Document { .. }))
            .count(),
        1
    );
    assert_eq!(
        state.item_authoring[0]
            .document
            .as_ref()
            .expect("Item Document")
            .family,
        ProjectV2Family::Document
    );
    let book = state
        .placements
        .iter()
        .find(|placement| placement.key == "oteryn:placement.library-book")
        .expect("book placement");
    assert_eq!(
        book.parent_placement.as_deref(),
        Some("oteryn:placement.library-shelf")
    );
    assert_eq!(
        book.document.as_ref().expect("placement Document").family,
        ProjectV2Family::Document
    );
    assert_eq!(
        parsed
            .canonical_documents(limits())
            .expect("canonical rewrite")
            .documents(),
        documents.documents()
    );

    let reference = parsed
        .lower_reference_source()
        .expect("reference projection remains executable-only");
    assert_eq!(reference.definitions.len(), 7);
}

#[test]
fn wiki_authoring_rejects_mutable_state_and_wrong_profile_ownership() {
    let documents = CanonicalProjectDocuments::from_v2_draft(wiki_coverage_candidate(), limits())
        .expect("wiki-wide v2 documents");

    for (kind, field, value) in [
        ("Document", "player_written_text", json!("mutable")),
        ("Achievement", "unlocked", json!(true)),
    ] {
        let mut changed = documents.documents().clone();
        let path = "definitions/declarations.json";
        let mut declarations: Value = serde_json::from_slice(&changed[path]).expect("declarations");
        let record = declarations["records"]
            .as_array_mut()
            .expect("records")
            .iter_mut()
            .find(|record| record["kind"] == kind)
            .expect("target declaration");
        record[field] = value;
        let bytes = canonical(&declarations);
        changed.insert(path.into(), bytes.clone());
        let mut manifest: Value =
            serde_json::from_slice(&changed["manifest.json"]).expect("manifest");
        let entry = manifest["documents"]
            .as_array_mut()
            .expect("inventory")
            .iter_mut()
            .find(|entry| entry["locator"] == path)
            .expect("entry");
        entry["byte_length"] = json!(bytes.len());
        entry["sha256"] = json!(world_project_sha256(&bytes));
        rebind_manifest_and_lock(&mut changed, &manifest);
        assert!(
            ProjectSnapshot::new(changed, limits())
                .expect("admit")
                .parse(limits())
                .is_err(),
            "mutable field admitted into {kind}: {field}"
        );
    }

    let mut wrong_owner = wiki_coverage_candidate();
    let creature = wrong_owner
        .state
        .authoring_profiles
        .iter_mut()
        .find(|profile| matches!(&profile.data, ProjectV2AuthoringProfileData::Creature(_)))
        .expect("Creature profile");
    creature.target.family = ProjectV2Family::Item;
    assert!(CanonicalProjectDocuments::from_v2_draft(wrong_owner, limits()).is_err());
}

#[test]
fn wiki_placement_hierarchy_rejects_parent_cycles() {
    let mut candidate = wiki_coverage_candidate();
    let shelf_key = "oteryn:placement.library-shelf";
    let book_key = "oteryn:placement.library-book";

    let shelf = candidate
        .state
        .placements
        .iter_mut()
        .find(|placement| placement.key == shelf_key)
        .expect("shelf placement");
    shelf.parent_placement = Some(book_key.into());

    assert!(CanonicalProjectDocuments::from_v2_draft(candidate, limits()).is_err());
}
