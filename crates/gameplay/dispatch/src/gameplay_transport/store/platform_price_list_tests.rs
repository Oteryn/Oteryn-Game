//! S-5 shared offer constraints, not a price-list producer or a full schema validator.
use super::{CATALOGUE, rules};
use serde::Deserialize;
use std::collections::BTreeSet;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const S5_CURRENCIES: [(rules::CurrencyKind, Option<&str>); 2] = [
    (rules::CurrencyKind::TibiaCoin, Some("COINS")),
    (rules::CurrencyKind::TransferableTibiaCoin, None),
];
const VALID: &str = include_str!(
    "../../../../../../docs/contracts/fixtures/store-price-list-v1/price-list.valid.json"
);

fn s5_currency(currency: rules::CurrencyKind) -> Option<&'static str> {
    S5_CURRENCIES
        .iter()
        .find(|(kind, _)| *kind == currency)
        .and_then(|(_, name)| *name)
}

fn valid_offer_key(key: &str) -> bool {
    let mut bytes = key.bytes();
    (1..=128).contains(&key.len())
        && bytes
            .next()
            .is_some_and(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        && bytes.all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'.' | b':' | b'-')
        })
}

// Both the actual admitted catalogue and the Platform vectors use this checker.
// An unmapped Game currency stays None and refuses the whole projection.
fn conforming_offers<'a>(
    offers: impl IntoIterator<Item = (&'a str, Option<&'a str>, Option<u64>)>,
) -> bool {
    let mut keys = BTreeSet::new();
    let mut count = 0;
    offers.into_iter().all(|(key, currency, price)| {
        count += 1;
        count <= 10_000
            && valid_offer_key(key)
            && currency == Some("COINS")
            && price.is_some_and(|price| (1..=i64::MAX as u64).contains(&price))
            && keys.insert(key)
    }) && count > 0
}

#[derive(Deserialize)]
struct PlatformOffer {
    offer_key: String,
    currency: String,
    unit_price: String,
}
#[derive(Deserialize)]
struct PlatformPriceList {
    offers: Vec<PlatformOffer>,
}
fn fixture_conforms(document: &str) -> Result<bool, serde_json::Error> {
    let list: PlatformPriceList = serde_json::from_str(document)?;
    Ok(conforming_offers(list.offers.iter().map(|offer| {
        (
            offer.offer_key.as_str(),
            Some(offer.currency.as_str()),
            offer.unit_price.parse::<u64>().ok(),
        )
    })))
}

#[test]
fn store_catalogue_offers_conform_to_platform_s5_price_list() -> TestResult {
    // This is the LazyLock and RuntimeCatalogue::from_json used by admitted Store calls.
    let catalogue = CATALOGUE
        .as_ref()
        .ok_or("dispatch runtime catalogue was refused")?;
    assert!(conforming_offers(catalogue.entries().map(|offer| (
        offer.key.as_str(),
        s5_currency(offer.currency),
        Some(offer.price_coins),
    ))));
    Ok(())
}

#[test]
fn platform_s5_schema_pins_offer_key_and_currency() -> TestResult {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../docs/contracts/fixtures/store-price-list-v1/price-list.schema.json"
    ))?;
    let properties = &schema["properties"]["offers"]["items"]["properties"];
    assert_eq!(
        properties["offer_key"]["pattern"],
        "^[a-z0-9][a-z0-9_.:-]{0,127}$"
    );
    assert_eq!(properties["currency"]["enum"], serde_json::json!(["COINS"]));
    assert_eq!(
        s5_currency(rules::CurrencyKind::TibiaCoin),
        properties["currency"]["enum"][0].as_str()
    );
    assert_eq!(
        s5_currency(rules::CurrencyKind::TransferableTibiaCoin),
        None
    );
    Ok(())
}

#[test]
fn platform_price_list_checker_rejects_invalid_offers() -> TestResult {
    assert!(fixture_conforms(VALID)?);
    for document in [
        include_str!(
            "../../../../../../docs/contracts/fixtures/store-price-list-v1/price-list.invalid-transferable-currency.json"
        ),
        include_str!(
            "../../../../../../docs/contracts/fixtures/store-price-list-v1/price-list.invalid-offer-key-too-long.json"
        ),
        include_str!(
            "../../../../../../docs/contracts/fixtures/store-price-list-v1/price-list.invalid-amount-overflow.json"
        ),
        include_str!(
            "../../../../../../docs/contracts/fixtures/store-price-list-v1/price-list.invalid-duplicate-offer.json"
        ),
    ] {
        assert!(!fixture_conforms(document)?);
    }
    let list: PlatformPriceList = serde_json::from_str(VALID)?;
    let offer = list.offers.first().ok_or("fixture offer")?;
    let price = offer.unit_price.parse::<u64>()?;
    for key in ["Store:X", "/a", "a b", "", &"a".repeat(129)] {
        assert!(
            !conforming_offers([(key, Some(offer.currency.as_str()), Some(price))]),
            "{key}"
        );
    }
    assert!(conforming_offers([(
        "store:exercise-sword",
        Some(offer.currency.as_str()),
        Some(price)
    )]));
    // Boundary checks use the same fixture's price and currency, not a separate oracle.
    let longest_key = "a".repeat(128);
    assert!(conforming_offers([(
        longest_key.as_str(),
        Some(offer.currency.as_str()),
        Some(i64::MAX as u64)
    )]));
    for price in [0, i64::MAX as u64 + 1] {
        assert!(!conforming_offers([(
            offer.offer_key.as_str(),
            Some(offer.currency.as_str()),
            Some(price)
        )]));
    }
    let projected = (
        offer.offer_key.as_str(),
        Some(offer.currency.as_str()),
        Some(price),
    );
    assert!(!conforming_offers([projected, projected]));
    assert!(!conforming_offers([(
        offer.offer_key.as_str(),
        s5_currency(rules::CurrencyKind::TransferableTibiaCoin),
        Some(price)
    )]));
    let unsupported_key = format!("{}:transferable", offer.offer_key);
    for currency in [None, Some("TRANSFERABLE_COINS")] {
        assert!(!conforming_offers([
            projected,
            (unsupported_key.as_str(), currency, Some(price))
        ]));
    }
    let keys: Vec<_> = (0..10_001)
        .map(|i| format!("{}:{i}", offer.offer_key))
        .collect();
    assert!(conforming_offers(keys[..10_000].iter().map(|key| (
        key.as_str(),
        Some(offer.currency.as_str()),
        Some(price)
    ))));
    assert!(!conforming_offers(keys.iter().map(|key| (
        key.as_str(),
        Some(offer.currency.as_str()),
        Some(price)
    ))));
    Ok(())
}
