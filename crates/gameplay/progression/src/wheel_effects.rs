//! Runtime projection of Wheel dedication, conviction and revelation perks.

use crate::wheel::{WHEEL_LEVEL_OFFSET, WheelAllocation};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// Socket-facing resonance unlocked by full Wheel slices. Gems remain owned by Gem Atelier.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct WheelSocketSeam {
    pub resonance_stages: BTreeMap<String, u8>,
}

/// Contributions consumed by the existing stat, spell and combat owners.
/// Percent values use hundredths of a percentage point, matching the combat leech owner.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct WheelRuntimeEffects {
    pub max_health: u32,
    pub max_mana: u32,
    pub capacity: u32,
    pub mitigation_milli_percent: u32,
    pub skill_bonuses: BTreeMap<String, u32>,
    pub life_leech_hundredths_percent: u32,
    pub mana_leech_hundredths_percent: u32,
    pub active_unique_perks: BTreeSet<String>,
    pub revelation_stages: BTreeMap<String, u8>,
    pub revelation_numeric_effects: BTreeMap<String, BTreeMap<String, u64>>,
    pub sockets: WheelSocketSeam,
}

/// Backwards-compatible name for the original dedication-only dispatch seam.
pub type WheelStatBonuses = WheelRuntimeEffects;

/// Projects a current allocation through the closed Wheel catalogue. A full slice activates its
/// conviction, copies stack, and domain totals select one revelation stage. Gems are not read.
/// Any malformed catalogue or unknown effect fails closed (`None`).
pub fn derive_runtime_effects(
    catalogue: &Value,
    allocation: &WheelAllocation,
    vocation: &str,
    level: u32,
    promoted: bool,
    premium: bool,
) -> Option<WheelRuntimeEffects> {
    if !allocation.current || level <= WHEEL_LEVEL_OFFSET || !promoted || !premium {
        return Some(WheelRuntimeEffects::default());
    }
    let vocation = crate::wheel::base_vocation(vocation)?;
    let topology = catalogue["topology"].as_array()?;
    let vocation = &catalogue["vocations"][vocation];
    let slots = vocation["slots"].as_array()?;
    if slots.len() != allocation.slots.points().len() || topology.len() != slots.len() {
        return None;
    }

    let mut result = WheelRuntimeEffects::default();
    let mut domain_points = BTreeMap::<&str, u32>::new();
    for (index, (slot, topology)) in slots.iter().zip(topology).enumerate() {
        if slot["state_slot"].as_u64()? != (index + 1) as u64
            || topology["state_slot"].as_u64()? != (index + 1) as u64
        {
            return None;
        }
        let points = u32::from(allocation.slots.points()[index]);
        *domain_points
            .entry(topology["domain"].as_str()?)
            .or_default() += points;
        project_dedication(slot, points, &mut result)?;
        if u64::from(points) == topology["capacity"].as_u64()? {
            project_conviction(&slot["conviction"], &mut result)?;
        }
    }
    project_revelations(vocation, &domain_points, &mut result)?;
    Some(result)
}

/// Compatibility entry point used by composed admission.
pub fn derive_stat_bonuses(
    catalogue: &Value,
    allocation: &WheelAllocation,
    vocation: &str,
    level: u32,
    promoted: bool,
    premium: bool,
) -> Option<WheelStatBonuses> {
    derive_runtime_effects(catalogue, allocation, vocation, level, promoted, premium)
}

fn project_dedication(slot: &Value, points: u32, out: &mut WheelRuntimeEffects) -> Option<()> {
    for effect in slot["dedication"].as_array()? {
        let amount = effect["value_per_point"].as_f64()? * f64::from(points);
        match effect["stat"].as_str()? {
            "max_health" => out.max_health = out.max_health.checked_add(amount as u32)?,
            "max_mana" => out.max_mana = out.max_mana.checked_add(amount as u32)?,
            "capacity" => out.capacity = out.capacity.checked_add(amount as u32)?,
            "mitigation_multiplier" => {
                out.mitigation_milli_percent = out
                    .mitigation_milli_percent
                    .checked_add((amount * 1_000.0).round() as u32)?;
            }
            _ => return None,
        }
    }
    Some(())
}

fn project_conviction(conviction: &Value, out: &mut WheelRuntimeEffects) -> Option<()> {
    let key = conviction["key"].as_str()?;
    match conviction["category"].as_str()? {
        "skill_bonus" => {
            let target = key.strip_suffix("_boost")?;
            let value = u32::try_from(conviction["native_value"].as_u64()?).ok()?;
            let entry = out.skill_bonuses.entry(target.to_owned()).or_default();
            *entry = entry.checked_add(value)?;
        }
        "leech" => {
            let value = fixed_hundredths(&conviction["native_value"])?;
            let target = match key {
                "life_leech" => &mut out.life_leech_hundredths_percent,
                "mana_leech" => &mut out.mana_leech_hundredths_percent,
                _ => return None,
            };
            *target = target.checked_add(value)?;
        }
        "unique" => {
            out.active_unique_perks.insert(key.to_owned());
        }
        "vessel_resonance" => {
            let value = u8::try_from(conviction["native_value"].as_u64()?).ok()?;
            let entry = out
                .sockets
                .resonance_stages
                .entry(key.to_owned())
                .or_default();
            *entry = entry.checked_add(value)?;
        }
        // Augment convictions are already applied by WheelStages in the spell owner.
        "augmentation" => {}
        _ => return None,
    }
    Some(())
}

fn project_revelations(
    vocation: &Value,
    domain_points: &BTreeMap<&str, u32>,
    out: &mut WheelRuntimeEffects,
) -> Option<()> {
    for revelation in vocation["revelations"].as_array()? {
        let domain = revelation["domain"].as_str()?;
        let points = *domain_points.get(domain).unwrap_or(&0);
        let stage = revelation["stages"].as_array()?.iter().rfind(|entry| {
            entry["minimum_domain_points"]
                .as_u64()
                .is_some_and(|value| u64::from(points) >= value)
        });
        let Some(stage) = stage else { continue };
        let key = revelation["key"].as_str()?;
        out.revelation_stages
            .insert(key.to_owned(), u8::try_from(stage["stage"].as_u64()?).ok()?);
        let mut effects = BTreeMap::new();
        for effect in stage["numeric_effects"].as_array()? {
            if effects
                .insert(
                    effect["kind"].as_str()?.to_owned(),
                    effect["value"].as_u64()?,
                )
                .is_some()
            {
                return None;
            }
        }
        out.revelation_numeric_effects
            .insert(key.to_owned(), effects);
    }
    Some(())
}

fn fixed_hundredths(value: &Value) -> Option<u32> {
    let scaled = value.as_f64()? * 100.0;
    (scaled.is_finite() && scaled >= 0.0).then_some(scaled.round() as u32)
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "static qualified test fixtures")]
mod tests {
    use super::*;
    use crate::wheel::WheelSlots;
    use crate::wheel_gem_data::WheelGemData;

    #[test]
    fn full_slices_project_all_perk_classes_and_socket_seam() {
        let data = WheelGemData::embedded().expect("qualified embedded catalogue");
        let topology = data.wheel()["topology"].as_array().expect("topology");
        let mut points = [0; 36];
        for (index, slot) in topology.iter().enumerate() {
            if slot["domain"] == "green" {
                points[index] = u16::try_from(slot["capacity"].as_u64().expect("capacity"))
                    .expect("bounded capacity");
            }
        }
        let allocation = WheelAllocation {
            wheel_ruleset_revision: Some(data.revision().to_owned()),
            wheel_revision: 1,
            slots: WheelSlots::new(points).expect("bounded allocation"),
            current: true,
        };
        let effects =
            derive_runtime_effects(data.wheel(), &allocation, "elite_knight", 2_000, true, true)
                .expect("closed catalogue");
        assert_eq!(effects.revelation_stages.get("gift_of_life"), Some(&3));
        assert_eq!(
            effects.revelation_numeric_effects["gift_of_life"]["flat_damage_bonus"],
            20
        );
        assert!(effects.life_leech_hundredths_percent > 0);
        assert!(!effects.sockets.resonance_stages.is_empty());
        assert!(effects.max_health > 0);
        assert_eq!(
            derive_runtime_effects(
                data.wheel(),
                &allocation,
                "elite_knight",
                2_000,
                true,
                false
            ),
            Some(WheelRuntimeEffects::default())
        );
    }
}
