#![cfg(target_os = "linux")]
#![allow(clippy::expect_used, clippy::panic)]
use oteryn_game_server::content::*;
use std::{collections::BTreeMap, ffi::OsStr, path::PathBuf};

fn build() -> NpcDataCatalogue {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../content/world");
    let project = capture_world_project(
        root.parent().expect("parent"),
        OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .expect("validated repository source");
    NpcDataCatalogue::from_project(project, npc_catalogue_preproduction_limits().project)
        .expect("data import")
}

#[test]
fn pinned_catalogue_service_model_counts() {
    let catalogue = build();
    let model = catalogue
        .service_model_for_project()
        .expect("service model");
    let again = catalogue
        .service_model_for_project()
        .expect("service model");
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
