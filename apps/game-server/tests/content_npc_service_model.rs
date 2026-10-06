#![cfg(target_os = "linux")]
#![allow(clippy::expect_used, clippy::panic)]
use oteryn_game_server::content::*;
use std::{collections::BTreeMap, ffi::OsStr, path::PathBuf};

fn build() -> (NpcDataCatalogue, WorldProject) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../content/world");
    let project = capture_world_project(
        root.parent().expect("parent"),
        OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .expect("validated repository source");
    let catalogue = NpcDataCatalogue::from_project(
        project.clone(),
        npc_catalogue_preproduction_limits().project,
    )
    .expect("data import");
    (catalogue, project)
}

#[test]
fn pinned_catalogue_service_model_counts() {
    let (catalogue, project) = build();
    let source = project.lower_reference_source().expect("Reference source");
    let model = catalogue.service_model_for_reference_source(&source);
    let again = catalogue.service_model_for_reference_source(&source);
    assert_eq!(model, again, "built twice");
    assert_eq!(model.source_tree_digest(), catalogue.source_tree_digest());
    let counts: BTreeMap<&str, usize> = model.held_offer_counts();
    eprintln!(
        "MEASURED digest={} npcs={} held_npcs={} services={} offers={} routes={} generated={} held={counts:?}",
        model.source_tree_digest(),
        model.npcs().len(),
        model.held_npcs().len(),
        model.services().len(),
        model.admitted_offer_count(),
        model.admitted_route_count(),
        model.generated_reply_npc_count(),
    );
}
