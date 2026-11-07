#![allow(clippy::unwrap_used)]

use super::*;

fn offer(key: &str) -> AdmittedOffer {
    AdmittedOffer {
        key: key.into(),
        category: "featured".into(),
        kind: OfferKind::Service,
        price_coins: 10,
        currency: CurrencyKind::TibiaCoin,
        delivery: DeliveryKind::Service,
        eligibility: Eligibility::default(),
    }
}

#[test]
fn admission_rejects_invalid_identity_and_duplicate_keys() {
    assert_eq!(
        Catalogue::admit("", []).unwrap_err(),
        CatalogueError::EmptyRevision
    );
    assert_eq!(
        Catalogue::admit("r1", [offer("")]).unwrap_err(),
        CatalogueError::EmptyOfferKey
    );
    assert_eq!(
        Catalogue::admit("r1", [offer("a"), offer("a")]).unwrap_err(),
        CatalogueError::DuplicateOfferKey
    );
}

#[test]
fn category_pages_are_bounded_and_stably_keyed() {
    let catalogue = Catalogue::admit("r1", [offer("b"), offer("a"), offer("c")]).unwrap();
    let page = catalogue
        .offers_in_category("featured", Some("a"), 2)
        .unwrap();
    assert_eq!(
        page.iter()
            .map(|offer| offer.key.as_str())
            .collect::<Vec<_>>(),
        ["b", "c"]
    );
    assert_eq!(
        catalogue
            .offers_in_category("featured", None, 0)
            .unwrap_err(),
        CatalogueError::ZeroPageSize
    );
    assert_eq!(
        catalogue
            .offers_in_category("featured", None, MAX_OFFERS_PER_PAGE + 1)
            .unwrap_err(),
        CatalogueError::PageTooLarge
    );
}
