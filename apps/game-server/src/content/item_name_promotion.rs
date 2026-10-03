//! Only the closed139 proven imported generic display names may be corrected.
use super::{
    DefinitionIdentityDocument, ProjectReferenceRecord, ProjectV2Draft, ProjectV2Family,
    ProjectV2ItemAuthoring, ReferenceItemField, ReferenceItemSemantics, world_project_sha256,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
pub const ITEM_NAME_PACKET: &[u8] =
    include_bytes!("../../../../docs/agents/evidence/OTV2-20261002-item-name-promotion-v1.json");
pub const ITEM_NAME_PACKET_SHA256: &str =
    "fcc6dcc001bffa1febb2a240d7f4a87b44badab77d59a9372b92e7c3b8b3586a";
#[derive(Deserialize)]
struct Packet {
    schema: String,
    counts: Counts,
    promotions: Vec<Row>,
}
#[derive(Deserialize)]
struct Counts {
    fields: usize,
    items: usize,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Row {
    target: DefinitionIdentityDocument,
    previous_imported_name: String,
    name: String,
}
pub fn apply_item_name_promotion_v1(draft: &mut ProjectV2Draft) -> Result<usize, String> {
    if world_project_sha256(ITEM_NAME_PACKET) != ITEM_NAME_PACKET_SHA256 {
        return Err("name packet digest drift".into());
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
            } => Some((&*identity, semantics)),
            _ => None,
        });
    apply_rows(items, &draft.state.item_authoring, ITEM_NAME_PACKET, 139)
}
fn apply_rows<'a>(
    mut items: impl Iterator<
        Item = (
            &'a DefinitionIdentityDocument,
            &'a mut ReferenceItemSemantics,
        ),
    >,
    owners: &[ProjectV2ItemAuthoring],
    bytes: &[u8],
    count: usize,
) -> Result<usize, String> {
    let packet: Packet = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if packet.schema != "OTERYN_ITEM_IMPORTED_NAME_PROMOTION/v1"
        || packet.counts.fields != count
        || packet.counts.items != count
        || packet.promotions.len() != count
    {
        return Err("closed name scope drift".into());
    }
    let mut indexed = items.try_fold(BTreeMap::new(), |mut map, (id, semantics)| {
        if map.insert(id.key.as_str(), (id, semantics)).is_some() {
            return Err("duplicate native identity".to_owned());
        }
        Ok(map)
    })?;
    let mut seen = BTreeSet::new();
    for row in &packet.promotions {
        if !seen.insert(&row.target.key)
            || row.target.family != "Item"
            || row.target.revision != "definition-r1"
            || row.name.trim().is_empty()
            || !matches!(
                row.previous_imported_name.as_str(),
                "weapon of carving" | "event item"
            )
        {
            return Err("closed imported literal/full target drift".into());
        }
        let (id, semantics) = indexed
            .get(row.target.key.as_str())
            .ok_or("name Item missing")?;
        if *id != &row.target {
            return Err("name current full identity drift".into());
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
            return Err("name authoring identity/duplicate/GameOwned guard".into());
        }
        if !matches!(&semantics.presentation, ReferenceItemField::Known(p) if matches!(&p.name, ReferenceItemField::Known(value) if value == &row.previous_imported_name || value == &row.name))
        {
            return Err("closed imported Known-name exception".into());
        }
    }
    // All139 current native and GameOwned-owner preconditions passed before any write.
    let mut changed = 0;
    for row in packet.promotions {
        let (_, semantics) = indexed
            .get_mut(row.target.key.as_str())
            .ok_or("prepared name Item missing")?;
        if let ReferenceItemField::Known(p) = &mut semantics.presentation {
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
    fn target(key: &str) -> DefinitionIdentityDocument {
        DefinitionIdentityDocument {
            family: "Item".into(),
            key: key.into(),
            revision: "definition-r1".into(),
        }
    }
    fn original() -> ReferenceItemSemantics {
        serde_json::from_value(serde_json::json!({"presentation":{"state":"KNOWN","value":{"name":{"state":"KNOWN","value":"weapon of carving"},"description":{"state":"CONFLICT"}}}})).expect("semantics")
    }
    fn packet(keys: &[&str]) -> Vec<u8> {
        let rows:Vec<_> = keys.iter().map(|key|serde_json::json!({"target":target(key),"previous_imported_name":"weapon of carving","name":"bow of mayhem"})).collect();
        serde_json::to_vec(&serde_json::json!({"schema":"OTERYN_ITEM_IMPORTED_NAME_PROMOTION/v1","counts":{"fields":rows.len(),"items":rows.len()},"promotions":rows})).expect("packet")
    }
    #[test]
    fn repeated_official_names_are_numeric_targets_and_idempotent() {
        let (a, b) = (target("normal"), target("charged"));
        let (mut first, mut second) = (original(), original());
        let bytes = packet(&["normal", "charged"]);
        assert_eq!(
            apply_rows(
                [(&a, &mut first), (&b, &mut second)].into_iter(),
                &[],
                &bytes,
                2
            )
            .expect("names"),
            2
        );
        let mut expected = original();
        if let Known(p) = &mut expected.presentation {
            p.name = Known("bow of mayhem".into());
        }
        assert_eq!(first, expected);
        assert_eq!(second, expected);
        assert_eq!(
            apply_rows(
                [(&a, &mut first), (&b, &mut second)].into_iter(),
                &[],
                &bytes,
                2
            )
            .expect("repeat"),
            0
        );
    }
    #[test]
    fn exact_event_previous_literal_cannot_override_another_generic_name() {
        let id = target("event");
        let bytes = String::from_utf8(packet(&["event"]))
            .expect("utf8")
            .replace("weapon of carving", "event item");
        let mut item = original();
        let before = item.clone();
        assert!(apply_rows(std::iter::once((&id, &mut item)), &[], bytes.as_bytes(), 1).is_err());
        assert_eq!(item, before);
        if let Known(p) = &mut item.presentation {
            p.name = Known("event item".into());
        }
        assert_eq!(
            apply_rows(std::iter::once((&id, &mut item)), &[], bytes.as_bytes(), 1).expect("event"),
            1
        );
        assert_eq!(
            apply_rows(std::iter::once((&id, &mut item)), &[], bytes.as_bytes(), 1)
                .expect("repeat"),
            0
        );
    }
    #[test]
    fn unknown_blocked_and_unrelated_known_names_never_overwrite() {
        let id = target("item");
        for name in [
            Unknown,
            Conflict,
            NotApplicable,
            Known("GameOwned label".into()),
        ] {
            let mut item = original();
            if let Known(p) = &mut item.presentation {
                p.name = name;
            }
            let before = item.clone();
            assert!(
                apply_rows(
                    std::iter::once((&id, &mut item)),
                    &[],
                    &packet(&["item"]),
                    1
                )
                .is_err()
            );
            assert_eq!(item, before);
        }
        for group in [Unknown, Conflict, NotApplicable] {
            let mut item = ReferenceItemSemantics {
                presentation: group,
                ..ReferenceItemSemantics::default()
            };
            let before = item.clone();
            assert!(
                apply_rows(
                    std::iter::once((&id, &mut item)),
                    &[],
                    &packet(&["item"]),
                    1
                )
                .is_err()
            );
            assert_eq!(item, before);
        }
    }
    #[test]
    fn later_missing_owner_override_duplicate_and_revision_fail_atomically() {
        let mut id = target("item");
        let mut item = original();
        let before = item.clone();
        for keys in [vec!["item", "missing"], vec!["item", "item"]] {
            assert!(apply_rows(std::iter::once((&id, &mut item)), &[], &packet(&keys), 2).is_err());
            assert_eq!(item, before);
        }
        let owner:ProjectV2ItemAuthoring=serde_json::from_value(serde_json::json!({"item":id,"presentation":{"family":"Presentation","key":"owned","revision":"r1"}})).expect("owner");
        assert!(
            apply_rows(
                std::iter::once((&id, &mut item)),
                &[owner],
                &packet(&["item"]),
                1
            )
            .is_err()
        );
        assert_eq!(item, before);
        id.revision = "wrong".into();
        assert!(
            apply_rows(
                std::iter::once((&id, &mut item)),
                &[],
                &packet(&["item"]),
                1
            )
            .is_err()
        );
        assert_eq!(item, before);
    }
}
