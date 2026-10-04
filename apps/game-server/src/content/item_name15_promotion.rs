//! Closed15 official numeric-name corrections; no family or sibling inference.
use super::{
    DefinitionIdentityDocument, ItemStackDocument, ProjectReferenceRecord, ProjectV2Draft,
    ProjectV2Family, ProjectV2ItemAuthoring, ProjectionDocument, REFERENCE_ITEM_MAX_NAME_BYTES,
    ReferenceItemField, world_project_sha256,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

pub const ITEM_NAME15_PACKET: &[u8] =
    include_bytes!("../../../../docs/agents/evidence/OTV2-20261002-item-name15-promotion-v1.json");
pub const ITEM_NAME15_PACKET_SHA256: &str =
    "e68470b2ec6bbe22600edb081f894215de694c1a852a4e60d2d74c729a0b09cc";
const IDS: [u32; 15] = [
    23577, 23583, 23589, 23596, 23605, 23609, 23619, 23624, 23638, 23641, 23644, 23656, 23659,
    23662, 23665,
];
#[derive(Deserialize)]
struct Packet {
    schema: String,
    counts: Counts,
    promotions: Vec<Row>,
}
#[derive(Deserialize)]
struct Counts {
    items: usize,
    fields: usize,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Headers {
    kind: String,
    client_projection: ProjectionDocument,
    materializable: bool,
    stack_class: ItemStackDocument,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Row {
    target: DefinitionIdentityDocument,
    headers: Headers,
    previous_imported_name: String,
    name: String,
}
pub fn apply_item_name15_promotion_v1(draft: &mut ProjectV2Draft) -> Result<usize, String> {
    if world_project_sha256(ITEM_NAME15_PACKET) != ITEM_NAME15_PACKET_SHA256 {
        return Err("name15 packet digest drift".into());
    }
    apply_rows(
        &mut draft.core.records,
        &draft.state.item_authoring,
        ITEM_NAME15_PACKET,
    )
}
fn apply_rows(
    records: &mut [ProjectReferenceRecord],
    owners: &[ProjectV2ItemAuthoring],
    bytes: &[u8],
) -> Result<usize, String> {
    let packet: Packet = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if packet.schema != "OTERYN_ITEM_IMPORTED_NAME15_PROMOTION/v1"
        || packet.counts.items != 15
        || packet.counts.fields != 15
        || packet.promotions.len() != 15
    {
        return Err("closed name15 cardinality/schema drift".into());
    }
    let expected: BTreeSet<_> = IDS
        .iter()
        .map(|id| format!("oteryn:item.tibia.i{id}"))
        .collect();
    let actual: BTreeSet<_> = packet
        .promotions
        .iter()
        .map(|r| r.target.key.clone())
        .collect();
    if actual != expected {
        return Err("closed name15 targets drift".into());
    }
    let mut indexed = BTreeMap::new();
    for (index, record) in records.iter().enumerate() {
        if let ProjectReferenceRecord::Item { identity, .. } = record
            && indexed.insert(identity.key.clone(), index).is_some()
        {
            return Err("duplicate native Item identity".into());
        }
    }
    for row in &packet.promotions {
        if row.target.family != "Item"
            || row.target.revision != "definition-r1"
            || row.headers.kind != "Item"
            || row.previous_imported_name != "weapon of mayhem"
            || row.name.trim().is_empty()
            || row.name.trim() != row.name
            || row.name.len() > REFERENCE_ITEM_MAX_NAME_BYTES
            || row.name.chars().any(char::is_control)
        {
            return Err("name15 identity/imported literal/name limit drift".into());
        }
        let index = *indexed
            .get(&row.target.key)
            .ok_or("name15 target missing")?;
        let ProjectReferenceRecord::Item {
            identity,
            client_projection,
            materializable,
            stack_class,
            semantics,
        } = &records[index]
        else {
            return Err("name15 nonItem target".into());
        };
        if identity != &row.target
            || *client_projection != row.headers.client_projection
            || *materializable != row.headers.materializable
            || *stack_class != row.headers.stack_class
        {
            return Err("name15 full identity/four-header drift".into());
        }
        let matching: Vec<_> = owners
            .iter()
            .filter(|o| o.item.key == row.target.key)
            .collect();
        if matching.len() > 1
            || matching.iter().any(|o| {
                o.item.family != ProjectV2Family::Item
                    || o.item.revision != row.target.revision
                    || o.presentation.is_some()
            })
        {
            return Err("name15 GameOwned presentation/duplicate owner conflict".into());
        }
        if !matches!(&semantics.presentation, ReferenceItemField::Known(p) if matches!(&p.name, ReferenceItemField::Known(value) if value == &row.previous_imported_name || value == &row.name))
        {
            return Err("name15 only proven imported or identical Known name allowed".into());
        }
    }
    // All15 targets and owners passed before any name write; no sibling/header is assigned.
    let mut changed = 0;
    for row in packet.promotions {
        let index = *indexed
            .get(&row.target.key)
            .ok_or("prepared name15 target missing")?;
        if let ProjectReferenceRecord::Item { semantics, .. } = &mut records[index]
            && let ReferenceItemField::Known(p) = &mut semantics.presentation
        {
            changed += usize::from(p.name != ReferenceItemField::Known(row.name.clone()));
            p.name = ReferenceItemField::Known(row.name);
        }
    }
    Ok(changed)
}
#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use ReferenceItemField::{Conflict, Known, NotApplicable, Unknown};
    fn records() -> Vec<ProjectReferenceRecord> {
        let packet: serde_json::Value =
            serde_json::from_slice(ITEM_NAME15_PACKET).expect("sealed packet");
        let mut rows: Vec<ProjectReferenceRecord> = packet["promotions"].as_array().expect("rows").iter().map(|r| serde_json::from_value(serde_json::json!({"kind":"Item","identity":r["target"],"client_projection":r["headers"]["client_projection"],"materializable":r["headers"]["materializable"],"stack_class":r["headers"]["stack_class"],"semantics":{"presentation":{"state":"KNOWN","value":{"name":{"state":"KNOWN","value":"weapon of mayhem"},"description":{"state":"CONFLICT"}}},"stack":{"state":"KNOWN","value":{"stackable":{"state":"KNOWN","value":false},"stack_max":{"state":"UNKNOWN"}}}}})).expect("real target shape")).collect();
        let mut outside = rows[0].clone();
        if let ProjectReferenceRecord::Item { identity, .. } = &mut outside {
            identity.key = "oteryn:item.tibia.i999".into();
        }
        rows.push(outside);
        rows
    }
    fn name(record: &mut ProjectReferenceRecord, value: ReferenceItemField<String>) {
        if let ProjectReferenceRecord::Item { semantics, .. } = record
            && let Known(p) = &mut semantics.presentation
        {
            p.name = value;
        }
    }
    #[test]
    fn production_setter_exact15_preserves_full_siblings_outside_and_repeated_names() {
        let mut records = records();
        let mut expected = records.clone();
        let packet: Packet = serde_json::from_slice(ITEM_NAME15_PACKET).expect("packet");
        for (record, row) in expected.iter_mut().zip(&packet.promotions) {
            name(record, Known(row.name.clone()));
        }
        assert_eq!(
            apply_rows(&mut records, &[], ITEM_NAME15_PACKET).expect("names"),
            15
        );
        assert_eq!(records, expected);
        assert_eq!(
            apply_rows(&mut records, &[], ITEM_NAME15_PACKET).expect("repeat"),
            0
        );
        assert_eq!(records, expected);
    }
    #[test]
    fn late_blocked_or_unrelated_generic_name_has_zero_partial_writes() {
        for value in [
            Unknown,
            Conflict,
            NotApplicable,
            Known("weapon of carving".into()),
            Known("GameOwned name".into()),
        ] {
            let mut records = records();
            name(&mut records[14], value);
            let before = records.clone();
            assert!(apply_rows(&mut records, &[], ITEM_NAME15_PACKET).is_err());
            assert_eq!(records, before);
        }
    }
    #[test]
    fn late_header_identity_missing_duplicate_or_game_owned_owner_is_atomic() {
        for operation in 0..7 {
            let mut records = records();
            let mut owners = Vec::new();
            if operation == 0 {
                if let ProjectReferenceRecord::Item { materializable, .. } = &mut records[14] {
                    *materializable = true;
                }
            } else if operation == 1 {
                if let ProjectReferenceRecord::Item { identity, .. } = &mut records[14] {
                    identity.revision = "definition-r2".into();
                }
            } else if operation == 2 {
                records.remove(14);
            } else if operation == 3 {
                records.push(records[14].clone());
            } else {
                let packet: Packet = serde_json::from_slice(ITEM_NAME15_PACKET).expect("packet");
                let id = &packet.promotions[14].target;
                let mut owner: ProjectV2ItemAuthoring =
                    serde_json::from_value(serde_json::json!({"item":id})).expect("owner");
                if operation == 4 {
                    owner.presentation = Some(serde_json::from_value(serde_json::json!({"family":"Presentation","key":"oteryn:presentation.gameowned","revision":"definition-r1"})).expect("presentation"));
                } else if operation == 5 {
                    owner.item.revision = "definition-r2".into();
                } else {
                    owners.push(owner.clone());
                }
                owners.push(owner);
            }
            let before = records.clone();
            assert!(apply_rows(&mut records, &owners, ITEM_NAME15_PACKET).is_err());
            assert_eq!(records, before);
        }
    }
    #[test]
    fn sealed_scope_literal_and_utf8_byte_limit_cannot_be_widened() {
        for operation in 0..4 {
            let mut packet: serde_json::Value =
                serde_json::from_slice(ITEM_NAME15_PACKET).expect("packet");
            if operation == 0 {
                packet["promotions"][14]["previous_imported_name"] = "weapon of carving".into();
            } else if operation == 1 {
                packet["promotions"][14]["target"]["key"] = "oteryn:item.tibia.i23578".into();
            } else if operation == 2 {
                packet["promotions"][14]["name"] = "é".repeat(24).into();
            } else {
                packet["promotions"][14]["headers"]["materializable"] = 0.into();
            }
            let bytes = serde_json::to_vec(&packet).expect("bytes");
            let mut records = records();
            let before = records.clone();
            assert!(apply_rows(&mut records, &[], &bytes).is_err());
            assert_eq!(records, before);
        }
    }
}
