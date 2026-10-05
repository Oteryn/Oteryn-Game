#![allow(clippy::expect_used)]

#[path = "../examples/support/item_fx_audio_raw_import.rs"]
mod raw;
use oteryn_game_server::content::*;
use serde_json::Value;

fn limits() -> ProjectEvidenceLimits {
    ProjectEvidenceLimits {
        max_documents: 11,
        max_document_bytes: 96_000_000,
        max_total_bytes: 160_000_000,
        max_json_depth: 24,
        max_decoded_fields: 10_000,
        max_string_bytes: 7_000_000,
        max_locator_bytes: 160,
        max_locator_segments: 8,
        max_reference_records: 1,
        max_import_records: 1,
        max_reimport_states: raw::STATE_COUNT,
    }
}
fn draft() -> ProjectV2Draft {
    let mut imports = Vec::new();
    assert_eq!(raw::append(&mut imports), Ok(296));
    ProjectV2Draft {
        core: ProjectDraft {
            project_revision: "raw-intake-r1".into(),
            package_key: "oteryn:content.world-project".into(),
            semantic_schema_version: "reference-schema-v1".into(),
            licensing_metadata: "PENDING".into(),
            world_id: "0123456789ab70cd8ef0123456789abc".into(),
            coordinate_frame: "global-target-2026-09-27".into(),
            records: vec![],
            imports,
            metadata: vec![],
        },
        state: ProjectV2State::default(),
    }
}
#[test]
fn raw_evidence_is_typed_inert_lossless_and_canonical_roundtrip() {
    let candidate = draft();
    let states = &candidate.core.imports[0].reimport_states;
    assert_eq!(states.len(), 296);
    assert_eq!(
        states
            .iter()
            .filter(|s| s.field_path.ends_with("raw-contextual-frame"))
            .count(),
        1
    );
    assert!(states.iter().all(|s| s.baseline.is_none()
        && s.local.is_none()
        && matches!(&s.upstream, Some(CandidateValue::Text(_)))
        && s.decision == decide_reimport(&s.baseline, &s.upstream, &s.local)));
    let docs = CanonicalProjectDocuments::from_v2_draft(candidate, limits())
        .expect("raw canonical writer");
    let parsed = ProjectSnapshot::new(docs.documents().clone(), limits())
        .expect("bounded snapshot")
        .parse(limits())
        .expect("strict source parser");
    assert_eq!(
        parsed
            .canonical_documents(limits())
            .expect("rewrite")
            .documents(),
        docs.documents()
    );
    assert!(parsed.migrate_to_v2().core.records.is_empty());
    assert!(parsed.v2().expect("v2 state").item_authoring.is_empty());
}
#[test]
fn existing_all_twenty_six_batches_preserved_idempotently_and_conflict_atomic() {
    let value: Value = serde_json::from_slice(include_bytes!(
        "../../../content/world/provenance/imports.json"
    ))
    .expect("repository imports");
    let mut batches: Vec<ImportBatch> =
        serde_json::from_value(value["batches"].clone()).expect("typed repository imports");
    batches.retain(|b| b.batch_id != raw::BATCH_ID);
    assert_eq!(batches.len(), 26);
    assert_eq!(
        batches
            .iter()
            .map(|b| b.reimport_states.len())
            .sum::<usize>(),
        104
    );
    let prior = batches.clone();
    assert_eq!(raw::append(&mut batches), Ok(296));
    assert_eq!(&batches[..26], prior.as_slice());
    let once = batches.clone();
    assert_eq!(raw::append(&mut batches), Ok(0));
    assert_eq!(batches, once);
    batches
        .last_mut()
        .expect("raw batch")
        .mapper_revision
        .push_str("-opposed");
    let opposed = batches.clone();
    assert!(raw::append(&mut batches).is_err());
    assert_eq!(batches, opposed);
    batches.push(once.last().expect("raw batch").clone());
    let duplicate = batches.clone();
    assert!(raw::append(&mut batches).is_err());
    assert_eq!(batches, duplicate);
}
#[test]
fn strict_unknown_duplicate_malformed_text_and_false_decision_reject() {
    assert!(CanonicalProjectDocuments::from_v2_draft(draft(), limits()).is_ok());
    let text = std::str::from_utf8(raw::PAYLOAD).expect("payload utf8");
    let unknown = text.replacen("{", "{\"unowned\":true,", 1);
    let duplicate = text.replacen("{", "{\"batch_id\":\"duplicate\",", 1);
    for malformed in [unknown, duplicate] {
        assert!(serde_json::from_str::<ImportBatch>(&malformed).is_err());
    }
    let mut malformed: Value = serde_json::from_slice(raw::PAYLOAD).expect("payload");
    malformed["reimport_states"][0]["upstream"]["value"] = Value::Bool(true);
    assert!(serde_json::from_value::<ImportBatch>(malformed).is_err());
    let mut false_decision = draft();
    false_decision.core.imports[0].reimport_states[295].decision = ReimportDecision::Unchanged;
    assert!(CanonicalProjectDocuments::from_v2_draft(false_decision, limits()).is_err());
    let mut finite = limits();
    finite.max_reimport_states = 295;
    assert!(CanonicalProjectDocuments::from_v2_draft(draft(), finite).is_err());
}
