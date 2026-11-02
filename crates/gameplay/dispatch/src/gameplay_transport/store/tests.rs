#![allow(clippy::expect_used)]

use super::*;

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

#[test]
fn prey_expansion_placeholder_is_hidden_until_it_has_an_approved_price() {
    let embedded = include_bytes!("../../../../../../rulesets/economy/store/catalogue.json");
    let mut document: serde_json::Value = serde_json::from_slice(embedded).expect("catalogue");
    let expansion = document["offers"].as_array_mut().expect("offers").iter_mut()
        .find(|offer| offer["key"] == "store:prey-list-expansion").expect("placeholder");
    assert_eq!(expansion["price"], serde_json::Value::Null);
    assert_eq!(expansion["enabled"], false);
    assert!(CATALOGUE.as_ref().expect("catalogue").rules.offer("store:prey-list-expansion").is_none());
    expansion["enabled"] = serde_json::json!(true);
    assert!(RuntimeCatalogue::from_json(&serde_json::to_vec(&document).expect("encode")).is_none());
}

#[test]
fn prey_expansion_store_offer_has_one_per_character_unlock_and_no_item_delivery() {
    let embedded = include_bytes!("../../../../../../rulesets/economy/store/catalogue.json");
    let mut document: serde_json::Value = serde_json::from_slice(embedded).expect("catalogue");
    let expansion = document["offers"].as_array_mut().expect("offers").iter_mut()
        .find(|offer| offer["key"] == "store:prey-list-expansion").expect("placeholder");
    // A fixture price exercises admission; the production offer remains owner-priced and disabled.
    expansion["enabled"] = serde_json::json!(true);
    expansion["price"] = serde_json::json!(17);
    let catalogue = RuntimeCatalogue::from_json(&serde_json::to_vec(&document).expect("encode")).expect("priced catalogue");
    let offer = catalogue.rules.offer("store:prey-list-expansion").expect("offer");
    assert_eq!(offer.delivery, rules::DeliveryKind::Unlock { unlock_key: "prey-list-expansion".into() });
    assert_eq!(offer.eligibility.per_character_limit, Some(1));
    assert_eq!(catalogue.item_revision(&offer.key), None);
    let mut character = rules::CharacterStoreView::default();
    assert_eq!(rules::build_quote(&catalogue.rules, &offer.key, &character).expect("new").price, 17);
    character.owned_unlocks.insert("prey-list-expansion".into());
    assert_eq!(rules::build_quote(&catalogue.rules, &offer.key, &character), Err(rules::QuoteRefusal::UnlockAlreadyOwned));
}
