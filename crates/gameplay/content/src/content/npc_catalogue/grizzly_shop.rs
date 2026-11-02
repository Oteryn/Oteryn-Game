//! Crystal 201189f Grizzly shop facts, admitted by the existing NPC service model.
//! Rank is a quest-point requirement; the durable NPC trade owner checks it again.

use super::service::Record;
use crate::content::{
    ProjectV2Declaration, ProjectV2DefinitionRef, ProjectV2Family, ProjectV2Identity,
    ProjectV2ServiceOffer, ProjectV2ServiceOfferDirection,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::sync::OnceLock;

pub const NPC: &str = "oteryn:npc.grizzly_adams";
const SERVICE: &str = "oteryn:service.grizzly_adams.trade";
const SOURCE: &str = include_str!("../../../../../../content/quests/killing-in-the-name-of.json");

#[derive(Debug, Deserialize)]
pub struct GrizzlyShopOffer {
    pub item: ProjectV2DefinitionRef,
    pub direction: ProjectV2ServiceOfferDirection,
    pub unit_price: u64,
    pub minimum_points: i64,
    pub donor_line: u32,
}

impl GrizzlyShopOffer {
    fn offer(&self) -> ProjectV2ServiceOffer {
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

#[derive(Deserialize)]
struct Catalogue {
    schema: String,
    source_revision: String,
    shop: Vec<GrizzlyShopOffer>,
}

pub fn offers() -> Result<&'static [GrizzlyShopOffer], &'static str> {
    static CATALOGUE: OnceLock<Result<Catalogue, &'static str>> = OnceLock::new();
    CATALOGUE
        .get_or_init(|| {
            let catalogue: Catalogue =
                serde_json::from_str(SOURCE).map_err(|_| "Grizzly shop data")?;
            if catalogue.schema != "OTERYN_GRIZZLY_ADAMS/v1"
                || catalogue.source_revision != "201189f871d23597b82caf88fd611454a2ddefb8"
                || catalogue.shop.len() != 56
                || catalogue.shop.iter().any(|row| {
                    row.item.family != ProjectV2Family::Item
                        || !matches!(row.minimum_points, 10 | 20 | 40 | 70)
                        || row.unit_price == 0
                })
            {
                return Err("Grizzly shop source bounds");
            }
            let unique: std::collections::BTreeSet<_> = catalogue
                .shop
                .iter()
                .map(|row| (&row.item, row.direction))
                .collect();
            if unique.len() != catalogue.shop.len() {
                return Err("Grizzly shop duplicate Item");
            }
            Ok(catalogue)
        })
        .as_ref()
        .map(|catalogue| catalogue.shop.as_slice())
        .map_err(|error| *error)
}

/// Exact source offer identity, including price and count. Unknown Grizzly offers are
/// refused, so a caller cannot remove the gate by changing one request field.
pub fn required_points(
    npc: &str,
    item: &str,
    direction: ProjectV2ServiceOfferDirection,
    unit_price: u64,
    count: u32,
) -> Result<Option<i64>, &'static str> {
    if npc != NPC {
        return Ok(None);
    }
    if count != 1 {
        return Err("Grizzly shop count");
    }
    offers()?
        .iter()
        .find(|row| {
            row.item.key == item && row.direction == direction && row.unit_price == unit_price
        })
        .map(|row| Some(row.minimum_points))
        .ok_or("unknown Grizzly shop offer")
}

pub fn visible(npc: &str, offer: &ProjectV2ServiceOffer, points: Option<i64>) -> bool {
    match required_points(
        npc,
        &offer.item.key,
        offer.direction,
        offer.unit_price,
        offer.count.unwrap_or(1),
    ) {
        Ok(None) => true,
        Ok(Some(minimum)) => points.is_some_and(|points| points >= minimum),
        Err(_) => false,
    }
}

/// The ordinary book's revision includes the newly qualified source facts. The original
/// World Project source-tree digest remains unchanged and continues to name its own bytes.
pub fn book_digest(source_tree_digest: &str) -> String {
    Sha256::new()
        .chain_update(b"oteryn:grizzly-shop:v1\0")
        .chain_update(source_tree_digest.as_bytes())
        .chain_update(SOURCE.as_bytes())
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Two data declarations extend the existing service model before its ordinary Item,
/// count, currency and arbitrage checks. No second shop model or price table is installed.
pub(super) fn declarations(
    records: &[Record<'_>],
) -> Vec<(ProjectV2DefinitionRef, ProjectV2Declaration)> {
    let Some((reference, declaration)) = records
        .iter()
        .find(|(reference, _)| reference.family == ProjectV2Family::Npc && reference.key == NPC)
    else {
        return Vec::new();
    };
    let Ok(offers) = offers() else {
        return Vec::new();
    };
    let service_ref = ProjectV2DefinitionRef {
        family: ProjectV2Family::Service,
        key: SERVICE.into(),
        revision: "definition-r1".into(),
    };
    if records
        .iter()
        .any(|(reference, _)| **reference == service_ref)
    {
        return Vec::new();
    }
    let mut npc = (**declaration).clone();
    let ProjectV2Declaration::Npc { services, .. } = &mut npc else {
        return Vec::new();
    };
    services.push(service_ref.clone());
    vec![
        ((**reference).clone(), npc),
        (
            service_ref,
            ProjectV2Declaration::Service {
                identity: ProjectV2Identity {
                    key: SERVICE.into(),
                    revision: "definition-r1".into(),
                },
                bank: false,
                offers: offers.iter().map(GrizzlyShopOffer::offer).collect(),
                recipes: Vec::new(),
                routes: Vec::new(),
                fields: Vec::new(),
            },
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grizzly_rank_shop_thresholds_and_unknown_offer_fail_closed() -> Result<(), &'static str> {
        let rows = offers()?;
        for points in [-1, 0, 9, 10, 19, 20, 39, 40, 69, 70, 100] {
            for row in rows {
                assert_eq!(
                    visible(NPC, &row.offer(), Some(points)),
                    points >= row.minimum_points
                );
                assert!(!visible(NPC, &row.offer(), None));
            }
        }
        let buy = ProjectV2ServiceOfferDirection::SellToPlayer;
        assert_eq!(
            required_points(NPC, "oteryn:item.tibia.i5907", buy, 35_000, 1)?,
            Some(20)
        );
        assert_eq!(
            required_points(NPC, "oteryn:item.tibia.i9601", buy, 1_000, 1)?,
            Some(70)
        );
        assert!(required_points(NPC, "oteryn:item.tibia.i5907", buy, 1, 1).is_err());
        assert!(required_points(NPC, "oteryn:item.tibia.i5907", buy, 35_000, 2).is_err());
        Ok(())
    }

    #[test]
    fn grizzly_eye_and_bone_are_distinct_canonical_items_at_source_prices()
    -> Result<(), &'static str> {
        let sell = ProjectV2ServiceOfferDirection::BuyFromPlayer;
        assert_eq!(
            required_points(NPC, "oteryn:item.tibia.i11488", sell, 350, 1)?,
            Some(10)
        );
        assert_eq!(
            required_points(NPC, "oteryn:item.tibia.i11491", sell, 500, 1)?,
            Some(10)
        );
        assert!(required_points(NPC, "oteryn:item.tibia.i11491", sell, 350, 1).is_err());
        Ok(())
    }
}
