//! S-5 shared offer constraints, not a price-list producer or a full schema validator.
use super::{CATALOGUE, COSMETIC_CATALOGUE, HIRELING_CATALOGUE, rules};
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
struct PlatformUnit {
    unit_kind: String,
}
#[derive(Deserialize)]
struct PlatformOffer {
    units: Vec<PlatformUnit>,
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
fn active_platform_import_covers_every_dispatch_quote() -> TestResult {
    let bytes = include_bytes!("../../../../../../rulesets/economy/store/platform-catalogue.json");
    let active: PlatformPriceList = serde_json::from_slice(bytes)?;
    let revision = super::active_catalogue_revision();
    let mut quoted = BTreeSet::new();
    for catalogue in [
        &CATALOGUE,
        &COSMETIC_CATALOGUE,
        &HIRELING_CATALOGUE,
        &super::BLESSING_CATALOGUE,
    ] {
        let catalogue = catalogue.as_ref().ok_or("dispatch catalogue")?;
        assert_eq!(catalogue.rules.revision(), revision);
        for offer in catalogue.entries() {
            let imported = active
                .offers
                .iter()
                .find(|row| row.offer_key == offer.key)
                .ok_or_else(|| format!("Platform import is missing {}", offer.key))?;
            assert_eq!(imported.currency, "COINS");
            assert_eq!(imported.unit_price.parse::<u64>()?, offer.price_coins);
            let [unit] = imported.units.as_slice() else {
                return Err(format!("expected one imported unit for {}", offer.key).into());
            };
            assert_eq!(
                super::platform_unit_kind(revision, &offer.key),
                Some(unit.unit_kind.as_str())
            );
            assert!(quoted.insert(&offer.key));
        }
    }
    let game_offers = active.offers.iter().filter(|offer| {
        assert!(!offer.units.is_empty());
        !offer
            .units
            .iter()
            .all(|unit| unit.unit_kind == "PREMIUM_TIME")
    });
    assert_eq!(quoted.len(), game_offers.count());
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

#[test]
fn retained_runtime_cosmetics_use_the_platform_unit_kind() -> TestResult {
    use super::rules::DeliveryKind;

    let mut legacy_cosmetics = 0;
    let mut imported_cosmetics = 0;
    for catalogue in super::PREDECESSOR_CATALOGUES.iter().flatten() {
        let revision = catalogue.rules.revision();
        // Retained Runtime cosmetics CI finding: legacy confirmations were
        // sent as ITEM, while retained Platform imports bind COSMETIC.
        let kind = if super::PLATFORM_PREDECESSORS
            .iter()
            .any(|(old, _)| old == revision)
        {
            "COSMETIC"
        } else {
            "ITEM"
        };
        for offer in catalogue.entries() {
            if matches!(&offer.delivery, DeliveryKind::Unlock { unlock_key }
                if unlock_key.starts_with("oteryn:outfit.") || unlock_key.starts_with("oteryn:mount."))
            {
                if kind == "COSMETIC" {
                    imported_cosmetics += 1;
                } else {
                    legacy_cosmetics += 1;
                }
                assert_eq!(
                    super::platform_unit_kind(revision, &offer.key),
                    Some(kind),
                    "{}",
                    offer.key
                );
            }
        }
    }
    assert!(legacy_cosmetics > 0);
    assert!(imported_cosmetics > 0);
    Ok(())
}

#[test]
fn retained_platform_imports_reconstruct_the_original_purchase_bindings() -> TestResult {
    use super::{PLATFORM_PREDECESSORS_JSON, PREDECESSOR_CATALOGUES, PurchaseRequest};

    let active = CATALOGUE.as_ref().ok_or("active catalogue")?;
    let snapshots: Vec<String> = serde_json::from_slice(PLATFORM_PREDECESSORS_JSON)?;
    assert!(!snapshots.is_empty());
    for snapshot in snapshots {
        let revision = super::platform_revision(snapshot.as_bytes());
        let original: PlatformPriceList = serde_json::from_str(&snapshot)?;
        let archived = PREDECESSOR_CATALOGUES
            .iter()
            .flatten()
            .find(|old| old.rules.revision() == revision)
            .ok_or("archived catalogue")?;
        let (sword, quote) = original
            .offers
            .iter()
            .find_map(|offer| {
                let active_quote = rules::build_quote(
                    &active.rules,
                    &offer.offer_key,
                    &rules::CharacterStoreView::default(),
                )
                .ok()?;
                (offer.unit_price.parse::<u64>().ok()? == active_quote.price)
                    .then_some((offer, active_quote))
            })
            .ok_or("shared active offer")?;
        let account = crate::domain::AccountId::from_bytes(uuid(1))?;
        let character = crate::domain::CharacterId::from_bytes(uuid(2))?;
        let old_id = rules::PurchaseRequestId::from_bytes(uuid(3)).map_err(|_| "original ID")?;
        let confirmed = rules::PurchaseStoreOffer::from_quote(
            rules::PurchaseRequestId::from_bytes(uuid(4)).map_err(|_| "retry ID")?,
            account,
            character,
            quote,
        );
        let original = rules::PurchaseStoreOffer {
            purchase_request_id: old_id,
            account_id: account,
            character_id: character,
            offer_key: sword.offer_key.clone(),
            catalog_revision: revision.clone(),
            price: sword.unit_price.parse()?,
            currency: rules::CurrencyKind::TibiaCoin,
        };
        let binding_digest = PurchaseRequest {
            purchase_request_id: *old_id.as_bytes(),
            account_id: *account.as_bytes(),
            character_id: *character.as_bytes(),
            binding: original.binding_digest().into(),
        }
        .binding_digest();
        let pending = crate::durability::store::PendingStorePurchase {
            purchase_id: *old_id.as_bytes(),
            character_id: *character.as_bytes(),
            world_id: uuid(5),
            binding_digest: binding_digest.to_vec(),
        };
        assert!(archived.rules.offer(&sword.offer_key).is_some());
        let (_, retained) =
            super::retained_binding(active, &confirmed, &pending).ok_or("original binding")?;
        assert_eq!(retained.binding_digest(), original.binding_digest());
    }
    Ok(())
}

fn uuid(suffix: u8) -> [u8; 16] {
    [0, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, suffix]
}
