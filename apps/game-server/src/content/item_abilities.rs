//! Typed Equipment abilities (EQUIP-CONTENT-1; EQUIP-0 §3.1-§3.2, architect bundle §2.9).
//!
//! The six abilities of EQUIP-0 §3.1 are a derived view over the existing `skill_modifiers` and
//! `protection` groups of an Item definition: no codec or schema change (control-plane ruling a).
//! `timed` is derived from `charges.count` or `temporal.duration`, and the Extra slot from an
//! Equipment pattern whose `primary_slot` is Extra; neither has a flag of its own.
//!
//! The pinned facts packet `docs/agents/evidence/OTV2-20261003-equip-abilities-v1.json` holds
//! Game-owned field values only: Item key, field path and canonical value. The authoring tool
//! `tools/content-schema/item-authoring/lower_equip_abilities_packet.py` does all source lowering
//! and keeps the provenance in its own record, so nothing here knows where a value came from. The
//! materializer applies the packet once, last: a fact fills an Unknown leaf only, and every
//! Item's derived view must decode. Anything else fails closed.
//!
//! STAT_BOOST and LIGHT have the type but no rows: no v1 source states them (a follow-up question
//! for EQUIP-RT-1).

use super::{
    ProjectReferenceRecord, ProjectV2Draft, ReferenceEquipmentSlot, ReferenceItemField,
    ReferenceItemProtection, ReferenceItemSemantics, ReferenceItemSkillModifiers,
    ReferenceModifierBinding, ReferenceModifierParameter, ReferenceRationalPercent,
    ReferenceResistance, ReferenceResistanceKind, ReferenceSignedPoints,
    ReferenceSkillModifierKind, world_project_sha256,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

/// The pinned facts packet bytes; any change is a new candidate with a new digest.
pub const EQUIP_ABILITIES_V1_FACTS: &[u8] =
    include_bytes!("../../../../docs/agents/evidence/OTV2-20261003-equip-abilities-v1.json");
pub const EQUIP_ABILITIES_V1_FACTS_SHA256: &str =
    "669ced861ca5f90bf3929f62de00a92e3fd0b6e13bdceb423ee7535811ab4668";
pub const EQUIP_ABILITIES_V1_ITEM_COUNT: usize = 27;
pub const EQUIP_ABILITIES_V1_FIELD_COUNT: usize = 32;
const SCHEMA: &str = "OTERYN_EQUIP_ABILITY_FACTS/v1";

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
    /// Typed, without rows in v1 (no v1 source states it).
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
    /// Typed, without rows in v1 (no v1 source states it).
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
            Self::Digest => formatter.write_str("equip ability facts digest mismatch"),
            Self::Decode(error) => write!(formatter, "equip ability facts: {error}"),
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
    /// Items that took at least one fact.
    pub items: usize,
    pub fields: usize,
    /// Item records whose derived view holds at least one ability.
    pub ability_items: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Packet {
    schema: String,
    counts: Counts,
    facts: Vec<Fact>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Counts {
    items: usize,
    fields: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Fact {
    item_key: String,
    field_path: String,
    value: serde_json::Value,
}

enum FactRow {
    Modifiers(Vec<ReferenceModifierBinding>),
    Resistances(Vec<ReferenceResistance>),
}

fn fact_row(fact: Fact) -> Result<(String, FactRow), ItemAbilityError> {
    let decode = |error: serde_json::Error| ItemAbilityError::Decode(error.to_string());
    let row = match fact.field_path.as_str() {
        "skill_modifiers.modifiers" => {
            FactRow::Modifiers(serde_json::from_value(fact.value).map_err(decode)?)
        }
        "protection.resistances" => {
            FactRow::Resistances(serde_json::from_value(fact.value).map_err(decode)?)
        }
        _ => return Err(ItemAbilityError::Decode("field_path".to_owned())),
    };
    Ok((fact.item_key, row))
}

/// Apply the pinned facts packet to every Item record in `draft`.
pub fn apply_equip_abilities_v1(
    draft: &mut ProjectV2Draft,
) -> Result<EquipAbilities, ItemAbilityError> {
    if world_project_sha256(EQUIP_ABILITIES_V1_FACTS) != EQUIP_ABILITIES_V1_FACTS_SHA256 {
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
    let applied = apply_packet(items, EQUIP_ABILITIES_V1_FACTS)?;
    if applied.items != EQUIP_ABILITIES_V1_ITEM_COUNT
        || applied.fields != EQUIP_ABILITIES_V1_FIELD_COUNT
    {
        return Err(ItemAbilityError::Counts);
    }
    Ok(applied)
}

fn apply_packet<'a>(
    items: impl Iterator<Item = (&'a str, &'a mut ReferenceItemSemantics)>,
    bytes: &[u8],
) -> Result<EquipAbilities, ItemAbilityError> {
    let packet: Packet = serde_json::from_slice(bytes)
        .map_err(|error| ItemAbilityError::Decode(error.to_string()))?;
    if packet.schema != SCHEMA {
        return Err(ItemAbilityError::Decode("schema".to_owned()));
    }
    let fields = packet.facts.len();
    let mut by_item: BTreeMap<String, Vec<FactRow>> = BTreeMap::new();
    let mut paths = BTreeSet::new();
    for fact in packet.facts {
        if !paths.insert((fact.item_key.clone(), fact.field_path.clone())) {
            return Err(item_error(&fact.item_key, "duplicate fact"));
        }
        let (key, row) = fact_row(fact)?;
        by_item.entry(key).or_default().push(row);
    }
    if packet.counts.items != by_item.len() || packet.counts.fields != fields {
        return Err(ItemAbilityError::Counts);
    }

    let mut applied = EquipAbilities {
        items: 0,
        fields: 0,
        ability_items: 0,
    };
    for (key, semantics) in items {
        if let Some(rows) = by_item.remove(key) {
            for row in &rows {
                apply_fact(semantics, row).map_err(|reason| item_error(key, reason))?;
                applied.fields += 1;
            }
            applied.items += 1;
        }
        let profile = item_ability_profile(semantics).map_err(|reason| item_error(key, reason))?;
        applied.ability_items += usize::from(!profile.abilities.is_empty());
    }
    if let Some(key) = by_item.keys().next() {
        return Err(item_error(key, "no Item record with this key"));
    }
    Ok(applied)
}

/// Fill one Unknown leaf from a fact; any other evidence state fails closed.
fn apply_fact(semantics: &mut ReferenceItemSemantics, row: &FactRow) -> Result<(), &'static str> {
    match row {
        FactRow::Modifiers(modifiers) => {
            if modifiers.is_empty() {
                return Err("an empty fact");
            }
            for modifier in modifiers {
                if modifier_ability(modifier)?.is_none() {
                    return Err("a fact modifier is not an EQUIP-0 ability");
                }
            }
            if semantics.skill_modifiers.is_unknown() {
                semantics.skill_modifiers =
                    ReferenceItemField::Known(ReferenceItemSkillModifiers {
                        modifiers: ReferenceItemField::Unknown,
                    });
            }
            let ReferenceItemField::Known(group) = &mut semantics.skill_modifiers else {
                return Err("the fact group holds another evidence state");
            };
            if !group.modifiers.is_unknown() {
                return Err("a fact targets a leaf that is not Unknown");
            }
            group.modifiers = ReferenceItemField::Known(modifiers.clone());
        }
        FactRow::Resistances(resistances) => {
            if resistances.is_empty() {
                return Err("an empty fact");
            }
            for resistance in resistances {
                if let ReferenceItemField::Known(percent) = resistance.percent {
                    percent
                        .validate()
                        .map_err(|_| "a fact percent is not canonical")?;
                }
                resistance_ability(resistance)?;
            }
            if semantics.protection.is_unknown() {
                semantics.protection = ReferenceItemField::Known(ReferenceItemProtection {
                    armor: ReferenceItemField::Unknown,
                    resistances: ReferenceItemField::Unknown,
                });
            }
            let ReferenceItemField::Known(group) = &mut semantics.protection else {
                return Err("the fact group holds another evidence state");
            };
            if !group.resistances.is_unknown() {
                return Err("a fact targets a leaf that is not Unknown");
            }
            group.resistances = ReferenceItemField::Known(resistances.clone());
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::panic, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::content::{ReferenceEquipmentPattern, ReferenceItemCharges, ReferenceItemEquipment};

    const KEY: &str = "oteryn:item.tibia.i3087";

    fn packet(facts: &[&str]) -> Vec<u8> {
        let items = usize::from(!facts.is_empty());
        let fields = facts.len();
        let facts = facts
            .iter()
            .map(|fact| format!(r#"{{"item_key":"{KEY}",{fact}}}"#))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            r#"{{"schema":"{SCHEMA}","counts":{{"items":{items},"fields":{fields}}},"facts":[{facts}]}}"#
        )
        .into_bytes()
    }

    const FIST: &str = r#""field_path":"skill_modifiers.modifiers","value":[{"evaluation_phase":{"state":"UNKNOWN"},"kind":"SKILL_FIST","parameter":{"state":"KNOWN","value":{"kind":"SIGNED_POINTS","value":6}},"priority":{"state":"UNKNOWN"},"target_domain":{"state":"UNKNOWN"}}]"#;
    const FIRE_FIELD: &str = r#""field_path":"protection.resistances","value":[{"kind":"FIRE_FIELD","percent":{"state":"KNOWN","value":{"denominator":1,"numerator":90}}}]"#;

    fn apply(
        bytes: &[u8],
        semantics: &mut ReferenceItemSemantics,
    ) -> Result<EquipAbilities, ItemAbilityError> {
        apply_packet(std::iter::once((KEY, semantics)), bytes)
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
    fn pinned_packet_matches_its_digest_and_names_no_source() {
        assert_eq!(
            world_project_sha256(EQUIP_ABILITIES_V1_FACTS),
            EQUIP_ABILITIES_V1_FACTS_SHA256
        );
        let text = std::str::from_utf8(EQUIP_ABILITIES_V1_FACTS)
            .unwrap()
            .to_lowercase();
        for word in ["canary", "wiki", "source", "hypothesis"] {
            assert!(!text.contains(word), "{word}");
        }
    }

    #[test]
    fn facts_fill_unknown_leaves_and_derive_the_abilities() {
        let mut semantics = charged();
        let bytes = packet(&[FIST, FIRE_FIELD]);
        let applied = apply(&bytes, &mut semantics).unwrap();
        assert_eq!(
            applied,
            EquipAbilities {
                items: 1,
                fields: 2,
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
            "a fact targets a leaf that is not Unknown"
        );
        // An Item without facts is still checked through its derived view.
        assert_eq!(
            apply(&packet(&[]), &mut semantics).unwrap(),
            EquipAbilities {
                items: 0,
                fields: 0,
                ability_items: 1
            }
        );
    }

    #[test]
    fn malformed_packets_fail_closed() {
        let mut semantics = ReferenceItemSemantics::default();
        let mut miscounted = String::from_utf8(packet(&[FIST])).unwrap();
        miscounted = miscounted.replace(r#""fields":1"#, r#""fields":2"#);
        assert_eq!(
            apply(miscounted.as_bytes(), &mut semantics),
            Err(ItemAbilityError::Counts)
        );
        assert_eq!(
            reason(apply(&packet(&[FIST, FIST]), &mut semantics).unwrap_err()),
            "duplicate fact"
        );
        let out_of_range = FIRE_FIELD.replace("90", "101");
        assert_eq!(
            reason(apply(&packet(&[&out_of_range]), &mut semantics).unwrap_err()),
            "a protection percent is outside [-100, 100]"
        );
        let not_ability = FIST.replace("SKILL_FIST", "MANA_SHIELD");
        assert_eq!(
            reason(apply(&packet(&[&not_ability]), &mut semantics).unwrap_err()),
            "a fact modifier is not an EQUIP-0 ability"
        );
        let unknown_path = FIST.replace("skill_modifiers.modifiers", "weapon.attack");
        assert!(matches!(
            apply(&packet(&[&unknown_path]), &mut semantics),
            Err(ItemAbilityError::Decode(_))
        ));
        let with_source = String::from_utf8(packet(&[FIST]))
            .unwrap()
            .replace(r#""facts":"#, r#""source":{},"facts":"#);
        assert!(matches!(
            apply(with_source.as_bytes(), &mut semantics),
            Err(ItemAbilityError::Decode(_))
        ));
        let other = String::from_utf8(packet(&[FIST]))
            .unwrap()
            .replace(KEY, "oteryn:item.tibia.i3052");
        assert_eq!(
            reason(apply(other.as_bytes(), &mut semantics).unwrap_err()),
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
