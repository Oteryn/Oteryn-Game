//! Player spell core (plan phase P3a, `docs/architecture/OTERYN_SPELL_AUTHORING_SCHEMA_V1.md` §9).
//!
//! Server-side rules of a player cast: spoken words to a spell, the cast checks in the engine order
//! (cooldowns, level, magic level, mana, soul, learning/vocation, premium), the mana and soul debit,
//! own and group cooldowns, and the damage/heal magnitude from the spell formula. The result is data
//! for the Ability pipeline; applying it to actors is the owner commit's job.
//!
//! This module is not composed into the live server: it adds no protocol command, no persisted
//! state and no runtime slot field (those need the P3b contracts). Spell definitions come from the
//! candidate authoring bundles through [`authoring`]; the admitted route is the WorldProject/v2
//! lowering of P3b.

pub(crate) mod authoring;
pub(crate) mod formula;
pub(crate) mod plan;
#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use oteryn_simulation_determinism::SemanticTimeMicros;

pub(crate) use formula::{Formula, FormulaError, FormulaInputs};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Vocation {
    Druid,
    ElderDruid,
    Sorcerer,
    MasterSorcerer,
    Knight,
    EliteKnight,
    Paladin,
    RoyalPaladin,
    Monk,
    ExaltedMonk,
}

impl Vocation {
    pub(crate) fn from_key(key: &str) -> Option<Self> {
        Some(match key {
            "druid" => Self::Druid,
            "elder_druid" => Self::ElderDruid,
            "sorcerer" => Self::Sorcerer,
            "master_sorcerer" => Self::MasterSorcerer,
            "knight" => Self::Knight,
            "elite_knight" => Self::EliteKnight,
            "paladin" => Self::Paladin,
            "royal_paladin" => Self::RoyalPaladin,
            "monk" => Self::Monk,
            "exalted_monk" => Self::ExaltedMonk,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Carrier {
    Instant {
        words: String,
        takes_parameter: bool,
    },
    Rune {
        item: u32,
        charges: u32,
        magic_level: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManaCost {
    Fixed(u32),
    /// Percent of the caster's maximum mana (`Spell::getManaCost`).
    PercentOfMaximum(u32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CooldownGroup {
    pub(crate) key: String,
    pub(crate) cooldown_micros: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SpellEffect {
    Damage {
        damage_type: String,
        formula: Formula,
    },
    Heal {
        formula: Formula,
    },
    RemoveCondition {
        condition: String,
    },
    /// An authored effect this core does not resolve yet (conditions, fields, presentation).
    Other {
        operation: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Execution {
    Effects(Vec<SpellEffect>),
    Conjure {
        reagent: Option<u32>,
        result: u32,
        count: u32,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SpellDefinition {
    pub(crate) key: String,
    pub(crate) name: String,
    pub(crate) carrier: Carrier,
    pub(crate) vocations: BTreeSet<Vocation>,
    pub(crate) level: u32,
    pub(crate) premium: bool,
    pub(crate) learning_required: bool,
    pub(crate) mana: ManaCost,
    pub(crate) soul: u32,
    pub(crate) cooldown_micros: u64,
    pub(crate) groups: Vec<CooldownGroup>,
    pub(crate) needs_target: bool,
    pub(crate) self_target: bool,
    pub(crate) aggressive: bool,
    pub(crate) range_tiles: Option<u32>,
    pub(crate) base_power: Option<i64>,
    pub(crate) execution: Execution,
}

/// Two admitted spells claim the same words, rune item or key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SpellBookError {
    Words(String),
    Rune(u32),
    Key(String),
}

impl Display for SpellBookError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Words(words) => {
                write!(formatter, "two spells share the words {words:?}")
            }
            Self::Rune(item) => write!(formatter, "two rune spells share the item {item}"),
            Self::Key(key) => write!(formatter, "two spells share the key {key}"),
        }
    }
}

impl Error for SpellBookError {}

/// The admitted spells of a world, indexed by spoken words and by rune item.
#[derive(Debug, Clone, Default)]
pub(crate) struct SpellBook {
    spells: Vec<SpellDefinition>,
    by_words: BTreeMap<String, usize>,
    by_rune: BTreeMap<u32, usize>,
}

/// A spoken message that names a spell.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SpokenSpell<'a> {
    pub(crate) spell: &'a SpellDefinition,
    pub(crate) parameter: Option<String>,
}

impl SpellBook {
    pub(crate) fn new(spells: Vec<SpellDefinition>) -> Result<Self, SpellBookError> {
        let mut book = Self::default();
        let mut keys = BTreeSet::new();
        for (index, spell) in spells.iter().enumerate() {
            if !keys.insert(spell.key.clone()) {
                return Err(SpellBookError::Key(spell.key.clone()));
            }
            match &spell.carrier {
                Carrier::Instant { words, .. } => {
                    if book.by_words.insert(words.clone(), index).is_some() {
                        return Err(SpellBookError::Words(words.clone()));
                    }
                }
                Carrier::Rune { item, .. } => {
                    if book.by_rune.insert(*item, index).is_some() {
                        return Err(SpellBookError::Rune(*item));
                    }
                }
            }
        }
        book.spells = spells;
        Ok(book)
    }

    /// The instant spell a spoken message casts, if any. Words compare case-insensitively with
    /// collapsed whitespace; a spell that takes a parameter also matches `words "parameter`.
    pub(crate) fn spoken(&self, message: &str) -> Option<SpokenSpell<'_>> {
        let normalized = message
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();
        if let Some(&index) = self.by_words.get(&normalized) {
            return Some(SpokenSpell {
                spell: &self.spells[index],
                parameter: None,
            });
        }
        let (words, parameter) = normalized.split_once(" \"")?;
        let &index = self.by_words.get(words)?;
        let spell = &self.spells[index];
        match spell.carrier {
            Carrier::Instant {
                takes_parameter: true,
                ..
            } => {
                let parameter = parameter.trim_end_matches('"').trim();
                Some(SpokenSpell {
                    spell,
                    parameter: (!parameter.is_empty()).then(|| parameter.to_owned()),
                })
            }
            _ => None,
        }
    }

    pub(crate) fn rune(&self, item: u32) -> Option<&SpellDefinition> {
        self.by_rune.get(&item).map(|&index| &self.spells[index])
    }
}

/// What the cast checks read from the caster.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CasterState {
    pub(crate) vocation: Vocation,
    pub(crate) level: u32,
    pub(crate) magic_level: u32,
    pub(crate) premium: bool,
    pub(crate) mana: u32,
    pub(crate) max_mana: u32,
    pub(crate) soul: u32,
    pub(crate) learned: BTreeSet<String>,
    pub(crate) attack_skill: u32,
    pub(crate) attack_value: u32,
    pub(crate) attack_factor: f64,
    pub(crate) shielding_skill: u32,
}

/// Ready times of a caster's spell and group cooldowns.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Cooldowns {
    spells: BTreeMap<String, SemanticTimeMicros>,
    groups: BTreeMap<String, SemanticTimeMicros>,
}

impl Cooldowns {
    pub(crate) fn spell_ready_at(&self, key: &str) -> Option<SemanticTimeMicros> {
        self.spells.get(key).copied()
    }

    pub(crate) fn group_ready_at(&self, group: &str) -> Option<SemanticTimeMicros> {
        self.groups.get(group).copied()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CastRejection {
    GroupCooling {
        group: String,
        ready_at: SemanticTimeMicros,
    },
    SpellCooling {
        ready_at: SemanticTimeMicros,
    },
    LevelTooLow {
        required: u32,
    },
    MagicLevelTooLow {
        required: u32,
    },
    NotEnoughMana {
        required: u32,
    },
    NotEnoughSoul {
        required: u32,
    },
    NotLearned,
    VocationCannotUse,
    PremiumRequired,
    TargetRequired,
    TimeOverflow,
    Formula(FormulaError),
}

impl Display for CastRejection {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::GroupCooling { group, .. } => {
                write!(formatter, "the {group} group is cooling down")
            }
            Self::SpellCooling { .. } => formatter.write_str("the spell is cooling down"),
            Self::LevelTooLow { required } => write!(formatter, "level {required} is required"),
            Self::MagicLevelTooLow { required } => {
                write!(formatter, "magic level {required} is required")
            }
            Self::NotEnoughMana { required } => write!(formatter, "{required} mana is required"),
            Self::NotEnoughSoul { required } => {
                write!(formatter, "{required} soul points are required")
            }
            Self::NotLearned => formatter.write_str("the spell has not been learned"),
            Self::VocationCannotUse => formatter.write_str("the vocation cannot use this spell"),
            Self::PremiumRequired => formatter.write_str("a premium account is required"),
            Self::TargetRequired => formatter.write_str("the spell needs a target"),
            Self::TimeOverflow => formatter.write_str("cooldown time overflow"),
            Self::Formula(error) => write!(formatter, "formula: {error}"),
        }
    }
}

impl Error for CastRejection {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ResolvedEffect {
    Damage {
        damage_type: String,
        magnitude: i64,
    },
    Heal {
        magnitude: i64,
    },
    RemoveCondition {
        condition: String,
    },
    Conjure {
        reagent: Option<u32>,
        result: u32,
        count: u32,
    },
    Unresolved {
        operation: String,
    },
}

/// The outcome of an accepted cast; nothing is applied yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CastResolution {
    pub(crate) mana_spent: u32,
    pub(crate) soul_spent: u32,
    pub(crate) effects: Vec<ResolvedEffect>,
    pub(crate) cooldowns: Cooldowns,
}

/// Mana a cast costs this caster (`Spell::getManaCost`: a fixed amount or a share of maximum mana).
pub(crate) fn mana_cost(spell: &SpellDefinition, caster: &CasterState) -> u32 {
    match spell.mana {
        ManaCost::Fixed(mana) => mana,
        ManaCost::PercentOfMaximum(percent) => {
            u32::try_from(u64::from(caster.max_mana) * u64::from(percent) / 100).unwrap_or(u32::MAX)
        }
    }
}

/// Check and resolve one cast at `now`. `draw(minimum, maximum)` is the world damage distribution.
pub(crate) fn resolve_cast(
    spell: &SpellDefinition,
    caster: &CasterState,
    cooldowns: &Cooldowns,
    now: SemanticTimeMicros,
    has_target: bool,
    draw: &mut dyn FnMut(i64, i64) -> i64,
) -> Result<CastResolution, CastRejection> {
    // Order of Canary Spell::playerSpellCheck (group, spell, secondary group cooldowns first).
    let mut groups = spell.groups.iter();
    if let Some(primary) = groups.next() {
        check_group(cooldowns, &primary.key, now)?;
    }
    if let Some(ready_at) = cooldowns
        .spell_ready_at(&spell.key)
        .filter(|ready_at| *ready_at > now)
    {
        return Err(CastRejection::SpellCooling { ready_at });
    }
    for group in groups {
        check_group(cooldowns, &group.key, now)?;
    }
    if caster.level < spell.level {
        return Err(CastRejection::LevelTooLow {
            required: spell.level,
        });
    }
    if let Carrier::Rune { magic_level, .. } = spell.carrier
        && caster.magic_level < magic_level
    {
        return Err(CastRejection::MagicLevelTooLow {
            required: magic_level,
        });
    }
    let mana = mana_cost(spell, caster);
    if caster.mana < mana {
        return Err(CastRejection::NotEnoughMana { required: mana });
    }
    if caster.soul < spell.soul {
        return Err(CastRejection::NotEnoughSoul {
            required: spell.soul,
        });
    }
    // Canary checks the vocation only for a spell that is not learned (learning already required it).
    if spell.learning_required {
        if !caster.learned.contains(&spell.key) {
            return Err(CastRejection::NotLearned);
        }
    } else if !spell.vocations.contains(&caster.vocation) {
        return Err(CastRejection::VocationCannotUse);
    }
    if spell.premium && !caster.premium {
        return Err(CastRejection::PremiumRequired);
    }
    if spell.needs_target && !has_target {
        return Err(CastRejection::TargetRequired);
    }
    let inputs = FormulaInputs {
        level: caster.level,
        magic_level: caster.magic_level,
        base_power: spell.base_power,
        attack_skill: caster.attack_skill,
        attack_value: caster.attack_value,
        attack_factor: caster.attack_factor,
        shielding_skill: caster.shielding_skill,
    };
    let effects = match &spell.execution {
        Execution::Conjure {
            reagent,
            result,
            count,
        } => {
            vec![ResolvedEffect::Conjure {
                reagent: *reagent,
                result: *result,
                count: *count,
            }]
        }
        Execution::Effects(effects) => effects
            .iter()
            .map(|effect| resolve_effect(effect, &inputs, draw))
            .collect::<Result<_, _>>()
            .map_err(CastRejection::Formula)?,
    };
    let mut after = cooldowns.clone();
    let ready = |delay: u64| {
        now.checked_add(delay)
            .map_err(|_| CastRejection::TimeOverflow)
    };
    after
        .spells
        .insert(spell.key.clone(), ready(spell.cooldown_micros)?);
    for group in &spell.groups {
        after
            .groups
            .insert(group.key.clone(), ready(group.cooldown_micros)?);
    }
    Ok(CastResolution {
        mana_spent: mana,
        soul_spent: spell.soul,
        effects,
        cooldowns: after,
    })
}

fn check_group(
    cooldowns: &Cooldowns,
    group: &str,
    now: SemanticTimeMicros,
) -> Result<(), CastRejection> {
    match cooldowns.group_ready_at(group) {
        Some(ready_at) if ready_at > now => Err(CastRejection::GroupCooling {
            group: group.to_owned(),
            ready_at,
        }),
        _ => Ok(()),
    }
}

fn resolve_effect(
    effect: &SpellEffect,
    inputs: &FormulaInputs,
    draw: &mut dyn FnMut(i64, i64) -> i64,
) -> Result<ResolvedEffect, FormulaError> {
    Ok(match effect {
        SpellEffect::Damage {
            damage_type,
            formula,
        } => {
            let (low, high) = formula.bounds(inputs)?;
            ResolvedEffect::Damage {
                damage_type: damage_type.clone(),
                magnitude: draw(low, high).clamp(low, high),
            }
        }
        SpellEffect::Heal { formula } => {
            let (low, high) = formula.bounds(inputs)?;
            ResolvedEffect::Heal {
                magnitude: draw(low, high).clamp(low, high),
            }
        }
        SpellEffect::RemoveCondition { condition } => ResolvedEffect::RemoveCondition {
            condition: condition.clone(),
        },
        SpellEffect::Other { operation } => ResolvedEffect::Unresolved {
            operation: operation.clone(),
        },
    })
}

/// Uniform draw in `[minimum, maximum]` from one deterministic decision value. Canary and Crystal
/// draw with `normal_random`; the Reference distribution is not evidenced, so this stays a
/// replaceable world rule.
pub(crate) fn uniform_draw(decision: u64, minimum: i64, maximum: i64) -> i64 {
    if maximum <= minimum {
        return minimum;
    }
    let span = u64::try_from(maximum - minimum)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let offset = i64::try_from(decision % span).unwrap_or(0);
    minimum + offset
}
