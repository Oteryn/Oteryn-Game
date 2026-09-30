//! Player cast composition (`OTERYN_PLAYER_SPELL_CAST_WIRE_AND_VITALS_CONTRACT_CANDIDATE_V1` §9
//! step 2): one cast intent through the core checks, the Ability plan and the owner commit, with
//! the §5 anchor payment in the same mutation.
//!
//! [`PlayerSpellState`] is the runtime actor's own vitals and cooldowns (§4, SPELL-D2). The
//! Channel owner keeps it beside the actor; it is never durable and never belongs to a
//! GameSession. [`cast`] is pure: it returns the next state, and the owner stores that value in one
//! write, so a rejected cast changes nothing and an accepted one pays mana, soul and cooldowns
//! exactly with its heal (SPELL-D3).

use std::collections::BTreeSet;

use oteryn_protocol_oteryn::actor_spell::{
    ActorVitals, MAX_SOUL, MAX_VITAL_POOL, SpellCastDisposition, SpellCastIntent, SpellTarget,
};
use oteryn_simulation_determinism::SemanticTimeMicros;
use serde_json::Value;

use super::authoring::spell_from_bundle;
use super::plan::{CastPlanError, effect_plan};
use super::{
    CastRejection, CasterState, Cooldowns, ResolvedEffect, SpellBook, SpellDefinition, Vocation,
    resolve_cast,
};
use crate::ability::{AbilityOccurrence, Effect};

macro_rules! starter {
    ($dir:literal) => {
        (
            include_str!(concat!(
                "../../../../tools/content-schema/spell-authoring/samples/starter-bundles/",
                $dir,
                "/spell.json"
            )),
            include_str!(concat!(
                "../../../../tools/content-schema/spell-authoring/samples/starter-bundles/",
                $dir,
                "/dependencies.json"
            )),
        )
    };
}

/// The V1 castable set of §6 among the candidate starter bundles: self heal and condition removal
/// (`exana pox`, `exura gran`, `exura`).
const V1_BUNDLES: [(&str, &str); 3] = [
    starter!("instant-cure_poison"),
    starter!("instant-intense_healing"),
    starter!("instant-light_healing"),
];

/// The V1 spell book, in the canonical SPELL-D1 order. Any bundle that does not load fails the
/// whole book closed; nothing is skipped, so an index never shifts silently.
pub(crate) fn v1_spell_book() -> Result<SpellBook, String> {
    book_from_bundles(&V1_BUNDLES)
}

/// The loader behind [`v1_spell_book`], parameterized only so a test can inject a bundle that
/// does not load (behaviour-neutral for production, which passes `V1_BUNDLES`).
fn book_from_bundles(bundles: &[(&str, &str)]) -> Result<SpellBook, String> {
    let spells = bundles
        .iter()
        .map(|(spell, dependencies)| {
            let spell: Value = serde_json::from_str(spell).map_err(|error| error.to_string())?;
            let dependencies: Value =
                serde_json::from_str(dependencies).map_err(|error| error.to_string())?;
            spell_from_bundle(&spell, &dependencies).map_err(|error| error.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    SpellBook::canonical(spells).map_err(|error| error.to_string())
}

/// The Character-owned inputs of a cast and of the vitals maxima (§4). The Character owner
/// supplies them; the client never does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CharacterCastFacts {
    pub(crate) vocation: Vocation,
    pub(crate) level: u32,
    pub(crate) magic_level: u32,
    pub(crate) max_health: u32,
    pub(crate) max_mana: u32,
    pub(crate) max_soul: u32,
}

/// The runtime actor's vitals and cooldowns (§4). `revision` is the `ACTOR_VITALS` revision: 1
/// when the actor's state is created, one more for every committed cast.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlayerSpellState {
    facts: CharacterCastFacts,
    health: u32,
    mana: u32,
    soul: u32,
    cooldowns: Cooldowns,
    revision: u64,
}

impl PlayerSpellState {
    /// A new runtime actor starts at its maxima (SPELL-D2). Maxima above the SPELL-D8 wire bounds
    /// are refused.
    pub(crate) fn new(facts: CharacterCastFacts) -> Option<Self> {
        (facts.max_health <= MAX_VITAL_POOL
            && facts.max_mana <= MAX_VITAL_POOL
            && facts.max_soul <= MAX_SOUL)
            .then(|| Self {
                facts,
                health: facts.max_health,
                mana: facts.max_mana,
                soul: facts.max_soul,
                cooldowns: Cooldowns::default(),
                revision: 1,
            })
    }

    /// Test only: stages a wounded actor (no damage owner exists yet).
    #[cfg(test)]
    pub(crate) const fn set_health_for_test(&mut self, health: u32) {
        self.health = health;
    }

    pub(crate) const fn revision(&self) -> u64 {
        self.revision
    }

    /// The own-actor `ActorVitalsV1` value. Harmony and Serene stay 0 and false: the SPELL-D8
    /// Harmony owner (H-1, H-2) is not delivered, and no admitted spell changes them.
    pub(crate) const fn vitals(&self) -> ActorVitals {
        ActorVitals {
            health: self.health,
            max_health: self.facts.max_health,
            mana: self.mana,
            max_mana: self.facts.max_mana,
            soul: self.soul,
            harmony: 0,
            serene: false,
        }
    }

    fn caster(&self) -> CasterState {
        CasterState {
            vocation: self.facts.vocation,
            level: self.facts.level,
            magic_level: self.facts.magic_level,
            // SPELL-D5: Premium activation is not authorized, so no account is premium.
            premium: false,
            mana: self.mana,
            max_mana: self.facts.max_mana,
            soul: self.soul,
            // §4: V1 serves no spell with `learning_required`.
            learned: BTreeSet::new(),
            // No V1 spell reads skills or equipment; they have no owner here yet.
            attack_skill: 0,
            attack_value: 0,
            attack_factor: 0.0,
            shielding_skill: 0,
            melee_weapon: false,
            shield_defense: None,
        }
    }
}

/// Everything a cast reads besides the book, the actor's state and the intent. The owner derives
/// it from the command; nothing in it comes from the client.
pub(crate) struct CastContext<'a> {
    /// The caster's exact actor atom (the Ability target of its own heal).
    pub(crate) caster: &'a str,
    pub(crate) owner_scope: &'a str,
    pub(crate) occurrence: AbilityOccurrence,
    pub(crate) now: SemanticTimeMicros,
    pub(crate) draw: &'a mut dyn FnMut(i64, i64) -> i64,
}

/// The §3 mapping from a core rejection to the wire disposition. A rejection the table does not
/// name (weapon, shield, chain or party facts) cannot occur for the V1 book and fails closed.
pub(crate) const fn disposition(rejection: &CastRejection) -> SpellCastDisposition {
    match rejection {
        CastRejection::GroupCooling { .. } | CastRejection::SpellCooling { .. } => {
            SpellCastDisposition::CoolingDown
        }
        CastRejection::LevelTooLow { .. } => SpellCastDisposition::LevelTooLow,
        CastRejection::MagicLevelTooLow { .. } => SpellCastDisposition::MagicLevelTooLow,
        CastRejection::NotEnoughMana { .. } => SpellCastDisposition::NotEnoughMana,
        CastRejection::NotEnoughSoul { .. } => SpellCastDisposition::NotEnoughSoul,
        CastRejection::NotLearned
        | CastRejection::VocationCannotUse
        | CastRejection::PremiumRequired => SpellCastDisposition::NotAvailable,
        CastRejection::TargetRequired => SpellCastDisposition::TargetRequired,
        CastRejection::TargetNotAllowed => SpellCastDisposition::TargetIllegal,
        _ => SpellCastDisposition::Rejected,
    }
}

/// One cast of `intent` by the actor whose state is `state`. On success the result is the next
/// state: the heal applied, mana and soul paid and the cooldowns started, with the revision one
/// higher. Any failure returns its disposition and leaves `state` as it was.
pub(crate) fn cast(
    book: &SpellBook,
    state: &PlayerSpellState,
    intent: &SpellCastIntent,
    context: CastContext<'_>,
) -> Result<PlayerSpellState, SpellCastDisposition> {
    let spell = book
        .indexed(intent.spell)
        .ok_or(SpellCastDisposition::Rejected)?;
    let has_target = match intent.target {
        SpellTarget::None => false,
        // No attack-target owner exists yet, so the server holds no attack target for the actor.
        // `aim_at_target` is honoured only with one (SPELL-D7), so it is ignored too.
        SpellTarget::AttackTarget => false,
        // SPELL-D7: only a `cast_at_position` spell takes a position, and the core admits none.
        SpellTarget::Position(_) => return Err(SpellCastDisposition::Rejected),
    };
    let resolution = resolve_cast(
        spell,
        &state.caster(),
        &state.cooldowns,
        context.now,
        has_target,
        context.draw,
    )
    .map_err(|rejection| disposition(&rejection))?;
    let plan = effect_plan(
        spell,
        &resolution,
        context.caster,
        None,
        context.occurrence,
        context.owner_scope,
    )
    .map_err(|error| match error {
        CastPlanError::MissingTarget => SpellCastDisposition::TargetRequired,
        _ => SpellCastDisposition::Rejected,
    })?;
    // The runtime actor carries no condition yet (conditions S8 have no owner), so removing one is
    // satisfied as Canary does for a caster without it. Any other side effect has no owner here.
    if plan
        .side_effects
        .iter()
        .any(|effect| !matches!(effect, ResolvedEffect::RemoveCondition { .. }))
    {
        return Err(SpellCastDisposition::Rejected);
    }
    let mut next = state.clone();
    if let Some(plan) = &plan.effects {
        for effect in plan.effects() {
            commit_effect(&mut next, spell, effect, context.caster)?;
        }
    }
    // §5 anchor: mana, soul and cooldowns are paid in this same value, only because it succeeded.
    next.mana = next
        .mana
        .checked_sub(resolution.mana_spent)
        .ok_or(SpellCastDisposition::Rejected)?;
    next.soul = next
        .soul
        .checked_sub(resolution.soul_spent)
        .ok_or(SpellCastDisposition::Rejected)?;
    next.cooldowns = resolution.cooldowns;
    next.revision = next
        .revision
        .checked_add(1)
        .ok_or(SpellCastDisposition::Rejected)?;
    Ok(next)
}

/// The owner commit of one planned effect onto the caster's own vitals. Only a heal of the caster
/// itself is admitted in V1; health never rises above its maximum.
fn commit_effect(
    next: &mut PlayerSpellState,
    spell: &SpellDefinition,
    effect: &Effect,
    caster: &str,
) -> Result<(), SpellCastDisposition> {
    match effect {
        Effect::Heal { target, magnitude } if target.as_str() == caster && *magnitude > 0 => {
            let healed = u64::from(next.health)
                .saturating_add(u64::try_from(*magnitude).unwrap_or(u64::MAX))
                .min(u64::from(next.facts.max_health));
            next.health = u32::try_from(healed).map_err(|_| SpellCastDisposition::Rejected)?;
            Ok(())
        }
        // A damage or foreign-target effect needs the Target Resolver, which V1 does not compose.
        _ if spell.aggressive => Err(SpellCastDisposition::TargetIllegal),
        _ => Err(SpellCastDisposition::Rejected),
    }
}

#[cfg(test)]
#[path = "cast_tests.rs"]
mod tests;
