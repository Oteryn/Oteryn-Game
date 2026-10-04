//! Only closed114 own in-game flavor strings; preserve every admission and sibling.
use super::{
    DefinitionIdentityDocument, ItemStackDocument, ProjectReferenceRecord, ProjectV2Draft,
    ProjectV2Family, ProjectV2ItemAuthoring, ProjectionDocument,
    REFERENCE_ITEM_MAX_DESCRIPTION_BYTES, ReferenceItemField, world_project_sha256,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

pub const ITEM_DESCRIPTION_PACKET: &[u8] = include_bytes!(
    "../../../../docs/agents/evidence/OTV2-20261002-item-description-promotion-v1.json"
);
pub const ITEM_DESCRIPTION_PACKET_SHA256: &str =
    "38c725236fa80b5b0fffdc7b66ddb18b9753b03df468393dfa12f753cee028fd";
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
    name: String,
    description: String,
}
pub fn apply_item_description_promotion_v1(draft: &mut ProjectV2Draft) -> Result<usize, String> {
    if world_project_sha256(ITEM_DESCRIPTION_PACKET) != ITEM_DESCRIPTION_PACKET_SHA256 {
        return Err("description packet digest drift".into());
    }
    apply_rows(
        &mut draft.core.records,
        &draft.state.item_authoring,
        ITEM_DESCRIPTION_PACKET,
        114,
    )
}
pub(super) fn apply_rows(
    records: &mut [ProjectReferenceRecord],
    owners: &[ProjectV2ItemAuthoring],
    bytes: &[u8],
    count: usize,
) -> Result<usize, String> {
    let packet: Packet = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if packet.schema != "OTERYN_ITEM_DESCRIPTION_PROMOTION/v1"
        || packet.counts.items != count
        || packet.counts.fields != count
        || packet.promotions.len() != count
    {
        return Err("closed description scope drift".into());
    }
    let mut indexed = BTreeMap::new();
    for (index, record) in records.iter().enumerate() {
        if let ProjectReferenceRecord::Item { identity, .. } = record
            && indexed.insert(identity.key.clone(), index).is_some()
        {
            return Err("duplicate native Item identity".into());
        }
    }
    let mut seen = BTreeSet::new();
    for row in &packet.promotions {
        if !seen.insert(&row.target.key)
            || row.target.family != "Item"
            || row.target.revision != "definition-r1"
            || row.headers.kind != "Item"
            || row.description.is_empty()
            || row.description.trim() != row.description
            || row.description.len() > REFERENCE_ITEM_MAX_DESCRIPTION_BYTES
            || row.description.chars().any(char::is_control)
            || ["{{", "}}", "[[", "]]", "<", ">", "&", "''"]
                .iter()
                .any(|s| row.description.contains(s))
        {
            return Err("description identity/literal/byte limit drift".into());
        }
        let index = *indexed
            .get(&row.target.key)
            .ok_or("description Item missing")?;
        let ProjectReferenceRecord::Item {
            identity,
            client_projection,
            materializable,
            stack_class,
            semantics,
        } = &records[index]
        else {
            return Err("description non-Item target".into());
        };
        if identity != &row.target
            || *client_projection != row.headers.client_projection
            || *materializable != row.headers.materializable
            || *stack_class != row.headers.stack_class
        {
            return Err("description full identity/four-header drift".into());
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
            return Err("description authoring identity/duplicate/GameOwned conflict".into());
        }
        if !matches!(&semantics.presentation, ReferenceItemField::Known(p)
            if p.name == ReferenceItemField::Known(row.name.clone())
            && (matches!(&p.description, ReferenceItemField::Unknown) || matches!(&p.description, ReferenceItemField::Known(value) if value == &row.description)))
        {
            return Err("description current Known-name/unknown-or-same leaf guard".into());
        }
    }
    // Every target passed before the first mutation; no header or sibling is assigned.
    let mut changed = 0;
    for row in packet.promotions {
        let index = *indexed
            .get(&row.target.key)
            .ok_or("prepared description missing")?;
        if let ProjectReferenceRecord::Item { semantics, .. } = &mut records[index]
            && let ReferenceItemField::Known(p) = &mut semantics.presentation
        {
            changed +=
                usize::from(p.description != ReferenceItemField::Known(row.description.clone()));
            p.description = ReferenceItemField::Known(row.description);
        }
    }
    Ok(changed)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use ReferenceItemField::{Conflict, Known, NotApplicable, Unknown};
    fn item(id: u32, materializable: bool) -> ProjectReferenceRecord {
        serde_json::from_value(serde_json::json!({"kind":"Item","identity":{"family":"Item","key":format!("oteryn:item.tibia.i{id}"),"revision":"definition-r1"},"client_projection":"ClientSafe","materializable":materializable,"stack_class":"Unknown","semantics":{"presentation":{"state":"KNOWN","value":{"name":{"state":"KNOWN","value":"special flask"},"description":{"state":"UNKNOWN"}}},"physical":{"state":"KNOWN","value":{"weight":{"state":"KNOWN","value":123},"pickupable":{"state":"KNOWN","value":true},"movable":{"state":"CONFLICT"}}}}})).expect("Item")
    }
    fn packet(records: &[ProjectReferenceRecord]) -> Vec<u8> {
        let rows: Vec<_> = records.iter().map(|r| {
            let ProjectReferenceRecord::Item { identity, client_projection, materializable, stack_class, .. } = r else { panic!("Item") };
            serde_json::json!({"target":identity,"headers":{"kind":"Item","client_projection":client_projection,"materializable":materializable,"stack_class":stack_class},"name":"special flask","description":"Literal flavor."})
        }).collect();
        serde_json::to_vec(&serde_json::json!({"schema":"OTERYN_ITEM_DESCRIPTION_PROMOTION/v1","counts":{"items":rows.len(),"fields":rows.len()},"promotions":rows})).expect("packet")
    }
    fn description(r: &mut ProjectReferenceRecord, value: ReferenceItemField<String>) {
        if let ProjectReferenceRecord::Item { semantics, .. } = r
            && let Known(p) = &mut semantics.presentation
        {
            p.description = value;
        }
    }
    #[test]
    fn actual_setter_preserves_full_headers_siblings_and_outside_and_is_idempotent() {
        let mut records = vec![
            item(237, true),
            item(239, true),
            item(107, false),
            item(999, false),
        ];
        let bytes = packet(&records[..3]);
        let mut expected = records.clone();
        for r in &mut expected[..3] {
            description(r, Known("Literal flavor.".into()));
        }
        assert_eq!(apply_rows(&mut records, &[], &bytes, 3).expect("apply"), 3);
        assert_eq!(records, expected);
        assert_eq!(apply_rows(&mut records, &[], &bytes, 3).expect("repeat"), 0);
        assert_eq!(records, expected);
    }
    #[test]
    fn late_blocked_or_conflicting_leaf_is_atomic() {
        let initial = vec![item(107, false), item(237, true)];
        let bytes = packet(&initial);
        for value in [Conflict, NotApplicable, Known("GameOwned text".into())] {
            let mut records = initial.clone();
            description(&mut records[1], value);
            let before = records.clone();
            assert!(apply_rows(&mut records, &[], &bytes, 2).is_err());
            assert_eq!(records, before);
        }
    }
    #[test]
    fn name_header_identity_group_and_duplicate_mutations_are_atomic() {
        let initial = vec![item(107, false), item(237, true)];
        let bytes = packet(&initial);
        for operator in 0..6 {
            let mut records = initial.clone();
            if let ProjectReferenceRecord::Item {
                identity,
                materializable,
                semantics,
                ..
            } = &mut records[1]
            {
                match operator {
                    0 => identity.revision = "other".into(),
                    1 => *materializable = false,
                    2 => semantics.presentation = Unknown,
                    3 => {
                        if let Known(p) = &mut semantics.presentation {
                            p.name = Known("Other name".into());
                        }
                    }
                    4 => {
                        if let Known(p) = &mut semantics.presentation {
                            p.name = Unknown;
                        }
                    }
                    _ => identity.key = "oteryn:item.tibia.i107".into(),
                }
            }
            let before = records.clone();
            assert!(apply_rows(&mut records, &[], &bytes, 2).is_err());
            assert_eq!(records, before);
        }
    }
    #[test]
    fn utf8_byte_boundary_and_unsupported_markup_fail_closed() {
        let initial = vec![item(107, false)];
        for text in ["é".repeat(100), "é".repeat(101), "{{dynamic}}".into()] {
            let mut records = initial.clone();
            let before = records.clone();
            let mut raw: serde_json::Value =
                serde_json::from_slice(&packet(&records)).expect("json");
            raw["promotions"][0]["description"] = text.clone().into();
            let bytes = serde_json::to_vec(&raw).expect("bytes");
            if text.len() == 200 {
                assert_eq!(apply_rows(&mut records, &[], &bytes, 1).expect("200"), 1);
            } else {
                assert!(apply_rows(&mut records, &[], &bytes, 1).is_err());
                assert_eq!(records, before);
            }
        }
    }
}
