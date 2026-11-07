//! Executed Crystal gem values at 201189f. The imported catalogue also carries client
//! descriptions; source integer assignments and the differences below are applied once,
//! before contributions from separate gems are added.

use super::WheelRuntimeEffects;
use serde_json::Value;

impl WheelRuntimeEffects {
    /// Signed thousandths of the declared unit. Spell targets are the source spell name
    /// without the catalogue's `Augmented ` display prefix. Momentum has an empty target.
    pub fn gem_value_milli(&self, kind: &str, target: &str) -> i64 {
        self.sockets
            .gem_values_milli
            .iter()
            .find_map(|((key, name), value)| (key == kind && name == target).then_some(*value))
            .unwrap_or(0)
    }
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

pub(super) fn basic_milli(id: u16, grade: u8, vocation: &str, effect: &Value) -> Option<i64> {
    if grade > 3 {
        return None;
    }
    let name = effect["name"].as_str()?;
    let value = catalogue_milli(&effect["values_by_vocation"][vocation][usize::from(grade)])?;
    match name {
        // Crystal wheel_gems.cpp:741-800,817-824: the four mixed Mana maps have no
        // Monk entry and return zero; their independently added resistance remains active.
        "Mana" if vocation == "monk" && (33..=36).contains(&id) => Some(0),
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

pub(super) struct SupremeEffect<'a> {
    pub kind: &'a str,
    pub target: &'a str,
    pub value_milli: i64,
}

pub(super) fn supreme_effect(id: u16, grade: u8, effect: &Value) -> Option<SupremeEffect<'_>> {
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
            // Primary-source behavior: each case adds the points immediately, then
            // processActiveGems executes the same strategy once (wheel_gems.cpp:261-263,
            // 271-320,633-643; player_wheel.cpp:2757). Preserve both additions explicitly.
            (value / 1_000).checked_mul(2)?.checked_mul(1_000)?
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
    use crate::wheel::{WheelAllocation, WheelDomain, WheelSlots};
    use crate::wheel_gem_data::WheelGemData;

    type Error = Box<dyn std::error::Error>;

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
    fn mastery_preserves_both_source_additions_and_momentum_is_global() -> Result<(), Error> {
        let data = WheelGemData::embedded()?;
        for grade in 0..4_u8 {
            let row = effect(&data, 5, grade, "revelation_mastery_points")?;
            let actual = supreme_effect(5, grade, &row).ok_or("mastery")?;
            assert_eq!(actual.target, "Gift of Life");
            assert_eq!(
                actual.value_milli,
                [300_000, 330_000, 360_000, 450_000][usize::from(grade)]
            );
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
    fn monk_mana_and_capacity_follow_source_tables_without_dropping_resistance() -> Result<(), Error>
    {
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
                        assert_eq!(actual, 0);
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
        for (id, domain, resonance) in [
            (1, WheelDomain::Green, "vessel_resonance_top_left"),
            (2, WheelDomain::Red, "vessel_resonance_top_right"),
        ] {
            state.gems.insert(
                id,
                Gem {
                    id,
                    quality: GemQuality::Greater,
                    domain,
                    mods: vec![31, 0, 4],
                    locked: false,
                    vessel: true,
                    ruleset_revision: data.revision().into(),
                },
            );
            effects.sockets.resonance_stages.insert(resonance.into(), 3);
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
