//! The ATTACK-0 §5 constants table (`content/combat/attack_constants_v1.json`).
//!
//! Every value is Canary-derived (`OTS_HYPOTHESIS_ONLY`) and carries `PARITY_PENDING` until
//! ATTACK-PARITY-1 matches it against the TibiaPal calculator (§6).

use std::sync::OnceLock;

const TABLE_JSON: &str = include_str!("../../../../../content/combat/attack_constants_v1.json");
const TABLE_SCHEMA: &str = "OTERYN_ATTACK_CONSTANTS/v1";
/// The only parity state this slice admits; ATTACK-PARITY-1 changes it.
const PARITY_PENDING: &str = "PARITY_PENDING";

/// Fight mode (`FIGHT_MODES_INTENT`, ATTACK-0 §3); balanced is the default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum FightMode {
    Offensive,
    #[default]
    Balanced,
    Defensive,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FightModeConstants {
    /// `getAttackFactor` (`player.cpp:840-851`).
    pub(crate) attack_factor: f64,
    /// `getDefenseFactor` while the defender swung within its attack interval
    /// (`player.cpp:853-872`); 1.0 otherwise.
    pub(crate) defence_factor_after_swing: f64,
    /// The defence value of a defending skill of 0 (`player.cpp:776-818`).
    pub(crate) zero_skill_defence: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FightModes {
    pub(crate) offensive: FightModeConstants,
    pub(crate) balanced: FightModeConstants,
    pub(crate) defensive: FightModeConstants,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PlayerMeleeConstants {
    pub(crate) damage_coefficient: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FistConstants {
    pub(crate) attack: u32,
    pub(crate) defence: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreatureMeleeConstants {
    pub(crate) skill_attack_coefficient: f64,
    pub(crate) attack_coefficient: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DefenceConstants {
    pub(crate) skill_divisor: f64,
    pub(crate) skill_offset: f64,
    pub(crate) scaling_shield: f64,
    pub(crate) scaling_weapon: f64,
    pub(crate) scaling_fist: f64,
    pub(crate) scaling_creature: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ArmorConstants {
    /// Armor from 1 up to this value removes exactly 1 damage (`creature.cpp:976-982`).
    pub(crate) flat_reduction_max_armor: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BlockConstants {
    pub(crate) refill_ms: u64,
    pub(crate) max_blocks: u8,
    pub(crate) initial_blocks: u8,
}

/// The checked-in ATTACK-0 constants.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AttackConstants {
    schema: String,
    #[allow(dead_code, reason = "provenance text")]
    decision: String,
    #[allow(dead_code, reason = "provenance text")]
    source: String,
    parity: String,
    pub(crate) attack_interval_ms: u64,
    pub(crate) in_fight_ms: u64,
    /// The skill every character fights with until CHAR-BUILD-1 and SKILLS-0 ship (§4).
    pub(crate) starting_skill: u32,
    pub(crate) fight_modes: FightModes,
    pub(crate) player_melee: PlayerMeleeConstants,
    pub(crate) fist: FistConstants,
    pub(crate) creature_melee: CreatureMeleeConstants,
    pub(crate) defence: DefenceConstants,
    pub(crate) armor: ArmorConstants,
    pub(crate) block: BlockConstants,
}

impl AttackConstants {
    /// Parse and check a table: schema, parity state, nonzero periods and in-range factors.
    pub(crate) fn parse(text: &str) -> Option<Self> {
        let table: Self = serde_json::from_str(text).ok()?;
        let modes = [
            table.fight_modes.offensive,
            table.fight_modes.balanced,
            table.fight_modes.defensive,
        ];
        let defence = table.defence;
        let valid = table.schema == TABLE_SCHEMA
            && table.parity == PARITY_PENDING
            && table.attack_interval_ms > 0
            && table.in_fight_ms > 0
            && table.block.refill_ms > 0
            && table.block.initial_blocks <= table.block.max_blocks
            && modes.iter().all(|mode| {
                unit_factor(mode.attack_factor) && unit_factor(mode.defence_factor_after_swing)
            })
            && positive(table.player_melee.damage_coefficient)
            && positive(table.creature_melee.skill_attack_coefficient)
            && positive(table.creature_melee.attack_coefficient)
            && positive(defence.skill_divisor)
            && defence.skill_offset.is_finite()
            && defence.skill_offset >= 0.0
            && [
                defence.scaling_shield,
                defence.scaling_weapon,
                defence.scaling_fist,
                defence.scaling_creature,
            ]
            .into_iter()
            .all(positive);
        valid.then_some(table)
    }

    /// The checked-in table, parsed once. `None` if the embedded file does not check, which the
    /// tests rule out.
    pub(crate) fn checked_in() -> Option<&'static Self> {
        static TABLE: OnceLock<Option<AttackConstants>> = OnceLock::new();
        TABLE.get_or_init(|| Self::parse(TABLE_JSON)).as_ref()
    }

    pub(crate) fn fight_mode(&self, mode: FightMode) -> FightModeConstants {
        match mode {
            FightMode::Offensive => self.fight_modes.offensive,
            FightMode::Balanced => self.fight_modes.balanced,
            FightMode::Defensive => self.fight_modes.defensive,
        }
    }

    pub(crate) fn attack_interval_micros(&self) -> u64 {
        self.attack_interval_ms.saturating_mul(1_000)
    }

    pub(crate) fn in_fight_micros(&self) -> u64 {
        self.in_fight_ms.saturating_mul(1_000)
    }

    pub(crate) fn block_refill_micros(&self) -> u64 {
        self.block.refill_ms.saturating_mul(1_000)
    }
}

fn unit_factor(value: f64) -> bool {
    value.is_finite() && value > 0.0 && value <= 1.0
}

fn positive(value: f64) -> bool {
    value.is_finite() && value > 0.0
}

#[cfg(test)]
pub(crate) const CHECKED_IN_JSON: &str = TABLE_JSON;
