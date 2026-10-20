//! Player spell core (plan phase P3a, `docs/architecture/OTERYN_SPELL_AUTHORING_SCHEMA_V1.md` §9).
//!
//! Server-side rules of a player cast: spoken words to a spell, the cast checks in the engine order
//! (cooldowns, level, magic level, mana, soul, learning/vocation, premium), the mana and soul debit,
//! own and group cooldowns, and the damage/heal magnitude from the spell formula. The result is data
//! for the Ability pipeline; applying it to actors is the owner commit's job.
//!
//! [`cast`] composes the core into the live cast command (P3b §9 step 2); it adds no persisted
//! state. Spell definitions come from the candidate authoring bundles through [`authoring`]; the
//! admitted route is the WorldProject/v2 lowering of P3b.

pub(crate) mod actor_conditions;
pub(crate) mod actor_execution;
pub(crate) mod authoring;
pub(crate) mod cast;
pub(crate) use crate::combat::chain;
#[cfg(test)]
mod chain_tests;
pub(crate) mod combat_batch;
pub(crate) mod combat_execution;
pub(crate) mod companion_execution;
pub(crate) mod companion_lifecycle;
pub(crate) mod delayed_execution;
pub(crate) mod executable_catalog;
pub(crate) mod field_history;
pub(crate) mod find_person_execution;
pub(crate) mod food_regeneration;
pub(crate) use crate::combat::formula;
pub(crate) mod harmony;
#[cfg(test)]
mod harmony_tests;
pub(crate) mod locate;
pub(crate) mod magnitude_owner;
pub(crate) mod mana_training;
pub(crate) mod native;
pub(crate) mod native_actor_states;
pub(crate) mod native_cast_commit;
pub(crate) mod native_combat;
pub(crate) mod native_companions;
pub(crate) mod native_delayed;
pub(crate) mod native_house_movement;
pub(crate) mod native_items;
pub(crate) mod ordinary_timer;
pub(crate) mod owned_cast_facts;
#[cfg(test)]
mod part_b_tests;
#[cfg(test)]
mod part_d_tests;
pub(crate) mod party;
#[cfg(test)]
mod party_tests;
pub(crate) mod plan;
pub(crate) mod spell_presentations;
pub(crate) mod stance_execution;
pub(crate) mod target;
#[cfg(test)]
mod tests;
pub(crate) mod world_execution;
pub(crate) mod world_items_execution;

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use oteryn_simulation_determinism::SemanticTimeMicros;

use chain::{ChainCreature, ChainHit, ChainStart, ChainWorld, pick_chain, step_value};
pub(crate) use formula::{FormulaError, FormulaInputs};
use harmony::HarmonyMultiplier;
use party::{PartyFailure, PartyWorld};
use target::{AllowedTargets, CastTarget, CheckedTarget};

pub(crate) use crate::content::spell_data::*;

/// The caster's Premium status for one cast (PREMIUM-ACTIVATION-0 §1.3): the gate's
/// `PremiumStatus` without its pre-delivery witness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CasterPremium {
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
    pub(crate) const fn from_current(current: bool) -> Self {
        if current {
            Self::Current
        } else {
            Self::NotCurrent
        }
    }
    pub(crate) fn is_current(self) -> bool {
        self == Self::Current
    }
    /// A Premium spell needs `Current` (§1.3). WHEEL0-PS-1 lets a `wheel_unlock` spell skip the
    /// check under `NotActivated`; until W-1 applies it inside the gate, it fails closed too.
    pub(crate) fn admits(self, spell: &SpellDefinition) -> bool {
        !spell.premium || self == Self::Current
    }
}

/// What the cast checks read from the caster.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CasterState {
    pub(crate) vocation: Vocation,
    pub(crate) level: u32,
    pub(crate) magic_level: u32,
    pub(crate) premium: CasterPremium,
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
    /// The Harmony multiplier a spender's damage bounds take (§A.2 step 3b), from the monk's
    /// current charges and Serene; [`HarmonyMultiplier::ONE`] for anyone else.
    pub(crate) harmony_multiplier: HarmonyMultiplier,
}

/// Ready times of a caster's spell and group cooldowns.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Cooldowns {
    spells: BTreeMap<String, SemanticTimeMicros>,
    groups: BTreeMap<String, SemanticTimeMicros>,
}

impl Cooldowns {
    /// Exact preflight state in semantic microseconds. Family prefixes prevent
    /// a spell key and group key with the same text from aliasing each other.
    pub(crate) fn canonical_deadlines(&self) -> Vec<(String, u64)> {
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
    pub(crate) fn rearm(
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
    pub(crate) fn spell_ready_at(&self, key: &str) -> Option<SemanticTimeMicros> {
        self.spells.get(key).copied()
    }

    pub(crate) fn group_ready_at(&self, group: &str) -> Option<SemanticTimeMicros> {
        self.groups.get(group).copied()
    }

    /// Focus resets the resolved cooldown successor before the common anchor
    /// rearms the casting spell and its groups. Individual keys absent from
    /// the admitted book are preserved. Group lookups follow the qualified
    /// recipe: an unresolved group clears; a resolved entry uses spender status.
    pub(crate) fn reset_for_focus(
        &mut self,
        book: &SpellBook,
        plan: &native_actor_states::FocusPlan,
    ) {
        self.spells.retain(|key, _| {
            let Some(spell) = book.spells.iter().find(|spell| &spell.key == key) else {
                return true;
            };
            !plan
                .individual_reset
                .clears(spell.harmony_role == Some(HarmonyRole::Spender))
        });
        self.groups.retain(
            |key, _| match book.spells.iter().find(|spell| &spell.key == key) {
                Some(spell) => !plan
                    .group_with_spell_reset
                    .clears(spell.harmony_role == Some(HarmonyRole::Spender)),
                None => !plan.group_without_spell_reset.clears(false),
            },
        );
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

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(
    clippy::large_enum_variant,
    reason = "transient owner result; boxing would add an allocation to the owner turn"
)]
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
    ResolvedOther {
        profile: executable_catalog::EffectProfile,
        dependencies: executable_catalog::DependencyProfile,
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
    pub(crate) ability_variant: Option<executable_catalog::DefinitionRef>,
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
    Owned(&'a OperationalCastFacts, bool),
    CarriedItem(&'a OperationalCastFacts),
}

/// Independent current snapshots supplied by the owning runtime. They cannot
/// be reconstructed from the immutable authored header or from client fields.
#[derive(Debug, Clone)]
pub(crate) struct OperationalCastFacts {
    pub(crate) caster_position: chain::TilePosition,
    pub(crate) target_position: Option<chain::TilePosition>,
    pub(crate) target: Option<CastTarget>,
    pub(crate) line_of_sight_clear: Option<bool>,
    pub(crate) direction_available: bool,
    pub(crate) wheel_unlocked: Option<bool>,
    pub(crate) in_protection_zone: bool,
    pub(crate) target_tile_solid: Option<bool>,
    pub(crate) target_tile_creature: Option<bool>,
}

fn check_operational_rules(
    spell: &SpellDefinition,
    facts: &OperationalCastFacts,
    carried_item: bool,
) -> Result<(), CastRejection> {
    let Some(profile) = &spell.authored else {
        return Ok(());
    };
    let header = &profile.header;
    if header.requirements.wheel_unlock == Some(true) && facts.wheel_unlocked != Some(true) {
        return Err(CastRejection::WheelUnlockRequired);
    }
    if header.targeting.aggressive && facts.in_protection_zone {
        return Err(CastRejection::ProtectionZoneRejected);
    }
    if header.targeting.needs_direction && !facts.direction_available {
        return Err(CastRejection::SpatialRuleRejected);
    }
    if header.targeting.target_or_direction == Some(true)
        && facts.target.is_none()
        && !facts.direction_available
    {
        return Err(CastRejection::SpatialRuleRejected);
    }
    if !carried_item
        && header.targeting.cast_at_position == Some(true)
        && facts.target_position.is_none()
    {
        return Err(CastRejection::OperationalFactsRequired);
    }
    if let Some(target) = facts.target_position {
        if header.targeting.check_floor && target.floor != facts.caster_position.floor {
            return Err(CastRejection::SpatialRuleRejected);
        }
        if header.targeting.block_walls && facts.line_of_sight_clear != Some(true) {
            return Err(CastRejection::SpatialRuleRejected);
        }
        let distance = target
            .x
            .abs_diff(facts.caster_position.x)
            .max(target.y.abs_diff(facts.caster_position.y));
        if header
            .targeting
            .range_tiles
            .is_some_and(|range| distance > range)
        {
            return Err(CastRejection::SpatialRuleRejected);
        }
        if let Some(rune) = &header.rune {
            if !rune.allow_far_use && distance > 1 {
                return Err(CastRejection::SpatialRuleRejected);
            }
            // Source Rune::playerRuneSpellCheck permits a known solid target
            // cell when its independently resolved creature is present. Unknown
            // solidity never receives this exception; blockingCreature still
            // rejects occupied cells below when that source flag is set.
            if rune.blocking.solid
                && !(facts.target_tile_solid == Some(false)
                    || (facts.target_tile_solid == Some(true)
                        && facts.target_tile_creature == Some(true)))
            {
                return Err(CastRejection::SpatialRuleRejected);
            }
            if rune.blocking.creature && facts.target_tile_creature != Some(false) {
                return Err(CastRejection::SpatialRuleRejected);
            }
        }
    } else if header.carrier == "rune" && !carried_item {
        return Err(CastRejection::OperationalFactsRequired);
    }
    if let Some(target) = &facts.target {
        if !spell.allowed_targets.allows(target) {
            return Err(CastRejection::TargetNotAllowed);
        }
    } else if spell.needs_target || spell.allowed_targets != AllowedTargets::Any {
        return Err(CastRejection::TargetFactsRequired);
    }
    Ok(())
}

pub(crate) fn resolve_cast_with_operational_facts(
    spell: &SpellDefinition,
    caster: &CasterState,
    cooldowns: &Cooldowns,
    now: SemanticTimeMicros,
    facts: &OperationalCastFacts,
    draw: &mut dyn FnMut(i64, i64) -> i64,
) -> Result<CastResolution, CastRejection> {
    resolve(
        spell,
        caster,
        cooldowns,
        now,
        facts.target.is_some(),
        Facts::Owned(facts, false),
        draw,
    )
}

#[derive(Debug)]
pub(crate) struct NativeCastResolution {
    pub(crate) common: CastResolution,
    pub(crate) plan: native::Plan,
}

pub(crate) fn resolve_native_cast(
    spell: &SpellDefinition,
    caster: &CasterState,
    cooldowns: &Cooldowns,
    now: SemanticTimeMicros,
    operational: &OperationalCastFacts,
    facts: native::Facts<'_>,
    draw: &mut dyn FnMut(i64, i64) -> i64,
) -> Result<NativeCastResolution, CastRejection> {
    let Execution::NativeProfile(profile) = &spell.execution else {
        return Err(CastRejection::NativeFactsRequired);
    };
    let common = resolve(
        spell,
        caster,
        cooldowns,
        now,
        operational.target.is_some(),
        Facts::Owned(operational, true),
        draw,
    )?;
    let plan = profile
        .plan(facts, draw)
        .map_err(|error| CastRejection::NativePlan(error.0))?;
    Ok(NativeCastResolution { common, plan })
}

/// Engine cost/cooldown admission for source callbacks. Their world/ACL/message
/// plan belongs to the separately qualified callback owner, never a numeric fixture.
pub(crate) fn resolve_native_callback_common(
    spell: &SpellDefinition,
    caster: &CasterState,
    cooldowns: &Cooldowns,
    now: SemanticTimeMicros,
    operational: &OperationalCastFacts,
) -> Result<CastResolution, CastRejection> {
    resolve_native_callback_common_inner(spell, caster, cooldowns, now, operational, false)
}

/// Only the source Chameleon callback may address a current carried Item.
/// The compositor must verify the reader's sealed physical transaction before
/// entering this admission. No invented world position is used for inventory.
pub(crate) fn resolve_native_callback_common_for_carried_item(
    spell: &SpellDefinition,
    caster: &CasterState,
    cooldowns: &Cooldowns,
    now: SemanticTimeMicros,
    operational: &OperationalCastFacts,
    carried: &crate::durability::spell_item_transaction::CarriedSpellTarget,
) -> Result<CastResolution, CastRejection> {
    let Execution::NativeProfile(profile) = &spell.execution else {
        return Err(CastRejection::NativeFactsRequired);
    };
    let native = &profile.spell()["execution"]["native_behavior"];
    if !matches!(spell.carrier, Carrier::Rune { .. })
        || native["key"] != "tile_item_operation"
        || native["parameters"]["operation"] != "mimic_item"
        || operational.target_position.is_some()
        || operational.target.is_some()
        || carried.definition().family != "Item"
        || carried.state_revision() == 0
    {
        return Err(CastRejection::NativeFactsRequired);
    }
    resolve_native_callback_common_inner(spell, caster, cooldowns, now, operational, true)
}

fn resolve_native_callback_common_inner(
    spell: &SpellDefinition,
    caster: &CasterState,
    cooldowns: &Cooldowns,
    now: SemanticTimeMicros,
    operational: &OperationalCastFacts,
    carried_item: bool,
) -> Result<CastResolution, CastRejection> {
    let Execution::NativeProfile(profile) = &spell.execution else {
        return Err(CastRejection::NativeFactsRequired);
    };
    let canonical_barrier = profile.spell()["execution"]
        .get("native_behavior")
        .is_none()
        && profile.dependencies()["effects"]
            .as_array()
            .is_some_and(|effects| {
                effects.len() == 1
                    && effects[0]["operation"] == "create_item"
                    && effects[0]["duration_selection"] == "uniform_integer_seconds"
                    && effects[0].get("pvp_safe_item").is_some()
            });
    if !matches!(
        profile.spell()["execution"]["native_behavior"]["key"].as_str(),
        Some(
            "locate_message"
                | "house_access"
                | "vertical_move"
                | "tile_item_operation"
                | "acquire_summon"
                | "companion_acquisition"
        )
    ) && !canonical_barrier
    {
        return Err(CastRejection::NativeFactsRequired);
    }
    let mut requested_draw = false;
    let mut no_draw = |_: i64, _: i64| {
        requested_draw = true;
        0
    };
    let common = resolve(
        spell,
        caster,
        cooldowns,
        now,
        operational.target.is_some(),
        if carried_item {
            Facts::CarriedItem(operational)
        } else {
            Facts::Owned(operational, true)
        },
        &mut no_draw,
    )?;
    if requested_draw || !common.effects.is_empty() {
        return Err(CastRejection::NativeFactsRequired);
    }
    Ok(common)
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
    if !caster.premium.admits(spell) {
        return Err(CastRejection::PremiumRequired);
    }
    if let Facts::Owned(operational, _) | Facts::CarriedItem(operational) = facts {
        check_operational_rules(spell, operational, matches!(facts, Facts::CarriedItem(_)))?;
    } else if let Some(profile) = &spell.authored {
        if profile.dependencies.abilities.iter().any(|ability| {
            ability.target_selection.is_some() || ability.zero_damage_health_path == Some(true)
        }) || profile.dependencies.effects.iter().any(|effect| {
            effect.presentation.as_ref().is_some_and(|presentation| {
                presentation.caster_effect_asset_binding.is_some()
                    || presentation.caster_effect_timing.is_some()
            })
        }) {
            return Err(CastRejection::OperationalFactsRequired);
        }
        if profile.header.requirements.wheel_unlock == Some(true) {
            return Err(CastRejection::WheelUnlockRequired);
        }
        if profile.header.targeting.cast_at_position == Some(true) {
            return Err(CastRejection::OperationalFactsRequired);
        }
    }
    if matches!(spell.execution, Execution::NativeProfile(_))
        && !matches!(facts, Facts::Owned(_, true) | Facts::CarriedItem(_))
    {
        return Err(CastRejection::NativeFactsRequired);
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
    // §A.2 step 3b: only a spender's damage takes the multiplier.
    let multiplier = match spell.harmony_role {
        Some(HarmonyRole::Spender) => caster.harmony_multiplier,
        Some(HarmonyRole::Builder) | None => HarmonyMultiplier::ONE,
    };
    let selected_variant = if let Execution::AbilityVariants(variants) = &spell.execution {
        let maximum = variants
            .len()
            .checked_sub(1)
            .and_then(|n| i64::try_from(n).ok())
            .ok_or(CastRejection::InvalidVariantDraw)?;
        let index =
            usize::try_from(draw(0, maximum)).map_err(|_| CastRejection::InvalidVariantDraw)?;
        Some(
            variants
                .get(index)
                .ok_or(CastRejection::InvalidVariantDraw)?,
        )
    } else {
        None
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
            .map(|effect| resolve_effect(effect, &inputs, multiplier, draw))
            .collect::<Result<Vec<_>, _>>()
            .map_err(CastRejection::Formula),
        Execution::AbilityVariants(_) => selected_variant
            .ok_or(CastRejection::InvalidVariantDraw)?
            .effects
            .iter()
            .map(|effect| resolve_effect(effect, &inputs, multiplier, draw))
            .collect::<Result<Vec<_>, _>>()
            .map_err(CastRejection::Formula),
        Execution::PartyBuff(buff) => buff
            .effects
            .iter()
            .map(|effect| resolve_effect(effect, &inputs, multiplier, draw))
            .collect::<Result<Vec<_>, _>>()
            .map_err(CastRejection::Formula),
        Execution::ActorFocus { .. } | Execution::NativeProfile(_) => Ok(Vec::new()),
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
        ability_variant: selected_variant.map(|variant| variant.reference.clone()),
        mana_spent: mana,
        soul_spent: spell.soul,
        effects,
        chain,
        party,
        target: match facts {
            Facts::Target(target) => Some(target.checked()),
            Facts::Owned(operational, _) | Facts::CarriedItem(operational) => {
                operational.target.as_ref().map(CastTarget::checked)
            }
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
    multiplier: HarmonyMultiplier,
    draw: &mut dyn FnMut(i64, i64) -> i64,
) -> Result<ResolvedEffect, FormulaError> {
    Ok(match effect {
        SpellEffect::Damage {
            damage_type,
            formula,
        } => {
            // §A.2 step 3b: each bound is multiplied and truncated toward zero before the roll.
            let (low, high) = formula.bounds(inputs)?;
            let (low, high) = (multiplier.apply(low), multiplier.apply(high));
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
        SpellEffect::ResolvedOther {
            profile,
            dependencies,
        } => ResolvedEffect::ResolvedOther {
            profile: profile.clone(),
            dependencies: dependencies.clone(),
        },
    })
}

pub(crate) use crate::durability::uniform_draw::uniform_draw;

pub(crate) mod house_execution;
pub(crate) mod source_map_initialization;

pub(crate) mod periodic_execution;

mod source_player_conditions;
