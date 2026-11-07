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

const SOURCE: &str = include_str!("../../../../../../content/quests/shared-quest-shops.json");

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
    pub requirements: Vec<Requirement>,
}
impl Offer {
    fn project(&self) -> ProjectV2ServiceOffer {
        ProjectV2ServiceOffer {
            item: self.item.clone(),
            direction: self.direction,
            unit_price: self.unit_price,
            currency: None,
            count: Some(1),
            sub_type: None,
            parity_pending: false,
        }
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
        .get_or_init(|| parse(SOURCE))
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
    shop.offers
        .iter()
        .find(|o| {
            o.item == *item && o.direction == direction && o.unit_price == price && count == 1
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
    Sha256::new()
        .chain_update(b"oteryn:quest-shops:v1\0")
        .chain_update(previous.as_bytes())
        .chain_update(SOURCE.as_bytes())
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
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
        if records.iter().any(|(r, _)| **r == service) {
            continue;
        }
        let mut npc = (**declaration).clone();
        let ProjectV2Declaration::Npc { services, .. } = &mut npc else {
            continue;
        };
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
                offers: shop.offers.iter().map(Offer::project).collect(),
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
}
