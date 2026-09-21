#![allow(clippy::expect_used, clippy::panic)]

use oteryn_game_server::content::*;
use serde_json::Value;
use std::collections::BTreeMap;

const B1_EVIDENCE: &[u8] = include_bytes!(
    "../../../docs/agents/evidence/OTV2-20260919-content-world-cw2-b1-item-identity-catalog.json"
);

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

fn import() -> ProtectedCw2B1VaseImport {
    protected_cw2_b1_vase_import(B1_EVIDENCE).expect("protected B1 vase import")
}

fn draft(import: ProtectedCw2B1VaseImport) -> ProjectDraft {
    ProjectDraft {
        project_revision: "project-r1".to_owned(),
        package_key: "oteryn:content.world-project".to_owned(),
        semantic_schema_version: "reference-schema-v1".to_owned(),
        licensing_metadata: "PENDING".to_owned(),
        world_id: "0123456789ab70cd8ef0123456789abc".to_owned(),
        coordinate_frame: "global-target-2026-07-28".to_owned(),
        records: vec![import.record],
        imports: vec![import.batch],
        metadata: Vec::new(),
    }
}

fn binding_mut(batch: &mut ImportBatch) -> &mut NativeItemBindingDocument {
    batch.candidates[0]
        .normalized_fields
        .iter_mut()
        .find_map(|field| match &mut field.value {
            CandidateValue::NativeItemBinding(binding) => Some(binding),
            _ => None,
        })
        .expect("typed native item binding")
}

fn canonical_json(value: &Value) -> Vec<u8> {
    let mut bytes = serde_json::to_vec(value).expect("JSON encoding");
    bytes.push(b'\n');
    bytes
}

fn assert_authored_item(linked: &CanonicalReferencePlayableContent) {
    assert_eq!(linked.definitions.len(), 1);
    let definition = &linked.definitions[0];
    assert_eq!(definition.definition.family(), DefinitionFamily::Item);
    assert_eq!(definition.definition.key().as_str(), CW2_B1_VASE_KEY);
    assert_eq!(
        definition.definition.revision().as_str(),
        CW2_B1_VASE_REVISION
    );
    assert_eq!(
        definition.client_projection,
        ClientProjectionClass::ClientSafe
    );
    match &definition.kind {
        ReferenceDefinitionKind::Item(item) => {
            assert_eq!(item.physical_class, ReferenceItemPhysicalClass::Physical);
            assert!(item.materializable);
            assert_eq!(item.stack_class, ReferenceItemStackClass::NonStackable);
            assert_eq!(
                item.legal_destinations,
                [ReferenceItemDestination::CharacterInventory]
            );
        }
        unexpected => panic!("unexpected definition: {unexpected:?}"),
    }

    assert_eq!(linked.client_safe_definitions().len(), 1);
    match &linked.client_safe_definitions()[0].kind {
        ClientSafeDefinitionKind::Item(item) => {
            assert_eq!(item.physical_class, ReferenceItemPhysicalClass::Physical);
            assert_eq!(item.stack_class, ReferenceItemStackClass::NonStackable);
        }
        unexpected => panic!("unexpected client-safe definition: {unexpected:?}"),
    }
}

#[test]
fn exact_protected_source_maps_to_the_one_approved_typed_item() {
    assert_eq!(B1_EVIDENCE.len(), PROTECTED_CW2_B1_EVIDENCE_BYTES);
    assert_eq!(
        world_project_sha256(B1_EVIDENCE),
        PROTECTED_CW2_B1_EVIDENCE_SHA256
    );

    let imported = import();
    assert_eq!(imported.batch.source_repository, CW2_B1_SOURCE_REPOSITORY);
    assert_eq!(imported.batch.source_revision, CW2_B1_SOURCE_REVISION);
    assert_eq!(
        imported.batch.source_artifact_sha256,
        PROTECTED_CW2_B1_EVIDENCE_SHA256
    );
    assert_eq!(imported.batch.mapper_revision, PROTECTED_CW2_B1_MAPPER_BLOB);
    assert_eq!(imported.batch.mapper_sha256, PROTECTED_CW2_B1_MAPPER_SHA256);
    assert_eq!(imported.batch.access_disposition, "PENDING");
    assert_eq!(imported.batch.candidates.len(), 1);

    let candidate = &imported.batch.candidates[0];
    assert_eq!(candidate.source_numeric_id, Some(CW2_B1_SOURCE_ITEM_ID));
    assert_eq!(candidate.candidate_family, ImportCandidateFamily::Item);
    assert_eq!(
        candidate.candidate_operation,
        ImportCandidateOperation::BindNativeItem
    );
    assert_eq!(
        candidate.closure_disposition,
        CandidateDisposition::LocalNonProduction
    );
    assert_eq!(candidate.evidence_class, "OTS_HYPOTHESIS_ONLY");

    let fields = candidate
        .normalized_fields
        .iter()
        .map(|field| (field.field_path.as_str(), &field.value))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        fields.get("evidence.catalog-blob"),
        Some(&&CandidateValue::Text(
            PROTECTED_CW2_B1_EVIDENCE_BLOB.to_owned()
        ))
    );
    assert_eq!(
        fields.get("evidence.catalog-product-sha256"),
        Some(&&CandidateValue::Text(
            PROTECTED_CW2_B1_PRODUCT_SHA256.to_owned()
        ))
    );
    assert_eq!(
        fields.get("evidence.field-profile-sha256"),
        Some(&&CandidateValue::Text(
            PROTECTED_CW2_B1_FIELD_PROFILE_SHA256.to_owned()
        ))
    );
    assert_eq!(
        fields.get("evidence.node-sha256"),
        Some(&&CandidateValue::Text(
            PROTECTED_CW2_B1_NODE_SHA256.to_owned()
        ))
    );
    assert_eq!(
        fields.get("source.items-xml-blob"),
        Some(&&CandidateValue::Text(CW2_B1_SOURCE_BLOB.to_owned()))
    );
    assert_eq!(
        fields.get("source.items-xml-sha256"),
        Some(&&CandidateValue::Text(CW2_B1_SOURCE_SHA256.to_owned()))
    );
    assert_eq!(
        fields.get("source.pickup-eligibility"),
        Some(&&CandidateValue::Integer(1))
    );
    assert_eq!(
        fields.get("source.weight-raw"),
        Some(&&CandidateValue::Integer(940))
    );
    assert_eq!(
        fields.get("loss.weight-unit"),
        Some(&&CandidateValue::Text("UNKNOWN".to_owned()))
    );
    assert_eq!(
        fields.get("source.b3-loot-row-identity"),
        Some(&&CandidateValue::Text(CW2_B1_VASE_B3_ROW.to_owned()))
    );
    assert_eq!(
        fields.get("loss.b3-loot-semantics"),
        Some(&&CandidateValue::Text("NOT_PROMOTED".to_owned()))
    );
}

#[test]
fn canonical_round_trip_reaches_the_existing_linker_and_client_boundary() {
    let first = CanonicalProjectDocuments::from_draft(draft(import()), limits())
        .expect("canonical project");
    let second = CanonicalProjectDocuments::from_draft(draft(import()), limits())
        .expect("repeated canonical project");
    assert_eq!(first.documents(), second.documents());

    let project = first
        .clone()
        .into_snapshot(limits())
        .expect("snapshot")
        .parse(limits())
        .expect("parsed project");
    assert_eq!(project.imports().len(), 1);
    assert_eq!(
        project
            .canonical_documents(limits())
            .expect("canonical rewrite")
            .documents(),
        first.documents()
    );
    let source = project.lower_reference_source().expect("Reference source");
    assert_eq!(source.definitions.len(), 1);
    let linked = project.link().expect("existing Reference linker");
    assert_authored_item(&linked);
    assert!(
        linked
            .definitions
            .iter()
            .all(|definition| !matches!(definition.kind, ReferenceDefinitionKind::Loot(_)))
    );
}

#[test]
fn source_observations_and_editor_metadata_cannot_infer_item_semantics() {
    let mut imported = import();
    let weight = imported
        .batch
        .reimport_states
        .iter_mut()
        .find(|state| state.field_path == "source.weight-raw")
        .expect("weight state");
    weight.local = Some(CandidateValue::Integer(950));
    weight.decision = ReimportDecision::RetainLocal;

    let mut candidate = draft(imported);
    candidate.metadata.push(AuthorMetadataEntry {
        stable_identity: CW2_B1_VASE_KEY.to_owned(),
        display_name: "Not authoritative".to_owned(),
        description: "Claims stackable, server-only and immaterial".to_owned(),
        categories: vec!["loot".to_owned()],
        notes: vec!["Must not select gameplay".to_owned()],
    });
    let project = CanonicalProjectDocuments::from_draft(candidate, limits())
        .expect("local source correction remains auditable")
        .into_snapshot(limits())
        .expect("snapshot")
        .parse(limits())
        .expect("parsed project");
    assert_authored_item(&project.link().expect("linked item"));
}

#[test]
fn conflicts_and_invalid_native_targets_fail_closed() {
    let mut conflicting = import();
    let weight = conflicting
        .batch
        .reimport_states
        .iter_mut()
        .find(|state| state.field_path == "source.weight-raw")
        .expect("weight state");
    weight.upstream = None;
    weight.local = Some(CandidateValue::Integer(950));
    weight.decision = ReimportDecision::Conflict;
    assert!(matches!(
        CanonicalProjectDocuments::from_draft(draft(conflicting), limits()),
        Err(ProjectError::InvalidProject(
            "local native item proof has an unresolved import conflict"
        ))
    ));

    let mut missing = import();
    binding_mut(&mut missing.batch).identity.revision = "definition-r2".to_owned();
    missing.batch.candidates[0].candidate_target = format!("{CW2_B1_VASE_KEY}@definition-r2");
    assert!(matches!(
        CanonicalProjectDocuments::from_draft(draft(missing), limits()),
        Err(ProjectError::InvalidProject(
            "native item binding target is missing"
        ))
    ));

    let mut wrong_family = import();
    binding_mut(&mut wrong_family.batch).identity.family = "Creature".to_owned();
    assert!(CanonicalProjectDocuments::from_draft(draft(wrong_family), limits()).is_err());

    let mut wrong_access = import();
    wrong_access.batch.access_disposition = "COPY".to_owned();
    assert!(matches!(
        CanonicalProjectDocuments::from_draft(draft(wrong_access), limits()),
        Err(ProjectError::InvalidProject(
            "local native item proof requires PENDING access disposition"
        ))
    ));

    let mut wrong_license = draft(import());
    wrong_license.licensing_metadata = "CLEARED".to_owned();
    assert!(matches!(
        CanonicalProjectDocuments::from_draft(wrong_license, limits()),
        Err(ProjectError::InvalidProject(
            "local native item proof requires PENDING licensing metadata"
        ))
    ));
}

#[test]
fn coherent_canonical_documents_with_non_pending_licensing_fail_closed() {
    let mut documents = CanonicalProjectDocuments::from_draft(draft(import()), limits())
        .expect("canonical project")
        .documents()
        .clone();

    let mut manifest: Value =
        serde_json::from_slice(&documents["manifest.json"]).expect("manifest JSON");
    manifest["licensing_metadata"] = Value::String("CLEARED".to_owned());
    let manifest_bytes = canonical_json(&manifest);
    documents.insert("manifest.json".to_owned(), manifest_bytes.clone());

    let package = PackageManifestBinding::new(
        ProductionKey::new(manifest["package_key"].as_str().expect("package key"))
            .expect("package key"),
        ProductionAtom::new(
            "package revision",
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
        Sha256HexDigest::new(&world_project_sha256(&manifest_bytes)).expect("manifest digest"),
    );
    let mut lock: Value =
        serde_json::from_slice(&documents["content.lock.json"]).expect("lock JSON");
    lock["entries"][0]["package_provenance_digest"] = Value::String(
        package
            .package_provenance_digest()
            .expect("package provenance")
            .as_str()
            .to_owned(),
    );
    let lock_bytes = canonical_json(&lock);
    documents.insert("content.lock.json".to_owned(), lock_bytes.clone());

    let mut root: Value = serde_json::from_slice(&documents["project.json"]).expect("root JSON");
    root["manifest_sha256"] = Value::String(world_project_sha256(&manifest_bytes));
    root["content_lock_sha256"] = Value::String(world_project_sha256(&lock_bytes));
    documents.insert("project.json".to_owned(), canonical_json(&root));

    let snapshot = ProjectSnapshot::new(documents, limits()).expect("coherent snapshot");
    assert!(matches!(
        snapshot.parse(limits()),
        Err(ProjectError::InvalidProject(
            "local native item proof requires PENDING licensing metadata"
        ))
    ));
}

#[test]
fn exact_input_bound_and_digest_reject_any_catalogue_drift() {
    let mut above = B1_EVIDENCE.to_vec();
    above.push(b' ');
    assert!(matches!(
        protected_cw2_b1_vase_import(&above),
        Err(ProtectedCw2B1ImportError::InputLimitExceeded {
            actual,
            limit: PROTECTED_CW2_B1_EVIDENCE_BYTES
        }) if actual == PROTECTED_CW2_B1_EVIDENCE_BYTES + 1
    ));

    assert!(matches!(
        protected_cw2_b1_vase_import(&B1_EVIDENCE[..B1_EVIDENCE.len() - 1]),
        Err(ProtectedCw2B1ImportError::EvidenceMismatch(
            "evidence byte length"
        ))
    ));

    let mut drifted = B1_EVIDENCE.to_vec();
    let last = drifted.last_mut().expect("nonempty evidence");
    *last ^= 1;
    assert!(matches!(
        protected_cw2_b1_vase_import(&drifted),
        Err(ProtectedCw2B1ImportError::EvidenceMismatch(
            "evidence byte digest"
        ))
    ));
}
