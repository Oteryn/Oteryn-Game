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
    assert_eq!(model.npcs().len(), 1282);
    assert_eq!(model.held_npcs().len(), 0);
    assert_eq!(model.services().len(), 380);
    assert_eq!(model.admitted_offer_count(), 11_903);
    assert_eq!(model.admitted_route_count(), 195);
    assert_eq!(model.generated_reply_npc_count(), 446);
    let expected: BTreeMap<&str, usize> = BTreeMap::from([
        ("ArbitragePaying", 1),
        ("CountOutOfRange", 0),
        ("NonGoldCurrency", 31),
        ("ParityPending", 0),
        ("SellPriceAboveCoinCapacity", 299),
        ("TimedCountMismatch", 0),
        ("UnknownItem", 0),
    ]);
    assert_eq!(counts, expected);
}
