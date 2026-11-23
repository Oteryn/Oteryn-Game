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
    // Grizzly Adams rank shop, S10 services, exact NPC-TRADE-3 subtype restoration,
    // and the four source-owned Djinn shops.
    assert_eq!(model.services().len(), 407);
    assert_eq!(model.admitted_offer_count(), 12_703);
    assert_eq!(model.admitted_route_count(), 199);
    assert_eq!(model.generated_reply_npc_count(), 446);
    let expected: BTreeMap<&str, usize> = BTreeMap::from([
        // Exact subtype qualification preserves the existing arbitrage protection.
        ("ArbitragePaying", 0),
        // The Nah'Bob i3284 offer lacks a native subtype; i3083 lacks full-charge proof.
        ("ChargedOrFluidPending", 1),
        ("CountOutOfRange", 0),
        ("NonGoldCurrency", 31),
        ("ParityPending", 0),
        ("SellPriceAboveCoinCapacity", 17),
        ("TimedCountMismatch", 9),
        ("UnknownItem", 0),
    ]);
    assert_eq!(counts, expected);
}

#[test]
fn gamon_offers_resolve_to_distinct_inventory_kits() -> Result<(), Box<dyn std::error::Error>> {
    use native_gameplay::test_support::activated_with_loot_tables;
    use oteryn_gameplay_domain::domain::item_definition::{ItemStackClass, TypedDefinitionRef};
    use oteryn_gameplay_items::item_ref_admission::generation_item_facts;
    let world = oteryn_protocol_oteryn::WorldId::decode(&[
        0x10, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 42,
    ])?;
    let (controller, _) = activated_with_loot_tables(world, None);
    let native = controller
        .active()
        .and_then(ActiveGeneration::native_gameplay)
        .ok_or("native item profiles")?;
    let model = build().service_model_for_project()?;
    let book = oteryn_gameplay_npc::npc_talk::NpcTalkBook::from_model(&model);
    let gamon = book.get("oteryn:npc.gamon").ok_or("Gamon's shop")?;
    let mut kits = Vec::new();
    let index = native.item_index().ok_or("native item index")?;
    for (_, offer) in
        oteryn_gameplay_npc::npc_command::projected(gamon.offers(), index).filter(|(_, offer)| {
            offer.direction == ProjectV2ServiceOfferDirection::SellToPlayer
                && matches!(offer.unit_price, 30 | 50)
                && matches!(
                    offer.item.key.as_str(),
                    "oteryn:item.tibia.i2797"
                        | "oteryn:item.tibia.i2800"
                        | "oteryn:item.tibia.i2998"
                )
        })
    {
        let definition = TypedDefinitionRef {
            family: "Item".into(),
            production_key: offer.item.key.clone(),
            revision_ref: offer.item.revision.clone(),
        };
        let facts = generation_item_facts(native, &definition)?;
        kits.push((offer.unit_price, offer.item.key.as_str(), facts.stack));
    }
    kits.sort_by_key(|kit| kit.0);
    assert_eq!(
        kits,
        [
            (30, "oteryn:item.tibia.i2800", ItemStackClass::NonStackable),
            (50, "oteryn:item.tibia.i2797", ItemStackClass::NonStackable),
        ]
    );
    Ok(())
}
