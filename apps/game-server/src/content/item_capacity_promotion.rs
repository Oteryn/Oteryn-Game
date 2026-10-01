//! Bounded source-qualified capacity repair; no materialization or physical admission.
//! Wiki values require exact source binding and client container corroboration. Only the
//! separately qualified Adventurer Backpack correction may replace a known different value.

use super::{
    ProjectReferenceRecord, ProjectV2Draft, ReferenceItemContainer, ReferenceItemField,
    ReferenceItemSemantics, world_project_sha256,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

pub const ITEM_CAPACITY_PACKET: &[u8] = include_bytes!(
    "../../../../docs/agents/evidence/OTV2-20261001-item-capacity-promotion-v1.json"
);
pub const ITEM_CAPACITY_PACKET_SHA256: &str =
    "68ed4fd01906fab7c302708101c8abda8337ae8e05fe8543cfa80adcb17f4264";
const SCHEMA: &str = "OTERYN_ITEM_CAPACITY_PROMOTION/v1";

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
    capacity: u16,
    expected_known_capacity: Option<u16>,
    client_container_flag: bool,
}

/// Apply only the 18 pinned fields, preserving every other Item property.
pub fn apply_item_capacity_promotion_v1(draft: &mut ProjectV2Draft) -> Result<usize, String> {
    if world_project_sha256(ITEM_CAPACITY_PACKET) != ITEM_CAPACITY_PACKET_SHA256 {
        return Err("capacity packet digest mismatch".into());
    }
    let items = draft
        .core
        .records
        .iter_mut()
        .filter_map(|record| match record {
            ProjectReferenceRecord::Item {
                identity,
                semantics,
                ..
            } => Some((identity.key.as_str(), semantics)),
            _ => None,
        });
    let count = apply_rows(items, ITEM_CAPACITY_PACKET)?;
    if count != 18 {
        return Err("capacity repair count drift".into());
    }
    Ok(count)
}

fn capacity(semantics: &ReferenceItemSemantics) -> Result<Option<u16>, String> {
    match &semantics.container {
        ReferenceItemField::Known(container) => match container.capacity {
            ReferenceItemField::Known(value) => Ok(Some(value)),
            ReferenceItemField::Unknown => Ok(None),
            _ => Err("capacity has a blocked evidence state".into()),
        },
        ReferenceItemField::Unknown => Ok(None),
        _ => Err("container has a blocked evidence state".into()),
    }
}

fn apply_rows<'a>(
    items: impl Iterator<Item = (&'a str, &'a mut ReferenceItemSemantics)>,
    bytes: &[u8],
) -> Result<usize, String> {
    let packet: Packet = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if packet.schema != SCHEMA || packet.counts.promotions != packet.promotions.len() {
        return Err("capacity packet shape/counts".into());
    }
    let mut items: BTreeMap<_, _> = items.collect();
    let mut seen = BTreeSet::new();
    // Validate the entire packet before any mutation, including the known-value fence.
    for row in &packet.promotions {
        if !seen.insert(&row.item_key) || row.capacity == 0 || !row.client_container_flag {
            return Err("invalid or duplicate capacity row".into());
        }
        let semantics = items
            .get(row.item_key.as_str())
            .ok_or_else(|| format!("capacity row has no Item: {}", row.item_key))?;
        let old = capacity(semantics)?;
        let qualified = match row.expected_known_capacity {
            None => old.is_none() || old == Some(row.capacity),
            Some(expected) => old == Some(expected) || old == Some(row.capacity),
        };
        if !qualified {
            return Err(format!("capacity precondition drift: {}", row.item_key));
        }
    }
    for row in &packet.promotions {
        let semantics = items
            .get_mut(row.item_key.as_str())
            .ok_or_else(|| "validated capacity Item missing".to_owned())?;
        semantics.container = ReferenceItemField::Known(ReferenceItemContainer {
            capacity: ReferenceItemField::Known(row.capacity),
        });
    }
    Ok(packet.promotions.len())
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    fn packet(rows: serde_json::Value) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "schema": SCHEMA, "counts": {"promotions": rows.as_array().expect("rows").len()},
            "promotions": rows,
        }))
        .expect("packet")
    }
    fn row(expected: Option<u16>) -> serde_json::Value {
        serde_json::json!({"item_key": "item", "capacity": 22,
            "expected_known_capacity": expected, "client_container_flag": true})
    }
    #[test]
    fn bounded_unknown_and_known_correction_are_idempotent() {
        let mut item = ReferenceItemSemantics::default();
        let bytes = packet(serde_json::json!([row(None)]));
        for _ in 0..2 {
            assert_eq!(
                apply_rows(std::iter::once(("item", &mut item)), &bytes),
                Ok(1)
            );
        }
        item.container = ReferenceItemField::Known(ReferenceItemContainer {
            capacity: ReferenceItemField::Known(20),
        });
        assert!(apply_rows(std::iter::once(("item", &mut item)), &bytes).is_err());
        let correction = packet(serde_json::json!([row(Some(20))]));
        for _ in 0..2 {
            assert_eq!(
                apply_rows(std::iter::once(("item", &mut item)), &correction),
                Ok(1)
            );
        }
    }
    #[test]
    fn rejects_unknown_correction_missing_item_duplicates_and_no_partial_mutation() {
        let mut item = ReferenceItemSemantics::default();
        for rows in [
            serde_json::json!([row(Some(20))]),
            serde_json::json!([row(None), row(None)]),
            serde_json::json!([row(None), {"item_key":"missing", "capacity": 8,
                "expected_known_capacity":null, "client_container_flag":true}]),
        ] {
            let before = item.clone();
            assert!(apply_rows(std::iter::once(("item", &mut item)), &packet(rows)).is_err());
            assert_eq!(item, before);
        }
    }
}
