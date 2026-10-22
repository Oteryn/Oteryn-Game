//! EQUIP-0 §3.2–§3.3: pure, ephemeral equipment contributions; no durable writes.

use oteryn_gameplay_content::content::item_abilities::{
    AbilityAmount, AbilitySkill, AbilityStat, AbilitySuppression, ItemAbility, ItemAbilityProfile,
};
use oteryn_gameplay_content::content::{ReferenceRationalPercent, ReferenceResistanceKind};
use oteryn_gameplay_domain::domain::death::EquipmentSlot;
use std::collections::{BTreeMap, BTreeSet};

/// The dispatch owner translates wire/durable slots to the canonical death slot order and
/// proves designated-slot membership from the pinned definition's primary slot patterns.
pub struct EquipmentEffectInput<'a> {
    pub slot: EquipmentSlot,
    pub definition_key: (&'a str, &'a str),
    pub in_own_slot: bool,
    pub profile: ItemAbilityProfile,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EquipmentEffects {
    pub skills: BTreeMap<AbilitySkill, i64>,
    pub stat_flats: BTreeMap<AbilityStat, i64>,
    pub stat_percents: BTreeMap<AbilityStat, ReferenceRationalPercent>,
    pub speed: i64,
    pub protections: BTreeMap<ReferenceResistanceKind, ReferenceRationalPercent>,
    pub field_protections: BTreeMap<ReferenceResistanceKind, ReferenceRationalPercent>,
    pub suppressions: BTreeSet<AbilitySuppression>,
    /// Highest level; equal levels prefer the first canonical slot/key, then lowest colour.
    /// EQUIP-0 names the brightest item but supplies no explicit equal-level colour rule.
    pub light: Option<(u8, u8)>,
}

/// EQUIP-0 §3.3. Authored ability order is lost by `item_ability_profile`. All numerical
/// operations here are exact additive reductions, suppression is set union, and light selects
/// the minimum canonical contributor among those with maximal level. These are commutative:
/// there is no per-ability clamp or truncation. Percent sums retain their rational remainder;
/// only application truncates. Checked i64 intermediates can reject an unrepresentable sum,
/// never yield a partial value. Sorting abilities gives even that rejection a fixed order.
pub fn derive_equipment_effects(
    mut equipment: Vec<EquipmentEffectInput<'_>>,
) -> Option<EquipmentEffects> {
    equipment.sort_by_key(|item| (item.slot, item.definition_key));
    // A snapshot occupies each slot at most once; reject malformed input rather than double it.
    if equipment
        .windows(2)
        .any(|pair| pair[0].slot == pair[1].slot)
    {
        return None;
    }
    let mut effects = EquipmentEffects::default();
    for mut item in equipment {
        if item.profile.timed || (!item.in_own_slot && item.slot != EquipmentSlot::Ammo) {
            continue;
        }
        item.profile.abilities.sort();
        for ability in item.profile.abilities {
            if item.slot == EquipmentSlot::Ammo && !matches!(ability, ItemAbility::Light { .. }) {
                continue;
            }
            match ability {
                ItemAbility::SkillBoost { skill, points } => {
                    let sum = effects.skills.entry(skill).or_default();
                    *sum = sum.checked_add(i64::from(points))?;
                }
                ItemAbility::StatBoost { stat, amount } => match amount {
                    AbilityAmount::Flat(points) => {
                        let sum = effects.stat_flats.entry(stat).or_default();
                        *sum = sum.checked_add(i64::from(points))?;
                    }
                    AbilityAmount::Percent(percent) => {
                        add_percent(&mut effects.stat_percents, stat, percent)?;
                    }
                },
                ItemAbility::Speed(points) => {
                    effects.speed = effects.speed.checked_add(i64::from(points))?;
                }
                ItemAbility::Protection {
                    element,
                    percent,
                    field_only,
                } => {
                    let sums = if field_only {
                        &mut effects.field_protections
                    } else {
                        &mut effects.protections
                    };
                    add_percent(sums, element, percent)?;
                }
                ItemAbility::Suppress(kind) => {
                    effects.suppressions.insert(kind);
                }
                ItemAbility::Light { level, color } => {
                    if effects.light.is_none_or(|(current, _)| level > current) {
                        effects.light = Some((level, color));
                    }
                }
            }
        }
    }
    for percent in effects
        .protections
        .values_mut()
        .chain(effects.field_protections.values_mut())
    {
        *percent = clamp_protection(*percent)?;
    }
    Some(effects)
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

fn add_percent<K: Ord>(
    sums: &mut BTreeMap<K, ReferenceRationalPercent>,
    key: K,
    percent: ReferenceRationalPercent,
) -> Option<()> {
    percent.validate().ok()?;
    let sum = sums.entry(key).or_insert(ReferenceRationalPercent {
        numerator: 0,
        denominator: 1,
    });
    let common = gcd(sum.denominator, percent.denominator);
    let left = i64::try_from(percent.denominator / common).ok()?;
    let right = i64::try_from(sum.denominator / common).ok()?;
    let numerator = sum
        .numerator
        .checked_mul(left)?
        .checked_add(percent.numerator.checked_mul(right)?)?;
    let denominator = sum.denominator.checked_mul(percent.denominator / common)?;
    let divisor = gcd(numerator.unsigned_abs(), denominator);
    *sum = ReferenceRationalPercent {
        numerator: numerator.checked_div(i64::try_from(divisor).ok()?)?,
        denominator: denominator / divisor,
    };
    Some(())
}

fn clamp_protection(percent: ReferenceRationalPercent) -> Option<ReferenceRationalPercent> {
    percent.validate().ok()?;
    // Compare without multiplying the denominator, which itself could overflow.
    if percent.numerator.unsigned_abs() / percent.denominator >= 100 {
        Some(ReferenceRationalPercent {
            numerator: percent.numerator.signum() * 100,
            denominator: 1,
        })
    } else {
        Some(percent)
    }
}

fn percent_of(base: i64, percent: ReferenceRationalPercent) -> Option<i64> {
    percent.validate().ok()?;
    base.checked_mul(percent.numerator)?
        .checked_div(i64::try_from(percent.denominator).ok()?.checked_mul(100)?)
}

/// EQUIP-0 §3.3: percent of the base, truncated toward zero once, then flat points.
/// Maxima clamping belongs to the future STAT_BOOST consumer.
pub fn apply_stat(base: i64, flat: i64, percent: ReferenceRationalPercent) -> Option<i64> {
    base.checked_add(percent_of(base, percent)?)?
        .checked_add(flat)
}

/// EQUIP-0 §3.3: apply the combined protection once, after armor and defence.
/// The incoming compositor owns combining equipment, imbuements and other admitted sources.
pub fn apply_protection(damage: i64, sum: ReferenceRationalPercent) -> Option<i64> {
    if damage < 0 {
        return None;
    }
    damage.checked_sub(percent_of(damage, clamp_protection(sum)?)?)
}

#[cfg(test)]
#[path = "equipment_effects_tests.rs"]
mod tests;
