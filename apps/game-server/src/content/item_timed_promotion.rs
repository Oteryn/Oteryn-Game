//! Timed-item facts (TIMED-CONTENT-1): charges, duration and the timed transforms on Item records.
//!
//! The pinned packet `docs/agents/evidence/OTV2-20261003-timed-item-facts-v1.json` is lowered by
//! `lower_timed_item_packet.py` from TibiaWiki first, with Canary `items.xml` (OTS_HYPOTHESIS_ONLY)
//! as the fallback and the only source of transform pairs (TIMED-ITEM-0 brief and sections 4 and 6,
//! TIMED-ITEM-0B sections 4 and 10.1). The materializer applies it once, after
//! [`super::item_stats_promotion`]. Rows are decoded strictly (exact shape, bounds, closed enums);
//! each names an existing Item record, a field appears once per item, a timed definition is never
//! stackable, a decay target never disagrees with an earlier `temporal.decay_target`, and a
//! `temporal` group is complete (duration and mode). Anything else fails closed.
//!
//! Mapping to the Reference Item model: `ON_EQUIP` is the authoritative active-time budget and
//! `CONTINUOUS` the durable absolute deadline (GAME-ITEM-01 section 4.4); `transform.equip`,
//! `unequip`, `use` and `decay` are the UseTransform kinds Equip, Deequip, Use and Decay.

use super::{
    ProjectReferenceRecord, ProjectV2Draft, ReferenceItemCharges, ReferenceItemField,
    ReferenceItemSemantics, ReferenceItemTarget, ReferenceItemTemporal, ReferenceItemUseTransform,
    ReferenceMilliseconds, ReferenceTemporalMode, ReferenceTransformKind, ReferenceTransformTarget,
    world_project_sha256,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

/// The pinned packet bytes; any change is a new candidate with a new digest.
pub const ITEM_TIMED_PROMOTION_V1_PACKET: &[u8] =
    include_bytes!("../../../../docs/agents/evidence/OTV2-20261003-timed-item-facts-v1.json");
pub const ITEM_TIMED_PROMOTION_V1_PACKET_SHA256: &str =
    "2158aff0d7da64b455e9db99c8720095451f8f28135f447f590f0a3168487ebf";
pub const ITEM_TIMED_PROMOTION_V1_FIELD_COUNT: usize = 271;
pub const ITEM_TIMED_PROMOTION_V1_ITEM_COUNT: usize = 110;
const SCHEMA: &str = "OTERYN_ITEM_TIMED_PROMOTION/v1";
/// TIMEDITEM0-RL-01: charges per item.
pub const MAX_CHARGES: u32 = 65_535;
/// TIMEDITEM0-RL-02: active-time budget per item, 7 days.
pub const MAX_DURATION_MS: u64 = 604_800_000;
const DEFINITION_REVISION: &str = "definition-r1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemTimedPromotionError {
    Digest,
    Decode(String),
    Row {
        item_key: String,
        field_path: String,
        reason: &'static str,
    },
    Counts,
}

impl std::fmt::Display for ItemTimedPromotionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Digest => formatter.write_str("item timed promotion packet digest mismatch"),
            Self::Decode(error) => write!(formatter, "item timed promotion packet: {error}"),
            Self::Row {
                item_key,
                field_path,
                reason,
            } => write!(
                formatter,
                "item timed promotion {item_key} {field_path}: {reason}"
            ),
            Self::Counts => formatter.write_str("item timed promotion counts drifted"),
        }
    }
}

impl std::error::Error for ItemTimedPromotionError {}

/// What one application changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemTimedPromotion {
    pub fields: usize,
    pub items: usize,
    /// Rows whose field already held a different value (an earlier promotion).
    pub replaced: usize,
    /// Inactive equip forms whose inherited `temporal` group was cleared: such a form is defined
    /// only through its paired active form (TIMED-ITEM-0 §4), never by its own duration.
    pub inactive_temporal_cleared: usize,
}

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
struct Row {
    item_key: String,
    field_path: String,
    typed_value: TypedValue,
}

#[derive(Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "SCREAMING_SNAKE_CASE",
    deny_unknown_fields
)]
enum TypedValue {
    CountU32(u32),
    Milliseconds(u64),
    ConsumptionMode(ConsumptionMode),
    Bool(bool),
    ItemTarget(String),
}

/// The closed schema enum (`temporal.consumption_mode`); `on_use` is not admitted by this packet.
#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum ConsumptionMode {
    OnEquip,
    Continuous,
}

/// Apply the pinned packet to every Item record in `draft`.
pub fn apply_item_timed_promotion_v1(
    draft: &mut ProjectV2Draft,
) -> Result<ItemTimedPromotion, ItemTimedPromotionError> {
    if world_project_sha256(ITEM_TIMED_PROMOTION_V1_PACKET) != ITEM_TIMED_PROMOTION_V1_PACKET_SHA256
    {
        return Err(ItemTimedPromotionError::Digest);
    }
    let applied = apply_packet(draft, ITEM_TIMED_PROMOTION_V1_PACKET)?;
    if applied.fields != ITEM_TIMED_PROMOTION_V1_FIELD_COUNT
        || applied.items != ITEM_TIMED_PROMOTION_V1_ITEM_COUNT
    {
        return Err(ItemTimedPromotionError::Counts);
    }
    Ok(applied)
}

fn apply_packet(
    draft: &mut ProjectV2Draft,
    bytes: &[u8],
) -> Result<ItemTimedPromotion, ItemTimedPromotionError> {
    let keys = draft
        .core
        .records
        .iter()
        .filter_map(|record| match record {
            ProjectReferenceRecord::Item { identity, .. } => Some(identity.key.clone()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
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
    apply_rows(items, &keys, bytes)
}

/// Apply a packet to `(Item key, semantics)` pairs; every row must name one of them and every
/// transform target must be one of `keys`.
fn apply_rows<'a>(
    items: impl Iterator<Item = (&'a str, &'a mut ReferenceItemSemantics)>,
    keys: &BTreeSet<String>,
    bytes: &[u8],
) -> Result<ItemTimedPromotion, ItemTimedPromotionError> {
    let packet: Packet = serde_json::from_slice(bytes)
        .map_err(|error| ItemTimedPromotionError::Decode(error.to_string()))?;
    if packet.schema != SCHEMA {
        return Err(ItemTimedPromotionError::Decode("schema".to_owned()));
    }
    let mut by_item: BTreeMap<&str, Vec<&Row>> = BTreeMap::new();
    let mut seen = BTreeSet::new();
    for row in &packet.promotions {
        if !seen.insert((row.item_key.as_str(), row.field_path.as_str())) {
            return Err(row_error(row, "duplicate field row"));
        }
        by_item.entry(row.item_key.as_str()).or_default().push(row);
    }
    if packet.counts.fields != packet.promotions.len() || packet.counts.items != by_item.len() {
        return Err(ItemTimedPromotionError::Counts);
    }

    let mut applied = ItemTimedPromotion {
        fields: 0,
        items: 0,
        replaced: 0,
        inactive_temporal_cleared: 0,
    };
    for (key, semantics) in items {
        let Some(rows) = by_item.remove(key) else {
            continue;
        };
        let first = rows[0];
        let timed = rows.iter().any(|row| {
            row.field_path.starts_with("charges.") || row.field_path.starts_with("temporal.")
        });
        if timed
            && matches!(
                &semantics.stack,
                ReferenceItemField::Known(stack)
                    if stack.stackable == ReferenceItemField::Known(true)
            )
        {
            return Err(row_error(first, "a timed definition is not stackable"));
        }
        for row in &rows {
            if set_field(semantics, row, key, keys)? {
                applied.replaced += 1;
            }
            applied.fields += 1;
        }
        let packet_temporal = rows
            .iter()
            .any(|row| row.field_path.starts_with("temporal."));
        // An inactive equip form carries only `transform.equip`; a duration an earlier promotion
        // gave it (a wiki page listing the ring's time on its unworn form) is the active form's,
        // so the inactive form keeps no temporal group of its own.
        let inactive_form = rows.iter().any(|row| row.field_path == "transform.equip");
        if inactive_form && !packet_temporal && semantics.temporal != ReferenceItemField::Unknown {
            semantics.temporal = ReferenceItemField::Unknown;
            applied.inactive_temporal_cleared += 1;
        }
        // An earlier stats promotion may have set `temporal.duration` alone on the inactive form
        // of a pair; only items this packet gives temporal rows must end complete.
        if packet_temporal && let ReferenceItemField::Known(temporal) = &semantics.temporal {
            let complete = matches!(temporal.duration, ReferenceItemField::Known(_))
                && matches!(temporal.consumption_mode, ReferenceItemField::Known(_));
            if !complete {
                return Err(row_error(first, "temporal needs duration and mode"));
            }
        }
        applied.items += 1;
    }
    if let Some((item_key, rows)) = by_item.into_iter().next() {
        return Err(ItemTimedPromotionError::Row {
            item_key: item_key.to_owned(),
            field_path: rows[0].field_path.clone(),
            reason: "no Item record with this key",
        });
    }
    Ok(applied)
}

fn row_error(row: &Row, reason: &'static str) -> ItemTimedPromotionError {
    ItemTimedPromotionError::Row {
        item_key: row.item_key.clone(),
        field_path: row.field_path.clone(),
        reason,
    }
}

/// Set one field; `true` when it replaced a different known value.
fn set<T: PartialEq>(field: &mut ReferenceItemField<T>, value: T) -> bool {
    let replaced = matches!(field, ReferenceItemField::Known(old) if *old != value);
    *field = ReferenceItemField::Known(value);
    replaced
}

fn set_field(
    semantics: &mut ReferenceItemSemantics,
    row: &Row,
    item_key: &str,
    keys: &BTreeSet<String>,
) -> Result<bool, ItemTimedPromotionError> {
    let wrong = || row_error(row, "typed value does not fit the field");
    Ok(match (row.field_path.as_str(), &row.typed_value) {
        ("charges.count", TypedValue::CountU32(count)) => {
            if !(1..=MAX_CHARGES).contains(count) {
                return Err(row_error(row, "charges outside TIMEDITEM0-RL-01"));
            }
            set(&mut charges(semantics, row)?.count, *count)
        }
        ("temporal.duration_ms", TypedValue::Milliseconds(ms)) => {
            if !(1..=MAX_DURATION_MS).contains(ms) {
                return Err(row_error(row, "duration outside TIMEDITEM0-RL-02"));
            }
            set(
                &mut temporal(semantics, row)?.duration,
                ReferenceMilliseconds(*ms),
            )
        }
        ("temporal.consumption_mode", TypedValue::ConsumptionMode(mode)) => {
            // D397: both modes run on an active-time budget in a slot (TIMED-ITEM-0B §10.2);
            // the codec is unchanged, and a continuous form is the one with
            // `stop_duration = false` (the Ground deadline is runtime state, not content).
            let mode = match mode {
                ConsumptionMode::OnEquip | ConsumptionMode::Continuous => {
                    ReferenceTemporalMode::AuthoritativeActiveTimeBudget
                }
            };
            set(&mut temporal(semantics, row)?.consumption_mode, mode)
        }
        ("temporal.stop_duration_while_unequipped", TypedValue::Bool(stop)) => {
            set(&mut temporal(semantics, row)?.stop_duration, *stop)
        }
        (path, TypedValue::ItemTarget(target)) if path.starts_with("transform.") => {
            let kind = match path {
                "transform.use" => ReferenceTransformKind::Use,
                "transform.equip" => ReferenceTransformKind::Equip,
                "transform.unequip" => ReferenceTransformKind::Deequip,
                "transform.decay" => ReferenceTransformKind::Decay,
                _ => return Err(row_error(row, "unknown transform trigger")),
            };
            if target == item_key || !keys.contains(target) {
                return Err(row_error(
                    row,
                    "transform target is not another Item record",
                ));
            }
            let target = ReferenceItemTarget::new(target, DEFINITION_REVISION)
                .map_err(|_| row_error(row, "transform target is not a definition reference"))?;
            if kind == ReferenceTransformKind::Decay
                && let ReferenceItemField::Known(temporal) = &semantics.temporal
                && let ReferenceItemField::Known(existing) = &temporal.decay_target
                && *existing != target
            {
                return Err(row_error(
                    row,
                    "decay target differs from temporal.decay_target",
                ));
            }
            let entry = use_transform(semantics, row)?
                .targets
                .iter_mut()
                .find(|entry| entry.kind == kind)
                .ok_or_else(|| row_error(row, "UseTransform kind is missing"))?;
            set(&mut entry.target, target)
        }
        _ => return Err(wrong()),
    })
}

/// The Known group of `field`, created (all members Unknown) when the group is Unknown.
fn group<'a, T>(
    field: &'a mut ReferenceItemField<T>,
    empty: impl FnOnce() -> T,
    row: &Row,
) -> Result<&'a mut T, ItemTimedPromotionError> {
    if matches!(field, ReferenceItemField::Unknown) {
        *field = ReferenceItemField::Known(empty());
    }
    match field {
        ReferenceItemField::Known(value) => Ok(value),
        _ => Err(row_error(
            row,
            "field group is not applicable or in conflict",
        )),
    }
}

fn charges<'a>(
    semantics: &'a mut ReferenceItemSemantics,
    row: &Row,
) -> Result<&'a mut ReferenceItemCharges, ItemTimedPromotionError> {
    group(
        &mut semantics.charges,
        || ReferenceItemCharges {
            count: ReferenceItemField::Unknown,
        },
        row,
    )
}

fn temporal<'a>(
    semantics: &'a mut ReferenceItemSemantics,
    row: &Row,
) -> Result<&'a mut ReferenceItemTemporal, ItemTimedPromotionError> {
    group(
        &mut semantics.temporal,
        || ReferenceItemTemporal {
            consumption_mode: ReferenceItemField::Unknown,
            duration: ReferenceItemField::Unknown,
            stop_duration: ReferenceItemField::Unknown,
            decay_target: ReferenceItemField::Unknown,
        },
        row,
    )
}

fn use_transform<'a>(
    semantics: &'a mut ReferenceItemSemantics,
    row: &Row,
) -> Result<&'a mut ReferenceItemUseTransform, ItemTimedPromotionError> {
    group(
        &mut semantics.use_transform,
        || ReferenceItemUseTransform {
            targets: (1..=10)
                .filter_map(|wire| ReferenceTransformKind::from_wire(wire).ok())
                .map(|kind| ReferenceTransformTarget {
                    kind,
                    target: ReferenceItemField::Unknown,
                })
                .collect(),
        },
        row,
    )
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::content::ReferenceItemStack;

    const KEY: &str = "oteryn:item.tibia.i3089";
    const OTHER: &str = "oteryn:item.tibia.i3052";

    fn keys() -> BTreeSet<String> {
        [KEY, OTHER].map(str::to_owned).into_iter().collect()
    }

    fn packet(rows: &str, fields: usize, items: usize) -> Vec<u8> {
        format!(
            r#"{{"schema":"{SCHEMA}","counts":{{"fields":{fields},"items":{items}}},"promotions":[{rows}]}}"#
        )
        .into_bytes()
    }

    fn row(key: &str, path: &str, typed: &str) -> String {
        format!(r#"{{"item_key":"{key}","field_path":"{path}","typed_value":{typed}}}"#)
    }

    fn apply(
        bytes: &[u8],
        semantics: &mut ReferenceItemSemantics,
    ) -> Result<ItemTimedPromotion, ItemTimedPromotionError> {
        apply_rows(std::iter::once((KEY, semantics)), &keys(), bytes)
    }

    fn ring_rows() -> Vec<String> {
        vec![
            row(KEY, "charges.count", r#"{"kind":"COUNT_U32","value":200}"#),
            row(
                KEY,
                "temporal.duration_ms",
                r#"{"kind":"MILLISECONDS","value":1200000}"#,
            ),
            row(
                KEY,
                "temporal.consumption_mode",
                r#"{"kind":"CONSUMPTION_MODE","value":"ON_EQUIP"}"#,
            ),
            row(
                KEY,
                "temporal.stop_duration_while_unequipped",
                r#"{"kind":"BOOL","value":true}"#,
            ),
            row(
                KEY,
                "transform.unequip",
                &format!(r#"{{"kind":"ITEM_TARGET","value":"{OTHER}"}}"#),
            ),
        ]
    }

    #[test]
    fn applies_timed_facts() {
        let mut semantics = ReferenceItemSemantics::default();
        let rows = ring_rows();
        let applied =
            apply(&packet(&rows.join(","), rows.len(), 1), &mut semantics).expect("applies");
        assert_eq!(
            applied,
            ItemTimedPromotion {
                fields: 5,
                items: 1,
                replaced: 0,
                inactive_temporal_cleared: 0,
            }
        );
        let ReferenceItemField::Known(charges) = &semantics.charges else {
            panic!("charges group");
        };
        assert_eq!(charges.count, ReferenceItemField::Known(200));
        let ReferenceItemField::Known(temporal) = &semantics.temporal else {
            panic!("temporal group");
        };
        assert_eq!(
            temporal.consumption_mode,
            ReferenceItemField::Known(ReferenceTemporalMode::AuthoritativeActiveTimeBudget)
        );
        assert_eq!(
            temporal.duration,
            ReferenceItemField::Known(ReferenceMilliseconds(1_200_000))
        );
        let ReferenceItemField::Known(transform) = &semantics.use_transform else {
            panic!("use transform group");
        };
        assert_eq!(transform.targets.len(), 10);
        let deequip = transform
            .targets
            .iter()
            .find(|entry| entry.kind == ReferenceTransformKind::Deequip)
            .expect("deequip entry");
        assert_eq!(
            deequip.target,
            ReferenceItemField::Known(
                ReferenceItemTarget::new(OTHER, DEFINITION_REVISION).expect("target")
            )
        );
    }

    #[test]
    fn rejects_bad_rows() {
        let charges = row(KEY, "charges.count", r#"{"kind":"COUNT_U32","value":5}"#);
        let duration = row(
            KEY,
            "temporal.duration_ms",
            r#"{"kind":"MILLISECONDS","value":1000}"#,
        );
        let cases = [
            // duplicate field for one item
            packet(&format!("{charges},{charges}"), 2, 1),
            // unknown Item key
            packet(
                &row(
                    "oteryn:item.tibia.i3030",
                    "charges.count",
                    r#"{"kind":"COUNT_U32","value":5}"#,
                ),
                1,
                1,
            ),
            // value kind that does not fit the field
            packet(
                &row(KEY, "charges.count", r#"{"kind":"BOOL","value":true}"#),
                1,
                1,
            ),
            // unknown field path
            packet(
                &row(KEY, "charges.show_count", r#"{"kind":"BOOL","value":true}"#),
                1,
                1,
            ),
            // closed enum: on_use is not admitted
            packet(
                &format!(
                    "{duration},{}",
                    row(
                        KEY,
                        "temporal.consumption_mode",
                        r#"{"kind":"CONSUMPTION_MODE","value":"ON_USE"}"#
                    )
                ),
                2,
                1,
            ),
            // TIMEDITEM0-RL-01 and RL-02
            packet(
                &row(
                    KEY,
                    "charges.count",
                    r#"{"kind":"COUNT_U32","value":65536}"#,
                ),
                1,
                1,
            ),
            packet(
                &row(
                    KEY,
                    "temporal.duration_ms",
                    r#"{"kind":"MILLISECONDS","value":604800001}"#,
                ),
                1,
                1,
            ),
            packet(
                &row(KEY, "charges.count", r#"{"kind":"COUNT_U32","value":0}"#),
                1,
                1,
            ),
            // temporal without its mode
            packet(&duration, 1, 1),
            // transform to an unknown Item, to itself, with an unknown trigger
            packet(
                &row(
                    KEY,
                    "transform.decay",
                    r#"{"kind":"ITEM_TARGET","value":"oteryn:item.tibia.i3030"}"#,
                ),
                1,
                1,
            ),
            packet(
                &row(
                    KEY,
                    "transform.decay",
                    &format!(r#"{{"kind":"ITEM_TARGET","value":"{KEY}"}}"#),
                ),
                1,
                1,
            ),
            packet(
                &row(
                    KEY,
                    "transform.destroy",
                    &format!(r#"{{"kind":"ITEM_TARGET","value":"{OTHER}"}}"#),
                ),
                1,
                1,
            ),
            // counts that do not match the rows
            packet(&charges, 2, 1),
        ];
        for bytes in cases {
            let mut semantics = ReferenceItemSemantics::default();
            assert!(
                apply(&bytes, &mut semantics).is_err(),
                "{}",
                String::from_utf8_lossy(&bytes)
            );
        }
    }

    #[test]
    fn rejects_stackable_and_conflicting_decay() {
        let charges = packet(
            &row(KEY, "charges.count", r#"{"kind":"COUNT_U32","value":5}"#),
            1,
            1,
        );
        let mut stackable = ReferenceItemSemantics {
            stack: ReferenceItemField::Known(ReferenceItemStack {
                stackable: ReferenceItemField::Known(true),
                stack_max: ReferenceItemField::Unknown,
            }),
            ..ReferenceItemSemantics::default()
        };
        assert!(apply(&charges, &mut stackable).is_err());

        let decay = packet(
            &row(
                KEY,
                "transform.decay",
                &format!(r#"{{"kind":"ITEM_TARGET","value":"{OTHER}"}}"#),
            ),
            1,
            1,
        );
        let mut conflicting = ReferenceItemSemantics {
            temporal: ReferenceItemField::Known(ReferenceItemTemporal {
                consumption_mode: ReferenceItemField::Unknown,
                duration: ReferenceItemField::Unknown,
                stop_duration: ReferenceItemField::Unknown,
                decay_target: ReferenceItemField::Known(
                    ReferenceItemTarget::new("oteryn:item.tibia.i3030", DEFINITION_REVISION)
                        .expect("target"),
                ),
            }),
            ..ReferenceItemSemantics::default()
        };
        assert!(apply(&decay, &mut conflicting).is_err());
    }

    #[test]
    fn clears_inherited_temporal_on_inactive_equip_forms() {
        let equip = packet(
            &row(
                KEY,
                "transform.equip",
                &format!(r#"{{"kind":"ITEM_TARGET","value":"{OTHER}"}}"#),
            ),
            1,
            1,
        );
        let mut inactive = ReferenceItemSemantics {
            temporal: ReferenceItemField::Known(ReferenceItemTemporal {
                consumption_mode: ReferenceItemField::Unknown,
                duration: ReferenceItemField::Known(ReferenceMilliseconds(1_200_000)),
                stop_duration: ReferenceItemField::Unknown,
                decay_target: ReferenceItemField::Unknown,
            }),
            ..ReferenceItemSemantics::default()
        };
        let applied = apply(&equip, &mut inactive).expect("applies");
        assert_eq!(applied.inactive_temporal_cleared, 1);
        assert_eq!(inactive.temporal, ReferenceItemField::Unknown);
    }

    #[test]
    fn pinned_packet_decodes_with_its_counts() {
        assert_eq!(
            world_project_sha256(ITEM_TIMED_PROMOTION_V1_PACKET),
            ITEM_TIMED_PROMOTION_V1_PACKET_SHA256
        );
        let packet: Packet =
            serde_json::from_slice(ITEM_TIMED_PROMOTION_V1_PACKET).expect("pinned packet decodes");
        assert_eq!(packet.counts.fields, ITEM_TIMED_PROMOTION_V1_FIELD_COUNT);
        assert_eq!(packet.counts.items, ITEM_TIMED_PROMOTION_V1_ITEM_COUNT);
        assert_eq!(packet.promotions.len(), ITEM_TIMED_PROMOTION_V1_FIELD_COUNT);
    }
}
