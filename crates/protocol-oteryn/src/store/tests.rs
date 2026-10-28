#![allow(clippy::unwrap_used)]
use super::*;
fn uuid(n: u8) -> [u8; 16] {
    let mut v = [n; 16];
    v[6] = (v[6] & 15) | 0x70;
    v[8] = (v[8] & 63) | 0x80;
    v
}
#[test]
fn store_codecs_round_trip() {
    let q = StoreCatalogueRequest {
        category: "featured".into(),
        page_cursor: "page-2".into(),
    };
    assert_eq!(
        decode_catalogue_request(&encode_catalogue_request(&q).unwrap()).unwrap(),
        q
    );
    let page = StoreCataloguePage {
        catalog_revision: "rev-7".into(),
        offers: vec![StoreOffer {
            offer_key: "mount:lion".into(),
            title: "Lion".into(),
            price: 250,
            currency: "tibia_coin".into(),
        }],
        next_page_cursor: "next".into(),
        total_coins: 500,
        transferable_coins: 300,
    };
    assert_eq!(
        decode_catalogue_page(&encode_catalogue_page(&page).unwrap()).unwrap(),
        page
    );
    let h = StoreHistoryPage {
        entries: vec![StoreHistoryEntry {
            order_id: uuid(1),
            offer_key: "mount:lion".into(),
            catalog_revision: "rev-7".into(),
            price: 250,
            currency: "tibia_coin".into(),
            purchased_at_unix_ms: 12,
        }],
        next_page_cursor: String::new(),
    };
    assert_eq!(
        decode_history_page(&encode_history_page(&h).unwrap()).unwrap(),
        h
    );
    let p = StorePurchaseRequest {
        purchase_request_id: uuid(2),
        offer_key: "mount:lion".into(),
        catalog_revision: "rev-7".into(),
        price: 250,
        currency: "tibia_coin".into(),
    };
    assert_eq!(
        decode_purchase_request(&encode_purchase_request(&p).unwrap()).unwrap(),
        p
    );
    let r = StorePurchaseResult {
        code: StorePurchaseCode::Accepted,
        order_id: Some(uuid(3)),
        operation_id: Some(uuid(4)),
    };
    assert_eq!(
        decode_purchase_result(&encode_purchase_result(&r).unwrap()).unwrap(),
        r
    );
    let c = StoreClaimReceipt {
        operation_id: uuid(5),
        fulfilment_unit_id: "unit:1".into(),
        result: StoreClaimCode::Committed,
        recovery_generation: 9,
        decided_at_unix_ms: 42,
    };
    assert_eq!(
        decode_claim_receipt(&encode_claim_receipt(&c).unwrap()).unwrap(),
        c
    )
}
#[test]
fn store_decoders_reject_bounds_before_allocating() {
    assert_eq!(
        decode_catalogue_request(&vec![0; MAX_STORE_REQUEST_BYTES + 1]),
        Err(StoreWireError::LimitExceeded)
    );
    assert_eq!(
        decode_catalogue_page(&vec![0; MAX_STORE_RESULT_BYTES + 1]),
        Err(StoreWireError::LimitExceeded)
    );
    let mut repeated = Vec::new();
    for _ in 0..=MAX_STORE_PAGE_ENTRIES {
        repeated.extend([10, 0])
    }
    assert_eq!(
        decode_history_page(&repeated),
        Err(StoreWireError::LimitExceeded)
    );
    let too_long = StoreCatalogueRequest {
        category: "x".repeat(MAX_STORE_KEY_BYTES + 1),
        page_cursor: String::new(),
    };
    assert!(encode_catalogue_request(&too_long).is_err());
    let mut invalid = uuid(1);
    invalid[6] = 0x40;
    let p = StorePurchaseRequest {
        purchase_request_id: invalid,
        offer_key: "a".into(),
        catalog_revision: "b".into(),
        price: 1,
        currency: "c".into(),
    };
    assert!(encode_purchase_request(&p).is_err())
}
#[test]
fn purchase_results_are_closed_and_accepted_ids_are_required() {
    for code in [
        StorePurchaseCode::StaleQuote,
        StorePurchaseCode::OfferUnavailable,
        StorePurchaseCode::PurchaseDisabled,
        StorePurchaseCode::InsufficientBalance,
        StorePurchaseCode::AccountRestricted,
    ] {
        let r = StorePurchaseResult {
            code,
            order_id: None,
            operation_id: None,
        };
        assert_eq!(
            decode_purchase_result(&encode_purchase_result(&r).unwrap()).unwrap(),
            r
        )
    }
    assert!(decode_purchase_result(&[8, 7]).is_err());
    assert!(
        encode_purchase_result(&StorePurchaseResult {
            code: StorePurchaseCode::Accepted,
            order_id: None,
            operation_id: None
        })
        .is_err()
    )
}
