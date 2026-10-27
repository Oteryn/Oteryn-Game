//! Runtime projection of Wheel dedication (small stat-perk) points.

use crate::wheel::{WHEEL_LEVEL_OFFSET, WheelAllocation};
use serde_json::Value;

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct WheelStatBonuses {
    pub max_health: u32,
    pub max_mana: u32,
    pub capacity: u32,
    /// Percentage points, retained as thousandths to avoid floating point in combat owners.
    pub mitigation_milli_percent: u32,
}

/// Projects allocated points through the vocation's dedication table. Effects fail closed when
/// the allocation or eligibility facts are not current. Convictions, revelations and gems are
/// deliberately not interpreted here.
pub fn derive_stat_bonuses(
    catalogue: &Value,
    allocation: &WheelAllocation,
    vocation: &str,
    level: u32,
    promoted: bool,
    premium: bool,
) -> Option<WheelStatBonuses> {
    if !allocation.current || level <= WHEEL_LEVEL_OFFSET || !promoted || !premium {
        return Some(WheelStatBonuses::default());
    }
    let vocation = base_vocation(vocation)?;
    let slots = catalogue["vocations"][vocation]["slots"].as_array()?;
    if slots.len() != allocation.slots.points().len() {
        return None;
    }
    let mut result = WheelStatBonuses::default();
    for (index, slot) in slots.iter().enumerate() {
        if slot["state_slot"].as_u64()? != (index + 1) as u64 {
            return None;
        }
        let points = u32::from(allocation.slots.points()[index]);
        for effect in slot["dedication"].as_array()? {
            let stat = effect["stat"].as_str()?;
            let per_point = effect["value_per_point"].as_f64()?;
            let value = per_point * f64::from(points);
            match stat {
                "max_health" => result.max_health = result.max_health.checked_add(value as u32)?,
                "max_mana" => result.max_mana = result.max_mana.checked_add(value as u32)?,
                "capacity" => result.capacity = result.capacity.checked_add(value as u32)?,
                "mitigation_multiplier" => {
                    let milli = (value * 1_000.0).round() as u32;
                    result.mitigation_milli_percent =
                        result.mitigation_milli_percent.checked_add(milli)?;
                }
                _ => return None,
            }
        }
    }
    Some(result)
}

fn base_vocation(value: &str) -> Option<&str> {
    ["druid", "knight", "monk", "paladin", "sorcerer"]
        .into_iter()
        .find(|base| value == *base || value.strip_suffix(base).is_some_and(|p| !p.is_empty()))
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "static qualified test fixtures")]
mod tests {
    use super::*;
    use crate::wheel::WheelSlots;
    use crate::wheel_gem_data::WheelGemData;

    #[test]
    fn allocated_knight_dedications_project_and_eligibility_fails_closed() {
        let data = WheelGemData::embedded().expect("qualified embedded catalogue");
        let mut points = [0; 36];
        points[0] = 10;
        points[1] = 20;
        points[9] = 5;
        let allocation = WheelAllocation {
            wheel_ruleset_revision: Some(data.revision().to_owned()),
            wheel_revision: 1,
            slots: WheelSlots::new(points).expect("bounded allocation"),
            current: true,
        };
        let bonuses =
            derive_stat_bonuses(data.wheel(), &allocation, "elite_knight", 100, true, true)
                .expect("closed catalogue");
        assert_eq!((bonuses.max_health, bonuses.max_mana), (30, 10));
        assert_eq!(bonuses.capacity, 25);
        assert_eq!(bonuses.mitigation_milli_percent, 1_500);
        assert_eq!(
            derive_stat_bonuses(data.wheel(), &allocation, "knight", 100, true, false),
            Some(WheelStatBonuses::default())
        );
    }
}
