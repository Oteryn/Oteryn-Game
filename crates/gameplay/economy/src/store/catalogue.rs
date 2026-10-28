use std::collections::{BTreeMap, BTreeSet};

pub const MAX_OFFERS_PER_PAGE: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CurrencyKind {
    TibiaCoin,
    TransferableTibiaCoin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfferKind {
    Item,
    Mount,
    Outfit,
    Service,
    CoinPackage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeliveryKind {
    Item { item_key: String, count: u32 },
    Unlock { unlock_key: String },
    Service,
    CoinPackage,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Eligibility {
    pub premium_required: bool,
    pub vocations: BTreeSet<String>,
    pub minimum_level: Option<u32>,
    pub reject_if_unlock_owned: Option<String>,
    pub per_character_limit: Option<u32>,
}

/// The runtime projection of one record in `store-offer.schema.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedOffer {
    pub key: String,
    pub category: String,
    pub kind: OfferKind,
    pub price_coins: u64,
    pub currency: CurrencyKind,
    pub delivery: DeliveryKind,
    pub eligibility: Eligibility,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Catalogue {
    revision: String,
    offers: BTreeMap<String, AdmittedOffer>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogueError {
    EmptyRevision,
    EmptyOfferKey,
    DuplicateOfferKey,
    ZeroPageSize,
    PageTooLarge,
}

impl Catalogue {
    pub fn admit(
        catalog_revision: impl Into<String>,
        offers: impl IntoIterator<Item = AdmittedOffer>,
    ) -> Result<Self, CatalogueError> {
        let revision = catalog_revision.into();
        if revision.is_empty() {
            return Err(CatalogueError::EmptyRevision);
        }
        let mut admitted = BTreeMap::new();
        for offer in offers {
            if offer.key.is_empty() {
                return Err(CatalogueError::EmptyOfferKey);
            }
            if admitted.insert(offer.key.clone(), offer).is_some() {
                return Err(CatalogueError::DuplicateOfferKey);
            }
        }
        Ok(Self {
            revision,
            offers: admitted,
        })
    }

    #[must_use]
    pub fn revision(&self) -> &str {
        &self.revision
    }

    #[must_use]
    pub fn offer(&self, key: &str) -> Option<&AdmittedOffer> {
        self.offers.get(key)
    }

    pub fn offers_in_category(
        &self,
        category: &str,
        after_key: Option<&str>,
        limit: usize,
    ) -> Result<Vec<&AdmittedOffer>, CatalogueError> {
        if limit == 0 {
            return Err(CatalogueError::ZeroPageSize);
        }
        if limit > MAX_OFFERS_PER_PAGE {
            return Err(CatalogueError::PageTooLarge);
        }
        Ok(self
            .offers
            .values()
            .filter(|offer| {
                offer.category == category && after_key.is_none_or(|key| offer.key.as_str() > key)
            })
            .take(limit)
            .collect())
    }
}

#[cfg(test)]
#[path = "tests/catalogue_tests.rs"]
mod tests;
