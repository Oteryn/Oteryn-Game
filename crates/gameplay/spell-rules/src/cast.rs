//! Pure caster snapshots, cost rules and cooldown plans for player spells.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use oteryn_gameplay_content::content::spell_data::formula::FormulaError;
use oteryn_gameplay_content::content::spell_data::{
    HarmonyRole, ManaCost, SpellBook, SpellDefinition, Vocation,
};
use oteryn_simulation_determinism::SemanticTimeMicros;

use super::harmony::HarmonyMultiplier;
use super::native_actor_states;

/// The caster's Premium status for one cast (PREMIUM-ACTIVATION-0 §1.3): the gate's
/// `PremiumStatus` without its pre-delivery witness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CasterPremium {
    Current,
    NotCurrent,
    #[allow(
        dead_code,
        reason = "WHEEL0-PS-1: the cast reads the gate's NotActivated with the W-1 Wheel exception"
    )]
    NotActivated,
}
impl CasterPremium {
    /// `Current` only for a current Premium projection.
    pub const fn from_current(current: bool) -> Self {
        if current {
            Self::Current
        } else {
            Self::NotCurrent
        }
    }
    pub fn is_current(self) -> bool {
        self == Self::Current
    }
    /// A Premium spell needs `Current` (§1.3). WHEEL0-PS-1 lets a `wheel_unlock` spell skip the
    /// check under `NotActivated`; until W-1 applies it inside the gate, it fails closed too.
    pub fn admits(self, spell: &SpellDefinition) -> bool {
        !spell.premium || self == Self::Current
    }
}

/// What the cast checks read from the caster.
#[derive(Debug, Clone, PartialEq)]
pub struct CasterState {
    pub vocation: Vocation,
    pub level: u32,
    pub magic_level: u32,
    pub premium: CasterPremium,
    pub mana: u32,
    pub max_mana: u32,
    pub soul: u32,
    pub learned: BTreeSet<String>,
    pub attack_skill: u32,
    pub attack_value: u32,
    pub attack_factor: f64,
    pub shielding_skill: u32,
    /// The caster wields a sword, club, axe or fist weapon (Canary/Crystal `needWeapon`), as the
    /// equipment owner reports it.
    pub melee_weapon: bool,
    /// Defense of the first shield in the caster's left or right hand; `None` without a shield (D.4).
    pub shield_defense: Option<u32>,
    /// The Harmony multiplier a spender's damage bounds take (§A.2 step 3b), from the monk's
    /// current charges and Serene; [`HarmonyMultiplier::ONE`] for anyone else.
    pub harmony_multiplier: HarmonyMultiplier,
}

/// Runtime-only movement pacification, kept in the existing cooldown owner. This is not a
/// spell group and neither Focus nor client spell-group presentation may reset/expose it.
pub const STAIRHOP_COOLDOWN_KEY: &str = "runtime:stairhop";
pub const STAIRHOP_MICROS: u64 = 2_000_000;

/// Ready times of a caster's spell and group cooldowns.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Cooldowns {
    pub spells: BTreeMap<String, SemanticTimeMicros>,
    pub groups: BTreeMap<String, SemanticTimeMicros>,
}

impl Cooldowns {
    /// Exact preflight state in semantic microseconds. Family prefixes prevent
    /// a spell key and group key with the same text from aliasing each other.
    pub fn canonical_deadlines(&self) -> Vec<(String, u64)> {
        let mut deadlines: Vec<_> = self
            .spells
            .iter()
            .map(|(key, time)| (format!("spell:{key}"), time.get()))
            .chain(
                self.groups
                    .iter()
                    .map(|(key, time)| (format!("group:{key}"), time.get())),
            )
            .collect();
        deadlines.sort_by(|left, right| left.0.as_bytes().cmp(right.0.as_bytes()));
        deadlines
    }
    /// Rearm only this successful cast's individual and group cooldowns. Build all deadlines
    /// before changing any slot so overflow never leaves a partial successor.
    pub fn rearm(
        &mut self,
        spell: &SpellDefinition,
        now: SemanticTimeMicros,
    ) -> Result<(), CastRejection> {
        let ready = now
            .checked_add(spell.cooldown_micros)
            .map_err(|_| CastRejection::TimeOverflow)?;
        let groups = spell
            .groups
            .iter()
            .map(|group| {
                now.checked_add(group.cooldown_micros)
                    .map(|ready| (group.key.clone(), ready))
                    .map_err(|_| CastRejection::TimeOverflow)
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.spells.insert(spell.key.clone(), ready);
        self.groups.extend(groups);
        Ok(())
    }
    pub fn spell_ready_at(&self, key: &str) -> Option<SemanticTimeMicros> {
        self.spells.get(key).copied()
    }

    pub fn group_ready_at(&self, group: &str) -> Option<SemanticTimeMicros> {
        self.groups.get(group).copied()
    }

    /// Focus resets the resolved cooldown successor before the common anchor
    /// rearms the casting spell and its groups. Individual keys absent from
    /// the admitted book are preserved. Group lookups follow the qualified
    /// recipe: an unresolved group clears; a resolved entry uses spender status.
    pub fn reset_for_focus(&mut self, book: &SpellBook, plan: &native_actor_states::FocusPlan) {
        self.spells.retain(|key, _| {
            let Some(spell) = book.spells.iter().find(|spell| &spell.key == key) else {
                return true;
            };
            !plan
                .individual_reset
                .clears(spell.harmony_role == Some(HarmonyRole::Spender))
        });
        self.groups.retain(|key, _| {
            key == STAIRHOP_COOLDOWN_KEY
                || match book.spells.iter().find(|spell| &spell.key == key) {
                    Some(spell) => !plan
                        .group_with_spell_reset
                        .clears(spell.harmony_role == Some(HarmonyRole::Spender)),
                    None => !plan.group_without_spell_reset.clears(false),
                }
        });
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CastRejection {
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
    /// (`resolve_targeted_cast`).
    TargetFactsRequired,
    /// A chain cast found no first creature (chain §3 step 2); nothing is spent.
    NoChainTarget,
    /// A chain cast was resolved without the world facts it needs (`resolve_chain_cast`).
    ChainWorldRequired,
    /// A party buff found no party, or too few members in its area (C.3 steps 1 and 3); nothing
    /// is spent.
    NoPartyMembers,
    /// A party buff was resolved without the party facts it needs (`resolve_party_cast`).
    PartyWorldRequired,
    /// A party buff's scaled mana does not fit the exact integer computation.
    PartyCostOverflow,
    OperationalFactsRequired,
    WheelUnlockRequired,
    SpatialRuleRejected,
    ProtectionZoneRejected,
    NativeFactsRequired,
    NativePlan(String),
    InvalidVariantDraw,
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
            Self::OperationalFactsRequired => {
                formatter.write_str("current operational cast facts are required")
            }
            Self::WheelUnlockRequired => {
                formatter.write_str("the Wheel owner has not unlocked this spell")
            }
            Self::SpatialRuleRejected => {
                formatter.write_str("the cast violates its authored spatial rules")
            }
            Self::ProtectionZoneRejected => {
                formatter.write_str("an aggressive spell cannot be cast in a protection zone")
            }
            Self::NativeFactsRequired => formatter.write_str("native owner facts are required"),
            Self::NativePlan(reason) => write!(formatter, "native plan: {reason}"),
            Self::InvalidVariantDraw => {
                formatter.write_str("variant draw is outside its authored bounds")
            }
            Self::TimeOverflow => formatter.write_str("cooldown time overflow"),
            Self::Formula(error) => write!(formatter, "formula: {error}"),
        }
    }
}

impl Error for CastRejection {}

/// Mana a cast costs this caster (`Spell::getManaCost`: a fixed amount or a share of maximum mana).
pub fn mana_cost(spell: &SpellDefinition, caster: &CasterState) -> u32 {
    match spell.mana {
        ManaCost::Fixed(mana) => mana,
        ManaCost::PercentOfMaximum(percent) => {
            u32::try_from(u64::from(caster.max_mana) * u64::from(percent) / 100).unwrap_or(u32::MAX)
        }
    }
}
