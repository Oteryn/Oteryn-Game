#![allow(clippy::expect_used)]
//! The same regenerated Service declarations and immutable model as node composition.
use super::*;
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::npc_talk::NpcTalkBook;

#[test]
fn promotion_and_premium_blessing_services_require_the_npcs_own_declarations() {
    let catalogue = NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .expect("canonical catalogue");
    let model = catalogue
        .service_model_for_project()
        .expect("canonical model");
    let book = NpcTalkBook::from_model(&model);
    for npc in ["king_tibianus", "queen_eloise", "emperor_kruzak", "eremo"] {
        let entry = book
            .get(&format!("oteryn:npc.{npc}"))
            .expect("production offer");
        assert!(select("promotion", &entry.premium_services).is_some());
        assert!(select("promotion 1 gold", &entry.premium_services).is_none());
    }
    for npc in ["eremo", "kais", "nomad"] {
        let entry = book
            .get(&format!("oteryn:npc.{npc}"))
            .expect("production blessing offer");
        let service = select("blessing", &entry.premium_services).expect("blessing");
        assert!(matches!(service, PremiumNpcService::Blessing(_)));
        assert!(prompt(service).contains("current level"));
    }
    assert!(
        book.get("oteryn:npc.paulie")
            .expect("bank")
            .premium_services
            .is_empty()
    );
}
