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
                },
                ProjectV2Declaration::WorldObject {
                    identity: identity("object.sign"),
                    presentation: Some(presentation.clone()),
                },
                ProjectV2Declaration::Dialogue {
                    identity: identity("dialogue.courier"),
                },
                ProjectV2Declaration::Service {
                    identity: identity("service.courier"),
                },
                ProjectV2Declaration::Interaction {
                    identity: identity("interaction.courier"),
                },
                ProjectV2Declaration::Quest {
                    identity: identity("quest.courier"),
                },
                ProjectV2Declaration::Transition {
                    identity: identity("transition.courier"),
                },
                ProjectV2Declaration::House {
                    identity: identity("house.courier"),
                },
                ProjectV2Declaration::Encounter {
                    identity: identity("encounter.courier"),
                },
            ],
            worlds: vec![ProjectV2World {
                key: "oteryn:world.reference".into(),
                world_id: core().world_id,
                coordinate_frame: core().coordinate_frame,
            }],
            placements: vec![ProjectV2Placement {
                key: "oteryn:placement.courier".into(),
                world: "oteryn:world.reference".into(),
                map_revision: "map-r1".into(),
                definition: npc.clone(),
                coordinate_frame: "global-target-2026-07-28".into(),
                x: 100,
                y: 200,
                z: 7,
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
            sources: vec![ProjectV2Source {
                key: "oteryn:source.reference".into(),
                revision: "source-r1".into(),
                sha256: "97fbfe027f93834bfaef365e4271dbb56b479ba29528e3f00a1b046aae0a7491".into(),
                evidence: ProjectV2EvidenceClass::Unknown,
            }],
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

fn rebind_manifest_and_lock(documents: &mut std::collections::BTreeMap<String, Vec<u8>>, manifest: &Value) {
    let manifest_bytes = canonical(manifest);
    let package = PackageManifestBinding::new(
        ProductionKey::new(manifest["package_key"].as_str().expect("package key")).expect("key"),
        ProductionAtom::new("revision", manifest["package_revision"].as_str().expect("revision")).expect("revision"),
        ProductionAtom::new("schema", manifest["semantic_schema_version"].as_str().expect("schema")).expect("schema"),
        ProductionAtom::new("license", manifest["licensing_metadata"].as_str().expect("license")).expect("license"),
        Sha256HexDigest::new(&world_project_sha256(&manifest_bytes)).expect("digest"),
    );
    let mut lock: Value = serde_json::from_slice(&documents["content.lock.json"]).expect("lock");
    lock["entries"][0]["package_provenance_digest"] = json!(package.package_provenance_digest().expect("provenance").as_str());
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
    let documents = CanonicalProjectDocuments::from_v2_draft(candidate(), limits()).expect("v2 documents");
    let mut unknown_role = documents.documents().clone();
    let mut manifest: Value = serde_json::from_slice(&unknown_role["manifest.json"]).expect("manifest");
    let role = manifest["documents"].as_array_mut().expect("inventory").iter_mut()
        .find(|entry| entry["role"] == "declarative-definitions").expect("role");
    role["role"] = json!("executable-definitions");
    rebind_manifest_and_lock(&mut unknown_role, &manifest);
    assert!(ProjectSnapshot::new(unknown_role, limits()).expect("admit").parse(limits()).is_err());

    let mut unknown_field = documents.documents().clone();
    let path = "worlds/world.json";
    let mut world: Value = serde_json::from_slice(&unknown_field[path]).expect("world");
    world["placements"][0]["runtime_instance"] = json!("implicit");
    let bytes = canonical(&world);
    unknown_field.insert(path.into(), bytes.clone());
    let mut manifest: Value = serde_json::from_slice(&unknown_field["manifest.json"]).expect("manifest");
    let entry = manifest["documents"].as_array_mut().expect("inventory").iter_mut()
        .find(|entry| entry["locator"] == path).expect("entry");
    entry["byte_length"] = json!(bytes.len());
    entry["sha256"] = json!(world_project_sha256(&bytes));
    rebind_manifest_and_lock(&mut unknown_field, &manifest);
    assert!(ProjectSnapshot::new(unknown_field, limits()).expect("admit").parse(limits()).is_err());
}
