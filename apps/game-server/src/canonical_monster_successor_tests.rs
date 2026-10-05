//! Root-only local authoring driver: reuse the current canonical writer, never regenerate Items.
use crate::content::{
    CanonicalProjectDocuments, ProjectEvidenceLimits, ProjectReferenceRecord, ProjectSnapshot,
    ProjectV2Draft,
};
use serde::de::DeserializeOwned;
use std::{
    collections::BTreeMap,
    error::Error,
    fs,
    path::{Path, PathBuf},
};
fn limits() -> ProjectEvidenceLimits {
    // Existing repository limits, with exact source16 and reimport404 successor bounds.
    ProjectEvidenceLimits {
        max_documents: 11,
        max_document_bytes: 96_000_000,
        max_total_bytes: 160_000_000,
        max_json_depth: 24,
        max_decoded_fields: 2_400_000,
        max_string_bytes: 43_000_000,
        max_locator_bytes: 160,
        max_locator_segments: 8,
        max_reference_records: 65_310,
        max_import_records: 16,
        max_reimport_states: 404,
    }
}
fn field<T: DeserializeOwned>(root: &Path, path: &str, name: &str) -> Result<T, Box<dyn Error>> {
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(root.join(path))?)?;
    let value = value
        .as_object_mut()
        .ok_or("object expected")?
        .remove(name)
        .ok_or("field missing")?;
    Ok(serde_json::from_value(value)?)
}
fn qualify() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = [
        "OTERYN_MONSTER_TRUSTED_WORLD_ROOT",
        "OTERYN_MONSTER_MERGED_WORLD_ROOT",
        "OTERYN_MONSTER_SEALED_WORLD_OUTPUT",
    ]
    .iter()
    .map(|name| std::env::var_os(name).ok_or("required qualification environment missing"))
    .collect::<Result<_, _>>()?;
    if args.len() != 3 {
        return Err("expected TRUSTED_MAIN11_ROOT MERGED_WORLD4_ROOT OUTPUT11_ROOT".into());
    }
    let trusted = PathBuf::from(&args[0]);
    let merged = PathBuf::from(&args[1]);
    let output = PathBuf::from(&args[2]);
    if trusted == output || merged == output || output.exists() {
        return Err("fresh isolated output required".into());
    }
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(trusted.join("manifest.json"))?)?;
    let mut locators = vec![
        "project.json".to_owned(),
        "manifest.json".to_owned(),
        "content.lock.json".to_owned(),
    ];
    for document in manifest["documents"].as_array().ok_or("documents array")? {
        locators.push(document["locator"].as_str().ok_or("locator")?.to_owned());
    }
    let mut documents = BTreeMap::new();
    for locator in locators {
        documents.insert(locator.clone(), fs::read(trusted.join(locator))?);
    }
    let original = ProjectSnapshot::new(documents, limits())?.parse(limits())?;
    let mut draft = ProjectV2Draft::from_project(&original);
    let original_items = draft.state.item_authoring.clone();
    let original_declarations = draft.state.declarations.clone();
    if original_items.len() != 700 {
        return Err("original native Item authoring count not700".into());
    }
    draft.core.project_revision = "monster-canonical-1863-20261004-r1".into();
    draft.core.records = field(&merged, "definitions/reference.json", "records")?;
    draft.core.imports = field(&merged, "provenance/imports.json", "batches")?;
    draft.state.declarations = field(&merged, "definitions/declarations.json", "records")?;
    draft.state.authoring_profiles = field(
        &merged,
        "definitions/declarations.json",
        "authoring_profiles",
    )?;
    draft.state.item_authoring = field(&merged, "definitions/declarations.json", "item_authoring")?;
    draft.state.sources = field(&merged, "provenance/sources.json", "sources")?;
    draft.state.source_identity_bindings = field(
        &merged,
        "provenance/sources.json",
        "source_identity_bindings",
    )?;
    if draft.state.item_authoring != original_items {
        return Err("current main Item authoring changed".into());
    }
    let creatures = draft
        .core
        .records
        .iter()
        .filter(|r| matches!(r, ProjectReferenceRecord::Creature { .. }))
        .count();
    if creatures != 1863 {
        return Err("Creature count is not1863".into());
    }
    if draft.core.records.len() != 62456
        || draft.state.sources.len() != 16
        || draft.core.imports.len() != 18
    {
        return Err("successor records/sources/batches count drift".into());
    }
    if draft
        .core
        .imports
        .iter()
        .map(|b| b.reimport_states.len())
        .sum::<usize>()
        != 404
    {
        return Err("reimport state count drift".into());
    }
    if draft
        .state
        .declarations
        .iter()
        .filter(|r| matches!(r, crate::content::ProjectV2Declaration::Encounter { .. }))
        .count()
        != 108
    {
        return Err("encounter count drift".into());
    }
    if !original_declarations
        .iter()
        .all(|d| draft.state.declarations.contains(d))
    {
        return Err("current declarations/reward variants lost".into());
    }
    // The current writer sorts/validates/re-admits its output and computes every package seal.
    let canonical = CanonicalProjectDocuments::from_v2_draft(draft, limits())?;
    let recaptured =
        ProjectSnapshot::new(canonical.documents().clone(), limits())?.parse(limits())?;
    if ProjectV2Draft::from_project(&recaptured)
        .core
        .records
        .iter()
        .filter(|r| matches!(r, ProjectReferenceRecord::Creature { .. }))
        .count()
        != 1863
    {
        return Err("recaptured count drift".into());
    }
    let checked = ProjectV2Draft::from_project(&recaptured);
    if checked.state.item_authoring != original_items
        || !original_declarations
            .iter()
            .all(|d| checked.state.declarations.contains(d))
    {
        return Err("canonical roundtrip preservation drift".into());
    }
    let repeated = CanonicalProjectDocuments::from_v2_draft(checked, limits())?;
    if canonical.documents() != repeated.documents() {
        return Err("canonical encoding not idempotent".into());
    }
    for (locator, bytes) in canonical.documents() {
        let path = output.join(locator);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?
        }
        fs::write(path, bytes)?;
    }
    println!(
        "PASS canonical11 parse+rewrite+re-admit creatures1863; output={} (no server)",
        output.display()
    );
    Ok(())
}

#[test]
#[ignore = "root-only explicit isolated canonical successor qualification"]
fn canonical_monster_successor_reseal_and_preservation() -> Result<(), Box<dyn Error>> {
    qualify()
}
