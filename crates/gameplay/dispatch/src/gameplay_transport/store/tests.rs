#![allow(clippy::expect_used)]

use super::*;

#[test]
fn hireling_store_catalogue_has_every_pinned_crystal_offer_and_keeps_item_revision() {
    let catalogue=HIRELING_CATALOGUE.as_ref().expect("hireling catalogue");
    assert_eq!(catalogue.rules.revision(),"store-hireling-crystal-201189f-v1");
    // Crystal201189f gamestore.lua:6303-6472, generated together from the donor.
    for (id,price) in [(25440,150),(25438,250),(25437,120),(1001,250),(1002,900),(1003,250),(1004,250),(2001,500),(2002,500),(2003,500),(2004,500),(2005,300),(2006,900),(2007,900),(2008,900),(2009,900)] {
        let key=format!("store:hireling:{id}");
        assert_eq!(catalogue.rules.offer(&key).expect("offer").price_coins,price);
        assert_eq!(catalogue.hireling(&key).expect("metadata").source_id,id);
    }
    assert_eq!(catalogue.rules.offers_in_category("houses.hirelings",None,32).expect("page").len(),7);
    assert_eq!(catalogue.rules.offers_in_category("houses.hireling-dresses",None,32).expect("page").len(),9);
    assert_eq!(catalogue.item_revision("store:hireling:25440"),Some("definition-r1"));
    assert_eq!(CATALOGUE.as_ref().expect("existing catalogue").rules.revision(),"store-catalogue-r1");
}

#[test]
fn store_catalogue_admits_the_exercise_weapon_offers() {
    let catalogue = CATALOGUE.as_ref().expect("embedded catalogue");
    assert_eq!(catalogue.rules.revision(), "store-catalogue-r1");
    let offer = catalogue.rules.offer("store:exercise-sword").expect("offer");
    // Canary 52550d5e consumables_exercise_weapons.lua:118-121: 25 coins, item 28552.
    assert_eq!(offer.price_coins, 25);
    assert_eq!(
        offer.delivery,
        rules::DeliveryKind::Item {
            item_key: "oteryn:item.tibia.i28552".into(),
            count: 1
        }
    );
    assert_eq!(catalogue.title("store:exercise-sword"), "Exercise Sword");
    assert_eq!(catalogue.item_revision("store:exercise-sword"), Some("definition-r1"));
    let page = catalogue
        .rules
        .offers_in_category("consumables.exercise-weapons", None, 100)
        .expect("page");
    assert_eq!(page.len(), 23);
}

#[test]
fn store_catalogue_refuses_open_or_unpriced_documents() {
    let base = r#"{"schema":"OTERYN_STORE_RUNTIME_CATALOGUE/v1","catalog_revision":"r","source":{},
        "offers":[{"key":"k","category":"c","title":"t","price":PRICE,"currency":"CUR",
        "item":{"key":"i","revision":"r","charges":1},"source_line":1}]}"#;
    let document = |price: &str, currency: &str| base.replace("PRICE", price).replace("CUR", currency);
    assert!(RuntimeCatalogue::from_json(document("5", "TIBIA_COIN").as_bytes()).is_some());
    assert!(RuntimeCatalogue::from_json(document("0", "TIBIA_COIN").as_bytes()).is_none());
    assert!(RuntimeCatalogue::from_json(document("5", "GOLD").as_bytes()).is_none());
    let extra = document("5", "TIBIA_COIN").replace("\"source\":{}", "\"source\":{},\"x\":1");
    assert!(RuntimeCatalogue::from_json(extra.as_bytes()).is_none());
}

#[test]
fn store_uuid_text_round_trips_and_item_ids_are_v7() {
    let id = [
        0x01, 0x8f, 0x3a, 0x00, 0x00, 0x00, 0x70, 0x00, 0x80, 0, 0, 0, 0, 0, 0, 7,
    ];
    let text = uuid_text(&id);
    assert_eq!(text, "018f3a00-0000-7000-8000-000000000007");
    assert_eq!(uuid_bytes(&text), Some(id));
    assert_eq!(uuid_bytes("018f3a00000070008000000000000007"), None);
    let item = item_instance_id(&id);
    assert_eq!((item[6] >> 4, item[8] >> 6), (7, 0b10));
    assert_eq!(item, item_instance_id(&id));
    assert_ne!(item, item_instance_id(&[1; 16]));
}

#[test]
fn store_retained_answer_keeps_every_platform_result() {
    for (result, code) in [
        (PurchaseResult::StaleQuote, wire::StorePurchaseCode::StaleQuote),
        (PurchaseResult::OfferUnavailable, wire::StorePurchaseCode::OfferUnavailable),
        (PurchaseResult::PurchaseDisabled, wire::StorePurchaseCode::PurchaseDisabled),
        (
            PurchaseResult::InsufficientBalance,
            wire::StorePurchaseCode::InsufficientBalance,
        ),
        (PurchaseResult::AccountRestricted, wire::StorePurchaseCode::AccountRestricted),
        (
            PurchaseResult::Accepted {
                order_id: "o".into(),
                operation_ids: vec!["p".into()],
            },
            wire::StorePurchaseCode::Accepted,
        ),
    ] {
        let bytes = serde_json::to_vec(&RetainedAnswer::of(&result)).expect("encode");
        let answer: RetainedAnswer = serde_json::from_slice(&bytes).expect("decode");
        assert_eq!(answer.code(), Some(code));
    }
}
