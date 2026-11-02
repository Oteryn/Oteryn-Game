//! The admitted runtime Store catalogue (`rulesets/economy/store/catalogue.json`, built from the
//! pinned Canary GameStore catalogue by `tools/content-schema/store-authoring/build_runtime_catalogue.py`).
use oteryn_gameplay_economy::store as rules;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: String,
    catalog_revision: String,
    #[serde(rename = "source")]
    _source: serde_json::Value,
    offers: Vec<Offer>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Offer {
    key: String,
    category: String,
    title: String,
    price: Option<u64>,
    #[serde(default = "enabled_by_default")]
    enabled: bool,
    currency: String,
    item: Option<Item>,
    #[serde(default)]
    unlock_key: Option<String>,
    #[serde(rename = "source_line")]
    _source_line: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Item {
    key: String,
    revision: String,
    #[serde(rename = "charges")]
    _charges: u32,
}

pub(super) const PREY_EXPANSION_UNLOCK: &str = "prey-list-expansion";

fn enabled_by_default() -> bool { true }

pub(super) struct RuntimeCatalogue {
    pub(super) rules: rules::Catalogue,
    titles: BTreeMap<String, String>,
    item_revisions: BTreeMap<String, String>,
}

impl RuntimeCatalogue {
    /// `None` for anything but a closed, non-empty v1 document of positive Tibia Coin prices.
    pub(super) fn from_json(bytes: &[u8]) -> Option<Self> {
        let document: Document = serde_json::from_slice(bytes).ok()?;
        if document.schema != "OTERYN_STORE_RUNTIME_CATALOGUE/v1" || document.offers.is_empty() {
            return None;
        }
        let mut titles = BTreeMap::new();
        let mut item_revisions = BTreeMap::new();
        let mut offers = Vec::with_capacity(document.offers.len());
        for offer in document.offers {
            // Owner-priced offers remain unavailable while their price is a placeholder.
            if !offer.enabled {
                continue;
            }
            let price = offer.price.filter(|price| *price > 0)?;
            if offer.currency != currency_text(rules::CurrencyKind::TibiaCoin) {
                return None;
            }
            let (kind, delivery, eligibility) = match (offer.item, offer.unlock_key) {
                (Some(item), None) => {
                    item_revisions.insert(offer.key.clone(), item.revision);
                    (rules::OfferKind::Item, rules::DeliveryKind::Item { item_key: item.key, count: 1 }, rules::Eligibility::default())
                }
                (None, Some(unlock_key)) if unlock_key == PREY_EXPANSION_UNLOCK => {
                    let eligibility = rules::Eligibility {
                        reject_if_unlock_owned: Some(unlock_key.clone()),
                        per_character_limit: Some(1),
                        ..rules::Eligibility::default()
                    };
                    (rules::OfferKind::Service, rules::DeliveryKind::Unlock { unlock_key }, eligibility)
                }
                _ => return None,
            };
            titles.insert(offer.key.clone(), offer.title);
            offers.push(rules::AdmittedOffer {
                key: offer.key,
                category: offer.category,
                kind,
                price_coins: price,
                currency: rules::CurrencyKind::TibiaCoin,
                delivery,
                eligibility,
            });
        }
        Some(Self {
            rules: rules::Catalogue::admit(document.catalog_revision, offers).ok()?,
            titles,
            item_revisions,
        })
    }

    pub(super) fn title(&self, offer: &str) -> &str {
        self.titles.get(offer).map_or("", String::as_str)
    }

    pub(super) fn item_revision(&self, offer: &str) -> Option<&str> {
        self.item_revisions.get(offer).map(String::as_str)
    }
}

/// The Platform and wire spelling of a currency.
pub(super) const fn currency_text(currency: rules::CurrencyKind) -> &'static str {
    match currency {
        rules::CurrencyKind::TibiaCoin => "TIBIA_COIN",
        rules::CurrencyKind::TransferableTibiaCoin => "TRANSFERABLE_TIBIA_COIN",
    }
}
