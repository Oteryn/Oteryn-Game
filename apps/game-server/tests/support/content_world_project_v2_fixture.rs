//! Shared canonical v2 package fixture, owned once across integration targets.

use oteryn_game_server::content::*;
use serde_json::{Value, json};

pub(crate) fn limits() -> ProjectEvidenceLimits {
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

pub(crate) fn core() -> ProjectDraft {
    ProjectDraft {
        project_revision: "project-r1".into(),
        package_key: "oteryn:content.world-project".into(),
        semantic_schema_version: "reference-schema-v1".into(),
        licensing_metadata: "license:project-owned-v1".into(),
        world_id: "0123456789ab70cd8ef0123456789abc".into(),
        coordinate_frame: "global-target-2026-09-27".into(),
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

pub(crate) fn reference(family: ProjectV2Family, key: &str) -> ProjectV2DefinitionRef {
    ProjectV2DefinitionRef {
        family,
        key: key.into(),
        revision: "definition-r1".into(),
    }
}

pub(crate) fn canonical(value: &Value) -> Vec<u8> {
    let mut bytes = serde_json::to_vec(value).expect("encode JSON");
    bytes.push(b'\n');
    bytes
}

pub(crate) fn rebind_manifest_and_lock(
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
