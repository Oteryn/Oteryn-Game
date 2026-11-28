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
    // Catalogue size is independent of the existing 32-offer wire page.
    (document.offers.len() <= 128 && document.offers.iter().all(|o| o.price > 0))
        .then_some(document)
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
    fn field_surgeon_store_price_and_looks_match_proven_reference()
    -> Result<(), Box<dyn std::error::Error>> {
        let evidence: serde_json::Value = serde_json::from_slice(include_bytes!(
            "../../../../../docs/reference/content-prep/outfits-content-1/admission-evidence/evidence-00000-00135.json"
        ))?;
        let records = evidence["records"].as_array().ok_or("records")?;
        let offer = super::offer("store:outfit:field-surgeon").ok_or("offer")?;
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
        Ok(())
    }

    #[test]
    fn production_store_outfit_bundles_grant_base_and_both_addons()
    -> Result<(), Box<dyn std::error::Error>> {
        use super::*;
        use crate::content::native_gameplay::{
            NativeGameplayInput, test_support::cosmetics_fixture,
        };
        use crate::domain::appearance::AccountUnlocks;
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../content/spells.manifest.json");
        let input = NativeGameplayInput::from_manifest(&manifest).map_err(|e| format!("{e:?}"))?;
        let cosmetics = cosmetics_fixture::decode_cosmetics(&input.cosmetics.bytes)
            .map_err(|e| format!("{e:?}"))?;
        let expected = [
            ("aerial-disciple", 1936, 1937, 870),
            ("arbalester", 1449, 1450, 600),
            ("arena-champion", 884, 885, 870),
            ("armoured-archer", 1618, 1619, 600),
            ("bat-knight", 1874, 1875, 870),
            ("beekeeper", 1776, 1777, 870),
            ("blade-dancer", 1745, 1746, 750),
            ("breezy-garb", 1245, 1246, 600),
            ("celestial-avenger", 1725, 1726, 870),
            ("ceremonial-garb", 695, 694, 750),
            ("darklight-evoker", 1675, 1676, 1500),
            ("doom-knight", 1713, 1714, 600),
            ("dragon-knight", 1444, 1445, 870),
            ("dragon-slayer", 1288, 1289, 5000),
            ("evoker", 725, 724, 840),
            ("fencer", 1575, 1576, 750),
            ("field-surgeon", 1814, 1815, 870),
            ("flamefury-mage", 1680, 1681, 870),
            ("forest-warden", 1415, 1416, 750),
            ("frost-tracer", 1612, 1613, 750),
            ("ghost-blade", 1489, 1490, 600),
            ("grove-keeper", 908, 909, 870),
            ("guidon-bearer", 1186, 1187, 870),
            ("herbalist", 1021, 1020, 750),
            ("herder", 1279, 1280, 750),
            ("jouster", 1331, 1332, 870),
            ("lion-of-war", 1206, 1207, 1750),
            ("lupine-warden", 899, 900, 840),
            ("martial-artist", 1837, 1838, 870),
            ("mercenary", 1056, 1057, 870),
            ("merry-garb", 1382, 1383, 600),
            ("moon-guardian", 1976, 1977, 870),
            ("moth-cape", 1338, 1339, 600),
            ("necromancer", 1845, 1846, 870),
            ("nordic-chieftain", 1500, 1501, 750),
            ("owl-keeper", 1173, 1174, 600),
            ("pharaoh", 955, 956, 750),
            ("philosopher", 873, 874, 750),
            ("phoenix-evoker", 1920, 1921, 870),
            ("pumpkin-mummy", 1127, 1128, 870),
            ("royal-pumpkin", 760, 759, 840),
            ("rune-master", 1384, 1385, 870),
            ("sea-dog", 750, 749, 600),
            ("seaweaver", 733, 732, 570),
            ("shadowlotus-disciple", 1581, 1582, 600),
            ("siege-master", 1051, 1050, 600),
            ("sinister-archer", 1102, 1103, 600),
            ("spirit-caller", 699, 698, 600),
            ("sun-priest", 1023, 1024, 750),
            ("trailblazer", 1292, 1293, 600),
            ("trophy-hunter", 957, 958, 870),
            ("vampire-noble", 1948, 1949, 750),
            ("veteran-paladin", 1204, 1205, 750),
            ("void-master", 1202, 1203, 750),
            ("winged-druid", 1831, 1832, 870),
            ("winter-warden", 853, 852, 870),
        ];
        for (slug, male, female, price) in expected {
            let offer_key = format!("store:outfit:{slug}");
            let outfit_key = format!("oteryn:outfit.{}", slug.replace('-', "_"));
            let offer = offer(&offer_key).ok_or_else(|| format!("missing {offer_key}"))?;
            assert_eq!(
                (offer.unlock_key.as_str(), offer.price),
                (outfit_key.as_str(), price)
            );
            assert_eq!(cosmetics.look_type(&outfit_key, Sex::Male), Some(male));
            assert_eq!(cosmetics.look_type(&outfit_key, Sex::Female), Some(female));
            assert!(compatible(&cosmetics, offer));
            let facts = [(offer_key, PROVENANCE.to_owned())];
            let owned = crate::domain::appearance::EarnedCosmetics {
                content: &cosmetics,
                facts: &facts,
            };
            assert_eq!(owned.outfit(&outfit_key), UnlockFact::Compatible);
            for addon in [1, 2] {
                assert_eq!(owned.addon(&outfit_key, addon), UnlockFact::Compatible);
            }
        }
        for (slug, male, female) in [
            ("retro-citizen", 974, 975),
            ("retro-hunter", 972, 973),
            ("retro-knight", 970, 971),
            ("retro-mage", 968, 969),
        ] {
            let offer_key = format!("store:outfit:{slug}");
            let outfit_key = format!("oteryn:outfit.{}", slug.replace('-', "_"));
            let offer = offer(&offer_key).ok_or_else(|| format!("missing {offer_key}"))?;
            assert_eq!(
                (offer.unlock_key.as_str(), offer.price),
                (outfit_key.as_str(), 870)
            );
            assert_eq!(cosmetics.look_type(&outfit_key, Sex::Male), Some(male));
            assert_eq!(cosmetics.look_type(&outfit_key, Sex::Female), Some(female));
            assert_eq!(cosmetics.outfit_addon_count(&outfit_key), Some(0));
            assert!(compatible(&cosmetics, offer));
        }
        let offers = offers().ok_or("catalogue")?;
        assert_eq!(
            offers
                .iter()
                .filter(|offer| offer.category == "cosmetics.outfits")
                .count(),
            expected.len() + 7
        );
        assert_eq!(
            offers
                .iter()
                .filter(|offer| offer.category == "cosmetics.mounts")
                .count(),
            19
        );
        assert_eq!(offers.len(), expected.len() + 7 + 19);
        Ok(())
    }

    #[test]
    fn production_wiki_outfit_definitions_load_with_both_addons()
    -> Result<(), Box<dyn std::error::Error>> {
        use super::*;
        use crate::content::native_gameplay::{
            NativeGameplayInput, test_support::cosmetics_fixture,
        };

        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../content/spells.manifest.json");
        let input = NativeGameplayInput::from_manifest(&manifest).map_err(|e| format!("{e:?}"))?;
        let cosmetics = cosmetics_fixture::decode_cosmetics(&input.cosmetics.bytes)
            .map_err(|e| format!("{e:?}"))?;
        for (name, male, female) in [
            ("aerial_disciple", 1936, 1937),
            ("captains", 1940, 1941),
            ("illuminated_warrior", 1973, 1974),
            ("moon_guardian", 1976, 1977),
            ("phoenix_evoker", 1920, 1921),
            ("vampire_noble", 1948, 1949),
        ] {
            let key = format!("oteryn:outfit.{name}");
            assert_eq!(cosmetics.look_type(&key, Sex::Male), Some(male), "{key}");
            assert_eq!(
                cosmetics.look_type(&key, Sex::Female),
                Some(female),
                "{key}"
            );
            assert_eq!(cosmetics.outfit_addon_count(&key), Some(2), "{key}");
        }
        Ok(())
    }
}
