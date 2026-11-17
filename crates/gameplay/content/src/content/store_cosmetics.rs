//! Bounded Store cosmetics admitted from priced reference evidence.
use crate::content::native_gameplay::NativeCosmeticsView;
use crate::domain::appearance::{AppearanceContent, EarnedCosmetic, Sex, UnlockFact};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::LazyLock;

pub const PROVENANCE: &str = "store-cosmetics-v1";
pub const DOCUMENT: &[u8] = include_bytes!("../../../../../rulesets/economy/store/cosmetics.json");
#[derive(Debug, Deserialize)]
pub struct StoreCosmeticOffer {
    pub key: String,
    pub category: String,
    pub price: u64,
    pub unlock_key: String,
    pub preview: Preview,
}
#[derive(Debug, Deserialize)]
pub struct Preview {
    pub kind: u8,
    pub key: String,
}
#[derive(Deserialize)]
struct Document {
    source: Source,
    offers: Vec<StoreCosmeticOffer>,
}
#[derive(Deserialize)]
struct Source {
    outfit_looks: BTreeMap<String, [u32; 2]>,
    mount_looks: BTreeMap<String, u32>,
}
static CATALOGUE: LazyLock<Option<Document>> = LazyLock::new(|| {
    let document: Document = serde_json::from_slice(DOCUMENT).ok()?;
    (document.offers.len() <= 32 && document.offers.iter().all(|o| o.price > 0)).then_some(document)
});
pub fn offers() -> Option<&'static [StoreCosmeticOffer]> {
    Some(&CATALOGUE.as_ref()?.offers)
}
pub fn offer(key: &str) -> Option<&'static StoreCosmeticOffer> {
    offers()?.iter().find(|offer| offer.key == key)
}
pub fn compatible(content: &NativeCosmeticsView, offer: &StoreCosmeticOffer) -> bool {
    let Some(document) = CATALOGUE.as_ref() else {
        return false;
    };
    if offer.preview.key != offer.unlock_key {
        return false;
    }
    match offer.preview.kind {
        2 => document
            .source
            .outfit_looks
            .get(&offer.unlock_key)
            .is_some_and(|looks| {
                content.look_type(&offer.unlock_key, Sex::Male) == Some(looks[0])
                    && content.look_type(&offer.unlock_key, Sex::Female) == Some(looks[1])
            }),
        3 => document
            .source
            .mount_looks
            .get(&offer.unlock_key)
            .is_some_and(|look| {
                content
                    .mount_metadata(&offer.unlock_key)
                    .is_some_and(|(actual, _, _)| actual == *look)
            }),
        _ => false,
    }
}
pub fn unlock(
    content: &NativeCosmeticsView,
    key: &str,
    provenance: &str,
    target: EarnedCosmetic<'_>,
) -> Option<UnlockFact> {
    let offer = offer(key)?;
    let matches = provenance == PROVENANCE
        && compatible(content, offer)
        && match target {
            EarnedCosmetic::Outfit(key) => offer.preview.kind == 2 && key == offer.unlock_key,
            EarnedCosmetic::Addon(key, addon) => {
                offer.preview.kind == 2
                    && key == offer.unlock_key
                    && addon > 0
                    && content
                        .outfit_addon_count(&offer.unlock_key)
                        .is_some_and(|count| addon <= count)
            }
            EarnedCosmetic::Mount(key) => offer.preview.kind == 3 && key == offer.unlock_key,
        };
    Some(if matches {
        UnlockFact::Compatible
    } else {
        UnlockFact::Incompatible
    })
}
#[cfg(test)]
mod tests {
    #[test]
    fn admitted_store_price_and_looks_match_proven_reference()
    -> Result<(), Box<dyn std::error::Error>> {
        let evidence: serde_json::Value = serde_json::from_slice(include_bytes!(
            "../../../../../docs/reference/content-prep/outfits-content-1/admission-evidence/evidence-00000-00135.json"
        ))?;
        let records = evidence["records"].as_array().ok_or("records")?;
        for offer in super::offers().ok_or("catalogue")? {
            let reference = records
                .iter()
                .find(|r| r["facts"]["name"]["value"] == "Field Surgeon")
                .ok_or("reference")?;
            assert_eq!(
                reference["facts"]["full_store_price"]["classification"],
                "PROVEN"
            );
            assert_eq!(
                reference["facts"]["full_store_price"]["value"].as_u64(),
                Some(offer.price)
            );
            let looks = super::CATALOGUE
                .as_ref()
                .ok_or("catalogue")?
                .source
                .outfit_looks
                .get(&offer.unlock_key)
                .ok_or("looks")?;
            assert_eq!(reference["facts"]["male_look_type"]["value"], looks[0]);
            assert_eq!(reference["facts"]["female_look_type"]["value"], looks[1]);
        }
        Ok(())
    }
}
