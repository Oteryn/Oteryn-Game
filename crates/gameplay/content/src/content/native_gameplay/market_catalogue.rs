//! Market type admission, independent of spell-use/physical Item profiles.
//!
//! Crystal 201189f871d23597b82caf88fd611454a2ddefb8 `src/items/items.cpp:215-228`
//! derives ware metadata from appearances; `src/game/game.cpp:10440-10443` requires it.
//! The accepted 2026-10-31 Crystal parity packet supplies that population. This is
//! type admission only: it grants no physical policy, mint authority or instance
//! attributes. The durable Market selector still qualifies the actual depot items.
use super::super::{item_ref::ItemDefinitionIndex, model::ContentError};
use serde::Deserialize;

const SOURCE: &[u8] = include_bytes!(
    "../../../../../../docs/agents/evidence/OTV2-20261031-item-market-crystal-parity-v1.json"
);
pub(super) const SOURCE_SHA256: &str =
    "0ea1fbee37110cdf3fa81a8db8e1ff75167653055926cfc1b3b4ec4ff5b1114d";
const SOURCE_COUNT: usize = 5_095;

#[derive(Debug, Clone)]
pub(super) struct MarketCatalogue {
    ids: Vec<u32>,
}

#[derive(Deserialize)]
struct Admission {
    schema: String,
    crystal_market_ids: Vec<u32>,
}

impl MarketCatalogue {
    pub(super) fn load(expected_sha256: &str) -> Result<Self, ContentError> {
        if expected_sha256 != SOURCE_SHA256 {
            return Err(ContentError::InvalidArtifact("native Market admission pin"));
        }
        Self::decode(SOURCE)
    }

    fn decode(bytes: &[u8]) -> Result<Self, ContentError> {
        let invalid = || ContentError::InvalidArtifact("native Market admission source");
        if bytes.len() > 128 * 1024 || super::hex(super::sha256(bytes)) != SOURCE_SHA256 {
            return Err(invalid());
        }
        let admission: Admission = serde_json::from_slice(bytes).map_err(|_| invalid())?;
        let ids = admission.crystal_market_ids;
        if admission.schema != "OTERYN_ITEM_MARKET_CRYSTAL_PARITY/v1"
            || ids.len() != SOURCE_COUNT
            || ids.first() == Some(&0)
            || ids.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(invalid());
        }
        Ok(Self { ids })
    }

    pub(super) fn admits(&self, index: &ItemDefinitionIndex, key: &str, revision: &str) -> bool {
        // The packet names these existing r1 definitions, not future revisions or aliases.
        if revision != "definition-r1" || index.definition_ref("Item", key, revision).is_none() {
            return false;
        }
        let Some(suffix) = key.strip_prefix("oteryn:item.tibia.i") else {
            return false;
        };
        if suffix.starts_with('0') || !suffix.bytes().all(|byte| byte.is_ascii_digit()) {
            return false;
        }
        suffix
            .parse::<u32>()
            .is_ok_and(|id| self.ids.binary_search(&id).is_ok())
    }
}

/// Numeric navigation facts only. They never participate in type admission or activation.
#[derive(Debug, Clone, Copy)]
pub struct MarketMetadata {
    pub category: u8,
    pub forge_class: Option<u8>,
}
#[derive(Deserialize)]
struct MetadataDocument {
    schema: String,
    admission_sha256: String,
    appearances_sha256: String,
    rows: Vec<(u32, u8, Option<u8>)>,
}
fn filter_metadata() -> Option<&'static std::collections::BTreeMap<u32, MarketMetadata>> {
    static DATA: std::sync::OnceLock<Option<std::collections::BTreeMap<u32, MarketMetadata>>> =
        std::sync::OnceLock::new();
    DATA.get_or_init(|| {
        let document: MetadataDocument =
            serde_json::from_slice(include_bytes!("market_filter_catalogue.json")).ok()?;
        let admitted = MarketCatalogue::decode(SOURCE).ok()?;
        if document.schema != "OTERYN_MARKET_FILTERS/v1"
            || document.admission_sha256 != SOURCE_SHA256
            || document.appearances_sha256
                != "2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2"
            || document
                .rows
                .iter()
                .map(|r| r.0)
                .ne(admitted.ids.iter().copied())
            || document
                .rows
                .iter()
                .any(|r| !(1..=31).contains(&r.1) || r.2.is_some_and(|v| v > 4))
        {
            return None;
        }
        Some(
            document
                .rows
                .into_iter()
                .map(|(id, category, forge_class)| {
                    (
                        id,
                        MarketMetadata {
                            category,
                            forge_class,
                        },
                    )
                })
                .collect(),
        )
    })
    .as_ref()
}

impl super::NativeGameplayState {
    /// Stable active-generation Market type references, without rescanning non-marketable types.
    pub fn market_references(&self) -> Vec<std::num::NonZeroU32> {
        let Some(index) = self.item_index() else {
            return Vec::new();
        };
        let mut found = std::collections::BTreeSet::new();
        if let Some(catalogue) = &self.market_catalogue {
            for id in &catalogue.ids {
                let key = format!("oteryn:item.tibia.i{id}");
                if let Some(reference) = index.definition_ref("Item", &key, "definition-r1") {
                    found.insert(reference);
                }
            }
        }
        for record in self.item_profiles() {
            use super::super::ReferenceItemField::Known;
            if !matches!(&record.semantics.trade_restrictions,Known(trade) if trade.marketable==Known(true))
            {
                continue;
            }
            for (key, revision) in [
                (
                    &record.production_definition.production_key,
                    &record.production_definition.revision_ref,
                ),
                (&record.authoring.item.key, &record.authoring.item.revision),
            ] {
                if let Some(reference) = index.definition_ref("Item", key, revision) {
                    found.insert(reference);
                }
            }
        }
        found.into_iter().collect()
    }
    pub fn market_metadata_ready(&self) -> bool {
        filter_metadata().is_some()
    }
    pub fn market_metadata(&self, key: &str, revision: &str) -> Option<MarketMetadata> {
        if revision != "definition-r1" || !self.marketable_definition(key, revision) {
            return None;
        }
        let suffix = key.strip_prefix("oteryn:item.tibia.i")?;
        if suffix.starts_with('0') {
            return None;
        }
        filter_metadata()?
            .get(&suffix.parse::<u32>().ok()?)
            .copied()
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "test fixture construction and assertions"
)]
mod tests {
    use super::*;

    #[test]
    fn reaudit_market_population_catalogue_covers_all_source_ids() {
        let catalogue = MarketCatalogue::load(SOURCE_SHA256).unwrap();
        let index = ItemDefinitionIndex::decode(include_bytes!(
            "../../../../../../tools/content-schema/native-gameplay/item-keys.json"
        ))
        .unwrap();
        assert_eq!(catalogue.ids.len(), 5_095);
        for id in &catalogue.ids {
            let key = format!("oteryn:item.tibia.i{id}");
            assert!(catalogue.admits(&index, &key, "definition-r1"), "{key}");
        }
        assert!(catalogue.admits(&index, "oteryn:item.tibia.i10219", "definition-r1"));
        assert!(!catalogue.admits(&index, "oteryn:item.tibia.i10219", "definition-r2"));
        assert!(!catalogue.admits(&index, "candidate:item/10219", "definition-r1"));
        assert!(!catalogue.admits(&index, "oteryn:item.tibia.i010219", "definition-r1"));
        assert!(!catalogue.admits(&index, "oteryn:item.tibia.i3031", "definition-r1"));
    }

    #[test]
    fn market_catalogue_requires_active_definition_and_exact_source_pin() {
        let catalogue = MarketCatalogue::load(SOURCE_SHA256).unwrap();
        let empty =
            ItemDefinitionIndex::decode(br#"{"schema":"OTERYN_NATIVE_ITEM_KEYS/v1","records":[]}"#)
                .unwrap();
        assert!(!catalogue.admits(&empty, "oteryn:item.tibia.i10219", "definition-r1"));
        assert!(MarketCatalogue::load(&"0".repeat(64)).is_err());
        let mut changed = SOURCE.to_vec();
        changed.push(b' ');
        assert!(MarketCatalogue::decode(&changed).is_err());
        assert!(MarketCatalogue::decode(b"{}").is_err());
    }
}

#[cfg(test)]
mod filter_tests {
    use super::*;
    #[test]
    fn t124_numeric_filters_cover_the_full_admitted_cohort()
    -> Result<(), Box<dyn std::error::Error>> {
        let metadata = filter_metadata().ok_or("qualified navigation metadata")?;
        assert_eq!(metadata.len(), 5095);
        assert_eq!(metadata.values().filter(|m| m.category == 26).count(), 835);
        assert_eq!(metadata.values().filter(|m| m.category == 27).count(), 36);
        let groups: std::collections::BTreeSet<_> = metadata.values().map(|m| m.category).collect();
        assert_eq!(groups.len(), 27);
        Ok(())
    }
}
