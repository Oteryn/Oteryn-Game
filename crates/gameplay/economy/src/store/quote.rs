use std::collections::{BTreeMap, BTreeSet};

use super::{Catalogue, CurrencyKind};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CharacterStoreView {
    pub premium: bool,
    pub vocation: String,
    pub level: u32,
    pub owned_unlocks: BTreeSet<String>,
    pub purchases_by_offer: BTreeMap<String, u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurchaseQuote {
    pub catalog_revision: String,
    pub offer_key: String,
    pub price: u64,
    pub currency: CurrencyKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuoteRefusal {
    UnknownOffer,
    PremiumRequired,
    VocationNotEligible,
    LevelTooLow { required: u32 },
    UnlockAlreadyOwned,
    PerCharacterLimitReached { limit: u32 },
}

pub fn build_quote(
    catalogue: &Catalogue,
    offer_key: &str,
    character: &CharacterStoreView,
) -> Result<PurchaseQuote, QuoteRefusal> {
    let offer = catalogue
        .offer(offer_key)
        .ok_or(QuoteRefusal::UnknownOffer)?;
    let eligibility = &offer.eligibility;
    if eligibility.premium_required && !character.premium {
        return Err(QuoteRefusal::PremiumRequired);
    }
    if !eligibility.vocations.is_empty() && !eligibility.vocations.contains(&character.vocation) {
        return Err(QuoteRefusal::VocationNotEligible);
    }
    if let Some(required) = eligibility
        .minimum_level
        .filter(|level| character.level < *level)
    {
        return Err(QuoteRefusal::LevelTooLow { required });
    }
    if eligibility
        .reject_if_unlock_owned
        .as_ref()
        .is_some_and(|key| character.owned_unlocks.contains(key))
    {
        return Err(QuoteRefusal::UnlockAlreadyOwned);
    }
    if let Some(limit) = eligibility.per_character_limit.filter(|limit| {
        character
            .purchases_by_offer
            .get(offer_key)
            .copied()
            .unwrap_or(0)
            >= *limit
    }) {
        return Err(QuoteRefusal::PerCharacterLimitReached { limit });
    }
    Ok(PurchaseQuote {
        catalog_revision: catalogue.revision().to_owned(),
        offer_key: offer.key.clone(),
        price: offer.price_coins,
        currency: offer.currency,
    })
}

#[cfg(test)]
#[path = "tests/quote_tests.rs"]
mod tests;
