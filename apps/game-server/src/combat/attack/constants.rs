//! The ATTACK-0 §5 constants table (`content/combat/attack_constants_v1.json`).
//!
//! Every value is Canary-derived (`OTS_HYPOTHESIS_ONLY`) and carries `PARITY_PENDING` until
//! ATTACK-PARITY-1 matches it against the TibiaPal calculator (§6).

use std::sync::OnceLock;

const TABLE_JSON: &str = include_str!("../../../../../content/combat/attack_constants_v1.json");
const TABLE_SCHEMA: &str = "OTERYN_ATTACK_CONSTANTS/v1";
/// The only parity state this slice admits; ATTACK-PARITY-1 changes it.
const PARITY_PENDING: &str = "PARITY_PENDING";

/// Every value the table labels (packet 1.1); the parser requires exactly this key set.
const PARITY_VALUE_KEYS: [&str; 10] = [
    "player_melee_offensive_attack",
    "player_melee_balanced_attack",
    "player_melee_defensive_attack",
    "monk_melee",
    "defence",
    "armor",
    "block",
    "creature_melee",
    "attack_interval_ms",
    "in_fight_ms",
];

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
    /// `floor(attack * numerator / denominator)`: the TibiaTools `floor(6 * attack / 5)` step.
    pub(crate) attack_numerator: u32,
    pub(crate) attack_denominator: u32,
    /// `v = stepped attack * (skill + skill_offset) / skill_divisor`.
    pub(crate) skill_offset: u32,
    pub(crate) skill_divisor: u32,
    /// `min = floor(level term + v / minimum_divisor)`.
    pub(crate) minimum_divisor: u32,
    /// `max = floor(level term + maximum_multiplier * v)`.
    pub(crate) maximum_multiplier: u32,
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
    /// The parity state of each value (ATTACK-PARITY-1 packet 1.4); every value stays pending
    /// until its comparison passes.
    #[allow(
        dead_code,
        reason = "per-value parity labels are checked by parse and the parity tests"
    )]
    parity_by_value: std::collections::BTreeMap<String, String>,
    #[allow(dead_code, reason = "provenance text")]
    player_melee_source: String,
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
            && table.parity_by_value.len() == PARITY_VALUE_KEYS.len()
            && PARITY_VALUE_KEYS.iter().all(|key| {
                table.parity_by_value.get(*key).map(String::as_str) == Some(PARITY_PENDING)
            })
            && table.attack_interval_ms > 0
            && table.in_fight_ms > 0
            && table.block.refill_ms > 0
            && table.block.initial_blocks <= table.block.max_blocks
            && modes.iter().all(|mode| {
                unit_factor(mode.attack_factor) && unit_factor(mode.defence_factor_after_swing)
            })
            && table.player_melee.attack_numerator > 0
            && table.player_melee.attack_denominator > 0
            && table.player_melee.skill_divisor > 0
            && table.player_melee.minimum_divisor > 0
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

    #[allow(
        dead_code,
        reason = "ATTACK-2 composes weapon melee and creature block, defence and armor; player defence, distance and the scaled defence formula are follow-ups"
    )]
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
