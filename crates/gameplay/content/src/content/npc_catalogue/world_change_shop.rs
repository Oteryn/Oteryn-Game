//! Source offers extend the existing NPC service model before its normal checks.
//!
//! Lumberjack retained wiki page76247/rev893906 npc_conditions:9-13, SHA256
//! a31cf523648ff168e34f12a31429a05d8db42be4e7a0daa6df49dc6e32355ce2.
//! It records a 50-gold wood sale, no quantity, and no guaranteed quest payout.
//! Conservative contract: Chip buys ONE wood for 50 gold through ordinary trade.
//! Wood5901: Crystal201189f data/items/items.xml:15919, SHA256
//! 8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509.
use super::service::Record;
use crate::content::{
    ProjectV2Declaration, ProjectV2DefinitionRef, ProjectV2Family, ProjectV2Identity,
    ProjectV2ServiceOffer, ProjectV2ServiceOfferDirection,
};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldChangeOffer {
    pub change: &'static str,
    pub npc: &'static str,
    pub item: &'static str,
    pub direction: ProjectV2ServiceOfferDirection,
    pub unit_price: u64,
    pub count: u32,
}
pub const OFFERS: [WorldChangeOffer; 1] = [WorldChangeOffer {
    change: "lumberjack",
    npc: "oteryn:npc.chip",
    item: "oteryn:item.tibia.i5901",
    direction: ProjectV2ServiceOfferDirection::BuyFromPlayer,
    unit_price: 50,
    count: 1,
}];
const CONTRACT: &str = "lumberjack:chip:buy-from-player:wood5901:50gold:assume-count1:v1";

pub fn required_change(
    npc: &str,
    item: &str,
    direction: ProjectV2ServiceOfferDirection,
    price: u64,
    count: u32,
) -> Result<Option<&'static str>, &'static str> {
    if !OFFERS.iter().any(|row| row.npc == npc) {
        return Ok(None);
    }
    OFFERS
        .iter()
        .find(|row| {
            row.npc == npc
                && row.item == item
                && row.direction == direction
                && row.unit_price == price
                && row.count == count
        })
        .map(|row| Some(row.change))
        .ok_or("world-change NPC offer mismatch")
}
pub fn book_digest(source_tree_digest: &str) -> String {
    Sha256::new()
        .chain_update(source_tree_digest.as_bytes())
        .chain_update([0])
        .chain_update(CONTRACT.as_bytes())
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn reference(family: ProjectV2Family, key: &str) -> ProjectV2DefinitionRef {
    ProjectV2DefinitionRef {
        family,
        key: key.into(),
        revision: "definition-r1".into(),
    }
}
/// A replacement NPC declaration plus one ordinary Service declaration per NPC.
/// No parallel price resolver, vendor, scheduler or value ledger is installed.
pub(super) fn declarations(
    records: &[Record<'_>],
) -> Vec<(ProjectV2DefinitionRef, ProjectV2Declaration)> {
    let npcs: std::collections::BTreeSet<_> = OFFERS.iter().map(|r| r.npc).collect();
    let mut declarations = Vec::new();
    for npc in npcs {
        let Some((npc_ref, declaration)) = records
            .iter()
            .find(|(r, _)| r.family == ProjectV2Family::Npc && r.key == npc)
        else {
            continue;
        };
        let service_key = format!(
            "oteryn:service.world_change.{}.trade",
            npc.trim_start_matches("oteryn:npc.")
        );
        let service = reference(ProjectV2Family::Service, &service_key);
        if records.iter().any(|(r, _)| **r == service) {
            continue;
        }
        let mut updated = (**declaration).clone();
        let ProjectV2Declaration::Npc { services, .. } = &mut updated else {
            continue;
        };
        services.push(service.clone());
        declarations.push(((**npc_ref).clone(), updated));
        declarations.push((
            service,
            ProjectV2Declaration::Service {
                identity: ProjectV2Identity {
                    key: service_key,
                    revision: "definition-r1".into(),
                },
                bank: false,
                offers: OFFERS
                    .iter()
                    .filter(|r| r.npc == npc)
                    .map(|r| ProjectV2ServiceOffer {
                        item: reference(ProjectV2Family::Item, r.item),
                        direction: r.direction,
                        unit_price: r.unit_price,
                        currency: None,
                        count: Some(r.count),
                        sub_type: None,
                        parity_pending: false,
                    })
                    .collect(),
                recipes: Vec::new(),
                routes: Vec::new(),
                fields: Vec::new(),
            },
        ));
    }
    declarations
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chip_has_one_price_owner_and_no_quest_gold_reward() {
        let offer = OFFERS[0];
        assert_eq!(
            required_change(offer.npc, offer.item, offer.direction, 50, 1),
            Ok(Some("lumberjack"))
        );
        for (price, count) in [(0, 1), (49, 1), (50, 2), (500, 1)] {
            assert!(required_change(offer.npc, offer.item, offer.direction, price, count).is_err());
        }
        assert!(
            required_change(
                offer.npc,
                offer.item,
                ProjectV2ServiceOfferDirection::SellToPlayer,
                50,
                1
            )
            .is_err()
        );
        assert_eq!(
            required_change(
                "oteryn:npc.someone_else",
                offer.item,
                offer.direction,
                50,
                1
            ),
            Ok(None)
        );
        assert_ne!(book_digest("a"), book_digest("b"));
    }
}
