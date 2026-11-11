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

// Regenerate strict count pins: python tools/contracts/refresh_npc_service_counts.py
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
    // The repository refresh tool captures every count before any assertion stops the test.
    if std::env::var_os("OTERYN_CAPTURE_NPC_SERVICE_COUNTS").is_some() {
        eprintln!(
            "NPC_SERVICE_COUNTS {}",
            serde_json::json!({
                "npcs": model.npcs().len(), "held_npcs": model.held_npcs().len(),
                "services": model.services().len(), "offers": model.admitted_offer_count(),
                "routes": model.admitted_route_count(), "replies": model.generated_reply_npc_count(),
                "held": counts, "source_tree_digest": model.source_tree_digest(),
                "service_keys": model.services().keys().map(|r| r.key.as_str()).collect::<Vec<_>>(),
            })
        );
    }
    assert_eq!(model.npcs().len(), 1_282);
    assert_eq!(model.held_npcs().len(), 0);
    // NPC-ECON-1 Bank, PREM-5's services, BANK-3's Henricus blessing offer, and #2810's
    // Grizzly Adams rank shop, S10 services, and exact NPC-TRADE-3 subtype restoration.
    assert_eq!(model.services().len(), 400);
    assert_eq!(model.admitted_offer_count(), 12_565);
    assert_eq!(model.admitted_route_count(), 195);
    assert_eq!(model.generated_reply_npc_count(), 446);
    let expected: BTreeMap<&str, usize> = BTreeMap::from([
        // Exact subtype qualification preserves the existing arbitrage protection.
        ("ArbitragePaying", 0),
        ("ChargedOrFluidPending", 0),
        ("CountOutOfRange", 0),
        ("NonGoldCurrency", 31),
        ("ParityPending", 0),
        ("SellPriceAboveCoinCapacity", 17),
        ("TimedCountMismatch", 7),
        ("UnknownItem", 0),
    ]);
    assert_eq!(counts, expected);
}
