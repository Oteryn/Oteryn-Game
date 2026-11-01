//! E06: mint the client-only Crystal market Items (owner decision 2026-10-31).
//!
//! Each record is identity plus the facts the pinned 15.30 client appearance proves (name,
//! portability, stackability); it is not materializable. The Crystal market parity packet then
//! promotes its marketability.
use super::{
    ProjectReferenceRecord, ProjectV2Draft, item_identity::tibia_item_key, world_project_sha256,
};
use serde::Deserialize;
use std::collections::BTreeSet;

pub const ITEM_CLIENT_MARKET_MINT_PACKET: &[u8] = include_bytes!(
    "../../../../../docs/agents/evidence/OTV2-20261031-item-client-market-mint-v1.json"
);
// Regenerate with `lower_client_market_mint_packet.py`; the generator prints this digest.
pub const ITEM_CLIENT_MARKET_MINT_PACKET_SHA256: &str =
    "892e3ad7c40c4fd1ad0a4b29d1cb8aae5037d62e52fc633571356e7739b12681";
const RECORDS: usize = 68;

#[derive(Deserialize)]
struct Packet {
    schema: String,
    counts: Counts,
    records: Vec<Row>,
}
#[derive(Deserialize)]
struct Counts {
    records: usize,
}
#[derive(Deserialize)]
struct Row {
    source_item_id: u64,
    definition: ProjectReferenceRecord,
}

/// Appends the minted Items; every key must be new and every record an identity-only Item shape.
pub fn apply_item_client_market_mint_v1(draft: &mut ProjectV2Draft) -> Result<usize, String> {
    if world_project_sha256(ITEM_CLIENT_MARKET_MINT_PACKET) != ITEM_CLIENT_MARKET_MINT_PACKET_SHA256
    {
        return Err("client market mint packet digest drift".into());
    }
    let existing = draft
        .core
        .records
        .iter()
        .filter_map(|record| match record {
            ProjectReferenceRecord::Item { identity, .. } => Some(identity.key.clone()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let minted = mint_rows(ITEM_CLIENT_MARKET_MINT_PACKET, &existing, RECORDS)?;
    draft.core.records.extend(minted);
    Ok(RECORDS)
}

fn mint_rows(
    bytes: &[u8],
    existing: &BTreeSet<String>,
    expected: usize,
) -> Result<Vec<ProjectReferenceRecord>, String> {
    let packet: Packet = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if packet.schema != "OTERYN_ITEM_CLIENT_MARKET_MINT/v1"
        || packet.counts.records != expected
        || packet.records.len() != expected
    {
        return Err("client market mint packet shape/count drift".into());
    }
    let mut seen = BTreeSet::new();
    let mut records = Vec::with_capacity(expected);
    for row in packet.records {
        let key = tibia_item_key(row.source_item_id).ok_or("minted id has no Tibia key")?;
        match &row.definition {
            ProjectReferenceRecord::Item {
                identity,
                materializable: false,
                ..
            } if identity.family == "Item"
                && identity.key == key
                && identity.revision == "definition-r1" => {}
            _ => return Err(format!("minted record shape drift: {key}")),
        }
        if existing.contains(&key) || !seen.insert(key.clone()) {
            return Err(format!("minted Item key already exists: {key}"));
        }
        records.push(row.definition);
    }
    Ok(records)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    fn record(id: u64, materializable: bool) -> serde_json::Value {
        serde_json::json!({"source_item_id": id, "definition": {"kind": "Item",
            "identity": {"family": "Item", "key": format!("oteryn:item.tibia.i{id}"),
                "revision": "definition-r1"},
            "client_projection": "ClientSafe", "materializable": materializable,
            "stack_class": "NonStackable"}})
    }
    fn packet(rows: Vec<serde_json::Value>) -> Vec<u8> {
        serde_json::to_vec(
            &serde_json::json!({"schema": "OTERYN_ITEM_CLIENT_MARKET_MINT/v1",
            "counts": {"records": rows.len()}, "records": rows}),
        )
        .expect("packet")
    }
    #[test]
    fn mints_new_identity_only_items() {
        let minted = mint_rows(
            &packet(vec![record(42085, false), record(47899, false)]),
            &BTreeSet::new(),
            2,
        )
        .expect("minted");
        assert_eq!(minted.len(), 2);
    }
    #[test]
    fn rejects_existing_duplicate_materializable_and_count_drift() {
        let existing = BTreeSet::from(["oteryn:item.tibia.i42085".to_owned()]);
        assert!(mint_rows(&packet(vec![record(42085, false)]), &existing, 1).is_err());
        let none = BTreeSet::new();
        assert!(mint_rows(&packet(vec![record(1, false), record(1, false)]), &none, 2).is_err());
        assert!(mint_rows(&packet(vec![record(1, true)]), &none, 1).is_err());
        assert!(mint_rows(&packet(vec![record(1, false)]), &none, 2).is_err());
        let mut wrong = record(2, false);
        wrong["source_item_id"] = serde_json::json!(3);
        assert!(mint_rows(&packet(vec![wrong]), &none, 1).is_err());
    }
}
