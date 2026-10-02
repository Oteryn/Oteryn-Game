//! Item stat promotion v2 (ITEM-SEM-2b): TibiaWiki stats into the canonical Item records.
//!
//! The source policy ranks tibia.com, then TibiaWiki; Crystal and Canary are hypotheses. The
//! pinned packet `docs/agents/evidence/OTV2-20260930-item-stats-promotion-v2.json` is lowered
//! from the pinned TibiaWiki snapshot by `lower_wiki_stats_packet.py`, keyed by canonical Tibia
//! Item keys (A12). The materializer applies it once, after [`super::item_identity`]'s key
//! switch: each row sets its field to the wiki value, replacing whatever an earlier promotion
//! put there. A field the wiki is silent on is left as it is.
//! Equipment rows contain one source-qualified pattern; absent restrictions remain Unknown.
//! They do not grant materialization or a legal destination, and the main backpack retains
//! its separately qualified starter admission. Declared charges and duration do not infer
//! consumption, decay, activation, materialization or any other temporal behavior.
//! Resistance percentages are signed percentage points; they do not activate Combat behavior.
//!
//! Every row is decoded strictly (exact shape, bounds and closed enums), must name an existing
//! Item record, and a field may appear once per item; anything else fails closed.

use super::{
    ProjectReferenceRecord, ProjectV2Draft, REFERENCE_ITEM_MAX_MODIFIERS,
    REFERENCE_ITEM_MAX_RESISTANCES, ReferenceCells, ReferenceElementalAttack,
    ReferenceEquipmentPattern, ReferenceEquipmentSlot, ReferenceItemCharges,
    ReferenceItemEquipment, ReferenceItemField, ReferenceItemImbuement, ReferenceItemPhysical,
    ReferenceItemProtection, ReferenceItemSemantics, ReferenceItemSkillModifiers,
    ReferenceItemTemporal, ReferenceItemWeapon, ReferenceMilliseconds, ReferenceModifierBinding,
    ReferenceModifierParameter, ReferenceRationalPercent, ReferenceResistance,
    ReferenceResistanceKind, ReferenceSignedPoints, ReferenceSkillModifierKind,
    ReferenceWeaponElement, ReferenceWeaponType, world_project_sha256,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

/// The pinned packet bytes; any change is a new candidate with a new digest.
pub const ITEM_STATS_PROMOTION_V2_PACKET: &[u8] =
    include_bytes!("../../../../docs/agents/evidence/OTV2-20260930-item-stats-promotion-v2.json");
pub const ITEM_STATS_PROMOTION_V2_PACKET_SHA256: &str =
    "f833bc32bfc8762dbece8a7c2abe2985946fd153da4915e163c81e94f91c7bcc";
pub const ITEM_STATS_PROMOTION_V2_FIELD_COUNT: usize = 13_298;
pub const ITEM_STATS_PROMOTION_V2_ITEM_COUNT: usize = 6_533;
const SCHEMA: &str = "OTERYN_ITEM_STATS_PROMOTION/v2";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemStatsPromotionError {
    Digest,
    Decode(String),
    Row {
        item_key: String,
        field_path: String,
        reason: &'static str,
    },
    Counts,
}

impl std::fmt::Display for ItemStatsPromotionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Digest => formatter.write_str("item stat promotion packet digest mismatch"),
            Self::Decode(error) => write!(formatter, "item stat promotion packet: {error}"),
            Self::Row {
                item_key,
                field_path,
                reason,
            } => {
                write!(
                    formatter,
                    "item stat promotion {item_key} {field_path}: {reason}"
                )
            }
            Self::Counts => formatter.write_str("item stat promotion counts drifted"),
        }
    }
}

impl std::error::Error for ItemStatsPromotionError {}

/// What one application changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemStatsPromotion {
    pub fields: usize,
    pub items: usize,
    /// Rows whose field already held a different value (an earlier, non-wiki promotion).
    pub replaced: usize,
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
    EquipmentPatterns(Vec<ReferenceEquipmentPattern>),
    SignedPoints(i32),
    Cells(u16),
    WeaponType(ReferenceWeaponType),
    ElementalAttacks(Vec<ElementalAttack>),
    CountU8(u8),
    CountU32(u32),
    DurationMs(u64),
    Resistances(Vec<DeclaredResistance>),
    Modifiers(Vec<ReferenceModifierBinding>),
    WeightCentiOz(u32),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeclaredResistance {
    kind: ReferenceResistanceKind,
    percent: ReferenceRationalPercent,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ElementalAttack {
    element: ReferenceWeaponElement,
    points: i32,
}

/// Apply the pinned packet to every Item record in `draft`.
pub fn apply_item_stats_promotion_v2(
    draft: &mut ProjectV2Draft,
) -> Result<ItemStatsPromotion, ItemStatsPromotionError> {
    if world_project_sha256(ITEM_STATS_PROMOTION_V2_PACKET) != ITEM_STATS_PROMOTION_V2_PACKET_SHA256
    {
        return Err(ItemStatsPromotionError::Digest);
    }
    let applied = apply_packet(draft, ITEM_STATS_PROMOTION_V2_PACKET)?;
    if applied.fields != ITEM_STATS_PROMOTION_V2_FIELD_COUNT
        || applied.items != ITEM_STATS_PROMOTION_V2_ITEM_COUNT
    {
        return Err(ItemStatsPromotionError::Counts);
    }
    Ok(applied)
}

pub(super) fn apply_packet(
    draft: &mut ProjectV2Draft,
    bytes: &[u8],
) -> Result<ItemStatsPromotion, ItemStatsPromotionError> {
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
    apply_rows(items, bytes)
}

/// Apply a packet to `(Item key, semantics)` pairs; every row must name one of them.
fn apply_rows<'a>(
    items: impl Iterator<Item = (&'a str, &'a mut ReferenceItemSemantics)>,
    bytes: &[u8],
) -> Result<ItemStatsPromotion, ItemStatsPromotionError> {
    let packet: Packet = serde_json::from_slice(bytes)
        .map_err(|error| ItemStatsPromotionError::Decode(error.to_string()))?;
    if packet.schema != SCHEMA {
        return Err(ItemStatsPromotionError::Decode("schema".to_owned()));
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
        return Err(ItemStatsPromotionError::Counts);
    }

    // Validate every binding and row against a private candidate before any mutation.
    // This also makes a late conflict or missing Item key reject the whole packet.
    let items = items.collect::<Vec<_>>();
    let mut unmatched = by_item.clone();
    for (key, semantics) in &items {
        if let Some(rows) = unmatched.remove(key) {
            let mut candidate = (**semantics).clone();
            for row in rows {
                set_field(&mut candidate, row)?;
            }
        }
    }
    if let Some((item_key, rows)) = unmatched.into_iter().next() {
        return Err(ItemStatsPromotionError::Row {
            item_key: item_key.to_owned(),
            field_path: rows[0].field_path.clone(),
            reason: "no Item record with this key",
        });
    }
    let mut applied = ItemStatsPromotion {
        fields: 0,
        items: 0,
        replaced: 0,
    };
    for (key, semantics) in items {
        let Some(rows) = by_item.remove(key) else {
            continue;
        };
        for row in rows {
            if set_field(semantics, row)? {
                applied.replaced += 1;
            }
            applied.fields += 1;
        }
        applied.items += 1;
    }
    if let Some((item_key, rows)) = by_item.into_iter().next() {
        return Err(ItemStatsPromotionError::Row {
            item_key: item_key.to_owned(),
            field_path: rows[0].field_path.clone(),
            reason: "no Item record with this key",
        });
    }
    Ok(applied)
}

fn row_error(row: &Row, reason: &'static str) -> ItemStatsPromotionError {
    ItemStatsPromotionError::Row {
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

// Declared count/time values only fill unknowns or repeat the same known value.
// A blocked leaf must not become known just because its containing group is known.
fn set_declared<T: PartialEq>(
    field: &mut ReferenceItemField<T>,
    value: T,
    row: &Row,
) -> Result<bool, ItemStatsPromotionError> {
    match field {
        ReferenceItemField::Unknown => {
            *field = ReferenceItemField::Known(value);
            Ok(false)
        }
        ReferenceItemField::Known(old) if *old == value => Ok(false),
        _ => Err(row_error(
            row,
            "declared field is blocked or has a conflicting known value",
        )),
    }
}

fn set_field(
    semantics: &mut ReferenceItemSemantics,
    row: &Row,
) -> Result<bool, ItemStatsPromotionError> {
    let wrong = || row_error(row, "typed value does not fit the field");
    Ok(match (row.field_path.as_str(), &row.typed_value) {
        ("equipment.patterns", TypedValue::EquipmentPatterns(patterns)) => {
            if patterns.len() != 1 || !qualified_equipment_pattern(&patterns[0]) {
                return Err(wrong());
            }
            let equipment = group(
                &mut semantics.equipment,
                || ReferenceItemEquipment {
                    patterns: ReferenceItemField::Unknown,
                },
                row,
            )?;
            set(&mut equipment.patterns, patterns.clone())
        }
        ("weapon.attack", TypedValue::SignedPoints(points)) => set(
            &mut weapon(semantics, row)?.attack,
            ReferenceSignedPoints(*points),
        ),
        ("weapon.defense", TypedValue::SignedPoints(points)) => set(
            &mut weapon(semantics, row)?.defense,
            ReferenceSignedPoints(*points),
        ),
        ("weapon.extra_defense", TypedValue::SignedPoints(points)) => set(
            &mut weapon(semantics, row)?.extra_defense,
            ReferenceSignedPoints(*points),
        ),
        ("weapon.range_cells", TypedValue::Cells(cells)) => {
            set(&mut weapon(semantics, row)?.range, ReferenceCells(*cells))
        }
        ("weapon.weapon_type", TypedValue::WeaponType(kind)) => {
            set(&mut weapon(semantics, row)?.weapon_type, *kind)
        }
        ("weapon.elemental", TypedValue::ElementalAttacks(attacks)) => {
            let elements = attacks
                .iter()
                .map(|attack| attack.element)
                .collect::<BTreeSet<_>>();
            if attacks.is_empty() || elements.len() != attacks.len() {
                return Err(wrong());
            }
            let value = attacks
                .iter()
                .map(|attack| ReferenceElementalAttack {
                    element: attack.element,
                    points: ReferenceItemField::Known(ReferenceSignedPoints(attack.points)),
                })
                .collect();
            set(&mut weapon(semantics, row)?.elemental, value)
        }
        ("protection.armor", TypedValue::SignedPoints(points)) => set(
            &mut protection(semantics, row)?.armor,
            ReferenceSignedPoints(*points),
        ),
        ("imbuement.slot_count", TypedValue::CountU8(count)) => {
            set(&mut imbuement(semantics, row)?.slot_count, *count)
        }
        ("physical.weight", TypedValue::WeightCentiOz(weight)) => {
            set(&mut physical(semantics, row)?.weight, *weight)
        }
        ("charges.count", TypedValue::CountU32(count)) => {
            let charges = group(
                &mut semantics.charges,
                || ReferenceItemCharges {
                    count: ReferenceItemField::Unknown,
                },
                row,
            )?;
            set_declared(&mut charges.count, *count, row)?
        }
        ("temporal.duration", TypedValue::DurationMs(milliseconds)) => {
            let temporal = group(
                &mut semantics.temporal,
                || ReferenceItemTemporal {
                    consumption_mode: ReferenceItemField::Unknown,
                    duration: ReferenceItemField::Unknown,
                    stop_duration: ReferenceItemField::Unknown,
                    decay_target: ReferenceItemField::Unknown,
                },
                row,
            )?;
            set_declared(
                &mut temporal.duration,
                ReferenceMilliseconds(*milliseconds),
                row,
            )?
        }
        ("skill_modifiers.modifiers", TypedValue::Modifiers(entries)) => {
            use ReferenceItemField::{Known, Unknown};
            use ReferenceModifierParameter::{Element, RationalPercent, SignedPoints};
            use ReferenceSkillModifierKind as Kind;
            if entries.is_empty()
                || entries.len() > REFERENCE_ITEM_MAX_MODIFIERS
                || entries.windows(2).any(|pair| pair[0].kind >= pair[1].kind)
                || entries.iter().any(|entry| {
                    let parameter = match (entry.kind, &entry.parameter) {
                        (Kind::Mantra, Known(SignedPoints(points))) => {
                            i16::try_from(points.0).is_ok()
                        }
                        (Kind::ElementalBond, Known(Element(value))) => matches!(
                            value,
                            super::ReferenceModifierElement::Earth
                                | super::ReferenceModifierElement::Energy
                        ),
                        (
                            Kind::DeathMagicLevelPoints
                            | Kind::EarthMagicLevelPoints
                            | Kind::EnergyMagicLevelPoints
                            | Kind::FireMagicLevelPoints
                            | Kind::HealingMagicLevelPoints
                            | Kind::HolyMagicLevelPoints
                            | Kind::IceMagicLevelPoints
                            | Kind::MagicLevelPoints
                            | Kind::SkillAxe
                            | Kind::SkillClub
                            | Kind::SkillDistance
                            | Kind::SkillFist
                            | Kind::SkillShield
                            | Kind::SkillSword
                            | Kind::Speed,
                            Known(SignedPoints(_)),
                        ) => true,
                        (
                            Kind::CriticalHitChance
                            | Kind::CriticalHitDamage
                            | Kind::LifeLeechAmount
                            | Kind::LifeLeechChance
                            | Kind::ManaLeechAmount
                            | Kind::ManaLeechChance,
                            Known(RationalPercent(value)),
                        ) => {
                            value.validate().is_ok()
                                && value.numerator >= 0
                                && i128::from(value.numerator)
                                    <= 100 * i128::from(value.denominator)
                        }
                        _ => false,
                    };
                    !parameter
                        || !matches!(&entry.target_domain, Unknown)
                        || !matches!(&entry.evaluation_phase, Unknown)
                        || !matches!(&entry.priority, Unknown)
                })
            {
                return Err(wrong());
            }
            let modifiers = group(
                &mut semantics.skill_modifiers,
                || ReferenceItemSkillModifiers { modifiers: Unknown },
                row,
            )?;
            set_declared(&mut modifiers.modifiers, entries.clone(), row)?
        }
        ("protection.resistances", TypedValue::Resistances(entries)) => {
            if entries.is_empty()
                || entries.len() > REFERENCE_ITEM_MAX_RESISTANCES
                || entries.windows(2).any(|pair| pair[0].kind >= pair[1].kind)
                || entries.iter().any(|entry| {
                    entry.percent.validate().is_err()
                        || i128::from(entry.percent.numerator).abs()
                            > 100 * i128::from(entry.percent.denominator)
                })
            {
                return Err(wrong());
            }
            let value = entries
                .iter()
                .map(|entry| ReferenceResistance {
                    kind: entry.kind,
                    percent: ReferenceItemField::Known(entry.percent),
                })
                .collect();
            set_declared(&mut protection(semantics, row)?.resistances, value, row)?
        }
        _ => return Err(wrong()),
    })
}

// This packet lowers one observed equip pattern, never an arbitrary grammar.
fn qualified_equipment_pattern(pattern: &ReferenceEquipmentPattern) -> bool {
    use ReferenceItemField::{Known, Unknown};
    let Known(primary) = pattern.primary_slot else {
        return false;
    };
    pattern.pattern_id == 1
        && matches!(pattern.mutually_exclusive_groups, Unknown)
        && matches!(pattern.compatibility_rule, Unknown)
        && matches!(pattern.level, Known(_) | Unknown)
        && match &pattern.additional_reserved_slots {
            Unknown => true,
            Known(slots) => {
                slots.is_empty()
                    || (primary == ReferenceEquipmentSlot::Weapon
                        && slots.as_slice() == [ReferenceEquipmentSlot::Shield])
            }
            _ => false,
        }
        && match &pattern.vocations {
            Unknown => true,
            Known(values) => {
                !values.is_empty()
                    && values.len() <= 5
                    && values.windows(2).all(|pair| pair[0] < pair[1])
            }
            _ => false,
        }
}

/// The Known group of `field`, created (all members Unknown) when the group is Unknown.
fn group<'a, T>(
    field: &'a mut ReferenceItemField<T>,
    empty: impl FnOnce() -> T,
    row: &Row,
) -> Result<&'a mut T, ItemStatsPromotionError> {
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

fn weapon<'a>(
    semantics: &'a mut ReferenceItemSemantics,
    row: &Row,
) -> Result<&'a mut ReferenceItemWeapon, ItemStatsPromotionError> {
    group(
        &mut semantics.weapon,
        || ReferenceItemWeapon {
            weapon_type: ReferenceItemField::Unknown,
            attack: ReferenceItemField::Unknown,
            defense: ReferenceItemField::Unknown,
            extra_defense: ReferenceItemField::Unknown,
            range: ReferenceItemField::Unknown,
            hit_chance: ReferenceItemField::Unknown,
            max_hit_chance: ReferenceItemField::Unknown,
            ammunition: ReferenceItemField::Unknown,
            elemental: ReferenceItemField::Unknown,
        },
        row,
    )
}

fn protection<'a>(
    semantics: &'a mut ReferenceItemSemantics,
    row: &Row,
) -> Result<&'a mut ReferenceItemProtection, ItemStatsPromotionError> {
    group(
        &mut semantics.protection,
        || ReferenceItemProtection {
            armor: ReferenceItemField::Unknown,
            resistances: ReferenceItemField::Unknown,
        },
        row,
    )
}

fn imbuement<'a>(
    semantics: &'a mut ReferenceItemSemantics,
    row: &Row,
) -> Result<&'a mut ReferenceItemImbuement, ItemStatsPromotionError> {
    group(
        &mut semantics.imbuement,
        || ReferenceItemImbuement {
            slot_count: ReferenceItemField::Unknown,
            allowed_family_tiers: ReferenceItemField::Unknown,
            excluded_families: ReferenceItemField::Unknown,
        },
        row,
    )
}

fn physical<'a>(
    semantics: &'a mut ReferenceItemSemantics,
    row: &Row,
) -> Result<&'a mut ReferenceItemPhysical, ItemStatsPromotionError> {
    group(
        &mut semantics.physical,
        || ReferenceItemPhysical {
            weight: ReferenceItemField::Unknown,
            movable: ReferenceItemField::Unknown,
            pickupable: ReferenceItemField::Unknown,
        },
        row,
    )
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    const KEY: &str = "oteryn:item.tibia.i34086";

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
    ) -> Result<ItemStatsPromotion, ItemStatsPromotionError> {
        apply_rows(std::iter::once((KEY, semantics)), bytes)
    }

    #[test]
    fn applies_and_replaces_wiki_values() {
        let mut semantics = ReferenceItemSemantics::default();
        weapon(
            &mut semantics,
            &Row {
                item_key: KEY.to_owned(),
                field_path: String::new(),
                typed_value: TypedValue::CountU8(0),
            },
        )
        .expect("weapon group")
        .attack = ReferenceItemField::Known(ReferenceSignedPoints(5));
        let rows = [
            row(
                KEY,
                "weapon.attack",
                r#"{"kind":"SIGNED_POINTS","value":6}"#,
            ),
            row(
                KEY,
                "weapon.weapon_type",
                r#"{"kind":"WEAPON_TYPE","value":"CLUB"}"#,
            ),
            row(
                KEY,
                "weapon.elemental",
                r#"{"kind":"ELEMENTAL_ATTACKS","value":[{"element":"ICE","points":46}]}"#,
            ),
            row(
                KEY,
                "imbuement.slot_count",
                r#"{"kind":"COUNT_U8","value":2}"#,
            ),
            row(
                KEY,
                "physical.weight",
                r#"{"kind":"WEIGHT_CENTI_OZ","value":4100}"#,
            ),
        ];
        let applied = apply(&packet(&rows.join(","), 5, 1), &mut semantics).expect("applies");
        assert_eq!(
            applied,
            ItemStatsPromotion {
                fields: 5,
                items: 1,
                replaced: 1
            }
        );
        let ReferenceItemField::Known(weapon) = &semantics.weapon else {
            panic!("weapon group");
        };
        assert_eq!(
            weapon.attack,
            ReferenceItemField::Known(ReferenceSignedPoints(6))
        );
        assert_eq!(
            weapon.weapon_type,
            ReferenceItemField::Known(ReferenceWeaponType::Club)
        );
        let ReferenceItemField::Known(physical) = &semantics.physical else {
            panic!("physical group");
        };
        assert_eq!(physical.weight, ReferenceItemField::Known(4100));
    }

    #[test]
    fn equipment_is_typed_and_rejects_invalid_claims() {
        let pattern = serde_json::json!({
            "pattern_id": 1,
            "primary_slot": {"state": "KNOWN", "value": "WEAPON"},
            "additional_reserved_slots": {"state": "KNOWN", "value": ["SHIELD"]},
            "mutually_exclusive_groups": {"state": "UNKNOWN"},
            "vocations": {"state": "KNOWN", "value": ["KNIGHT", "PALADIN"]},
            "level": {"state": "KNOWN", "value": 400},
            "compatibility_rule": {"state": "UNKNOWN"}
        });
        let apply_pattern = |value: serde_json::Value| {
            let typed = serde_json::json!({"kind": "EQUIPMENT_PATTERNS", "value": [value]});
            let bytes = packet(&row(KEY, "equipment.patterns", &typed.to_string()), 1, 1);
            let mut semantics = ReferenceItemSemantics::default();
            apply(&bytes, &mut semantics).map(|_| semantics)
        };
        let semantics = apply_pattern(pattern.clone()).expect("qualified pattern");
        assert!(matches!(semantics.equipment, ReferenceItemField::Known(_)));
        for (field, bad) in [
            (
                "primary_slot",
                serde_json::json!({"state": "KNOWN", "value": "AMMO"}),
            ),
            (
                "additional_reserved_slots",
                serde_json::json!({"state": "KNOWN", "value": ["WEAPON"]}),
            ),
            (
                "vocations",
                serde_json::json!({"state": "KNOWN", "value": ["PALADIN", "KNIGHT"]}),
            ),
            (
                "vocations",
                serde_json::json!({"state": "KNOWN", "value": ["KNIGHT", "KNIGHT"]}),
            ),
            (
                "level",
                serde_json::json!({"state": "KNOWN", "value": 65536}),
            ),
            (
                "compatibility_rule",
                serde_json::json!({"state": "KNOWN", "value": null}),
            ),
        ] {
            let mut invalid = pattern.clone();
            invalid[field] = bad;
            assert!(apply_pattern(invalid).is_err(), "{field}");
        }
    }

    #[test]
    fn declared_count_and_milliseconds_preserve_unknowns_and_are_idempotent() {
        let rows = format!(
            "{},{}",
            row(KEY, "charges.count", r#"{"kind":"COUNT_U32","value":100}"#),
            row(
                KEY,
                "temporal.duration",
                r#"{"kind":"DURATION_MS","value":450000}"#
            )
        );
        let bytes = packet(&rows, 2, 1);
        let mut semantics = ReferenceItemSemantics::default();
        for _ in 0..2 {
            assert_eq!(
                apply(&bytes, &mut semantics)
                    .expect("declared values")
                    .replaced,
                0
            );
        }
        let expected = ReferenceItemSemantics {
            charges: ReferenceItemField::Known(ReferenceItemCharges {
                count: ReferenceItemField::Known(100),
            }),
            temporal: ReferenceItemField::Known(ReferenceItemTemporal {
                consumption_mode: ReferenceItemField::Unknown,
                duration: ReferenceItemField::Known(ReferenceMilliseconds(450000)),
                stop_duration: ReferenceItemField::Unknown,
                decay_target: ReferenceItemField::Unknown,
            }),
            ..ReferenceItemSemantics::default()
        };
        assert_eq!(semantics, expected);
    }

    fn modifier_value() -> serde_json::Value {
        serde_json::json!({"kind":"MODIFIERS", "value":[
            {"kind":"MAGIC_LEVEL_POINTS", "target_domain":{"state":"UNKNOWN"},
             "evaluation_phase":{"state":"UNKNOWN"}, "priority":{"state":"UNKNOWN"},
             "parameter":{"state":"KNOWN", "value":{"kind":"SIGNED_POINTS", "value":1}}},
            {"kind":"MANA_LEECH_AMOUNT", "target_domain":{"state":"UNKNOWN"},
             "evaluation_phase":{"state":"UNKNOWN"}, "priority":{"state":"UNKNOWN"},
             "parameter":{"state":"KNOWN", "value":{"kind":"RATIONAL_PERCENT", "value":{"numerator":1,"denominator":8}}}}
        ]})
    }

    #[test]
    fn modifier_metadata_preserves_unknown_context_and_other_groups_idempotently() {
        let value = modifier_value();
        let bytes = packet(
            &row(KEY, "skill_modifiers.modifiers", &value.to_string()),
            1,
            1,
        );
        let mut semantics = ReferenceItemSemantics::default();
        let before = semantics.clone();
        apply(&bytes, &mut semantics).expect("qualified source metadata");
        let expected = semantics.clone();
        assert_eq!(
            apply(&bytes, &mut semantics)
                .expect("idempotent metadata")
                .replaced,
            0
        );
        assert_eq!(semantics, expected);
        let ReferenceItemField::Known(group) = &semantics.skill_modifiers else {
            panic!("modifier group")
        };
        let ReferenceItemField::Known(entries) = &group.modifiers else {
            panic!("modifier vector")
        };
        assert_eq!(
            serde_json::to_value(entries).expect("metadata serialization"),
            value["value"]
        );
        semantics.skill_modifiers = ReferenceItemField::Unknown;
        assert_eq!(semantics, before);
    }

    #[test]
    fn modifier_wrong_context_parameter_order_or_late_row_is_atomic() {
        let valid = modifier_value();
        let mut invalid = vec![serde_json::json!({"kind":"MODIFIERS", "value":[]})];
        for field in ["target_domain", "evaluation_phase", "priority"] {
            let mut value = valid.clone();
            value["value"][0][field] = serde_json::json!({"state":"KNOWN", "value":1});
            invalid.push(value);
        }
        let mut reversed = valid.clone();
        reversed["value"].as_array_mut().expect("array").reverse();
        invalid.push(reversed);
        let mut duplicate = valid.clone();
        duplicate["value"][1] = duplicate["value"][0].clone();
        invalid.push(duplicate);
        for parameter in [
            serde_json::json!({"state":"UNKNOWN"}),
            serde_json::json!({"state":"KNOWN", "value":{"kind":"SIGNED_POINTS", "value":1}}),
            serde_json::json!({"state":"KNOWN", "value":{"kind":"RATIONAL_PERCENT", "value":{"numerator":2,"denominator":2}}}),
            serde_json::json!({"state":"KNOWN", "value":{"kind":"RATIONAL_PERCENT", "value":{"numerator":101,"denominator":1}}}),
        ] {
            let mut value = valid.clone();
            value["value"][1]["parameter"] = parameter;
            invalid.push(value);
        }
        for value in invalid {
            let rows = format!(
                "{},{}",
                row(
                    KEY,
                    "physical.weight",
                    r#"{"kind":"WEIGHT_CENTI_OZ","value":4100}"#
                ),
                row(KEY, "skill_modifiers.modifiers", &value.to_string())
            );
            let bytes = packet(&rows, 2, 1);
            let mut semantics = ReferenceItemSemantics::default();
            let before = semantics.clone();
            assert!(apply(&bytes, &mut semantics).is_err());
            assert_eq!(semantics, before);
        }
    }

    #[test]
    fn elemental_magic_points_use_existing_atomic_metadata_guards() {
        for kind in [
            "DEATH_MAGIC_LEVEL_POINTS",
            "EARTH_MAGIC_LEVEL_POINTS",
            "ENERGY_MAGIC_LEVEL_POINTS",
            "FIRE_MAGIC_LEVEL_POINTS",
            "HEALING_MAGIC_LEVEL_POINTS",
            "HOLY_MAGIC_LEVEL_POINTS",
            "ICE_MAGIC_LEVEL_POINTS",
        ] {
            let mut value = modifier_value();
            value["value"][0]["kind"] = serde_json::json!(kind);
            let bytes = packet(
                &row(KEY, "skill_modifiers.modifiers", &value.to_string()),
                1,
                1,
            );
            let mut semantics = ReferenceItemSemantics::default();
            apply(&bytes, &mut semantics).expect("existing signed-point kind");
            let before = semantics.clone();
            assert!(apply(&bytes, &mut semantics).is_ok());
            assert_eq!(semantics, before);
            value["value"][0]["parameter"] = value["value"][1]["parameter"].clone();
            let wrong = packet(
                &row(KEY, "skill_modifiers.modifiers", &value.to_string()),
                1,
                1,
            );
            assert!(apply(&wrong, &mut semantics).is_err());
            assert_eq!(semantics, before);
        }
    }

    #[test]
    fn modifier_existing_known_or_blocked_vector_never_overwrites() {
        let value = modifier_value();
        let bytes = packet(
            &row(KEY, "skill_modifiers.modifiers", &value.to_string()),
            1,
            1,
        );
        let mut semantics = ReferenceItemSemantics::default();
        apply(&bytes, &mut semantics).expect("qualified metadata");
        let ReferenceItemField::Known(group) = &mut semantics.skill_modifiers else {
            panic!("group")
        };
        let ReferenceItemField::Known(entries) = &mut group.modifiers else {
            panic!("vector")
        };
        entries[0].parameter = ReferenceItemField::Known(ReferenceModifierParameter::SignedPoints(
            ReferenceSignedPoints(2),
        ));
        let before = semantics.clone();
        assert!(apply(&bytes, &mut semantics).is_err());
        assert_eq!(semantics, before);
        for state in [
            ReferenceItemField::Conflict,
            ReferenceItemField::NotApplicable,
        ] {
            semantics.skill_modifiers =
                ReferenceItemField::Known(ReferenceItemSkillModifiers { modifiers: state });
            let before = semantics.clone();
            assert!(apply(&bytes, &mut semantics).is_err());
            assert_eq!(semantics, before);
        }
    }

    fn resistance_value() -> serde_json::Value {
        serde_json::json!({"kind":"RESISTANCES", "value":[
            {"kind":"FIRE", "percent":{"numerator":5,"denominator":1}},
            {"kind":"ICE", "percent":{"numerator":-3,"denominator":2}}
        ]})
    }

    fn resistance_packet(value: &serde_json::Value) -> Vec<u8> {
        packet(
            &row(KEY, "protection.resistances", &value.to_string()),
            1,
            1,
        )
    }

    #[test]
    fn resistance_points_preserve_armor_unknowns_and_are_idempotent() {
        let bytes = resistance_packet(&resistance_value());
        let expected_values = serde_json::json!({"state":"KNOWN","value":[
            {"kind":"FIRE","percent":{"state":"KNOWN","value":{"numerator":5,"denominator":1}}},
            {"kind":"ICE","percent":{"state":"KNOWN","value":{"numerator":-3,"denominator":2}}}
        ]});
        for armor in [
            ReferenceItemField::Unknown,
            ReferenceItemField::Known(ReferenceSignedPoints(17)),
        ] {
            let mut semantics = ReferenceItemSemantics {
                protection: ReferenceItemField::Known(ReferenceItemProtection {
                    armor: armor.clone(),
                    resistances: ReferenceItemField::Unknown,
                }),
                ..ReferenceItemSemantics::default()
            };
            let mut expected = semantics.clone();
            expected.protection = ReferenceItemField::Known(ReferenceItemProtection {
                armor,
                resistances: serde_json::from_value(expected_values.clone())
                    .expect("native expected percentages"),
            });
            for _ in 0..2 {
                assert_eq!(
                    apply(&bytes, &mut semantics)
                        .expect("qualified resistance points")
                        .replaced,
                    0
                );
                assert_eq!(semantics, expected);
            }
        }
    }

    #[test]
    fn resistance_shape_order_rationals_and_bounds_fail_closed() {
        let valid = resistance_value();
        let mut invalid = vec![serde_json::json!({"kind":"RESISTANCES", "value":[]})];
        let mut reversed = valid.clone();
        reversed["value"].as_array_mut().expect("array").reverse();
        invalid.push(reversed);
        let mut duplicate = valid.clone();
        duplicate["value"][1] = duplicate["value"][0].clone();
        invalid.push(duplicate);
        let mut oversized = valid.clone();
        oversized["value"] = serde_json::json!(vec![valid["value"][0].clone(); 13]);
        invalid.push(oversized);
        for percent in [
            serde_json::json!({"numerator":1,"denominator":0}),
            serde_json::json!({"numerator":2,"denominator":2}),
            serde_json::json!({"numerator":0,"denominator":2}),
            serde_json::json!({"numerator":101,"denominator":1}),
            serde_json::json!({"numerator":-101,"denominator":1}),
            serde_json::json!({"numerator":1,"denominator":-1}),
            serde_json::json!({"numerator":9223372036854775808u64,"denominator":1}),
            serde_json::json!({"numerator":1,"denominator":1,"extra":0}),
        ] {
            let mut value = valid.clone();
            value["value"][0]["percent"] = percent;
            invalid.push(value);
        }
        let mut unknown_kind = valid.clone();
        unknown_kind["value"][0]["kind"] = serde_json::json!("CRITICAL_HIT_CHANCE");
        invalid.push(unknown_kind);
        let mut unknown_field = valid.clone();
        unknown_field["value"][0]["extra"] = serde_json::json!(0);
        invalid.push(unknown_field);
        for value in invalid {
            let mut semantics = ReferenceItemSemantics::default();
            assert!(
                apply(&resistance_packet(&value), &mut semantics).is_err(),
                "{value}"
            );
            assert!(semantics.is_all_unknown());
        }
    }

    #[test]
    fn resistance_conflicts_and_late_errors_reject_without_partial_mutation() {
        let bytes = packet(
            &format!(
                "{},{}",
                row(
                    KEY,
                    "weapon.attack",
                    r#"{"kind":"SIGNED_POINTS","value":6}"#
                ),
                row(
                    KEY,
                    "protection.resistances",
                    &resistance_value().to_string()
                )
            ),
            2,
            1,
        );
        for blocked in [
            serde_json::json!({"state":"CONFLICT"}),
            serde_json::json!({"state":"NOT_APPLICABLE"}),
            serde_json::json!({"state":"KNOWN","value":[{"kind":"FIRE","percent":{"state":"UNKNOWN"}}]}),
            serde_json::json!({"state":"KNOWN","value":[{"kind":"FIRE","percent":{"state":"KNOWN","value":{"numerator":4,"denominator":1}}}]}),
        ] {
            let resistances = serde_json::from_value(blocked).expect("native blocked fixture");
            let mut semantics = ReferenceItemSemantics {
                protection: ReferenceItemField::Known(ReferenceItemProtection {
                    armor: ReferenceItemField::Known(ReferenceSignedPoints(17)),
                    resistances,
                }),
                ..ReferenceItemSemantics::default()
            };
            let before = semantics.clone();
            assert!(apply(&bytes, &mut semantics).is_err());
            assert_eq!(semantics, before);
        }
        for protection in [
            ReferenceItemField::Conflict,
            ReferenceItemField::NotApplicable,
        ] {
            let mut semantics = ReferenceItemSemantics {
                protection,
                ..ReferenceItemSemantics::default()
            };
            let before = semantics.clone();
            assert!(apply(&bytes, &mut semantics).is_err());
            assert_eq!(semantics, before);
        }
        let first = row(
            KEY,
            "weapon.attack",
            r#"{"kind":"SIGNED_POINTS","value":6}"#,
        );
        for second in [
            row(
                KEY,
                "protection.resistances",
                r#"{"kind":"RESISTANCES","value":[]}"#,
            ),
            row(
                "oteryn:item.test.missing",
                "protection.resistances",
                &resistance_value().to_string(),
            ),
        ] {
            let mut semantics = ReferenceItemSemantics::default();
            let item_count = 1 + usize::from(second.contains("item.test.missing\""));
            assert!(
                apply(
                    &packet(&format!("{first},{second}"), 2, item_count),
                    &mut semantics
                )
                .is_err()
            );
            assert!(semantics.is_all_unknown());
        }
    }

    #[test]
    fn declared_values_reject_wrong_tags_and_numeric_bounds_without_mutation() {
        for (path, typed) in [
            ("charges.count", r#"{"kind":"COUNT_U8","value":100}"#),
            (
                "temporal.duration",
                r#"{"kind":"COUNT_U32","value":450000}"#,
            ),
            (
                "charges.count",
                r#"{"kind":"COUNT_U32","value":4294967296}"#,
            ),
            ("temporal.duration", r#"{"kind":"DURATION_MS","value":-1}"#),
            (
                "temporal.duration",
                r#"{"kind":"DURATION_SECONDS","value":450}"#,
            ),
        ] {
            let mut semantics = ReferenceItemSemantics::default();
            assert!(apply(&packet(&row(KEY, path, typed), 1, 1), &mut semantics).is_err());
            assert!(semantics.is_all_unknown());
        }
    }

    #[test]
    fn declared_fields_hold_blocked_groups_leaves_and_conflicting_known_values() {
        let charge = packet(
            &row(KEY, "charges.count", r#"{"kind":"COUNT_U32","value":100}"#),
            1,
            1,
        );
        let duration = packet(
            &row(
                KEY,
                "temporal.duration",
                r#"{"kind":"DURATION_MS","value":450000}"#,
            ),
            1,
            1,
        );
        for blocked in [
            ReferenceItemField::Conflict,
            ReferenceItemField::NotApplicable,
            ReferenceItemField::Known(99),
        ] {
            let mut semantics = ReferenceItemSemantics {
                charges: ReferenceItemField::Known(ReferenceItemCharges { count: blocked }),
                ..ReferenceItemSemantics::default()
            };
            let before = semantics.clone();
            assert!(apply(&charge, &mut semantics).is_err());
            assert_eq!(semantics, before);
        }
        for blocked in [
            ReferenceItemField::Conflict,
            ReferenceItemField::NotApplicable,
            ReferenceItemField::Known(ReferenceMilliseconds(449999)),
        ] {
            let mut semantics = ReferenceItemSemantics {
                temporal: ReferenceItemField::Known(ReferenceItemTemporal {
                    consumption_mode: ReferenceItemField::Unknown,
                    duration: blocked,
                    stop_duration: ReferenceItemField::Known(true),
                    decay_target: ReferenceItemField::Unknown,
                }),
                ..ReferenceItemSemantics::default()
            };
            let before = semantics.clone();
            assert!(apply(&duration, &mut semantics).is_err());
            assert_eq!(semantics, before);
        }
        for in_conflict in [false, true] {
            let mut semantics = ReferenceItemSemantics {
                charges: if in_conflict {
                    ReferenceItemField::Conflict
                } else {
                    ReferenceItemField::NotApplicable
                },
                temporal: if in_conflict {
                    ReferenceItemField::Conflict
                } else {
                    ReferenceItemField::NotApplicable
                },
                ..ReferenceItemSemantics::default()
            };
            let before = semantics.clone();
            assert!(apply(&charge, &mut semantics).is_err());
            assert!(apply(&duration, &mut semantics).is_err());
            assert_eq!(semantics, before);
        }
    }

    #[test]
    fn rejects_bad_rows() {
        let attack = row(
            KEY,
            "weapon.attack",
            r#"{"kind":"SIGNED_POINTS","value":6}"#,
        );
        let cases = [
            // duplicate field for one item
            packet(&format!("{attack},{attack}"), 2, 1),
            // unknown Item key
            packet(
                &row(
                    concat!("oteryn:item.tibia.", "i1"),
                    "weapon.attack",
                    r#"{"kind":"SIGNED_POINTS","value":6}"#,
                ),
                1,
                1,
            ),
            // value kind that does not fit the field
            packet(
                &row(KEY, "weapon.attack", r#"{"kind":"COUNT_U8","value":6}"#),
                1,
                1,
            ),
            // unknown field path
            packet(
                &row(
                    KEY,
                    "weapon.hit_chance",
                    r#"{"kind":"SIGNED_POINTS","value":6}"#,
                ),
                1,
                1,
            ),
            // closed enum
            packet(
                &row(
                    KEY,
                    "weapon.weapon_type",
                    r#"{"kind":"WEAPON_TYPE","value":"ROD"}"#,
                ),
                1,
                1,
            ),
            // bounds
            packet(
                &row(
                    KEY,
                    "imbuement.slot_count",
                    r#"{"kind":"COUNT_U8","value":300}"#,
                ),
                1,
                1,
            ),
            // repeated element
            packet(
                &row(
                    KEY,
                    "weapon.elemental",
                    r#"{"kind":"ELEMENTAL_ATTACKS","value":[{"element":"ICE","points":1},{"element":"ICE","points":2}]}"#,
                ),
                1,
                1,
            ),
            // counts that do not match the rows
            packet(&attack, 2, 1),
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
    fn pinned_packet_decodes_with_its_counts() {
        assert_eq!(
            world_project_sha256(ITEM_STATS_PROMOTION_V2_PACKET),
            ITEM_STATS_PROMOTION_V2_PACKET_SHA256
        );
        let packet: Packet =
            serde_json::from_slice(ITEM_STATS_PROMOTION_V2_PACKET).expect("pinned packet decodes");
        assert_eq!(packet.counts.fields, ITEM_STATS_PROMOTION_V2_FIELD_COUNT);
        assert_eq!(packet.counts.items, ITEM_STATS_PROMOTION_V2_ITEM_COUNT);
        assert_eq!(packet.promotions.len(), ITEM_STATS_PROMOTION_V2_FIELD_COUNT);
    }
    #[test]
    fn mantra_bond_metadata_is_idempotent_with_unknown_context() {
        for (kind, parameter) in [
            (
                "MANTRA",
                serde_json::json!({"kind":"SIGNED_POINTS", "value":-32768}),
            ),
            (
                "MANTRA",
                serde_json::json!({"kind":"SIGNED_POINTS", "value":32767}),
            ),
            (
                "ELEMENTAL_BOND",
                serde_json::json!({"kind":"ELEMENT", "value":"EARTH"}),
            ),
            (
                "ELEMENTAL_BOND",
                serde_json::json!({"kind":"ELEMENT", "value":"ENERGY"}),
            ),
        ] {
            let mut value = modifier_value();
            value["value"] = serde_json::json!([value["value"][0]]);
            value["value"][0]["kind"] = serde_json::json!(kind);
            value["value"][0]["parameter"]["value"] = parameter;
            let bytes = packet(
                &row(KEY, "skill_modifiers.modifiers", &value.to_string()),
                1,
                1,
            );
            let mut semantics = ReferenceItemSemantics::default();
            apply(&bytes, &mut semantics).expect("source-qualified typed metadata");
            let before = semantics.clone();
            assert_eq!(
                apply(&bytes, &mut semantics)
                    .expect("same-vector idempotence")
                    .replaced,
                0
            );
            assert_eq!(semantics, before);
            assert_eq!(
                serde_json::to_value(&semantics.skill_modifiers).expect("metadata")["value"]["modifiers"]
                    ["value"],
                value["value"]
            );
            semantics.skill_modifiers = ReferenceItemField::Unknown;
            assert_eq!(semantics, ReferenceItemSemantics::default());
        }
    }

    #[test]
    fn mantra_bond_wrong_unit_bounds_element_and_late_conflict_are_atomic() {
        for (kind, parameter) in [
            (
                "MANTRA",
                serde_json::json!({"kind":"SIGNED_POINTS", "value":32768}),
            ),
            (
                "MANTRA",
                serde_json::json!({"kind":"SIGNED_POINTS", "value":-32769}),
            ),
            (
                "MANTRA",
                serde_json::json!({"kind":"RATIONAL_PERCENT", "value":{"numerator":1,"denominator":1}}),
            ),
            (
                "ELEMENTAL_BOND",
                serde_json::json!({"kind":"SIGNED_POINTS", "value":1}),
            ),
            (
                "ELEMENTAL_BOND",
                serde_json::json!({"kind":"ELEMENT", "value":"FIRE"}),
            ),
            (
                "ELEMENTAL_BOND",
                serde_json::json!({"kind":"ELEMENT", "value":"PHYSICAL"}),
            ),
        ] {
            let mut value = modifier_value();
            value["value"] = serde_json::json!([value["value"][0]]);
            value["value"][0]["kind"] = serde_json::json!(kind);
            value["value"][0]["parameter"]["value"] = parameter;
            let rows = format!(
                "{},{}",
                row(
                    KEY,
                    "physical.weight",
                    r#"{"kind":"WEIGHT_CENTI_OZ","value":4100}"#
                ),
                row(KEY, "skill_modifiers.modifiers", &value.to_string())
            );
            let bytes = packet(&rows, 2, 1);
            let mut semantics = ReferenceItemSemantics::default();
            let before = semantics.clone();
            assert!(apply(&bytes, &mut semantics).is_err());
            assert_eq!(semantics, before);
        }
    }
}
