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
    pub(super) facts: CharacterCastFacts,
    pub(super) premium_current: bool,
    pub(super) premium_valid_until_micros: Option<u64>,
    pub(super) health: u32,
    pub(super) field_attack_history: super::field_history::FieldAttackHistory,
    pub(super) mana: u32,
    pub(super) soul: u32,
    pub(super) cooldowns: Cooldowns,
    pub(super) revision: u64,
    /// SPELL-D8 §8.2: a monk's live Harmony and Serene; `None` for every other vocation.
    pub(super) monk: Option<MonkState>,
    pub(super) conditions: crate::ability::condition::ConditionStore<String>,
    pub(super) base_speed: u32,
    pub(super) movement_pacing: crate::movement::speed::StepPacing,
    pub(super) stance: Option<super::native_actor_states::StandardStance>,
    pub(super) stance_key: Option<String>,
    pub(super) pending_stance: Option<Box<super::stance_execution::PreparedStance>>,
    owned_skills: Option<[u16; 7]>,
    pub(super) training: Option<super::mana_training::LiveManaTraining>,
    pub(super) training_checkpoint: Option<Box<super::mana_training::RetainedTrainingCheckpoint>>,
    pub(super) training_payment: Option<super::mana_training::TrainingPaymentWitness>,
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
    pub(crate) fn source_cooldown_deadlines(&self) -> Vec<(String, u64)> {
        self.cooldowns.canonical_deadlines()
    }
    pub(crate) const fn character_facts(&self) -> CharacterCastFacts {
        self.facts
    }

    pub(crate) fn stage_committed_player_hit_history(
        &mut self,
        source: crate::foundation::ExactActorRef,
        session: crate::foundation::GameSessionId,
        lease: u64,
        at_us: u64,
    ) -> Result<(), SpellCastDisposition> {
        self.field_attack_history
            .record_staged_player_hit(source, session, lease, at_us)
    }
    pub(crate) fn source_field_attack_history(
        &self,
        source: crate::foundation::ExactActorRef,
        session: crate::foundation::GameSessionId,
        lease: u64,
        now_us: u64,
        in_fight_ms: u32,
    ) -> Option<bool> {
        self.field_attack_history
            .has_been_attacked(source, session, lease, now_us, in_fight_ms)
    }
    pub(crate) const fn source_party_vitals(&self) -> (u32, u32) {
        (self.health, self.facts.max_health)
    }
    pub(crate) const fn source_party_vocation(&self) -> Vocation {
        self.facts.vocation
    }
    pub(crate) const fn owned_base_speed(&self) -> u32 {
        self.base_speed
    }
    pub(crate) const fn standard_stance(
        &self,
    ) -> Option<super::native_actor_states::StandardStance> {
        self.stance
    }
    pub(crate) fn durable_stance_key(&self) -> Option<&str> {
        self.stance_key.as_deref()
    }
    pub(crate) fn pending_stance(&self) -> Option<&super::stance_execution::PreparedStance> {
        self.pending_stance.as_deref()
    }

    pub(crate) fn load_owned_build(
        &mut self,
        build: &crate::durability::character_build::DurableBuildState,
    ) -> Result<(), SpellCastDisposition> {
        if Vocation::from_key(build.vocation()) != Some(self.facts.vocation)
            || u32::from(build.magic().0) != self.facts.magic_level
        {
            return Err(SpellCastDisposition::Rejected);
        }
        let skills = build.skills().map(|(level, _)| level);
        if self.owned_skills.is_some_and(|current| current != skills) {
            return Err(SpellCastDisposition::Rejected);
        }
        self.owned_skills = Some(skills);
        Ok(())
    }
    pub(crate) fn load_owned_stance(
        &mut self,
        loaded: &crate::durability::character_stance::DurableCharacterStance,
        book: &SpellBook,
    ) -> Result<(), SpellCastDisposition> {
        if self
            .stance_key
            .as_deref()
            .is_some_and(|key| Some(key) != loaded.key())
        {
            return Err(SpellCastDisposition::Rejected);
        }
        self.stance = super::stance_execution::active_stance_from_book(
            loaded.key(),
            self.facts.vocation,
            book,
        );
        self.stance_key = loaded.key().map(str::to_owned);
        Ok(())
    }
    /// A new runtime actor starts at its maxima (SPELL-D2), with the durable Harmony and remaining
    /// forced Serene time the Character owner loaded (§8.2; both 0 for any other vocation).
    /// Maxima above the SPELL-D8 wire bounds, and corrupt Harmony values, are refused.
    pub(crate) fn new(
        facts: CharacterCastFacts,
        harmony: u8,
        serene_forced_micros: u64,
    ) -> Option<Self> {
        let monk = MonkState::load(facts.vocation, harmony, serene_forced_micros).ok()?;
        // Canary's pinned vocations.xml assigns base speed 110 to every class.
        // The accepted SPEED-1 curve is class base plus (level - 1).
        let base_speed = crate::movement::speed::player_base_speed(facts.level, 110).ok()?;
        (facts.max_health <= MAX_VITAL_POOL
            && facts.max_mana <= MAX_VITAL_POOL
            && facts.max_soul <= MAX_SOUL)
            .then(|| Self {
                facts,
                health: facts.max_health,
                mana: facts.max_mana,
                premium_current: false,
                premium_valid_until_micros: None,
                soul: facts.max_soul,
                cooldowns: Cooldowns::default(),
                revision: 1,
                monk,
                conditions: crate::ability::condition::ConditionStore::new(),
                base_speed,
                movement_pacing: crate::movement::speed::StepPacing::default(),
                stance: None,
                stance_key: None,
                pending_stance: None,
                owned_skills: None,
                training: None,
                training_payment: None,
                field_attack_history: super::field_history::FieldAttackHistory::default(),
                training_checkpoint: None,
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
        self.tick_with_world(now, &solo())
    }
    pub(crate) fn tick_with_world(
        &mut self,
        now: SemanticTimeMicros,
        world: &dyn super::harmony::SereneWorld,
    ) -> Result<bool, MonkStateError> {
        let Some(monk) = &self.monk else {
            return Ok(false);
        };
        let mut monk = monk.clone();
        if !monk.tick(now, world)? {
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

    /// Test only: stages a wounded actor (no damage owner exists yet).
    #[cfg(test)]
    pub(crate) const fn set_health_for_test(&mut self, health: u32) {
        self.health = health;
    }

    pub(crate) const fn revision(&self) -> u64 {
        self.revision
    }

    pub(crate) fn payment_anchor_from(
        &self,
        before: &Self,
    ) -> Option<super::combat_batch::SpellAnchor> {
        let anchor = super::combat_batch::SpellAnchor {
            expected_revision: before.revision,
            next_revision: self.revision,
            paid_mana: before.mana.checked_sub(self.mana)?,
            paid_soul: before.soul.checked_sub(self.soul)?,
            cooldown_deadlines: self.cooldowns.canonical_deadlines(),
        };
        self.paid_successor_of(before, &anchor).then_some(anchor)
    }

    /// Compare the complete common payment against the actual predecessor. This is data
    /// consistency, never current actor/session authority; the Channel owner checks those.
    pub(crate) fn paid_successor_of(
        &self,
        before: &Self,
        anchor: &super::combat_batch::SpellAnchor,
    ) -> bool {
        (self.facts == before.facts && self.training == before.training
            || self.training_payment.as_ref().is_some_and(|witness| {
                witness.matches(
                    before.training.as_ref(),
                    self.training.as_ref(),
                    anchor.paid_mana,
                    before.facts,
                    self.facts,
                )
            }))
            && self.premium_current == before.premium_current
            && self.premium_valid_until_micros == before.premium_valid_until_micros
            && self.owned_skills == before.owned_skills
            && self.base_speed == before.base_speed
            && self.movement_pacing == before.movement_pacing
            && anchor.expected_revision == before.revision
            && before.revision.checked_add(1) == Some(anchor.next_revision)
            && self.revision == anchor.next_revision
            && before.mana.checked_sub(anchor.paid_mana) == Some(self.mana)
            && before.soul.checked_sub(anchor.paid_soul) == Some(self.soul)
            && self.cooldowns.canonical_deadlines() == anchor.cooldown_deadlines
    }

    pub(crate) fn apply_batch_heal(&mut self, magnitude: i64) -> Result<(), SpellCastDisposition> {
        let magnitude = u64::try_from(magnitude).map_err(|_| SpellCastDisposition::Rejected)?;
        if self.health > self.facts.max_health {
            return Err(SpellCastDisposition::Rejected);
        }
        self.health = u32::try_from(
            u64::from(self.health)
                .saturating_add(magnitude)
                .min(u64::from(self.facts.max_health)),
        )
        .map_err(|_| SpellCastDisposition::Rejected)?;
        Ok(())
    }

    pub(crate) fn advance_batch_revision(&mut self) -> Result<(), SpellCastDisposition> {
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or(SpellCastDisposition::Rejected)?;
        Ok(())
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
            premium: self.premium_current,
            mana: self.mana,
            max_mana: self.facts.max_mana,
            soul: self.soul,
            // §4: V1 serves no spell with `learning_required`.
            learned: BTreeSet::new(),
            // No V1 spell reads skills or equipment; they have no owner here yet.
            attack_skill: 0,
            attack_value: 0,
            attack_factor: 0.0,
            shielding_skill: self.owned_skills.map_or(0, |skills| u32::from(skills[5])),
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
impl PlayerSpellState {
    pub(crate) fn owned_harmony_multiplier(
        &self,
        spell: &SpellDefinition,
    ) -> Result<HarmonyMultiplier, SpellCastDisposition> {
        let Some(monk) = &self.monk else {
            return Ok(HarmonyMultiplier::ONE);
        };
        monk.accept_command()
            .map_err(|_| SpellCastDisposition::Rejected)?;
        if spell.harmony_role == Some(HarmonyRole::Spender) {
            monk.spender_multiplier(
                self.facts.level,
                self.stance == Some(super::native_actor_states::StandardStance::Harmony),
            )
            .map_err(|_| SpellCastDisposition::Rejected)
        } else {
            Ok(HarmonyMultiplier::ONE)
        }
    }
}

pub(crate) fn cast(
    book: &SpellBook,
    state: &PlayerSpellState,
    intent: &SpellCastIntent,
    context: CastContext<'_>,
) -> Result<PlayerSpellState, SpellCastDisposition> {
    if state.pending_stance.is_some() || state.training_checkpoint.is_some() {
        return Err(SpellCastDisposition::Rejected);
    }
    let spell = book
        .indexed(intent.spell)
        .ok_or(SpellCastDisposition::Rejected)?;
    cast_resolved(book, state, spell, intent.target, context, None).map(|(next, _)| next)
}
fn cast_resolved(
    book: &SpellBook,
    state: &PlayerSpellState,
    spell: &SpellDefinition,
    target: SpellTarget,
    context: CastContext<'_>,
    qualified: Option<(&CasterState, &super::OperationalCastFacts)>,
) -> Result<
    (
        PlayerSpellState,
        Option<super::executable_catalog::DefinitionRef>,
    ),
    SpellCastDisposition,
> {
    let has_target = match target {
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
                    .spender_multiplier(
                        state.facts.level,
                        state.stance == Some(super::native_actor_states::StandardStance::Harmony),
                    )
                    .map_err(|_| SpellCastDisposition::Rejected)?,
                Some(HarmonyRole::Builder) | None => HarmonyMultiplier::ONE,
            }
        }
        None => HarmonyMultiplier::ONE,
    };
    let mut resolution = match qualified {
        Some((caster, operational)) => {
            if caster.vocation != state.facts.vocation
                || caster.level != state.facts.level
                || caster.magic_level != state.owned_effective_magic_level(context.now.get())?
                || caster.mana != state.mana
                || caster.max_mana != state.facts.max_mana
                || caster.soul != state.soul
                || caster.harmony_multiplier != harmony_multiplier
            {
                return Err(SpellCastDisposition::Rejected);
            }
            super::resolve_cast_with_operational_facts(
                spell,
                caster,
                &state.cooldowns,
                context.now,
                operational,
                context.draw,
            )
        }
        None => {
            let mut caster = state.caster(harmony_multiplier);
            caster.magic_level = state.owned_effective_magic_level(context.now.get())?;
            resolve_cast(
                spell,
                &caster,
                &state.cooldowns,
                context.now,
                has_target,
                context.draw,
            )
        }
    }
    .map_err(|rejection| disposition(&rejection))?;
    let mut next = state.clone();
    let focus_profile = match &spell.execution {
        super::Execution::ActorFocus { profile } => Some(profile),
        super::Execution::NativeProfile(profile)
            if profile.spell()["execution"]["native_behavior"]["key"] == "monk_focus" =>
        {
            Some(&profile.spell()["execution"]["native_behavior"]["parameters"])
        }
        // Other native families must commit through their actual owner compositor.
        // An empty common ability plan never constitutes a successful native cast.
        super::Execution::NativeProfile(_) => return Err(SpellCastDisposition::Rejected),
        _ => None,
    };
    if let Some(profile) = focus_profile {
        if qualified.is_some() {
            let monk = state.monk.as_ref().ok_or(SpellCastDisposition::Rejected)?;
            let planned = super::native_actor_states::plan_focus(
                profile,
                super::native_actor_states::FocusFacts {
                    harmony: monk.harmony(),
                    serene: monk.serene(),
                    forced_until: monk.serene_forced_until(),
                    now: context.now,
                    level: state.facts.level,
                    sustain_active: state.stance
                        == Some(super::native_actor_states::StandardStance::Sustain),
                },
            )
            .map_err(|_| SpellCastDisposition::Rejected)?;
            if planned.healing.is_some() {
                return Err(SpellCastDisposition::TargetIllegal);
            }
        }
        let focus =
            super::actor_execution::apply_focus(&mut next, profile, context.now, context.draw)?;
        resolution.cooldowns.reset_for_focus(book, &focus);
        if focus.rearm_cast_cooldowns {
            resolution
                .cooldowns
                .rearm(spell, context.now)
                .map_err(|rejection| disposition(&rejection))?;
        }
    }
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
    for effect in &plan.side_effects {
        match effect {
            ResolvedEffect::RemoveCondition { condition } => {
                super::actor_conditions::remove_condition(&mut next, condition)?;
            }
            ResolvedEffect::ResolvedOther { profile, .. }
                if qualified.is_some()
                    && super::actor_conditions::is_presentation_only(profile) => {}
            ResolvedEffect::ResolvedOther { profile, .. } => {
                super::actor_conditions::apply_effect(
                    &mut next,
                    profile,
                    context.now.get() / 1_000,
                )?;
            }
            _ => return Err(SpellCastDisposition::Rejected),
        }
    }
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
    Ok((next, resolution.ability_variant))
}

#[derive(Debug)]
pub(crate) struct PaidSourceSelfCast {
    pub(crate) ability_variant: Option<super::executable_catalog::DefinitionRef>,
    pub(crate) next: PlayerSpellState,
    pub(crate) anchor: super::combat_batch::SpellAnchor,
}
#[derive(Debug)]
pub(crate) struct PaidFocusOwnerCast {
    pub(crate) next: PlayerSpellState,
    pub(crate) anchor: super::combat_batch::SpellAnchor,
    pub(crate) plan: super::native_actor_states::FocusPlan,
}
/// Fill/Serenity/cooldowns on the genuine caster successor. Healing remains
/// a typed source roll for the real party compositor to target and draw once.
/// This helper performs no healing draw and introduces no solo-party fact.
pub(crate) fn prepare_focus_owner_cast_with_caster(
    state: &PlayerSpellState,
    spell: &SpellDefinition,
    book: &SpellBook,
    operational: &super::OperationalCastFacts,
    qualified_caster: &CasterState,
    context: CastContext<'_>,
) -> Result<PaidFocusOwnerCast, SpellCastDisposition> {
    let super::Execution::NativeProfile(profile) = &spell.execution else {
        return Err(SpellCastDisposition::Rejected);
    };
    if profile.spell()["execution"]["native_behavior"]["key"] != "monk_focus"
        || !book.spells.iter().any(|active| active == spell)
    {
        return Err(SpellCastDisposition::Rejected);
    }
    let monk = state.monk.as_ref().ok_or(SpellCastDisposition::Rejected)?;
    monk.accept_command()
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let facts = super::native_actor_states::FocusFacts {
        harmony: monk.harmony(),
        serene: monk.serene(),
        forced_until: monk.serene_forced_until(),
        now: context.now,
        level: state.facts.level,
        sustain_active: state.stance == Some(super::native_actor_states::StandardStance::Sustain),
    };
    let mut paid = prepare_native_owner_cast_with_caster(
        state,
        spell,
        operational,
        super::native::Facts::Focus(facts),
        context.now,
        context.draw,
        qualified_caster,
    )?;
    let super::native::Plan::Focus(plan) = paid.plan else {
        return Err(SpellCastDisposition::Rejected);
    };
    let mut next_monk = paid
        .next
        .monk
        .as_ref()
        .ok_or(SpellCastDisposition::Rejected)?
        .clone();
    let gained = next_monk
        .commit_fill()
        .map_err(|_| SpellCastDisposition::Rejected)?;
    if gained != plan.gained_charges || next_monk.harmony() != plan.harmony_after {
        return Err(SpellCastDisposition::Rejected);
    }
    if plan.arm_forced_serene {
        next_monk
            .commit_focus_serenity(context.now)
            .map_err(|_| SpellCastDisposition::Rejected)?;
    }
    if next_monk.serene() != plan.serene_after
        || next_monk.serene_forced_until() != plan.forced_until
    {
        return Err(SpellCastDisposition::Rejected);
    }
    paid.next.monk = Some(next_monk);
    paid.next.cooldowns.reset_for_focus(book, &plan);
    if plan.rearm_cast_cooldowns {
        paid.next
            .cooldowns
            .rearm(spell, context.now)
            .map_err(|reason| disposition(&reason))?;
    }
    paid.anchor.cooldown_deadlines = paid.next.cooldowns.canonical_deadlines();
    Ok(PaidFocusOwnerCast {
        next: paid.next,
        anchor: paid.anchor,
        plan: *plan,
    })
}
/// Source self effects and their real anchor are prepared on one exact actor successor.
pub(crate) fn prepare_source_self_cast_with_caster(
    state: &PlayerSpellState,
    spell: &SpellDefinition,
    book: &SpellBook,
    operational: &super::OperationalCastFacts,
    qualified_caster: &CasterState,
    context: CastContext<'_>,
) -> Result<PaidSourceSelfCast, SpellCastDisposition> {
    if let super::Execution::NativeProfile(profile) = &spell.execution {
        if profile.spell()["execution"]["native_behavior"]["key"] != "monk_focus"
            || !book.spells.iter().any(|active| active == spell)
        {
            return Err(SpellCastDisposition::Rejected);
        }
        let monk = state.monk.as_ref().ok_or(SpellCastDisposition::Rejected)?;
        monk.accept_command()
            .map_err(|_| SpellCastDisposition::Rejected)?;
        let facts = super::native_actor_states::FocusFacts {
            harmony: monk.harmony(),
            serene: monk.serene(),
            forced_until: monk.serene_forced_until(),
            now: context.now,
            level: state.facts.level,
            sustain_active: state.stance
                == Some(super::native_actor_states::StandardStance::Sustain),
        };
        let mut paid = prepare_native_owner_cast_with_caster(
            state,
            spell,
            operational,
            super::native::Facts::Focus(facts),
            context.now,
            context.draw,
            qualified_caster,
        )?;
        // Source chooses the lowest absolute HP among self and visible party.
        // This route has no genuine membership/no-party owner proof, so refuse
        // a gained-charge heal before the first draw or any owner mutation.
        let super::native::Plan::Focus(planned) = &paid.plan else {
            return Err(SpellCastDisposition::Rejected);
        };
        if planned.healing.is_some() {
            return Err(SpellCastDisposition::TargetIllegal);
        }
        let focus = super::actor_execution::apply_focus(
            &mut paid.next,
            &profile.spell()["execution"]["native_behavior"]["parameters"],
            context.now,
            context.draw,
        )?;
        paid.next.cooldowns.reset_for_focus(book, &focus);
        if focus.rearm_cast_cooldowns {
            paid.next
                .cooldowns
                .rearm(spell, context.now)
                .map_err(|r| disposition(&r))?;
        }
        paid.anchor.cooldown_deadlines = paid.next.cooldowns.canonical_deadlines();
        return Ok(PaidSourceSelfCast {
            next: paid.next,
            anchor: paid.anchor,
            ability_variant: None,
        });
    }
    if state.pending_stance.is_some()
        || state.training_checkpoint.is_some()
        || spell.needs_target
        || spell.target_or_direction
        || spell.chain.is_some()
        || operational.target.is_some()
        || operational.target_position.is_some()
        || !book.spells.iter().any(|active| active == spell)
        || !matches!(
            spell.execution,
            super::Execution::Effects(_)
                | super::Execution::AbilityVariants(_)
                | super::Execution::ActorFocus { .. }
        )
    {
        return Err(SpellCastDisposition::Rejected);
    }
    let (next, ability_variant) = cast_resolved(
        book,
        state,
        spell,
        SpellTarget::None,
        context,
        Some((qualified_caster, operational)),
    )?;
    let anchor = super::combat_batch::SpellAnchor {
        expected_revision: state.revision,
        next_revision: next.revision,
        paid_mana: state
            .mana
            .checked_sub(next.mana)
            .ok_or(SpellCastDisposition::Rejected)?,
        paid_soul: state
            .soul
            .checked_sub(next.soul)
            .ok_or(SpellCastDisposition::Rejected)?,
        cooldown_deadlines: next.cooldowns.canonical_deadlines(),
    };
    Ok(PaidSourceSelfCast {
        next,
        anchor,
        ability_variant,
    })
}

/// An ordinary source cast pays only its common anchor here. Every resolved
/// effect is retained for the actual target/condition/presentation owner batch.
#[derive(Debug)]
pub(crate) struct PaidOrdinaryCast {
    pub(crate) next: PlayerSpellState,
    pub(crate) anchor: super::combat_batch::SpellAnchor,
    pub(crate) resolution: super::CastResolution,
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn prepare_ordinary_owner_cast_with_caster(
    state: &PlayerSpellState,
    spell: &SpellDefinition,
    operational: &super::OperationalCastFacts,
    now: SemanticTimeMicros,
    caster: &CasterState,
    party: Option<&dyn super::party::PartyWorld>,
    chain: Option<(&dyn super::chain::ChainWorld, super::chain::ChainStart)>,
    draw: &mut dyn FnMut(i64, i64) -> i64,
) -> Result<PaidOrdinaryCast, SpellCastDisposition> {
    if state.pending_stance.is_some()
        || state.training_checkpoint.is_some()
        || !matches!(
            spell.execution,
            super::Execution::Effects(_)
                | super::Execution::AbilityVariants(_)
                | super::Execution::PartyBuff(_)
        )
        || caster.vocation != state.facts.vocation
        || caster.level != state.facts.level
        || caster.magic_level != state.owned_effective_magic_level(now.get())?
        || caster.mana != state.mana
        || caster.max_mana != state.facts.max_mana
        || caster.soul != state.soul
        || caster.harmony_multiplier != state.owned_harmony_multiplier(spell)?
    {
        return Err(SpellCastDisposition::Rejected);
    }
    super::check_operational_rules(spell, operational, false).map_err(|e| disposition(&e))?;
    let resolution = if matches!(spell.execution, super::Execution::PartyBuff(_)) {
        super::resolve_party_cast(
            spell,
            caster,
            &state.cooldowns,
            now,
            party.ok_or(SpellCastDisposition::TargetRequired)?,
            draw,
        )
    } else if spell.chains(operational.target.is_some()) {
        let (world, start) = chain.ok_or(SpellCastDisposition::TargetRequired)?;
        super::resolve_chain_cast(
            spell,
            caster,
            &state.cooldowns,
            now,
            world,
            start,
            &mut |minimum, _| minimum,
        )
    } else {
        super::resolve_cast_with_operational_facts(
            spell,
            caster,
            &state.cooldowns,
            now,
            operational,
            draw,
        )
    }
    .map_err(|e| disposition(&e))?;
    let mut next = state.clone();
    next.mana = next
        .mana
        .checked_sub(resolution.mana_spent)
        .ok_or(SpellCastDisposition::Rejected)?;
    next.soul = next
        .soul
        .checked_sub(resolution.soul_spent)
        .ok_or(SpellCastDisposition::Rejected)?;
    next.cooldowns = resolution.cooldowns.clone();
    next.revision = next
        .revision
        .checked_add(1)
        .ok_or(SpellCastDisposition::Rejected)?;
    if let Some(role) = spell.harmony_role {
        let monk = next.monk.as_mut().ok_or(SpellCastDisposition::Rejected)?;
        match role {
            HarmonyRole::Builder => monk.commit_builder().map(|_| ()),
            HarmonyRole::Spender => monk.commit_spender().map(|_| ()),
        }
        .map_err(|_| SpellCastDisposition::Rejected)?;
    }
    let anchor = super::combat_batch::SpellAnchor {
        expected_revision: state.revision,
        next_revision: next.revision,
        paid_mana: resolution.mana_spent,
        paid_soul: resolution.soul_spent,
        cooldown_deadlines: next.cooldowns.canonical_deadlines(),
    };
    Ok(PaidOrdinaryCast {
        next,
        anchor,
        resolution,
    })
}
/// Source chain callbacks obtain a fresh numerical cast from actual due owner
/// facts. This does not recheck or pay cooldowns, mana, soul or Harmony receipts.
pub(crate) fn resolve_ordinary_due_effects(
    spell: &SpellDefinition,
    caster: &CasterState,
    step: u32,
    draw: &mut dyn FnMut(i64, i64) -> i64,
) -> Result<Vec<super::ResolvedEffect>, SpellCastDisposition> {
    let super::Execution::Effects(effects) = &spell.execution else {
        return Err(SpellCastDisposition::Rejected);
    };
    let chain = spell.chain.as_ref().ok_or(SpellCastDisposition::Rejected)?;
    if spell.authored.is_none() || step > chain.max_targets {
        return Err(SpellCastDisposition::Rejected);
    }
    let inputs = super::formula::FormulaInputs {
        level: caster.level,
        magic_level: caster.magic_level,
        base_power: spell.base_power,
        attack_skill: caster.attack_skill,
        attack_value: caster.attack_value,
        attack_factor: caster.attack_factor,
        shielding_skill: caster.shielding_skill,
        shield_defense: caster.shield_defense,
    };
    effects
        .iter()
        .map(|effect| {
            super::resolve_effect(effect, &inputs, caster.harmony_multiplier, draw)
                .map(|effect| super::scale(effect, step, chain.damage_step_percent))
                .map_err(|_| SpellCastDisposition::Rejected)
        })
        .collect()
}

impl PlayerSpellState {
    pub(crate) fn owned_conditions(&self) -> &crate::ability::condition::ConditionStore<String> {
        &self.conditions
    }
    pub(crate) fn apply_batch_conditions(
        &mut self,
        expected: &crate::ability::condition::ConditionStore<String>,
        next: &crate::ability::condition::ConditionStore<String>,
    ) -> Result<(), SpellCastDisposition> {
        if &self.conditions != expected {
            return Err(SpellCastDisposition::Rejected);
        }
        self.conditions = next.clone();
        Ok(())
    }
}

/// A prepared native cast has no authority to mutate either owner. The transport compositor
/// joins this exact anchor to a physical/world/durable owner transaction and compares the
/// actual player predecessor again before installing `next`.
#[derive(Debug)]
pub(crate) struct PaidNativeCast {
    pub(crate) next: PlayerSpellState,
    pub(crate) anchor: super::combat_batch::SpellAnchor,
    pub(crate) plan: super::native::Plan,
}

pub(crate) fn prepare_native_owner_cast(
    state: &PlayerSpellState,
    spell: &SpellDefinition,
    operational: &super::OperationalCastFacts,
    facts: super::native::Facts<'_>,
    now: SemanticTimeMicros,
    draw: &mut dyn FnMut(i64, i64) -> i64,
) -> Result<PaidNativeCast, SpellCastDisposition> {
    prepare_native_owner_cast_impl(state, spell, operational, facts, now, draw, None)
}

pub(crate) fn prepare_native_owner_cast_with_caster(
    state: &PlayerSpellState,
    spell: &SpellDefinition,
    operational: &super::OperationalCastFacts,
    facts: super::native::Facts<'_>,
    now: SemanticTimeMicros,
    draw: &mut dyn FnMut(i64, i64) -> i64,
    caster: &CasterState,
) -> Result<PaidNativeCast, SpellCastDisposition> {
    prepare_native_owner_cast_impl(state, spell, operational, facts, now, draw, Some(caster))
}
fn prepare_native_owner_cast_impl(
    state: &PlayerSpellState,
    spell: &SpellDefinition,
    operational: &super::OperationalCastFacts,
    facts: super::native::Facts<'_>,
    now: SemanticTimeMicros,
    draw: &mut dyn FnMut(i64, i64) -> i64,
    qualified_caster: Option<&CasterState>,
) -> Result<PaidNativeCast, SpellCastDisposition> {
    if state.pending_stance.is_some() || state.training_checkpoint.is_some() {
        return Err(SpellCastDisposition::Rejected);
    }
    let multiplier = match &state.monk {
        Some(monk) => {
            monk.accept_command()
                .map_err(|_| SpellCastDisposition::Rejected)?;
            if spell.harmony_role == Some(HarmonyRole::Spender) {
                monk.spender_multiplier(
                    state.facts.level,
                    state.stance == Some(super::native_actor_states::StandardStance::Harmony),
                )
                .map_err(|_| SpellCastDisposition::Rejected)?
            } else {
                HarmonyMultiplier::ONE
            }
        }
        None => HarmonyMultiplier::ONE,
    };
    let mut baseline = state.caster(multiplier);
    baseline.magic_level = state.owned_effective_magic_level(now.get())?;
    let caster = qualified_caster.unwrap_or(&baseline);
    if caster.vocation != state.facts.vocation
        || caster.level != state.facts.level
        || caster.magic_level != state.owned_effective_magic_level(now.get())?
        || caster.mana != state.mana
        || caster.max_mana != state.facts.max_mana
        || caster.soul != state.soul
        || caster.harmony_multiplier != multiplier
    {
        return Err(SpellCastDisposition::Rejected);
    }
    let resolved = super::resolve_native_cast(
        spell,
        caster,
        &state.cooldowns,
        now,
        operational,
        facts,
        draw,
    )
    .map_err(|error| disposition(&error))?;
    let mut next = state.clone();
    next.mana = next
        .mana
        .checked_sub(resolved.common.mana_spent)
        .ok_or(SpellCastDisposition::Rejected)?;
    next.soul = next
        .soul
        .checked_sub(resolved.common.soul_spent)
        .ok_or(SpellCastDisposition::Rejected)?;
    next.cooldowns = resolved.common.cooldowns;
    next.revision = next
        .revision
        .checked_add(1)
        .ok_or(SpellCastDisposition::Rejected)?;
    if let Some(role) = spell.harmony_role {
        let monk = next.monk.as_mut().ok_or(SpellCastDisposition::Rejected)?;
        match role {
            HarmonyRole::Builder => monk.commit_builder().map(|_| ()),
            HarmonyRole::Spender => monk.commit_spender().map(|_| ()),
        }
        .map_err(|_| SpellCastDisposition::Rejected)?;
    }
    let anchor = super::combat_batch::SpellAnchor {
        expected_revision: state.revision,
        next_revision: next.revision,
        paid_mana: resolved.common.mana_spent,
        paid_soul: resolved.common.soul_spent,
        cooldown_deadlines: next.cooldowns.canonical_deadlines(),
    };
    Ok(PaidNativeCast {
        next,
        anchor,
        plan: resolved.plan,
    })
}

#[derive(Debug)]
pub(crate) struct PaidNativeCallbackCast {
    pub(crate) next: PlayerSpellState,
    pub(crate) anchor: super::combat_batch::SpellAnchor,
}

pub(crate) fn prepare_native_callback_owner_cast_with_caster(
    state: &PlayerSpellState,
    spell: &SpellDefinition,
    operational: &super::OperationalCastFacts,
    now: SemanticTimeMicros,
    qualified_caster: &CasterState,
) -> Result<PaidNativeCallbackCast, SpellCastDisposition> {
    prepare_native_callback_owner_cast_core(state, spell, operational, now, qualified_caster, None)
}
/// The source acquisition owns its dynamic Creature mana. Its qualified header
/// is still resolved through all ordinary common checks exactly once.
pub(crate) fn prepare_direct_acquisition_owner_cast_with_caster(
    state: &PlayerSpellState,
    spell: &SpellDefinition,
    operational: &super::OperationalCastFacts,
    now: SemanticTimeMicros,
    qualified_caster: &CasterState,
    reservation: &super::companion_lifecycle::DirectCompanionReservation,
) -> Result<PaidNativeCallbackCast, SpellCastDisposition> {
    let super::Execution::NativeProfile(profile) = &spell.execution else {
        return Err(SpellCastDisposition::Rejected);
    };
    if !reservation.qualifies_payment(profile, state.mana)
        || super::mana_cost(spell, qualified_caster) != 0
    {
        return Err(SpellCastDisposition::Rejected);
    }
    let mut paid = prepare_native_callback_owner_cast_with_caster(
        state,
        spell,
        operational,
        now,
        qualified_caster,
    )?;
    if paid.anchor.paid_mana != 0 {
        return Err(SpellCastDisposition::Rejected);
    }
    let cost = reservation.mana_to_deduct();
    paid.next.mana = paid
        .next
        .mana
        .checked_sub(cost)
        .ok_or(SpellCastDisposition::NotEnoughMana)?;
    paid.anchor.paid_mana = cost;
    Ok(paid)
}
pub(crate) fn prepare_native_callback_owner_cast_with_carried_item(
    state: &PlayerSpellState,
    spell: &SpellDefinition,
    operational: &super::OperationalCastFacts,
    now: SemanticTimeMicros,
    qualified_caster: &CasterState,
    target: &crate::durability::spell_item_transaction::CarriedSpellTarget,
) -> Result<PaidNativeCallbackCast, SpellCastDisposition> {
    prepare_native_callback_owner_cast_core(
        state,
        spell,
        operational,
        now,
        qualified_caster,
        Some(target),
    )
}
fn prepare_native_callback_owner_cast_core(
    state: &PlayerSpellState,
    spell: &SpellDefinition,
    operational: &super::OperationalCastFacts,
    now: SemanticTimeMicros,
    qualified_caster: &CasterState,
    carried: Option<&crate::durability::spell_item_transaction::CarriedSpellTarget>,
) -> Result<PaidNativeCallbackCast, SpellCastDisposition> {
    if state.pending_stance.is_some() || state.training_checkpoint.is_some() {
        return Err(SpellCastDisposition::Rejected);
    }
    let multiplier = match &state.monk {
        Some(monk) => {
            monk.accept_command()
                .map_err(|_| SpellCastDisposition::Rejected)?;
            if spell.harmony_role == Some(HarmonyRole::Spender) {
                monk.spender_multiplier(
                    state.facts.level,
                    state.stance == Some(super::native_actor_states::StandardStance::Harmony),
                )
                .map_err(|_| SpellCastDisposition::Rejected)?
            } else {
                HarmonyMultiplier::ONE
            }
        }
        None => HarmonyMultiplier::ONE,
    };
    let caster = qualified_caster;
    if caster.vocation != state.facts.vocation
        || caster.level != state.facts.level
        || caster.magic_level != state.owned_effective_magic_level(now.get())?
        || caster.mana != state.mana
        || caster.max_mana != state.facts.max_mana
        || caster.soul != state.soul
        || caster.harmony_multiplier != multiplier
    {
        return Err(SpellCastDisposition::Rejected);
    }
    let common = match carried {
        Some(target) => super::resolve_native_callback_common_for_carried_item(
            spell,
            caster,
            &state.cooldowns,
            now,
            operational,
            target,
        ),
        None => {
            super::resolve_native_callback_common(spell, caster, &state.cooldowns, now, operational)
        }
    }
    .map_err(|error| disposition(&error))?;
    let mut next = state.clone();
    next.mana = next
        .mana
        .checked_sub(common.mana_spent)
        .ok_or(SpellCastDisposition::Rejected)?;
    next.soul = next
        .soul
        .checked_sub(common.soul_spent)
        .ok_or(SpellCastDisposition::Rejected)?;
    next.cooldowns = common.cooldowns;
    next.revision = next
        .revision
        .checked_add(1)
        .ok_or(SpellCastDisposition::Rejected)?;
    if let Some(role) = spell.harmony_role {
        let monk = next.monk.as_mut().ok_or(SpellCastDisposition::Rejected)?;
        match role {
            HarmonyRole::Builder => monk.commit_builder().map(|_| ()),
            HarmonyRole::Spender => monk.commit_spender().map(|_| ()),
        }
        .map_err(|_| SpellCastDisposition::Rejected)?;
    }
    let anchor = super::combat_batch::SpellAnchor {
        expected_revision: state.revision,
        next_revision: next.revision,
        paid_mana: common.mana_spent,
        paid_soul: common.soul_spent,
        cooldown_deadlines: next.cooldowns.canonical_deadlines(),
    };
    Ok(PaidNativeCallbackCast { next, anchor })
}

/// Pure common payment for the exact ordinary Conjure source profile. Items
/// and the real reagent custody must be staged by their durable owner; this
/// helper performs no grant, charge mutation or random selection.
pub(crate) fn prepare_conjure_owner_cast_with_caster(
    state: &PlayerSpellState,
    spell: &SpellDefinition,
    operational: &super::OperationalCastFacts,
    now: SemanticTimeMicros,
    qualified_caster: &CasterState,
) -> Result<PaidSourceSelfCast, SpellCastDisposition> {
    let super::Execution::Conjure {
        reagent,
        result,
        count,
    } = &spell.execution
    else {
        return Err(SpellCastDisposition::Rejected);
    };
    if *result == 0
        || *count == 0
        || *count > 50000
        || spell.needs_target
        || spell.target_or_direction
        || operational.target.is_some()
        || operational.target_position.is_some()
    {
        return Err(SpellCastDisposition::Rejected);
    }
    if state.pending_stance.is_some() || state.training_checkpoint.is_some() {
        return Err(SpellCastDisposition::Rejected);
    }
    let multiplier = match &state.monk {
        Some(monk) => {
            monk.accept_command()
                .map_err(|_| SpellCastDisposition::Rejected)?;
            if spell.harmony_role == Some(HarmonyRole::Spender) {
                monk.spender_multiplier(
                    state.facts.level,
                    state.stance == Some(super::native_actor_states::StandardStance::Harmony),
                )
                .map_err(|_| SpellCastDisposition::Rejected)?
            } else {
                HarmonyMultiplier::ONE
            }
        }
        None => HarmonyMultiplier::ONE,
    };
    let caster = qualified_caster;
    if caster.vocation != state.facts.vocation
        || caster.level != state.facts.level
        || caster.magic_level != state.owned_effective_magic_level(now.get())?
        || caster.mana != state.mana
        || caster.max_mana != state.facts.max_mana
        || caster.soul != state.soul
        || caster.harmony_multiplier != multiplier
    {
        return Err(SpellCastDisposition::Rejected);
    }
    let mut unexpected_draw = false;
    let common = super::resolve_cast_with_operational_facts(
        spell,
        caster,
        &state.cooldowns,
        now,
        operational,
        &mut |minimum, _| {
            unexpected_draw = true;
            minimum
        },
    )
    .map_err(|error| disposition(&error))?;
    if unexpected_draw
        || common.effects
            != vec![super::ResolvedEffect::Conjure {
                reagent: *reagent,
                result: *result,
                count: *count,
            }]
    {
        return Err(SpellCastDisposition::Rejected);
    }
    let mut next = state.clone();
    next.mana = next
        .mana
        .checked_sub(common.mana_spent)
        .ok_or(SpellCastDisposition::Rejected)?;
    next.soul = next
        .soul
        .checked_sub(common.soul_spent)
        .ok_or(SpellCastDisposition::Rejected)?;
    next.cooldowns = common.cooldowns;
    next.revision = next
        .revision
        .checked_add(1)
        .ok_or(SpellCastDisposition::Rejected)?;
    if let Some(role) = spell.harmony_role {
        let monk = next.monk.as_mut().ok_or(SpellCastDisposition::Rejected)?;
        match role {
            HarmonyRole::Builder => monk.commit_builder().map(|_| ()),
            HarmonyRole::Spender => monk.commit_spender().map(|_| ()),
        }
        .map_err(|_| SpellCastDisposition::Rejected)?;
    }
    let anchor = super::combat_batch::SpellAnchor {
        expected_revision: state.revision,
        next_revision: next.revision,
        paid_mana: common.mana_spent,
        paid_soul: common.soul_spent,
        cooldown_deadlines: next.cooldowns.canonical_deadlines(),
    };
    Ok(PaidSourceSelfCast {
        next,
        anchor,
        ability_variant: None,
    })
}

/// The source name resolver's rejected Find Person callback starts its engine
/// cooldowns but debits no resources and produces no successful Harmony change.
pub(crate) fn prepare_locate_name_failure_owner_cast_with_caster(
    state: &PlayerSpellState,
    spell: &SpellDefinition,
    operational: &super::OperationalCastFacts,
    now: SemanticTimeMicros,
    caster: &CasterState,
) -> Result<PaidNativeCallbackCast, SpellCastDisposition> {
    let super::Execution::NativeProfile(profile) = &spell.execution else {
        return Err(SpellCastDisposition::Rejected);
    };
    let behavior = &profile.spell()["execution"]["native_behavior"];
    if behavior["key"] != "locate_message"
        || behavior["parameters"]["name_resolution_failure_starts_cooldown"] != true
        || behavior["parameters"]["name_resolution_failure_spends_mana"] != false
        || spell.harmony_role.is_some()
    {
        return Err(SpellCastDisposition::Rejected);
    }
    let common =
        prepare_native_callback_owner_cast_with_caster(state, spell, operational, now, caster)?;
    let mut next = state.clone();
    next.cooldowns = common.next.cooldowns;
    next.revision = common.next.revision;
    let anchor = super::combat_batch::SpellAnchor {
        expected_revision: state.revision,
        next_revision: next.revision,
        paid_mana: 0,
        paid_soul: 0,
        cooldown_deadlines: next.cooldowns.canonical_deadlines(),
    };
    Ok(PaidNativeCallbackCast { next, anchor })
}

/// Source engine name-resolution failure: access checks precede name lookup,
/// then only spell/group cooldowns change. No healing draw or Harmony success.
pub(crate) fn prepare_named_player_failure_owner_cast_with_caster(
    state: &PlayerSpellState,
    spell: &SpellDefinition,
    now: SemanticTimeMicros,
    caster: &CasterState,
) -> Result<PaidNativeCallbackCast, SpellCastDisposition> {
    let profile = spell
        .authored
        .as_ref()
        .ok_or(SpellCastDisposition::Rejected)?;
    if !matches!(
        spell.name.as_str(),
        "Heal Friend" | "Nature's Embrace" | "Restore Balance"
    ) || profile.header.targeting.parameter != "player_name"
        || !spell.needs_target
        || spell.aggressive
        || state.pending_stance.is_some()
        || state.training_checkpoint.is_some()
        || caster.vocation != state.facts.vocation
        || caster.level != state.facts.level
        || caster.magic_level != state.owned_effective_magic_level(now.get())?
        || caster.mana != state.mana
        || caster.max_mana != state.facts.max_mana
        || caster.soul != state.soul
        || caster.harmony_multiplier != state.owned_harmony_multiplier(spell)?
    {
        return Err(SpellCastDisposition::Rejected);
    }
    if let Some(group) = spell.groups.first() {
        super::check_group(&state.cooldowns, &group.key, now).map_err(|e| disposition(&e))?;
    }
    if state
        .cooldowns
        .spell_ready_at(&spell.key)
        .is_some_and(|at| at > now)
    {
        return Err(SpellCastDisposition::CoolingDown);
    }
    for group in spell.groups.iter().skip(1) {
        super::check_group(&state.cooldowns, &group.key, now).map_err(|e| disposition(&e))?;
    }
    if caster.level < spell.level {
        return Err(SpellCastDisposition::LevelTooLow);
    }
    if caster.mana < super::mana_cost(spell, caster) {
        return Err(SpellCastDisposition::NotEnoughMana);
    }
    if caster.soul < spell.soul {
        return Err(SpellCastDisposition::NotEnoughSoul);
    }
    if (spell.learning_required && !caster.learned.contains(&spell.key))
        || (!spell.learning_required && !spell.vocations.contains(&caster.vocation))
        || (spell.needs_weapon && !caster.melee_weapon)
        || (spell.premium && !caster.premium)
        || (spell.needs_shield && caster.shield_defense.is_none())
    {
        return Err(SpellCastDisposition::Rejected);
    }
    let mut next = state.clone();
    next.cooldowns
        .rearm(spell, now)
        .map_err(|e| disposition(&e))?;
    next.revision = next
        .revision
        .checked_add(1)
        .ok_or(SpellCastDisposition::Rejected)?;
    let anchor = super::combat_batch::SpellAnchor {
        expected_revision: state.revision,
        next_revision: next.revision,
        paid_mana: 0,
        paid_soul: 0,
        cooldown_deadlines: next.cooldowns.canonical_deadlines(),
    };
    Ok(PaidNativeCallbackCast { next, anchor })
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
    /// The owning transport resolves current account evidence and physical authority
    /// first. This pure state transition never issues commercial entitlement.
    pub(crate) fn apply_owner_premium_transition(
        &mut self,
        current: bool,
        valid_until_micros: Option<u64>,
    ) -> Result<bool, SpellCastDisposition> {
        if current && valid_until_micros.is_none() {
            return Err(SpellCastDisposition::Rejected);
        }
        let maximum = crate::domain::premium::soul_maximum(current);
        let soul = crate::domain::premium::apply_account_transition(self.soul, current);
        let changed =
            self.premium_current != current || self.facts.max_soul != maximum || self.soul != soul;
        let revision = if changed {
            self.revision
                .checked_add(1)
                .ok_or(SpellCastDisposition::Rejected)?
        } else {
            self.revision
        };
        self.premium_current = current;
        self.premium_valid_until_micros = if current { valid_until_micros } else { None };
        self.facts.max_soul = maximum;
        self.soul = soul;
        self.revision = revision;
        Ok(changed)
    }
    pub(crate) fn expire_owner_premium(
        &mut self,
        now_micros: u64,
    ) -> Result<bool, SpellCastDisposition> {
        if self.premium_current
            && self
                .premium_valid_until_micros
                .is_none_or(|until| now_micros >= until)
        {
            self.apply_owner_premium_transition(false, None)
        } else {
            Ok(false)
        }
    }
}
