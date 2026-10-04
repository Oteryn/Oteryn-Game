//! Typed Equipment abilities (EQUIP-CONTENT-1; EQUIP-0 §3.1-§3.2, architect bundle §2.9).
//!
//! The six abilities of EQUIP-0 §3.1 are a derived view over the existing `skill_modifiers` and
//! `protection` groups of an Item definition: no codec or schema change (control-plane ruling a).
//! `timed` is derived from `charges.count` or `temporal.duration`, and the Extra slot from an
//! Equipment pattern whose `primary_slot` is Extra; neither has a flag of its own.
//!
//! The pinned record `docs/agents/evidence/OTV2-20261003-equip-abilities-v1.json` is written by
//! `tools/content-schema/item-authoring` from TibiaWiki first. Where every wiki page of an Item is
//! silent on a group, it carries a Canary `items.xml` fallback row (D384 pin, OTS_HYPOTHESIS_ONLY;
//! speed is in displayed units 1:1). The materializer applies it once, last: a fallback row
//! fills an Unknown leaf only; the listed `timed` flag must equal the derived one; and every Item
//! with a derived ability must be listed with its sources. Anything else fails closed.
//!
//! STAT_BOOST and LIGHT have the type but no rows: neither the TibiaWiki stats snapshot nor Canary
//! `items.xml` (D384 pin) is a source for them (a follow-up question for EQUIP-RT-1).

use super::{
    ProjectReferenceRecord, ProjectV2Draft, ReferenceEquipmentSlot, ReferenceItemField,
    ReferenceItemProtection, ReferenceItemSemantics, ReferenceItemSkillModifiers,
    ReferenceModifierBinding, ReferenceModifierParameter, ReferenceRationalPercent,
    ReferenceResistance, ReferenceResistanceKind, ReferenceSignedPoints,
    ReferenceSkillModifierKind, world_project_sha256,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

/// The pinned record bytes; any change is a new candidate with a new digest.
pub const EQUIP_ABILITIES_V1_RECORD: &[u8] =
    include_bytes!("../../../../docs/agents/evidence/OTV2-20261003-equip-abilities-v1.json");
pub const EQUIP_ABILITIES_V1_RECORD_SHA256: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";
pub const EQUIP_ABILITIES_V1_ITEM_COUNT: usize = 0;
pub const EQUIP_ABILITIES_V1_FALLBACK_FIELD_COUNT: usize = 0;
pub const EQUIP_ABILITIES_V1_TIMED_ITEM_COUNT: usize = 0;
const SCHEMA: &str = "OTERYN_EQUIP_ABILITIES/v1";
const MODIFIERS: &str = "skill_modifiers.modifiers";
const RESISTANCES: &str = "protection.resistances";

/// The weapon skills and magic level a SKILL_BOOST raises (EQUIP-0 §3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AbilitySkill {
    Axe,
    Club,
    Distance,
    Fist,
    Shield,
    Sword,
    MagicLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AbilityStat {
    MaxHealth,
    MaxMana,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AbilityAmount {
    Flat(i32),
    Percent(ReferenceRationalPercent),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AbilitySuppression {
    Drown,
    Drunk,
}

/// One EQUIP-0 §3.1 ability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ItemAbility {
    SkillBoost {
        skill: AbilitySkill,
        points: i32,
    },
    /// Typed, without rows in v1 (no source in the TibiaWiki snapshot nor Canary items.xml).
    StatBoost {
        stat: AbilityStat,
        amount: AbilityAmount,
    },
    /// Signed, in displayed speed units.
    Speed(i32),
    /// Signed percent in [-100, 100]; `element` is never `FireField`, which is `Fire` with
    /// `field_only`.
    Protection {
        element: ReferenceResistanceKind,
        percent: ReferenceRationalPercent,
        field_only: bool,
    },
    Suppress(AbilitySuppression),
    /// Typed, without rows in v1 (no source in the TibiaWiki snapshot nor Canary items.xml).
    Light {
        level: u8,
        color: u8,
    },
}

/// The derived ability view of one Item definition.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ItemAbilityProfile {
    /// `charges.count` or `temporal.duration` is Known (EQUIP-0 §3.2).
    pub timed: bool,
    /// An Equipment pattern has `primary_slot == Extra`.
    pub extra_slot: bool,
    pub abilities: Vec<ItemAbility>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemAbilityError {
    Digest,
    Decode(String),
    Item {
        item_key: String,
        reason: &'static str,
    },
    Counts,
}

impl std::fmt::Display for ItemAbilityError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Digest => formatter.write_str("equip abilities record digest mismatch"),
            Self::Decode(error) => write!(formatter, "equip abilities record: {error}"),
            Self::Item { item_key, reason } => {
                write!(formatter, "equip abilities {item_key}: {reason}")
            }
            Self::Counts => formatter.write_str("equip abilities counts drifted"),
        }
    }
}

impl std::error::Error for ItemAbilityError {}

fn item_error(item_key: &str, reason: &'static str) -> ItemAbilityError {
    ItemAbilityError::Item {
        item_key: item_key.to_owned(),
        reason,
    }
}

/// Derive the ability view; an ability-kind entry without a Known, well-typed value fails closed.
pub fn item_ability_profile(
    semantics: &ReferenceItemSemantics,
) -> Result<ItemAbilityProfile, &'static str> {
    let timed = matches!(&semantics.charges, ReferenceItemField::Known(charges)
            if matches!(charges.count, ReferenceItemField::Known(_)))
        || matches!(&semantics.temporal, ReferenceItemField::Known(temporal)
            if matches!(temporal.duration, ReferenceItemField::Known(_)));
    let extra_slot = matches!(&semantics.equipment, ReferenceItemField::Known(equipment)
    if matches!(&equipment.patterns, ReferenceItemField::Known(patterns)
        if patterns.iter().any(|pattern| {
            pattern.primary_slot == ReferenceItemField::Known(ReferenceEquipmentSlot::Extra)
        })));
    let mut abilities = Vec::new();
    if let ReferenceItemField::Known(group) = &semantics.skill_modifiers
        && let ReferenceItemField::Known(modifiers) = &group.modifiers
    {
        for modifier in modifiers {
            if let Some(ability) = modifier_ability(modifier)? {
                abilities.push(ability);
            }
        }
    }
    if let ReferenceItemField::Known(group) = &semantics.protection
        && let ReferenceItemField::Known(resistances) = &group.resistances
    {
        for resistance in resistances {
            abilities.push(resistance_ability(resistance)?);
        }
    }
    abilities.sort();
    Ok(ItemAbilityProfile {
        timed,
        extra_slot,
        abilities,
    })
}

fn modifier_ability(
    modifier: &ReferenceModifierBinding,
) -> Result<Option<ItemAbility>, &'static str> {
    use ReferenceSkillModifierKind as Kind;
    let skill = match modifier.kind {
        Kind::SkillAxe => Some(AbilitySkill::Axe),
        Kind::SkillClub => Some(AbilitySkill::Club),
        Kind::SkillDistance => Some(AbilitySkill::Distance),
        Kind::SkillFist => Some(AbilitySkill::Fist),
        Kind::SkillShield => Some(AbilitySkill::Shield),
        Kind::SkillSword => Some(AbilitySkill::Sword),
        Kind::MagicLevelPoints => Some(AbilitySkill::MagicLevel),
        Kind::Speed | Kind::SuppressDrown | Kind::SuppressDrunk => None,
        // Not an EQUIP-0 §3.1 ability: other systems own these modifiers.
        _ => return Ok(None),
    };
    let points = || match &modifier.parameter {
        ReferenceItemField::Known(ReferenceModifierParameter::SignedPoints(
            ReferenceSignedPoints(points),
        )) => Ok(*points),
        _ => Err("an ability modifier needs Known signed points"),
    };
    Ok(Some(match (skill, modifier.kind) {
        (Some(skill), _) => ItemAbility::SkillBoost {
            skill,
            points: points()?,
        },
        (None, Kind::Speed) => ItemAbility::Speed(points()?),
        (None, kind) => {
            if modifier.parameter
                != ReferenceItemField::Known(ReferenceModifierParameter::Boolean(true))
            {
                return Err("a suppression needs Known true");
            }
            ItemAbility::Suppress(if kind == Kind::SuppressDrown {
                AbilitySuppression::Drown
            } else {
                AbilitySuppression::Drunk
            })
        }
    }))
}

fn resistance_ability(resistance: &ReferenceResistance) -> Result<ItemAbility, &'static str> {
    let ReferenceItemField::Known(percent) = resistance.percent else {
        return Err("a protection needs a Known percent");
    };
    // The rational is canonical, so |numerator| <= 100 * denominator bounds it to [-100, 100].
    if percent.numerator.unsigned_abs() > percent.denominator.saturating_mul(100) {
        return Err("a protection percent is outside [-100, 100]");
    }
    let (element, field_only) = match resistance.kind {
        ReferenceResistanceKind::FireField => (ReferenceResistanceKind::Fire, true),
        kind => (kind, false),
    };
    Ok(ItemAbility::Protection {
        element,
        percent,
        field_only,
    })
}

/// What one application changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EquipAbilities {
    pub items: usize,
    pub fallback_fields: usize,
    pub timed_items: usize,
    /// Item records whose derived view holds at least one ability.
    pub ability_items: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    schema: String,
    #[serde(rename = "policy")]
    _policy: serde_json::Value,
    #[serde(rename = "source")]
    _source: serde_json::Value,
    counts: Counts,
    items: Vec<ListedItem>,
    #[serde(rename = "holds")]
    _holds: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct Counts {
    items: usize,
    timed_items: usize,
    fallback_items: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ListedItem {
    item_key: String,
    timed: bool,
    sources: Vec<serde_json::Value>,
    fallback: Vec<FallbackRow>,
}

#[derive(Deserialize)]
#[serde(tag = "field_path", content = "value", deny_unknown_fields)]
enum FallbackRow {
    #[serde(rename = "skill_modifiers.modifiers")]
    Modifiers(Vec<ReferenceModifierBinding>),
    #[serde(rename = "protection.resistances")]
    Resistances(Vec<ReferenceResistance>),
}

/// Apply the pinned record to every Item record in `draft`.
pub fn apply_equip_abilities_v1(
    draft: &mut ProjectV2Draft,
) -> Result<EquipAbilities, ItemAbilityError> {
    if world_project_sha256(EQUIP_ABILITIES_V1_RECORD) != EQUIP_ABILITIES_V1_RECORD_SHA256 {
        return Err(ItemAbilityError::Digest);
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
    let applied = apply_record(items, EQUIP_ABILITIES_V1_RECORD)?;
    if applied.items != EQUIP_ABILITIES_V1_ITEM_COUNT
        || applied.fallback_fields != EQUIP_ABILITIES_V1_FALLBACK_FIELD_COUNT
        || applied.timed_items != EQUIP_ABILITIES_V1_TIMED_ITEM_COUNT
    {
        return Err(ItemAbilityError::Counts);
    }
    Ok(applied)
}

fn apply_record<'a>(
    items: impl Iterator<Item = (&'a str, &'a mut ReferenceItemSemantics)>,
    bytes: &[u8],
) -> Result<EquipAbilities, ItemAbilityError> {
    let record: Record = serde_json::from_slice(bytes)
        .map_err(|error| ItemAbilityError::Decode(error.to_string()))?;
    if record.schema != SCHEMA {
        return Err(ItemAbilityError::Decode("schema".to_owned()));
    }
    let mut listed = BTreeMap::new();
    for item in &record.items {
        if item.sources.is_empty() {
            return Err(item_error(&item.item_key, "a listed Item has no source"));
        }
        if listed.insert(item.item_key.as_str(), item).is_some() {
            return Err(item_error(&item.item_key, "duplicate Item"));
        }
    }
    if record.counts.items != listed.len()
        || record.counts.timed_items != record.items.iter().filter(|item| item.timed).count()
        || record.counts.fallback_items
            != record
                .items
                .iter()
                .filter(|item| !item.fallback.is_empty())
                .count()
    {
        return Err(ItemAbilityError::Counts);
    }

    let mut applied = EquipAbilities {
        items: 0,
        fallback_fields: 0,
        timed_items: 0,
        ability_items: 0,
    };
    let mut seen = BTreeSet::new();
    for (key, semantics) in items {
        if let Some(item) = listed.get(key) {
            seen.insert(key.to_owned());
            for row in &item.fallback {
                apply_fallback(semantics, row).map_err(|reason| item_error(key, reason))?;
                applied.fallback_fields += 1;
            }
        }
        let profile = item_ability_profile(semantics).map_err(|reason| item_error(key, reason))?;
        match listed.get(key) {
            Some(item) => {
                if item.timed != profile.timed {
                    return Err(item_error(
                        key,
                        "the listed timed flag differs from the derived one",
                    ));
                }
                applied.items += 1;
                applied.timed_items += usize::from(profile.timed);
            }
            None if !profile.abilities.is_empty() => {
                return Err(item_error(
                    key,
                    "an Item with abilities is not listed with its sources",
                ));
            }
            None => {}
        }
        applied.ability_items += usize::from(!profile.abilities.is_empty());
    }
    if let Some(key) = listed.keys().find(|key| !seen.contains(**key)) {
        return Err(item_error(key, "no Item record with this key"));
    }
    Ok(applied)
}

/// Fill one Unknown leaf from a Canary fallback row; any known evidence state fails closed.
fn apply_fallback(
    semantics: &mut ReferenceItemSemantics,
    row: &FallbackRow,
) -> Result<(), &'static str> {
    match row {
        FallbackRow::Modifiers(modifiers) => {
            if modifiers.is_empty() {
                return Err("an empty fallback row");
            }
            for modifier in modifiers {
                if modifier_ability(modifier)?.is_none() {
                    return Err("a fallback modifier is not an EQUIP-0 ability");
                }
            }
            let group = match &mut semantics.skill_modifiers {
                field @ ReferenceItemField::Unknown => {
                    *field = ReferenceItemField::Known(ReferenceItemSkillModifiers {
                        modifiers: ReferenceItemField::Unknown,
                    });
                    let ReferenceItemField::Known(group) = field else {
                        unreachable!()
                    };
                    group
                }
                ReferenceItemField::Known(group) => group,
                _ => return Err("the fallback group holds another evidence state"),
            };
            if !group.modifiers.is_unknown() {
                return Err("a fallback row targets a leaf that is not Unknown");
            }
            group.modifiers = ReferenceItemField::Known(modifiers.clone());
        }
        FallbackRow::Resistances(resistances) => {
            if resistances.is_empty() {
                return Err("an empty fallback row");
            }
            for resistance in resistances {
                if let ReferenceItemField::Known(percent) = resistance.percent {
                    percent
                        .validate()
                        .map_err(|_| "a fallback percent is not canonical")?;
                }
                resistance_ability(resistance)?;
            }
            let group = match &mut semantics.protection {
                field @ ReferenceItemField::Unknown => {
                    *field = ReferenceItemField::Known(ReferenceItemProtection {
                        armor: ReferenceItemField::Unknown,
                        resistances: ReferenceItemField::Unknown,
                    });
                    let ReferenceItemField::Known(group) = field else {
                        unreachable!()
                    };
                    group
                }
                ReferenceItemField::Known(group) => group,
                _ => return Err("the fallback group holds another evidence state"),
            };
            if !group.resistances.is_unknown() {
                return Err("a fallback row targets a leaf that is not Unknown");
            }
            group.resistances = ReferenceItemField::Known(resistances.clone());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{ReferenceEquipmentPattern, ReferenceItemCharges, ReferenceItemEquipment};

    const KEY: &str = "oteryn:item.tibia.i3087";

    fn record(items: &str, listed: usize, timed: usize, fallback: usize) -> Vec<u8> {
        format!(
            r#"{{"schema":"{SCHEMA}","policy":{{}},"source":{{}},"counts":{{"items":{listed},"timed_items":{timed},"fallback_items":{fallback},"holds":0}},"items":[{items}],"holds":[]}}"#
        )
        .into_bytes()
    }

    fn listed(timed: bool, fallback: &str) -> String {
        format!(
            r#"{{"item_key":"{KEY}","timed":{timed},"sources":[{{"class":"OTS_HYPOTHESIS_ONLY"}}],"fallback":[{fallback}]}}"#
        )
    }

    const FIST: &str = r#"{"field_path":"skill_modifiers.modifiers","value":[{"evaluation_phase":{"state":"UNKNOWN"},"kind":"SKILL_FIST","parameter":{"state":"KNOWN","value":{"kind":"SIGNED_POINTS","value":6}},"priority":{"state":"UNKNOWN"},"target_domain":{"state":"UNKNOWN"}}]}"#;
    const FIRE_FIELD: &str = r#"{"field_path":"protection.resistances","value":[{"kind":"FIRE_FIELD","percent":{"state":"KNOWN","value":{"denominator":1,"numerator":90}}}]}"#;

    fn apply(
        bytes: &[u8],
        semantics: &mut ReferenceItemSemantics,
    ) -> Result<EquipAbilities, ItemAbilityError> {
        apply_record(std::iter::once((KEY, semantics)), bytes)
    }

    fn charged() -> ReferenceItemSemantics {
        ReferenceItemSemantics {
            charges: ReferenceItemField::Known(ReferenceItemCharges {
                count: ReferenceItemField::Known(200),
            }),
            ..ReferenceItemSemantics::default()
        }
    }

    fn reason(error: ItemAbilityError) -> &'static str {
        match error {
            ItemAbilityError::Item { reason, .. } => reason,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn pinned_record_matches_its_digest() {
        assert_eq!(
            world_project_sha256(EQUIP_ABILITIES_V1_RECORD),
            EQUIP_ABILITIES_V1_RECORD_SHA256
        );
    }

    #[test]
    fn fallback_fills_unknown_leaves_and_derives_the_abilities() {
        let mut semantics = charged();
        let bytes = record(&listed(true, &format!("{FIST},{FIRE_FIELD}")), 1, 1, 1);
        let applied = apply(&bytes, &mut semantics).unwrap();
        assert_eq!(
            applied,
            EquipAbilities {
                items: 1,
                fallback_fields: 2,
                timed_items: 1,
                ability_items: 1
            }
        );
        let profile = item_ability_profile(&semantics).unwrap();
        assert!(profile.timed);
        assert!(!profile.extra_slot);
        assert_eq!(
            profile.abilities,
            vec![
                ItemAbility::SkillBoost {
                    skill: AbilitySkill::Fist,
                    points: 6
                },
                ItemAbility::Protection {
                    element: ReferenceResistanceKind::Fire,
                    percent: ReferenceRationalPercent::new(90, 1).unwrap(),
                    field_only: true
                },
            ]
        );
        // Applying again finds Known leaves and fails closed.
        assert_eq!(
            reason(apply(&bytes, &mut semantics).unwrap_err()),
            "a fallback row targets a leaf that is not Unknown"
        );
    }

    #[test]
    fn timed_mismatch_and_unlisted_abilities_fail_closed() {
        let bytes = record(&listed(false, FIST), 1, 0, 1);
        assert_eq!(
            reason(apply(&bytes, &mut charged()).unwrap_err()),
            "the listed timed flag differs from the derived one"
        );
        let mut semantics = ReferenceItemSemantics::default();
        apply(&record(&listed(false, FIST), 1, 0, 1), &mut semantics).unwrap();
        assert_eq!(
            reason(apply(&record("", 0, 0, 0), &mut semantics).unwrap_err()),
            "an Item with abilities is not listed with its sources"
        );
    }

    #[test]
    fn malformed_records_fail_closed() {
        let mut semantics = ReferenceItemSemantics::default();
        assert_eq!(
            apply(&record(&listed(false, FIST), 1, 0, 0), &mut semantics),
            Err(ItemAbilityError::Counts)
        );
        let sourceless = listed(false, "").replace(r#"{"class":"OTS_HYPOTHESIS_ONLY"}"#, "");
        assert_eq!(
            reason(apply(&record(&sourceless, 1, 0, 0), &mut semantics).unwrap_err()),
            "a listed Item has no source"
        );
        let out_of_range = FIRE_FIELD.replace("90", "101");
        assert_eq!(
            reason(
                apply(
                    &record(&listed(false, &out_of_range), 1, 0, 1),
                    &mut semantics
                )
                .unwrap_err()
            ),
            "a protection percent is outside [-100, 100]"
        );
        let not_ability = FIST.replace("SKILL_FIST", "MANA_SHIELD");
        assert_eq!(
            reason(
                apply(
                    &record(&listed(false, &not_ability), 1, 0, 1),
                    &mut semantics
                )
                .unwrap_err()
            ),
            "a fallback modifier is not an EQUIP-0 ability"
        );
        let unknown_path = FIST.replace("skill_modifiers.modifiers", "weapon.attack");
        assert!(matches!(
            apply(
                &record(&listed(false, &unknown_path), 1, 0, 1),
                &mut semantics
            ),
            Err(ItemAbilityError::Decode(_))
        ));
        let other = listed(false, "").replace(KEY, "oteryn:item.tibia.i1");
        assert_eq!(
            reason(apply(&record(&other, 1, 0, 0), &mut semantics).unwrap_err()),
            "no Item record with this key"
        );
    }

    #[test]
    fn extra_slot_and_suppressions_are_derived() {
        let pattern = ReferenceEquipmentPattern {
            pattern_id: 1,
            primary_slot: ReferenceItemField::Known(ReferenceEquipmentSlot::Extra),
            additional_reserved_slots: ReferenceItemField::Unknown,
            mutually_exclusive_groups: ReferenceItemField::Unknown,
            vocations: ReferenceItemField::Unknown,
            level: ReferenceItemField::Unknown,
            compatibility_rule: ReferenceItemField::Unknown,
        };
        let binding = |kind, parameter| ReferenceModifierBinding {
            kind,
            target_domain: ReferenceItemField::Unknown,
            evaluation_phase: ReferenceItemField::Unknown,
            priority: ReferenceItemField::Unknown,
            parameter,
        };
        let mut semantics = ReferenceItemSemantics {
            equipment: ReferenceItemField::Known(ReferenceItemEquipment {
                patterns: ReferenceItemField::Known(vec![pattern]),
            }),
            skill_modifiers: ReferenceItemField::Known(ReferenceItemSkillModifiers {
                modifiers: ReferenceItemField::Known(vec![
                    binding(
                        ReferenceSkillModifierKind::SuppressDrown,
                        ReferenceItemField::Known(ReferenceModifierParameter::Boolean(true)),
                    ),
                    binding(
                        ReferenceSkillModifierKind::Speed,
                        ReferenceItemField::Known(ReferenceModifierParameter::SignedPoints(
                            ReferenceSignedPoints(-20),
                        )),
                    ),
                    binding(
                        ReferenceSkillModifierKind::ManaShield,
                        ReferenceItemField::Unknown,
                    ),
                ]),
            }),
            ..ReferenceItemSemantics::default()
        };
        let profile = item_ability_profile(&semantics).unwrap();
        assert!(profile.extra_slot);
        assert!(!profile.timed);
        assert_eq!(
            profile.abilities,
            vec![
                ItemAbility::Speed(-20),
                ItemAbility::Suppress(AbilitySuppression::Drown)
            ]
        );
        let ReferenceItemField::Known(group) = &mut semantics.skill_modifiers else {
            unreachable!()
        };
        let ReferenceItemField::Known(modifiers) = &mut group.modifiers else {
            unreachable!()
        };
        modifiers[1].parameter = ReferenceItemField::Unknown;
        assert_eq!(
            item_ability_profile(&semantics),
            Err("an ability modifier needs Known signed points")
        );
    }
}
