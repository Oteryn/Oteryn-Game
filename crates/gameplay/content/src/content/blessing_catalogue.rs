//! Admitted blessing data; the 2026-10-18 owner ruling supersedes source price conflicts.

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum PriceFamily {
    Regular,
    Enhanced,
    Twist,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Blessing {
    pub service: String,
    pub storage_key: String,
    pub name: String,
    pub npcs: Vec<String>,
    price_family: PriceFamily,
    pub premium: bool,
    pub store_tibia_coins: u32,
}

impl Blessing {
    /// Interprets the owner-selected price curve at the transaction's locked level.
    pub fn price(&self, level: u32) -> u64 {
        let level = u64::from(level);
        match self.price_family {
            PriceFamily::Regular if level <= 30 => 2_000,
            PriceFamily::Regular if level < 120 => 2_000 + 200 * (level - 30),
            PriceFamily::Regular => 20_000 + 75 * (level - 120),
            PriceFamily::Enhanced if level <= 30 => 2_600,
            PriceFamily::Enhanced if level < 120 => 2_600 + 260 * (level - 30),
            PriceFamily::Enhanced => 26_000 + 100 * (level - 120),
            PriceFamily::Twist if level <= 30 => 2_000,
            PriceFamily::Twist if level < 270 => 2_000 + 200 * (level - 30),
            PriceFamily::Twist => 50_000,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct BlessingCatalogue {
    schema: String,
    classification: String,
    ruling_date: Option<String>,
    promotion_price_effect: String,
    pub blessings: Vec<Blessing>,
}

impl BlessingCatalogue {
    pub fn load() -> Result<Self, &'static str> {
        Self::parse(include_str!(
            "../../../../../rulesets/character/blessings/catalogue.json"
        ))
    }

    fn parse(input: &str) -> Result<Self, &'static str> {
        let catalogue: Self = serde_json::from_str(input).map_err(|_| "invalid blessing data")?;
        if catalogue.schema != "OTERYN_BLESSING_CATALOGUE/v1"
            || !matches!(
                catalogue.classification.as_str(),
                "PROVEN" | "DERIVED" | "PROVEN_BY_OWNER"
            )
            || (catalogue.classification == "PROVEN_BY_OWNER"
                && catalogue.ruling_date.as_deref() != Some("2026-10-18"))
            || catalogue.promotion_price_effect != "none"
            || catalogue.blessings.len() != 8
        {
            return Err("unadmitted blessing evidence");
        }
        let mut keys = std::collections::BTreeSet::new();
        for blessing in &catalogue.blessings {
            let Some(stem) = blessing.service.strip_prefix("oteryn:service.blessing.") else {
                return Err("invalid blessing service");
            };
            if blessing.storage_key != format!("oteryn:blessing.{stem}")
                || !keys.insert(&blessing.service)
                || blessing.npcs.is_empty()
                || blessing.npcs.len() > 2
            {
                return Err("invalid blessing binding");
            }
        }
        Ok(catalogue)
    }
}

#[cfg(test)]
#[path = "blessing_catalogue_tests.rs"]
mod tests;
