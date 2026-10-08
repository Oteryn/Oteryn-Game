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
//!
//! Item admission v2 (D3-7) is the decision-backed rat corpse `oteryn:item.tibia.i5964`
//! (D3 D137): non-stackable, a 16-entry container, a 60 s durable absolute deadline and no decay
//! successor. Its pinned packet `docs/agents/evidence/OTV2-20261004-d3-7-corpse-item-admission.json`
//! pins only the Canary row it diverges from. It runs right after v1, and fails closed unless the
//! record is still identity-only.

use super::{
    ItemStackDocument, ProjectReferenceRecord, ProjectV2Draft, ReferenceEquipmentPattern,
    ReferenceEquipmentSlot, ReferenceItemContainer, ReferenceItemEquipment, ReferenceItemField,
    ReferenceItemSemantics, ReferenceItemStack, ReferenceItemTemporal, ReferenceMilliseconds,
    ReferenceTemporalMode, world_project_sha256,
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

/// The pinned v2 packet bytes (D3-7); any change is a new candidate with a new digest.
pub const ITEM_ADMISSION_V2_PACKET: &[u8] = include_bytes!(
    "../../../../docs/agents/evidence/OTV2-20261004-d3-7-corpse-item-admission.json"
);
pub const ITEM_ADMISSION_V2_PACKET_SHA256: &str =
    "59ff17d440ede062ec1ca0287bbd72da3acde35ffe753f0e4022a6589ac64cfe";
pub const ITEM_ADMISSION_V2_ITEM_COUNT: usize = 1;
/// The pinned v2 cheese packet bytes (D3-8): the stack-capable food `oteryn:item.tibia.i3607`.
pub const ITEM_ADMISSION_V2_CHEESE_PACKET: &[u8] = include_bytes!(
    "../../../../docs/agents/evidence/OTV2-20261007-d3-8-cheese-item-admission.json"
);
pub const ITEM_ADMISSION_V2_CHEESE_PACKET_SHA256: &str =
    "6bcd22fe11ab5ba02a86af744e494af92630516b22efffa6a408987066e02e0d";
pub const ITEM_ADMISSION_V2_CHEESE_ITEM_COUNT: usize = 1;
const SCHEMA_V2: &str = "OTERYN_ITEM_ADMISSION/v2";

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
enum StackClassV2 {
    NonStackable,
    StackCapable,
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PacketV2 {
    schema: String,
    task: String,
    decision: String,
    policy: PolicyV2,
    admissions: Vec<AdmissionV2>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicyV2 {
    agreement: String,
    scope: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AdmissionV2 {
    item_key: String,
    materializable: bool,
    stack_class: StackClassV2,
    /// Present exactly for the container shape (the corpse).
    container_capacity: Option<u16>,
    equipment: Option<NotApplicable>,
    decay_target: Option<NotApplicable>,
    temporal_mode: Option<TemporalMode>,
    temporal_duration_ms: Option<u64>,
    temporal_stop_duration: Option<bool>,
    /// Present exactly for the stack-capable shape (the cheese).
    stack_max: Option<u16>,
    /// Canary is always pinned; the reference candidate is the only other source that decodes.
    #[allow(dead_code)]
    evidence: EvidenceV2,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EvidenceV2 {
    #[allow(dead_code)]
    canary: serde_json::Value,
    #[allow(dead_code)]
    reference_candidate: Option<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum NotApplicable {
    NotApplicable,
}

#[derive(Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum TemporalMode {
    DurableAbsoluteDeadline,
}

/// Apply the pinned D3-7 corpse packet to `draft`; returns the number of admitted Items.
pub fn apply_item_admission_v2(draft: &mut ProjectV2Draft) -> Result<usize, ItemAdmissionError> {
    apply_packet_v2(
        draft,
        ITEM_ADMISSION_V2_PACKET,
        ITEM_ADMISSION_V2_PACKET_SHA256,
        ITEM_ADMISSION_V2_ITEM_COUNT,
    )
}

/// Apply the pinned D3-8 cheese packet to `draft`; returns the number of admitted Items.
pub fn apply_item_admission_v2_cheese(
    draft: &mut ProjectV2Draft,
) -> Result<usize, ItemAdmissionError> {
    apply_packet_v2(
        draft,
        ITEM_ADMISSION_V2_CHEESE_PACKET,
        ITEM_ADMISSION_V2_CHEESE_PACKET_SHA256,
        ITEM_ADMISSION_V2_CHEESE_ITEM_COUNT,
    )
}

fn apply_packet_v2(
    draft: &mut ProjectV2Draft,
    packet: &[u8],
    sha256: &str,
    expected: usize,
) -> Result<usize, ItemAdmissionError> {
    if world_project_sha256(packet) != sha256 {
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
    let admitted = apply_admissions_v2(items, packet)?;
    if admitted != expected {
        return Err(ItemAdmissionError::Counts);
    }
    Ok(admitted)
}

fn apply_admissions_v2<'a>(
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
    let packet: PacketV2 = serde_json::from_slice(bytes)
        .map_err(|error| ItemAdmissionError::Decode(error.to_string()))?;
    if packet.schema != SCHEMA_V2
        || packet.task.is_empty()
        || packet.decision.is_empty()
        || !matches!(
            packet.policy.agreement.as_str(),
            "DECISION_SHAPE_WITH_PINNED_EVIDENCE" | "PINNED_EVIDENCE_NO_CONFLICT"
        )
        || packet.policy.scope.is_empty()
    {
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
        admit_v2(admission, materializable, stack_class, semantics)?;
        admitted += 1;
    }
    if let Some(item_key) = pending.into_iter().next() {
        return Err(item_error(item_key, "no Item record with this key"));
    }
    Ok(admitted)
}

fn admit_v2(
    admission: &AdmissionV2,
    materializable: &mut bool,
    stack_class: &mut ItemStackDocument,
    semantics: &mut ReferenceItemSemantics,
) -> Result<(), ItemAdmissionError> {
    use ReferenceItemField::{Known, NotApplicable as NotApplicableField, Unknown};
    let key = admission.item_key.as_str();
    if *materializable
        || *stack_class != ItemStackDocument::Unknown
        || !matches!(semantics.stack, Unknown)
        || !matches!(semantics.container, Unknown)
        || !matches!(semantics.equipment, Unknown)
        || !matches!(semantics.temporal, Unknown)
    {
        return Err(item_error(key, "baseline is not an identity-only Item"));
    }
    match (
        &admission.stack_class,
        admission.container_capacity,
        &admission.equipment,
        &admission.decay_target,
        &admission.temporal_mode,
        admission.temporal_duration_ms,
        admission.temporal_stop_duration,
        admission.stack_max,
    ) {
        (
            StackClassV2::NonStackable,
            Some(capacity),
            Some(NotApplicable::NotApplicable),
            Some(NotApplicable::NotApplicable),
            Some(TemporalMode::DurableAbsoluteDeadline),
            Some(duration_ms),
            Some(stop_duration),
            None,
        ) => {
            *materializable = true;
            *stack_class = ItemStackDocument::NonStackable;
            semantics.stack = Known(ReferenceItemStack {
                stackable: Known(false),
                stack_max: Unknown,
            });
            semantics.container = Known(ReferenceItemContainer {
                capacity: Known(capacity),
            });
            semantics.equipment = NotApplicableField;
            semantics.temporal = Known(ReferenceItemTemporal {
                consumption_mode: Known(ReferenceTemporalMode::DurableAbsoluteDeadline),
                duration: Known(ReferenceMilliseconds(duration_ms)),
                stop_duration: Known(stop_duration),
                decay_target: NotApplicableField,
            });
        }
        (StackClassV2::StackCapable, None, None, None, None, None, None, Some(stack_max))
            if stack_max > 1 =>
        {
            *materializable = true;
            *stack_class = ItemStackDocument::StackCapable;
            semantics.stack = Known(ReferenceItemStack {
                stackable: Known(true),
                stack_max: Known(stack_max),
            });
        }
        _ => {
            return Err(item_error(
                key,
                "admission shape is neither corpse nor stack",
            ));
        }
    }
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

    const CORPSE: &str = "oteryn:item.tibia.i5964";

    fn corpse_packet() -> serde_json::Value {
        serde_json::from_slice(ITEM_ADMISSION_V2_PACKET).expect("pinned v2 packet decodes")
    }

    fn apply_v2(
        packet: &serde_json::Value,
        key: &str,
        materializable: &mut bool,
        stack_class: &mut ItemStackDocument,
        semantics: &mut ReferenceItemSemantics,
    ) -> Result<usize, ItemAdmissionError> {
        let bytes = serde_json::to_vec(packet).expect("encodes");
        apply_admissions_v2(
            std::iter::once((key, materializable, stack_class, semantics)),
            &bytes,
        )
    }

    #[test]
    fn admits_the_rat_corpse_shape() {
        let (mut materializable, mut stack_class, mut semantics) = (
            false,
            ItemStackDocument::Unknown,
            ReferenceItemSemantics::default(),
        );
        let admitted = apply_v2(
            &corpse_packet(),
            CORPSE,
            &mut materializable,
            &mut stack_class,
            &mut semantics,
        )
        .expect("admits");
        assert_eq!(admitted, 1);
        assert!(materializable);
        assert_eq!(stack_class, ItemStackDocument::NonStackable);
        assert_eq!(
            semantics.container,
            ReferenceItemField::Known(ReferenceItemContainer {
                capacity: ReferenceItemField::Known(16)
            })
        );
        assert_eq!(semantics.equipment, ReferenceItemField::NotApplicable);
        assert_eq!(
            semantics.temporal,
            ReferenceItemField::Known(ReferenceItemTemporal {
                consumption_mode: ReferenceItemField::Known(
                    ReferenceTemporalMode::DurableAbsoluteDeadline
                ),
                duration: ReferenceItemField::Known(ReferenceMilliseconds(60_000)),
                stop_duration: ReferenceItemField::Known(false),
                decay_target: ReferenceItemField::NotApplicable,
            })
        );
        // A second run fails closed: the record is no longer identity-only.
        assert!(
            apply_v2(
                &corpse_packet(),
                CORPSE,
                &mut materializable,
                &mut stack_class,
                &mut semantics,
            )
            .is_err()
        );
    }

    #[test]
    fn rejects_bad_corpse_admissions() {
        // A missing Item.
        let (mut m, mut s, mut sem) = (
            false,
            ItemStackDocument::Unknown,
            ReferenceItemSemantics::default(),
        );
        assert!(
            apply_v2(
                &corpse_packet(),
                concat!("oteryn:item.tibia.", "i1"),
                &mut m,
                &mut s,
                &mut sem
            )
            .is_err()
        );
        // A materializable Item.
        let (mut m, mut s, mut sem) = (
            true,
            ItemStackDocument::Unknown,
            ReferenceItemSemantics::default(),
        );
        assert!(apply_v2(&corpse_packet(), CORPSE, &mut m, &mut s, &mut sem).is_err());
        // A set `temporal` group.
        let mut sem = ReferenceItemSemantics {
            temporal: ReferenceItemField::Known(ReferenceItemTemporal {
                consumption_mode: ReferenceItemField::Unknown,
                duration: ReferenceItemField::Known(ReferenceMilliseconds(10_000)),
                stop_duration: ReferenceItemField::Unknown,
                decay_target: ReferenceItemField::Unknown,
            }),
            ..ReferenceItemSemantics::default()
        };
        let (mut m, mut s) = (false, ItemStackDocument::Unknown);
        assert!(apply_v2(&corpse_packet(), CORPSE, &mut m, &mut s, &mut sem).is_err());
        // A packet with another source, a changed schema or a non-materializable row.
        for edit in [
            |p: &mut serde_json::Value| {
                p["admissions"][0]["evidence"]["crystalserver"] = serde_json::json!({});
            },
            |p: &mut serde_json::Value| p["schema"] = serde_json::json!("OTERYN_ITEM_ADMISSION/v1"),
            |p: &mut serde_json::Value| {
                p["admissions"][0]["materializable"] = serde_json::json!(false);
            },
            |p: &mut serde_json::Value| {
                p["policy"]["agreement"] = serde_json::json!("ALL_SOURCES_AGREE_ELSE_NO_ADMISSION");
            },
        ] {
            let mut packet = corpse_packet();
            edit(&mut packet);
            let (mut m, mut s, mut sem) = (
                false,
                ItemStackDocument::Unknown,
                ReferenceItemSemantics::default(),
            );
            assert!(apply_v2(&packet, CORPSE, &mut m, &mut s, &mut sem).is_err());
        }
    }

    #[test]
    fn pinned_v2_packet_digest_and_canary_evidence() {
        assert_eq!(
            world_project_sha256(ITEM_ADMISSION_V2_PACKET),
            ITEM_ADMISSION_V2_PACKET_SHA256
        );
        let packet = corpse_packet();
        assert_eq!(
            packet["admissions"].as_array().map(Vec::len),
            Some(ITEM_ADMISSION_V2_ITEM_COUNT)
        );
        let canary = &packet["admissions"][0]["evidence"]["canary"];
        let items_xml = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../imports/canary/items-xml/items.xml"
        ))
        .expect("pinned Canary import");
        assert_eq!(
            canary["sha256"].as_str(),
            Some(world_project_sha256(&items_xml).as_str())
        );
        let text = String::from_utf8(items_xml).expect("utf8");
        let lines: Vec<&str> = text.lines().collect();
        let block = lines[16140..16146].join("\n");
        assert_eq!(canary["lines"], "16141-16146");
        assert!(block.contains(r#"<item id="5964""#));
        for (name, value) in canary["attributes"].as_object().expect("attributes") {
            let needle = format!(
                r#"<attribute key="{name}" value="{}"/>"#,
                value.as_str().expect("string")
            );
            assert!(block.contains(&needle), "{needle}");
        }
        assert_eq!(canary["attributes"].as_object().map(|a| a.len()), Some(4));
    }

    const CHEESE: &str = "oteryn:item.tibia.i3607";

    fn cheese_packet() -> serde_json::Value {
        serde_json::from_slice(ITEM_ADMISSION_V2_CHEESE_PACKET).expect("pinned cheese packet")
    }

    #[test]
    fn admits_the_cheese_stack_shape() {
        let (mut materializable, mut stack_class, mut semantics) = (
            false,
            ItemStackDocument::Unknown,
            ReferenceItemSemantics::default(),
        );
        let admitted = apply_v2(
            &cheese_packet(),
            CHEESE,
            &mut materializable,
            &mut stack_class,
            &mut semantics,
        )
        .expect("admits");
        assert_eq!(admitted, 1);
        assert!(materializable);
        assert_eq!(stack_class, ItemStackDocument::StackCapable);
        assert_eq!(
            semantics.stack,
            ReferenceItemField::Known(ReferenceItemStack {
                stackable: ReferenceItemField::Known(true),
                stack_max: ReferenceItemField::Known(100),
            })
        );
        assert_eq!(semantics.container, ReferenceItemField::Unknown);
        assert_eq!(semantics.temporal, ReferenceItemField::Unknown);
        // A second run fails closed: the record is no longer identity-only.
        assert!(
            apply_v2(
                &cheese_packet(),
                CHEESE,
                &mut materializable,
                &mut stack_class,
                &mut semantics,
            )
            .is_err()
        );
    }

    #[test]
    fn rejects_mixed_or_bad_cheese_admissions() {
        for edit in [
            |p: &mut serde_json::Value| p["admissions"][0]["container_capacity"] = 16.into(),
            |p: &mut serde_json::Value| p["admissions"][0]["stack_max"] = 1.into(),
            |p: &mut serde_json::Value| {
                p["admissions"][0]["stack_class"] = "NON_STACKABLE".into();
            },
            |p: &mut serde_json::Value| {
                p["admissions"][0]["evidence"]["crystalserver"] = serde_json::json!({});
            },
        ] {
            let mut packet = cheese_packet();
            edit(&mut packet);
            let (mut m, mut s, mut sem) = (
                false,
                ItemStackDocument::Unknown,
                ReferenceItemSemantics::default(),
            );
            assert!(apply_v2(&packet, CHEESE, &mut m, &mut s, &mut sem).is_err());
        }
    }

    #[test]
    fn pinned_cheese_packet_digest_and_evidence() {
        assert_eq!(
            world_project_sha256(ITEM_ADMISSION_V2_CHEESE_PACKET),
            ITEM_ADMISSION_V2_CHEESE_PACKET_SHA256
        );
        let packet = cheese_packet();
        assert_eq!(
            packet["admissions"].as_array().map(Vec::len),
            Some(ITEM_ADMISSION_V2_CHEESE_ITEM_COUNT)
        );
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../");
        let evidence = &packet["admissions"][0]["evidence"];
        let canary = &evidence["canary"];
        let items_xml = std::fs::read(format!("{root}imports/canary/items-xml/items.xml"))
            .expect("pinned Canary import");
        assert_eq!(
            canary["sha256"].as_str(),
            Some(world_project_sha256(&items_xml).as_str())
        );
        let text = String::from_utf8(items_xml).expect("utf8");
        let lines: Vec<&str> = text.lines().collect();
        let block = lines[11462..11467].join("\n");
        assert_eq!(canary["lines"], "11463-11467");
        assert!(block.contains(r#"<item id="3607" name="cheese""#));
        for (name, value) in canary["attributes"].as_object().expect("attributes") {
            let needle = format!(
                r#"<attribute key="{name}" value="{}"/>"#,
                value.as_str().expect("string")
            );
            assert!(block.contains(&needle), "{needle}");
        }
        let candidate = &evidence["reference_candidate"];
        let bytes = std::fs::read(format!(
            "{root}{}",
            candidate["path"].as_str().expect("path")
        ))
        .expect("reference candidate");
        assert_eq!(
            candidate["sha256"].as_str(),
            Some(world_project_sha256(&bytes).as_str())
        );
        let text = String::from_utf8(bytes).expect("utf8");
        let lines: Vec<&str> = text.lines().collect();
        let record = lines[14038..14181].join("\n");
        assert_eq!(candidate["lines"], "14039-14181");
        assert!(record.contains(r#""key": "candidate:item/3607""#));
        assert!(record.contains(r#""stack_class": "StackCapable""#));
        assert!(record.contains(r#""stack_max": {"#));
        let parsed: serde_json::Value =
            serde_json::from_str(&format!("{{\"r\":{}}}", record.trim_end_matches(',')))
                .expect("record decodes");
        let stack = &parsed["r"]["semantics"]["stack"]["value"];
        assert_eq!(stack["stackable"]["value"], candidate["stackable"]);
        assert_eq!(stack["stack_max"]["value"], candidate["stack_max"]);
    }
}
