//! Crystal market parity: Items whose Crystal appearance carries the market flag are marketable.
//!
//! Successor of the Market-true v2 promotion. Crystal's `flags.market` wins over the Wiki and
//! name evidence that held these Items back. A row that names the Wave 1 Wiki `marketable: false`
//! it supersedes (owner decision 2026-10-31) may flip that one known false to true.
use super::{
    ProjectReferenceRecord, ProjectV2Draft, ReferenceItemField, ReferenceItemTradeRestrictions,
    world_project_sha256,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

pub const ITEM_MARKET_CRYSTAL_PARITY_PACKET: &[u8] = include_bytes!(
    "../../../../../docs/agents/evidence/OTV2-20261031-item-market-crystal-parity-v1.json"
);
// Regenerate with `lower_client_market_crystal_parity_packet.py --donor <crystal checkout>`;
// the generator prints this digest. The packet follows the pinned Crystal market list.
pub const ITEM_MARKET_CRYSTAL_PARITY_PACKET_SHA256: &str =
    "0ea1fbee37110cdf3fa81a8db8e1ff75167653055926cfc1b3b4ec4ff5b1114d";
const PROMOTIONS: usize = 205;
const SUPERSEDES_WAVE1_FALSE: &str = "WAVE1_WIKI_MARKETABLE_FALSE";

type TradeGroup = ReferenceItemField<ReferenceItemTradeRestrictions>;

#[derive(Deserialize)]
struct Packet {
    schema: String,
    counts: Counts,
    promotions: Vec<Row>,
}
#[derive(Deserialize)]
struct Counts {
    promotions: usize,
}
#[derive(Deserialize)]
struct Row {
    item_key: String,
    marketable: bool,
    #[serde(default)]
    supersedes: Option<String>,
}
#[derive(Debug, PartialEq, Eq)]
pub struct MarketCrystalParity {
    pub fields: usize,
    pub changed: usize,
}

pub fn apply_item_market_crystal_parity_v1(
    draft: &mut ProjectV2Draft,
) -> Result<MarketCrystalParity, String> {
    if world_project_sha256(ITEM_MARKET_CRYSTAL_PARITY_PACKET)
        != ITEM_MARKET_CRYSTAL_PARITY_PACKET_SHA256
    {
        return Err("Market Crystal parity packet digest drift".into());
    }
    let items = draft.core.records.iter_mut().filter_map(|r| match r {
        ProjectReferenceRecord::Item {
            identity,
            semantics,
            ..
        } => Some((identity.key.as_str(), &mut semantics.trade_restrictions)),
        _ => None,
    });
    apply_rows(items, ITEM_MARKET_CRYSTAL_PARITY_PACKET, PROMOTIONS)
}

fn current(group: &TradeGroup) -> Result<Option<bool>, String> {
    use ReferenceItemField::{Known, Unknown};
    match group {
        Unknown => Ok(None),
        Known(trade) => match trade.marketable {
            Unknown => Ok(None),
            Known(value) => Ok(Some(value)),
            _ => Err("blocked marketable evidence state".into()),
        },
        _ => Err("blocked trade group evidence state".into()),
    }
}

fn apply_rows<'a>(
    mut items: impl Iterator<Item = (&'a str, &'a mut TradeGroup)>,
    bytes: &[u8],
    expected: usize,
) -> Result<MarketCrystalParity, String> {
    let packet: Packet = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if packet.schema != "OTERYN_ITEM_MARKET_CRYSTAL_PARITY/v1"
        || packet.counts.promotions != expected
        || packet.promotions.len() != expected
    {
        return Err("Market Crystal parity packet shape/count drift".into());
    }
    let mut items = items.try_fold(BTreeMap::new(), |mut map, (key, group)| {
        if map.insert(key, group).is_some() {
            return Err("duplicate native Item key".to_owned());
        }
        Ok(map)
    })?;
    let (mut seen, mut changed) = (BTreeSet::new(), 0);
    for row in &packet.promotions {
        let supersedes_false = match row.supersedes.as_deref() {
            None => false,
            Some(SUPERSEDES_WAVE1_FALSE) => true,
            Some(_) => return Err("unknown Market Crystal parity supersession".into()),
        };
        if !row.marketable || !seen.insert(&row.item_key) {
            return Err("invalid or duplicate Market Crystal parity row".into());
        }
        let group = items
            .get(row.item_key.as_str())
            .ok_or("Market Crystal parity has no native Item")?;
        match current(group)? {
            Some(false) if !supersedes_false => return Err("known marketable conflict".into()),
            Some(false) | None => changed += 1,
            Some(true) => (),
        }
    }
    // Validate the complete packet before changing the independently optional bool.
    for row in &packet.promotions {
        let group = items
            .get_mut(row.item_key.as_str())
            .ok_or("validated Market Item missing")?;
        if matches!(**group, ReferenceItemField::Unknown) {
            **group = ReferenceItemField::Known(ReferenceItemTradeRestrictions {
                tradeable: ReferenceItemField::Unknown,
                marketable: ReferenceItemField::Unknown,
                vocations: ReferenceItemField::Unknown,
                account_binding_policy: ReferenceItemField::Unknown,
                character_binding_policy: ReferenceItemField::Unknown,
            });
        }
        if let ReferenceItemField::Known(trade) = &mut **group {
            trade.marketable = ReferenceItemField::Known(true);
        }
    }
    Ok(MarketCrystalParity {
        fields: expected,
        changed,
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use ReferenceItemField::{Conflict, Known, NotApplicable, Unknown};
    fn row(key: &str) -> serde_json::Value {
        serde_json::json!({"item_key":key,"marketable":true})
    }
    fn packet(rows: serde_json::Value) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({"schema":"OTERYN_ITEM_MARKET_CRYSTAL_PARITY/v1","counts":{"promotions":rows.as_array().expect("rows").len()},"promotions":rows})).expect("packet")
    }
    fn trade(state: ReferenceItemField<bool>) -> TradeGroup {
        Known(ReferenceItemTradeRestrictions {
            tradeable: Known(false),
            marketable: state,
            vocations: Conflict,
            account_binding_policy: Unknown,
            character_binding_policy: NotApplicable,
        })
    }
    #[test]
    fn promotes_unknown_and_missing_groups_and_is_idempotent() {
        let (mut unknown, mut missing, mut known) = (trade(Unknown), Unknown, trade(Known(true)));
        let bytes = packet(serde_json::json!([row("a"), row("b"), row("c")]));
        for changed in [2, 0] {
            let applied = apply_rows(
                [("a", &mut unknown), ("b", &mut missing), ("c", &mut known)].into_iter(),
                &bytes,
                3,
            )
            .expect("applied");
            assert_eq!(applied.changed, changed);
            assert_eq!(unknown, trade(Known(true)));
            assert!(matches!(&missing, Known(t) if t.marketable == Known(true)));
        }
    }
    #[test]
    fn rejects_every_block_atomically() {
        for state in [
            Conflict,
            NotApplicable,
            trade(Known(false)),
            trade(Conflict),
            trade(NotApplicable),
        ] {
            let mut first = trade(Unknown);
            let mut second = state;
            let (first_before, second_before) = (first.clone(), second.clone());
            assert!(
                apply_rows(
                    [("first", &mut first), ("second", &mut second)].into_iter(),
                    &packet(serde_json::json!([row("first"), row("second")])),
                    2
                )
                .is_err()
            );
            assert_eq!((first, second), (first_before, second_before));
        }
    }
    #[test]
    fn flips_known_false_only_for_a_superseding_row() {
        let superseding = serde_json::json!([{"item_key":"item","marketable":true,"supersedes":SUPERSEDES_WAVE1_FALSE}]);
        let mut item = trade(Known(false));
        let applied = apply_rows(
            std::iter::once(("item", &mut item)),
            &packet(superseding),
            1,
        )
        .expect("superseded");
        assert_eq!((applied.changed, item), (1, trade(Known(true))));
        for rows in [
            serde_json::json!([row("item")]),
            serde_json::json!([{"item_key":"item","marketable":true,"supersedes":"OTHER"}]),
        ] {
            let mut item = trade(Known(false));
            assert!(apply_rows(std::iter::once(("item", &mut item)), &packet(rows), 1).is_err());
            assert_eq!(item, trade(Known(false)));
        }
    }
    #[test]
    fn rejects_bad_rows_missing_items_and_duplicate_native_identity() {
        for rows in [
            serde_json::json!([row("item"), row("item")]),
            serde_json::json!([row("item"), row("missing")]),
            serde_json::json!([{"item_key":"item","marketable":false}]),
        ] {
            let expected = rows.as_array().expect("rows").len();
            let mut item: TradeGroup = Unknown;
            assert!(
                apply_rows(
                    std::iter::once(("item", &mut item)),
                    &packet(rows),
                    expected
                )
                .is_err()
            );
            assert_eq!(item, Unknown);
        }
        let (mut first, mut second): (TradeGroup, TradeGroup) = (Unknown, Unknown);
        assert!(
            apply_rows(
                [("item", &mut first), ("item", &mut second)].into_iter(),
                &packet(serde_json::json!([row("item")])),
                1
            )
            .is_err()
        );
    }
}
