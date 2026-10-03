#![cfg(target_os = "linux")]
#![allow(clippy::expect_used, clippy::panic)]
use oteryn_game_server::content::*;
use std::{ffi::OsStr, path::PathBuf};
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../content/world")
}
fn captured() -> WorldProject {
    capture_world_project(
        root().parent().expect("parent"),
        OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .expect("validated repository source")
}
#[test]
fn repository_import_preserves_all_data_and_approximation_flags() {
    let project = captured();
    let baseline = project.v2().expect("v2");
    let npc = baseline.declarations.iter().find(|d| matches!(d,ProjectV2Declaration::Npc {identity,..} if identity.key=="oteryn:npc.mud")).expect("MUD").clone();
    let sources = baseline.sources.clone();
    let bindings = baseline.source_identity_bindings.clone();
    let catalogue =
        NpcDataCatalogue::from_project(project, npc_catalogue_preproduction_limits().project)
            .expect("data import");
    assert_eq!(
        catalogue.project_revision(),
        "g4-npc-provisional-enrichment-r28"
    );
    assert_eq!(
        catalogue.source_tree_digest(),
        "3fcab27b778849cb2453d34161fd16c9cbd58c0bff0d00c54fc12e5583c78148"
    );
    assert_eq!(catalogue.npc_count(), 1282);
    assert_eq!(catalogue.dialogue_count(), 836);
    assert_eq!(catalogue.service_count(), 380);
    assert_eq!(catalogue.sources(), sources);
    assert_eq!(catalogue.source_identity_bindings(), bindings);
    let (reference, imported) = catalogue
        .records()
        .find(|(r, _)| r.key == "oteryn:npc.mud")
        .expect("imported MUD");
    assert_eq!(imported, &npc);
    assert_eq!(catalogue.declaration(reference), Some(&npc));
    if let ProjectV2Declaration::Npc {
        presentation,
        behavior,
        fields,
        ..
    } = imported
    {
        assert!(
            presentation
                .as_ref()
                .is_some_and(|r| catalogue.profile(r).is_some())
        );
        assert!(
            behavior
                .as_ref()
                .is_some_and(|r| catalogue.profile(r).is_some())
        );
        assert!(fields.iter().any(|f|matches!(&f.value,ProjectV2CandidateValue::Text(v) if v.contains("APPROXIMATE_WIKI_VISUAL_INVISIBLE_MAPPING"))));
    }
}
#[test]
fn dangling_dialogue_is_rejected_by_the_existing_import_admission() {
    let mut draft = captured().migrate_to_v2();
    let actor = draft
        .state
        .declarations
        .iter_mut()
        .find(|d| {
            matches!(
                d,
                ProjectV2Declaration::Npc {
                    dialogue: Some(_),
                    ..
                }
            )
        })
        .expect("NPC with dialogue");
    if let ProjectV2Declaration::Npc {
        dialogue: Some(reference),
        ..
    } = actor
    {
        reference.key = "oteryn:dialogue.npc.missing_data_import_target".to_owned();
    }
    assert!(
        CanonicalProjectDocuments::from_v2_draft(
            draft,
            npc_catalogue_preproduction_limits().project
        )
        .is_err()
    );
}
#[test]
fn malformed_and_mismatched_source_pins_fail_closed() {
    assert!(load_data_only_npc_catalogue(&root(), &"0".repeat(64)).is_err());
    assert!(
        load_data_only_npc_catalogue(&PathBuf::from("/nonexistent/npc-project"), "not-a-sha256")
            .is_err()
    );
}

#[test]
fn legacy_project_without_v2_cannot_be_imported_as_npc_data() {
    let core = captured().migrate_to_v2().core;
    let limits = npc_catalogue_preproduction_limits().project;
    let legacy = CanonicalProjectDocuments::from_draft(core, limits)
        .expect("valid legacy project")
        .into_snapshot(limits)
        .expect("legacy snapshot")
        .parse(limits)
        .expect("validated legacy");
    assert!(legacy.v2().is_none());
    assert!(NpcDataCatalogue::from_project(legacy, limits).is_err());
}
