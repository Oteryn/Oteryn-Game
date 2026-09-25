use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use oteryn_game_server::content::{
    CW2_B1_FULL_ITEM_FAMILY_COUNT, CanonicalProjectDocuments, ProjectDraft,
    ProjectEvidenceLimits, ProjectV2Draft, ProjectV2State,
    protected_cw2_b1_promoted_item_family_import,
};
use sha2::{Digest, Sha256};

const B1_EVIDENCE: &[u8] = include_bytes!(
    "../../../docs/agents/evidence/OTV2-20260919-content-world-cw2-b1-item-identity-catalog.json"
);
const DOCUMENT_COUNT: usize = 11;
const FULL_FAMILY_MAX_DECODED_FIELDS: usize = 2_098_651;
const FULL_FAMILY_MAX_STRING_BYTES: usize = 42_332_603;

fn limits() -> ProjectEvidenceLimits {
    ProjectEvidenceLimits {
        max_documents: DOCUMENT_COUNT,
        max_document_bytes: 96_000_000,
        max_total_bytes: 160_000_000,
        max_json_depth: 24,
        max_decoded_fields: FULL_FAMILY_MAX_DECODED_FIELDS,
        max_string_bytes: FULL_FAMILY_MAX_STRING_BYTES,
        max_locator_bytes: 160,
        max_locator_segments: 8,
        max_reference_records: CW2_B1_FULL_ITEM_FAMILY_COUNT,
        max_import_records: CW2_B1_FULL_ITEM_FAMILY_COUNT,
        max_reimport_states: CW2_B1_FULL_ITEM_FAMILY_COUNT,
    }
}

fn output_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut arguments = env::args_os().skip(1);
    let flag = arguments.next();
    let Some(value) = arguments.next() else {
        return Err("usage: materialize_content_world_project_v2 --output-root <path>".into());
    };
    if flag.as_deref() != Some(std::ffi::OsStr::new("--output-root"))
        || arguments.next().is_some()
    {
        return Err("usage: materialize_content_world_project_v2 --output-root <path>".into());
    }
    Ok(PathBuf::from(value))
}

fn require_fresh_root(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if fs::symlink_metadata(root).is_ok() {
        return Err(format!("output root already exists: {}", root.display()).into());
    }
    let parent = root
        .parent()
        .ok_or("output root must have an existing parent")?;
    let metadata = fs::symlink_metadata(parent)?;
    if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
        return Err("output parent must be a real directory".into());
    }
    Ok(())
}

fn write_documents(
    root: &Path,
    documents: &CanonicalProjectDocuments,
) -> Result<String, Box<dyn std::error::Error>> {
    require_fresh_root(root)?;
    fs::create_dir(root)?;
    let mut tree = Sha256::new();
    for (locator, bytes) in documents.documents() {
        tree.update((locator.len() as u64).to_be_bytes());
        tree.update(locator.as_bytes());
        tree.update((bytes.len() as u64).to_be_bytes());
        tree.update(bytes);

        let destination = root.join(locator);
        let parent = destination.parent().ok_or("document locator has no parent")?;
        fs::create_dir_all(parent)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    Ok(format!("{:x}", tree.finalize()))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = output_root()?;
    let promoted = protected_cw2_b1_promoted_item_family_import(B1_EVIDENCE)?;
    if promoted.family.records.len() != CW2_B1_FULL_ITEM_FAMILY_COUNT {
        return Err("protected promoted Item family count drifted".into());
    }
    let documents = CanonicalProjectDocuments::from_v2_draft(
        ProjectV2Draft {
            core: ProjectDraft {
                project_revision: "g4-canonical-worldproject-r1".to_owned(),
                package_key: "oteryn:content.world-project".to_owned(),
                semantic_schema_version: "reference-schema-v1".to_owned(),
                licensing_metadata: "PENDING".to_owned(),
                world_id: "0123456789ab70cd8ef0123456789abc".to_owned(),
                coordinate_frame: "global-target-2026-07-28".to_owned(),
                records: promoted.family.records,
                imports: vec![promoted.family.batch],
                metadata: Vec::new(),
            },
            state: ProjectV2State::default(),
        },
        limits(),
    )?;
    if documents.documents().len() != DOCUMENT_COUNT {
        return Err("canonical WorldProject/v2 document count drifted".into());
    }
    let tree_sha256 = write_documents(&root, &documents)?;
    println!(
        "documents={DOCUMENT_COUNT} items={CW2_B1_FULL_ITEM_FAMILY_COUNT} promoted_items={} promoted_fields={} tree_sha256={tree_sha256}",
        promoted.promoted_items, promoted.promoted_fields
    );
    Ok(())
}
