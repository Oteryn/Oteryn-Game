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
pub(crate) mod chain;
#[cfg(test)]
mod chain_tests;
pub(crate) mod formula;
pub(crate) mod locate;
#[cfg(test)]
mod part_b_tests;
#[cfg(test)]
mod part_d_tests;
pub(crate) mod party;
#[cfg(test)]
mod party_tests;
pub(crate) mod plan;
pub(crate) mod target;
#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use oteryn_simulation_determinism::SemanticTimeMicros;

use chain::{ChainCreature, ChainHit, ChainSpec, ChainStart, ChainWorld, pick_chain, step_value};
pub(crate) use formula::{Formula, FormulaError, FormulaInputs};
use party::{PartyBuffSpec, PartyFailure, PartyWorld};
use target::{AllowedTargets, CastTarget, CheckedTarget};

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
    /// An authored effect this core does not resolve yet (conditions, fields, presentation);
    /// `effect` is its Effect key for the owner that applies it.
    Other {
        operation: String,
        effect: String,
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
    /// `native_behavior` `party_buff` (part C.3).
    PartyBuff(PartyBuffSpec),
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
    /// Cast at a target or, without one, in the looking direction (Canary `needCasterTargetOrDirection`).
    pub(crate) target_or_direction: bool,
    pub(crate) self_target: bool,
    /// Who the cast may be aimed at (D.3 and B.5, `targeting.allowed_targets`).
    pub(crate) allowed_targets: AllowedTargets,
    pub(crate) aggressive: bool,
    /// The caster must wield a melee weapon (Canary/Crystal `needWeapon`; D.4).
    pub(crate) needs_weapon: bool,
    /// The caster must wield a shield (D.4, `needs_shield`).
    pub(crate) needs_shield: bool,
    pub(crate) range_tiles: Option<u32>,
    pub(crate) base_power: Option<i64>,
    pub(crate) execution: Execution,
    /// The ability hits a chain of creatures (D12, S23).
    pub(crate) chain: Option<ChainSpec>,
}

impl SpellDefinition {
    /// Whether a cast chains: a chain spell cast by direction, without a target, hits the tile in
    /// front and does not chain (Canary `lightning.lua`).
    pub(crate) fn chains(&self, has_target: bool) -> bool {
        self.chain.is_some() && (has_target || !self.target_or_direction)
    }

    fn takes_target(&self) -> bool {
        self.needs_target || self.target_or_direction
    }
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
                    // Spoken words match case-insensitively, so they are indexed and deduplicated
                    // that way too.
                    if book
                        .by_words
                        .insert(words.to_ascii_lowercase(), index)
                        .is_some()
                    {
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

    /// The instant spell a spoken message casts, if any (part B.3 P1 and P2, Canary
    /// `getInstantSpell` and `playerSaySpell`). Whitespace runs collapse to one space and the ends
    /// are trimmed. The spell whose words are the longest case-insensitive prefix is chosen; a
    /// spell without a parameter must match exactly, one with a parameter needs a space and at
    /// least one more character after its words. A parameter that opens with a quote is the text
    /// up to the next quote (an unclosed quote runs to the end; text after the closing quote makes
    /// the message chat); any other parameter is a single word kept as spoken, quotes included
    /// (two words make it chat). It keeps the
    /// spoken case; an empty parameter is `None`. `None` means the message is chat.
    pub(crate) fn spoken(&self, message: &str) -> Option<SpokenSpell<'_>> {
        let message = message.split_whitespace().collect::<Vec<_>>().join(" ");
        let (words, &index) = self
            .by_words
            .iter()
            .filter(|(words, _)| {
                message
                    .as_bytes()
                    .get(..words.len())
                    .is_some_and(|prefix| prefix.eq_ignore_ascii_case(words.as_bytes()))
            })
            .max_by_key(|(words, _)| words.len())?;
        let spell = &self.spells[index];
        let takes_parameter = matches!(
            spell.carrier,
            Carrier::Instant {
                takes_parameter: true,
                ..
            }
        );
        let rest = message.get(words.len()..)?;
        if rest.is_empty() {
            return Some(SpokenSpell {
                spell,
                parameter: None,
            });
        }
        let parameter = rest.strip_prefix(' ').filter(|_| takes_parameter)?;
        let parameter = match parameter.strip_prefix('"') {
            Some(quoted) => match quoted.find('"') {
                None => quoted,
                Some(close) if close + 1 == quoted.len() => &quoted[..close],
                Some(_) => return None,
            },
            None if parameter.contains(' ') => return None,
            None => parameter,
        };
        Some(SpokenSpell {
            spell,
            parameter: (!parameter.is_empty()).then(|| parameter.to_owned()),
        })
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
    /// The caster wields a sword, club, axe or fist weapon (Canary/Crystal `needWeapon`), as the
    /// equipment owner reports it.
    pub(crate) melee_weapon: bool,
    /// Defense of the first shield in the caster's left or right hand; `None` without a shield (D.4).
    pub(crate) shield_defense: Option<u32>,
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
    /// A `needs_weapon` spell cast without a melee weapon (D.4); nothing is spent.
    WeaponRequired,
    /// A `needs_shield` spell cast without a shield (D.4.1); nothing is spent.
    ShieldRequired,
    /// The target is not one `allowed_targets` permits (D.3.1 step 1, B.5 step 1); nothing is
    /// spent.
    TargetNotAllowed,
    /// A spell with `allowed_targets` was resolved with a target but without the target facts
    /// ([`resolve_targeted_cast`]).
    TargetFactsRequired,
    /// A chain cast found no first creature (chain §3 step 2); nothing is spent.
    NoChainTarget,
    /// A chain cast was resolved without the world facts it needs ([`resolve_chain_cast`]).
    ChainWorldRequired,
    /// A party buff found no party, or too few members in its area (C.3 steps 1 and 3); nothing
    /// is spent.
    NoPartyMembers,
    /// A party buff was resolved without the party facts it needs ([`resolve_party_cast`]).
    PartyWorldRequired,
    /// A party buff's scaled mana does not fit the exact integer computation.
    PartyCostOverflow,
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
            Self::WeaponRequired => formatter.write_str("a weapon is required"),
            Self::ShieldRequired => formatter.write_str("a shield is required"),
            Self::TargetNotAllowed => {
                formatter.write_str("the spell cannot be used on this target")
            }
            Self::TargetFactsRequired => {
                formatter.write_str("a targeted cast needs the target facts")
            }
            Self::NoChainTarget => formatter.write_str("no valid creature is in range"),
            Self::ChainWorldRequired => formatter.write_str("a chain cast needs the world facts"),
            Self::NoPartyMembers => formatter.write_str("no party members in range"),
            Self::PartyWorldRequired => formatter.write_str("a party cast needs the party facts"),
            Self::PartyCostOverflow => formatter.write_str("party mana cost overflow"),
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
        effect: String,
    },
}

/// One creature of a chain cast and the spell's effects on it, scaled by its step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChainHitResolution {
    pub(crate) hit: ChainHit,
    pub(crate) effects: Vec<ResolvedEffect>,
}

/// One party member a party buff affects and the effects it receives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PartyMemberResolution {
    pub(crate) creature: u64,
    pub(crate) actor: String,
    pub(crate) effects: Vec<ResolvedEffect>,
}

/// The outcome of an accepted cast; nothing is applied yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CastResolution {
    pub(crate) mana_spent: u32,
    pub(crate) soul_spent: u32,
    /// Effects of a cast that neither chains nor buffs a party.
    pub(crate) effects: Vec<ResolvedEffect>,
    /// Creatures of a chain cast in hit order, each with its own effects; empty otherwise.
    pub(crate) chain: Vec<ChainHitResolution>,
    /// Members a party buff affects, by creature id, each with its own effects; empty otherwise.
    pub(crate) party: Vec<PartyMemberResolution>,
    /// The target of a cast by [`resolve_targeted_cast`]; its plan applies to this creature only.
    pub(crate) target: Option<CheckedTarget>,
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

/// The world facts a cast reads beyond the caster.
#[derive(Clone, Copy)]
enum Facts<'a> {
    None,
    Chain(&'a dyn ChainWorld, ChainStart),
    Party(&'a dyn PartyWorld),
    Target(&'a CastTarget),
}

/// Check and resolve one cast at `now`. `draw(minimum, maximum)` is the world damage distribution.
/// A cast that chains needs [`resolve_chain_cast`], a party buff [`resolve_party_cast`], a cast
/// at a target of a spell with `allowed_targets` [`resolve_targeted_cast`].
pub(crate) fn resolve_cast(
    spell: &SpellDefinition,
    caster: &CasterState,
    cooldowns: &Cooldowns,
    now: SemanticTimeMicros,
    has_target: bool,
    draw: &mut dyn FnMut(i64, i64) -> i64,
) -> Result<CastResolution, CastRejection> {
    if spell.chains(has_target) {
        return Err(CastRejection::ChainWorldRequired);
    }
    if has_target && spell.allowed_targets != AllowedTargets::Any {
        return Err(CastRejection::TargetFactsRequired);
    }
    resolve(spell, caster, cooldowns, now, has_target, Facts::None, draw)
}

/// [`resolve_cast`] at a resolved target creature (part D.3). The cast checks run first; then a
/// target that `allowed_targets` does not permit fails the cast before anything is spent.
pub(crate) fn resolve_targeted_cast(
    spell: &SpellDefinition,
    caster: &CasterState,
    cooldowns: &Cooldowns,
    now: SemanticTimeMicros,
    target: CastTarget,
    draw: &mut dyn FnMut(i64, i64) -> i64,
) -> Result<CastResolution, CastRejection> {
    resolve(
        spell,
        caster,
        cooldowns,
        now,
        true,
        Facts::Target(&target),
        draw,
    )
}

/// [`resolve_cast`] for a party buff (part C.3), with the party facts it reads. The cast checks
/// run first; then a caster without a party, or with fewer than `min_affected` members in the
/// area, fails the cast, and then the caster must have the computed mana. Nothing is spent on a
/// failure. Each affected member draws its own values.
pub(crate) fn resolve_party_cast(
    spell: &SpellDefinition,
    caster: &CasterState,
    cooldowns: &Cooldowns,
    now: SemanticTimeMicros,
    world: &dyn PartyWorld,
    draw: &mut dyn FnMut(i64, i64) -> i64,
) -> Result<CastResolution, CastRejection> {
    resolve(
        spell,
        caster,
        cooldowns,
        now,
        false,
        Facts::Party(world),
        draw,
    )
}

/// [`resolve_cast`] for any spell, with the world facts a chain reads (chain §3). The cast checks
/// run first; a chain with no first creature then fails the cast before anything is spent. Each
/// creature hit draws its own value (Canary `doCombat` per chain target), scaled by its step.
pub(crate) fn resolve_chain_cast(
    spell: &SpellDefinition,
    caster: &CasterState,
    cooldowns: &Cooldowns,
    now: SemanticTimeMicros,
    world: &dyn ChainWorld,
    start: ChainStart,
    draw: &mut dyn FnMut(i64, i64) -> i64,
) -> Result<CastResolution, CastRejection> {
    // A spell that takes no target starts from the attacked or the nearest creature (§3 step 1).
    let start = ChainStart {
        target: start.target.filter(|_| spell.takes_target()),
        ..start
    };
    resolve(
        spell,
        caster,
        cooldowns,
        now,
        start.target.is_some(),
        Facts::Chain(world, start),
        draw,
    )
}

fn resolve(
    spell: &SpellDefinition,
    caster: &CasterState,
    cooldowns: &Cooldowns,
    now: SemanticTimeMicros,
    has_target: bool,
    facts: Facts<'_>,
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
    let mut mana = mana_cost(spell, caster);
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
    // Canary/Crystal check the weapon after the vocation and before premium.
    if spell.needs_weapon && !caster.melee_weapon {
        return Err(CastRejection::WeaponRequired);
    }
    if spell.premium && !caster.premium {
        return Err(CastRejection::PremiumRequired);
    }
    if spell.needs_target && !has_target {
        return Err(CastRejection::TargetRequired);
    }
    // The script checks of D.3 and D.4 run after the engine checks (Canary `onCastSpell`).
    if spell.allowed_targets != AllowedTargets::Any
        && let Facts::Target(target) = facts
        && !spell.allowed_targets.allows(target)
    {
        return Err(CastRejection::TargetNotAllowed);
    }
    if spell.needs_shield && caster.shield_defense.is_none() {
        return Err(CastRejection::ShieldRequired);
    }
    let hits = match (&spell.chain, facts) {
        (Some(chain), Facts::Chain(world, start)) if spell.chains(has_target) => {
            let target_range = spell.range_tiles.unwrap_or(chain.initial_range_tiles);
            let hits = pick_chain(chain, world, start, target_range);
            if hits.is_empty() {
                return Err(CastRejection::NoChainTarget);
            }
            hits
        }
        (Some(_), _) if spell.chains(has_target) => {
            return Err(CastRejection::ChainWorldRequired);
        }
        _ => Vec::new(),
    };
    let members = match (&spell.execution, facts) {
        (Execution::PartyBuff(buff), Facts::Party(world)) => {
            let members = buff.affected(world).map_err(party_rejection)?;
            // The authored `costs.mana` is 0 for a party buff (reader rule); the buff's mana is added.
            mana = buff
                .mana_cost(members.len())
                .ok()
                .and_then(|cost| mana.checked_add(cost))
                .ok_or(CastRejection::PartyCostOverflow)?;
            if caster.mana < mana {
                return Err(CastRejection::NotEnoughMana { required: mana });
            }
            members
        }
        (Execution::PartyBuff(_), _) => return Err(CastRejection::PartyWorldRequired),
        _ => Vec::new(),
    };
    let inputs = FormulaInputs {
        level: caster.level,
        magic_level: caster.magic_level,
        base_power: spell.base_power,
        attack_skill: caster.attack_skill,
        attack_value: caster.attack_value,
        attack_factor: caster.attack_factor,
        shielding_skill: caster.shielding_skill,
        shield_defense: caster.shield_defense,
    };
    let resolve_effects = |draw: &mut dyn FnMut(i64, i64) -> i64| match &spell.execution {
        Execution::Conjure {
            reagent,
            result,
            count,
        } => Ok(vec![ResolvedEffect::Conjure {
            reagent: *reagent,
            result: *result,
            count: *count,
        }]),
        Execution::Effects(effects) => effects
            .iter()
            .map(|effect| resolve_effect(effect, &inputs, draw))
            .collect::<Result<Vec<_>, _>>()
            .map_err(CastRejection::Formula),
        Execution::PartyBuff(buff) => buff
            .effects
            .iter()
            .map(|effect| resolve_effect(effect, &inputs, draw))
            .collect::<Result<Vec<_>, _>>()
            .map_err(CastRejection::Formula),
    };
    let step_percent = spell
        .chain
        .as_ref()
        .map_or(0, |chain| chain.damage_step_percent);
    let party = members
        .into_iter()
        .map(|member: &ChainCreature| {
            Ok(PartyMemberResolution {
                creature: member.id,
                actor: member.actor.clone(),
                effects: resolve_effects(draw)?,
            })
        })
        .collect::<Result<Vec<_>, CastRejection>>()?;
    let (effects, chain) = if !party.is_empty() {
        (Vec::new(), Vec::new())
    } else if hits.is_empty() {
        (resolve_effects(draw)?, Vec::new())
    } else {
        let chain = hits
            .into_iter()
            .map(|hit| {
                let effects = resolve_effects(draw)?
                    .into_iter()
                    .map(|effect| scale(effect, hit.step, step_percent))
                    .collect();
                Ok(ChainHitResolution { hit, effects })
            })
            .collect::<Result<Vec<_>, CastRejection>>()?;
        (Vec::new(), chain)
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
        chain,
        party,
        target: match facts {
            Facts::Target(target) => Some(target.checked()),
            Facts::None | Facts::Chain(..) | Facts::Party(_) => None,
        },
        cooldowns: after,
    })
}

fn party_rejection(failure: PartyFailure) -> CastRejection {
    match failure {
        PartyFailure::NoMembersInRange => CastRejection::NoPartyMembers,
        PartyFailure::CostOverflow => CastRejection::PartyCostOverflow,
    }
}

/// A chain step scales the rolled damage or heal value (chain §3 step 8; D12 rounding).
fn scale(effect: ResolvedEffect, step: u32, step_percent: i32) -> ResolvedEffect {
    match effect {
        ResolvedEffect::Damage {
            damage_type,
            magnitude,
        } => ResolvedEffect::Damage {
            damage_type,
            magnitude: step_value(magnitude, step, step_percent),
        },
        ResolvedEffect::Heal { magnitude } => ResolvedEffect::Heal {
            magnitude: step_value(magnitude, step, step_percent),
        },
        other => other,
    }
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
        SpellEffect::Other { operation, effect } => ResolvedEffect::Unresolved {
            operation: operation.clone(),
            effect: effect.clone(),
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
