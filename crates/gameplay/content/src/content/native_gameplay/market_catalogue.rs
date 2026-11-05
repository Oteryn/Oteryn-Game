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
