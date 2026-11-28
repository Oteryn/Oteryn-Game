use super::{Bindings, MAX_BINDING_BYTES, MAX_BOUND_QUESTS, embedded::AUTHORED_BINDING_DOCUMENTS};
use serde_json::Value;

#[test]
fn qbind_fragment_matches_boot_verifier() -> Result<(), String> {
    let report: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../tools/content-schema/quest-authoring/samples/server-completion/qbind-admission.json"
    )))
    .map_err(|e| e.to_string())?;
    let admitted = &report["admitted"][0];
    let source = admitted["registry_document"]
        .as_str()
        .ok_or("registered control document")?;
    let fragment = admitted["bindings"].to_string();
    let parsed = Bindings::parse_documents(&[&fragment])?;
    assert_eq!(parsed.quests.len(), 1);
    parsed.verify_embedded_recipe_evidence()?;

    let mut stale = parsed.clone();
    stale.quests[0].recipe_sha256 = "f".repeat(64);
    assert!(
        stale
            .verify_embedded_recipe_evidence()
            .is_err_and(|error| error.contains("digest differs"))
    );

    let mut documents: Vec<_> = AUTHORED_BINDING_DOCUMENTS
        .iter()
        .map(|(_, bytes)| *bytes)
        .collect();
    let base = Bindings::parse_documents(&documents)?;
    assert!(documents.len() < MAX_BOUND_QUESTS);
    assert!(base.quests.len() < MAX_BOUND_QUESTS);
    assert!(
        documents.iter().map(|doc| doc.len()).sum::<usize>() + fragment.len() <= MAX_BINDING_BYTES
    );
    let mut duplicate = documents.clone();
    duplicate.push(&fragment);
    assert!(Bindings::parse_documents(&duplicate).is_err());

    let control = AUTHORED_BINDING_DOCUMENTS
        .iter()
        .position(|(path, _)| *path == source)
        .ok_or("control must already be registered")?;
    documents[control] = &fragment;
    let combined = Bindings::parse_documents(&documents)?;
    combined.verify_embedded_recipe_evidence()?;
    assert_eq!(combined, base);
    Ok(())
}
