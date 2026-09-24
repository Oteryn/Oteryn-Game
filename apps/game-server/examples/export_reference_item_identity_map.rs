use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::PathBuf;

use oteryn_game_server::content::{
    CandidateValue, protected_cw2_b1_full_item_family_import, world_project_sha256,
};
use serde::Serialize;

const B1_EVIDENCE: &[u8] = include_bytes!(
    "../../../docs/agents/evidence/OTV2-20260919-content-world-cw2-b1-item-identity-catalog.json"
);
const SCHEMA: &str = "OTERYN_PROTECTED_ITEM_IDENTITY_MAP_EXPORT/v1";
const PROFILE: &str = "OTERYN_PROTECTED_ITEM_IDENTITY_MAP_EXPORTER/v1";
const B1_SHA256: &str = "7836c78cad130a5c404f648e76e0823f53ae6a34c6952b9b88c8bed2e50d96a7";
const ALLOCATION_SHA256: &str = "ee9219ccf9d8b2350911abca321507ff924ccd4cb83196efd08b91fbdf098966";
const ITEM_COUNT: usize = 38_157;
const PRESERVED_COUNT: usize = 64;
const OPAQUE_COUNT: usize = 38_093;

#[derive(Serialize)]
struct Export<'a> {
    schema: &'a str,
    profile: &'a str,
    producer: &'a str,
    protected_catalog_sha256: &'a str,
    item_count: usize,
    preserved_semantic_bindings: usize,
    opaque_registry_allocations: usize,
    allocation_digest_sha256: &'a str,
    records: Vec<Row>,
}

#[derive(Serialize)]
struct Row {
    source_item_id: u64,
    native_key: String,
    native_revision: String,
    identity_origin: &'static str,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: export_reference_item_identity_map <output.json>")?;
    if env::args_os().nth(2).is_some() {
        return Err("usage: export_reference_item_identity_map <output.json>".into());
    }

    let imported = protected_cw2_b1_full_item_family_import(B1_EVIDENCE)?;
    if imported.allocation_digest_sha256 != ALLOCATION_SHA256 {
        return Err("protected allocation digest mismatch".into());
    }

    let mut rows = Vec::with_capacity(imported.batch.candidates.len());
    for candidate in &imported.batch.candidates {
        let source_item_id = candidate
            .source_numeric_id
            .ok_or("canonical Item candidate lacks source numeric identity")?;
        let mut bindings = candidate.normalized_fields.iter().filter_map(|field| {
            if field.field_path != "binding.native-item" {
                return None;
            }
            match &field.value {
                CandidateValue::NativeItemBinding(binding) => Some(binding),
                _ => None,
            }
        });
        let binding = bindings
            .next()
            .ok_or("canonical Item candidate lacks native binding")?;
        if bindings.next().is_some() {
            return Err("canonical Item candidate has duplicate native bindings".into());
        }
        let identity_origin = match candidate.disposition_reason.as_str() {
            "PROTECTED_EXISTING_SEMANTIC_BINDING" => "PRESERVED_SEMANTIC_BINDING",
            "FAMILY_SCALE_IDENTITY_ONLY_GAMEPLAY_SEMANTICS_UNRESOLVED" => {
                "OPAQUE_REGISTRY_ALLOCATION"
            }
            _ => return Err("canonical Item candidate disposition is unknown".into()),
        };
        rows.push(Row {
            source_item_id,
            native_key: binding.identity.key.clone(),
            native_revision: binding.identity.revision.clone(),
            identity_origin,
        });
    }
    rows.sort_by_key(|row| row.source_item_id);

    let source_ids = rows
        .iter()
        .map(|row| row.source_item_id)
        .collect::<BTreeSet<_>>();
    let native_keys = rows
        .iter()
        .map(|row| row.native_key.as_str())
        .collect::<BTreeSet<_>>();
    let preserved = rows
        .iter()
        .filter(|row| row.identity_origin == "PRESERVED_SEMANTIC_BINDING")
        .count();
    let opaque = rows
        .iter()
        .filter(|row| row.identity_origin == "OPAQUE_REGISTRY_ALLOCATION")
        .count();
    if rows.len() != ITEM_COUNT
        || source_ids.len() != ITEM_COUNT
        || native_keys.len() != ITEM_COUNT
        || preserved != PRESERVED_COUNT
        || opaque != OPAQUE_COUNT
    {
        return Err("canonical Item identity map closure mismatch".into());
    }

    let mut digest_input = Vec::with_capacity(ITEM_COUNT * 48);
    for row in &rows {
        digest_input.extend_from_slice(row.source_item_id.to_string().as_bytes());
        digest_input.push(0);
        digest_input.extend_from_slice(row.native_key.as_bytes());
        digest_input.push(b'\n');
    }
    if world_project_sha256(&digest_input) != ALLOCATION_SHA256 {
        return Err("canonical Item identity map digest reconstruction mismatch".into());
    }

    let export = Export {
        schema: SCHEMA,
        profile: PROFILE,
        producer: "protected_cw2_b1_full_item_family_import",
        protected_catalog_sha256: B1_SHA256,
        item_count: ITEM_COUNT,
        preserved_semantic_bindings: preserved,
        opaque_registry_allocations: opaque,
        allocation_digest_sha256: ALLOCATION_SHA256,
        records: rows,
    };
    let mut bytes = serde_json::to_vec(&export)?;
    bytes.push(b'\n');
    fs::write(output, bytes)?;
    Ok(())
}
