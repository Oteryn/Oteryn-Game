//! Source-owned conditional shops extend the ordinary NPC catalogue and fenced trade owner.
use super::service::Record;
use crate::content::{
    ProjectV2Declaration, ProjectV2DefinitionRef, ProjectV2Family, ProjectV2Identity,
    ProjectV2ServiceOffer, ProjectV2ServiceOfferDirection,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::sync::OnceLock;

mod native_fields;

const SOURCE: &str = include_str!("../../../../../../content/quests/shared-quest-shops.json");
const FARMINE: &str = include_str!("../../../../../../content/npcs/farmine-quest-shops.json");

pub const ALBINIUS: &str = "oteryn:npc.albinius";
pub const BLANK_IMBUEMENT_SCROLL: &str = "oteryn:item.tibia.i51442";
pub const HEAVY_OLD_TOME: &str = "oteryn:item.tibia.i23986";
pub const FORGOTTEN_KNOWLEDGE: &str = "oteryn:quest.forgotten_knowledge_quest";

pub const ORDINARY_UNLOCK_VALUE: i64 = 1;

/// Crystal 201189f8: albinius.lua:73-95 and telas.lua:633-679.
pub const ORDINARY_REQUIREMENTS: [(&str, &str, &str); 2] = [
    (
        ALBINIUS,
        FORGOTTEN_KNOWLEDGE,
        "oteryn:quest-progress/quest/u11_02/forgotten_knowledge/tomes",
    ),
    (
        "oteryn:npc.telas",
        "oteryn:quest.shadows_of_yalahar_quest",
        "oteryn:quest-progress/crystalserver/quest/u8_5/shadows_of_yalahar/door_final",
    ),
];

pub fn ordinary_requirement(npc: &str) -> Option<(&'static str, &'static str)> {
    ORDINARY_REQUIREMENTS
        .iter()
        .find_map(|&(key, quest, track)| (npc == key).then_some((quest, track)))
}

fn ordinary_service(npc: &str) -> Option<&'static str> {
    match npc {
        "oteryn:npc.esrik" => Some("oteryn:service.trade.esrik"),
        "oteryn:npc.pompan" => Some("oteryn:service.trade.pompan"),
        _ => None,
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Requirement {
    pub quest: String,
    pub track: String,
    pub initial: i64,
    pub minimum: i64,
    pub maximum: i64,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Offer {
    pub item: ProjectV2DefinitionRef,
    pub direction: ProjectV2ServiceOfferDirection,
    pub unit_price: u64,
    #[serde(default)]
    pub count: Option<u32>,
    #[serde(default)]
    pub sub_type: Option<u16>,
    pub requirements: Vec<Requirement>,
}
impl Offer {
    fn project(&self) -> ProjectV2ServiceOffer {
        native_fields::project(self)
    }
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shop {
    pub npc: String,
    pub service: String,
    pub offers: Vec<Offer>,
    pub sources: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: u8,
    shops: Vec<Shop>,
}

fn parse(source: &str) -> Result<Vec<Shop>, &'static str> {
    let document: Document = serde_json::from_str(source).map_err(|_| "quest shop data")?;
    let mut npcs = BTreeSet::new();
    let mut services = BTreeSet::new();
    if document.schema != 1 || document.shops.len() > 256 {
        return Err("quest shop bound");
    }
    for shop in &document.shops {
        let mut items = BTreeSet::new();
        if !shop.npc.starts_with("oteryn:npc.")
            || shop.npc.len() > 128
            || !shop.service.starts_with("oteryn:service.")
            || shop.service.len() > 128
            || !npcs.insert(&shop.npc)
            || !services.insert(&shop.service)
            || shop.offers.is_empty()
            || shop.offers.len() > 128
            || shop.sources.is_empty()
            || shop.sources.iter().any(String::is_empty)
        {
            return Err("quest shop identity");
        }
        for offer in &shop.offers {
            if offer.item.family != ProjectV2Family::Item
                || !offer.item.key.starts_with("oteryn:item.")
                || offer.item.revision.is_empty()
                || !native_fields::valid(offer)
                || offer.unit_price == 0
                || offer.unit_price > u64::MAX / 100
                || !items.insert((&offer.item, offer.direction))
                || offer.requirements.is_empty()
                || offer.requirements.len() > 8
            {
                return Err("quest shop offer");
            }
            let mut tracks = BTreeSet::new();
            if offer.requirements.iter().any(|r| {
                !r.quest.starts_with("oteryn:quest.")
                    || !r.track.starts_with("oteryn:quest-progress/")
                    || r.track.len() > 128
                    || r.minimum > r.maximum
                    || !tracks.insert(&r.track)
            }) {
                return Err("quest shop predicate");
            }
        }
    }
    Ok(document.shops)
}

pub fn shops() -> Result<&'static [Shop], &'static str> {
    static SHOPS: OnceLock<Result<Vec<Shop>, &'static str>> = OnceLock::new();
    SHOPS
        .get_or_init(|| {
            let mut shops = parse(SOURCE)?;
            shops.extend(parse(FARMINE)?);
            Ok(shops)
        })
        .as_ref()
        .map(Vec::as_slice)
        .map_err(|e| *e)
}

/// Unknown fields of a protected NPC's offer cannot remove its requirements.
pub fn required<'a>(
    catalogue: &'a [Shop],
    npc: &str,
    item: &ProjectV2DefinitionRef,
    direction: ProjectV2ServiceOfferDirection,
    price: u64,
    count: u32,
) -> Result<Option<&'a Offer>, &'static str> {
    let Some(shop) = catalogue.iter().find(|s| s.npc == npc) else {
        return Ok(None);
    };
    if ordinary_service(npc).is_some()
        && !shop
            .offers
            .iter()
            .any(|offer| offer.item == *item && offer.direction == direction)
    {
        return Ok(None);
    }
    shop.offers
        .iter()
        .find(|o| {
            o.item == *item
                && o.direction == direction
                && o.unit_price == price
                && count == o.count.unwrap_or(1)
        })
        .map(Some)
        .ok_or("unknown protected quest shop offer")
}

pub fn eligible(offer: &Offer, mut value: impl FnMut(&Requirement) -> Option<i64>) -> bool {
    offer.requirements.iter().all(|r| {
        let value = value(r).unwrap_or(r.initial);
        (r.minimum..=r.maximum).contains(&value)
    })
}

pub fn book_digest(previous: &str) -> String {
    if shops().is_ok_and(|shops| shops.is_empty()) {
        return previous.into();
    }
    let mut hash = Sha256::new()
        .chain_update(b"oteryn:quest-shops:v1\0")
        .chain_update(previous.as_bytes())
        .chain_update(SOURCE.as_bytes())
        .chain_update(FARMINE.as_bytes());
    for (npc, quest, track) in ORDINARY_REQUIREMENTS {
        for value in [npc, quest, track] {
            hash.update(value.as_bytes());
            hash.update([0]);
        }
    }
    hash.update(ORDINARY_UNLOCK_VALUE.to_be_bytes());
    hash.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

pub(super) fn declarations(
    records: &[Record<'_>],
) -> Vec<(ProjectV2DefinitionRef, ProjectV2Declaration)> {
    let Ok(shops) = shops() else {
        return Vec::new();
    };
    let mut additions = Vec::new();
    for shop in shops {
        let Some((reference, declaration)) = records
            .iter()
            .find(|(r, _)| r.family == ProjectV2Family::Npc && r.key == shop.npc)
        else {
            continue;
        };
        let service = ProjectV2DefinitionRef {
            family: ProjectV2Family::Service,
            key: shop.service.clone(),
            revision: "definition-r1".into(),
        };
        let mut npc = (**declaration).clone();
        let ProjectV2Declaration::Npc { services, .. } = &mut npc else {
            continue;
        };
        if let Some(key) = ordinary_service(&shop.npc)
            && let Some((reference, declaration)) = records
                .iter()
                .find(|(r, _)| r.family == ProjectV2Family::Service && r.key == key)
        {
            let mut declaration = (**declaration).clone();
            if let ProjectV2Declaration::Service { offers, .. } = &mut declaration {
                offers.retain(|offer| {
                    !shop
                        .offers
                        .iter()
                        .any(|gated| gated.item == offer.item && gated.direction == offer.direction)
                });
                additions.push(((**reference).clone(), declaration));
            }
        }
        let offers = shop.offers.iter().map(Offer::project).collect();
        if let Some((_, existing)) = records.iter().find(|(r, _)| **r == service) {
            // A protected merchant may already own this service in the base
            // catalogue. Publish only the reviewed offers with matching gates.
            let mut existing = (**existing).clone();
            if let ProjectV2Declaration::Service { offers: stock, .. } = &mut existing
                && services.contains(&service)
            {
                *stock = offers;
                additions.push((service, existing));
            }
            continue;
        }
        services.push(service.clone());
        additions.push(((**reference).clone(), npc));
        additions.push((
            service,
            ProjectV2Declaration::Service {
                identity: ProjectV2Identity {
                    key: shop.service.clone(),
                    revision: "definition-r1".into(),
                },
                bank: false,
                offers,
                recipes: Vec::new(),
                routes: Vec::new(),
                fields: Vec::new(),
            },
        ));
    }
    additions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn black_bert_retained_stock_requires_the_completed_guild_track()
    -> Result<(), Box<dyn std::error::Error>> {
        let model = crate::content::NpcDataCatalogue::load(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../content"),
            std::ffi::OsStr::new("world"),
            crate::content::npc_catalogue_preproduction_limits(),
        )?
        .service_model_for_project()?;
        let npc_key = "oteryn:npc.black_bert";
        let (_, npc) = model
            .npcs()
            .iter()
            .find(|(reference, _)| reference.key == npc_key)
            .ok_or("Black Bert")?;
        let service = npc
            .services
            .iter()
            .find(|key| key.key == "oteryn:service.trade.black_bert")
            .and_then(|key| model.services().get(key))
            .ok_or("Black Bert's shop")?;
        assert_eq!(service.offers.len(), 127);
        let arthei = service
            .offers
            .iter()
            .find(|offer| offer.item.key == "oteryn:item.tibia.i8720")
            .ok_or("Dust of Arthei stock")?;
        assert_eq!(arthei.unit_price, 4_000, "Dust of Arthei live shop price");
        for offer in &service.offers {
            let guard = required(
                shops()?,
                npc_key,
                &offer.item,
                offer.direction,
                offer.unit_price,
                offer.count.unwrap_or(1),
            )?
            .ok_or("unguarded Black Bert offer")?;
            assert!(!eligible(guard, |_| None));
            assert!(!eligible(guard, |_| Some(8)));
            assert!(eligible(guard, |requirement| {
                assert_eq!(
                    requirement.track,
                    "oteryn:quest-progress/quest/u8_2/the_thieves_guild_quest/questline"
                );
                Some(9)
            }));
        }
        Ok(())
    }

    #[test]
    fn alesar_catalogue_matches_crystal_after_efreet_mission03()
    -> Result<(), Box<dyn std::error::Error>> {
        use ProjectV2ServiceOfferDirection::{BuyFromPlayer, SellToPlayer};

        let catalogue = crate::content::NpcDataCatalogue::load(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../content"),
            std::ffi::OsStr::new("world"),
            crate::content::npc_catalogue_preproduction_limits(),
        )?;
        let model = catalogue.service_model_for_project()?;
        let npc_key = "oteryn:npc.alesar";
        let (_, npc) = model
            .npcs()
            .iter()
            .find(|(r, _)| r.key == npc_key)
            .ok_or("Alesar")?;
        let offers: Vec<_> = npc
            .services
            .iter()
            .flat_map(|r| &model.services()[r].offers)
            .collect();
        // Crystal data-global/npc/alesar.lua:24-48, in player buy/sell order.
        let items = [
            (3432, Some(5000), Some(900)),
            (3429, None, Some(800)),
            (7428, None, Some(10000)),
            (3383, Some(1500), Some(400)),
            (3384, Some(1000), Some(250)),
            (3322, None, Some(2000)),
            (7419, None, Some(15000)),
            (3281, None, Some(17000)),
            (7407, None, Some(8000)),
            (3284, Some(5000), None),
            (3370, None, Some(5000)),
            (3318, None, Some(2000)),
            (3371, None, Some(5000)),
            (3574, None, Some(150)),
            (7421, None, Some(22000)),
            (7411, None, Some(20000)),
            (3299, None, Some(50)),
            (3307, None, Some(150)),
            (3297, Some(6000), Some(900)),
            (3324, None, Some(6000)),
            (3373, None, Some(500)),
            (7413, None, Some(4000)),
            (3428, None, Some(8000)),
            (3434, None, Some(15000)),
            (3369, None, Some(5000)),
        ];
        for (id, buy, sell) in items {
            let key = format!("oteryn:item.tibia.i{id}");
            for (direction, price) in [(SellToPlayer, buy), (BuyFromPlayer, sell)] {
                let offer = offers
                    .iter()
                    .find(|o| o.item.key == key && o.direction == direction);
                // Crystal data/items/items.xml:8551 gives the ice rapier one charge.
                let expected = price.map(|price| (price, (id == 3284).then_some(1)));
                assert_eq!(
                    offer.map(|o| (o.unit_price, o.sub_type)),
                    expected,
                    "Alesar {id} {direction:?}"
                );
            }
        }
        assert_eq!(offers.len(), 29);

        for offer in offers {
            let gate = required(
                shops()?,
                npc_key,
                &offer.item,
                offer.direction,
                offer.unit_price,
                offer.count.unwrap_or(1),
            )?
            .ok_or("Alesar trade gate")?;
            assert_eq!(gate.requirements.len(), 1);
            let requirement = &gate.requirements[0];
            assert_eq!(
                requirement.quest,
                "oteryn:quest.the_djinn_war_efreet_faction"
            );
            assert_eq!(
                requirement.track,
                "oteryn:quest-progress/quest/u7_4/djinn_war/efreet_faction/mission03"
            );
            assert!(!eligible(gate, |_| None));
            for stage in 0..=4 {
                assert_eq!(eligible(gate, |_| Some(stage)), stage == 3);
            }
        }
        Ok(())
    }

    #[test]
    fn protected_offer_identity_and_fresh_requirements_fail_closed() -> Result<(), &'static str> {
        let source = r#"{"schema":1,"shops":[{"npc":"oteryn:npc.cael","service":"oteryn:service.quest_shop.cael","sources":["fixture:source"],"offers":[{"item":{"family":"Item","key":"oteryn:item.tibia.i2965","revision":"definition-r1"},"direction":"SellToPlayer","unit_price":5000,"requirements":[{"quest":"oteryn:quest.the_new_frontier_quest","track":"oteryn:quest-progress/quest/u8_54/the_new_frontier/tomeof_knowledge","initial":0,"minimum":6,"maximum":12}]}]}]}"#;
        let catalogue = parse(source)?;
        let offer = &catalogue[0].offers[0];
        assert!(!eligible(offer, |_| None));
        assert!(!eligible(offer, |_| Some(5)));
        assert!(eligible(offer, |_| Some(6)));
        assert!(
            required(
                &catalogue,
                "oteryn:npc.cael",
                &offer.item,
                offer.direction,
                5000,
                1
            )?
            .is_some()
        );
        assert!(
            required(
                &catalogue,
                "oteryn:npc.cael",
                &offer.item,
                offer.direction,
                1,
                1
            )
            .is_err()
        );
        assert!(
            required(
                &catalogue,
                "oteryn:npc.cael",
                &offer.item,
                offer.direction,
                5000,
                2
            )
            .is_err()
        );
        let mut forged = offer.item.clone();
        forged.revision = "foreign".into();
        assert!(
            required(
                &catalogue,
                "oteryn:npc.cael",
                &forged,
                offer.direction,
                5000,
                1
            )
            .is_err()
        );
        Ok(())
    }
    #[test]
    fn quest_shops_native_fields_preserve_charge_count_and_exact_offer_match()
    -> Result<(), &'static str> {
        let source = r#"{"schema":1,"shops":[{"npc":"oteryn:npc.yaman","service":"oteryn:service.quest_shop.yaman","sources":["fixture:source"],"offers":[{"item":{"family":"Item","key":"oteryn:item.tibia.i3085","revision":"definition-r1"},"direction":"SellToPlayer","unit_price":1000,"count":1,"sub_type":200,"requirements":[{"quest":"oteryn:quest.the_djinn_war_efreet_faction","track":"oteryn:quest-progress/quest/u7_4/djinn_war/efreet_faction/mission03","initial":0,"minimum":3,"maximum":3}]}]}]}"#;
        let catalogue = parse(source)?;
        let offer = &catalogue[0].offers[0];
        let projected = offer.project();
        assert_eq!(projected.count, Some(1));
        assert_eq!(projected.sub_type, Some(200));
        assert!(
            required(
                &catalogue,
                "oteryn:npc.yaman",
                &offer.item,
                offer.direction,
                1000,
                1
            )?
            .is_some()
        );
        assert!(
            required(
                &catalogue,
                "oteryn:npc.yaman",
                &offer.item,
                offer.direction,
                1000,
                200
            )
            .is_err()
        );
        assert!(!eligible(offer, |_| Some(2)));
        assert!(eligible(offer, |_| Some(3)));
        let fluid = source.replace("\"count\":1,\"sub_type\":200", "\"count\":1,\"sub_type\":6");
        assert_eq!(parse(&fluid)?[0].offers[0].project().sub_type, Some(6));
        assert!(parse(&source.replace("\"count\":1", "\"count\":0")).is_err());
        // Main NPC-TRADE-3 admits u16 charge subtypes; the live trade owner
        // separately checks fluid ranges and exact native full-charge counts.
        assert_eq!(
            parse(&source.replace("\"sub_type\":200", "\"sub_type\":256"))?[0].offers[0]
                .project()
                .sub_type,
            Some(256)
        );
        assert!(parse(&source.replace("\"sub_type\":200", "\"sub_type\":65536")).is_err());
        Ok(())
    }
}
