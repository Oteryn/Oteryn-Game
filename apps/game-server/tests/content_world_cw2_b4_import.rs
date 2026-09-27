#![allow(clippy::expect_used, clippy::panic)]

use oteryn_game_server::content::*;
use serde_json::{Value, json};

const B4_EVIDENCE: &[u8] = include_bytes!(
    "../../../docs/agents/evidence/OTV2-20260919-content-world-cw2-b4-ability-effect-formula-evidence.json"
);

fn limits() -> ProjectEvidenceLimits {
    ProjectEvidenceLimits {
        max_documents: 6,
        max_document_bytes: 16_384,
        max_total_bytes: 65_536,
        max_json_depth: 20,
        max_decoded_fields: 1_024,
        max_string_bytes: 32_768,
        max_locator_bytes: 160,
        max_locator_segments: 8,
        max_reference_records: 1,
        max_import_records: 2,
        max_reimport_states: 10,
    }
}

fn caller_owned_draft(batch: ImportBatch) -> ProjectDraft {
    ProjectDraft {
        project_revision: "project-r1".to_owned(),
        package_key: "oteryn:content.world-project".to_owned(),
        semantic_schema_version: "reference-schema-v1".to_owned(),
        licensing_metadata: "license:project-owned-v1".to_owned(),
        world_id: "0123456789ab70cd8ef0123456789abc".to_owned(),
        coordinate_frame: "global-target-2026-07-28".to_owned(),
        records: Vec::new(),
        imports: vec![batch],
        metadata: Vec::new(),
    }
}

fn changed(mutator: impl FnOnce(&mut Value)) -> Vec<u8> {
    let mut evidence: Value = serde_json::from_slice(B4_EVIDENCE).expect("protected JSON");
    mutator(&mut evidence);
    serde_json::to_vec(&evidence).expect("mutated JSON")
}

#[test]
fn protected_batch_round_trips_through_the_existing_canonical_project_api() {
    let batch = protected_cw2_b4_import_batch(B4_EVIDENCE).expect("protected B4 batch");
    assert_eq!(batch.source_repository, "Oteryn/Oteryn-Game");
    assert_eq!(
        batch.source_revision,
        "03a821edd828e24ccff6e2cb7fc819a776cbd238"
    );
    assert_eq!(
        batch.source_artifact_sha256,
        PROTECTED_CW2_B4_EVIDENCE_SHA256
    );
    assert_eq!(
        batch.mapper_revision,
        "e6d98aadd352ad36b466970e1f7182e1bf93643b"
    );
    assert_eq!(
        batch.mapper_sha256,
        "bc68f0f63a5dd6ea5ee7a3c20b708d6ea78c6033a2f5dddb518c49d6f45f8666"
    );
    assert_eq!(batch.access_disposition, "PENDING");
    assert_eq!(batch.candidates.len(), 2);
    assert_eq!(batch.reimport_states.len(), 10);

    let canonical = CanonicalProjectDocuments::from_draft(caller_owned_draft(batch), limits())
        .expect("canonical project");
    let snapshot = canonical
        .clone()
        .into_snapshot(limits())
        .expect("canonical snapshot");
    let project = snapshot.parse(limits()).expect("canonical project parses");
    assert_eq!(project.imports().len(), 1);
    assert!(
        project.imports()[0]
            .candidates
            .iter()
            .all(
                |candidate| candidate.closure_disposition == CandidateDisposition::Blocked
                    && candidate.source_numeric_id.is_none()
                    && candidate.candidate_formula == "UNKNOWN"
                    && candidate.evidence_class == "UNKNOWN"
            )
    );
    assert!(
        project
            .lower_reference_source()
            .expect("candidate-only project lowers")
            .definitions
            .is_empty()
    );
    assert_eq!(
        project
            .canonical_documents(limits())
            .expect("canonical rewrite")
            .documents(),
        canonical.documents()
    );
}

#[test]
fn repeated_import_is_deterministic_and_retains_explicit_losses() {
    let first = protected_cw2_b4_import_batch(B4_EVIDENCE).expect("first import");
    let second = protected_cw2_b4_import_batch(B4_EVIDENCE).expect("second import");
    assert_eq!(first, second);
    assert_eq!(
        first
            .candidates
            .iter()
            .map(|candidate| candidate.source_candidate_id.as_str())
            .collect::<Vec<_>>(),
        [
            "reference-source:ability:ice_strike",
            "reference-source:ability:light_healing",
        ]
    );
    for candidate in &first.candidates {
        let fields = candidate
            .normalized_fields
            .iter()
            .map(|field| (field.field_path.as_str(), &field.value))
            .collect::<std::collections::BTreeMap<_, _>>();
        assert!(matches!(
            fields.get("candidate.formula-unknown-fields"),
            Some(CandidateValue::Text(losses)) if !losses.is_empty()
        ));
        assert_eq!(
            fields.get("evidence.product-digest-sha256"),
            Some(&&CandidateValue::Text(
                PROTECTED_CW2_B4_PRODUCT_SHA256.to_owned()
            ))
        );
    }
    assert!(first.reimport_states.iter().all(|state| {
        state.decision == ReimportDecision::Unchanged
            && state.baseline == state.upstream
            && state.upstream == state.local
    }));
}

#[test]
fn input_bound_is_checked_before_json_decoding() {
    assert_eq!(B4_EVIDENCE.len(), PROTECTED_CW2_B4_EVIDENCE_MAX_BYTES);
    protected_cw2_b4_import_batch(B4_EVIDENCE).expect("exact input limit");

    let mut above = B4_EVIDENCE.to_vec();
    above.push(b' ');
    assert!(matches!(
        protected_cw2_b4_import_batch(&above),
        Err(ProtectedCw2B4ImportError::InputLimitExceeded {
            actual: 7_429,
            limit: 7_428
        })
    ));
}

#[test]
fn protected_digest_and_material_semantics_fail_closed_on_drift() {
    let same_semantics_different_bytes =
        serde_json::to_vec(&serde_json::from_slice::<Value>(B4_EVIDENCE).expect("protected JSON"))
            .expect("compact JSON");
    assert!(matches!(
        protected_cw2_b4_import_batch(&same_semantics_different_bytes),
        Err(ProtectedCw2B4ImportError::EvidenceMismatch(
            "evidence byte digest"
        ))
    ));

    let provenance = changed(|evidence| {
        evidence["source_snapshot"]["revision"] = json!("different-revision");
    });
    let extra = changed(|evidence| {
        evidence["unexpected"] = json!(true);
    });
    let missing = changed(|evidence| {
        evidence["ability_effect_formula_candidates"]
            .as_array_mut()
            .expect("candidate array")
            .pop();
    });
    let native = changed(|evidence| {
        evidence["ability_effect_formula_candidates"][0]["native_ability_identity"]["content_key"] =
            json!(0);
    });
    let formula = changed(|evidence| {
        evidence["ability_effect_formula_candidates"][0]["effect_to_formula"]["formula_state"] =
            json!("RESOLVED");
    });
    let disposition = changed(|evidence| {
        evidence["ability_effect_formula_candidates"][0]["executable_promotion"]["disposition"] =
            json!("PROMOTED");
    });
    let product_digest = changed(|evidence| {
        evidence["product_digest_sha256"] =
            json!("0000000000000000000000000000000000000000000000000000000000000000");
    });

    for (name, drift) in [
        ("provenance", provenance),
        ("extra field", extra),
        ("missing candidate", missing),
        ("native identity", native),
        ("formula", formula),
        ("disposition", disposition),
        ("product digest", product_digest),
    ] {
        let error = protected_cw2_b4_import_batch(&drift).expect_err(name);
        if name == "extra field" {
            assert!(
                matches!(error, ProtectedCw2B4ImportError::InputLimitExceeded { .. }),
                "{name}: {error}"
            );
        } else {
            assert!(
                matches!(error, ProtectedCw2B4ImportError::EvidenceMismatch(_)),
                "{name}: {error}"
            );
        }
    }
}
