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
    assert_eq!(model.npcs().len(), 1_282);
    assert_eq!(model.held_npcs().len(), 0);
    // NPC-ECON-1 Bank, PREM-5's services, BANK-3's Henricus blessing offer, and #2810's
    // Grizzly Adams rank shop. Collector #3030 (78294c081), s10-nonquest.json,
    // adds ten: one bank, five blessings, three Monk promotions and Kawill's
    // free Phoenix prerequisite. These services have no trade offers or routes.
    assert_eq!(model.services().len(), 404);
    // NPC-TRADE-3 in #3030 restores source fluid subtypes and full-charge offers
    // through npc_catalogue/subtype_trades.rs and service.rs.
    // QUESTPC8 Rashid (+150) and L2 Djinn (+122) restore 272 offers; their timed-count holds total nine.
    assert_eq!(model.admitted_offer_count(), 12_687);
    assert_eq!(model.admitted_route_count(), 195);
    assert_eq!(model.generated_reply_npc_count(), 446);
    let expected: BTreeMap<&str, usize> = BTreeMap::from([
        // NPC-TRADE-3 admits the proven fluid/full-charge tuples. Remaining
        // oversized ordinary offers still obey the physical coin capacity.
        ("ArbitragePaying", 0),
        ("ChargedOrFluidPending", 2),
        ("CountOutOfRange", 0),
        ("NonGoldCurrency", 31),
        ("ParityPending", 0),
        ("SellPriceAboveCoinCapacity", 17),
        ("TimedCountMismatch", 9),
        ("UnknownItem", 0),
    ]);
    assert_eq!(counts, expected);
}
