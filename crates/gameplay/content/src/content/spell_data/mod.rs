//! Immutable spell definitions and source-qualified candidate compilation.
#![allow(
    dead_code,
    reason = "preserves the candidate spell core scope while production composition remains closed"
)]
use chain::ChainSpec;
pub use formula::Formula;
use party::PartyBuffSpec;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use target::AllowedTargets;
pub mod authoring;
pub mod chain;
pub mod executable_catalog;
pub mod formula;
pub mod native;
pub mod party;
pub mod target;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Vocation {
    None,
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
    pub fn from_key(key: &str) -> Option<Self> {
        Some(match key {
            "none" => Self::None,
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
pub enum Carrier {
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
pub enum ManaCost {
    Fixed(u32),
    /// Percent of the caster's maximum mana (`Spell::getManaCost`).
    PercentOfMaximum(u32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CooldownGroup {
    pub key: String,
    pub cooldown_micros: u64,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(
    clippy::large_enum_variant,
    reason = "transient owner result; boxing would add an allocation to the owner turn"
)]
pub enum SpellEffect {
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
    /// A complete typed non-health effect for its owning item/condition domain.
    ResolvedOther {
        profile: executable_catalog::EffectProfile,
        dependencies: executable_catalog::DependencyProfile,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Execution {
    Effects(Vec<SpellEffect>),
    AbilityVariants(Vec<AbilityVariant>),
    Conjure {
        reagent: Option<u32>,
        result: u32,
        count: u32,
    },
    /// `native_behavior` `party_buff` (part C.3).
    PartyBuff(PartyBuffSpec),
    /// Focus effects use the existing actor's MonkState and common cast anchor.
    /// The authoring reader qualifies the complete native profile before constructing this.
    ActorFocus {
        profile: serde_json::Value,
    },
    NativeProfile(native::CompiledNativeSpell),
}

#[derive(Debug, Clone, PartialEq)]
pub struct AbilityVariant {
    pub reference: executable_catalog::DefinitionRef,
    pub effects: Vec<SpellEffect>,
}

/// The monk Harmony role of a spell (S26, `harmony_role`; SPELL-D8 §8.2, native behaviours §A.2
/// steps 3 and 4). The Channel owner applies it at the cast's PRIMARY COMMIT.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HarmonyRole {
    /// +1 Harmony up to 5 after a successful cast.
    Builder,
    /// Damage bounds scaled by the Harmony multiplier; Harmony becomes 0 after a successful cast.
    Spender,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpellDefinition {
    /// Complete closed source rules. Fixture-only definitions may omit it;
    /// source-compiled definitions always retain it for common owner checks.
    pub authored: Option<std::sync::Arc<executable_catalog::OperationalProfile>>,
    pub key: String,
    pub name: String,
    pub carrier: Carrier,
    pub vocations: BTreeSet<Vocation>,
    pub level: u32,
    pub premium: bool,
    pub learning_required: bool,
    pub mana: ManaCost,
    pub soul: u32,
    pub cooldown_micros: u64,
    pub groups: Vec<CooldownGroup>,
    pub needs_target: bool,
    /// Cast at a target or, without one, in the looking direction (Canary `needCasterTargetOrDirection`).
    pub target_or_direction: bool,
    pub self_target: bool,
    /// Who the cast may be aimed at (D.3 and B.5, `targeting.allowed_targets`).
    pub allowed_targets: AllowedTargets,
    pub aggressive: bool,
    /// The caster must wield a melee weapon (Canary/Crystal `needWeapon`; D.4).
    pub needs_weapon: bool,
    /// The caster must wield a shield (D.4, `needs_shield`).
    pub needs_shield: bool,
    pub range_tiles: Option<u32>,
    pub base_power: Option<i64>,
    pub execution: Execution,
    /// The ability hits a chain of creatures (D12, S23).
    pub chain: Option<ChainSpec>,
    /// The monk Harmony role (S26); `None` for every other spell.
    pub harmony_role: Option<HarmonyRole>,
}

impl SpellDefinition {
    /// Whether a cast chains: a chain spell cast by direction, without a target, hits the tile in
    /// front and does not chain (Canary `lightning.lua`).
    pub fn chains(&self, has_target: bool) -> bool {
        self.chain.is_some() && (has_target || !self.target_or_direction)
    }

    pub fn takes_target(&self) -> bool {
        self.needs_target || self.target_or_direction
    }
}

/// Two admitted spells claim the same words, rune item or key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpellBookError {
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
pub struct SpellBook {
    pub spells: Vec<SpellDefinition>,
    by_words: BTreeMap<String, usize>,
    by_rune: BTreeMap<u32, usize>,
    pub inactive_aliases: BTreeSet<usize>,
}

/// A spoken message that names a spell.
#[derive(Debug, Clone, PartialEq)]
pub struct SpokenSpell<'a> {
    pub spell: &'a SpellDefinition,
    pub parameter: Option<String>,
}

impl SpellBook {
    pub fn source_len(&self) -> usize {
        self.spells.len()
    }
    pub fn new(spells: Vec<SpellDefinition>) -> Result<Self, SpellBookError> {
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

    /// Keep every source identity in the canonical index. Duplicate spoken
    /// words require an explicit identity choice; that choice affects only the
    /// word lookup, never deleting the alternative definitions.
    pub fn new_with_alias_selections(
        mut spells: Vec<SpellDefinition>,
        selections: &BTreeMap<String, String>,
    ) -> Result<Self, SpellBookError> {
        spells.sort_by(|left, right| left.key.as_bytes().cmp(right.key.as_bytes()));
        let mut book = Self::default();
        let mut keys = BTreeSet::new();
        let mut groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (index, spell) in spells.iter().enumerate() {
            if !keys.insert(spell.key.clone()) {
                return Err(SpellBookError::Key(spell.key.clone()));
            }
            match &spell.carrier {
                Carrier::Instant { words, .. } => groups
                    .entry(words.to_ascii_lowercase())
                    .or_default()
                    .push(index),
                Carrier::Rune { item, .. } => {
                    if book.by_rune.insert(*item, index).is_some() {
                        return Err(SpellBookError::Rune(*item));
                    }
                }
            }
        }
        let mut used = BTreeSet::new();
        for (words, indices) in groups {
            let index = if indices.len() == 1 {
                if selections.contains_key(&words) {
                    return Err(SpellBookError::Words(words));
                }
                indices[0]
            } else {
                let selected = selections
                    .get(&words)
                    .ok_or_else(|| SpellBookError::Words(words.clone()))?;
                let index = indices
                    .iter()
                    .copied()
                    .find(|&index| &spells[index].key == selected)
                    .ok_or_else(|| SpellBookError::Words(words.clone()))?;
                used.insert(words.clone());
                book.inactive_aliases.extend(
                    indices
                        .iter()
                        .copied()
                        .filter(|candidate| *candidate != index),
                );
                index
            };
            book.by_words.insert(words, index);
        }
        if used.len() != selections.len() {
            return Err(SpellBookError::Words(
                selections
                    .keys()
                    .find(|key| !used.contains(*key))
                    .cloned()
                    .unwrap_or_default(),
            ));
        }
        book.spells = spells;
        Ok(book)
    }

    /// The instant spell a spoken message casts, if any (part B.3 P1 and P2, Canary
    /// `getInstantSpell` and `playerSaySpell`). Only ASCII spaces at the ends are trimmed;
    /// interior whitespace and parameter case are preserved. The longest case-insensitive prefix is chosen; a
    /// spell without a parameter must match exactly, one with a parameter needs a space and at
    /// least one more character after its words. A parameter containing a quote is the text
    /// up to the next quote (an unclosed quote runs to the end; text after the closing quote makes
    /// the message chat); an unquoted parameter must be a single word
    /// (two words make it chat). It keeps the
    /// spoken case; an empty parameter is `None`. `None` means the message is chat.
    pub fn spoken(&self, message: &str) -> Option<SpokenSpell<'_>> {
        let message = message.trim_matches(' ');
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
        let parameter = match parameter.find('"') {
            Some(open) => {
                let quoted = &parameter[open + 1..];
                match quoted.find('"') {
                    None => quoted,
                    Some(close) if close + 1 == quoted.len() => &quoted[..close],
                    Some(_) => return None,
                }
            }
            None => {
                let parameter = parameter.trim_matches(' ');
                if parameter.contains(' ') {
                    return None;
                }
                parameter
            }
        };
        Some(SpokenSpell {
            spell,
            parameter: (!parameter.is_empty()).then(|| parameter.to_owned()),
        })
    }

    pub fn rune(&self, item: u32) -> Option<&SpellDefinition> {
        self.by_rune.get(&item).map(|&index| &self.spells[index])
    }

    /// A book in the canonical SPELL-D1 order: `ProductionKey` ascending by UTF-8 bytes.
    pub fn canonical(mut spells: Vec<SpellDefinition>) -> Result<Self, SpellBookError> {
        spells.sort_by(|left, right| left.key.as_bytes().cmp(right.key.as_bytes()));
        Self::new(spells)
    }

    /// The spell at a 1-based index of the book's order (SPELL-D1 for a [`Self::canonical`] book).
    pub fn indexed(&self, index: std::num::NonZeroU32) -> Option<&SpellDefinition> {
        usize::try_from(index.get() - 1)
            .ok()
            .filter(|index| !self.inactive_aliases.contains(index))
            .and_then(|index| self.spells.get(index))
    }
    /// Inspect the complete source index without granting a cast lookup. An
    /// unselected alias remains represented and cannot be cast via its index.
    pub fn source_indexed(&self, index: std::num::NonZeroU32) -> Option<(&SpellDefinition, bool)> {
        let index = usize::try_from(index.get() - 1).ok()?;
        self.spells
            .get(index)
            .map(|spell| (spell, !self.inactive_aliases.contains(&index)))
    }
}
