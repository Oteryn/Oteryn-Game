//! Affirmative official Market metadata, independent of general trade or admission.
use super::{
    ProjectReferenceRecord, ProjectV2Draft, ReferenceItemField, ReferenceItemSemantics,
    ReferenceItemTradeRestrictions, world_project_sha256,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

pub const ITEM_MARKET_TRUE_PACKET: &[u8] = include_bytes!(
    "../../../../docs/agents/evidence/OTV2-20261001-item-market-true-promotion-v1.json"
);
pub const ITEM_MARKET_TRUE_PACKET_SHA256: &str =
    "a3caa1afa06b56ead7e54531408be02caf1b94e13d6c72d087b3669e91d8c312";
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
}
#[derive(Debug, PartialEq, Eq)]
pub struct MarketTruePromotion {
    pub fields: usize,
    pub changed: usize,
}

pub fn apply_item_market_true_promotion_v1(
    draft: &mut ProjectV2Draft,
) -> Result<MarketTruePromotion, String> {
    if world_project_sha256(ITEM_MARKET_TRUE_PACKET) != ITEM_MARKET_TRUE_PACKET_SHA256 {
        return Err("Market packet digest drift".into());
    }
    let items = draft.core.records.iter_mut().filter_map(|r| match r {
        ProjectReferenceRecord::Item {
            identity,
            semantics,
            ..
        } => Some((identity.key.as_str(), semantics)),
        _ => None,
    });
    apply_rows(items, ITEM_MARKET_TRUE_PACKET, 4893)
}
fn current(semantics: &ReferenceItemSemantics) -> Result<Option<bool>, String> {
    use ReferenceItemField::{Known, Unknown};
    match &semantics.trade_restrictions {
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
    mut items: impl Iterator<Item = (&'a str, &'a mut ReferenceItemSemantics)>,
    bytes: &[u8],
    expected: usize,
) -> Result<MarketTruePromotion, String> {
    let packet: Packet = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if packet.schema != "OTERYN_ITEM_MARKET_TRUE_PROMOTION/v1"
        || packet.counts.promotions != expected
        || packet.promotions.len() != expected
    {
        return Err("Market packet shape/count drift".into());
    }
    let mut items = items.try_fold(BTreeMap::new(), |mut map, (key, semantics)| {
        if map.insert(key, semantics).is_some() {
            return Err("duplicate native Item key".to_owned());
        }
        Ok(map)
    })?;
    let (mut seen, mut changed) = (BTreeSet::new(), 0);
    for row in &packet.promotions {
        if !row.marketable || !seen.insert(&row.item_key) {
            return Err("invalid or duplicate Market row".into());
        }
        let semantics = items
            .get(row.item_key.as_str())
            .ok_or("Market has no native Item")?;
        match current(semantics)? {
            Some(false) => return Err("known marketable conflict".into()),
            None => changed += 1,
            Some(true) => (),
        }
    }
    // Validate the complete packet before changing the independently optional bool.
    for row in &packet.promotions {
        let semantics = items
            .get_mut(row.item_key.as_str())
            .ok_or("validated Market Item missing")?;
        if matches!(semantics.trade_restrictions, ReferenceItemField::Unknown) {
            semantics.trade_restrictions =
                ReferenceItemField::Known(ReferenceItemTradeRestrictions {
                    tradeable: ReferenceItemField::Unknown,
                    marketable: ReferenceItemField::Unknown,
                    vocations: ReferenceItemField::Unknown,
                    account_binding_policy: ReferenceItemField::Unknown,
                    character_binding_policy: ReferenceItemField::Unknown,
                });
        }
        if let ReferenceItemField::Known(trade) = &mut semantics.trade_restrictions {
            trade.marketable = ReferenceItemField::Known(true);
        }
    }
    Ok(MarketTruePromotion {
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
        serde_json::to_vec(&serde_json::json!({"schema":"OTERYN_ITEM_MARKET_TRUE_PROMOTION/v1","counts":{"promotions":rows.as_array().expect("rows").len()},"promotions":rows})).expect("packet")
    }
    fn trade(state: ReferenceItemField<bool>) -> ReferenceItemTradeRestrictions {
        ReferenceItemTradeRestrictions {
            tradeable: Known(false),
            marketable: state,
            vocations: Conflict,
            account_binding_policy: Unknown,
            character_binding_policy: NotApplicable,
        }
    }
    #[test]
    fn preserves_siblings_and_is_idempotent() {
        let mut item = ReferenceItemSemantics {
            trade_restrictions: Known(trade(Unknown)),
            ..ReferenceItemSemantics::default()
        };
        let mut expected = item.clone();
        expected.trade_restrictions = Known(trade(Known(true)));
        let bytes = packet(serde_json::json!([row("item")]));
        for changed in [1, 0] {
            assert_eq!(
                apply_rows(std::iter::once(("item", &mut item)), &bytes, 1),
                Ok(MarketTruePromotion { fields: 1, changed })
            );
            assert_eq!(item, expected);
        }
    }
    #[test]
    fn rejects_every_block_atomically() {
        for state in [
            Conflict,
            NotApplicable,
            Known(trade(Known(false))),
            Known(trade(Conflict)),
            Known(trade(NotApplicable)),
        ] {
            let mut first = ReferenceItemSemantics::default();
            let mut second = ReferenceItemSemantics {
                trade_restrictions: state,
                ..ReferenceItemSemantics::default()
            };
            let before = second.clone();
            assert!(
                apply_rows(
                    [("first", &mut first), ("second", &mut second)].into_iter(),
                    &packet(serde_json::json!([row("first"), row("second")])),
                    2
                )
                .is_err()
            );
            assert_eq!(first, ReferenceItemSemantics::default());
            assert_eq!(second, before);
        }
    }
    #[test]
    fn rejects_bad_rows_missing_items_and_duplicate_native_identity_atomically() {
        for rows in [
            serde_json::json!([row("item"), row("item")]),
            serde_json::json!([row("item"), row("missing")]),
            serde_json::json!([{"item_key":"item","marketable":false}]),
        ] {
            let expected = rows.as_array().expect("rows").len();
            let mut item = ReferenceItemSemantics::default();
            assert!(
                apply_rows(
                    std::iter::once(("item", &mut item)),
                    &packet(rows),
                    expected
                )
                .is_err()
            );
            assert_eq!(item, ReferenceItemSemantics::default());
        }
        let (mut first, mut second) = (
            ReferenceItemSemantics::default(),
            ReferenceItemSemantics::default(),
        );
        assert!(
            apply_rows(
                [("item", &mut first), ("item", &mut second)].into_iter(),
                &packet(serde_json::json!([row("item")])),
                1
            )
            .is_err()
        );
        assert_eq!(first, ReferenceItemSemantics::default());
        assert_eq!(second, ReferenceItemSemantics::default());
    }
}
