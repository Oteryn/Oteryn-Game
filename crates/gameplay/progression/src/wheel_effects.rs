//! Runtime projection of Wheel dedication, conviction and revelation perks.

use crate::wheel::{WHEEL_LEVEL_OFFSET, WheelAllocation};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

mod gems;

/// Socket-facing resonance unlocked by full Wheel slices. Gems remain owned by Gem Atelier.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct WheelSocketSeam {
    pub resonance_stages: BTreeMap<String, u8>,
    /// Active positions at their own shared modifier grade (Crystal player_wheel.cpp:2735-2752).
    pub effective_grades: BTreeMap<u128, Vec<u8>>,
    pub grade_iv_points: u32,
    /// Signed source values in thousandths, including resistance penalties.
    pub gem_values_milli: BTreeMap<(String, String), i64>,
}

/// One non-negative catalogue value, represented exactly in thousandths.
/// Wheel catalogues include fractional percentages (for example 7.5 and 37.5).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct WheelNumericValue(u64);

impl WheelNumericValue {
    /// Whole units, for integral flat stats, counts and durations.
    pub const fn whole(self) -> u64 {
        self.0 / 1_000
    }

    /// Thousandths of the catalogue's declared unit; percentages retain their fractions.
    pub const fn milli(self) -> u64 {
        self.0
    }

    fn from_catalogue(value: &Value) -> Option<Self> {
        if let Some(value) = value.as_u64() {
            return value.checked_mul(1_000).map(Self);
        }
        let scaled = value.as_f64()? * 1_000.0;
        if !scaled.is_finite() || scaled < 0.0 || scaled >= u64::MAX as f64 || scaled.fract() != 0.0
        {
            return None;
        }
        Some(Self(scaled as u64))
    }
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
    pub revelation_numeric_effects: BTreeMap<String, BTreeMap<String, WheelNumericValue>>,
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
    /// Crystal player.cpp:7423-7437 adds Wheel skills after the base/condition projection.
    /// Crystal 201189f game.cpp:8013-8046 deliberately differs from the historical
    /// official catalogue: missing-health steps are 12/10/8, with a 50% defence cap.
    pub fn combat_mastery_percent(&self, health: u64, maximum: u64, doubled: bool) -> u32 {
        let threshold = match self
            .revelation_stages
            .get("combat_mastery")
            .copied()
            .unwrap_or(0)
        {
            1 => 12,
            2 => 10,
            3 => 8,
            _ => return 0,
        };
        if maximum == 0 || health > maximum {
            return 0;
        }
        let missing = (u128::from(maximum - health) * 100 / u128::from(maximum)) as u32;
        missing / threshold * if doubled { 2 } else { 1 }
    }

    /// Crystal player_wheel.cpp:3190-3218 rounds the target health percentage and
    /// includes both boundaries. Self-healing receives no low-health bonus.
    pub fn grove_healing_bonus_milli(&self, health: u64, maximum: u64, self_target: bool) -> u64 {
        if self_target || maximum == 0 || health > maximum {
            return 0;
        }
        let percentage = (u128::from(health) * 100 + u128::from(maximum) / 2) / u128::from(maximum);
        let bonus = self.revelation_value_milli("blessing_of_the_grove", "healing_bonus");
        if percentage
            <= u128::from(
                self.revelation_value("blessing_of_the_grove", "exclusive_low_target_health_limit"),
            )
        {
            bonus.saturating_mul(
                self.revelation_value("blessing_of_the_grove", "low_health_healing_multiplier"),
            )
        } else if percentage
            <= u128::from(
                self.revelation_value("blessing_of_the_grove", "exclusive_target_health_limit"),
            )
        {
            bonus
        } else {
            0
        }
    }

    /// Crystal game.cpp:7665-7685 and 8013-8029. Both zero components stop
    /// before flat bonuses; when a secondary survives, the primary also receives its flat.
    pub fn finish_damage_pair(
        &self,
        primary: i64,
        secondary: i64,
        health: u64,
        maximum: u64,
        two_handed: bool,
    ) -> Option<(i64, i64)> {
        if primary < 0 || secondary < 0 {
            return None;
        }
        if primary == 0 && secondary == 0 {
            return Some((0, 0));
        }
        let flat = i64::from(self.flat_damage_bonus()?);
        let multiplier = i128::from(100 + self.combat_mastery_percent(health, maximum, two_handed));
        let finish = |value: i64| {
            i64::try_from(i128::from(value.checked_add(flat)?) * multiplier / 100).ok()
        };
        Some((
            finish(primary)?,
            if secondary == 0 {
                0
            } else {
                finish(secondary)?
            },
        ))
    }

    pub fn skill_bonus(&self, index: usize) -> u32 {
        let key = match index {
            0 => "fist_fighting_skill",
            1..=3 => "weapon_skill",
            4 => "distance_skill",
            _ => return 0,
        };
        self.skill_bonuses.get(key).copied().unwrap_or(0)
    }

    pub fn magic_skill_bonus(&self) -> u32 {
        self.skill_bonuses.get("magic_skill").copied().unwrap_or(0)
    }

    pub fn revelation_value_milli(&self, perk: &str, kind: &str) -> u64 {
        self.revelation_numeric_effects
            .get(perk)
            .and_then(|effects| effects.get(kind))
            .map_or(0, |value| value.milli())
    }

    pub fn revelation_value(&self, perk: &str, kind: &str) -> u64 {
        self.revelation_numeric_effects
            .get(perk)
            .and_then(|effects| effects.get(kind))
            .map_or(0, |value| value.whole())
    }

    /// Crystal player_wheel.cpp:2767-2775: every active domain contributes its flat stats.
    pub fn flat_damage_bonus(&self) -> Option<u32> {
        self.flat_bonus("flat_damage_bonus")
    }

    pub fn flat_healing_bonus(&self) -> Option<u32> {
        self.flat_bonus("flat_healing_bonus")
    }

    fn flat_bonus(&self, kind: &str) -> Option<u32> {
        self.revelation_numeric_effects
            .values()
            .try_fold(0_u32, |total, effects| {
                total.checked_add(u32::try_from(effects.get(kind)?.whole()).ok()?)
            })
    }

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
        // Crystal player_wheel.cpp:2931-2945 adds every Grade IV modifier to each
        // revelation domain, independently of whether a gem using it is socketed.
        for domain in ["green", "red", "purple", "blue"] {
            let points = domain_points.entry(domain).or_default();
            *points = points.checked_add(self.sockets.grade_iv_points)?;
        }
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
            let positions = match gem.quality {
                crate::atelier::GemQuality::Lesser => 1,
                crate::atelier::GemQuality::Regular => 2,
                crate::atelier::GemQuality::Greater => 3,
            };
            let active = usize::from(
                *self
                    .sockets
                    .resonance_stages
                    .get(resonance_key)
                    .unwrap_or(&0),
            )
            .min(gem.mods.len())
            .min(positions);
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
                // Crystal applies each active modifier's own grade; preceding positions
                // do not cap it (player_wheel.cpp:2735-2752).
                effective.push(grade);
                let catalogue = &data.gems()["catalogue"];
                if kind == FragmentKind::Basic {
                    let row = catalogue["basic_mods"]
                        .as_array()?
                        .iter()
                        .find(|row| row["source_id"].as_u64() == Some(u64::from(id)))?;
                    for effect in row["effects"].as_array()? {
                        let name = effect["name"].as_str()?;
                        let value = gems::basic_milli(id, grade, vocation, effect)?;
                        let entry = self
                            .sockets
                            .gem_values_milli
                            .entry((name.into(), String::new()))
                            .or_default();
                        *entry = entry.checked_add(value)?;
                        match name {
                            "Hit Points" => {
                                self.max_health = self
                                    .max_health
                                    .checked_add(u32::try_from(value / 1_000).ok()?)?
                            }
                            "Mana" => {
                                self.max_mana = self
                                    .max_mana
                                    .checked_add(u32::try_from(value / 1_000).ok()?)?
                            }
                            "Capacity" => {
                                self.capacity = self
                                    .capacity
                                    .checked_add(u32::try_from(value / 1_000).ok()?)?
                            }
                            "Mitigation Multiplier" => {
                                self.mitigation_milli_percent = self
                                    .mitigation_milli_percent
                                    .checked_add(u32::try_from(value).ok()?)?
                            }
                            _ => (),
                        }
                    }
                } else {
                    let row = catalogue["supreme_mods"]
                        .as_array()?
                        .iter()
                        .find(|row| row["source_id"].as_u64() == Some(u64::from(id)))?;
                    for effect in row["grades"][usize::from(grade)]["numeric_effects"].as_array()? {
                        let projected = gems::supreme_effect(id, grade, effect)?;
                        let name = projected.kind;
                        let value = projected.value_milli;
                        let entry = self
                            .sockets
                            .gem_values_milli
                            .entry((name.into(), projected.target.into()))
                            .or_default();
                        *entry = entry.checked_add(value)?;
                        match name {
                            "revelation_mastery_points" => {
                                let target = projected.target;
                                let revelation = family["revelations"]
                                    .as_array()?
                                    .iter()
                                    .find(|row| row["name"].as_str() == Some(target))?;
                                let points = domain_points
                                    .entry(revelation["domain"].as_str()?)
                                    .or_default();
                                *points = points.checked_add(u32::try_from(value / 1_000).ok()?)?;
                            }
                            "life_leech" => {
                                self.life_leech_hundredths_percent = self
                                    .life_leech_hundredths_percent
                                    .checked_add(u32::try_from(value / 10).ok()?)?
                            }
                            "mana_leech" => {
                                self.mana_leech_hundredths_percent = self
                                    .mana_leech_hundredths_percent
                                    .checked_add(u32::try_from(value / 10).ok()?)?
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
                    WheelNumericValue::from_catalogue(&effect["value"])?,
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
    fn fractional_revelations_do_not_discard_the_whole_wheel_projection() {
        let data = WheelGemData::embedded().expect("qualified embedded catalogue");
        for (vocation, perk, stage, kind, expected_milli) in [
            (
                "sorcerer",
                "lord_of_destruction",
                2,
                "mastery_decay_critical_extra_damage",
                22_500,
            ),
            ("druid", "blessing_of_the_grove", 2, "healing_bonus", 7_500),
            (
                "monk",
                "spiritual_outburst",
                1,
                "full_harmony_repeat_damage",
                37_500,
            ),
            (
                "monk",
                "spiritual_outburst",
                3,
                "full_harmony_repeat_damage",
                62_500,
            ),
        ] {
            let family = &data.wheel()["vocations"][vocation];
            let revelation = family["revelations"]
                .as_array()
                .expect("revelations")
                .iter()
                .find(|entry| entry["key"] == perk)
                .expect("perk");
            let source_stage = revelation["stages"]
                .as_array()
                .expect("stages")
                .iter()
                .find(|entry| entry["stage"] == stage)
                .expect("stage");
            let points = u32::try_from(
                source_stage["minimum_domain_points"]
                    .as_u64()
                    .expect("domain points"),
            )
            .expect("bounded domain points");
            let domain_points =
                BTreeMap::from([(revelation["domain"].as_str().expect("domain"), points)]);
            let mut effects = WheelRuntimeEffects::default();
            project_revelations(family, &domain_points, &mut effects)
                .expect("fractional catalogue values are admitted");
            assert_eq!(
                effects.revelation_numeric_effects[perk][kind].milli(),
                expected_milli
            );
            assert_eq!(effects.revelation_stages[perk], stage);
            assert!(effects.revelation_numeric_effects[perk]["flat_damage_bonus"].whole() > 0);
        }
    }

    #[test]
    fn all_catalogue_revelation_stages_project_without_losing_flat_bonuses() {
        let data = WheelGemData::embedded().expect("qualified embedded catalogue");
        for family in data.wheel()["vocations"]
            .as_object()
            .expect("vocations")
            .values()
        {
            for revelation in family["revelations"].as_array().expect("revelations") {
                for stage in revelation["stages"].as_array().expect("stages") {
                    let key = revelation["key"].as_str().expect("key");
                    let domain = revelation["domain"].as_str().expect("domain");
                    let points = u32::try_from(
                        stage["minimum_domain_points"]
                            .as_u64()
                            .expect("minimum points"),
                    )
                    .expect("bounded points");
                    let mut effects = WheelRuntimeEffects::default();
                    project_revelations(family, &BTreeMap::from([(domain, points)]), &mut effects)
                        .expect("every qualified stage projects");
                    assert_eq!(u64::from(effects.revelation_stages[key]), stage["stage"]);
                    assert!(
                        effects.revelation_numeric_effects[key]["flat_damage_bonus"].whole() > 0
                    );
                    assert!(
                        effects.revelation_numeric_effects[key]["flat_healing_bonus"].whole() > 0
                    );
                }
            }
        }
    }

    #[test]
    fn numeric_effects_preserve_units_and_refuse_unrepresentable_values() {
        use serde_json::json;
        assert_eq!(
            WheelNumericValue::from_catalogue(&json!(20)).map(WheelNumericValue::whole),
            Some(20)
        );
        assert_eq!(
            WheelNumericValue::from_catalogue(&json!(7.5)).map(WheelNumericValue::milli),
            Some(7_500)
        );
        assert_eq!(
            WheelNumericValue::from_catalogue(&json!(0)).map(WheelNumericValue::milli),
            Some(0)
        );
        for invalid in [
            json!(-1),
            json!(0.0001),
            json!(u64::MAX),
            json!(null),
            json!("7.5"),
        ] {
            assert_eq!(WheelNumericValue::from_catalogue(&invalid), None);
        }
    }

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
    fn crystal_combat_mastery_uses_reworked_thresholds_and_equipment_multiplier() {
        let mut effects = WheelRuntimeEffects::default();
        assert_eq!(effects.combat_mastery_percent(1, 100, true), 0);
        for (stage, expected) in [(1, 4), (2, 5), (3, 6)] {
            effects
                .revelation_stages
                .insert("combat_mastery".to_owned(), stage);
            assert_eq!(effects.combat_mastery_percent(50, 100, false), expected);
            assert_eq!(effects.combat_mastery_percent(50, 100, true), expected * 2);
            assert_eq!(effects.combat_mastery_percent(100, 100, true), 0);
            assert_eq!(effects.combat_mastery_percent(101, 100, true), 0);
            assert_eq!(effects.combat_mastery_percent(0, 0, true), 0);
        }
    }

    #[test]
    fn crystal_auto_attack_components_add_flat_after_response_and_do_not_revive_zero_hits() {
        let mut effects = WheelRuntimeEffects::default();
        effects.revelation_numeric_effects.insert(
            "combat_mastery".into(),
            BTreeMap::from([("flat_damage_bonus".into(), WheelNumericValue(4_000))]),
        );
        effects.revelation_stages.insert("combat_mastery".into(), 1);
        assert_eq!(
            effects.finish_damage_pair(0, 0, 50, 100, true),
            Some((0, 0))
        );
        assert_eq!(
            effects.finish_damage_pair(100, 0, 50, 100, false),
            Some((108, 0))
        );
        assert_eq!(
            effects.finish_damage_pair(100, 0, 50, 100, true),
            Some((112, 0))
        );
        assert_eq!(
            effects.finish_damage_pair(0, 100, 50, 100, false),
            Some((4, 108)),
            "a surviving secondary keeps the primary flat component"
        );
        assert_eq!(
            effects.finish_damage_pair(100, 100, 100, 100, false),
            Some((104, 104))
        );
        assert_eq!(effects.finish_damage_pair(-1, 100, 50, 100, true), None);
        assert_eq!(effects.finish_damage_pair(i64::MAX, 0, 50, 100, true), None);
    }

    #[test]
    fn grove_keeps_fractional_bonus_rounded_inclusive_boundaries_and_self_exclusion() {
        let data = WheelGemData::embedded().expect("qualified embedded catalogue");
        let mut effects = WheelRuntimeEffects::default();
        project_revelations(
            &data.wheel()["vocations"]["druid"],
            &BTreeMap::from([("red", 500)]),
            &mut effects,
        )
        .expect("stage two Grove");
        for (health, bonus) in [
            (6_000, 7_500),
            (6_049, 7_500),
            (6_050, 0),
            (3_000, 15_000),
            (3_049, 15_000),
            (3_050, 7_500),
        ] {
            assert_eq!(
                effects.grove_healing_bonus_milli(health, 10_000, false),
                bonus
            );
        }
        assert_eq!(effects.grove_healing_bonus_milli(1, 10_000, true), 0);
        assert_eq!(effects.grove_healing_bonus_milli(1, 0, false), 0);
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
            effects.revelation_numeric_effects["gift_of_life"]["flat_damage_bonus"].whole(),
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
    fn atelier_resonance_uses_independent_shared_grades_and_revelation_mastery() {
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
            // The pinned Crystal applies independent grades, superseding the old cap.
            Some(&vec![1, 3, 3])
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

    fn allocation_with_domain_points(data: &WheelGemData, total: u16) -> WheelAllocation {
        let topology = data.wheel()["topology"].as_array().expect("topology");
        let mut points = [0; 36];
        for domain in ["green", "red", "purple", "blue"] {
            let mut remaining = total;
            for capacity in [50_u16, 75, 100, 150, 200] {
                for (index, slot) in topology.iter().enumerate() {
                    if slot["domain"] == domain
                        && slot["capacity"].as_u64() == Some(u64::from(capacity))
                    {
                        let assigned = remaining.min(capacity);
                        points[index] = assigned;
                        remaining -= assigned;
                    }
                }
            }
            assert_eq!(remaining, 0);
        }
        WheelAllocation {
            wheel_ruleset_revision: Some(data.revision().into()),
            wheel_revision: 1,
            slots: WheelSlots::new(points).expect("slots"),
            current: true,
        }
    }

    #[test]
    fn grade_iv_points_advance_every_domain_without_a_socketed_gem() {
        let data = WheelGemData::embedded().expect("catalogue");
        let allocation = allocation_with_domain_points(&data, 249);
        let ruleset = crate::wheel::WheelRuleset::from_catalogue(data.revision(), data.wheel())
            .expect("ruleset");
        ruleset
            .validate(&WheelSlots::ZERO, &allocation.slots, 1950, false)
            .expect("legal allocation");
        let mut state = AtelierState {
            vocation: "knight".into(),
            grade_ruleset_revision: Some(data.revision().into()),
            ..Default::default()
        };
        state.grades.insert((FragmentKind::Basic, 31), 3);
        let base = derive_runtime_effects(data.wheel(), &allocation, "knight", 2000, true, true)
            .expect("effects");
        assert!(base.revelation_stages.is_empty());
        let mut effects = base.clone();
        effects
            .contribute_gems(&state, &data, &allocation)
            .expect("grade bonus");
        assert_eq!(effects.sockets.grade_iv_points, 1);
        assert_eq!(effects.revelation_stages.len(), 4);
        assert!(effects.revelation_stages.values().all(|stage| *stage == 1));
        state.grade_ruleset_revision = Some("old".into());
        let mut stale = base;
        stale
            .contribute_gems(&state, &data, &allocation)
            .expect("stale grade ignored");
        assert_eq!(stale.sockets.grade_iv_points, 0);
        assert!(stale.revelation_stages.is_empty());
    }

    #[test]
    fn resonance_count_and_gem_quality_bound_the_active_modifier_positions() {
        let data = WheelGemData::embedded().expect("catalogue");
        let allocation = allocation_with_domain_points(&data, 0);
        for (quality, mods) in [
            (GemQuality::Lesser, vec![31]),
            (GemQuality::Regular, vec![31, 0]),
            (GemQuality::Greater, vec![31, 0, 5]),
        ] {
            let mut state = AtelierState {
                vocation: "knight".into(),
                grade_ruleset_revision: Some(data.revision().into()),
                ..Default::default()
            };
            state.gems.insert(
                1,
                Gem {
                    id: 1,
                    quality,
                    domain: WheelDomain::Green,
                    mods: mods.clone(),
                    locked: true,
                    vessel: true,
                    ruleset_revision: data.revision().into(),
                },
            );
            for count in 0..=3_u8 {
                let mut effects = WheelRuntimeEffects::default();
                effects
                    .sockets
                    .resonance_stages
                    .insert("vessel_resonance_top_left".into(), count);
                effects
                    .contribute_gems(&state, &data, &allocation)
                    .expect("contribution");
                assert_eq!(
                    effects.sockets.effective_grades[&1].len(),
                    usize::from(count).min(mods.len())
                );
                if count == 0 {
                    assert!(effects.sockets.gem_values_milli.is_empty());
                    assert_eq!(effects.max_health, 0);
                    assert_eq!(effects.max_mana, 0);
                }
            }
            state.gems.get_mut(&1).expect("gem").vessel = false;
            let mut effects = WheelRuntimeEffects::default();
            effects
                .sockets
                .resonance_stages
                .insert("vessel_resonance_top_left".into(), 3);
            effects
                .contribute_gems(&state, &data, &allocation)
                .expect("unsocketed");
            assert!(effects.sockets.effective_grades.is_empty());
            assert!(effects.sockets.gem_values_milli.is_empty());
        }
    }
}
