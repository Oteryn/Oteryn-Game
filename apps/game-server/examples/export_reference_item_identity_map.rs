use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::PathBuf;

use oteryn_game_server::content::{
    CW2_B1_FULL_ITEM_FAMILY_COUNT, CW2_B1_NATIVE_ITEM_BATCH_COUNT, CW2_B1_OPAQUE_ITEM_COUNT,
    CW2_B1_RETIRED_EPOCH1_ALLOCATION_SHA256, CW2_B1_SOURCE_CATALOG_ROW_COUNT, CandidateValue,
    ProtectedCw2B1FullItemFamilyImport, protected_cw2_b1_full_item_family_import,
    protected_cw2_b1_retired_item_family_allocation, world_project_sha256,
};
use serde::Serialize;

const B1_EVIDENCE: &[u8] = include_bytes!(
    "../../../docs/agents/evidence/OTV2-20260919-content-world-cw2-b1-item-identity-catalog.json"
);
const SCHEMA: &str = "OTERYN_PROTECTED_ITEM_IDENTITY_MAP_EXPORT/v1";
const PROFILE: &str = "OTERYN_PROTECTED_ITEM_IDENTITY_MAP_EXPORTER/v1";
const B1_SHA256: &str = "7836c78cad130a5c404f648e76e0823f53ae6a34c6952b9b88c8bed2e50d96a7";
/// `(source id, Tibia key)` digest of the A12 full family (ITEM-ID-1b).
const ALLOCATION_SHA256: &str = "53a6c2e3930a7ba2d953a0f115f6fc849f2ab92caf76440a82f5aa33d43bf795";
const USAGE: &str = "usage: export_reference_item_identity_map <output.json> [--retired]";

/// The export in force: every admitted Crystal row at its A12 Tibia-id key.
#[derive(Serialize)]
struct Export<'a> {
    schema: &'a str,
    profile: &'a str,
    producer: &'a str,
    protected_catalog_sha256: &'a str,
    item_count: usize,
    preserved_semantic_bindings: usize,
    identity_only_allocations: usize,
    allocation_digest_sha256: &'a str,
    records: Vec<Row>,
}

/// `--retired`: the retired epoch-1 registry allocation, byte-identical to the pre-A12 export.
/// It exists only so the pinned evidence pipelines keyed by retired keys (the classification
/// crosswalk and its successors) keep reproducing their history; it is never a key source.
#[derive(Serialize)]
struct RetiredExport<'a> {
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

fn rows(
    imported: &ProtectedCw2B1FullItemFamilyImport,
    identity_only_origin: &'static str,
) -> Result<Vec<Row>, Box<dyn std::error::Error>> {
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
            "FAMILY_SCALE_IDENTITY_ONLY_GAMEPLAY_SEMANTICS_UNRESOLVED" => identity_only_origin,
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
    Ok(rows)
}

/// Closure and digest checks shared by both modes; returns `(preserved, identity_only)`.
fn check_closure(
    rows: &[Row],
    item_count: usize,
    identity_only_origin: &str,
    allocation_sha256: &str,
) -> Result<(usize, usize), Box<dyn std::error::Error>> {
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
    let identity_only = rows
        .iter()
        .filter(|row| row.identity_origin == identity_only_origin)
        .count();
    if rows.len() != item_count
        || source_ids.len() != item_count
        || native_keys.len() != item_count
        || preserved != CW2_B1_NATIVE_ITEM_BATCH_COUNT
        || identity_only != item_count - CW2_B1_NATIVE_ITEM_BATCH_COUNT
    {
        return Err("canonical Item identity map closure mismatch".into());
    }

    let mut digest_input = Vec::with_capacity(item_count * 48);
    for row in rows {
        digest_input.extend_from_slice(row.source_item_id.to_string().as_bytes());
        digest_input.push(0);
        digest_input.extend_from_slice(row.native_key.as_bytes());
        digest_input.push(b'\n');
    }
    if world_project_sha256(&digest_input) != allocation_sha256 {
        return Err("canonical Item identity map digest reconstruction mismatch".into());
    }
    Ok((preserved, identity_only))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1);
    let output = args.next().map(PathBuf::from).ok_or(USAGE)?;
    let retired = match args.next() {
        None => false,
        Some(flag) if flag == "--retired" => true,
        Some(_) => return Err(USAGE.into()),
    };
    if args.next().is_some() {
        return Err(USAGE.into());
    }

    let mut bytes = if retired {
        let imported = protected_cw2_b1_retired_item_family_allocation(B1_EVIDENCE)?;
        let rows = rows(&imported, "OPAQUE_REGISTRY_ALLOCATION")?;
        let (preserved, opaque) = check_closure(
            &rows,
            CW2_B1_SOURCE_CATALOG_ROW_COUNT,
            "OPAQUE_REGISTRY_ALLOCATION",
            CW2_B1_RETIRED_EPOCH1_ALLOCATION_SHA256,
        )?;
        if opaque != CW2_B1_OPAQUE_ITEM_COUNT {
            return Err("retired opaque allocation count mismatch".into());
        }
        serde_json::to_vec(&RetiredExport {
            schema: SCHEMA,
            profile: PROFILE,
            producer: "protected_cw2_b1_full_item_family_import",
            protected_catalog_sha256: B1_SHA256,
            item_count: CW2_B1_SOURCE_CATALOG_ROW_COUNT,
            preserved_semantic_bindings: preserved,
            opaque_registry_allocations: opaque,
            allocation_digest_sha256: CW2_B1_RETIRED_EPOCH1_ALLOCATION_SHA256,
            records: rows,
        })?
    } else {
        let imported = protected_cw2_b1_full_item_family_import(B1_EVIDENCE)?;
        if imported.allocation_digest_sha256 != ALLOCATION_SHA256 {
            return Err("protected allocation digest mismatch".into());
        }
        let rows = rows(&imported, "TIBIA_ID_IDENTITY_ONLY")?;
        let (preserved, identity_only) = check_closure(
            &rows,
            CW2_B1_FULL_ITEM_FAMILY_COUNT,
            "TIBIA_ID_IDENTITY_ONLY",
            ALLOCATION_SHA256,
        )?;
        serde_json::to_vec(&Export {
            schema: SCHEMA,
            profile: PROFILE,
            producer: "protected_cw2_b1_full_item_family_import",
            protected_catalog_sha256: B1_SHA256,
            item_count: CW2_B1_FULL_ITEM_FAMILY_COUNT,
            preserved_semantic_bindings: preserved,
            identity_only_allocations: identity_only,
            allocation_digest_sha256: ALLOCATION_SHA256,
            records: rows,
        })?
    };
    bytes.push(b'\n');
    fs::write(output, bytes)?;
    Ok(())
}
