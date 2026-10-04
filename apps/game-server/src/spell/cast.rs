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
use super::chain::{ChainCreature, TilePosition};
use super::harmony::{HarmonyMultiplier, MonkState, MonkStateError};
use super::party::SoloParty;
use super::plan::{CastPlanError, effect_plan};
use super::{
    CastRejection, CasterState, Cooldowns, HarmonyRole, ResolvedEffect, SpellBook, SpellDefinition,
    Vocation, resolve_cast,
};
use crate::ability::creature_bite::{FlooredDamage, floor_creature_damage};
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
/// when the actor's state is created, one more for every committed cast and for every periodic
/// evaluation that changes Serene.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlayerSpellState {
    facts: CharacterCastFacts,
    health: u32,
    mana: u32,
    soul: u32,
    cooldowns: Cooldowns,
    revision: u64,
    /// SPELL-D8 §8.2: a monk's live Harmony and Serene; `None` for every other vocation.
    monk: Option<MonkState>,
}

/// The Serene world of a runtime actor. No party service exists, so every monk is solo (§4
/// interim rule) and always Serene by the rule; a solo evaluation never reads the creature.
fn solo() -> SoloParty {
    SoloParty(ChainCreature {
        id: 0,
        actor: String::new(),
        position: TilePosition {
            x: 0,
            y: 0,
            floor: 0,
        },
    })
}

impl PlayerSpellState {
    /// A new runtime actor starts at its maxima (SPELL-D2), with the durable Harmony and remaining
    /// forced Serene time the Character owner loaded (§8.2; both 0 for any other vocation).
    /// Maxima above the SPELL-D8 wire bounds, and corrupt Harmony values, are refused.
    pub(crate) fn new(
        facts: CharacterCastFacts,
        harmony: u8,
        serene_forced_micros: u64,
    ) -> Option<Self> {
        let monk = MonkState::load(facts.vocation, harmony, serene_forced_micros).ok()?;
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
                monk,
            })
    }

    /// The Serene initialization evaluation (§8.2), in the owner step that makes the actor
    /// playable: a fresh admission, a same-GameSession reconnect or FND-04B §21 recovery. No
    /// command of a monk is accepted before it.
    pub(crate) fn make_playable(&mut self, now: SemanticTimeMicros) -> Result<(), MonkStateError> {
        match &mut self.monk {
            Some(monk) => monk.initialize(now, &solo()),
            None => Ok(()),
        }
    }

    /// The session left the actor: Harmony and the forced time are kept, and commands wait for
    /// the next [`Self::make_playable`].
    pub(crate) fn detach(&mut self) {
        if let Some(monk) = &mut self.monk {
            monk.detach();
        }
    }

    /// The periodic 1000 ms Serene evaluation (§8.2). `true` when Serene changed; the change then
    /// has its own `ACTOR_VITALS` revision. On an error nothing changes.
    pub(crate) fn tick(&mut self, now: SemanticTimeMicros) -> Result<bool, MonkStateError> {
        let Some(monk) = &self.monk else {
            return Ok(false);
        };
        let mut monk = monk.clone();
        if !monk.tick(now, &solo())? {
            self.monk = Some(monk);
            return Ok(false);
        }
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or(MonkStateError::TimeOverflow)?;
        self.monk = Some(monk);
        Ok(true)
    }

    /// The values the actor-end save writes (§8.2): Harmony and the forced Serene time left at
    /// `now`. `None` for an actor that is not a monk.
    pub(crate) fn monk_save_values(&self, now: SemanticTimeMicros) -> Option<(u8, u64)> {
        self.monk
            .as_ref()
            .map(|monk| (monk.harmony(), monk.serene_forced_remaining(now)))
    }

    /// GAME-AI-01 slice §4.7 (D54): the state after one creature hit of `magnitude`. Health never
    /// drops below 1 (no player death in V1). `None` when the hit removes nothing (the actor is
    /// already at 1) or the revision is exhausted; the state is then unchanged.
    pub(crate) fn after_creature_damage(&self, magnitude: u32) -> Option<(Self, FlooredDamage)> {
        let damage = floor_creature_damage(self.health, magnitude);
        if damage.applied == 0 {
            return None;
        }
        let mut next = self.clone();
        next.health = damage.health_after;
        next.revision = next.revision.checked_add(1)?;
        Some((next, damage))
    }

    /// Native successor for an already resolved health credit. The occurrence owner
    /// supplies the magnitude and suppresses replay before applying the successor.
    /// Zero/full-pool credit keeps the revision; positive credit requires a successor.
    pub(crate) fn after_health_gain(&self, amount: u64) -> Option<(Self, u32)> {
        let available = self.facts.max_health.checked_sub(self.health)?;
        let gained = u32::try_from(amount.min(u64::from(available))).ok()?;
        let mut next = self.clone();
        if gained != 0 {
            next.health = next.health.checked_add(gained)?;
            next.revision = next.revision.checked_add(1)?;
        }
        Some((next, gained))
    }

    /// Native successor for an already resolved mana credit. This method does not
    /// choose a leech formula or decide which incoming damage component is inverted.
    /// Zero/full-pool credit keeps the revision; positive credit requires a successor.
    pub(crate) fn after_mana_gain(&self, amount: u64) -> Option<(Self, u32)> {
        let available = self.facts.max_mana.checked_sub(self.mana)?;
        let gained = u32::try_from(amount.min(u64::from(available))).ok()?;
        let mut next = self.clone();
        if gained != 0 {
            next.mana = next.mana.checked_add(gained)?;
            next.revision = next.revision.checked_add(1)?;
        }
        Some((next, gained))
    }

    /// Test only: stages a wounded actor (no damage owner exists yet).
    #[cfg(test)]
    pub(crate) const fn set_health_for_test(&mut self, health: u32) {
        self.health = health;
    }

    pub(crate) const fn revision(&self) -> u64 {
        self.revision
    }

    /// The own-actor `ActorVitalsV1` value. Harmony and Serene are the monk's live values
    /// (§8.2), and 0 and false for any other vocation.
    pub(crate) fn vitals(&self) -> ActorVitals {
        ActorVitals {
            health: self.health,
            max_health: self.facts.max_health,
            mana: self.mana,
            max_mana: self.facts.max_mana,
            soul: self.soul,
            harmony: self
                .monk
                .as_ref()
                .map_or(0, |monk| u32::from(monk.harmony())),
            serene: self.monk.as_ref().is_some_and(MonkState::serene),
        }
    }

    fn caster(&self, harmony_multiplier: HarmonyMultiplier) -> CasterState {
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
            harmony_multiplier,
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
    // §8.2: a monk accepts no command before its initialization evaluation. A spender's damage
    // takes the multiplier of the current charges and Serene; no stance owner exists, so no
    // virtue (§4 interim rule).
    let harmony_multiplier = match &state.monk {
        Some(monk) => {
            monk.accept_command()
                .map_err(|_| SpellCastDisposition::Rejected)?;
            match spell.harmony_role {
                Some(HarmonyRole::Spender) => monk
                    .spender_multiplier(state.facts.level, false)
                    .map_err(|_| SpellCastDisposition::Rejected)?,
                Some(HarmonyRole::Builder) | None => HarmonyMultiplier::ONE,
            }
        }
        None => HarmonyMultiplier::ONE,
    };
    let resolution = resolve_cast(
        spell,
        &state.caster(harmony_multiplier),
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
    // §8.2 PRIMARY COMMIT: Harmony changes in this same value, only because the cast succeeded
    // (§A.2 steps 3c and 4). A Harmony spell without a monk state is refused.
    if let Some(role) = spell.harmony_role {
        let monk = next.monk.as_mut().ok_or(SpellCastDisposition::Rejected)?;
        match role {
            HarmonyRole::Builder => monk.commit_builder().map(|_| ()),
            HarmonyRole::Spender => monk.commit_spender().map(|_| ()),
        }
        .map_err(|_| SpellCastDisposition::Rejected)?;
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

impl PlayerSpellState {
    /// DEATH-2 (Reference first player death decision §4.5, D63): the respawned actor's state,
    /// health and mana at their maxima under one successor revision, published as one
    /// `ACTOR_VITALS` delta. `None` when the revision is exhausted.
    pub(crate) fn respawned(&self) -> Option<Self> {
        let mut next = self.clone();
        next.health = next.facts.max_health;
        next.mana = next.facts.max_mana;
        next.revision = next.revision.checked_add(1)?;
        Some(next)
    }
}
