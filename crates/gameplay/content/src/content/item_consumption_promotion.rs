//! Food and potion consumption semantics (ITEM-SEM-USE-2): group 18 `consumption` and, for
//! potions with a source requirement, group 17 `use_requirements`.
//!
//! The pinned facts packet `docs/agents/evidence/OTV2-20261004-item-consumption-facts-v1.json` is
//! produced by `tools/content-schema/item-authoring/lower_item_consumption_packet.py` from the
//! pinned TibiaWiki and Canary sources. Each row targets one materializable Item whose
//! `consumption` is still unknown; the per-row source record is evidence only and is not decoded
//! here. The step is strict and fail-closed: any drift leaves the draft untouched.
use super::{
    DefinitionIdentityDocument, ProjectReferenceRecord, ProjectV2Draft, ReferenceItemConsumption,
    ReferenceItemField, ReferenceItemSemantics, ReferenceItemUseRequirements, world_project_sha256,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

pub const ITEM_CONSUMPTION_PACKET: &[u8] = include_bytes!(
    "../../../../../docs/agents/evidence/OTV2-20261004-item-consumption-facts-v1.json"
);
pub const ITEM_CONSUMPTION_PACKET_SHA256: &str =
    "d13625eadd669ca491964e4a0cd1a76ec0e92999c613a55c9b8964c01687e5e6";
pub const ITEM_CONSUMPTION_FOOD_COUNT: usize = 137;
pub const ITEM_CONSUMPTION_POTION_COUNT: usize = 12;
const SCHEMA: &str = "OTERYN_ITEM_CONSUMPTION_FACTS/v1";

#[derive(Deserialize)]
struct Packet {
    schema: String,
    rows: Vec<Row>,
}

#[derive(Deserialize)]
struct Row {
    target: DefinitionIdentityDocument,
    consumption: ReferenceItemConsumption,
    use_requirements: Option<ReferenceItemUseRequirements>,
}

/// Apply the pinned packet; returns `(food, potions)` lowered.
pub fn apply_item_consumption_promotion_v1(
    draft: &mut ProjectV2Draft,
) -> Result<(usize, usize), String> {
    if world_project_sha256(ITEM_CONSUMPTION_PACKET) != ITEM_CONSUMPTION_PACKET_SHA256 {
        return Err("item consumption packet digest drift".into());
    }
    let items = draft
        .core
        .records
        .iter_mut()
        .filter_map(|record| match record {
            ProjectReferenceRecord::Item {
                identity,
                materializable,
                semantics,
                ..
            } => Some((&*identity, *materializable, semantics)),
            _ => None,
        });
    apply_rows(items, ITEM_CONSUMPTION_PACKET)
}

fn apply_rows<'a>(
    items: impl Iterator<
        Item = (
            &'a DefinitionIdentityDocument,
            bool,
            &'a mut ReferenceItemSemantics,
        ),
    >,
    bytes: &[u8],
) -> Result<(usize, usize), String> {
    let packet: Packet = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    if packet.schema != SCHEMA {
        return Err("item consumption packet schema".into());
    }
    let mut rows = BTreeMap::new();
    for row in &packet.rows {
        if row.target.family != "Item" || row.target.revision != "definition-r1" {
            return Err(format!(
                "{}: target is not an Item definition-r1",
                row.target.key
            ));
        }
        if rows.insert(row.target.key.as_str(), row).is_some() {
            return Err(format!("{}: duplicate consumption row", row.target.key));
        }
        let is_potion = matches!(row.consumption, ReferenceItemConsumption::Potion(_));
        if !is_potion && row.use_requirements.is_some() {
            return Err(format!("{}: food carries use requirements", row.target.key));
        }
    }
    // Validate every row before mutating: a drift leaves the draft untouched.
    let mut targets = Vec::new();
    let mut seen = BTreeSet::new();
    for (identity, materializable, semantics) in items {
        let Some(row) = rows.get(identity.key.as_str()) else {
            continue;
        };
        if *identity != row.target {
            return Err(format!("{}: full identity drift", identity.key));
        }
        if !materializable
            || !matches!(semantics.consumption, ReferenceItemField::Unknown)
            || !matches!(semantics.use_requirements, ReferenceItemField::Unknown)
        {
            return Err(format!(
                "{}: not a materializable Item with unknown consumption",
                identity.key
            ));
        }
        seen.insert(identity.key.as_str());
        targets.push((*row, semantics));
    }
    if let Some(missing) = rows.keys().find(|key| !seen.contains(*key)) {
        return Err(format!("{missing}: no Item record with this key"));
    }
    let (mut food, mut potions) = (0, 0);
    for (row, semantics) in targets {
        semantics.consumption = ReferenceItemField::Known(row.consumption.clone());
        if let Some(requirements) = &row.use_requirements {
            semantics.use_requirements = ReferenceItemField::Known(requirements.clone());
        }
        match row.consumption {
            ReferenceItemConsumption::Food(_) => food += 1,
            ReferenceItemConsumption::Potion(_) => potions += 1,
        }
    }
    Ok((food, potions))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn pinned_packet_digest_and_counts() {
        assert_eq!(
            world_project_sha256(ITEM_CONSUMPTION_PACKET),
            ITEM_CONSUMPTION_PACKET_SHA256
        );
        let packet: Packet =
            serde_json::from_slice(ITEM_CONSUMPTION_PACKET).expect("pinned packet decodes");
        let potions = packet
            .rows
            .iter()
            .filter(|row| matches!(row.consumption, ReferenceItemConsumption::Potion(_)))
            .count();
        assert_eq!(potions, ITEM_CONSUMPTION_POTION_COUNT);
        assert_eq!(packet.rows.len() - potions, ITEM_CONSUMPTION_FOOD_COUNT);
    }

    fn item(key: &str) -> DefinitionIdentityDocument {
        DefinitionIdentityDocument {
            family: "Item".to_owned(),
            key: key.to_owned(),
            revision: "definition-r1".to_owned(),
        }
    }

    fn one_row(key: &str) -> Vec<u8> {
        format!(
            r#"{{"schema":"{SCHEMA}","rows":[{{"target":{{"family":"Item","key":"{key}","revision":"definition-r1"}},"consumption":{{"kind":"FOOD","value":{{"regeneration_seconds":360}}}}}}]}}"#
        )
        .into_bytes()
    }

    #[test]
    fn applies_food_and_fails_closed() {
        let identity = item("oteryn:item.tibia.i3582");
        let mut semantics = ReferenceItemSemantics::default();
        let counts = apply_rows(
            std::iter::once((&identity, true, &mut semantics)),
            &one_row("oteryn:item.tibia.i3582"),
        )
        .expect("applies");
        assert_eq!(counts, (1, 0));
        assert!(matches!(
            semantics.consumption,
            ReferenceItemField::Known(_)
        ));
        // A second run is refused: consumption is no longer unknown.
        assert!(
            apply_rows(
                std::iter::once((&identity, true, &mut semantics)),
                &one_row("oteryn:item.tibia.i3582")
            )
            .is_err()
        );
        // Not materializable, and a row with no record, are both refused.
        let mut fresh = ReferenceItemSemantics::default();
        assert!(
            apply_rows(
                std::iter::once((&identity, false, &mut fresh)),
                &one_row("oteryn:item.tibia.i3582")
            )
            .is_err()
        );
        assert!(apply_rows(std::iter::empty(), &one_row("oteryn:item.tibia.i3582")).is_err());
    }
}
