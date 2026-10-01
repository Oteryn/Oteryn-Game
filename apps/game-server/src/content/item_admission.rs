//! Item admission v1 (STARTER-CONTENT-1): make named Items materializable.
//!
//! STARTER-BACKPACK-0 §4 needs the main backpack `oteryn:item.tibia.i2854` to be materializable,
//! non-stackable and equippable in the container slot. The pinned packet
//! `docs/agents/evidence/OTV2-20261001-starter-backpack-item-admission.json` records those facts
//! with their sources (TibiaWiki, then the pinned Crystal and Canary `items.xml`). The
//! materializer applies it once, after [`super::item_stats_promotion`].
//!
//! Every admission is strict: the Item must exist, still be non-materializable with an unknown
//! stack class and unknown equipment, and already carry the expected container capacity. The
//! result is the D114 main-backpack shape: one complete container-slot equipment pattern.

use super::{
    ItemStackDocument, ProjectReferenceRecord, ProjectV2Draft, ReferenceEquipmentPattern,
    ReferenceEquipmentSlot, ReferenceItemEquipment, ReferenceItemField, ReferenceItemSemantics,
    ReferenceItemStack, world_project_sha256,
};
use serde::Deserialize;
use std::collections::BTreeSet;

/// The pinned packet bytes; any change is a new candidate with a new digest.
pub const ITEM_ADMISSION_V1_PACKET: &[u8] = include_bytes!(
    "../../../../docs/agents/evidence/OTV2-20261001-starter-backpack-item-admission.json"
);
pub const ITEM_ADMISSION_V1_PACKET_SHA256: &str =
    "5d6bdc500daa4f09590765273db32f22a9c9822b7fa7eb52d19de38e64e6270a";
pub const ITEM_ADMISSION_V1_ITEM_COUNT: usize = 1;
const SCHEMA: &str = "OTERYN_ITEM_ADMISSION/v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemAdmissionError {
    Digest,
    Decode(String),
    Item {
        item_key: String,
        reason: &'static str,
    },
    Counts,
}

impl std::fmt::Display for ItemAdmissionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Digest => formatter.write_str("item admission packet digest mismatch"),
            Self::Decode(error) => write!(formatter, "item admission packet: {error}"),
            Self::Item { item_key, reason } => {
                write!(formatter, "item admission {item_key}: {reason}")
            }
            Self::Counts => formatter.write_str("item admission counts drifted"),
        }
    }
}

impl std::error::Error for ItemAdmissionError {}

#[derive(Deserialize)]
struct Packet {
    schema: String,
    admissions: Vec<Admission>,
}

#[derive(Deserialize)]
struct Admission {
    item_key: String,
    materializable: bool,
    stack_class: StackClass,
    equipment_primary_slot: Slot,
    expected_container_capacity: u16,
}

#[derive(Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum StackClass {
    NonStackable,
}

#[derive(Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum Slot {
    Container,
}

/// Apply the pinned packet to `draft`; returns the number of admitted Items.
pub fn apply_item_admission_v1(draft: &mut ProjectV2Draft) -> Result<usize, ItemAdmissionError> {
    if world_project_sha256(ITEM_ADMISSION_V1_PACKET) != ITEM_ADMISSION_V1_PACKET_SHA256 {
        return Err(ItemAdmissionError::Digest);
    }
    let items = draft
        .core
        .records
        .iter_mut()
        .filter_map(|record| match record {
            ProjectReferenceRecord::Item {
                identity,
                materializable,
                stack_class,
                semantics,
                ..
            } => Some((
                identity.key.as_str(),
                materializable,
                stack_class,
                semantics,
            )),
            _ => None,
        });
    let admitted = apply_admissions(items, ITEM_ADMISSION_V1_PACKET)?;
    if admitted != ITEM_ADMISSION_V1_ITEM_COUNT {
        return Err(ItemAdmissionError::Counts);
    }
    Ok(admitted)
}

fn apply_admissions<'a>(
    items: impl Iterator<
        Item = (
            &'a str,
            &'a mut bool,
            &'a mut ItemStackDocument,
            &'a mut ReferenceItemSemantics,
        ),
    >,
    bytes: &[u8],
) -> Result<usize, ItemAdmissionError> {
    let packet: Packet = serde_json::from_slice(bytes)
        .map_err(|error| ItemAdmissionError::Decode(error.to_string()))?;
    if packet.schema != SCHEMA {
        return Err(ItemAdmissionError::Decode("schema".to_owned()));
    }
    let mut pending = BTreeSet::new();
    for admission in &packet.admissions {
        if !admission.materializable {
            return Err(item_error(
                &admission.item_key,
                "admission must be materializable",
            ));
        }
        if !pending.insert(admission.item_key.as_str()) {
            return Err(item_error(&admission.item_key, "duplicate admission"));
        }
    }
    let mut admitted = 0;
    for (key, materializable, stack_class, semantics) in items {
        if !pending.remove(key) {
            continue;
        }
        let Some(admission) = packet.admissions.iter().find(|row| row.item_key == key) else {
            continue;
        };
        admit(admission, materializable, stack_class, semantics)?;
        admitted += 1;
    }
    if let Some(item_key) = pending.into_iter().next() {
        return Err(item_error(item_key, "no Item record with this key"));
    }
    Ok(admitted)
}

fn admit(
    admission: &Admission,
    materializable: &mut bool,
    stack_class: &mut ItemStackDocument,
    semantics: &mut ReferenceItemSemantics,
) -> Result<(), ItemAdmissionError> {
    use ReferenceItemField::{Known, Unknown};
    let key = admission.item_key.as_str();
    if *materializable
        || *stack_class != ItemStackDocument::Unknown
        || !matches!(semantics.stack, Unknown)
        || !matches!(semantics.equipment, Unknown)
    {
        return Err(item_error(key, "baseline is not an unadmitted Item"));
    }
    let capacity = match &semantics.container {
        Known(container) => &container.capacity,
        _ => &Unknown,
    };
    if !matches!(capacity, Known(value) if *value == admission.expected_container_capacity) {
        return Err(item_error(
            key,
            "container capacity differs from the sources",
        ));
    }
    let StackClass::NonStackable = admission.stack_class;
    let Slot::Container = admission.equipment_primary_slot;
    *materializable = true;
    *stack_class = ItemStackDocument::NonStackable;
    semantics.stack = Known(ReferenceItemStack {
        stackable: Known(false),
        stack_max: Unknown,
    });
    semantics.equipment = Known(ReferenceItemEquipment {
        patterns: Known(vec![ReferenceEquipmentPattern {
            pattern_id: 1,
            primary_slot: Known(ReferenceEquipmentSlot::Container),
            additional_reserved_slots: Unknown,
            mutually_exclusive_groups: Unknown,
            vocations: Unknown,
            level: Unknown,
            compatibility_rule: Unknown,
        }]),
    });
    Ok(())
}

fn item_error(item_key: &str, reason: &'static str) -> ItemAdmissionError {
    ItemAdmissionError::Item {
        item_key: item_key.to_owned(),
        reason,
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::content::ReferenceItemContainer;

    const KEY: &str = "oteryn:item.tibia.i2854";

    fn packet(rows: &str) -> Vec<u8> {
        format!(r#"{{"schema":"{SCHEMA}","admissions":[{rows}]}}"#).into_bytes()
    }

    fn row(key: &str, materializable: bool, capacity: u16) -> String {
        format!(
            r#"{{"item_key":"{key}","materializable":{materializable},"stack_class":"NON_STACKABLE","equipment_primary_slot":"CONTAINER","expected_container_capacity":{capacity}}}"#
        )
    }

    fn backpack() -> ReferenceItemSemantics {
        ReferenceItemSemantics {
            container: ReferenceItemField::Known(ReferenceItemContainer {
                capacity: ReferenceItemField::Known(20),
            }),
            ..ReferenceItemSemantics::default()
        }
    }

    fn apply(
        bytes: &[u8],
        materializable: &mut bool,
        stack_class: &mut ItemStackDocument,
        semantics: &mut ReferenceItemSemantics,
    ) -> Result<usize, ItemAdmissionError> {
        apply_admissions(
            std::iter::once((KEY, materializable, stack_class, semantics)),
            bytes,
        )
    }

    #[test]
    fn admits_the_main_backpack_shape() {
        let (mut materializable, mut stack_class, mut semantics) =
            (false, ItemStackDocument::Unknown, backpack());
        let admitted = apply(
            &packet(&row(KEY, true, 20)),
            &mut materializable,
            &mut stack_class,
            &mut semantics,
        )
        .expect("admits");
        assert_eq!(admitted, 1);
        assert!(materializable);
        assert_eq!(stack_class, ItemStackDocument::NonStackable);
        let ReferenceItemField::Known(equipment) = &semantics.equipment else {
            panic!("equipment");
        };
        let ReferenceItemField::Known(patterns) = &equipment.patterns else {
            panic!("patterns");
        };
        assert_eq!(patterns.len(), 1);
        assert_eq!(
            patterns[0].primary_slot,
            ReferenceItemField::Known(ReferenceEquipmentSlot::Container)
        );
    }

    #[test]
    fn rejects_bad_admissions() {
        let good = row(KEY, true, 20);
        let cases = [
            packet(&format!("{good},{good}")),
            packet(&row(KEY, false, 20)),
            packet(&row(KEY, true, 8)),
            packet(&row(concat!("oteryn:item.tibia.", "i1"), true, 20)),
            packet(&good.replace("CONTAINER", "HEAD")),
            packet(&good.replace("NON_STACKABLE", "STACK_CAPABLE")),
            format!(r#"{{"schema":"OTERYN_ITEM_ADMISSION/v0","admissions":[{good}]}}"#)
                .into_bytes(),
        ];
        for bytes in cases {
            let (mut materializable, mut stack_class, mut semantics) =
                (false, ItemStackDocument::Unknown, backpack());
            assert!(
                apply(
                    &bytes,
                    &mut materializable,
                    &mut stack_class,
                    &mut semantics
                )
                .is_err(),
                "{}",
                String::from_utf8_lossy(&bytes)
            );
        }
        // An already admitted Item is not admitted twice.
        let (mut materializable, mut stack_class, mut semantics) =
            (true, ItemStackDocument::NonStackable, backpack());
        assert!(
            apply(
                &packet(&good),
                &mut materializable,
                &mut stack_class,
                &mut semantics
            )
            .is_err()
        );
    }

    #[test]
    fn pinned_packet_decodes() {
        assert_eq!(
            world_project_sha256(ITEM_ADMISSION_V1_PACKET),
            ITEM_ADMISSION_V1_PACKET_SHA256
        );
        let packet: Packet =
            serde_json::from_slice(ITEM_ADMISSION_V1_PACKET).expect("pinned packet decodes");
        assert_eq!(packet.schema, SCHEMA);
        assert_eq!(packet.admissions.len(), ITEM_ADMISSION_V1_ITEM_COUNT);
    }
}
