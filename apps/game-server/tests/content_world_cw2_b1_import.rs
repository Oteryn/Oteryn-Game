#![allow(clippy::expect_used, clippy::panic)]

use oteryn_game_server::content::*;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const B1_EVIDENCE: &[u8] = include_bytes!(
    "../../../docs/agents/evidence/OTV2-20260919-content-world-cw2-b1-item-identity-catalog.json"
);
const BATCH_PRODUCT: &[u8] = include_bytes!(
    "../../../docs/agents/evidence/OTV2-20260921-content-world-cw2-native-item-batch.json"
);
const FULL_FAMILY_MAX_DECODED_FIELDS: usize = 2_098_651;
const FULL_FAMILY_MAX_STRING_BYTES: usize = 42_332_603;

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

#[test]
fn all_b1_item_candidate_fields_have_one_typed_destination_or_explicit_loss() {
    let evidence: Value = serde_json::from_slice(B1_EVIDENCE).expect("protected B1 evidence");
    let expected = evidence
        .pointer("/semantic_catalog/field_disposition_records")
        .and_then(Value::as_array)
        .expect("field disposition records")
        .iter()
        .filter(|row| row["disposition"] == "GAME_ITEM_CANDIDATE")
        .map(|row| row["native_field"].as_str().expect("native field"))
        .collect::<BTreeSet<_>>();
    let actual = CW2_B1_ITEM_FIELD_DISPOSITIONS
        .iter()
        .map(|(field, destination)| {
            assert!(!destination.is_empty(), "{field}");
            *field
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(CW2_B1_ITEM_FIELD_DISPOSITIONS.len(), 90);
    assert_eq!(actual.len(), CW2_B1_ITEM_FIELD_DISPOSITIONS.len());
    assert_eq!(actual, expected);
    assert_eq!(
        CW2_B1_ITEM_FIELD_DISPOSITIONS
            .iter()
            .filter(|(_, destination)| destination.starts_with("EXPLICIT_UNSUPPORTED"))
            .count(),
        2,
    );
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

fn batch_limits() -> ProjectEvidenceLimits {
    ProjectEvidenceLimits {
        max_documents: 8,
        max_document_bytes: 2_097_152,
        max_total_bytes: 4_194_304,
        max_json_depth: 24,
        max_decoded_fields: 32_768,
        max_string_bytes: FULL_FAMILY_MAX_STRING_BYTES,
        max_locator_bytes: 160,
        max_locator_segments: 8,
        max_reference_records: CW2_B1_NATIVE_ITEM_BATCH_COUNT,
        max_import_records: CW2_B1_NATIVE_ITEM_BATCH_COUNT,
        max_reimport_states: CW2_B1_NATIVE_ITEM_BATCH_COUNT,
    }
}

fn batch_import() -> ProtectedCw2B1NativeItemBatchImport {
    protected_cw2_b1_native_item_batch_import(B1_EVIDENCE).expect("protected B1 native item batch")
}

fn batch_draft(imported: ProtectedCw2B1NativeItemBatchImport) -> ProjectDraft {
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
    }
}

fn candidate_binding(candidate: &ImportCandidate) -> &NativeItemBindingDocument {
    candidate
        .normalized_fields
        .iter()
        .find_map(|field| match &field.value {
            CandidateValue::NativeItemBinding(binding) => Some(binding),
            _ => None,
        })
        .expect("typed native item binding")
}

fn candidate_binding_mut_at(candidate: &mut ImportCandidate) -> &mut NativeItemBindingDocument {
    candidate
        .normalized_fields
        .iter_mut()
        .find_map(|field| match &mut field.value {
            CandidateValue::NativeItemBinding(binding) => Some(binding),
            _ => None,
        })
        .expect("typed native item binding")
}

#[test]
fn native_item_batch_matches_the_machine_readable_binding_product() {
    let imported = batch_import();
    assert_eq!(imported.records.len(), CW2_B1_NATIVE_ITEM_BATCH_COUNT);
    assert_eq!(
        imported.batch.candidates.len(),
        CW2_B1_NATIVE_ITEM_BATCH_COUNT
    );
    assert_eq!(
        imported.batch.reimport_states.len(),
        CW2_B1_NATIVE_ITEM_BATCH_COUNT
    );

    let product: Value = serde_json::from_slice(BATCH_PRODUCT).expect("binding product JSON");
    assert_eq!(product["schema"], "OTERYN_CW2_NATIVE_ITEM_BINDING_BATCH/v1");
    assert_eq!(product["batch"]["item_count"], 64);
    assert_eq!(product["batch"]["resolved_native_bindings"], 64);
    assert_eq!(product["batch"]["unresolved"], 0);
    assert_eq!(
        product["overlay_projection"]["b3_selected_exact_crosswalk_rows"],
        1_930
    );

    let rows = product["binding_map"]
        .as_array()
        .expect("binding map array");
    assert_eq!(rows.len(), CW2_B1_NATIVE_ITEM_BATCH_COUNT);
    let by_source = rows
        .iter()
        .map(|row| {
            (
                row["source_identity"]
                    .as_str()
                    .expect("source identity")
                    .to_owned(),
                row,
            )
        })
        .collect::<BTreeMap<_, _>>();

    let mut prior_candidate = None;
    let mut native_keys = std::collections::BTreeSet::new();
    for candidate in &imported.batch.candidates {
        if let Some(prior) = prior_candidate {
            assert!(prior < candidate.source_candidate_id.as_str());
        }
        prior_candidate = Some(candidate.source_candidate_id.as_str());

        let row = by_source
            .get(&candidate.source_candidate_id)
            .expect("candidate present in product");
        assert_eq!(
            candidate.source_label,
            row["source_label"].as_str().expect("source label")
        );
        assert_eq!(
            candidate.source_numeric_id,
            Some(
                candidate
                    .source_candidate_id
                    .strip_prefix("crystal:item:")
                    .expect("source prefix")
                    .parse()
                    .expect("numeric source identity")
            )
        );

        let binding = candidate_binding(candidate);
        assert!(native_keys.insert(binding.identity.key.as_str()));
        assert_eq!(binding.identity.family, "Item");
        assert_eq!(
            binding.identity.key,
            row["native_identity"]["key"].as_str().expect("native key")
        );
        assert_eq!(
            binding.identity.revision,
            row["native_identity"]["revision"]
                .as_str()
                .expect("native revision")
        );
        assert_eq!(
            candidate.candidate_target,
            format!("{}@definition-r1", binding.identity.key)
        );

        let fields = candidate
            .normalized_fields
            .iter()
            .map(|field| (field.field_path.as_str(), &field.value))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(
            fields.get("evidence.node-sha256"),
            Some(&&CandidateValue::Text(
                row["source_node_sha256"]
                    .as_str()
                    .expect("node digest")
                    .to_owned()
            ))
        );
        assert_eq!(
            fields.get("evidence.field-profile-sha256"),
            Some(&&CandidateValue::Text(
                row["field_profile_sha256"]
                    .as_str()
                    .expect("field profile")
                    .to_owned()
            ))
        );
        assert_eq!(
            fields.get("source.native-key-authorship"),
            Some(&&CandidateValue::Text(
                "OTERYN_EDITORIAL_SELECTION_NOT_SOURCE_DERIVED".to_owned()
            ))
        );
        assert_eq!(
            fields.get("loss.source-values-as-gameplay-truth"),
            Some(&&CandidateValue::Text("REJECTED".to_owned()))
        );
    }

    assert_eq!(by_source.len(), CW2_B1_NATIVE_ITEM_BATCH_COUNT);
    assert_eq!(native_keys.len(), CW2_B1_NATIVE_ITEM_BATCH_COUNT);
    assert!(by_source.contains_key("crystal:item:2876"));
    assert_eq!(
        by_source["crystal:item:2876"]["native_identity"]["key"],
        CW2_B1_VASE_KEY
    );
    assert_eq!(
        by_source["crystal:item:3357"]["native_identity"]["key"],
        "oteryn:item.armor.plate_armor"
    );
    assert_eq!(
        by_source["crystal:item:3155"]["native_identity"]["key"],
        "oteryn:item.consumable.sudden_death_rune"
    );
}

#[test]
fn native_item_batch_reimport_and_canonical_round_trip_are_deterministic() {
    let first = CanonicalProjectDocuments::from_draft(batch_draft(batch_import()), batch_limits())
        .expect("canonical batch project");
    let second = CanonicalProjectDocuments::from_draft(batch_draft(batch_import()), batch_limits())
        .expect("repeated canonical batch project");
    assert_eq!(first.documents(), second.documents());

    let project = first
        .clone()
        .into_snapshot(batch_limits())
        .expect("batch snapshot")
        .parse(batch_limits())
        .expect("parsed batch project");
    assert_eq!(
        project
            .canonical_documents(batch_limits())
            .expect("canonical rewrite")
            .documents(),
        first.documents()
    );

    let linked = project.link().expect("linked batch");
    assert_eq!(linked.definitions.len(), CW2_B1_NATIVE_ITEM_BATCH_COUNT);
    assert_eq!(
        linked.client_safe_definitions().len(),
        CW2_B1_NATIVE_ITEM_BATCH_COUNT
    );
    assert!(linked.definitions.iter().all(|definition| {
        matches!(
            &definition.kind,
            ReferenceDefinitionKind::Item(ReferenceItemDefinition {
                physical_class: ReferenceItemPhysicalClass::Physical,
                materializable: true,
                ..
            })
        )
    }));
}

#[test]
fn native_item_batch_conflicts_missing_targets_and_duplicates_fail_closed() {
    let mut conflicting = batch_import();
    conflicting.batch.reimport_states[0].upstream = None;
    conflicting.batch.reimport_states[0].local = Some(CandidateValue::SourceId(999_999));
    conflicting.batch.reimport_states[0].decision = ReimportDecision::Conflict;
    assert!(matches!(
        CanonicalProjectDocuments::from_draft(batch_draft(conflicting), batch_limits()),
        Err(ProjectError::InvalidProject(
            "local native item proof has an unresolved import conflict"
        ))
    ));

    let mut missing = batch_import();
    let first = &mut missing.batch.candidates[0];
    candidate_binding_mut_at(first).identity.revision = "definition-r2".to_owned();
    let missing_key = candidate_binding(first).identity.key.clone();
    first.candidate_target = format!("{missing_key}@definition-r2");
    assert!(matches!(
        CanonicalProjectDocuments::from_draft(batch_draft(missing), batch_limits()),
        Err(ProjectError::InvalidProject(
            "native item binding target is missing"
        ))
    ));

    let mut duplicate = batch_import();
    let first_identity = candidate_binding(&duplicate.batch.candidates[0])
        .identity
        .clone();
    let second = &mut duplicate.batch.candidates[1];
    candidate_binding_mut_at(second).identity = first_identity.clone();
    second.candidate_target = format!("{}@{}", first_identity.key, first_identity.revision);
    assert!(matches!(
        CanonicalProjectDocuments::from_draft(batch_draft(duplicate), batch_limits()),
        Err(ProjectError::InvalidProject(
            "duplicate native item binding target"
        ))
    ));
}

#[test]
fn native_item_batch_rejects_any_protected_catalogue_drift() {
    let mut above = B1_EVIDENCE.to_vec();
    above.push(b' ');
    assert!(matches!(
        protected_cw2_b1_native_item_batch_import(&above),
        Err(ProtectedCw2B1ImportError::InputLimitExceeded {
            actual,
            limit: PROTECTED_CW2_B1_EVIDENCE_BYTES
        }) if actual == PROTECTED_CW2_B1_EVIDENCE_BYTES + 1
    ));

    let mut drifted = B1_EVIDENCE.to_vec();
    *drifted.last_mut().expect("nonempty evidence") ^= 1;
    assert!(matches!(
        protected_cw2_b1_native_item_batch_import(&drifted),
        Err(ProtectedCw2B1ImportError::EvidenceMismatch(
            "evidence byte digest"
        ))
    ));
}

fn full_family_limits() -> ProjectEvidenceLimits {
    ProjectEvidenceLimits {
        max_documents: 8,
        max_document_bytes: 96_000_000,
        max_total_bytes: 160_000_000,
        max_json_depth: 24,
        max_decoded_fields: FULL_FAMILY_MAX_DECODED_FIELDS,
        max_string_bytes: 96_000_000,
        max_locator_bytes: 160,
        max_locator_segments: 8,
        max_reference_records: CW2_B1_FULL_ITEM_FAMILY_COUNT,
        max_import_records: CW2_B1_FULL_ITEM_FAMILY_COUNT,
        max_reimport_states: CW2_B1_FULL_ITEM_FAMILY_COUNT,
    }
}

fn full_family_import() -> ProtectedCw2B1FullItemFamilyImport {
    protected_cw2_b1_full_item_family_import(B1_EVIDENCE).expect("protected B1 full Item family")
}

fn promoted_family_import() -> ProtectedCw2B1PromotedItemFamilyImport {
    protected_cw2_b1_promoted_item_family_import(B1_EVIDENCE)
        .expect("protected B1 promoted Item family")
}

fn count_promoted_atoms(semantics: &ReferenceItemSemantics) -> usize {
    use ReferenceItemField::{Known, Unknown};

    let mut count = 0_usize;
    if let Known(presentation) = &semantics.presentation {
        if matches!(&presentation.name, Known(_)) {
            count += 1;
        }
        assert!(matches!(&presentation.description, Unknown));
    }
    if let Known(weapon) = &semantics.weapon {
        if matches!(&weapon.attack, Known(_)) {
            count += 1;
        }
        if matches!(&weapon.defense, Known(_)) {
            count += 1;
        }
        if matches!(&weapon.extra_defense, Known(_)) {
            count += 1;
        }
        if matches!(&weapon.range, Known(_)) {
            count += 1;
        }
        if matches!(&weapon.hit_chance, Known(_)) {
            count += 1;
        }
        assert!(matches!(&weapon.weapon_type, Unknown));
        assert!(matches!(&weapon.max_hit_chance, Unknown));
        assert!(matches!(&weapon.ammunition, Unknown));
        assert!(matches!(&weapon.elemental, Unknown));
    }
    if let Known(protection) = &semantics.protection {
        if matches!(&protection.armor, Known(_)) {
            count += 1;
        }
        assert!(matches!(&protection.resistances, Unknown));
    }
    if let Known(charges) = &semantics.charges
        && matches!(&charges.count, Known(_))
    {
        count += 1;
    }
    if let Known(container) = &semantics.container
        && matches!(&container.capacity, Known(_))
    {
        count += 1;
    }
    count
}

#[test]
fn protected_semantic_promotion_changes_exactly_69_atoms_without_identity_or_materialization_drift()
{
    let base = full_family_import();
    let promoted = promoted_family_import();

    assert_eq!(
        promoted.promoted_fields,
        ITEM_SEMANTIC_PROMOTION_FIELD_COUNT
    );
    assert_eq!(promoted.promoted_items, ITEM_SEMANTIC_PROMOTION_ITEM_COUNT);
    assert_eq!(
        promoted.family.allocation_digest_sha256, base.allocation_digest_sha256,
        "semantic promotion must not change the protected identity allocation"
    );
    assert_eq!(promoted.family.records.len(), CW2_B1_FULL_ITEM_FAMILY_COUNT);

    let base_shape = base
        .records
        .iter()
        .map(|record| {
            let ProjectReferenceRecord::Item {
                identity,
                materializable,
                stack_class,
                ..
            } = record
            else {
                panic!("full family contains only Items");
            };
            (identity.key.clone(), (*materializable, *stack_class))
        })
        .collect::<BTreeMap<_, _>>();

    let mut atom_count = 0_usize;
    let mut promoted_items = 0_usize;
    for record in &promoted.family.records {
        let ProjectReferenceRecord::Item {
            identity,
            materializable,
            stack_class,
            semantics,
            ..
        } = record
        else {
            panic!("promoted full family contains only Items");
        };
        assert_eq!(
            base_shape.get(&identity.key),
            Some(&(*materializable, *stack_class)),
            "promotion changed materializable/stack shape for {}",
            identity.key
        );
        let atoms = count_promoted_atoms(semantics);
        atom_count += atoms;
        promoted_items += usize::from(atoms > 0);
    }

    assert_eq!(atom_count, ITEM_SEMANTIC_PROMOTION_FIELD_COUNT);
    assert_eq!(promoted_items, ITEM_SEMANTIC_PROMOTION_ITEM_COUNT);
}

fn decoded_json_fields(bytes: &[u8]) -> usize {
    fn count(value: &serde_json::Value) -> usize {
        1 + match value {
            serde_json::Value::Array(values) => values.iter().map(count).sum::<usize>(),
            serde_json::Value::Object(values) => values.values().map(count).sum::<usize>(),
            _ => 0,
        }
    }

    count(&serde_json::from_slice(bytes).expect("canonical JSON document"))
}

fn decoded_json_string_bytes(bytes: &[u8]) -> usize {
    fn count(value: &serde_json::Value) -> usize {
        match value {
            serde_json::Value::String(value) => value.len(),
            serde_json::Value::Array(values) => values.iter().map(count).sum(),
            serde_json::Value::Object(values) => values
                .iter()
                .map(|(key, value)| key.len() + count(value))
                .sum(),
            _ => 0,
        }
    }

    count(&serde_json::from_slice(bytes).expect("canonical JSON document"))
}

fn full_family_draft(imported: ProtectedCw2B1FullItemFamilyImport) -> ProjectDraft {
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
    }
}

#[test]
fn full_item_family_registry_closes_the_exact_b1_denominator() {
    let imported = full_family_import();
    assert_eq!(imported.records.len(), CW2_B1_FULL_ITEM_FAMILY_COUNT);
    assert_eq!(
        imported.batch.candidates.len(),
        CW2_B1_FULL_ITEM_FAMILY_COUNT
    );
    assert_eq!(
        imported.batch.reimport_states.len(),
        CW2_B1_FULL_ITEM_FAMILY_COUNT
    );
    assert_eq!(imported.allocation_digest_sha256.len(), 64);
    assert_eq!(
        imported.allocation_digest_sha256,
        "ee9219ccf9d8b2350911abca321507ff924ccd4cb83196efd08b91fbdf098966",
        "CONTROLLED_RED_CAPTURE_FULL_FAMILY_ALLOCATION_DIGEST"
    );
    assert_eq!(
        imported.allocation_digest_sha256,
        full_family_import().allocation_digest_sha256
    );

    let native_keys = imported
        .batch
        .candidates
        .iter()
        .map(|candidate| candidate_binding(candidate).identity.key.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(native_keys.len(), CW2_B1_FULL_ITEM_FAMILY_COUNT);

    let vase = imported
        .batch
        .candidates
        .iter()
        .find(|candidate| candidate.source_numeric_id == Some(CW2_B1_SOURCE_ITEM_ID))
        .expect("preserved vase binding");
    assert_eq!(candidate_binding(vase).identity.key, CW2_B1_VASE_KEY);

    let plate = imported
        .batch
        .candidates
        .iter()
        .find(|candidate| candidate.source_numeric_id == Some(3357))
        .expect("preserved plate armor binding");
    assert_eq!(
        candidate_binding(plate).identity.key,
        "oteryn:item.armor.plate_armor"
    );

    let gold_coin = imported
        .batch
        .candidates
        .iter()
        .find(|candidate| candidate.source_numeric_id == Some(3031))
        .expect("full-family gold coin identity");
    assert!(
        candidate_binding(gold_coin)
            .identity
            .key
            .starts_with("oteryn:item.registry.i")
    );
    assert_eq!(
        gold_coin.disposition_reason,
        "FAMILY_SCALE_IDENTITY_ONLY_GAMEPLAY_SEMANTICS_UNRESOLVED"
    );

    let identity_only = imported
        .records
        .iter()
        .filter(|record| {
            matches!(
                record,
                ProjectReferenceRecord::Item {
                    materializable: false,
                    stack_class: ItemStackDocument::Unknown,
                    ..
                }
            )
        })
        .count();
    assert_eq!(identity_only, CW2_B1_OPAQUE_ITEM_COUNT);
}

#[test]
fn full_item_family_round_trip_compiles_v3_and_rejects_38158() {
    let documents = CanonicalProjectDocuments::from_draft(
        full_family_draft(full_family_import()),
        full_family_limits(),
    )
    .expect("canonical full-family project");
    let snapshot = documents
        .into_snapshot(full_family_limits())
        .expect("full-family snapshot");
    let max_strings = snapshot
        .documents()
        .iter()
        .map(|(locator, bytes)| (locator.as_str(), decoded_json_string_bytes(bytes)))
        .max_by_key(|(_, string_bytes)| *string_bytes)
        .expect("canonical project documents");
    let max_fields = snapshot
        .documents()
        .iter()
        .map(|(locator, bytes)| (locator.as_str(), decoded_json_fields(bytes)))
        .max_by_key(|(_, fields)| *fields)
        .expect("canonical project documents");
    assert_eq!(
        max_strings,
        ("imports/candidates.json", FULL_FAMILY_MAX_STRING_BYTES)
    );
    assert_eq!(
        max_fields,
        ("imports/candidates.json", FULL_FAMILY_MAX_DECODED_FIELDS)
    );

    let string_too_small = ProjectEvidenceLimits {
        max_string_bytes: FULL_FAMILY_MAX_STRING_BYTES - 1,
        ..full_family_limits()
    };
    assert!(matches!(
        snapshot.parse(string_too_small),
        Err(ProjectError::InvalidJson(message))
            if message.contains("project JSON string bytes exceed evidence limit")
    ));

    let fields_too_small = ProjectEvidenceLimits {
        max_decoded_fields: FULL_FAMILY_MAX_DECODED_FIELDS - 1,
        ..full_family_limits()
    };
    assert!(matches!(
        snapshot.parse(fields_too_small),
        Err(ProjectError::InvalidJson(message))
            if message.contains("project decoded fields exceed evidence limit")
    ));

    let project = snapshot
        .parse(full_family_limits())
        .expect("parsed full-family project");
    let linked = project.link().expect("linked full-family Items");
    assert_eq!(linked.definitions.len(), CW2_B1_FULL_ITEM_FAMILY_COUNT);

    let unresolved = linked
        .definitions
        .iter()
        .filter(|definition| {
            matches!(
                &definition.kind,
                ReferenceDefinitionKind::Item(ReferenceItemDefinition {
                    physical_class: ReferenceItemPhysicalClass::Unknown,
                    materializable: false,
                    stack_class: ReferenceItemStackClass::Unknown,
                    legal_destinations,
                    ..
                }) if legal_destinations.is_empty()
            )
        })
        .count();
    assert_eq!(unresolved, CW2_B1_OPAQUE_ITEM_COUNT);

    let first = compile_reference_playable(&linked).expect("full-family v3 artifact");
    let second =
        compile_reference_playable(&linked).expect("deterministic full-family v3 artifact");
    assert_eq!(first.server_artifact, second.server_artifact);
    assert_eq!(first.client_artifact, second.client_artifact);

    let mut above = linked.clone();
    above.definitions.push(
        above
            .definitions
            .last()
            .expect("last full-family definition")
            .clone(),
    );
    assert!(matches!(
        compile_reference_playable(&above),
        Err(ContentError::LimitExceeded {
            resource: "Reference playable definitions",
            actual: 38_158,
            limit: 38_157,
        })
    ));
}
