#![allow(clippy::expect_used)]

use super::*;

#[test]
fn store_purchase_history_cursor_round_trips_and_rejects_malformed() {
    let id = [
        0x01, 0x8f, 0x2a, 0x10, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 0x09,
    ];
    let text = format_history_cursor(1_790_000_000_123, &id);
    assert_eq!(text, "1790000000123.018f2a10000070008000000000000009");
    assert_eq!(
        parse_history_cursor(&text),
        Some((1_790_000_000_123, id))
    );
    for bad in [
        "",
        "1.",
        ".018f2a10000070008000000000000009",
        "1.018f2a1000007000800000000000009",
        "1.018F2A10000070008000000000000009",
        "+1.018f2a10000070008000000000000009",
        "18446744073709551616.018f2a10000070008000000000000009",
        "1.018f2a10000070008000000000000009x",
    ] {
        assert_eq!(parse_history_cursor(bad), None, "{bad}");
    }
}

#[test]
fn store_purchase_history_capability_is_offered_only_with_store() {
    const BASE: &[super::super::capabilities::OfferedCapability] = &[];
    assert!(
        !with_store_offer(BASE, false)
            .iter()
            .any(|capability| capability.id == wire::CAPABILITY_STORE_PURCHASE_HISTORY_V1)
    );
    let row = with_store_offer(BASE, true)
        .iter()
        .find(|capability| capability.id == wire::CAPABILITY_STORE_PURCHASE_HISTORY_V1)
        .expect("285 offered");
    assert_eq!(row.requires, &[wire::CAPABILITY_STORE_V1]);
}

#[test]
fn store_catalogue_revision_is_canonical_sha256_and_changes_with_an_offer() {
    let bytes = include_bytes!("../../../../../../rulesets/economy/store/catalogue.json");
    let mut document: serde_json::Value = serde_json::from_slice(bytes).expect("catalogue JSON");
    document.sort_all_objects();
    let canonical = serde_json::to_vec(&document).expect("canonical catalogue");
    let catalogue = RuntimeCatalogue::from_json(bytes).expect("catalogue");
    let revision = catalogue.rules.revision();
    assert_eq!(revision.len(), 64);
    assert!(revision.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));
    let expected: String = Sha256::digest(&canonical).iter().map(|byte| format!("{byte:02x}")).collect();
    assert_eq!(revision, expected);
    assert_eq!(RuntimeCatalogue::from_json(&canonical).expect("compact catalogue").rules.revision(), revision);
    document["offers"][0]["price"] = serde_json::json!(26);
    let changed = RuntimeCatalogue::from_json(&serde_json::to_vec(&document).expect("changed JSON")).expect("changed catalogue");
    assert_ne!(changed.rules.revision(), revision);
}

#[test]
fn hireling_store_catalogue_has_every_pinned_crystal_offer_and_keeps_item_revision() {
    let catalogue=HIRELING_CATALOGUE.as_ref().expect("hireling catalogue");
    assert_eq!(catalogue.rules.revision(),"e7e06a2b1ce23df40cc05be37882eb4b2d4139a5bbc112abd893e8eb526d890d");
    // Crystal201189f gamestore.lua:6303-6472, generated together from the donor.
    for (id,price) in [(25440,150),(25438,250),(25437,120),(1001,250),(1002,900),(1003,250),(1004,250),(2001,500),(2002,500),(2003,500),(2004,500),(2005,300),(2006,900),(2007,900),(2008,900),(2009,900)] {
        let key=format!("store:hireling:{id}");
        assert_eq!(catalogue.rules.offer(&key).expect("offer").price_coins,price);
        assert_eq!(catalogue.hireling(&key).expect("metadata").source_id,id);
    }
    assert_eq!(catalogue.rules.offers_in_category("houses.hirelings",None,32).expect("page").len(),7);
    assert_eq!(catalogue.rules.offers_in_category("houses.hireling-dresses",None,32).expect("page").len(),9);
    assert_eq!(catalogue.item_revision("store:hireling:25440"),Some("definition-r1"));
    assert_eq!(CATALOGUE.as_ref().expect("existing catalogue").rules.revision(),"451c944aa7b6ab9d27cfce868fcf4949c8996a7b6857cc2641e2c296fd415c8c");
}

#[test]
fn store_catalogue_admits_the_exercise_weapon_offers() {
    let catalogue = CATALOGUE.as_ref().expect("embedded catalogue");
    assert_eq!(catalogue.rules.revision(), "451c944aa7b6ab9d27cfce868fcf4949c8996a7b6857cc2641e2c296fd415c8c");
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
fn charm_expansion_store_offer_matches_crystal_and_is_one_per_character() {
    let catalogue = CATALOGUE.as_ref().expect("embedded catalogue");
    let offer = catalogue.rules.offer("store:charm-expansion").expect("offer");
    // Crystal data/modules/scripts/gamestore/gamestore.lua:6554-6558.
    assert_eq!((offer.price_coins, catalogue.title(&offer.key)), (450, "Charm Expansion"));
    assert_eq!(offer.delivery, rules::DeliveryKind::Unlock { unlock_key: "charm-expansion".into() });
    assert_eq!(offer.eligibility.per_character_limit, Some(1));
    let mut character = rules::CharacterStoreView::default();
    character.owned_unlocks.insert("charm-expansion".into());
    assert_eq!(rules::build_quote(&catalogue.rules, &offer.key, &character), Err(rules::QuoteRefusal::UnlockAlreadyOwned));
}

#[test]
fn store_catalogue_refuses_open_or_unpriced_documents() {
    let base = r#"{"schema":"OTERYN_STORE_RUNTIME_CATALOGUE/v1","source":{},
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

#[test]
fn store_platform_currency_refuses_the_removed_transferable_coin_kind() {
    assert_eq!(platform_currency(rules::CurrencyKind::TibiaCoin), Ok("COINS"));
    assert_eq!(
        platform_currency(rules::CurrencyKind::TransferableTibiaCoin),
        Err(wire::StorePurchaseCode::OfferUnavailable)
    );
}

#[test]
fn store_restored_backup_unit_refuses_without_platform_calls() {
    use crate::native_admission_source::{descriptor::Operation, store::{StoreTransport, TransportFailure}};
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct Platform(Arc<AtomicUsize>);
    impl StoreTransport for Platform {
        fn exchange<'a>(&'a mut self, _: Operation, _: &'a str) -> std::pin::Pin<Box<dyn std::future::Future<Output=Result<Vec<u8>, TransportFailure>> + Send + 'a>> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Box::pin(async { Err(TransportFailure::Unavailable) })
        }
    }
    let calls = Arc::new(AtomicUsize::new(0));
    let mut client = StoreClient::new(Box::new(Platform(calls.clone())), "oteryn-game/store-consumer".into());
    let runtime = tokio::runtime::Builder::new_current_thread().build().expect("runtime");
    let result = runtime.block_on(purchase_with_mode(
        StorePurchaseMode::DisabledRestoredBackup, async { Some(false) }, async {
            let _ = client.purchase(&PurchaseStoreOffer {
                purchase_request_id: "018f3a00-0000-7000-8000-000000000007".into(),
                account_id: "018f3a00-0000-7000-8000-000000000008".into(),
                character_id: "018f3a00-0000-7000-8000-000000000009".into(),
                offer_key: "store:exercise-sword".into(), catalog_revision: CATALOGUE.as_ref().expect("catalogue").rules.revision().into(),
                unit_price: "25".into(), world_id: "018f3a00-0000-7000-8000-000000000010".into(), quantity: 1, currency: "COINS".into(),
            }, "ITEM").await;
            None
        },
    ));
    assert_eq!(result.expect("disabled answer").code, wire::StorePurchaseCode::PurchaseDisabled);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn store_capability_requires_a_composed_platform_client() {
    static BASE: &[super::super::capabilities::OfferedCapability] = &[];
    assert!(!with_store_offer(BASE, false).iter().any(|row| row.id == wire::CAPABILITY_STORE_V1));
    assert!(with_store_offer(BASE, true).iter().any(|row| row.id == wire::CAPABILITY_STORE_V1));
}
