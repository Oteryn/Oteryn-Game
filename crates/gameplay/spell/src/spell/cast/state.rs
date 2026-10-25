//! Runtime player state transitions, vitals and payment witnesses.

use std::collections::BTreeSet;

use oteryn_protocol_oteryn::actor_spell::{ActorVitals, SpellCastDisposition};
use oteryn_simulation_determinism::SemanticTimeMicros;

use super::{CharacterCastFacts, PlayerSpellState, solo};
use crate::ability::creature_bite::{FlooredDamage, floor_creature_damage};
use crate::spell::harmony::{HarmonyMultiplier, MonkState, MonkStateError};
use crate::spell::{CasterState, HarmonyRole, SpellBook, SpellDefinition, Vocation};

impl PlayerSpellState {
    pub fn source_cooldown_deadlines(&self) -> Vec<(String, u64)> {
        self.cooldowns.canonical_deadlines()
    }
    pub const fn character_facts(&self) -> CharacterCastFacts {
        self.facts
    }

    pub fn stage_committed_player_hit_history(
        &mut self,
        source: crate::foundation::ExactActorRef,
        session: crate::foundation::GameSessionId,
        lease: u64,
        at_us: u64,
    ) -> Result<(), SpellCastDisposition> {
        self.field_attack_history
            .record_staged_player_hit(source, session, lease, at_us)
    }
    pub fn source_field_attack_history(
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
    pub const fn source_party_vitals(&self) -> (u32, u32) {
        (self.health, self.facts.max_health)
    }
    /// Current owner-installed Premium fact on the same semantic clock as the kill.
    pub fn hunting_premium_current(&self, now_micros: u64) -> bool {
        self.premium_current
            && self
                .premium_valid_until_micros
                .is_some_and(|until| now_micros < until)
    }
    pub const fn source_party_vocation(&self) -> Vocation {
        self.facts.vocation
    }
    pub const fn owned_base_speed(&self) -> u32 {
        self.base_speed
    }
    pub const fn standard_stance(
        &self,
    ) -> Option<oteryn_gameplay_spell_rules::native_actor_states::StandardStance> {
        self.stance
    }
    pub fn durable_stance_key(&self) -> Option<&str> {
        self.stance_key.as_deref()
    }
    pub fn pending_stance(&self) -> Option<&crate::spell::stance_execution::PreparedStance> {
        self.pending_stance.as_deref()
    }

    pub fn load_owned_build(
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
    pub fn load_owned_stance(
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
        self.stance = crate::spell::stance_execution::active_stance_from_book(
            loaded.key(),
            self.facts.vocation,
            book,
        );
        self.stance_key = loaded.key().map(str::to_owned);
        Ok(())
    }
    pub fn after_creature_mana_drain(&self, magnitude: u32) -> Option<(Self, u32)> {
        if self.health == 0 || magnitude == 0 {
            return None;
        }
        let applied = self.mana.min(magnitude);
        let mut next = self.clone();
        if applied > 0 {
            next.mana -= applied;
            next.revision = next.revision.checked_add(1)?;
        }
        Some((next, applied))
    }

    /// PARITY-EQUIP-MODIFIERS-1: the life and mana an attacker's hit leeches (`game.cpp:8760-8764`),
    /// each capped at its maximum. `None` for a dead actor or when nothing changes, so a leech
    /// that restores nothing is no vitals write.
    #[must_use]
    pub fn after_leech(&self, health_gain: u32, mana_gain: u32) -> Option<Self> {
        if self.health == 0 {
            return None;
        }
        let health = self.health.max(
            self.health
                .saturating_add(health_gain)
                .min(self.facts.max_health),
        );
        let mana = self
            .mana
            .max(self.mana.saturating_add(mana_gain).min(self.facts.max_mana));
        if health == self.health && mana == self.mana {
            return None;
        }
        let mut next = self.clone();
        next.health = health;
        next.mana = mana;
        next.revision = next.revision.checked_add(1)?;
        Some(next)
    }

    /// The Serene initialization evaluation (§8.2), in the owner step that makes the actor
    /// playable: a fresh admission, a same-GameSession reconnect or FND-04B §21 recovery. No
    /// command of a monk is accepted before it.
    pub fn make_playable(&mut self, now: SemanticTimeMicros) -> Result<(), MonkStateError> {
        match &mut self.monk {
            Some(monk) => monk.initialize(now, &solo()),
            None => Ok(()),
        }
    }

    /// The session left the actor: Harmony and the forced time are kept, and commands wait for
    /// the next [`Self::make_playable`].
    pub fn detach(&mut self) {
        if let Some(monk) = &mut self.monk {
            monk.detach();
        }
    }

    /// The periodic 1000 ms Serene evaluation (§8.2). `true` when Serene changed; the change then
    /// has its own `ACTOR_VITALS` revision. On an error nothing changes.
    pub fn tick(&mut self, now: SemanticTimeMicros) -> Result<bool, MonkStateError> {
        self.tick_with_world(now, &solo())
    }
    pub fn tick_with_world(
        &mut self,
        now: SemanticTimeMicros,
        world: &dyn crate::spell::harmony::SereneWorld,
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
    pub fn monk_save_values(&self, now: SemanticTimeMicros) -> Option<(u8, u64)> {
        self.monk
            .as_ref()
            .map(|monk| (monk.harmony(), monk.serene_forced_remaining(now)))
    }

    /// GAME-AI-01 slice §4.7 (D54): the state after one creature hit of `magnitude`. Health never
    /// drops below 1 (no player death in V1). `None` when the hit removes nothing (the actor is
    /// already at 1) or the revision is exhausted; the state is then unchanged.
    pub fn after_creature_damage(&self, magnitude: u32) -> Option<(Self, FlooredDamage)> {
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
    pub fn after_health_gain(&self, amount: u64) -> Option<(Self, u32)> {
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
    pub fn after_mana_gain(&self, amount: u64) -> Option<(Self, u32)> {
        let available = self.facts.max_mana.checked_sub(self.mana)?;
        let gained = u32::try_from(amount.min(u64::from(available))).ok()?;
        let mut next = self.clone();
        if gained != 0 {
            next.mana = next.mana.checked_add(gained)?;
            next.revision = next.revision.checked_add(1)?;
        }
        Some((next, gained))
    }

    /// CHAR-VITALS-1: a newly created runtime actor takes the health and mana its Character's
    /// last-position row saved, each capped at its current maximum, in the owner step that
    /// creates it, before its first command and before any vitals snapshot is sent. It is the
    /// actor's initial state, so the `ACTOR_VITALS` revision is unchanged. A health that would be
    /// zero changes nothing: a saved actor is alive, and the actor keeps its maxima.
    pub fn restore_saved_vitals(&mut self, health: u32, mana: u32) {
        let health = health.min(self.facts.max_health);
        if health == 0 {
            return;
        }
        self.health = health;
        self.mana = mana.min(self.facts.max_mana);
    }

    /// FOOD-REGEN-1 successor: the fed time extended by `regeneration_seconds` under one successor
    /// revision, so the new condition is published like any other ACTOR_VITALS-revision change.
    /// `None` when eating is refused (full or unrepresentable) or the revision is exhausted.
    pub fn after_food(&self, regeneration_seconds: u16, now_ms: u64) -> Option<Self> {
        let mut next = self.clone();
        crate::spell::actor_conditions::eat_food(&mut next, regeneration_seconds, now_ms).ok()?;
        next.revision = next.revision.checked_add(1)?;
        Some(next)
    }

    /// Test only: stages a wounded actor (no damage owner exists yet).
    #[cfg(any(test, feature = "test-support"))]
    pub const fn set_health_for_test(&mut self, health: u32) {
        self.health = health;
    }

    pub const fn revision(&self) -> u64 {
        self.revision
    }

    pub fn payment_anchor_from(
        &self,
        before: &Self,
    ) -> Option<crate::spell::combat_batch::SpellAnchor> {
        let anchor = crate::spell::combat_batch::SpellAnchor {
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
    pub fn paid_successor_of(
        &self,
        before: &Self,
        anchor: &crate::spell::combat_batch::SpellAnchor,
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
            && self.equipped_armor == before.equipped_armor
            && self.equipped_mana_shield == before.equipped_mana_shield
            && self.equipped_defence == before.equipped_defence
            && self.equipped_offence == before.equipped_offence
            && self.base_speed == before.base_speed
            && anchor.expected_revision == before.revision
            && before.revision.checked_add(1) == Some(anchor.next_revision)
            && self.revision == anchor.next_revision
            && before.mana.checked_sub(anchor.paid_mana) == Some(self.mana)
            && before.soul.checked_sub(anchor.paid_soul) == Some(self.soul)
            && self.cooldowns.canonical_deadlines() == anchor.cooldown_deadlines
    }

    pub fn apply_batch_heal(&mut self, magnitude: i64) -> Result<(), SpellCastDisposition> {
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

    pub fn advance_batch_revision(&mut self) -> Result<(), SpellCastDisposition> {
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or(SpellCastDisposition::Rejected)?;
        Ok(())
    }

    /// The own-actor `ActorVitalsV1` value. Harmony and Serene are the monk's live values
    /// (§8.2), and 0 and false for any other vocation.
    pub fn vitals(&self) -> ActorVitals {
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

    pub(super) fn caster(&self, harmony_multiplier: HarmonyMultiplier) -> CasterState {
        CasterState {
            vocation: self.facts.vocation,
            level: self.facts.level,
            magic_level: self.facts.magic_level,
            premium: crate::spell::CasterPremium::from_current(self.premium_current),
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

/// One cast of `intent` by the actor whose state is `state`. On success the result is the next
/// state: the heal applied, mana and soul paid and the cooldowns started, with the revision one
/// higher. Any failure returns its disposition and leaves `state` as it was.
impl PlayerSpellState {
    pub fn owned_harmony_multiplier(
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
                self.stance
                    == Some(
                        oteryn_gameplay_spell_rules::native_actor_states::StandardStance::Harmony,
                    ),
            )
            .map_err(|_| SpellCastDisposition::Rejected)
        } else {
            Ok(HarmonyMultiplier::ONE)
        }
    }
}

impl PlayerSpellState {
    pub fn owned_conditions(
        &self,
    ) -> &oteryn_gameplay_foundation::foundation::condition::ConditionStore<String> {
        &self.conditions
    }
    pub fn apply_batch_conditions(
        &mut self,
        expected: &oteryn_gameplay_foundation::foundation::condition::ConditionStore<String>,
        next: &oteryn_gameplay_foundation::foundation::condition::ConditionStore<String>,
    ) -> Result<(), SpellCastDisposition> {
        if &self.conditions != expected {
            return Err(SpellCastDisposition::Rejected);
        }
        self.conditions = next.clone();
        Ok(())
    }
}

impl PlayerSpellState {
    /// The owning transport resolves current account evidence and physical authority
    /// first. This pure state transition never issues commercial entitlement.
    pub fn apply_owner_premium_transition(
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
    pub fn expire_owner_premium(&mut self, now_micros: u64) -> Result<bool, SpellCastDisposition> {
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
    /// DEATH-2 (Reference first player death decision §4.5, D63): the respawned actor's state,
    /// health and mana at their maxima under one successor revision, published as one
    /// `ACTOR_VITALS` delta. A monk's Harmony and forced Serene are reset, as the death commit
    /// writes them (§8.2), and its Serene initialization evaluation runs at `now`, the owner step
    /// that makes the respawned actor playable. `None` when the revision is exhausted.
    pub fn respawned(&self, now: SemanticTimeMicros) -> Option<Self> {
        let mut next = self.clone();
        crate::spell::actor_conditions::clear_on_lifecycle(&mut next);
        next.health = next.facts.max_health;
        next.mana = next.facts.max_mana;
        next.revision = next.revision.checked_add(1)?;
        next.monk = MonkState::load(next.facts.vocation, 0, 0).ok()?;
        next.make_playable(now).ok()?;
        Some(next)
    }
}
