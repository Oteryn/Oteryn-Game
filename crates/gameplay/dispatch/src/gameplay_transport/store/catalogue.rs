//! The admitted runtime Store catalogue (`rulesets/economy/store/catalogue.json`).
//! `tools/content-schema/store-authoring/build_runtime_catalogue.py` projects the pinned offer sources.
use crate::content::blessing_catalogue::BlessingCatalogue;
use oteryn_gameplay_economy::store as rules;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: String,
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
    hireling: Option<HirelingOffer>,
    #[serde(default)]
    unlock_key: Option<String>,
    #[serde(default)]
    preview: Option<Preview>,
    #[serde(rename = "source_line")]
    _source_line: u32,
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct Preview {
    kind: u8,
    key: String,
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
pub(super) const CHARM_EXPANSION_UNLOCK: &str = "charm-expansion";
pub(super) const BLESSING_CATEGORY: &str = "consumables.blessings";
pub(super) const BLESSING_OFFER_PREFIX: &str = "store:blessing:";
const BLESSING_ENTITLEMENT_PREFIX: &str = "store-blessing:";

pub(super) fn blessing_storage_key(entitlement: &str) -> Option<&str> {
    entitlement.strip_prefix(BLESSING_ENTITLEMENT_PREFIX)
}

fn enabled_by_default() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct HirelingOffer {
    pub(super) effect: String,
    pub(super) source_id: u16,
}

pub(super) struct RuntimeCatalogue {
    pub(super) rules: rules::Catalogue,
    titles: BTreeMap<String, String>,
    keys: Vec<String>,
    previews: BTreeMap<String, (u8, String)>,
    item_revisions: BTreeMap<String, String>,
    hirelings: BTreeMap<String, HirelingOffer>,
}

impl RuntimeCatalogue {
    pub(super) fn from_blessings() -> Option<Self> {
        let blessings = BlessingCatalogue::load().ok()?.blessings;
        let offers = blessings
            .iter()
            .map(|blessing| {
                let suffix = blessing.storage_key.strip_prefix("oteryn:blessing.")?;
                Some(serde_json::json!({
                    "key": format!("{BLESSING_OFFER_PREFIX}{suffix}"),
                    "category": BLESSING_CATEGORY,
                    "title": blessing.name,
                    "price": blessing.store_tibia_coins,
                    "currency": currency_text(rules::CurrencyKind::TibiaCoin),
                    "unlock_key": format!("{BLESSING_ENTITLEMENT_PREFIX}{}", blessing.storage_key),
                    "source_line": 0,
                }))
            })
            .collect::<Option<Vec<_>>>()?;
        let document = serde_json::json!({
            "schema": "OTERYN_STORE_RUNTIME_CATALOGUE/v1",
            "source": "rulesets/character/blessings/catalogue.json",
            "offers": offers,
        });
        super::load_active_catalogue(&serde_json::to_vec(&document).ok()?)
    }

    /// `None` for anything but a closed, non-empty v1 document of positive Tibia Coin prices.
    pub(super) fn from_json(bytes: &[u8]) -> Option<Self> {
        Self::from_json_with_revision(bytes, None)
    }

    pub(super) fn from_json_with_revision(
        bytes: &[u8],
        active_revision: Option<&str>,
    ) -> Option<Self> {
        let mut value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
        let document_revision = catalogue_revision(&mut value)?;
        let document: Document = serde_json::from_value(value).ok()?;
        let revision = active_revision.map_or(document_revision, str::to_owned);
        Self::admit(document, revision)
    }

    /// The archived import preserves both the quote and its Platform grant identity.
    /// Only Game-owned offers participate in purchase replay.
    pub(super) fn from_platform(document: &serde_json::Value, revision: &str) -> Option<Self> {
        let hirelings: Document = serde_json::from_slice(super::HIRELING_CATALOGUE_JSON).ok()?;
        let mut entries = Vec::new();
        for entry in document.get("offers")?.as_array()? {
            let [unit] = entry.get("units")?.as_array()?.as_slice() else {
                continue; // Platform-only offers may contain several units.
            };
            let kind = unit.get("unit_kind")?.as_str()?;
            if kind == "PREMIUM_TIME" || entry["purchasable"] != true {
                continue;
            }
            if !matches!(kind, "ITEM" | "COSMETIC")
                || entry["currency"] != "COINS"
                || entry["max_quantity"] != 1
                || unit["quantity"] != 1
            {
                return None;
            }
            let key = entry.get("offer_key")?.as_str()?;
            let grant = unit.get("grant_key")?.as_str()?;
            let is_item = grant.starts_with("oteryn:item.");
            let item = if is_item {
                Some(Item {
                    key: grant.into(),
                    revision: format!("definition-r{}", unit.get("grant_revision")?.as_str()?),
                    _charges: 0,
                })
            } else {
                None
            };
            let hireling = hirelings
                .offers
                .iter()
                .find(|row| row.key == key)
                .and_then(|row| row.hireling.clone());
            if key.starts_with("store:hireling:") && hireling.is_none() {
                continue;
            }
            let unlock_key = if !is_item && hireling.is_none() {
                if !matches!(grant, PREY_EXPANSION_UNLOCK | CHARM_EXPANSION_UNLOCK)
                    && !(key == grant && super::hunting_offers::is_offer(key))
                    && !crate::content::store_cosmetics::offer(key)
                        .is_some_and(|offer| offer.unlock_key == grant)
                {
                    continue;
                }
                Some(grant.into())
            } else {
                None
            };
            entries.push(Offer {
                key: key.into(),
                category: "retained".into(),
                title: key.into(),
                price: Some(entry.get("unit_price")?.as_str()?.parse().ok()?),
                enabled: true,
                currency: "TIBIA_COIN".into(),
                item,
                unlock_key,
                hireling,
                preview: None,
                _source_line: 0,
            });
        }
        Self::admit(
            Document {
                schema: "OTERYN_STORE_RUNTIME_CATALOGUE/v1".into(),
                _source: serde_json::Value::Null,
                offers: entries,
            },
            revision.into(),
        )
    }

    fn admit(document: Document, revision: String) -> Option<Self> {
        if document.schema != "OTERYN_STORE_RUNTIME_CATALOGUE/v1" || document.offers.is_empty() {
            return None;
        }
        let mut titles = BTreeMap::new();
        let mut keys = Vec::new();
        let mut previews = BTreeMap::new();
        let mut item_revisions = BTreeMap::new();
        let mut hirelings = BTreeMap::new();
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
            let (kind, delivery, eligibility) = if let Some(hireling) = offer.hireling {
                let valid = match hireling.effect.as_str() {
                    "hire" => {
                        hireling.source_id == 25440
                            && offer.item.as_ref().is_some_and(|item| {
                                item.key == "oteryn:item.tibia.i29432"
                                    && item.revision == "definition-r1"
                            })
                    }
                    "unlock" => {
                        (1001..=1004).contains(&hireling.source_id)
                            || (2001..=2009).contains(&hireling.source_id)
                    }
                    "rename" => hireling.source_id == 25438,
                    "sex_change" => hireling.source_id == 25437,
                    _ => false,
                };
                if offer.unlock_key.is_some()
                    || !valid
                    || (hireling.effect != "hire" && offer.item.is_some())
                {
                    return None;
                }
                hirelings.insert(offer.key.clone(), hireling);
                (
                    rules::OfferKind::Service,
                    rules::DeliveryKind::Service,
                    rules::Eligibility::default(),
                )
            } else {
                match (offer.item.as_ref(), offer.unlock_key.as_ref()) {
                    (Some(item), None) => (
                        rules::OfferKind::Item,
                        rules::DeliveryKind::Item {
                            item_key: item.key.clone(),
                            count: 1,
                        },
                        rules::Eligibility::default(),
                    ),
                    (None, Some(unlock_key))
                        if offer.category == BLESSING_CATEGORY
                            && blessing_storage_key(unlock_key)
                                .and_then(|key| key.strip_prefix("oteryn:blessing."))
                                .is_some_and(|suffix| {
                                    offer.key.strip_prefix(BLESSING_OFFER_PREFIX) == Some(suffix)
                                }) =>
                    {
                        (
                            rules::OfferKind::Service,
                            rules::DeliveryKind::Unlock {
                                unlock_key: unlock_key.clone(),
                            },
                            rules::Eligibility::default(),
                        )
                    }
                    (None, Some(unlock_key))
                        if unlock_key == &offer.key
                            && super::hunting_offers::is_offer(unlock_key) =>
                    {
                        (
                            rules::OfferKind::Service,
                            rules::DeliveryKind::Unlock {
                                unlock_key: unlock_key.clone(),
                            },
                            rules::Eligibility::default(),
                        )
                    }
                    (None, Some(unlock_key))
                        if matches!(
                            unlock_key.as_str(),
                            PREY_EXPANSION_UNLOCK | CHARM_EXPANSION_UNLOCK
                        ) || crate::content::store_cosmetics::offers().is_some_and(
                            |offers| offers.iter().any(|o| o.unlock_key == *unlock_key),
                        ) =>
                    {
                        let eligibility = rules::Eligibility {
                            reject_if_unlock_owned: Some(unlock_key.clone()),
                            per_character_limit: Some(1),
                            ..rules::Eligibility::default()
                        };
                        (
                            rules::OfferKind::Service,
                            rules::DeliveryKind::Unlock {
                                unlock_key: unlock_key.clone(),
                            },
                            eligibility,
                        )
                    }
                    _ => return None,
                }
            };
            if let Some(preview) = offer.preview {
                if !(1..=3).contains(&preview.kind)
                    || preview.key.is_empty()
                    || preview.key.len() > 128
                {
                    return None;
                }
                previews.insert(offer.key.clone(), (preview.kind, preview.key));
            }
            keys.push(offer.key.clone());
            titles.insert(offer.key.clone(), offer.title);
            if let Some(item) = offer.item.as_ref() {
                item_revisions.insert(offer.key.clone(), item.revision.clone());
            }
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
            rules: rules::Catalogue::admit(revision, offers).ok()?,
            titles,
            keys,
            previews,
            item_revisions,
            hirelings,
        })
    }

    pub(super) fn entries(&self) -> impl Iterator<Item = &rules::AdmittedOffer> {
        self.keys.iter().filter_map(|key| self.rules.offer(key))
    }
    pub(super) fn preview(&self, key: &str) -> Option<&(u8, String)> {
        self.previews.get(key)
    }
    pub(super) fn hireling(&self, offer: &str) -> Option<&HirelingOffer> {
        self.hirelings.get(offer)
    }

    pub(super) fn title(&self, offer: &str) -> &str {
        self.titles.get(offer).map_or("", String::as_str)
    }

    pub(super) fn item_revision(&self, offer: &str) -> Option<&str> {
        self.item_revisions.get(offer).map(String::as_str)
    }
}

/// D2(b): lowercase hex SHA-256 of the entire catalogue JSON, with object keys
/// sorted recursively and no insignificant whitespace. Non-ASCII remains raw UTF-8,
/// never `\u` escapes. Numbers must be integers; floats (including `1.0` or `1e2`)
/// are rejected everywhere, including metadata. The derived revision is not an input field.
fn catalogue_revision(value: &mut serde_json::Value) -> Option<String> {
    fn integers_only(value: &serde_json::Value) -> bool {
        match value {
            serde_json::Value::Number(n) => n.is_i64() || n.is_u64(),
            serde_json::Value::Array(values) => values.iter().all(integers_only),
            serde_json::Value::Object(values) => values.values().all(integers_only),
            _ => true,
        }
    }
    if !integers_only(value) {
        return None;
    }
    value.sort_all_objects();
    Some(
        Sha256::digest(serde_json::to_vec(value).ok()?)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    )
}

/// The Platform and wire spelling of a currency.
pub(super) const fn currency_text(currency: rules::CurrencyKind) -> &'static str {
    match currency {
        rules::CurrencyKind::TibiaCoin => "TIBIA_COIN",
        rules::CurrencyKind::TransferableTibiaCoin => "TRANSFERABLE_TIBIA_COIN",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const UTF8: &[u8] = include_bytes!(
        "../../../../../../tools/content-schema/store-authoring/samples/revision-utf8.json"
    );

    #[test]
    fn storefix_revision_matches_python_utf8_fixture() -> Result<(), Box<dyn std::error::Error>> {
        let catalogue = RuntimeCatalogue::from_json(UTF8).ok_or("catalogue")?;
        assert_eq!(catalogue.rules.revision(), include_str!("../../../../../../tools/content-schema/store-authoring/samples/revision-utf8.sha256").trim());
        Ok(())
    }

    #[test]
    fn storefix_catalogue_rejects_all_float_numbers() -> Result<(), Box<dyn std::error::Error>> {
        for number in ["1.0", "1.5", "1e2"] {
            let mut value: serde_json::Value = serde_json::from_slice(UTF8)?;
            value["source"]["a"] = serde_json::from_str(number)?;
            assert!(
                RuntimeCatalogue::from_json(&serde_json::to_vec(&value)?).is_none(),
                "{number}"
            );
        }
        Ok(())
    }

    #[test]
    fn storefix_dispatch_loads_generated_catalogues() {
        assert!(super::super::CATALOGUE.is_some());
        assert!(super::super::HIRELING_CATALOGUE.is_some());
    }
}
