//! Runtime projection of Wheel dedication, conviction and revelation perks.

use crate::wheel::{WHEEL_LEVEL_OFFSET, WheelAllocation};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// Socket-facing resonance unlocked by full Wheel slices. Gems remain owned by Gem Atelier.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct WheelSocketSeam {
    pub resonance_stages: BTreeMap<String, u8>,
    /// Active positions only, after the preceding-position grade cap.
    pub effective_grades: BTreeMap<u128, Vec<u8>>,
    pub grade_iv_points: u32,
    /// Signed catalogue values in thousandths, including resistance penalties.
    pub gem_values_milli: BTreeMap<(String, String), i64>,
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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct BattleInstinctBonuses {
    pub melee: u32,
    pub shielding: u32,
}

/// Recomputes Battle Instinct from the current adjacent-creature census. Values are read from
/// the pinned Wheel catalogue rather than duplicated in the runtime owner.
pub fn battle_instinct_bonuses(
    catalogue: &Value,
    adjacent_creatures: u32,
) -> Option<BattleInstinctBonuses> {
    let slots = catalogue["vocations"]["knight"]["slots"].as_array()?;
    let conviction = slots
        .iter()
        .map(|slot| &slot["conviction"])
        .find(|conviction| conviction["key"] == "battle_instinct")?;
    let effects = conviction["unique_parameters"]["numeric_effects"].as_array()?;
    let value = |kind: &str| {
        effects.iter().find(|effect| effect["kind"] == kind)?["value"]
            .as_u64()
            .and_then(|value| u32::try_from(value).ok())
    };
    let threshold = value("adjacent_creature_threshold")?;
    let cap = value("adjacent_creature_cap")?;
    let counted = adjacent_creatures
        .min(cap)
        .saturating_sub(threshold.saturating_sub(1));
    Some(BattleInstinctBonuses {
        melee: counted.checked_mul(value("melee_skill_bonus_per_counted_creature")?)?,
        shielding: counted.checked_mul(value("shielding_bonus_per_counted_creature")?)?,
    })
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

impl WheelRuntimeEffects {
    /// Apply the Character's committed vessel projection to the existing Wheel contribution.
    pub fn contribute_gems(
        &mut self,
        state: &crate::atelier::AtelierState,
        data: &crate::wheel_gem_data::WheelGemData,
        allocation: &WheelAllocation,
    ) -> Option<()> {
        use crate::atelier::FragmentKind;
        let current = data.revision();
        let vocation = crate::wheel::base_vocation(&state.vocation)?;
        let family = &data.wheel()["vocations"][vocation];
        let mut domain_points = BTreeMap::<&str, u32>::new();
        for (slot, points) in data.wheel()["topology"]
            .as_array()?
            .iter()
            .zip(allocation.slots.points())
        {
            *domain_points.entry(slot["domain"].as_str()?).or_default() += u32::from(*points);
        }
        let grades_current = state.grade_ruleset_revision.as_deref() == Some(current);
        self.sockets.grade_iv_points = if grades_current {
            state.grades.values().filter(|grade| **grade == 3).count() as u32
        } else {
            0
        };
        for gem in state
            .gems
            .values()
            .filter(|gem| gem.vessel && gem.ruleset_revision == current)
        {
            let resonance_key = match gem.domain {
                crate::wheel::WheelDomain::Green => "vessel_resonance_top_left",
                crate::wheel::WheelDomain::Red => "vessel_resonance_top_right",
                crate::wheel::WheelDomain::Purple => "vessel_resonance_bottom_right",
                crate::wheel::WheelDomain::Blue => "vessel_resonance_bottom_left",
            };
            let active = usize::from(
                *self
                    .sockets
                    .resonance_stages
                    .get(resonance_key)
                    .unwrap_or(&0),
            )
            .min(gem.mods.len());
            let mut cap = 3;
            let mut effective = Vec::new();
            for (position, id) in gem.mods.iter().copied().take(active).enumerate() {
                let kind = if position == 2 {
                    FragmentKind::Supreme
                } else {
                    FragmentKind::Basic
                };
                let grade = if grades_current {
                    state.grades.get(&(kind, id)).copied().unwrap_or(0)
                } else {
                    0
                };
                cap = cap.min(grade);
                effective.push(cap);
                let catalogue = &data.gems()["catalogue"];
                if kind == FragmentKind::Basic {
                    let row = catalogue["basic_mods"]
                        .as_array()?
                        .iter()
                        .find(|row| row["source_id"].as_u64() == Some(u64::from(id)))?;
                    for effect in row["effects"].as_array()? {
                        let name = effect["name"].as_str()?;
                        let value =
                            effect["values_by_vocation"][vocation][usize::from(cap)].as_f64()?;
                        let entry = self
                            .sockets
                            .gem_values_milli
                            .entry((name.into(), String::new()))
                            .or_default();
                        *entry = entry.checked_add((value * 1000.0).round() as i64)?;
                        match name {
                            "Hit Points" => {
                                self.max_health = self.max_health.checked_add(value as u32)?
                            }
                            "Mana" => self.max_mana = self.max_mana.checked_add(value as u32)?,
                            "Capacity" => {
                                self.capacity = self.capacity.checked_add(value as u32)?
                            }
                            "Mitigation Multiplier" => {
                                self.mitigation_milli_percent = self
                                    .mitigation_milli_percent
                                    .checked_add((value * 1000.0).round() as u32)?
                            }
                            _ => (),
                        }
                    }
                } else {
                    let row = catalogue["supreme_mods"]
                        .as_array()?
                        .iter()
                        .find(|row| row["source_id"].as_u64() == Some(u64::from(id)))?;
                    for effect in row["grades"][usize::from(cap)]["numeric_effects"].as_array()? {
                        let name = effect["kind"].as_str()?;
                        let value = effect["value"].as_f64()?;
                        let entry = self
                            .sockets
                            .gem_values_milli
                            .entry((name.into(), effect["target"].as_str()?.into()))
                            .or_default();
                        *entry = entry.checked_add((value * 1000.0).round() as i64)?;
                        match name {
                            "revelation_mastery_points" => {
                                let target = effect["target"].as_str()?;
                                let revelation = family["revelations"]
                                    .as_array()?
                                    .iter()
                                    .find(|row| row["name"].as_str() == Some(target))?;
                                let points = domain_points
                                    .entry(revelation["domain"].as_str()?)
                                    .or_default();
                                *points = points.checked_add(value as u32)?;
                            }
                            "life_leech" => {
                                self.life_leech_hundredths_percent = self
                                    .life_leech_hundredths_percent
                                    .checked_add((value * 100.0).round() as u32)?
                            }
                            "mana_leech" => {
                                self.mana_leech_hundredths_percent = self
                                    .mana_leech_hundredths_percent
                                    .checked_add((value * 100.0).round() as u32)?
                            }
                            _ => (),
                        }
                    }
                }
            }
            self.sockets.effective_grades.insert(gem.id, effective);
        }
        self.revelation_stages.clear();
        self.revelation_numeric_effects.clear();
        project_revelations(family, &domain_points, self)?;
        Some(())
    }
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
    fn news_9024_battle_instinct_is_recomputed_for_each_adjacent_census() {
        let data = WheelGemData::embedded().expect("qualified embedded catalogue");
        let read = |adjacent| battle_instinct_bonuses(data.wheel(), adjacent).expect("catalogue");
        assert_eq!(read(4), BattleInstinctBonuses::default());
        assert_eq!(
            read(5),
            BattleInstinctBonuses {
                melee: 1,
                shielding: 6
            }
        );
        assert_eq!(
            read(7),
            BattleInstinctBonuses {
                melee: 3,
                shielding: 18
            }
        );
        assert_eq!(
            read(8),
            BattleInstinctBonuses {
                melee: 4,
                shielding: 24
            }
        );
        assert_eq!(read(9), read(8));
        assert_eq!(
            read(4),
            BattleInstinctBonuses::default(),
            "a later census removes the bonus"
        );
    }

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

#[cfg(test)]
#[allow(clippy::expect_used)]
mod atelier_tests {
    use super::*;
    use crate::atelier::{AtelierState, FragmentKind, Gem, GemQuality};
    use crate::wheel::{WheelDomain, WheelSlots};
    use crate::wheel_gem_data::WheelGemData;

    #[test]
    fn atelier_resonance_caps_shared_grades_and_revelation_mastery() {
        let data = WheelGemData::embedded().expect("catalogue");
        let mut points = [0; 36];
        let mut reduced = false;
        for (index, slot) in data.wheel()["topology"]
            .as_array()
            .expect("topology")
            .iter()
            .enumerate()
        {
            if slot["domain"] == "green" {
                points[index] =
                    u16::try_from(slot["capacity"].as_u64().expect("capacity")).expect("bounded");
                if points[index] == 200 && !reduced {
                    points[index] = 50;
                    reduced = true;
                }
            }
        }
        let allocation = WheelAllocation {
            wheel_ruleset_revision: Some(data.revision().into()),
            wheel_revision: 1,
            slots: WheelSlots::new(points).expect("slots"),
            current: true,
        };
        let mut state = AtelierState {
            vocation: "knight".into(),
            grade_ruleset_revision: Some(data.revision().into()),
            ..Default::default()
        };
        state.gems.insert(
            1,
            Gem {
                id: 1,
                quality: GemQuality::Greater,
                domain: WheelDomain::Green,
                mods: vec![31, 0, 5],
                locked: false,
                vessel: true,
                ruleset_revision: data.revision().into(),
            },
        );
        for (kind, id, grade) in [
            (FragmentKind::Basic, 31, 1),
            (FragmentKind::Basic, 0, 3),
            (FragmentKind::Supreme, 5, 3),
        ] {
            state.grades.insert((kind, id), grade);
        }
        let mut effects =
            derive_runtime_effects(data.wheel(), &allocation, "knight", 2000, true, true)
                .expect("effects");
        assert_eq!(effects.revelation_stages.get("gift_of_life"), Some(&2));
        effects
            .contribute_gems(&state, &data, &allocation)
            .expect("contribution");
        assert_eq!(
            effects.sockets.effective_grades.get(&1),
            Some(&vec![1, 1, 1])
        );
        assert_eq!(effects.sockets.grade_iv_points, 2);
        assert_eq!(effects.revelation_stages.get("gift_of_life"), Some(&3));
        state.gems.get_mut(&1).expect("gem").ruleset_revision = "old".into();
        let mut dormant =
            derive_runtime_effects(data.wheel(), &allocation, "knight", 2000, true, true)
                .expect("effects");
        dormant
            .contribute_gems(&state, &data, &allocation)
            .expect("dormant");
        assert!(dormant.sockets.effective_grades.is_empty());
        assert_eq!(dormant.revelation_stages.get("gift_of_life"), Some(&2));
    }
}
