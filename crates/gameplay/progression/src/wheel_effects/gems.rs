//! Gem values in thousandths, normalized before separate contributions are added.
//! Retained integer-field rules come from Crystal at 201189f; Revelation Mastery and
//! Monk mixed Mana use catalogue values corroborated by official test client 15.33.dd0955.

use super::WheelRuntimeEffects;
use serde_json::Value;

impl WheelRuntimeEffects {
    /// Signed thousandths of the declared unit. Spell targets are the source spell name
    /// without the catalogue's `Augmented ` display prefix. Momentum has an empty target.
    /// Gem resistance against one element is capped at `GEM_RESISTANCE_CAP_MILLI`.
    pub fn gem_value_milli(&self, kind: &str, target: &str) -> i64 {
        let value = self
            .sockets
            .gem_values_milli
            .iter()
            .find_map(|((key, name), value)| (key == kind && name == target).then_some(*value))
            .unwrap_or(0);
        if capped_resistance(kind) {
            value.min(GEM_RESISTANCE_CAP_MILLI)
        } else {
            value
        }
    }

    /// The client's cap warning: the socketed gems' summed resistance against at least one
    /// element exceeds the cap.
    pub fn gem_resistance_over_cap(&self) -> bool {
        self.sockets
            .gem_values_milli
            .iter()
            .any(|((kind, _), value)| capped_resistance(kind) && *value > GEM_RESISTANCE_CAP_MILLI)
    }
}

/// Winter Update 2026 (test server reply "Resistances cap at 30%", corrected from 18%):
/// summed gem resistance against one element is at most 30%; each element caps separately
/// and penalties below the cap keep their value. Thousandths of a percentage point.
pub const GEM_RESISTANCE_CAP_MILLI: i64 = 30_000;

fn capped_resistance(kind: &str) -> bool {
    matches!(
        kind,
        "Physical Resistance"
            | "Holy Resistance"
            | "Death Resistance"
            | "Fire Resistance"
            | "Earth Resistance"
            | "Ice Resistance"
            | "Energy Resistance"
            | "Mana Drain Resistance"
            | "Life Drain Resistance"
    )
}

fn catalogue_milli(value: &Value) -> Option<i64> {
    if let Some(value) = value.as_i64() {
        return value.checked_mul(1_000);
    }
    let scaled = value.as_f64()? * 1_000.0;
    if !scaled.is_finite() || scaled < i64::MIN as f64 || scaled >= i64::MAX as f64 {
        return None;
    }
    let rounded = scaled.round();
    if (scaled - rounded).abs() > 0.000_001 {
        return None;
    }
    Some(rounded as i64)
}

pub fn basic_milli(id: u16, grade: u8, vocation: &str, effect: &Value) -> Option<i64> {
    if grade > 3 {
        return None;
    }
    let name = effect["name"].as_str()?;
    let value = catalogue_milli(&effect["values_by_vocation"][vocation][usize::from(grade)])?;
    match name {
        // wheel_gems.cpp:861-869: Monk capacity is 500 * 1.5 at Grade IV;
        // the imported description's 700 is not the executed value.
        "Capacity" if id == 48 && vocation == "monk" && grade == 3 => Some(750_000),
        "Hit Points" | "Mana" | "Capacity" => Some(value / 1_000 * 1_000),
        // GemModifierResistanceStrategy and GemModifierStatStrategy take int32_t
        // hundredths (wheel_gems.hpp:43-68); signed penalties are retained in the sum.
        "Physical Resistance"
        | "Holy Resistance"
        | "Death Resistance"
        | "Fire Resistance"
        | "Earth Resistance"
        | "Ice Resistance"
        | "Energy Resistance"
        | "Mana Drain Resistance"
        | "Life Drain Resistance"
        | "Mitigation Multiplier" => Some(value / 10 * 10),
        _ => None,
    }
}

pub struct SupremeEffect<'a> {
    pub kind: &'a str,
    pub target: &'a str,
    pub value_milli: i64,
}

pub fn supreme_effect(id: u16, grade: u8, effect: &Value) -> Option<SupremeEffect<'_>> {
    let multiplier = *[10_i64, 11, 12, 15].get(usize::from(grade))?;
    let mut kind = effect["kind"].as_str()?;
    let display_target = effect["target"].as_str()?;
    let mut target = display_target
        .strip_prefix("Augmented ")
        .unwrap_or(display_target);
    let mut value = catalogue_milli(&effect["value"])?;
    if value < 0 {
        return None;
    }
    match (id, kind) {
        // The donor truncates the int32_t constructor argument before aggregation:
        // 28 * 1/1.1/1.2/1.5 becomes 28/30/33/42 hundredths, not rounded UI text.
        (0, "dodge") => value = (28 * multiplier / 10) * 10,
        // Crystal explicitly rounds Groundshaker, unlike the other int spell fields.
        // wheel_gems.cpp:365-367; wheel_definitions.hpp:287-305.
        (16, "base_damage_bonus") => value = ((6_500 * multiplier / 10 + 500) / 1_000) * 1_000,
        // Executed primary-source values differ from the imported descriptions.
        // wheel_gems.cpp:408-410,515-518,595-613. The Flurry assignments intentionally
        // follow the source fields even though their enum names describe the opposite.
        (29, "critical_extra_damage") => value = (12 * multiplier / 10) * 1_000,
        (60, "cooldown_reduction") => value = 5_000,
        (82, "base_damage_bonus") => {
            kind = "critical_extra_damage";
            value = (6_500 * multiplier / 10) / 1_000 * 1_000;
        }
        (83, "critical_extra_damage") => {
            kind = "base_damage_bonus";
            value = (8 * multiplier / 10) * 1_000;
        }
        (86, "base_damage_bonus") => value = (8 * multiplier / 10) * 1_000,
        _ => {}
    }
    value = match kind {
        "dodge" | "life_leech" | "mana_leech" => value / 10 * 10,
        "critical_extra_damage" if target == "Critical Extra Damage" => value / 10 * 10,
        "critical_extra_damage" | "base_damage_bonus" | "base_healing_bonus" => {
            value / 1_000 * 1_000
        }
        "cooldown_reduction" => value,
        "momentum_chance" => {
            // wheel_gems.cpp:325 et al. add to global momentum, regardless of spell label.
            target = "";
            value
        }
        "revelation_mastery_points" => {
            // A Revelation Mastery modifier contributes its displayed points once.
            value / 1_000 * 1_000
        }
        _ => return None,
    };
    Some(SupremeEffect {
        kind,
        target,
        value_milli: value,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atelier::{AtelierState, FragmentKind, Gem, GemQuality};
    use crate::wheel::{WheelAllocation, WheelSlots};
    use crate::wheel_gem_data::WheelGemData;

    type Error = Box<dyn std::error::Error>;

    fn resistances(values: &[(&str, i64)]) -> WheelRuntimeEffects {
        let mut effects = WheelRuntimeEffects::default();
        for (kind, value) in values {
            effects
                .sockets
                .gem_values_milli
                .insert(((*kind).into(), String::new()), *value);
        }
        effects
    }

    #[test]
    fn gem_fire_resistance_caps_at_thirty_percent() {
        let over = resistances(&[("Fire Resistance", 42_500)]);
        assert_eq!(over.gem_value_milli("Fire Resistance", ""), 30_000);
        assert!(over.gem_resistance_over_cap());
        let at = resistances(&[("Fire Resistance", 30_000)]);
        assert_eq!(at.gem_value_milli("Fire Resistance", ""), 30_000);
        assert!(!at.gem_resistance_over_cap());
        let under = resistances(&[("Fire Resistance", 18_000)]);
        assert_eq!(under.gem_value_milli("Fire Resistance", ""), 18_000);
        assert!(!under.gem_resistance_over_cap());
    }

    #[test]
    fn gem_caps_apply_per_element_independently() {
        let effects = resistances(&[
            ("Fire Resistance", 45_000),
            ("Ice Resistance", 12_000),
            ("Earth Resistance", -8_000),
            ("Mana Drain Resistance", 31_000),
            ("Hit Points", 400_000),
        ]);
        assert_eq!(effects.gem_value_milli("Fire Resistance", ""), 30_000);
        assert_eq!(effects.gem_value_milli("Ice Resistance", ""), 12_000);
        assert_eq!(effects.gem_value_milli("Earth Resistance", ""), -8_000);
        assert_eq!(effects.gem_value_milli("Mana Drain Resistance", ""), 30_000);
        assert_eq!(effects.gem_value_milli("Energy Resistance", ""), 0);
        assert_eq!(effects.gem_value_milli("Hit Points", ""), 400_000);
        assert!(effects.gem_resistance_over_cap());
        let capped_elsewhere =
            resistances(&[("Ice Resistance", 30_000), ("Fire Resistance", 30_000)]);
        assert!(!capped_elsewhere.gem_resistance_over_cap());
    }

    fn effect(data: &WheelGemData, id: u16, grade: u8, kind: &str) -> Result<Value, Error> {
        let row = data.gems()["catalogue"]["supreme_mods"]
            .as_array()
            .ok_or("supreme rows")?
            .iter()
            .find(|row| row["source_id"].as_u64() == Some(u64::from(id)))
            .ok_or("modifier")?;
        row["grades"][usize::from(grade)]["numeric_effects"]
            .as_array()
            .ok_or("grade effects")?
            .iter()
            .find(|effect| effect["kind"] == kind)
            .cloned()
            .ok_or_else(|| "effect".into())
    }

    #[test]
    fn spell_integer_rounding_occurs_per_gem_and_global_crit_keeps_hundredths() -> Result<(), Error>
    {
        let data = WheelGemData::embedded()?;
        for (id, kind, expected) in [
            (4, "base_healing_bonus", [5_000, 5_000, 6_000, 7_000]),
            (10, "base_damage_bonus", [5_000, 5_000, 6_000, 7_000]),
            (16, "base_damage_bonus", [7_000, 7_000, 8_000, 10_000]),
            (1, "critical_extra_damage", [2_000, 2_200, 2_400, 3_000]),
            (0, "dodge", [280, 300, 330, 420]),
        ] {
            for grade in 0..4_u8 {
                let row = effect(&data, id, grade, kind)?;
                let actual = supreme_effect(id, grade, &row).ok_or("projection")?;
                assert_eq!(
                    actual.value_milli,
                    expected[usize::from(grade)],
                    "modifier {id}, grade {grade}"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn executed_primary_spell_values_override_display_descriptions() -> Result<(), Error> {
        let data = WheelGemData::embedded()?;
        for (id, input_kind, output_kind, expected) in [
            (
                29,
                "critical_extra_damage",
                "critical_extra_damage",
                [12_000, 13_000, 14_000, 18_000],
            ),
            (60, "cooldown_reduction", "cooldown_reduction", [5_000; 4]),
            (
                82,
                "base_damage_bonus",
                "critical_extra_damage",
                [6_000, 7_000, 7_000, 9_000],
            ),
            (
                83,
                "critical_extra_damage",
                "base_damage_bonus",
                [8_000, 8_000, 9_000, 12_000],
            ),
            (
                86,
                "base_damage_bonus",
                "base_damage_bonus",
                [8_000, 8_000, 9_000, 12_000],
            ),
        ] {
            for grade in 0..4_u8 {
                let row = effect(&data, id, grade, input_kind)?;
                let actual = supreme_effect(id, grade, &row).ok_or("projection")?;
                assert_eq!(actual.kind, output_kind);
                assert_eq!(actual.value_milli, expected[usize::from(grade)]);
                assert!(!actual.target.starts_with("Augmented "));
            }
        }
        Ok(())
    }

    #[test]
    fn mastery_contributes_once_for_every_perk_and_momentum_is_global() -> Result<(), Error> {
        let data = WheelGemData::embedded()?;
        // The catalogue records 150/165/180/225 points. The current official test
        // planner independently shows 225 for every Mastery ID at Grade IV.
        for id in [
            5, 21, 22, 23, 39, 40, 41, 56, 57, 58, 73, 74, 75, 91, 92, 93,
        ] {
            for grade in 0..4_u8 {
                let row = effect(&data, id, grade, "revelation_mastery_points")?;
                let actual = supreme_effect(id, grade, &row).ok_or("mastery")?;
                assert_eq!(Some(actual.target), row["target"].as_str());
                assert_eq!(
                    actual.value_milli,
                    [150_000, 165_000, 180_000, 225_000][usize::from(grade)],
                    "mastery mod {id}, grade {grade}",
                );
            }
        }
        let row = effect(&data, 6, 1, "momentum_chance")?;
        let actual = supreme_effect(6, 1, &row).ok_or("momentum")?;
        assert_eq!(
            (actual.kind, actual.target, actual.value_milli),
            ("momentum_chance", "", 330)
        );
        Ok(())
    }

    #[test]
    fn monk_mixed_mana_is_preserved_alongside_resistance_and_capacity() -> Result<(), Error> {
        let data = WheelGemData::embedded()?;
        for id in 33..=36_u16 {
            let row = data.gems()["catalogue"]["basic_mods"]
                .as_array()
                .ok_or("basic rows")?
                .iter()
                .find(|row| row["source_id"].as_u64() == Some(u64::from(id)))
                .ok_or("modifier")?;
            for grade in 0..4_u8 {
                for value in row["effects"].as_array().ok_or("effects")? {
                    let actual = basic_milli(id, grade, "monk", value).ok_or("projection")?;
                    if value["name"] == "Mana" {
                        assert_eq!(
                            actual,
                            [100_000, 110_000, 120_000, 150_000][usize::from(grade)],
                            "Monk mixed Mana mod {id}, grade {grade}",
                        );
                    } else {
                        assert!(actual > 0);
                    }
                }
            }
        }
        let row = data.gems()["catalogue"]["basic_mods"]
            .as_array()
            .ok_or("basic rows")?
            .iter()
            .find(|row| row["source_id"] == 48)
            .ok_or("capacity modifier")?;
        let capacity = row["effects"]
            .as_array()
            .ok_or("effects")?
            .iter()
            .find(|effect| effect["name"] == "Capacity")
            .ok_or("capacity")?;
        for grade in 0..4_u8 {
            assert_eq!(
                basic_milli(48, grade, "monk", capacity),
                Some([500_000, 550_000, 600_000, 750_000][usize::from(grade)])
            );
        }
        Ok(())
    }

    #[test]
    fn current_socket_projection_sums_rounded_spells_and_keeps_them_targeted() -> Result<(), Error>
    {
        let data = WheelGemData::embedded()?;
        let allocation = WheelAllocation {
            wheel_ruleset_revision: Some(data.revision().into()),
            wheel_revision: 1,
            slots: WheelSlots::ZERO,
            current: true,
        };
        let mut state = AtelierState {
            vocation: "druid".into(),
            grade_ruleset_revision: Some(data.revision().into()),
            ..Default::default()
        };
        state.grades.insert((FragmentKind::Supreme, 4), 1);
        let mut effects = WheelRuntimeEffects::default();
        effects.sockets.gem_vessel_nodes = 6;
        for (id, slot) in [(1, 2), (2, 5)] {
            state.gems.insert(
                id,
                Gem {
                    id,
                    quality: GemQuality::Greater,
                    mods: vec![31, 0, 4],
                    locked: false,
                    vessel: true,
                    slot: Some(slot),
                    ruleset_revision: data.revision().into(),
                },
            );
        }
        effects
            .contribute_gems(&state, &data, &allocation)
            .ok_or("sockets")?;
        assert_eq!(
            effects.gem_value_milli("base_healing_bonus", "Ultimate Healing"),
            10_000
        );
        assert_eq!(
            effects.gem_value_milli("base_healing_bonus", "Augmented Ultimate Healing"),
            0
        );
        assert_eq!(
            effects.gem_value_milli("base_healing_bonus", "Heal Friend"),
            0
        );
        assert_eq!(effects.gem_value_milli("Physical Resistance", ""), 2_000);
        Ok(())
    }

    #[test]
    fn every_embedded_effect_is_supported_and_malformed_values_are_refused() -> Result<(), Error> {
        let data = WheelGemData::embedded()?;
        for row in data.gems()["catalogue"]["basic_mods"]
            .as_array()
            .ok_or("basic rows")?
        {
            let id = u16::try_from(row["source_id"].as_u64().ok_or("id")?)?;
            for effect in row["effects"].as_array().ok_or("effects")? {
                for vocation in ["druid", "knight", "monk", "paladin", "sorcerer"] {
                    for grade in 0..4_u8 {
                        assert!(basic_milli(id, grade, vocation, effect).is_some());
                    }
                }
            }
        }
        for row in data.gems()["catalogue"]["supreme_mods"]
            .as_array()
            .ok_or("supreme rows")?
        {
            let id = u16::try_from(row["source_id"].as_u64().ok_or("id")?)?;
            for grade in 0..4_u8 {
                for effect in row["grades"][usize::from(grade)]["numeric_effects"]
                    .as_array()
                    .ok_or("effects")?
                {
                    assert!(supreme_effect(id, grade, effect).is_some());
                }
            }
        }
        let mut row = effect(&data, 4, 0, "base_healing_bonus")?;
        assert!(supreme_effect(4, 4, &row).is_none());
        row["kind"] = Value::String("unknown".into());
        assert!(supreme_effect(4, 0, &row).is_none());
        row["kind"] = Value::String("base_healing_bonus".into());
        row["value"] = serde_json::json!(-1);
        assert!(supreme_effect(4, 0, &row).is_none());
        assert!(catalogue_milli(&serde_json::json!(0.0001)).is_none());
        assert!(catalogue_milli(&serde_json::json!("5")).is_none());
        Ok(())
    }
}
