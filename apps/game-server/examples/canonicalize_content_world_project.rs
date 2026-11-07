//! Strict current-package normalization; no predecessor admission or source promotion.
use oteryn_game_server::content::{
    NpcDataCatalogue, WorldProject, capture_world_project, npc_catalogue_preproduction_limits,
};
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    env,
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn sha(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn rows<T: Serialize>(values: &[T]) -> Result<Value> {
    let mut hashes = values
        .iter()
        .map(|v| serde_json::to_vec(v).map(|b| sha(&b)))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    hashes.sort();
    Ok(json!({"count":values.len(),"sha256":sha(&serde_json::to_vec(&hashes)?)}))
}
// Compare every typed collection as a multiset; nested authored matching order
// remains inside each row hash. Root/package scalars are compared separately.
fn semantics(project: &WorldProject) -> Result<Value> {
    let d = project.migrate_to_v2();
    let c = &d.core;
    let s = &d.state;
    Ok(
        json!({"identity":[c.project_revision,c.package_key,c.semantic_schema_version,c.licensing_metadata,c.world_id,c.coordinate_frame],
 "records":rows(&c.records)?,"imports":rows(&c.imports)?,"metadata":rows(&c.metadata)?,
 "declarations":rows(&s.declarations)?,"item_authoring":rows(&s.item_authoring)?,"authoring_profiles":rows(&s.authoring_profiles)?,
 "worlds":rows(&s.worlds)?,"placements":rows(&s.placements)?,"appearance_bindings":rows(&s.appearance_bindings)?,
 "assets":rows(&s.assets)?,"sources":rows(&s.sources)?,"source_identity_bindings":rows(&s.source_identity_bindings)?,"editor":rows(&s.editor)?}),
    )
}
fn tree(documents: &BTreeMap<String, Vec<u8>>) -> String {
    let mut h = Sha256::new();
    for (name, bytes) in documents {
        h.update((name.len() as u64).to_be_bytes());
        h.update(name.as_bytes());
        h.update((bytes.len() as u64).to_be_bytes());
        h.update(bytes);
    }
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}
fn main() -> Result<()> {
    let a: Vec<_> = env::args_os().skip(1).collect();
    if a.len() != 6 || a[0] != "--input-root" || a[2] != "--output-root" || a[4] != "--report" {
        return Err(
            "expected --input-root CURRENT_WORLD --output-root FRESH_WORLD --report FRESH_JSON"
                .into(),
        );
    }
    let input = PathBuf::from(&a[1]);
    let output = PathBuf::from(&a[3]);
    let report = PathBuf::from(&a[5]);
    if fs::symlink_metadata(&output).is_ok() || fs::symlink_metadata(&report).is_ok() {
        return Err("output/report already exists".into());
    }
    let parent = output.parent().ok_or("output parent")?;
    let m = fs::symlink_metadata(parent)?;
    if !m.is_dir() || m.file_type().is_symlink() {
        return Err("output parent must be a real existing directory".into());
    }
    let limits = npc_catalogue_preproduction_limits();
    let project = capture_world_project(
        input.parent().ok_or("input parent")?,
        input.file_name().ok_or("input name")?,
        limits,
    )?;
    let state = project
        .v2()
        .ok_or("current source must be WorldProject/v2")?;
    let source_count = state.sources.len();
    let candidate_count: usize = project.imports().iter().map(|b| b.candidates.len()).sum();
    let imports:Vec<_>=project.imports().iter().map(|b|json!({"batch_id":b.batch_id,"candidates":b.candidates.len(),"reimport_states":b.reimport_states.len()})).collect();
    let reimport_count: usize = project
        .imports()
        .iter()
        .map(|b| b.reimport_states.len())
        .sum();
    let before = semantics(&project)?;
    let mut exact = limits.project;
    exact.max_import_records = source_count.max(candidate_count).max(1);
    exact.max_reimport_states = reimport_count.max(1);
    let canonical = project.canonical_documents(exact)?;
    drop(project);
    let snapshot = canonical.into_snapshot(exact)?;
    let parsed = snapshot.parse(exact)?;
    if semantics(&parsed)? != before {
        return Err(
            "canonicalization changed typed identities, provenance or semantic rows".into(),
        );
    }
    let again = parsed.canonical_documents(exact)?;
    if again.documents() != snapshot.documents() {
        return Err("canonicalization is not byte-idempotent".into());
    }
    let digest = tree(snapshot.documents());
    let catalogue = NpcDataCatalogue::from_project(parsed, exact)?;
    let model = catalogue.service_model_for_project()?;
    if catalogue.source_tree_digest() != digest {
        return Err("NPC canonical digest differs from canonical package".into());
    }
    let files: Vec<_> = snapshot
        .documents()
        .iter()
        .map(|(name, b)| json!({"path":name,"bytes":b.len(),"sha256":sha(b)}))
        .collect();
    let census = json!({"schema":"OTERYN_CURRENT_WORLD_CENSUS/v1","qualification":"canonical source package only; no gameplay readiness","semantic_preserved":true,"byte_idempotent":true,"tree_sha256":digest,"documents":files,
 "source_count":source_count,"import_candidate_count":candidate_count,"reimport_count":reimport_count,"imports":imports,
 "npc_count":catalogue.npc_count(),"dialogue_count":catalogue.dialogue_count(),"service_count":catalogue.service_count(),
 "npc_service_model":{"npcs":model.npcs().len(),"held_npcs":model.held_npcs().len(),"services":model.services().len(),"offers":model.admitted_offer_count(),"routes":model.admitted_route_count(),"generated_replies":model.generated_reply_npc_count(),"held_offers":model.held_offer_counts()}});
    fs::create_dir(&output)?;
    for (name, bytes) in snapshot.documents() {
        let p = output.join(name);
        fs::create_dir_all(p.parent().ok_or("document parent")?)?;
        let mut f = OpenOptions::new().write(true).create_new(true).open(p)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&report)?;
    serde_json::to_writer_pretty(&mut f, &census)?;
    f.write_all(b"\n")?;
    f.sync_all()?;
    println!(
        "canonical_current_world semantic_preserved=true byte_idempotent=true documents={} sources={} tree_sha256={}",
        snapshot.documents().len(),
        source_count,
        digest
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn current_source_canonical_codec_preserves_semantics_and_is_byte_idempotent() -> Result<()> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../content/world");
        let limits = npc_catalogue_preproduction_limits();
        let project = capture_world_project(
            root.parent().ok_or("parent")?,
            root.file_name().ok_or("name")?,
            limits,
        )?;
        let before = semantics(&project)?;
        let original_source_pin = rows(&project.v2().ok_or("v2")?.sources)?;
        let mut altered = project.v2().ok_or("v2")?.sources.clone();
        let source = altered.first_mut().ok_or("source proof")?;
        source.sha256 = if source.sha256 == "a".repeat(64) {
            "b".repeat(64)
        } else {
            "a".repeat(64)
        };
        assert_ne!(
            rows(&altered)?,
            original_source_pin,
            "changing one exact provenance digest cannot pass the semantic conservation proof"
        );
        let canonical = project.canonical_documents(limits.project)?;
        let parsed = canonical
            .clone()
            .into_snapshot(limits.project)?
            .parse(limits.project)?;
        assert_eq!(
            semantics(&parsed)?,
            before,
            "all typed IDs, provenance and row values are preserved"
        );
        assert_eq!(
            parsed.canonical_documents(limits.project)?.documents(),
            canonical.documents(),
            "second strict pass has identical document bytes and controls"
        );
        Ok(())
    }
}
