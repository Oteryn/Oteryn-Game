#![allow(clippy::expect_used, clippy::panic)]

use oteryn_game_server::content::*;
use serde_json::{Value, json};
use std::collections::BTreeMap;

const B4_EVIDENCE: &str = include_str!(
    "../../../docs/agents/evidence/OTV2-20260919-content-world-cw2-b4-ability-effect-formula-evidence.json"
);

fn limits() -> ProjectEvidenceLimits {
    // Measured below against this narrow proof corpus. These are non-production evidence limits.
    ProjectEvidenceLimits {
        max_documents: 12,
        max_document_bytes: 16_384,
        max_total_bytes: 65_536,
        max_json_depth: 20,
        max_decoded_fields: 1_024,
        max_string_bytes: 32_768,
        max_locator_bytes: 160,
        max_locator_segments: 8,
        max_reference_records: 16,
        max_import_records: 8,
        max_reimport_states: 32,
    }
}

fn identity(family: &str, key: &str) -> DefinitionIdentityDocument {
    DefinitionIdentityDocument {
        family: family.to_owned(),
        key: key.to_owned(),
        revision: "definition-r1".to_owned(),
    }
}

fn reference(family: &str, key: &str) -> DefinitionReferenceDocument {
    DefinitionReferenceDocument {
        family: family.to_owned(),
        key: key.to_owned(),
        revision: "definition-r1".to_owned(),
    }
}

fn project_records() -> Vec<ProjectReferenceRecord> {
    vec![
        ProjectReferenceRecord::Creature {
            identity: identity(
                "Creature",
                "oteryn:reference.creature.project-owned-courier",
            ),
            client_projection: ProjectionDocument::ClientSafe,
            presentation: reference(
                "Presentation",
                "oteryn:reference.presentation.project-owned-courier",
            ),
            behavior: reference(
                "Behavior",
                "oteryn:reference.behavior.project-owned-courier",
            ),
            loot: None,
        },
        ProjectReferenceRecord::Item {
            identity: identity("Item", "oteryn:reference.item.project-owned-token"),
            client_projection: ProjectionDocument::ClientSafe,
            materializable: true,
            stack_class: ItemStackDocument::StackCapable,
            semantics: Default::default(),
        },
        ProjectReferenceRecord::Generic {
            identity: identity(
                "Presentation",
                "oteryn:reference.presentation.project-owned-courier",
            ),
            client_projection: ProjectionDocument::ClientSafe,
        },
        ProjectReferenceRecord::Generic {
            identity: identity(
                "Behavior",
                "oteryn:reference.behavior.project-owned-courier",
            ),
            client_projection: ProjectionDocument::ServerOnly,
        },
    ]
}

fn structural_project_records() -> Vec<ProjectReferenceRecord> {
    let heal = "oteryn:reference.effect.project-owned-heal";
    let damage = "oteryn:reference.effect.project-owned-damage";
    let heal_formula = "oteryn:reference.formula.project-owned-heal";
    let damage_formula = "oteryn:reference.formula.project-owned-damage";
    vec![
        ProjectReferenceRecord::Ability {
            identity: identity("Ability", "oteryn:reference.ability.project-owned-closure"),
            effects: vec![
                reference("Effect", heal),
                reference("Effect", damage),
                reference("Effect", heal),
            ],
        },
        ProjectReferenceRecord::Effect {
            identity: identity("Effect", heal),
            client_projection: ProjectionDocument::ClientSafe,
            effect_family: EffectFamilyDocument::Heal,
            formula: reference("Formula", heal_formula),
        },
        ProjectReferenceRecord::Effect {
            identity: identity("Effect", damage),
            client_projection: ProjectionDocument::ServerOnly,
            effect_family: EffectFamilyDocument::Damage,
            formula: reference("Formula", damage_formula),
        },
        ProjectReferenceRecord::Formula {
            identity: identity("Formula", heal_formula),
        },
        ProjectReferenceRecord::Formula {
            identity: identity("Formula", damage_formula),
        },
    ]
}

fn b4_batch() -> ImportBatch {
    let evidence: Value = serde_json::from_str(B4_EVIDENCE).expect("protected B4 evidence parses");
    let candidates = evidence["ability_effect_formula_candidates"]
        .as_array()
        .expect("B4 candidates")
        .iter()
        .map(|candidate| {
            let operation = match candidate["ability_to_effect"]["candidate_family"]
                .as_str()
                .expect("candidate family")
            {
                "DAMAGE" => ImportCandidateOperation::Damage,
                "HEAL" => ImportCandidateOperation::Heal,
                unexpected => panic!("unexpected protected B4 family: {unexpected}"),
            };
            assert!(candidate["native_ability_identity"]["content_key"].is_null());
            assert_eq!(candidate["executable_promotion"]["disposition"], "BLOCKED");
            ImportCandidate {
                source_candidate_id: candidate["source_candidate_id"]
                    .as_str()
                    .expect("source candidate id")
                    .to_owned(),
                source_label: candidate["display_name"]
                    .as_str()
                    .expect("display name")
                    .to_owned(),
                source_numeric_id: None,
                candidate_family: ImportCandidateFamily::AbilityEffectFormula,
                candidate_operation: operation,
                candidate_target: candidate["ability_to_effect"]["candidate_shape"]
                    .as_str()
                    .expect("candidate target")
                    .to_owned(),
                candidate_formula: candidate["effect_to_formula"]["formula_state"]
                    .as_str()
                    .expect("formula state")
                    .to_owned(),
                evidence_class: candidate["target_evidence"]
                    .as_str()
                    .expect("target evidence")
                    .to_owned(),
                closure_disposition: CandidateDisposition::Blocked,
                disposition_reason: candidate["executable_promotion"]["reason_codes"]
                    .as_array()
                    .expect("reason codes")
                    .iter()
                    .map(|reason| reason.as_str().expect("reason"))
                    .collect::<Vec<_>>()
                    .join(","),
                normalized_fields: vec![
                    NamedCandidateField {
                        field_path: "candidate.formula-state".to_owned(),
                        value: CandidateValue::Text("UNKNOWN".to_owned()),
                    },
                    NamedCandidateField {
                        field_path: "candidate.target-evidence".to_owned(),
                        value: CandidateValue::Text("UNKNOWN".to_owned()),
                    },
                ],
            }
        })
        .collect();

    ImportBatch {
        batch_id: "cw2-b4-ability-effect-formula".to_owned(),
        source_repository: evidence["source_snapshot"]["repository"]
            .as_str()
            .expect("repository")
            .to_owned(),
        source_revision: evidence["source_snapshot"]["revision"]
            .as_str()
            .expect("revision")
            .to_owned(),
        source_artifact_sha256: "97fbfe027f93834bfaef365e4271dbb56b479ba29528e3f00a1b046aae0a7491"
            .to_owned(),
        access_disposition: "PENDING".to_owned(),
        source_generation_profile: evidence["schema"].as_str().expect("schema").to_owned(),
        importer: "repository-protected-cw2-b4-evidence".to_owned(),
        mapper: evidence["mapper_profile"]
            .as_str()
            .expect("mapper profile")
            .to_owned(),
        mapper_revision: evidence["mapper_revision"]["git_blob"]
            .as_str()
            .expect("mapper git blob")
            .to_owned(),
        mapper_sha256: evidence["mapper_revision"]["canonical_sha256"]
            .as_str()
            .expect("mapper sha256")
            .to_owned(),
        candidates,
        reimport_states: vec![ReimportFieldState {
            stable_identity: "reference-source:ability:ice_strike".to_owned(),
            field_path: "candidate.formula-state".to_owned(),
            baseline: Some(CandidateValue::Text("UNKNOWN".to_owned())),
            upstream: Some(CandidateValue::Text("UNKNOWN".to_owned())),
            local: Some(CandidateValue::Text("UNKNOWN".to_owned())),
            decision: ReimportDecision::Unchanged,
        }],
    }
}

fn draft() -> ProjectDraft {
    ProjectDraft {
        project_revision: "project-r1".to_owned(),
        package_key: "oteryn:content.world-project".to_owned(),
        semantic_schema_version: "reference-schema-v1".to_owned(),
        licensing_metadata: "license:project-owned-v1".to_owned(),
        world_id: "0123456789ab70cd8ef0123456789abc".to_owned(),
        coordinate_frame: "global-target-2026-07-28".to_owned(),
        records: project_records(),
        imports: vec![b4_batch()],
        metadata: vec![AuthorMetadataEntry {
            stable_identity: "oteryn:reference.item.project-owned-token".to_owned(),
            display_name: "Project Token".to_owned(),
            description: "Author-facing description".to_owned(),
            categories: vec!["editor-category".to_owned()],
            notes: vec!["Does not select gameplay behavior".to_owned()],
        }],
    }
}

fn structural_draft() -> ProjectDraft {
    let mut candidate = draft();
    candidate.records.extend(structural_project_records());
    candidate.metadata.push(AuthorMetadataEntry {
        stable_identity: "oteryn:reference.ability.project-owned-closure".to_owned(),
        display_name: "Project-owned structural ability".to_owned(),
        description: "Non-authoritative editor text".to_owned(),
        categories: vec!["author-category".to_owned()],
        notes: vec!["No execution semantics".to_owned()],
    });
    candidate
}

fn documents(candidate: ProjectDraft) -> BTreeMap<String, Vec<u8>> {
    CanonicalProjectDocuments::from_draft(candidate, limits())
        .expect("canonical project")
        .documents()
        .clone()
}

fn parse(documents: BTreeMap<String, Vec<u8>>) -> Result<WorldProject, ProjectError> {
    ProjectSnapshot::new(documents, limits())?.parse(limits())
}

fn parse_with_limits(
    documents: BTreeMap<String, Vec<u8>>,
    evidence_limits: ProjectEvidenceLimits,
) -> Result<WorldProject, ProjectError> {
    ProjectSnapshot::new(documents, evidence_limits)?.parse(evidence_limits)
}

fn canonical_value(value: &Value) -> Vec<u8> {
    let mut bytes = serde_json::to_vec(value).expect("JSON encoding");
    bytes.push(b'\n');
    bytes
}

#[test]
fn canonical_project_round_trip_is_byte_stable_and_permutation_independent() {
    let expected = documents(draft());
    let project = parse(expected.clone()).expect("project parses");
    let rewritten = project
        .canonical_documents(limits())
        .expect("canonical rewrite");
    assert_eq!(rewritten.documents(), &expected);

    let mut permuted = draft();
    permuted.records.reverse();
    permuted.imports[0].candidates.reverse();
    permuted.metadata.reverse();
    assert_eq!(documents(permuted), expected);
}

#[test]
fn minimal_project_owned_closure_uses_the_existing_reference_linker() {
    let project = parse(documents(draft())).expect("project parses");
    let linked = project
        .link()
        .expect("existing Reference linker accepts closure");
    assert_eq!(linked.definitions.len(), 4);
    assert!(linked.definitions.iter().any(|definition| {
        definition.definition.key().as_str() == "oteryn:reference.item.project-owned-token"
    }));
}

#[test]
fn structural_project_round_trip_links_through_the_existing_reference_graph() {
    let expected = documents(structural_draft());
    let project = parse(expected.clone()).expect("structural project parses");
    assert_eq!(
        project
            .canonical_documents(limits())
            .expect("canonical structural rewrite")
            .documents(),
        &expected
    );
    let linked = project.link().expect("structural Reference closure links");
    let ability = linked
        .definitions
        .iter()
        .find_map(|definition| match &definition.kind {
            ReferenceDefinitionKind::Ability(ability) => Some(ability),
            _ => None,
        })
        .expect("typed structural ability");
    assert_eq!(ability.effects.len(), 3);
    assert_eq!(ability.effects[0], ability.effects[2]);
    assert_eq!(
        ability.effects[0].key().as_str(),
        "oteryn:reference.effect.project-owned-heal"
    );
    assert_eq!(
        ability.effects[1].key().as_str(),
        "oteryn:reference.effect.project-owned-damage"
    );

    let mut permuted = structural_draft();
    permuted.records.reverse();
    assert_eq!(documents(permuted), expected);
}

#[test]
fn project_keeps_terrain_world_objects_and_loot_typed_and_client_safe() {
    let mut candidate = draft();
    candidate.records.extend([
        ProjectReferenceRecord::Generic {
            identity: identity("Terrain", "oteryn:reference.terrain.project-floor"),
            client_projection: ProjectionDocument::ClientSafe,
        },
        ProjectReferenceRecord::LocalObject {
            identity: identity("LocalObject", "oteryn:reference.object.project-door"),
            client_projection: ProjectionDocument::ClientSafe,
            states: vec![
                "oteryn:reference.state.closed".to_owned(),
                "oteryn:reference.state.open".to_owned(),
            ],
        },
        ProjectReferenceRecord::Loot {
            identity: identity("Loot", "oteryn:reference.loot.project-courier"),
            algorithm: LootAlgorithmDocument::GuaranteedEntries,
            entries: vec![LootEntryDocument {
                item: reference("Item", "oteryn:reference.item.project-owned-token"),
                min_count: 1,
                max_count: 2,
                probability_ppm: None,
            }],
        },
    ]);
    let expected = documents(candidate);
    assert_eq!(expected.len(), 6, "existing v1 six-document writer");
    let project = parse(expected.clone()).expect("typed project parses");
    assert_eq!(
        project
            .canonical_documents(limits())
            .expect("rewrite")
            .documents(),
        &expected
    );
    let linked = project
        .link()
        .expect("existing Reference linker validates typed closure");
    assert_eq!(linked.definitions.len(), 7);
    let client = linked.client_safe_definitions();
    assert!(
        client
            .iter()
            .any(|entry| entry.definition.family() == DefinitionFamily::Terrain)
    );
    assert!(
        client
            .iter()
            .any(|entry| entry.definition.family() == DefinitionFamily::LocalObject)
    );
    assert!(
        !client
            .iter()
            .any(|entry| entry.definition.family() == DefinitionFamily::Loot)
    );
}

#[test]
fn new_project_families_reject_wrong_shape_and_unsupported_loot_semantics() {
    let mut wrong_item = draft();
    wrong_item.records.push(ProjectReferenceRecord::Loot {
        identity: identity("Loot", "oteryn:reference.loot.wrong-item"),
        algorithm: LootAlgorithmDocument::GuaranteedEntries,
        entries: vec![LootEntryDocument {
            item: reference(
                "Creature",
                "oteryn:reference.creature.project-owned-courier",
            ),
            min_count: 1,
            max_count: 1,
            probability_ppm: None,
        }],
    });
    assert!(CanonicalProjectDocuments::from_draft(wrong_item, limits()).is_err());

    let mut empty_states = draft();
    empty_states
        .records
        .push(ProjectReferenceRecord::LocalObject {
            identity: identity("LocalObject", "oteryn:reference.object.empty"),
            client_projection: ProjectionDocument::ClientSafe,
            states: vec![],
        });
    assert!(
        parse(documents(empty_states))
            .expect("source parses")
            .link()
            .is_err()
    );

    let mut unsupported = draft();
    unsupported.records.push(ProjectReferenceRecord::Loot {
        identity: identity("Loot", "oteryn:reference.loot.weighted"),
        algorithm: LootAlgorithmDocument::WeightedSingleSelection,
        entries: vec![LootEntryDocument {
            item: reference("Item", "oteryn:reference.item.project-owned-token"),
            min_count: 1,
            max_count: 1,
            probability_ppm: None,
        }],
    });
    assert!(
        parse(documents(unsupported))
            .expect("source parses")
            .link()
            .is_err()
    );

    let mut untyped = draft();
    untyped.records.push(ProjectReferenceRecord::Generic {
        identity: identity("Loot", "oteryn:reference.loot.generic"),
        client_projection: ProjectionDocument::ServerOnly,
    });
    assert!(CanonicalProjectDocuments::from_draft(untyped, limits()).is_err());
}

#[test]
fn structural_project_fails_closed_on_shape_and_exact_reference_errors() {
    let mut generic = draft();
    generic.records.push(ProjectReferenceRecord::Generic {
        identity: identity("Ability", "oteryn:reference.ability.project-owned-generic"),
        client_projection: ProjectionDocument::ServerOnly,
    });
    assert!(CanonicalProjectDocuments::from_draft(generic, limits()).is_err());

    let mut wrong_edge = structural_draft();
    let ProjectReferenceRecord::Ability { effects, .. } = wrong_edge
        .records
        .iter_mut()
        .find(|record| matches!(record, ProjectReferenceRecord::Ability { .. }))
        .expect("ability record")
    else {
        panic!("ability record shape")
    };
    effects[0].family = "Formula".to_owned();
    assert!(CanonicalProjectDocuments::from_draft(wrong_edge, limits()).is_err());

    let mut missing_effect = structural_draft();
    missing_effect.records.retain(|record| {
        !matches!(record, ProjectReferenceRecord::Effect { identity, .. }
            if identity.key == "oteryn:reference.effect.project-owned-heal")
    });
    let project = parse(documents(missing_effect)).expect("missing edge project parses");
    assert!(matches!(
        project.link(),
        Err(ProjectError::Content(ContentError::MissingReference { .. }))
    ));

    let mut stale_formula = structural_draft();
    let ProjectReferenceRecord::Effect { formula, .. } = stale_formula
        .records
        .iter_mut()
        .find(|record| {
            matches!(
                record,
                ProjectReferenceRecord::Effect {
                    effect_family: EffectFamilyDocument::Damage,
                    ..
                }
            )
        })
        .expect("damage effect")
    else {
        panic!("damage effect shape")
    };
    formula.revision = "definition-r2".to_owned();
    let project = parse(documents(stale_formula)).expect("stale edge project parses");
    assert!(matches!(
        project.link(),
        Err(ProjectError::Content(ContentError::RevisionMismatch(
            "reference-playable definition revision"
        )))
    ));
}

#[test]
fn opaque_formula_rejects_payload_and_project_metadata_cannot_redirect_edges() {
    let formula_with_payload = json!({
        "kind": "Formula",
        "identity": {
            "family": "Formula",
            "key": "oteryn:reference.formula.project-owned-opaque",
            "revision": "definition-r1"
        },
        "expression": "not-authorized"
    });
    assert!(serde_json::from_value::<ProjectReferenceRecord>(formula_with_payload).is_err());

    let first = parse(documents(structural_draft())).expect("first structural project");
    let first_source = first
        .lower_reference_source()
        .expect("first structural source");
    let mut changed = structural_draft();
    let metadata = changed
        .metadata
        .iter_mut()
        .find(|entry| entry.stable_identity == "oteryn:reference.ability.project-owned-closure")
        .expect("ability metadata");
    metadata.display_name = "Pretend redirected ability".to_owned();
    metadata.notes = vec!["oteryn:reference.effect.unrelated".to_owned()];
    let second = parse(documents(changed)).expect("second structural project");
    assert_eq!(
        first_source.definitions,
        second
            .lower_reference_source()
            .expect("second structural source")
            .definitions
    );
}

#[test]
fn structural_proof_corpus_uses_measured_finite_boundaries() {
    let emitted = documents(structural_draft());
    let largest = emitted.values().map(Vec::len).max().expect("documents");
    let total: usize = emitted.values().map(Vec::len).sum();
    let mut exact = limits();
    exact.max_documents = emitted.len();
    exact.max_document_bytes = largest;
    exact.max_total_bytes = total;
    exact.max_reference_records = structural_draft().records.len();
    CanonicalProjectDocuments::from_draft(structural_draft(), exact)
        .expect("exact structural corpus boundaries");

    let mut below_records = exact;
    below_records.max_reference_records -= 1;
    assert!(CanonicalProjectDocuments::from_draft(structural_draft(), below_records).is_err());
    let mut below_document = exact;
    below_document.max_document_bytes -= 1;
    assert!(CanonicalProjectDocuments::from_draft(structural_draft(), below_document).is_err());
    let mut below_total = exact;
    below_total.max_total_bytes -= 1;
    assert!(CanonicalProjectDocuments::from_draft(structural_draft(), below_total).is_err());
}

#[test]
fn real_b4_candidates_retain_exact_provenance_without_native_promotion() {
    let project = parse(documents(draft())).expect("project parses");
    let imports = project.imports();
    assert_eq!(imports.len(), 1);
    let batch = &imports[0];
    assert_eq!(batch.source_repository, "Oteryn/Oteryn-Game");
    assert_eq!(
        batch.source_revision,
        "03a821edd828e24ccff6e2cb7fc819a776cbd238"
    );
    assert_eq!(
        batch.source_artifact_sha256,
        "97fbfe027f93834bfaef365e4271dbb56b479ba29528e3f00a1b046aae0a7491"
    );
    assert_eq!(
        batch.mapper_sha256,
        "bc68f0f63a5dd6ea5ee7a3c20b708d6ea78c6033a2f5dddb518c49d6f45f8666"
    );
    assert_eq!(batch.candidates.len(), 2);
    assert!(
        batch
            .candidates
            .iter()
            .all(|candidate| candidate.source_numeric_id.is_none())
    );
    assert!(
        batch
            .candidates
            .iter()
            .all(|candidate| candidate.closure_disposition == CandidateDisposition::Blocked)
    );

    let source = project
        .lower_reference_source()
        .expect("project-owned records lower");
    assert_eq!(source.definitions.len(), 4);
    assert!(!source.definitions.iter().any(|definition| {
        definition.definition.key().as_str().contains("ice_strike")
            || definition
                .definition
                .key()
                .as_str()
                .contains("light_healing")
    }));
}

#[test]
fn strict_json_rejects_duplicate_names_unknown_fields_floats_and_trailing_values() {
    let base = documents(draft());
    let root = String::from_utf8(base["project.json"].clone()).expect("UTF-8 root");
    for malformed in [
        root.replacen(
            "{\"schema\":",
            "{\"schema\":\"OTERYN_WORLD_PROJECT_ROOT/v1\",\"schema\":",
            1,
        ),
        root.replacen("{", "{\"unknown\":true,", 1),
        root.replacen("{", "{\"number\":1.5,", 1),
        format!("{}{{}}", root.trim_end()),
    ] {
        let mut candidate = base.clone();
        candidate.insert("project.json".to_owned(), malformed.into_bytes());
        assert!(parse(candidate).is_err());
    }
}

#[test]
fn locator_aliases_and_control_collisions_fail_before_parse() {
    for locator in [
        "../records/a.json",
        "/records/a.json",
        "C:/records/a.json",
        "records\\a.json",
        "records//a.json",
        "records/./a.json",
        "records/A.json",
        "records/con.json",
        "records/a%2fb.json",
        "records/a.json.",
    ] {
        let result = ProjectSnapshot::new([(locator.to_owned(), b"{}\n".to_vec())], limits());
        assert!(
            matches!(result, Err(ProjectError::InvalidLocator(_))),
            "{locator}"
        );
    }
    let duplicate = ProjectSnapshot::new(
        [
            ("records/a.json".to_owned(), b"{}\n".to_vec()),
            ("records/a.json".to_owned(), b"{}\n".to_vec()),
        ],
        limits(),
    );
    assert!(matches!(duplicate, Err(ProjectError::DuplicateLocator(_))));
}

#[test]
fn inventory_digest_and_complete_document_set_are_enforced() {
    let base = documents(draft());
    let mut changed = base.clone();
    changed.get_mut("records/reference.json").expect("records")[0] ^= 1;
    assert!(matches!(
        parse(changed),
        Err(ProjectError::DigestMismatch(_))
    ));

    let mut missing = base.clone();
    missing.remove("metadata/author.json");
    assert!(matches!(
        parse(missing),
        Err(ProjectError::MissingDocument(_))
    ));

    let mut extra = base;
    extra.insert("metadata/unlisted.json".to_owned(), b"{}\n".to_vec());
    assert!(matches!(
        parse(extra),
        Err(ProjectError::UnexpectedDocument(_))
    ));
}

fn replace_lock_and_rebind_root(documents: &mut BTreeMap<String, Vec<u8>>, lock: &Value) {
    let lock_bytes = canonical_value(lock);
    let mut root: Value = serde_json::from_slice(&documents["project.json"]).expect("project root");
    root["content_lock_sha256"] = Value::String(world_project_sha256(&lock_bytes));
    documents.insert("content.lock.json".to_owned(), lock_bytes);
    documents.insert("project.json".to_owned(), canonical_value(&root));
}

fn replace_document_and_rebind(
    documents: &mut BTreeMap<String, Vec<u8>>,
    locator: &str,
    bytes: Vec<u8>,
) {
    documents.insert(locator.to_owned(), bytes.clone());
    let mut manifest: Value =
        serde_json::from_slice(&documents["manifest.json"]).expect("manifest");
    let entry = manifest["documents"]
        .as_array_mut()
        .expect("manifest inventory")
        .iter_mut()
        .find(|entry| entry["locator"] == locator)
        .expect("document inventory entry");
    entry["byte_length"] = Value::from(bytes.len());
    entry["sha256"] = Value::String(world_project_sha256(&bytes));
    let manifest_bytes = canonical_value(&manifest);
    documents.insert("manifest.json".to_owned(), manifest_bytes.clone());

    let package = PackageManifestBinding::new(
        ProductionKey::new(manifest["package_key"].as_str().expect("package key")).expect("key"),
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
    let mut lock: Value = serde_json::from_slice(&documents["content.lock.json"]).expect("lock");
    lock["entries"][0]["package_provenance_digest"] = Value::String(
        package
            .package_provenance_digest()
            .expect("provenance")
            .as_str()
            .to_owned(),
    );
    let lock_bytes = canonical_value(&lock);
    documents.insert("content.lock.json".to_owned(), lock_bytes.clone());
    let mut root: Value = serde_json::from_slice(&documents["project.json"]).expect("project root");
    root["manifest_sha256"] = Value::String(world_project_sha256(&manifest_bytes));
    root["content_lock_sha256"] = Value::String(world_project_sha256(&lock_bytes));
    documents.insert("project.json".to_owned(), canonical_value(&root));
}

#[test]
fn project_parser_rejects_unknown_members_in_nested_item_payloads() {
    let cases = [
        (
            "field state envelope",
            json!({
                "presentation": {
                    "state": "UNKNOWN",
                    "legacy_state": "UNKNOWN"
                }
            }),
        ),
        (
            "presentation aliases",
            json!({
                "presentation": {
                    "state": "KNOWN",
                    "value": {
                        "name": {"state": "UNKNOWN"},
                        "description": {"state": "UNKNOWN"},
                        "aliases": ["legacy alias"]
                    }
                }
            }),
        ),
        (
            "presentation tags",
            json!({
                "presentation": {
                    "state": "KNOWN",
                    "value": {
                        "name": {"state": "UNKNOWN"},
                        "description": {"state": "UNKNOWN"},
                        "tags": ["legacy-tag"]
                    }
                }
            }),
        ),
        (
            "presentation binding",
            json!({
                "presentation": {
                    "state": "KNOWN",
                    "value": {
                        "name": {"state": "UNKNOWN"},
                        "description": {"state": "UNKNOWN"},
                        "appearance_binding": "legacy-sprite"
                    }
                }
            }),
        ),
        (
            "nested equipment vector",
            json!({
                "equipment": {
                    "state": "KNOWN",
                    "value": {
                        "patterns": {
                            "state": "KNOWN",
                            "value": [{
                                "pattern_id": 1,
                                "primary_slot": {"state": "UNKNOWN"},
                                "additional_reserved_slots": {"state": "UNKNOWN"},
                                "mutually_exclusive_groups": {"state": "UNKNOWN"},
                                "vocations": {"state": "UNKNOWN"},
                                "level": {"state": "UNKNOWN"},
                                "compatibility_rule": {"state": "UNKNOWN"},
                                "legacy_pattern": true
                            }]
                        }
                    }
                }
            }),
        ),
        (
            "nested target reference",
            json!({
                "temporal": {
                    "state": "KNOWN",
                    "value": {
                        "consumption_mode": {"state": "UNKNOWN"},
                        "duration": {"state": "UNKNOWN"},
                        "stop_duration": {"state": "UNKNOWN"},
                        "decay_target": {
                            "state": "KNOWN",
                            "value": {
                                "key": "oteryn:reference.item.project-owned-token",
                                "revision": "definition-r1",
                                "source_numeric_id": 100
                            }
                        }
                    }
                }
            }),
        ),
        (
            "nested rational payload",
            json!({
                "weapon": {
                    "state": "KNOWN",
                    "value": {
                        "weapon_type": {"state": "UNKNOWN"},
                        "attack": {"state": "UNKNOWN"},
                        "defense": {"state": "UNKNOWN"},
                        "extra_defense": {"state": "UNKNOWN"},
                        "range": {"state": "UNKNOWN"},
                        "hit_chance": {
                            "state": "KNOWN",
                            "value": {"numerator": 1, "denominator": 1, "scale": 100}
                        },
                        "max_hit_chance": {"state": "UNKNOWN"},
                        "ammunition": {"state": "UNKNOWN"},
                        "elemental": {"state": "UNKNOWN"}
                    }
                }
            }),
        ),
        (
            "nested structured enum payload",
            json!({
                "skill_modifiers": {
                    "state": "KNOWN",
                    "value": {
                        "modifiers": {
                            "state": "KNOWN",
                            "value": [{
                                "kind": "CRITICAL_HIT_CHANCE",
                                "target_domain": {"state": "UNKNOWN"},
                                "evaluation_phase": {"state": "UNKNOWN"},
                                "priority": {"state": "UNKNOWN"},
                                "parameter": {
                                    "state": "KNOWN",
                                    "value": {
                                        "kind": "BOOLEAN",
                                        "value": true,
                                        "legacy_parameter": true
                                    }
                                }
                            }]
                        }
                    }
                }
            }),
        ),
    ];

    for (case, semantics) in cases {
        let mut emitted = documents(draft());
        let mut records: Value =
            serde_json::from_slice(&emitted["records/reference.json"]).expect("reference records");
        let item = records["records"]
            .as_array_mut()
            .expect("records")
            .iter_mut()
            .find(|record| record["kind"] == "Item")
            .expect("Item record");
        item["semantics"] = semantics;
        replace_document_and_rebind(
            &mut emitted,
            "records/reference.json",
            canonical_value(&records),
        );
        let result = parse(emitted);
        assert!(
            matches!(&result, Err(ProjectError::InvalidJson(_))),
            "{case} must fail closed at the Project parser boundary: {result:?}"
        );
    }
}

#[test]
fn content_lock_requires_exactly_one_immutable_root_entry() {
    for mutation in 0..4 {
        let mut emitted = documents(draft());
        let mut lock: Value =
            serde_json::from_slice(&emitted["content.lock.json"]).expect("Content Lock");
        match mutation {
            0 => {
                let extra = lock["entries"][0].clone();
                lock["entries"]
                    .as_array_mut()
                    .expect("lock entries")
                    .push(extra);
            }
            1 => lock["entries"][0]["floating"] = Value::Bool(true),
            2 => lock["entries"][0]["dependency"] = Value::Bool(true),
            3 => {
                lock["entries"][0]["package_key"] =
                    Value::String("oteryn:content.wrong-package".to_owned());
            }
            _ => return,
        }
        replace_lock_and_rebind_root(&mut emitted, &lock);
        assert!(matches!(
            parse(emitted),
            Err(ProjectError::InvalidProject(_))
        ));
    }
}

#[test]
fn measured_byte_limits_accept_exact_max_and_reject_max_plus_one() {
    let emitted = documents(draft());
    let largest = emitted.values().map(Vec::len).max().expect("documents");
    let total: usize = emitted.values().map(Vec::len).sum();
    assert_eq!(emitted.len(), 6);
    assert_eq!(largest, 2_260);
    assert_eq!(total, 5_314);
    let mut exact = limits();
    exact.max_document_bytes = largest;
    exact.max_total_bytes = total;
    ProjectSnapshot::new(emitted.clone(), exact).expect("exact measured boundary");
    CanonicalProjectDocuments::from_draft(draft(), exact).expect("exact bounded writer boundary");

    let mut below_document = exact;
    below_document.max_document_bytes = largest - 1;
    assert!(matches!(
        ProjectSnapshot::new(emitted.clone(), below_document),
        Err(ProjectError::LimitExceeded {
            resource: "project document bytes",
            ..
        })
    ));
    assert!(matches!(
        CanonicalProjectDocuments::from_draft(draft(), below_document),
        Err(ProjectError::LimitExceeded {
            resource: "project document bytes",
            ..
        })
    ));
    let mut below_total = exact;
    below_total.max_total_bytes = total - 1;
    assert!(matches!(
        ProjectSnapshot::new(emitted, below_total),
        Err(ProjectError::LimitExceeded {
            resource: "project total bytes",
            ..
        })
    ));
    assert!(matches!(
        CanonicalProjectDocuments::from_draft(draft(), below_total),
        Err(ProjectError::LimitExceeded {
            resource: "project total bytes",
            ..
        })
    ));
}

#[test]
fn every_used_count_and_locator_limit_has_a_measured_boundary() {
    let emitted = documents(draft());
    let longest_locator = emitted.keys().map(String::len).max().expect("locators");
    let most_segments = emitted
        .keys()
        .map(|locator| locator.split('/').count())
        .max()
        .expect("locators");

    let mut exact = limits();
    exact.max_documents = emitted.len();
    exact.max_locator_bytes = longest_locator;
    exact.max_locator_segments = most_segments;
    ProjectSnapshot::new(emitted.clone(), exact).expect("exact locator/count bounds");
    CanonicalProjectDocuments::from_draft(draft(), exact)
        .expect("exact writer locator/count bounds");

    let mut below_documents = exact;
    below_documents.max_documents -= 1;
    assert!(matches!(
        ProjectSnapshot::new(emitted.clone(), below_documents),
        Err(ProjectError::LimitExceeded {
            resource: "project documents",
            ..
        })
    ));
    assert!(matches!(
        CanonicalProjectDocuments::from_draft(draft(), below_documents),
        Err(ProjectError::LimitExceeded {
            resource: "project documents",
            ..
        })
    ));
    let mut below_locator = exact;
    below_locator.max_locator_bytes -= 1;
    assert!(matches!(
        ProjectSnapshot::new(emitted.clone(), below_locator),
        Err(ProjectError::LimitExceeded {
            resource: "project locator bytes",
            ..
        })
    ));
    assert!(matches!(
        CanonicalProjectDocuments::from_draft(draft(), below_locator),
        Err(ProjectError::LimitExceeded {
            resource: "project locator bytes",
            ..
        })
    ));
    let mut below_segments = exact;
    below_segments.max_locator_segments -= 1;
    assert!(matches!(
        ProjectSnapshot::new(emitted, below_segments),
        Err(ProjectError::LimitExceeded {
            resource: "project locator segments",
            ..
        })
    ));
    assert!(matches!(
        CanonicalProjectDocuments::from_draft(draft(), below_segments),
        Err(ProjectError::LimitExceeded {
            resource: "project locator segments",
            ..
        })
    ));

    let mut exact_records = limits();
    exact_records.max_reference_records = 4;
    exact_records.max_import_records = 2;
    exact_records.max_reimport_states = 1;
    CanonicalProjectDocuments::from_draft(draft(), exact_records)
        .expect("exact semantic count bounds");

    let mut too_many_records = exact_records;
    too_many_records.max_reference_records = 3;
    assert!(CanonicalProjectDocuments::from_draft(draft(), too_many_records).is_err());
    let mut too_many_imports = exact_records;
    too_many_imports.max_import_records = 1;
    assert!(CanonicalProjectDocuments::from_draft(draft(), too_many_imports).is_err());
    let mut reimport_overflow = draft();
    reimport_overflow.imports[0]
        .reimport_states
        .push(ReimportFieldState {
            stable_identity: "reference-source:ability:light_healing".to_owned(),
            field_path: "candidate.formula-state".to_owned(),
            baseline: Some(CandidateValue::Text("UNKNOWN".to_owned())),
            upstream: None,
            local: Some(CandidateValue::Text("UNKNOWN".to_owned())),
            decision: ReimportDecision::AdoptUpstream,
        });
    assert!(CanonicalProjectDocuments::from_draft(reimport_overflow, exact_records).is_err());
}

fn minimum_passing_json_limit(
    emitted: &BTreeMap<String, Vec<u8>>,
    set_limit: impl Fn(&mut ProjectEvidenceLimits, usize),
    upper: usize,
) -> usize {
    let mut low = 1_usize;
    let mut high = upper;
    while low < high {
        let candidate = low + ((high - low) / 2);
        let mut evidence_limits = limits();
        set_limit(&mut evidence_limits, candidate);
        if parse_with_limits(emitted.clone(), evidence_limits).is_ok() {
            high = candidate;
        } else {
            low = candidate + 1;
        }
    }
    low
}

#[test]
fn json_depth_value_and_string_budgets_accept_max_and_reject_max_plus_one() {
    let emitted = documents(draft());
    let measured_depth = minimum_passing_json_limit(
        &emitted,
        |limits, value| limits.max_json_depth = value,
        limits().max_json_depth,
    );
    let measured_values = minimum_passing_json_limit(
        &emitted,
        |limits, value| limits.max_decoded_fields = value,
        limits().max_decoded_fields,
    );
    let measured_strings = minimum_passing_json_limit(
        &emitted,
        |limits, value| limits.max_string_bytes = value,
        limits().max_string_bytes,
    );
    assert_eq!(
        (measured_depth, measured_values, measured_strings),
        (9, 73, 1_866)
    );
    for (minimum, set_limit) in [
        (
            measured_depth,
            (|limits: &mut ProjectEvidenceLimits, value| limits.max_json_depth = value)
                as fn(&mut ProjectEvidenceLimits, usize),
        ),
        (
            measured_values,
            (|limits: &mut ProjectEvidenceLimits, value| limits.max_decoded_fields = value)
                as fn(&mut ProjectEvidenceLimits, usize),
        ),
        (
            measured_strings,
            (|limits: &mut ProjectEvidenceLimits, value| limits.max_string_bytes = value)
                as fn(&mut ProjectEvidenceLimits, usize),
        ),
    ] {
        assert!(minimum > 1);
        let mut exact = limits();
        set_limit(&mut exact, minimum);
        parse_with_limits(emitted.clone(), exact).expect("exact JSON budget");
        let mut below = limits();
        set_limit(&mut below, minimum - 1);
        assert!(parse_with_limits(emitted.clone(), below).is_err());
    }
}

#[test]
fn reimport_rules_cover_unchanged_upstream_local_converged_conflict_and_deletes() {
    let b = Some(CandidateValue::Text("B".to_owned()));
    let u = Some(CandidateValue::Text("U".to_owned()));
    let l = Some(CandidateValue::Text("L".to_owned()));
    let x = Some(CandidateValue::Text("X".to_owned()));
    assert_eq!(decide_reimport(&b, &b, &b), ReimportDecision::Unchanged);
    assert_eq!(decide_reimport(&b, &u, &b), ReimportDecision::AdoptUpstream);
    assert_eq!(decide_reimport(&b, &b, &l), ReimportDecision::RetainLocal);
    assert_eq!(decide_reimport(&b, &x, &x), ReimportDecision::Converged);
    assert_eq!(decide_reimport(&b, &u, &l), ReimportDecision::Conflict);
    assert_eq!(
        decide_reimport(&b, &None, &b),
        ReimportDecision::AdoptUpstream
    );
    assert_eq!(decide_reimport(&b, &None, &l), ReimportDecision::Conflict);
    assert_eq!(
        decide_reimport(&b, &None, &None),
        ReimportDecision::Converged
    );
}

#[test]
fn inconsistent_stored_reimport_decision_fails_closed() {
    let mut candidate = draft();
    candidate.imports[0].reimport_states[0].decision = ReimportDecision::Conflict;
    assert!(matches!(
        CanonicalProjectDocuments::from_draft(candidate, limits()),
        Err(ProjectError::InvalidProject(
            "stored reimport decision is inconsistent"
        ))
    ));
}

#[test]
fn author_metadata_cannot_select_or_redirect_gameplay() {
    let first = parse(documents(draft())).expect("first project");
    let first_source = first.lower_reference_source().expect("first source");
    let mut changed = draft();
    changed.metadata[0].display_name = "Pretend Different Item".to_owned();
    changed.metadata[0].categories = vec!["loot".to_owned(), "ability".to_owned()];
    changed.metadata[0].notes = vec!["oteryn:reference.item.unrelated".to_owned()];
    let second = parse(documents(changed)).expect("second project");
    let second_source = second.lower_reference_source().expect("second source");
    assert_eq!(first_source.definitions, second_source.definitions);
    assert_ne!(first.author_metadata(), second.author_metadata());
}

#[test]
fn typed_reference_failures_are_delegated_to_the_existing_linker() {
    let mut candidate = draft();
    candidate.records.retain(|record| {
        !matches!(record, ProjectReferenceRecord::Generic { identity, .. } if identity.family == "Behavior")
    });
    let project = parse(documents(candidate)).expect("strict project parses");
    assert!(matches!(
        project.link(),
        Err(ProjectError::Content(ContentError::MissingReference { .. }))
    ));

    let mut wrong_family = draft();
    let ProjectReferenceRecord::Creature { behavior, .. } = &mut wrong_family.records[0] else {
        panic!("fixture creature");
    };
    behavior.family = "Item".to_owned();
    assert!(CanonicalProjectDocuments::from_draft(wrong_family, limits()).is_err());
}

#[test]
fn regrouped_record_documents_preserve_semantic_identity() {
    let original = parse(documents(draft())).expect("original project");
    let original_source = original.lower_reference_source().expect("original source");
    let mut grouped = documents(draft());
    let record_value: Value =
        serde_json::from_slice(&grouped.remove("records/reference.json").expect("records"))
            .expect("record JSON");
    let records = record_value["records"].as_array().expect("records array");
    let first = canonical_value(&json!({
        "schema": record_value["schema"],
        "world_id": record_value["world_id"],
        "coordinate_frame": record_value["coordinate_frame"],
        "records": records[..2],
    }));
    let second = canonical_value(&json!({
        "schema": record_value["schema"],
        "world_id": record_value["world_id"],
        "coordinate_frame": record_value["coordinate_frame"],
        "records": records[2..],
    }));
    grouped.insert("records/definitions/part-a.json".to_owned(), first.clone());
    grouped.insert("records/definitions/part-b.json".to_owned(), second.clone());

    let mut manifest: Value = serde_json::from_slice(&grouped["manifest.json"]).expect("manifest");
    let inventory = manifest["documents"].as_array_mut().expect("inventory");
    inventory.retain(|entry| entry["role"] != "reference-records");
    for (locator, bytes) in [
        ("records/definitions/part-a.json", first),
        ("records/definitions/part-b.json", second),
    ] {
        inventory.push(json!({
            "role": "reference-records",
            "schema": WORLD_PROJECT_REFERENCE_SCHEMA,
            "locator": locator,
            "byte_length": bytes.len(),
            "sha256": world_project_sha256(&bytes),
        }));
    }
    inventory.sort_by(|left, right| left["locator"].as_str().cmp(&right["locator"].as_str()));
    let manifest_bytes = canonical_value(&manifest);
    grouped.insert("manifest.json".to_owned(), manifest_bytes.clone());

    let package = PackageManifestBinding::new(
        ProductionKey::new(manifest["package_key"].as_str().expect("package key")).expect("key"),
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
    let mut lock: Value = serde_json::from_slice(&grouped["content.lock.json"]).expect("lock");
    lock["entries"][0]["package_provenance_digest"] = Value::String(
        package
            .package_provenance_digest()
            .expect("provenance")
            .as_str()
            .to_owned(),
    );
    let lock_bytes = canonical_value(&lock);
    grouped.insert("content.lock.json".to_owned(), lock_bytes.clone());
    let mut root: Value = serde_json::from_slice(&grouped["project.json"]).expect("root");
    root["manifest_sha256"] = Value::String(world_project_sha256(&manifest_bytes));
    root["content_lock_sha256"] = Value::String(world_project_sha256(&lock_bytes));
    grouped.insert("project.json".to_owned(), canonical_value(&root));

    let regrouped = parse(grouped).expect("regrouped project");
    let regrouped_source = regrouped
        .lower_reference_source()
        .expect("regrouped source");
    assert_eq!(regrouped_source.definitions, original_source.definitions);
}
