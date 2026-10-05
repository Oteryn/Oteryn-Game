//! Spell cast wire composition (`OTERYN_PLAYER_SPELL_CAST_WIRE_AND_VITALS_CONTRACT_CANDIDATE_V1`
//! §9 step 2). The typed payload codecs live in `oteryn-protocol-oteryn::actor_spell` (#1254) and
//! are re-exported here, as `world_spatial` does; this module adds the Channel owner's per-actor
//! vitals and cooldowns and the owner work item of one cast.

pub(crate) use oteryn_protocol_oteryn::actor_spell::*;
#[path = "actor_conditions.rs"]
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
mod actor_conditions;
#[path = "actor_invisibility.rs"]
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
mod actor_invisibility;
pub(crate) use actor_conditions::{
    NativeConditionAdmission, NativeConditionApplication, NativeConditionError,
};
#[cfg(test)]
pub(crate) use actor_invisibility::CreatureVision;
pub(crate) use actor_invisibility::{
    InvisibilityError, InvisibleTileCombatPolicy, SelfInvisibleSource, SelfSpeedSource,
};

#[path = "actor_movement.rs"]
mod actor_movement;
#[path = "familiar_cast.rs"]
mod familiar_cast;
#[path = "movement_equipment.rs"]
mod movement_equipment;
pub(in crate::gameplay_transport) use familiar_cast::FamiliarLogoutSave;
#[path = "familiar_defense.rs"]
mod familiar_defense;
#[path = "field_step_ingress.rs"]
mod field_step_ingress;
#[path = "native_combat_cast.rs"]
mod native_combat_cast;
#[path = "parameter_cast.rs"]
mod parameter_cast;
#[path = "spell_timer_callbacks.rs"]
mod spell_timer_callbacks;
#[path = "world_item_cast.rs"]
mod world_item_cast;
pub(in crate::gameplay_transport) use native_combat_cast::NativeCastDispatch;
pub(in crate::gameplay_transport) use native_combat_cast::ordinary_combat::due_chain_presentations as prepare_ordinary_due_presentations;
pub(in crate::gameplay_transport) use native_combat_cast::ordinary_combat::prepare_due as prepare_ordinary_due_from_owners;
#[path = "spell_periodic.rs"]
mod spell_periodic;
#[path = "training_save.rs"]
mod training_save;
pub(in crate::gameplay_transport) use training_save::TrainingSave;
#[path = "actor_spell_commit.rs"]
mod owner_commit;
#[path = "stance_cast.rs"]
mod stance_cast;
pub(crate) use owner_commit::{PlayerBatchPreflight, commit_owner_batch, stage_player_batch};

use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, SemanticTimeMicros, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

use crate::ability::creature_bite::{CreatureBiteVitals, CreatureHit};
use crate::ability::{AbilityOccurrence, RevisionSet};
use crate::durability::character_death::PlayerDeathOccurrence;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition};
use crate::spell::SpellBook;
use crate::spell::cast::{CastContext, CharacterCastFacts, PlayerSpellState, cast};

/// Spell cast §4 (SPELL-D2): the Channel owner's vitals and cooldowns of its present player
/// actors, runtime-actor-local and never durable. Like the door runtime it sits beside the
/// `ChannelRuntimeV1` and is only used while that runtime is locked (runtime first), within the
/// same owner work item. An entry is keyed by the exact actor (World, Channel, scope generation,
/// slot and slot generation), so it lives exactly as long as that actor stays present: every read
/// and write first proves the actor is still the committed player of the GameSession, and an
/// entry whose actor is gone is unreachable and dropped at the next initialization.
///
/// DEATH-2 (Reference first player death decision §4.2, §4.5): the vitals owner also keeps each
/// present player's death, from the lethal write until its respawn. A dead player takes no
/// command, no damage and no credit, and its death record goes with its entry.
#[derive(Debug)]
pub(crate) struct ChannelSpellStates {
    pub(in crate::gameplay_transport) owner_wake: std::sync::Arc<tokio::sync::Notify>,
    actors: Vec<(ExactActorRef, GameSessionId, PlayerSpellState)>,
    /// One real Channel-owner lane; no timer is restored from client intent.
    pub(crate) spell_timers: Option<crate::spell::delayed_execution::SpellTimerOwner>,
    pub(crate) pending_familiars: Vec<familiar_cast::PreparedFamiliarCast>,
    pub(crate) pending_familiar_lifecycle: Vec<familiar_cast::PreparedFamiliarLifecycle>,
    pub(crate) pending_familiar_logouts: Vec<familiar_cast::PreparedFamiliarLogout>,
    pub(crate) pending_native: Vec<native_combat_cast::PendingNativeCast>,
    pub(crate) pending_world_items: Vec<world_item_cast::PreparedWorldItemCast>,
    pub(crate) pending_parameters: Vec<parameter_cast::PreparedParameterCast>,
    pub(crate) presentations: Option<super::spell_presentations::SpellPresentationOwner>,
    /// One bounded Channel pass per second, shared by all present actors.
    pub(in crate::gameplay_transport) next_item_deadline_pass_us: u64,
    pub(in crate::gameplay_transport) source_map_initialized:
        Option<crate::durability::native_map_items_abi::NativeMapOwnerBinding>,
    pub(in crate::gameplay_transport) next_map_initialization_pass_us: u64,
    pub(in crate::gameplay_transport) next_party_deadline_pass_us: u64,
    pub(in crate::gameplay_transport) monster_melee: crate::ai_monster_melee::MonsterMeleeOwner,
    pub(in crate::gameplay_transport) next_monster_ai_pass_us: u64,
    pub(in crate::gameplay_transport) monster_ai_sequence: u64,
    pub(in crate::gameplay_transport) monster_ai_cursor: usize,
    deaths: Vec<(ExactActorRef, GameSessionId, PlayerDeath)>,
    mint_death: fn() -> Option<PlayerDeathOccurrence>,
    source_secondary_memos: Vec<actor_conditions::NativeSecondaryMemo>,
    source_damage: Vec<NativeSourceMemo<crate::player_lethal::PlayerDamageReceipt>>,
    source_mana: Vec<NativeSourceMemo<crate::player_lethal::PlayerManaDrainReceipt>>,
    source_heal: Vec<NativeSourceMemo<crate::player_lethal::PlayerHealReceipt>>,
}

impl Default for ChannelSpellStates {
    fn default() -> Self {
        Self {
            owner_wake: Default::default(),
            actors: Vec::new(),
            spell_timers: Default::default(),
            pending_familiars: Default::default(),
            pending_familiar_lifecycle: Default::default(),
            pending_familiar_logouts: Default::default(),
            pending_native: Default::default(),
            pending_world_items: Default::default(),
            pending_parameters: Default::default(),
            presentations: Default::default(),
            next_item_deadline_pass_us: Default::default(),
            source_map_initialized: Default::default(),
            next_map_initialization_pass_us: Default::default(),
            next_party_deadline_pass_us: Default::default(),
            monster_melee: Default::default(),
            next_monster_ai_pass_us: Default::default(),
            monster_ai_sequence: Default::default(),
            monster_ai_cursor: Default::default(),
            deaths: Vec::new(),
            mint_death: mint_player_death_occurrence,
            source_secondary_memos: Vec::new(),
            source_damage: Vec::new(),
            source_mana: Vec::new(),
            source_heal: Vec::new(),
        }
    }
}

/// One present player's death (§4.2): the occurrence minted at the lethal write, the idempotency
/// key of every consequence, and the cell the player died on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlayerDeath {
    pub(crate) occurrence: PlayerDeathOccurrence,
    pub(crate) cell: MovementLocalPosition,
}

/// A fresh UUIDv7 `PlayerDeathOccurrence`: wall-clock milliseconds and the TLS provider's secure
/// random source, like a GameSession id.
fn mint_player_death_occurrence() -> Option<PlayerDeathOccurrence> {
    let mut bytes = [0_u8; 16];
    rustls::crypto::aws_lc_rs::default_provider()
        .secure_random
        .fill(&mut bytes)
        .ok()?;
    let millis = u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_millis(),
    )
    .ok()?;
    bytes[..6].copy_from_slice(&millis.to_be_bytes()[2..]);
    bytes[6] = 0x70 | (bytes[6] & 0x0f);
    bytes[8] = 0x80 | (bytes[8] & 0x3f);
    PlayerDeathOccurrence::from_bytes(bytes).ok()
}

impl ChannelSpellStates {
    /// Unknown durable outcomes retain their original prepared owner state.
    pub(crate) fn has_pending_spell_commit(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> bool {
        self.has_pending_native(actor, session)
            || self.has_pending_world_items(actor, session)
            || self.has_pending_parameters(actor, session)
            || self.has_pending_familiar(actor, session)
            || self.actors.iter().any(|(a, s, state)| {
                *a == actor
                    && *s == session
                    && (state.pending_stance().is_some()
                        || state.pending_training_checkpoint().is_some())
            })
    }
    /// Account evidence is only data. Resolve the actual present controlled actor
    /// and exact current player revision independently before applying its transition.
    pub(crate) fn apply_current_premium(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        facts: &crate::spell::owned_cast_facts::OwnedCastFacts,
        now_micros: u64,
    ) -> Result<bool, SpellCastDisposition> {
        if runtime.owner_fence().is_err()
            || facts.binding().actor != actor
            || facts.binding().session != session
            || runtime.player_control_facts(actor, session).is_err()
        {
            return Err(SpellCastDisposition::Rejected);
        }
        let state = self
            .get_mut(runtime, actor, session)
            .ok_or(SpellCastDisposition::Rejected)?;
        if state.revision() != facts.binding().player_revision {
            return Err(SpellCastDisposition::Rejected);
        }
        let projection = facts.current_premium(now_micros);
        state.apply_owner_premium_transition(
            projection.is_some_and(|value| value.value),
            projection
                .filter(|value| value.value)
                .map(|value| value.valid_until_micros),
        )
    }

    /// Drains the one actor-owned lane from an actual current owner cycle.
    /// Callback receives the real current player states read-only; no awaited
    /// service or detached fence may cross this uninterrupted owner turn.
    pub(crate) fn drain_spell_timers(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        now: SemanticTimeMicros,
        mut build: impl FnMut(
            &ChannelRuntimeV1,
            &ChannelSpellStates,
            &crate::spell::delayed_execution::TimerPayload,
            crate::spell::delayed_execution::SpellTimerOccurrence,
            crate::foundation::owner_timer::SemanticTimeMicros,
            crate::foundation::RuntimeWorkStamp,
        ) -> Result<
            crate::spell::combat_batch::OwnerCombatBatch,
            crate::spell::delayed_execution::Error,
        >,
    ) -> Result<crate::spell::delayed_execution::FireReport, crate::spell::delayed_execution::Error>
    {
        use crate::spell::delayed_execution::Error;
        let Some(timer) = self.spell_timers.as_ref() else {
            return Ok(Default::default());
        };
        let binding = runtime.binding();
        if timer.scope()
            != crate::foundation::RuntimeScopeRefV1::channel(
                binding.world_id(),
                binding.channel_id(),
            )
            || timer.generation() != binding.scope_generation()
        {
            return Err(Error::StaleOwner);
        }
        let stamp = runtime.issue_owner_work().map_err(|_| Error::StaleOwner)?;
        let Some(mut timer) = self.spell_timers.take() else {
            return Err(Error::StaleOwner);
        };
        struct Clock(crate::foundation::owner_timer::SemanticTimeMicros);
        impl crate::foundation::owner_timer::OwnerClock for Clock {
            fn now(&self) -> crate::foundation::owner_timer::SemanticTimeMicros {
                self.0
            }
        }
        let result = timer.fire_due_current(
            runtime,
            &Clock(crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now.get())),
            stamp,
            |runtime, payload, occurrence, due, stamp| {
                build(runtime, self, payload, occurrence, due, stamp)
            },
        );
        self.spell_timers = Some(timer);
        result
    }
    pub(crate) fn load_owned_build(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        build: &crate::durability::character_build::DurableBuildState,
    ) -> bool {
        self.get_mut(runtime, actor, session)
            .is_some_and(|state| state.load_owned_build(build).is_ok())
    }
    pub(crate) fn load_owned_stance(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        stance: &crate::durability::character_stance::DurableCharacterStance,
        book: &SpellBook,
    ) -> bool {
        self.get_mut(runtime, actor, session)
            .is_some_and(|state| state.load_owned_stance(stance, book).is_ok())
    }
    /// Create the actor's vitals at its maxima and empty cooldowns in the owner step that makes
    /// it playable, with the durable monk values `(harmony, serene_forced_micros)` the Character
    /// owner loaded (SPELL-D8 §8.2), and run the Serene initialization evaluation at `now` before
    /// any command. A present actor that already has them (a retry, a same-GameSession
    /// reconnect) keeps them exactly: nothing is refilled or reset, and only Serene is evaluated
    /// again. `None` when the actor is not the committed player of `game_session_id`, the facts
    /// exceed the SPELL-D8 bounds, or the monk values are corrupt.
    pub(crate) fn initialize(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        facts: CharacterCastFacts,
        (harmony, serene_forced_micros): (u8, u64),
        now: SemanticTimeMicros,
    ) -> Option<&PlayerSpellState> {
        runtime.player_control_facts(actor, game_session_id).ok()?;
        let binding = runtime.binding();
        if self.spell_timers.as_ref().is_some_and(|timers| {
            timers.scope()
                != crate::foundation::RuntimeScopeRefV1::channel(
                    binding.world_id(),
                    binding.channel_id(),
                )
                || timers.generation() != binding.scope_generation()
        }) {
            return None;
        }
        if self.spell_timers.is_none() {
            self.spell_timers = Some(
                crate::spell::delayed_execution::SpellTimerOwner::new(
                    crate::foundation::RuntimeScopeRefV1::channel(
                        binding.world_id(),
                        binding.channel_id(),
                    ),
                    binding.scope_generation(),
                )
                .ok()?,
            );
        }
        if self.presentations.is_none() {
            self.presentations = Some(super::spell_presentations::SpellPresentationOwner::new(
                runtime,
            ));
        }
        self.actors.retain(|(present, session, _)| {
            runtime.player_control_facts(*present, *session).is_ok()
        });
        self.prune_source_memos(runtime);
        let actors = &self.actors;
        self.deaths.retain(|(dead, session, _)| {
            actors
                .iter()
                .any(|(present, held, _)| present == dead && held == session)
        });
        let index = match self.index(actor, game_session_id) {
            Some(index) => index,
            None => {
                let mut state = PlayerSpellState::new(facts, harmony, serene_forced_micros)?;
                state.make_playable(now).ok()?;
                self.actors.try_reserve(1).ok()?;
                self.actors.push((actor, game_session_id, state));
                return self.actors.last().map(|(_, _, state)| state);
            }
        };
        let state = &mut self.actors.get_mut(index)?.2;
        state.make_playable(now).ok()?;
        Some(state)
    }

    /// FND-04B §20/§21 recovery made the present actor playable again: the Serene
    /// initialization evaluation runs before its first command (§8.2). `false` when the actor has
    /// no spell state or the evaluation failed.
    pub(crate) fn resume(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        now: SemanticTimeMicros,
    ) -> bool {
        self.get_mut(runtime, actor, game_session_id)
            .is_some_and(|state| state.make_playable(now).is_ok())
    }

    /// The session left the actor (control loss): no command is accepted until [`Self::resume`].
    pub(crate) fn detach(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
    ) {
        if let Some(state) = self.get_mut(runtime, actor, game_session_id) {
            state.detach();
        }
    }

    /// The periodic 1000 ms Serene evaluation of the actor (§8.2): the new `ACTOR_VITALS`
    /// revision and value when Serene changed, which the owner publishes.
    #[allow(
        dead_code,
        reason = "the periodic Serene owner caller is not wired on this branch"
    )]
    pub(crate) fn tick(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        now: SemanticTimeMicros,
    ) -> Option<(u64, ActorVitals)> {
        if self.is_dead(actor) {
            return None;
        }
        if self.has_pending_spell_commit(actor, game_session_id) {
            return None;
        }
        self.get_mut(runtime, actor, game_session_id)?
            .expire_owner_premium(now.get())
            .ok()?;
        let state = self.get_mut(runtime, actor, game_session_id)?;
        let cycle = crate::spell::actor_conditions::stage_owner_cycle(state, now, None).ok()??;
        if !cycle.combat_ticks.is_empty() {
            return None;
        }
        let publish = cycle.publish_vitals;
        *state = cycle.next;
        publish.then(|| (state.revision(), state.vitals()))
    }

    /// Current authored PZ metadata belongs to the active map and the real
    /// movement snapshot. Missing metadata still permits non-ticking expiry.
    /// Outside a PZ, DOT additionally needs the actual dynamic field/source
    /// legality owners; this path deliberately leaves those ticks pending.
    pub(crate) fn tick_in_environment(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        cells: &crate::content::NativeEntryMovementCells,
        actor: ExactActorRef,
        session: GameSessionId,
        now: SemanticTimeMicros,
    ) -> Option<(u64, ActorVitals)> {
        use crate::ability::condition::{TickFacts, TickKind};
        if self.has_pending_spell_commit(actor, session) {
            return None;
        }
        self.get_mut(runtime, actor, session)?
            .expire_owner_premium(now.get())
            .ok()?;
        let controls = runtime.player_control_facts(actor, session).ok()?;
        if controls.control_loss.is_some() {
            return None;
        }
        let in_pz = qualified_protection_zone(runtime, cells, actor);
        let state = self.get_mut(runtime, actor, session)?;
        let has_dot = crate::spell::actor_conditions::has_periodic_damage(state);
        let facts = in_pz.map(|in_protection_zone| TickFacts {
            in_protection_zone,
            // Dynamic field cannot suppress a non-damage tick. PZ DOT is
            // consumed without damage, independent of standing-field facts.
            standing_on_field: None,
        });
        let cycle = if has_dot && facts.is_some_and(|facts| !facts.in_protection_zone) {
            crate::spell::actor_conditions::stage_source_owner_cycle_without_damage(
                state, now, facts?, None,
            )
        } else {
            crate::spell::actor_conditions::stage_source_owner_cycle(state, now, facts, None)
        }
        .ok()??;
        if cycle.combat_ticks.iter().any(|tick| {
            !matches!(
                tick.kind,
                TickKind::Damage { refused: true, .. }
                    | TickKind::Regeneration {
                        suppressed: true,
                        ..
                    }
            )
        }) {
            return None;
        }
        let publish = cycle.publish_vitals;
        *state = cycle.next;
        publish.then(|| (state.revision(), state.vitals()))
    }

    /// The monk values the actor-end save writes (§8.2), or `None` when the actor has no spell
    /// state or is not a monk.
    pub(crate) fn monk_save_values(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        now: SemanticTimeMicros,
    ) -> Option<(u8, u64)> {
        self.get(runtime, actor, game_session_id)?
            .monk_save_values(now)
    }

    /// Commit a resolved health credit under the Channel owner's runtime lock.
    /// The occurrence owner suppresses replay before calling this method.
    /// Zero/full-pool credit writes nothing; stale bindings and exhaustion refuse.
    #[allow(
        dead_code,
        reason = "prepared native combat credit awaits the owning occurrence caller"
    )]
    pub(crate) fn apply_health_gain(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        amount: u64,
    ) -> Option<(u32, u64)> {
        if self.is_dead(actor) {
            return None;
        }
        let state = self.get(runtime, actor, game_session_id)?;
        let (next, gained) = state.after_health_gain(amount)?;
        let revision = next.revision();
        if gained == 0 {
            return Some((0, revision));
        }
        self.commit(runtime, actor, game_session_id, next)
            .then_some((gained, revision))
    }

    /// Commit a resolved mana credit under the Channel owner's runtime lock.
    /// The occurrence owner suppresses replay before calling this method.
    /// Zero/full-pool credit writes nothing; stale bindings and exhaustion refuse.
    #[allow(
        dead_code,
        reason = "prepared native combat credit awaits the owning occurrence caller"
    )]
    pub(crate) fn apply_mana_gain(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        amount: u64,
    ) -> Option<(u32, u64)> {
        if self.is_dead(actor) {
            return None;
        }
        let state = self.get(runtime, actor, game_session_id)?;
        let (next, gained) = state.after_mana_gain(amount)?;
        let revision = next.revision();
        if gained == 0 {
            return Some((0, revision));
        }
        self.commit(runtime, actor, game_session_id, next)
            .then_some((gained, revision))
    }

    /// DEATH-2 §4.2: true from the lethal write until the respawn. A dead player takes no input.
    pub(crate) fn is_dead(&self, actor: ExactActorRef) -> bool {
        self.deaths.iter().any(|(dead, _, _)| *dead == actor)
    }

    /// The death of the committed player of `game_session_id`, while it waits for its respawn.
    pub(crate) fn player_death(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
    ) -> Option<PlayerDeath> {
        runtime.player_control_facts(actor, game_session_id).ok()?;
        self.deaths
            .iter()
            .find(|(dead, session, _)| *dead == actor && *session == game_session_id)
            .map(|(_, _, death)| *death)
    }

    /// DEATH-2 §4.5 (D63): the respawn of `occurrence` in one owner step, after the caller placed
    /// the actor at its respawn position. Health and mana are refilled to their maxima under one
    /// new `ACTOR_VITALS` revision, a monk's Harmony and forced Serene are reset with its Serene
    /// evaluation initialized at `now`, and the death record ends; the new revision and value are
    /// returned for the owner to publish. `None` changes nothing: the actor is not this death's
    /// committed player.
    pub(crate) fn respawn(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        occurrence: PlayerDeathOccurrence,
        now: SemanticTimeMicros,
    ) -> Option<(u64, ActorVitals)> {
        let death = self.deaths.iter().position(|(dead, session, death)| {
            *dead == actor && *session == game_session_id && death.occurrence == occurrence
        })?;
        let next = self.get(runtime, actor, game_session_id)?.respawned(now)?;
        let vitals = (next.revision(), next.vitals());
        if !self.commit(runtime, actor, game_session_id, next) {
            return None;
        }
        self.forget_source_memos(actor, game_session_id);
        self.deaths.swap_remove(death);
        Some(vitals)
    }

    pub(in crate::gameplay_transport) fn get_mut(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
    ) -> Option<&mut PlayerSpellState> {
        runtime.assert_actor_spell_unreserved(actor).ok()?;
        runtime.player_control_facts(actor, game_session_id).ok()?;
        let index = self.index(actor, game_session_id)?;
        self.actors.get_mut(index).map(|(_, _, state)| state)
    }

    pub(in crate::gameplay_transport) fn get(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
    ) -> Option<&PlayerSpellState> {
        runtime.player_control_facts(actor, game_session_id).ok()?;
        let index = self.index(actor, game_session_id)?;
        self.actors.get(index).map(|(_, _, state)| state)
    }

    /// Test-only full owned state read; admission remains the existing exact actor/session read.
    #[cfg(test)]
    pub(crate) fn read_owned_player_state_test_snapshot(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
    ) -> Option<&PlayerSpellState> {
        self.get(runtime, actor, game_session_id)
    }

    /// Compare-commit (SPELL-D3): `next` replaces the state only as the direct successor of the
    /// current revision, so one write carries the heal and the whole anchor payment.
    fn commit(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        next: PlayerSpellState,
    ) -> bool {
        if runtime
            .player_control_facts(actor, game_session_id)
            .is_err()
        {
            return false;
        }
        let Some(index) = self.index(actor, game_session_id) else {
            return false;
        };
        let current = &mut self.actors[index].2;
        if current.revision().checked_add(1) != Some(next.revision()) {
            return false;
        }
        *current = next;
        true
    }

    fn index(&self, actor: ExactActorRef, game_session_id: GameSessionId) -> Option<usize> {
        self.actors
            .iter()
            .position(|(present, session, _)| *present == actor && *session == game_session_id)
    }
}

fn qualified_protection_zone(
    runtime: &mut ChannelRuntimeV1,
    cells: &crate::content::NativeEntryMovementCells,
    actor: ExactActorRef,
) -> Option<bool> {
    let scope = cells.scope();
    if scope.world_id != runtime.binding().world_id()
        || scope.world_id != runtime.content_pin().world_id()
        || scope.generation_digest != runtime.content_pin().server_artifact_digest()
    {
        return None;
    }
    let position = runtime.borrow_movement_position().read(actor).ok()?;
    if position.context() != runtime.pinned_movement_context() {
        return None;
    }
    let position = position.position();
    let tile = cells
        .spell_tiles()
        .lookup(
            scope,
            crate::content::LogicalCell {
                x: position.x,
                y: position.y,
                z: i32::from(position.floor),
            },
        )
        .ok()?;
    Some(tile.flags().protection_zone)
}

/// GAME-AI-01 slice §4.6/§4.7: the vitals owner's side of a creature bite. One hit is one
/// compare-committed vitals revision. DEATH-2 (§4.1, §4.2): a hit to 0 is lethal; the death
/// occurrence is minted and the death recorded with the cell in the same write, and a dead
/// player is no target. A failed mint or position read refuses the hit with nothing written.
impl ChannelSpellStates {
    fn apply_creature_damage_native(
        &mut self,
        runtime: &ChannelRuntimeV1,
        target: ExactActorRef,
        target_session: GameSessionId,
        magnitude: u32,
        now: crate::foundation::owner_timer::SemanticTimeMicros,
    ) -> Option<CreatureHit> {
        // Durable unknown outcomes retain their exact player before-state. A bite
        // must not invalidate that state, including another caster's reserved target.
        if self.is_dead(target)
            || self.has_pending_spell_commit(target, target_session)
            || runtime.assert_actor_spell_unreserved(target).is_err()
        {
            return None;
        }
        let state = self.get(runtime, target, target_session)?;
        let (next, damage) =
            crate::spell::actor_conditions::stage_creature_hit(state, magnitude, now.get()).ok()?;
        if next.revision() == state.revision() {
            return Some(CreatureHit {
                damage,
                vitals_revision: state.revision(),
                death: None,
            });
        }
        let death = if damage.health_after == 0 {
            let cell = runtime.read_actor_position(target).ok()?.position();
            self.deaths.try_reserve(1).ok()?;
            Some(PlayerDeath {
                occurrence: (self.mint_death)()?,
                cell,
            })
        } else {
            None
        };
        let vitals_revision = next.revision();
        if !self.commit(runtime, target, target_session, next) {
            return None;
        }
        if let Some(death) = death {
            self.deaths.push((target, target_session, death));
        }
        Some(CreatureHit {
            damage,
            vitals_revision,
            death: death.map(|death| *death.occurrence.as_bytes()),
        })
    }
}

impl CreatureBiteVitals for ChannelSpellStates {
    fn apply_creature_damage(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        target: ExactActorRef,
        target_session: GameSessionId,
        magnitude: u32,
        now: crate::foundation::owner_timer::SemanticTimeMicros,
    ) -> Option<CreatureHit> {
        self.apply_creature_damage_native(runtime, target, target_session, magnitude, now)
    }
}

/// `SPELL-RL-01`: cast inputs applied per actor per Channel owner work cycle. [`cast_in_channel`]
/// is that work item and takes exactly one intent; the serve loop reads no further command of the
/// actor until it returns, so later inputs stay outstanding FND-02 commands.
#[cfg(test)]
pub(crate) const SPELL_RL_01_CAST_INPUTS_PER_WORK_CYCLE: usize = 1;

/// The owner's result of one cast: its disposition and, only when it committed, the actor's new
/// `ACTOR_VITALS` revision and value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SpellCastOutcome {
    pub(crate) disposition: SpellCastDisposition,
    pub(crate) vitals: Option<(u64, ActorVitals)>,
}

impl SpellCastOutcome {
    pub(crate) const fn rejected() -> Self {
        Self {
            disposition: SpellCastDisposition::Rejected,
            vitals: None,
        }
    }
}

/// The admitted actor's current `ACTOR_VITALS` revision and value, or `None` while it has no
/// Character cast facts.
pub(crate) fn observe_vitals(
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    actor: ExactActorRef,
    game_session_id: GameSessionId,
) -> Option<(u64, ActorVitals)> {
    let state = states.get(runtime, actor, game_session_id)?;
    Some((state.revision(), state.vitals()))
}

/// One cast of `intent` by the admitted actor, as one Channel-owner work item under the runtime
/// lock the caller holds (and `states` beside it): read the actor's state, resolve, plan, and compare-commit the next state
/// with its anchor payment (SPELL-D3). `command_id` binds the occurrence and its draws, so the
/// outcome never depends on anything the client chooses beyond its intent. An actor without
/// Character cast facts, a stale binding or an unknown spell is `REJECTED` with no effect.
#[allow(clippy::too_many_arguments)]
pub(crate) fn cast_in_channel(
    runtime: &ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    book: &SpellBook,
    actor: ExactActorRef,
    game_session_id: GameSessionId,
    command_id: u64,
    intent: &SpellCastIntent,
    now: SemanticTimeMicros,
) -> SpellCastOutcome {
    if states.is_dead(actor) {
        return SpellCastOutcome::rejected();
    }
    let Some(state) = states.get(runtime, actor, game_session_id) else {
        return SpellCastOutcome::rejected();
    };
    let placement = actor.placement_identity();
    let occurrence_bytes: [u8; 16] = {
        let digest = Sha256::new()
            .chain_update(b"oteryn:spell-cast-occurrence:v1")
            .chain_update(placement)
            .chain_update(game_session_id.as_bytes())
            .chain_update(command_id.to_be_bytes())
            .finalize();
        let mut bytes = [0_u8; 16];
        bytes.copy_from_slice(&digest[..16]);
        bytes
    };
    let caster = format!("actor:{}", hex(&placement));
    let occurrence_id = format!("spell-cast:{}", hex(&occurrence_bytes));
    let pin = runtime.content_pin();
    let content = format!("content:{}", pin.activation_sequence());
    let Ok(occurrence) = RevisionSet::new(
        "ruleset:spell-v1",
        &content,
        "world:spell-v1",
        "formula:spell-p3a",
        "simulation:v1",
    )
    .and_then(|revisions| AbilityOccurrence::new(&occurrence_id, revisions)) else {
        return SpellCastOutcome::rejected();
    };
    // SIM determinism: every magnitude is drawn from the owner stream bound to this occurrence.
    let root = GameplayDecisionRoot::from_bytes(pin.server_artifact_digest());
    let decision = DecisionOccurrenceId::from_bytes(occurrence_bytes);
    let mut draw_index = 0_u64;
    let mut draw = |minimum: i64, maximum: i64| {
        let value =
            deterministic_decision_u64(&root, decision, "spell.cast.draw", draw_index).unwrap_or(0);
        draw_index = draw_index.saturating_add(1);
        crate::spell::uniform_draw(value, minimum, maximum)
    };
    let next = cast(
        book,
        state,
        intent,
        CastContext {
            caster: &caster,
            owner_scope: "channel-owner",
            occurrence,
            now,
            draw: &mut draw,
        },
    );
    match next {
        Ok(next) => {
            let vitals = (next.revision(), next.vitals());
            if states.commit(runtime, actor, game_session_id, next) {
                SpellCastOutcome {
                    disposition: SpellCastDisposition::Cast,
                    vitals: Some(vitals),
                }
            } else {
                SpellCastOutcome::rejected()
            }
        }
        Err(disposition) => SpellCastOutcome {
            disposition,
            vitals: None,
        },
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
#[allow(clippy::expect_used)]
pub(crate) mod tests {
    use std::num::NonZeroU32;

    use super::*;
    use crate::foundation::{ChannelContentPin, ChannelId, NodeId, WorldId};
    use crate::spell::Vocation;

    const fn uuid_v7(tag: u8) -> [u8; 16] {
        [
            0x01, 0x90, 0x00, 0x00, 0x00, tag, 0x70, 0x00, 0x80, 0x00, 0, 0, 0, 0, 0, tag,
        ]
    }

    /// A Channel runtime with one committed player actor bound to the session `tag`.
    pub(crate) fn runtime_with_player(tag: u8) -> (ChannelRuntimeV1, ExactActorRef, GameSessionId) {
        runtime_with_capacity(tag, 2)
    }

    // Source multi-child fixtures use the SAME existing committed-assignment owner with enough
    // physical slots; legacy two-slot tests keep their original capacity. No production grant.
    pub(crate) fn runtime_with_capacity(
        tag: u8,
        capacity: usize,
    ) -> (ChannelRuntimeV1, ExactActorRef, GameSessionId) {
        let world = WorldId::decode(&uuid_v7(0x60)).expect("world");
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&uuid_v7(0x61)).expect("channel"),
            NodeId::decode(&uuid_v7(0x62)).expect("node"),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            capacity,
            ChannelContentPin::test(world),
        )
        .expect("channel runtime");
        let session = GameSessionId::decode(&uuid_v7(tag)).expect("session");
        let reservation = runtime.reserve_fresh_session(session).expect("reserve");
        let actor = runtime.commit_fresh_session(reservation).expect("commit");
        (runtime, actor, session)
    }

    /// Level 8 druid facts, as the Character owner would supply them.
    pub(crate) const FACTS: CharacterCastFacts = CharacterCastFacts {
        vocation: Vocation::Druid,
        level: 8,
        magic_level: 0,
        max_health: 185,
        max_mana: 90,
        max_soul: 100,
    };

    /// Test only: no damage owner exists yet, so a wounded actor is staged by writing its health.
    pub(crate) fn wound(
        states: &mut ChannelSpellStates,
        actor: ExactActorRef,
        session: GameSessionId,
        health: u32,
    ) {
        let index = states.index(actor, session).expect("present");
        states.actors[index].2.set_health_for_test(health);
    }

    /// `exura` (index 3 of the canonical V1 book) with no target.
    pub(crate) fn exura() -> SpellCastIntent {
        SpellCastIntent {
            spell: NonZeroU32::new(3).expect("index"),
            target: SpellTarget::None,
            aim_at_target: false,
        }
    }

    fn now(millis: u64) -> SemanticTimeMicros {
        SemanticTimeMicros::from_micros(millis * 1000)
    }

    fn cast_at(
        runtime: &ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        actor: ExactActorRef,
        session: GameSessionId,
        command_id: u64,
        millis: u64,
    ) -> SpellCastOutcome {
        let book = crate::spell::cast::v1_spell_book().expect("book");
        cast_in_channel(
            runtime,
            states,
            &book,
            actor,
            session,
            command_id,
            &exura(),
            now(millis),
        )
    }

    #[test]
    fn health_credit_commits_native_vitals_and_preserves_noops() {
        let (runtime, actor, session) = runtime_with_player(0x73);
        let mut states = ChannelSpellStates::default();
        states
            .initialize(&runtime, actor, session, FACTS, (0, 0), now(0))
            .expect("initialized");
        wound(&mut states, actor, session, 100);
        assert_eq!(
            states.apply_health_gain(&runtime, actor, session, 0),
            Some((0, 1))
        );
        assert_eq!(
            states.apply_health_gain(&runtime, actor, session, 7),
            Some((7, 2))
        );
        let (revision, vitals) =
            observe_vitals(&runtime, &states, actor, session).expect("observed");
        assert_eq!(
            (revision, vitals.health, vitals.mana, vitals.soul),
            (2, 107, 90, 100)
        );
        assert_eq!(
            states.apply_health_gain(&runtime, actor, session, u64::MAX),
            Some((78, 3))
        );
        let before = states
            .get(&runtime, actor, session)
            .expect("present")
            .clone();
        assert_eq!(
            states.apply_health_gain(&runtime, actor, session, 1),
            Some((0, 3))
        );
        assert_eq!(states.get(&runtime, actor, session), Some(&before));
    }

    #[test]
    fn mana_credit_commits_native_vitals_and_preserves_noops() {
        let (runtime, actor, session) = runtime_with_player(0x74);
        let mut states = ChannelSpellStates::default();
        states
            .initialize(&runtime, actor, session, FACTS, (0, 0), now(0))
            .expect("initialized");
        assert_eq!(
            cast_at(&runtime, &mut states, actor, session, 1, 0).disposition,
            SpellCastDisposition::Cast
        );
        assert_eq!(
            states.apply_mana_gain(&runtime, actor, session, 0),
            Some((0, 2))
        );
        assert_eq!(
            states.apply_mana_gain(&runtime, actor, session, 7),
            Some((7, 3))
        );
        let (revision, vitals) =
            observe_vitals(&runtime, &states, actor, session).expect("observed");
        assert_eq!(
            (revision, vitals.mana, vitals.health, vitals.soul),
            (3, 77, 185, 100)
        );
        assert_eq!(
            states.apply_mana_gain(&runtime, actor, session, u64::MAX),
            Some((13, 4))
        );
        let before = states
            .get(&runtime, actor, session)
            .expect("present")
            .clone();
        assert_eq!(
            states.apply_mana_gain(&runtime, actor, session, 1),
            Some((0, 4))
        );
        assert_eq!(states.get(&runtime, actor, session), Some(&before));
    }

    #[test]
    fn vitals_credit_refuses_foreign_sessions_and_ended_generations() {
        let (mut runtime, actor, session) = runtime_with_player(0x75);
        let other = GameSessionId::decode(&uuid_v7(0x76)).expect("other session");
        let mut states = ChannelSpellStates::default();
        assert!(
            states
                .apply_health_gain(&runtime, actor, session, 7)
                .is_none()
        );
        assert!(
            states
                .apply_mana_gain(&runtime, actor, session, 7)
                .is_none()
        );
        states
            .initialize(&runtime, actor, session, FACTS, (0, 0), now(0))
            .expect("initialized");
        assert_eq!(
            cast_at(&runtime, &mut states, actor, session, 1, 0).disposition,
            SpellCastDisposition::Cast
        );
        wound(&mut states, actor, session, 100);
        let before = states.actors.clone();
        assert!(
            states
                .apply_health_gain(&runtime, actor, other, 7)
                .is_none()
        );
        assert!(states.apply_mana_gain(&runtime, actor, other, 7).is_none());
        assert_eq!(states.actors, before);

        runtime
            .remove_terminal_session(session, actor)
            .expect("ended");
        assert!(
            states
                .apply_health_gain(&runtime, actor, session, 7)
                .is_none()
        );
        assert!(
            states
                .apply_mana_gain(&runtime, actor, session, 7)
                .is_none()
        );
        assert_eq!(states.actors, before);

        let reservation = runtime.reserve_fresh_session(other).expect("reserve");
        let successor = runtime
            .commit_fresh_session(reservation)
            .expect("successor");
        assert_ne!(actor, successor);
        states
            .initialize(&runtime, successor, other, FACTS, (0, 0), now(0))
            .expect("fresh state");
        assert!(
            states
                .apply_health_gain(&runtime, actor, session, 7)
                .is_none()
        );
        assert!(
            states
                .apply_mana_gain(&runtime, actor, session, 7)
                .is_none()
        );
        assert_eq!(
            states.apply_health_gain(&runtime, successor, other, 7),
            Some((0, 1))
        );
        assert_eq!(
            states.apply_mana_gain(&runtime, successor, other, 7),
            Some((0, 1))
        );
    }

    #[test]
    fn spell_rl_01_matches_the_registered_row() {
        let registry: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../docs/contracts/RESOURCE_LIMITS_REGISTRY.json"
        ))
        .expect("registry");
        let row = registry["entries"]
            .as_array()
            .and_then(|entries| entries.iter().find(|entry| entry["id"] == "SPELL-RL-01"))
            .and_then(|entry| entry["hard_maximum"].as_u64());
        assert_eq!(
            row,
            u64::try_from(SPELL_RL_01_CAST_INPUTS_PER_WORK_CYCLE).ok()
        );
    }

    #[test]
    fn a_cast_commits_once_and_a_repeated_proposal_never_pays_twice() {
        let (runtime, actor, session) = runtime_with_player(0x63);
        let mut states = ChannelSpellStates::default();
        states
            .initialize(&runtime, actor, session, FACTS, (0, 0), now(0))
            .expect("initialized");
        let full = ActorVitals {
            health: 185,
            max_health: 185,
            mana: 90,
            max_mana: 90,
            soul: 100,
            harmony: 0,
            serene: false,
        };
        assert_eq!(
            observe_vitals(&runtime, &states, actor, session),
            Some((1, full))
        );
        let cast = cast_at(&runtime, &mut states, actor, session, 1, 0);
        assert_eq!(cast.disposition, SpellCastDisposition::Cast);
        let (revision, vitals) = cast.vitals.expect("committed vitals");
        assert_eq!((revision, vitals.mana, vitals.health), (2, 70, 185));
        // A second input while cooling down is answered and pays nothing.
        assert_eq!(
            cast_at(&runtime, &mut states, actor, session, 2, 500),
            SpellCastOutcome {
                disposition: SpellCastDisposition::CoolingDown,
                vitals: None,
            }
        );
        assert_eq!(
            observe_vitals(&runtime, &states, actor, session),
            Some((2, vitals))
        );
        // The committed state proposed again is not a successor: compare-commit refuses it.
        let committed = states.get(&runtime, actor, session).expect("state").clone();
        assert!(!states.commit(&runtime, actor, session, committed));
        assert_eq!(
            observe_vitals(&runtime, &states, actor, session),
            Some((2, vitals))
        );
    }

    #[test]
    fn reinitializing_a_present_actor_keeps_its_vitals_and_cooldowns() {
        let (runtime, actor, session) = runtime_with_player(0x64);
        let mut states = ChannelSpellStates::default();
        states
            .initialize(&runtime, actor, session, FACTS, (0, 0), now(0))
            .expect("initialized");
        cast_at(&runtime, &mut states, actor, session, 1, 0);
        // A same-GameSession reconnect initializes again: no refill, no cooldown reset.
        let kept = states
            .initialize(&runtime, actor, session, FACTS, (0, 0), now(0))
            .expect("kept")
            .clone();
        assert_eq!((kept.revision(), kept.vitals().mana), (2, 70));
        assert_eq!(
            cast_at(&runtime, &mut states, actor, session, 2, 1),
            SpellCastOutcome {
                disposition: SpellCastDisposition::CoolingDown,
                vitals: None,
            }
        );
    }

    #[test]
    fn a_foreign_session_or_an_ended_actor_is_rejected_and_its_state_dropped() {
        let (mut runtime, actor, session) = runtime_with_player(0x65);
        let other = GameSessionId::decode(&uuid_v7(0x66)).expect("session");
        let mut states = ChannelSpellStates::default();
        assert!(
            states
                .initialize(&runtime, actor, other, FACTS, (0, 0), now(0))
                .is_none()
        );
        states
            .initialize(&runtime, actor, session, FACTS, (0, 0), now(0))
            .expect("initialized");
        assert_eq!(
            cast_at(&runtime, &mut states, actor, other, 1, 0),
            SpellCastOutcome::rejected()
        );
        assert_eq!(observe_vitals(&runtime, &states, actor, other), None);
        // The actor ends: its state is unreachable and the next initialization drops it.
        runtime
            .remove_terminal_session(session, actor)
            .expect("removed");
        assert_eq!(observe_vitals(&runtime, &states, actor, session), None);
        assert_eq!(
            cast_at(&runtime, &mut states, actor, session, 2, 0),
            SpellCastOutcome::rejected()
        );
        let reservation = runtime.reserve_fresh_session(other).expect("reserve");
        let successor = runtime.commit_fresh_session(reservation).expect("commit");
        let fresh = states
            .initialize(&runtime, successor, other, FACTS, (0, 0), now(0))
            .expect("fresh actor")
            .clone();
        assert_eq!((fresh.revision(), fresh.vitals().mana), (1, 90));
        assert_eq!(states.actors.len(), 1);
    }

    #[test]
    fn without_character_cast_facts_casting_stays_gated() {
        let (runtime, actor, session) = runtime_with_player(0x67);
        let mut states = ChannelSpellStates::default();
        assert_eq!(observe_vitals(&runtime, &states, actor, session), None);
        assert_eq!(
            cast_at(&runtime, &mut states, actor, session, 1, 0),
            SpellCastOutcome::rejected()
        );
    }

    #[test]
    fn the_occurrence_and_its_draws_are_bound_to_the_command() {
        let outcome = |command_id| {
            let (runtime, actor, session) = runtime_with_player(0x68);
            let mut states = ChannelSpellStates::default();
            states
                .initialize(&runtime, actor, session, FACTS, (0, 0), now(0))
                .expect("state");
            cast_at(&runtime, &mut states, actor, session, command_id, 0)
        };
        assert_eq!(outcome(7), outcome(7));
    }

    /// SPELL-D8 §8.2 in the Channel owner: a new monk actor loads the durable values and is
    /// Serene from its initialization evaluation; a reconnect keeps Harmony and the forced time,
    /// and the actor-end save reads the live values.
    #[test]
    fn a_monk_actor_loads_its_durable_values_and_keeps_them_across_a_reconnect() {
        let (runtime, actor, session) = runtime_with_player(0x69);
        let monk = CharacterCastFacts {
            vocation: Vocation::Monk,
            ..FACTS
        };
        let mut states = ChannelSpellStates::default();
        assert!(
            states
                .initialize(&runtime, actor, session, monk, (6, 0), now(0))
                .is_none(),
            "a corrupt durable Harmony fails the actor closed"
        );
        let state = states
            .initialize(&runtime, actor, session, monk, (3, 4_000_000), now(1000))
            .expect("monk actor");
        assert_eq!((state.vitals().harmony, state.vitals().serene), (3, true));
        assert_eq!(
            states.monk_save_values(&runtime, actor, session, now(2000)),
            Some((3, 3_000_000))
        );
        // A solo monk's periodic evaluation changes nothing, so nothing is published.
        assert_eq!(states.tick(&runtime, actor, session, now(2000)), None);
        // Control loss detaches the actor: its casts are refused until a recovery.
        states.detach(&runtime, actor, session);
        assert_eq!(
            cast_at(&runtime, &mut states, actor, session, 1, 2500),
            SpellCastOutcome::rejected()
        );
        assert!(states.resume(&runtime, actor, session, now(3000)));
        // A present actor keeps its values: a second initialization reloads nothing.
        let kept = states
            .initialize(&runtime, actor, session, monk, (0, 0), now(3000))
            .expect("kept");
        assert_eq!(kept.vitals().harmony, 3);
        assert_eq!(
            states.monk_save_values(&runtime, actor, session, now(3000)),
            Some((3, 2_000_000))
        );
        assert_eq!(
            cast_at(&runtime, &mut states, actor, session, 2, 3000).disposition,
            SpellCastDisposition::Cast
        );
        // A druid has no monk values to save.
        let (druid_runtime, druid, druid_session) = runtime_with_player(0x6a);
        let mut druids = ChannelSpellStates::default();
        druids
            .initialize(&druid_runtime, druid, druid_session, FACTS, (0, 0), now(0))
            .expect("druid");
        assert_eq!(
            druids.monk_save_values(&druid_runtime, druid, druid_session, now(0)),
            None
        );
    }

    #[test]
    fn creature_damage_preserves_actual_logical_stance_reservation() {
        use crate::domain::{CharacterId, CharacterRevision};
        use crate::durability::character_progression::CurrentCharacterGameplayFence;
        use crate::foundation::{ConnectionGeneration, RuntimeScopeRefV1};
        use crate::spell::stance_execution::PreparedStance;
        let (mut runtime, actor, session) = runtime_with_player(0x35);
        let mut states = ChannelSpellStates::default();
        states
            .initialize(
                &runtime,
                actor,
                session,
                CharacterCastFacts {
                    vocation: Vocation::EliteKnight,
                    level: 50,
                    magic_level: 0,
                    max_health: 500,
                    max_mana: 600,
                    max_soul: 100,
                },
                (0, 0),
                now(0),
            )
            .expect("state");
        let document: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tools/content-schema/spell-authoring/samples/native-spell-profiles.json"
        ))
        .expect("profiles");
        let profile = document["profiles"]
            .as_array()
            .expect("profiles")
            .iter()
            .find(|r| r["name"] == "Protector")
            .expect("Protector");
        let mut spell = crate::spell::authoring::spell_from_bundle(
            &serde_json::json!({"spell":profile["spell"]}),
            &profile["dependencies"],
        )
        .expect("source stance");
        // Header-only unit fixture: no commercial proof or durable receipt is fabricated.
        spell.premium = false;
        let state = states.get_mut(&runtime, actor, session).expect("state");
        let operational = crate::spell::OperationalCastFacts {
            caster_position: crate::spell::chain::TilePosition {
                x: 0,
                y: 0,
                floor: 7,
            },
            target_position: None,
            target: None,
            line_of_sight_clear: None,
            direction_available: false,
            wheel_unlocked: None,
            in_protection_zone: false,
            target_tile_solid: None,
            target_tile_creature: None,
        };
        let paid = crate::spell::cast::prepare_native_owner_cast(
            state,
            &spell,
            &operational,
            crate::spell::native::Facts::Stance {
                active: None,
                vocation: Vocation::EliteKnight,
            },
            now(0),
            &mut |_, _| 0,
        )
        .expect("prepare real stance plan");
        let binding = runtime.binding();
        let prepared = PreparedStance::new(
            state,
            paid,
            &spell,
            actor,
            session,
            1,
            NonZeroU32::new(1).expect("spell"),
            CurrentCharacterGameplayFence {
                character_id: CharacterId::from_bytes(uuid_v7(0x36)).expect("character"),
                game_session_id: session,
                connection_generation: ConnectionGeneration::new(1).expect("connection"),
                character_lease_generation: 1,
                runtime_scope: RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id()),
                scope_ownership_generation: binding.scope_generation(),
                expected_character_revision: CharacterRevision::new(1).expect("revision"),
            },
            "content:1".into(),
            "policy:1".into(),
            now(0),
        )
        .expect("retained stance");
        state
            .reserve_stance(prepared.clone())
            .expect("actual logical reservation");
        assert!(!runtime.actor_spell_reserved(actor));
        assert!(states.has_pending_spell_commit(actor, session));
        let before = states
            .get(&runtime, actor, session)
            .expect("before")
            .clone();
        assert_eq!(
            states.apply_creature_damage(
                &mut runtime,
                actor,
                session,
                8,
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0),
            ),
            None
        );
        assert_eq!(states.get(&runtime, actor, session), Some(&before));
        assert!(
            states
                .get_mut(&runtime, actor, session)
                .expect("state")
                .cancel_stance(&prepared)
        );
        let hit = states
            .apply_creature_damage(
                &mut runtime,
                actor,
                session,
                8,
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0),
            )
            .expect("unreserved positive control");
        assert_eq!(
            (
                hit.damage.applied,
                hit.damage.health_after,
                hit.vitals_revision,
                hit.death
            ),
            (8, 492, 2, None)
        );
    }

    #[test]
    fn creature_damage_preserves_both_caster_and_target_physical_reservations() {
        use crate::foundation::runtime_actor_spell_types::{
            OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellAnchor,
            SpellOccurrenceBinding,
        };
        use crate::foundation::{CharacterId, CommandId, CommandRef, MovementLocalPosition};

        let (mut runtime, caster, caster_session) = runtime_with_player(0x32);
        let target_session = GameSessionId::decode(&uuid_v7(0x33)).expect("target session");
        let target = runtime
            .reserve_fresh_session(target_session)
            .expect("reserve target");
        let target = runtime.commit_fresh_session(target).expect("commit target");
        for (actor, x) in [(caster, 10), (target, 11)] {
            runtime
                .initialize_movement_test_position(
                    actor,
                    MovementLocalPosition { x, y: 10, floor: 7 },
                )
                .expect("position");
        }
        let mut states = ChannelSpellStates::default();
        for (actor, session) in [(caster, caster_session), (target, target_session)] {
            states
                .initialize(&runtime, actor, session, FACTS, (0, 0), now(0))
                .expect("vitals");
        }
        let batch = OwnerCombatBatch {
            caster,
            attacker: CharacterId::decode(&uuid_v7(0x34)).expect("character"),
            current_lease_generation: 1,
            command: CommandRef::new(caster_session, CommandId::new(1).expect("command")),
            occurrence: SpellOccurrenceBinding {
                id: "spell:retained".into(),
                revisions: ["rules:1", "content:1", "world:1", "formula:1", "sim:1"]
                    .map(str::to_owned),
            },
            binding: b"retained-owner-reservation".to_vec(),
            anchor: Some(SpellAnchor {
                expected_revision: 1,
                next_revision: 2,
                paid_mana: 0,
                paid_soul: 0,
                cooldown_deadlines: vec![],
            }),
            now_ms: 0,
            effects: vec![OwnerCombatEffect {
                target,
                sub_ordinal: 0,
                change: OwnerCombatChange::ManaShield(
                    crate::foundation::runtime_actor_spell_types::ManaShieldState {
                        capacity: 50,
                        expires_ms: 1_000,
                    },
                ),
            }],
            deferred: None,
        };
        let mut staged = runtime.stage_spell_batch(&batch).expect("stage real batch");
        runtime
            .reserve_spell_batch(&mut staged)
            .expect("reserve actual slots");
        for (actor, session) in [(caster, caster_session), (target, target_session)] {
            assert!(runtime.actor_spell_reserved(actor));
            // The target has no own pending intent: another caster's reservation
            // independently prevents a hit from invalidating its exact before-state.
            assert!(!states.has_pending_spell_commit(actor, session));
            let before = states.get(&runtime, actor, session).expect("state").clone();
            assert_eq!(
                states.apply_creature_damage(
                    &mut runtime,
                    actor,
                    session,
                    8,
                    crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0),
                ),
                None
            );
            assert_eq!(states.get(&runtime, actor, session), Some(&before));
        }
        runtime
            .validate_staged_spell_batch(&staged)
            .expect("retained physical proof unchanged");
    }

    /// A runtime with one positioned player at (10, 10), its vitals at a wounded `health`, and one
    /// live creature beside it.
    fn bitten_player(
        tag: u8,
        health: u32,
    ) -> (
        ChannelRuntimeV1,
        ChannelSpellStates,
        ExactActorRef,
        GameSessionId,
        ExactActorRef,
    ) {
        bitten_actor(tag, health, FACTS, (0, 0))
    }

    /// [`bitten_player`] with the actor's cast facts and durable monk values.
    fn bitten_actor(
        tag: u8,
        health: u32,
        facts: CharacterCastFacts,
        monk: (u8, u64),
    ) -> (
        ChannelRuntimeV1,
        ChannelSpellStates,
        ExactActorRef,
        GameSessionId,
        ExactActorRef,
    ) {
        use crate::foundation::MovementLocalPosition;
        let (mut runtime, actor, session) = runtime_with_player(tag);
        let at = |x, y| MovementLocalPosition { x, y, floor: 7 };
        runtime
            .initialize_movement_test_position(actor, at(10, 10))
            .expect("player position");
        let creature = runtime.admit_test_creature(at(11, 10)).expect("creature");
        let mut states = ChannelSpellStates::default();
        states
            .initialize(&runtime, actor, session, facts, monk, now(0))
            .expect("vitals");
        wound(&mut states, actor, session, health);
        (runtime, states, actor, session, creature)
    }

    fn bite_at(
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        ledger: &mut crate::ability::creature_bite::CreatureBiteLedger,
        (creature, actor, session): (ExactActorRef, ExactActorRef, GameSessionId),
        sequence: u64,
        micros: u64,
    ) -> Result<
        crate::ability::creature_bite::AppliedBite,
        crate::ability::creature_bite::BiteRejection,
    > {
        use crate::ability::AiAbilityAdapter;
        use crate::ability::creature_bite::{
            CreatureBiteDefinition, ReentryProtection, commit_ai_bite,
        };
        use crate::foundation::owner_timer::SemanticTimeMicros as OwnerTime;
        commit_ai_bite(
            ledger,
            runtime,
            states,
            AiAbilityAdapter::bite(creature, sequence, actor, session),
            CreatureBiteDefinition::new(2_000_000, 8).expect("definition"),
            RevisionSet::new(
                "ruleset:ai-v1",
                "content:1",
                "world:ai-v1",
                "formula:bite-v1",
                "simulation:v1",
            )
            .expect("revisions"),
            ReentryProtection {
                protected_until: None,
            },
            OwnerTime::from_micros(micros),
        )
    }

    /// DEATH-2 §4.1/§4.2 on the real vitals owner: a creature bite to 0 is lethal, the death
    /// occurrence is minted and the death cell recorded in the same vitals revision, and the dead
    /// player takes no further bite, cast, credit or Serene change.
    #[test]
    fn a_lethal_creature_bite_records_the_death_and_the_dead_player_takes_nothing() {
        use crate::ability::creature_bite::{BiteRejection, CreatureBiteLedger, CreatureDamage};
        use crate::foundation::MovementLocalPosition;

        let (mut runtime, mut states, actor, session, creature) = bitten_player(0x31, 5);
        let mut ledger = CreatureBiteLedger::default();
        let actors = (creature, actor, session);
        let first = bite_at(&mut runtime, &mut states, &mut ledger, actors, 0, 0).expect("bite");
        assert_eq!(
            first.damage,
            CreatureDamage {
                applied: 5,
                health_after: 0
            }
        );
        assert_eq!(first.vitals_revision, 2);
        let death = states
            .player_death(&runtime, actor, session)
            .expect("recorded death");
        assert_eq!(first.death, Some(*death.occurrence.as_bytes()));
        assert_eq!(
            death.cell,
            MovementLocalPosition {
                x: 10,
                y: 10,
                floor: 7
            }
        );
        assert!(states.is_dead(actor));
        // The lethal think occurrence replays its first result; a later bite has no target.
        assert_eq!(
            bite_at(&mut runtime, &mut states, &mut ledger, actors, 0, 0),
            Ok(first)
        );
        assert_eq!(
            bite_at(&mut runtime, &mut states, &mut ledger, actors, 1, 2_000_000),
            Err(BiteRejection::StaleTarget)
        );
        assert_eq!(
            cast_at(&runtime, &mut states, actor, session, 7, 3000),
            SpellCastOutcome::rejected()
        );
        assert_eq!(states.apply_health_gain(&runtime, actor, session, 50), None);
        assert_eq!(states.apply_mana_gain(&runtime, actor, session, 5), None);
        assert_eq!(states.tick(&runtime, actor, session, now(4000)), None);
        let (revision, vitals) = observe_vitals(&runtime, &states, actor, session).expect("vitals");
        assert_eq!((revision, vitals.health), (2, 0));
    }

    /// DEATH-2 §4.5 (D63): the respawn refills health and mana under exactly one new vitals
    /// revision and ends the death; only the recorded occurrence of the exact session respawns.
    #[test]
    fn the_respawn_refills_health_and_mana_in_one_revision_and_ends_the_death() {
        use crate::ability::creature_bite::CreatureBiteLedger;

        let (mut runtime, mut states, actor, session, creature) = bitten_player(0x32, 8);
        let mut ledger = CreatureBiteLedger::default();
        // A heal spends mana, so the respawn has both pools to refill.
        let healed = cast_at(&runtime, &mut states, actor, session, 1, 0);
        assert_eq!(healed.disposition, SpellCastDisposition::Cast);
        let spent = healed.vitals.expect("vitals").1;
        assert!(spent.mana < FACTS.max_mana);
        wound(&mut states, actor, session, 8);
        bite_at(
            &mut runtime,
            &mut states,
            &mut ledger,
            (creature, actor, session),
            0,
            0,
        )
        .expect("lethal bite");
        let death = states
            .player_death(&runtime, actor, session)
            .expect("death");
        let other = PlayerDeathOccurrence::from_bytes(uuid_v7(0x77)).expect("occurrence");
        assert_eq!(
            states.respawn(&runtime, actor, session, other, now(0)),
            None
        );
        let (_, other_actor, other_session) = runtime_with_player(0x33);
        assert_eq!(
            states.respawn(
                &runtime,
                other_actor,
                other_session,
                death.occurrence,
                now(0)
            ),
            None
        );
        let (revision, vitals) = states
            .respawn(&runtime, actor, session, death.occurrence, now(0))
            .expect("respawned");
        assert_eq!(revision, 4);
        assert_eq!(
            (vitals.health, vitals.mana),
            (FACTS.max_health, FACTS.max_mana)
        );
        assert!(!states.is_dead(actor));
        assert_eq!(states.player_death(&runtime, actor, session), None);
        // The respawn happens once; the living actor takes commands again.
        assert_eq!(
            states.respawn(&runtime, actor, session, death.occurrence, now(0)),
            None
        );
        assert_eq!(
            cast_at(&runtime, &mut states, actor, session, 8, 10_000).disposition,
            SpellCastDisposition::Cast
        );
    }

    /// DEATH-2 §4.5 with SPELL-D8 §8.2: a monk respawns with Harmony and forced Serene reset, as
    /// the death commit writes them, and its Serene evaluation initialized again, so a monk that
    /// lost control while dead is playable after the respawn.
    #[test]
    fn a_monk_respawns_with_harmony_and_forced_serene_reset_and_initialized() {
        use crate::ability::creature_bite::CreatureBiteLedger;

        let monk = CharacterCastFacts {
            vocation: Vocation::Monk,
            ..FACTS
        };
        let (mut runtime, mut states, actor, session, creature) =
            bitten_actor(0x35, 8, monk, (3, 4_000_000));
        bite_at(
            &mut runtime,
            &mut states,
            &mut CreatureBiteLedger::default(),
            (creature, actor, session),
            0,
            0,
        )
        .expect("lethal bite");
        let death = states
            .player_death(&runtime, actor, session)
            .expect("death");
        assert_eq!(
            states.monk_save_values(&runtime, actor, session, now(1000)),
            Some((3, 3_000_000))
        );
        // Control loss detaches the dead monk; only the respawn initializes it again.
        states.detach(&runtime, actor, session);
        let (_, vitals) = states
            .respawn(&runtime, actor, session, death.occurrence, now(2000))
            .expect("respawned");
        assert_eq!((vitals.harmony, vitals.serene), (0, true));
        assert_eq!(
            states.monk_save_values(&runtime, actor, session, now(2000)),
            Some((0, 0))
        );
        assert_eq!(
            cast_at(&runtime, &mut states, actor, session, 1, 2000).disposition,
            SpellCastDisposition::Cast
        );
    }

    /// A lethal hit whose death occurrence cannot be minted is refused with nothing written.
    #[test]
    fn a_lethal_hit_without_an_occurrence_writes_nothing() {
        use crate::ability::creature_bite::{BiteRejection, CreatureBiteLedger};

        let (mut runtime, mut states, actor, session, creature) = bitten_player(0x34, 8);
        states.mint_death = || None;
        let mut ledger = CreatureBiteLedger::default();
        assert_eq!(
            bite_at(
                &mut runtime,
                &mut states,
                &mut ledger,
                (creature, actor, session),
                0,
                0
            ),
            Err(BiteRejection::StaleTarget)
        );
        assert!(!states.is_dead(actor));
        let (revision, vitals) = observe_vitals(&runtime, &states, actor, session).expect("vitals");
        assert_eq!((revision, vitals.health), (1, 8));
    }
}
// Reconciled source consumers: the existing ChannelSpellStates remains the only vitals/death owner.
#[derive(Debug)]
struct NativeSourceMemo<T> {
    actor: ExactActorRef,
    session: GameSessionId,
    occurrence: String,
    magnitude: u32,
    receipt: T,
}
impl ChannelSpellStates {
    fn prune_source_memos(&mut self, runtime: &ChannelRuntimeV1) {
        self.source_damage
            .retain(|m| runtime.player_control_facts(m.actor, m.session).is_ok());
        self.source_mana
            .retain(|m| runtime.player_control_facts(m.actor, m.session).is_ok());
        self.source_heal
            .retain(|m| runtime.player_control_facts(m.actor, m.session).is_ok());
    }
    fn forget_source_memos(&mut self, actor: ExactActorRef, session: GameSessionId) {
        self.source_secondary_memos
            .retain(|m| m.actor != actor || m.session != session);
        self.source_damage
            .retain(|m| m.actor != actor || m.session != session);
        self.source_mana
            .retain(|m| m.actor != actor || m.session != session);
        self.source_heal
            .retain(|m| m.actor != actor || m.session != session);
    }
    fn source_occurrence_text(occurrence: &str) -> Option<String> {
        if occurrence.is_empty() || occurrence.len() > 4096 {
            return None;
        }
        let mut text = String::new();
        text.try_reserve(occurrence.len()).ok()?;
        text.push_str(occurrence);
        Some(text)
    }
}
impl crate::player_lethal::PlayerLethalVitals for ChannelSpellStates {
    fn apply_source_appearance_batch(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        fence: &crate::foundation::ScopeRuntimeFence,
        stamp: crate::foundation::RuntimeWorkStamp,
        source: ExactActorRef,
        definitions: &[crate::foundation::ConditionDefinition],
        targets: &[crate::player_lethal::SourceAppearanceTarget<'_>],
    ) -> Option<Vec<crate::player_lethal::SourceAppearanceReceipt>> {
        self.native_source_appearance_batch(runtime, fence, stamp, source, definitions, targets)
    }

    fn apply_attack_conditions(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
        occurrence: &str,
        source: ExactActorRef,
        definitions: &[crate::foundation::ConditionDefinition],
        facts: &crate::foundation::ApplicationFacts<'_>,
        immunities: &[crate::foundation::ConditionType],
        fence: &crate::foundation::ScopeRuntimeFence,
        stamp: crate::foundation::RuntimeWorkStamp,
    ) -> bool {
        self.native_composite_conditions(
            runtime,
            target,
            session,
            occurrence,
            source,
            definitions,
            facts,
            immunities,
            None,
            fence,
            stamp,
        )
        .is_some()
    }
    fn apply_composite_attack_damage(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
        magnitude: u32,
        occurrence: &str,
        source: ExactActorRef,
        definitions: &[crate::foundation::ConditionDefinition],
        facts: &crate::foundation::ApplicationFacts<'_>,
        immunities: &[crate::foundation::ConditionType],
        fence: &crate::foundation::ScopeRuntimeFence,
        stamp: crate::foundation::RuntimeWorkStamp,
    ) -> Option<crate::player_lethal::PlayerDamageReceipt> {
        let binding = runtime.binding();
        if !fence.is_current_for_scope(
            crate::foundation::RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id()),
            binding.scope_generation(),
        ) || !fence.accepts_stamp(stamp)
        {
            return None;
        }

        if definitions.is_empty() {
            return self.apply_attack_damage(
                runtime,
                target,
                session,
                magnitude,
                occurrence,
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(facts.now),
            );
        }
        self.native_composite_conditions(
            runtime,
            target,
            session,
            occurrence,
            source,
            definitions,
            facts,
            immunities,
            Some(magnitude),
            fence,
            stamp,
        )?
    }
    fn remove_attack_invisibility(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
        source: ExactActorRef,
        now: u64,
        fence: &crate::foundation::ScopeRuntimeFence,
        stamp: crate::foundation::RuntimeWorkStamp,
    ) -> bool {
        self.native_remove_attack_invisibility(runtime, target, session, source, now, fence, stamp)
    }

    fn source_player_health(
        &self,
        runtime: &ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
    ) -> Option<(u32, u32, u64)> {
        if self.is_dead(target)
            || runtime
                .player_control_facts(target, session)
                .ok()?
                .control_loss
                .is_some()
        {
            return None;
        }
        let state = self.get(runtime, target, session)?;
        let v = state.vitals();
        (v.health > 0 && v.health <= v.max_health).then_some((
            v.health,
            v.max_health,
            state.revision(),
        ))
    }
    fn apply_attack_damage(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
        magnitude: u32,
        occurrence: &str,
        now: crate::foundation::owner_timer::SemanticTimeMicros,
    ) -> Option<crate::player_lethal::PlayerDamageReceipt> {
        use crate::player_lethal::{PlayerDamageReceipt, PlayerLethalReceipt};
        if magnitude == 0
            || runtime
                .player_control_facts(target, session)
                .ok()?
                .control_loss
                .is_some()
        {
            return None;
        }
        if self
            .source_secondary_memos
            .iter()
            .any(|m| m.actor == target && m.session == session && m.occurrence == occurrence)
            && !self
                .source_damage
                .iter()
                .any(|m| m.actor == target && m.session == session && m.occurrence == occurrence)
        {
            return None;
        }
        let text = Self::source_occurrence_text(occurrence)?;
        self.prune_source_memos(runtime);
        let old = self
            .source_damage
            .iter()
            .position(|m| m.actor == target && m.session == session);
        if let Some(i) = old {
            let m = &self.source_damage[i];
            if m.occurrence == occurrence {
                return (m.magnitude == magnitude).then_some(m.receipt);
            }
        }
        if self.is_dead(target) {
            return None;
        }
        // Position/allocation failures precede native HP. Existing native owner rechecks position,
        // reserves and mints its one PlayerDeathOccurrence before its own compare-commit.
        let position = runtime.read_actor_position(target).ok()?.position();
        if old.is_none() {
            self.source_damage.try_reserve(1).ok()?;
        }
        let hit = CreatureBiteVitals::apply_creature_damage(
            self, runtime, target, session, magnitude, now,
        )?;
        let death = hit.death.map(|bytes| {
            PlayerLethalReceipt::from_native_commit(
                runtime,
                target,
                session,
                hit.vitals_revision,
                bytes,
                position,
            )
        });
        let receipt = PlayerDamageReceipt {
            applied: hit.damage.applied,
            health_after: hit.damage.health_after,
            vitals_revision: hit.vitals_revision,
            death,
        };
        let memo = NativeSourceMemo {
            actor: target,
            session,
            occurrence: text,
            magnitude,
            receipt,
        };
        if let Some(i) = old {
            self.source_damage[i] = memo
        } else {
            self.source_damage.push(memo)
        }
        Some(receipt)
    }
    fn apply_source_player_heal(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
        magnitude: u32,
        occurrence: &str,
        now: crate::foundation::owner_timer::SemanticTimeMicros,
    ) -> Option<crate::player_lethal::PlayerHealReceipt> {
        use crate::player_lethal::PlayerHealReceipt;
        if self.has_pending_spell_commit(target, session)
            || runtime.assert_actor_spell_unreserved(target).is_err()
        {
            return None;
        }
        if !self
            .get(runtime, target, session)?
            .owned_conditions()
            .accepts_time(now.get())
        {
            return None;
        }
        let (health, max_health, _) = self.source_player_health(runtime, target, session)?;
        let text = Self::source_occurrence_text(occurrence)?;
        self.prune_source_memos(runtime);
        let old = self
            .source_heal
            .iter()
            .position(|m| m.actor == target && m.session == session);
        if let Some(i) = old {
            let m = &self.source_heal[i];
            if m.occurrence == occurrence {
                return (m.magnitude == magnitude).then_some(m.receipt);
            }
        }
        if old.is_none() {
            self.source_heal.try_reserve(1).ok()?;
        }
        // Reuse current main's cap/revision/no-op semantics; no duplicate health formula or cooldown.
        let (applied, vitals_revision) =
            self.apply_health_gain(runtime, target, session, u64::from(magnitude))?;
        let receipt = PlayerHealReceipt {
            applied,
            health_before: health,
            health_after: health + applied,
            max_health,
            vitals_revision,
        };
        let memo = NativeSourceMemo {
            actor: target,
            session,
            occurrence: text,
            magnitude,
            receipt,
        };
        if let Some(i) = old {
            self.source_heal[i] = memo
        } else {
            self.source_heal.push(memo)
        }
        Some(receipt)
    }
    fn apply_attack_mana_drain(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
        magnitude: u32,
        occurrence: &str,
        now: crate::foundation::owner_timer::SemanticTimeMicros,
    ) -> Option<crate::player_lethal::PlayerManaDrainReceipt> {
        use crate::player_lethal::PlayerManaDrainReceipt;
        if self.has_pending_spell_commit(target, session)
            || runtime.assert_actor_spell_unreserved(target).is_err()
        {
            return None;
        }
        if magnitude == 0
            || self.is_dead(target)
            || runtime
                .player_control_facts(target, session)
                .ok()?
                .control_loss
                .is_some()
        {
            return None;
        }
        if !self
            .get(runtime, target, session)?
            .owned_conditions()
            .accepts_time(now.get())
        {
            return None;
        }
        let text = Self::source_occurrence_text(occurrence)?;
        self.prune_source_memos(runtime);
        let old = self
            .source_mana
            .iter()
            .position(|m| m.actor == target && m.session == session);
        if let Some(i) = old {
            let m = &self.source_mana[i];
            if m.occurrence == occurrence {
                return (m.magnitude == magnitude).then_some(m.receipt);
            }
        }
        let current = self.get(runtime, target, session)?;
        let mana_before = current.vitals().mana;
        let (next, applied) = current.after_creature_mana_drain(magnitude)?;
        let receipt = PlayerManaDrainReceipt {
            applied,
            mana_before,
            mana_after: next.vitals().mana,
            vitals_revision: next.revision(),
        };
        if old.is_none() {
            self.source_mana.try_reserve(1).ok()?;
        }
        if applied > 0 && !self.commit(runtime, target, session, next) {
            return None;
        }
        let memo = NativeSourceMemo {
            actor: target,
            session,
            occurrence: text,
            magnitude,
            receipt,
        };
        if let Some(i) = old {
            self.source_mana[i] = memo
        } else {
            self.source_mana.push(memo)
        }
        Some(receipt)
    }
}

#[cfg(test)]
mod source_native_vitals_tests {
    use super::tests::{runtime_with_player, wound};
    use super::*;
    use crate::player_lethal::PlayerLethalVitals;
    fn ready(
        tag: u8,
    ) -> (
        ChannelRuntimeV1,
        ChannelSpellStates,
        ExactActorRef,
        GameSessionId,
    ) {
        let (mut runtime, actor, session) = runtime_with_player(tag);
        assert!(
            runtime
                .initialize_movement_test_position(
                    actor,
                    MovementLocalPosition {
                        x: 10,
                        y: 10,
                        floor: 7
                    }
                )
                .is_ok()
        );
        let mut states = ChannelSpellStates::default();
        let facts = CharacterCastFacts {
            vocation: crate::spell::Vocation::Druid,
            level: 30,
            magic_level: 20,
            max_health: 185,
            max_mana: 90,
            max_soul: 100,
        };
        assert!(
            states
                .initialize(
                    &runtime,
                    actor,
                    session,
                    facts,
                    (0, 0),
                    SemanticTimeMicros::from_micros(0)
                )
                .is_some()
        );
        (runtime, states, actor, session)
    }
    #[test]
    fn source_lethal_receipt_reuses_native_death_and_respawn_clears_source_replay() {
        let (mut r, mut s, a, g) = ready(0x41);
        wound(&mut s, a, g, 8);
        let Some(hit) = s.apply_attack_damage(
            &mut r,
            a,
            g,
            8,
            "source:lethal:0",
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0),
        ) else {
            panic!("native source hit")
        };
        let Some(native) = s.player_death(&r, a, g) else {
            panic!("native death")
        };
        let Some(receipt) = hit.death else {
            panic!("receipt")
        };
        assert_eq!(receipt.occurrence_bytes(), *native.occurrence.as_bytes());
        assert_eq!(receipt.position(), native.cell);
        assert_eq!(hit.health_after, 0);
        assert_eq!(
            s.apply_attack_damage(
                &mut r,
                a,
                g,
                8,
                "source:lethal:0",
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
            ),
            Some(hit)
        );
        assert_eq!(
            s.apply_attack_damage(
                &mut r,
                a,
                g,
                9,
                "source:lethal:0",
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
            ),
            None
        );
        assert_eq!(s.deaths.len(), 1);
        assert_eq!(
            s.apply_source_player_heal(
                &mut r,
                a,
                g,
                1,
                "heal:dead",
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
            ),
            None
        );
        assert_eq!(
            s.apply_attack_mana_drain(
                &mut r,
                a,
                g,
                1,
                "mana:dead",
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
            ),
            None
        );
        assert!(
            s.respawn(
                &r,
                a,
                g,
                native.occurrence,
                SemanticTimeMicros::from_micros(1)
            )
            .is_some()
        );
        assert!(s.source_damage.is_empty());
        assert!(s.source_mana.is_empty());
        assert!(s.source_heal.is_empty());
        assert!(
            s.apply_attack_damage(
                &mut r,
                a,
                g,
                8,
                "source:lethal:1",
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
            )
            .is_some()
        );
        assert!(!s.is_dead(a));
    }
    #[test]
    fn source_failed_native_mint_and_wrong_session_have_no_hp_or_memo_write() {
        let (mut r, mut s, a, g) = ready(0x42);
        wound(&mut s, a, g, 8);
        s.mint_death = || None;
        let before = s.get(&r, a, g).cloned();
        assert_eq!(
            s.apply_attack_damage(
                &mut r,
                a,
                g,
                8,
                "mint:failed",
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
            ),
            None
        );
        assert_eq!(s.get(&r, a, g).cloned(), before);
        assert!(s.source_damage.is_empty());
        assert!(s.deaths.is_empty());
        let (_, _, other) = runtime_with_player(0x43);
        assert_eq!(
            s.apply_attack_damage(
                &mut r,
                a,
                other,
                1,
                "session:wrong",
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
            ),
            None
        );
        assert_eq!(
            s.apply_source_player_heal(
                &mut r,
                a,
                other,
                1,
                "session:wrong",
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
            ),
            None
        );
        assert_eq!(
            s.apply_attack_mana_drain(
                &mut r,
                a,
                other,
                1,
                "session:wrong",
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
            ),
            None
        );
        assert_eq!(s.get(&r, a, g).cloned(), before);
    }
    #[test]
    fn source_heal_uses_current_maximum_native_credit_noop_revision_and_replay() {
        let (mut r, mut s, a, g) = ready(0x44);
        wound(&mut s, a, g, 100);
        let before = s.get(&r, a, g).cloned();
        let Some(heal) = s.apply_source_player_heal(
            &mut r,
            a,
            g,
            u32::MAX,
            "heal:0",
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0),
        ) else {
            panic!("native credit")
        };
        assert_eq!(
            (
                heal.applied,
                heal.health_after,
                heal.max_health,
                heal.vitals_revision
            ),
            (85, 185, 185, 2)
        );
        let Some(next) = s.get(&r, a, g).cloned() else {
            panic!("current")
        };
        let Some(before) = before else {
            panic!("before")
        };
        assert_eq!(
            (before.vitals().mana, before.vitals().soul),
            (next.vitals().mana, next.vitals().soul)
        );
        assert_eq!(
            s.apply_source_player_heal(
                &mut r,
                a,
                g,
                u32::MAX,
                "heal:0",
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
            ),
            Some(heal)
        );
        assert_eq!(s.get(&r, a, g), Some(&next));
        let Some(full) = s.apply_source_player_heal(
            &mut r,
            a,
            g,
            1,
            "heal:1",
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0),
        ) else {
            panic!("full credit")
        };
        assert_eq!((full.applied, full.vitals_revision), (0, 2));
        let Some(zero) = s.apply_source_player_heal(
            &mut r,
            a,
            g,
            0,
            "heal:2",
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0),
        ) else {
            panic!("zero")
        };
        assert_eq!((zero.applied, zero.vitals_revision), (0, 2));
    }
    #[test]
    fn source_mp_only_drain_clamps_current_pool_replays_and_exhaustion_keeps_revision() {
        let (mut r, mut s, a, g) = ready(0x45);
        let Some(before) = s.get(&r, a, g).cloned() else {
            panic!("before")
        };
        let Some(drain) = s.apply_attack_mana_drain(
            &mut r,
            a,
            g,
            u32::MAX,
            "mana:0",
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0),
        ) else {
            panic!("MP debit")
        };
        assert_eq!(
            (
                drain.applied,
                drain.mana_before,
                drain.mana_after,
                drain.vitals_revision
            ),
            (90, 90, 0, 2)
        );
        assert_eq!(
            s.apply_attack_mana_drain(
                &mut r,
                a,
                g,
                u32::MAX,
                "mana:0",
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
            ),
            Some(drain)
        );
        let Some(next) = s.get(&r, a, g) else {
            panic!("current")
        };
        assert_eq!(
            (before.vitals().health, before.vitals().soul),
            (next.vitals().health, next.vitals().soul)
        );
        assert!(!s.is_dead(a));
        let Some(empty) = s.apply_attack_mana_drain(
            &mut r,
            a,
            g,
            1,
            "mana:1",
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0),
        ) else {
            panic!("empty MP")
        };
        assert_eq!((empty.applied, empty.vitals_revision), (0, 2));
        assert_eq!(
            s.apply_attack_mana_drain(
                &mut r,
                a,
                g,
                2,
                "mana:1",
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
            ),
            None
        );
    }
}
/// Retained source payload reader ABI; source monster regeneration is N/A (zero captured definitions).
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
pub(crate) trait NativeRegenerationTickSource {
    fn gains(
        &self,
        runtime: &ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
        definition: &str,
        revision: u32,
        due: u64,
    ) -> Option<(u64, u64)>;
}
/// Immutable source payload retained from the trusted, already parsed native loader.
/// Runtime condition cursors remain exclusively in canonical actor slots.
#[derive(Debug)]
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
pub(crate) struct NativeRegenerationRegistry {
    world: crate::foundation::WorldId,
    server: [u8; 32],
    loader_digest: [u8; 32],
    entries: Vec<NativeRegenerationEntry>,
}
#[derive(Debug)]
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
struct NativeRegenerationEntry {
    key: String,
    revision: u32,
    source: crate::content::ProjectV2DefinitionRef,
    values: crate::content::ProjectV2ConditionRegeneration,
}
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
impl NativeRegenerationRegistry {
    /// Root's trusted native loader supplies its verified input SHA, never a network caller.
    pub(crate) fn from_trusted_native(
        runtime: &ChannelRuntimeV1,
        draft: &crate::content::ProjectV2Draft,
        loader_digest: [u8; 32],
    ) -> Option<Self> {
        use crate::content::{
            ProjectV2AbilityEffect, ProjectV2AuthoringProfileData, ProjectV2InlineEffectOperation,
        };
        let world = runtime.binding().world_id();
        let text: String = world
            .as_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        if draft.core.world_id != text || loader_digest == [0; 32] {
            return None;
        }
        let mut entries = Vec::new();
        for profile in &draft.state.authoring_profiles {
            let ProjectV2AuthoringProfileData::Ability(ability) = &profile.data else {
                continue;
            };
            let Some(details) = &ability.details else {
                continue;
            };
            let revision = profile
                .target
                .revision
                .strip_prefix("definition-r")?
                .parse::<u32>()
                .ok()?;
            if revision == 0 {
                return None;
            }
            for effect in &details.effects {
                let ProjectV2AbilityEffect::Inline(inline) = effect else {
                    continue;
                };
                let ProjectV2InlineEffectOperation::Condition { condition, .. } = &inline.operation
                else {
                    continue;
                };
                let Some(values) = condition.regeneration else {
                    continue;
                };
                let definition = crate::creature_condition_content::lower_condition_definition(
                    inline, revision, None,
                )
                .ok()?;
                if entries.iter().any(|e: &NativeRegenerationEntry| {
                    e.key == definition.key() && e.revision == revision
                }) {
                    return None;
                }
                entries.try_reserve(1).ok()?;
                entries.push(NativeRegenerationEntry {
                    key: definition.key().to_owned(),
                    revision,
                    source: profile.target.clone(),
                    values,
                });
            }
        }
        Some(Self {
            world,
            server: runtime.content_pin().server_artifact_digest(),
            loader_digest,
            entries,
        })
    }
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }
    pub(crate) fn loader_digest(&self) -> [u8; 32] {
        self.loader_digest
    }
}
impl NativeRegenerationTickSource for NativeRegenerationRegistry {
    fn gains(
        &self,
        _runtime: &ChannelRuntimeV1,
        _target: ExactActorRef,
        _session: GameSessionId,
        _definition: &str,
        _revision: u32,
        _due: u64,
    ) -> Option<(u64, u64)> {
        None
    }
}
#[derive(Debug)]
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
pub(crate) struct NativeOwnedPlayerTickReceipt {
    pub(crate) ticks: Vec<crate::ability::condition::ConditionTick<String>>,
    pub(crate) revision: u64,
    pub(crate) damage: u32,
    pub(crate) death: Option<PlayerDeath>,
}
impl ChannelSpellStates {
    /// Independently qualified map/field facts are supplied by the same source owner turn.
    /// Creature provenance is frozen by native source application, never reacquired from a
    /// dead caster. Player/unknown provenance must use the separate actual PvP legality owner.
    #[allow(
        dead_code,
        reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
    )]
    pub(crate) fn tick_source_player_conditions(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
        _occurrence: DecisionOccurrenceId,
        facts: crate::ability::condition::TickFacts,
        now: crate::foundation::owner_timer::SemanticTimeMicros,
    ) -> Option<NativeOwnedPlayerTickReceipt> {
        use crate::ability::condition::{ConditionSourceKind, TickKind};
        if self.is_dead(target)
            || self.has_pending_spell_commit(target, session)
            || runtime.assert_actor_spell_unreserved(target).is_err()
            || runtime
                .player_control_facts(target, session)
                .ok()?
                .control_loss
                .is_some()
        {
            return None;
        }
        let before = self.get(runtime, target, session)?;
        if before.vitals().health == 0 || !before.owned_conditions().accepts_time(now.get()) {
            return None;
        }
        let mut next = before.clone();
        let ticks = next.source_take_due(now.get(), facts);
        let mut damage = 0u32;
        for tick in &ticks {
            if next.vitals().health == 0 {
                break;
            }
            match tick.kind {
                TickKind::Damage {
                    amount,
                    refused: false,
                    ..
                } => {
                    if tick.provenance.source_kind != ConditionSourceKind::Creature
                        || !tick
                            .provenance
                            .source
                            .as_deref()
                            .is_some_and(|s| s.starts_with("creature:"))
                    {
                        return None;
                    }
                    let (successor, hit) = crate::spell::actor_conditions::stage_creature_hit(
                        &next,
                        amount,
                        now.get(),
                    )
                    .ok()?;
                    damage = damage.checked_add(hit.applied)?;
                    next = successor;
                }
                TickKind::Damage { refused: true, .. }
                | TickKind::Regeneration {
                    suppressed: true, ..
                } => {}
                TickKind::SpellRegeneration {
                    health_gain,
                    mana_gain,
                    suppressed,
                } => {
                    if !suppressed {
                        let (successor, _) = next.after_health_gain(u64::from(health_gain))?;
                        next = successor;
                        let (successor, _) = next.after_mana_gain(u64::from(mana_gain))?;
                        next = successor;
                    }
                }
                TickKind::Regeneration {
                    suppressed: false, ..
                } => return None,
            }
        }
        let changed = next != *before;
        let revision = if changed {
            before.revision().checked_add(1)?
        } else {
            before.revision()
        };
        next.source_set_batch_revision(revision);
        let death = if next.vitals().health == 0 {
            let cell = runtime.read_actor_position(target).ok()?.position();
            self.deaths.try_reserve(1).ok()?;
            Some(PlayerDeath {
                occurrence: (self.mint_death)()?,
                cell,
            })
        } else {
            None
        };
        if changed && !self.commit(runtime, target, session, next) {
            return None;
        }
        if let Some(death) = death {
            self.deaths.push((target, session, death));
        }
        Some(NativeOwnedPlayerTickReceipt {
            ticks,
            revision,
            damage,
            death,
        })
    }
}
#[cfg(test)]
mod owned_source_tick_tests {
    use super::*;
    use crate::ability::condition::{
        ApplicationFacts, ConditionDefinition, ConditionValues, DamageSchedule, DamageSegment,
        DotElement, TickFacts,
    };
    fn ready(
        tag: u8,
    ) -> (
        ChannelRuntimeV1,
        ChannelSpellStates,
        ExactActorRef,
        GameSessionId,
    ) {
        let (mut r, a, g) = super::tests::runtime_with_player(tag);
        assert!(r.initialize_first_entry_position(a).is_ok());
        let mut s = ChannelSpellStates::default();
        assert!(
            s.initialize(
                &r,
                a,
                g,
                super::tests::FACTS,
                (0, 0),
                SemanticTimeMicros::from_micros(0)
            )
            .is_some()
        );
        (r, s, a, g)
    }
    fn install(
        s: &mut ChannelSpellStates,
        r: &ChannelRuntimeV1,
        a: ExactActorRef,
        g: GameSessionId,
        defs: &[ConditionDefinition],
    ) {
        let root = GameplayDecisionRoot::from_bytes([111; 32]);
        let facts = ApplicationFacts {
            now: 0,
            base_speed: 220,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &root,
            occurrence: DecisionOccurrenceId::from_bytes([111; 16]),
        };
        let Some(before) = s.get(r, a, g) else {
            panic!("native state")
        };
        let Some(next) = before.stage_owned_test_conditions(defs, &facts) else {
            panic!("typed fixture")
        };
        assert!(s.commit(r, a, g, next));
    }
    fn poison(amount: u32) -> ConditionDefinition {
        let Some(d) = ConditionDefinition::new_damage_schedule(
            "test.source.poison",
            1,
            DotElement::Poison,
            DamageSchedule::Fixed {
                delayed: true,
                segments: vec![DamageSegment {
                    count: 3,
                    interval_ms: 1000,
                    amount,
                }],
            },
        ) else {
            panic!("source schedule")
        };
        d
    }
    // TEST_PARAMETERS: real captured monster regeneration count is zero, NOT_APPLICABLE.
    fn regen() -> ConditionDefinition {
        let Some(d) = ConditionDefinition::new(
            "test.typed.native.regeneration",
            1,
            ConditionValues::SpellRegeneration {
                duration_ms: 10000,
                sub_id: 0,
                health_gain: 5,
                health_interval_ms: 1000,
                mana_gain: 7,
                mana_interval_ms: 2000,
            },
        ) else {
            panic!("typed regeneration")
        };
        d
    }
    fn pass(
        s: &mut ChannelSpellStates,
        r: &mut ChannelRuntimeV1,
        a: ExactActorRef,
        g: GameSessionId,
        t: u64,
        f: TickFacts,
    ) -> Option<NativeOwnedPlayerTickReceipt> {
        s.tick_source_player_conditions(
            r,
            a,
            g,
            DecisionOccurrenceId::from_bytes([112; 16]),
            f,
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(t),
        )
    }
    #[test]
    fn actual_creature_field_periodic_routes_to_death_owner_and_failed_mint_keeps_cursor() {
        let (mut runtime, mut states, actor, session) = ready(0xb8);
        install(&mut states, &runtime, actor, session, &[poison(200)]);
        let before = states.get(&runtime, actor, session).cloned();
        let mint = states.mint_death;
        states.mint_death = || None;
        let now = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(1_000_000);
        assert_eq!(
            states.tick_current_creature_field_damage(
                &mut runtime,
                actor,
                session,
                now,
                TickFacts::default(),
                None
            ),
            Some(None)
        );
        assert_eq!(states.get(&runtime, actor, session), before.as_ref());
        assert!(states.deaths.is_empty());
        states.mint_death = mint;
        let result = states.tick_current_creature_field_damage(
            &mut runtime,
            actor,
            session,
            now,
            TickFacts::default(),
            None,
        );
        assert!(matches!(
            result,
            Some(Some((_, ActorVitals { health: 0, .. })))
        ));
        assert!(states.player_death(&runtime, actor, session).is_some());
        assert!(
            states
                .get(&runtime, actor, session)
                .is_some_and(|s| s.owned_conditions().instances().is_empty())
        );
        let retained = states.player_death(&runtime, actor, session);
        assert!(
            states
                .tick_current_creature_field_damage(
                    &mut runtime,
                    actor,
                    session,
                    now,
                    TickFacts::default(),
                    None
                )
                .is_none()
        );
        assert_eq!(states.player_death(&runtime, actor, session), retained);
    }
    #[test]
    fn actual_malformed_creature_field_origin_refuses_without_hp_or_cursor_write() {
        let (mut runtime, mut states, actor, session) = ready(0xba);
        let before = states.get(&runtime, actor, session).cloned();
        let Some(state) = before.as_ref() else {
            panic!("actual player owner");
        };
        let root = GameplayDecisionRoot::from_bytes([113; 32]);
        let facts = ApplicationFacts {
            now: 0,
            base_speed: 220,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &root,
            occurrence: DecisionOccurrenceId::from_bytes([113; 16]),
        };
        let Some((next, _)) = state.stage_creature_field_contact(
            "creature:field:{malformed}".into(),
            &poison(10),
            &facts,
            DotElement::Poison,
        ) else {
            panic!("TEST_PARAMETERS malformed historical source in actual canonical store");
        };
        assert!(states.commit(&runtime, actor, session, next));
        let before = states.get(&runtime, actor, session).cloned();
        let now = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(1_000_000);
        assert_eq!(
            states.tick_current_creature_field_damage(
                &mut runtime,
                actor,
                session,
                now,
                TickFacts::default(),
                None
            ),
            Some(None)
        );
        assert_eq!(states.get(&runtime, actor, session), before.as_ref());
        assert!(states.deaths.is_empty());
    }
    #[test]
    fn actual_creature_field_pz_preserves_hp_then_due_residual_uses_same_store() {
        let (mut runtime, mut states, actor, session) = ready(0xb9);
        install(&mut states, &runtime, actor, session, &[poison(10)]);
        let health = states
            .get(&runtime, actor, session)
            .map(|s| s.vitals().health);
        let pz = TickFacts {
            in_protection_zone: true,
            standing_on_field: None,
        };
        let now = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(1_000_000);
        assert!(
            states
                .tick_current_creature_field_damage(&mut runtime, actor, session, now, pz, None)
                .is_some()
        );
        assert_eq!(
            states
                .get(&runtime, actor, session)
                .map(|s| s.vitals().health),
            health
        );
        let now = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(2_000_000);
        assert!(
            states
                .tick_current_creature_field_damage(
                    &mut runtime,
                    actor,
                    session,
                    now,
                    TickFacts::default(),
                    None
                )
                .is_some()
        );
        assert!(
            states
                .get(&runtime, actor, session)
                .is_some_and(|s| Some(s.vitals().health) < health)
        );
        assert!(
            runtime
                .actor_conditions(actor, Some(session))
                .is_ok_and(|s| s.instances().is_empty())
        );
    }

    #[test]
    fn owned_tick_failed_mint_keeps_entire_state_then_lethal_stops_regeneration() {
        let (mut r, mut s, a, g) = ready(0xa1);
        install(&mut s, &r, a, g, &[poison(200), regen()]);
        let before = s.get(&r, a, g).cloned();
        let mint = s.mint_death;
        s.mint_death = || None;
        assert!(pass(&mut s, &mut r, a, g, 1000000, TickFacts::default()).is_none());
        assert_eq!(s.get(&r, a, g), before.as_ref());
        assert!(s.deaths.is_empty());
        s.mint_death = mint;
        let Some(receipt) = pass(&mut s, &mut r, a, g, 1000000, TickFacts::default()) else {
            panic!("owned lethal")
        };
        assert_eq!(receipt.damage, 185);
        assert!(receipt.death.is_some());
        let Some(state) = s.get(&r, a, g) else {
            panic!("nativeHP")
        };
        assert_eq!(state.vitals().health, 0);
        assert!(state.owned_conditions().instances().is_empty());
        assert_eq!(s.deaths.len(), 1);
        assert!(pass(&mut s, &mut r, a, g, 1000000, TickFacts::default()).is_none());
        assert_eq!(s.deaths.len(), 1);
    }
    #[test]
    fn owned_typed_regeneration_distinct_intervals_and_unknown_gain_refuses_without_cursor() {
        let (mut r, mut s, a, g) = ready(0xa2);
        super::tests::wound(&mut s, a, g, 100);
        let Some(state) = s.get(&r, a, g) else {
            panic!("current MP")
        };
        let Some((wounded, _)) = state.after_creature_mana_drain(20) else {
            panic!("MP stage")
        };
        assert!(s.commit(&r, a, g, wounded));
        install(&mut s, &r, a, g, &[regen()]);
        let Some(first) = pass(&mut s, &mut r, a, g, 1000000, TickFacts::default()) else {
            panic!("first interval")
        };
        assert_eq!(first.damage, 0);
        assert_eq!(s.get(&r, a, g).map(|v| v.vitals().health), Some(105));
        assert_eq!(s.get(&r, a, g).map(|v| v.vitals().mana), Some(70));
        let Some(_) = pass(&mut s, &mut r, a, g, 2000000, TickFacts::default()) else {
            panic!("second interval")
        };
        assert_eq!(s.get(&r, a, g).map(|v| v.vitals().health), Some(110));
        assert_eq!(s.get(&r, a, g).map(|v| v.vitals().mana), Some(77));
        let Some(unknown) = ConditionDefinition::new(
            "test.unknown.recovery",
            1,
            ConditionValues::Recovery {
                duration_ms: 10000,
                interval_ms: 1000,
            },
        ) else {
            panic!("unknown source")
        };
        let (mut r, mut s, a, g) = ready(0xa3);
        install(&mut s, &r, a, g, &[unknown]);
        let before = s.get(&r, a, g).cloned();
        assert!(pass(&mut s, &mut r, a, g, 1000000, TickFacts::default()).is_none());
        assert_eq!(s.get(&r, a, g), before.as_ref());
    }
    #[test]
    fn owned_tick_pz_refuses_damage_and_field_retains_compressed_source_budget() {
        let (mut r, mut s, a, g) = ready(0xa4);
        install(&mut s, &r, a, g, &[poison(5)]);
        let Some(receipt) = pass(
            &mut s,
            &mut r,
            a,
            g,
            1000000,
            TickFacts {
                in_protection_zone: true,
                standing_on_field: None,
            },
        ) else {
            panic!("current PZ")
        };
        assert_eq!(receipt.damage, 0);
        assert_eq!(s.get(&r, a, g).map(|v| v.vitals().health), Some(185));
        let (mut r, mut s, a, g) = ready(0xa5);
        install(&mut s, &r, a, g, &[poison(5)]);
        let Some(_) = pass(
            &mut s,
            &mut r,
            a,
            g,
            3000000,
            TickFacts {
                in_protection_zone: false,
                standing_on_field: Some(DotElement::Poison),
            },
        ) else {
            panic!("actual field facts")
        };
        assert_eq!(s.get(&r, a, g).map(|v| v.vitals().health), Some(170));
        assert_eq!(
            s.get(&r, a, g)
                .map(|v| v.owned_conditions().instances().len()),
            Some(1)
        );
    }
    #[test]
    fn owned_old_staged_state_rejected_after_current_revision_commit() {
        let (r, mut s, a, g) = ready(0xa6);
        let Some(state) = s.get(&r, a, g) else {
            panic!("current")
        };
        let Some((stale, _)) = state.after_creature_mana_drain(1) else {
            panic!("staged")
        };
        let current = stale.clone();
        assert!(s.commit(&r, a, g, current));
        let before = s.get(&r, a, g).cloned();
        assert!(!s.commit(&r, a, g, stale));
        assert_eq!(s.get(&r, a, g), before.as_ref());
    }
}

#[cfg(test)]
impl ChannelSpellStates {
    /// Component fixture seam inside the real owner: exact live creature provenance,
    /// native Player condition staging and existing session-checked CAS publication.
    /// This creates no production grant or mutable PlayerSpellState accessor.
    pub(crate) fn install_owned_source_condition_fixture(
        &mut self,
        runtime: &ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
        caster: ExactActorRef,
        definitions: &[crate::foundation::ConditionDefinition],
        facts: &crate::foundation::ApplicationFacts<'_>,
    ) -> bool {
        if !runtime.contains_live_creature(caster)
            || runtime.player_control_facts(target, session).is_err()
            || self.has_pending_spell_commit(target, session)
            || runtime.assert_actor_spell_unreserved(target).is_err()
        {
            return false;
        }
        let Some(before) = self.get(runtime, target, session) else {
            return false;
        };
        let source = format!(
            "creature:{}",
            caster
                .placement_identity()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        );
        let Some(next) = before.stage_source_player_conditions(source, definitions, &[], facts)
        else {
            return false;
        };
        self.commit(runtime, target, session, next)
    }
    /// Read-only copy of the actual canonical Player condition store for fixture assertions.
    pub(crate) fn owned_source_condition_fixture_snapshot(
        &self,
        runtime: &ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
    ) -> Option<crate::ability::condition::ConditionStore<String>> {
        Some(
            self.get(runtime, target, session)?
                .owned_conditions()
                .clone(),
        )
    }
}

impl ChannelSpellStates {
    /// Same locked Channel turn reads the canonical live attacker owner and
    /// commits Creature HP+store together. No player slot condition fallback.
    pub(crate) fn tick_source_creature_conditions(
        &self,
        runtime: &mut ChannelRuntimeV1,
        target: ExactActorRef,
        facts: crate::foundation::TickFacts,
        now: crate::foundation::owner_timer::SemanticTimeMicros,
    ) -> Result<crate::foundation::CreaturePeriodicReceipt, crate::foundation::CarrierError> {
        let mut lookup = |r: &ChannelRuntimeV1, source: ExactActorRef, at: u64| {
            if !r.borrow_exact_actor_lookup().contains(source) {
                return Ok(None);
            }
            let position = r.read_actor_position(source)?;
            if position.context() != r.pinned_movement_context() {
                return Err(crate::foundation::CarrierError::PositionContextMismatch);
            }
            let Some((_, session, state)) = self.actors.iter().find(|(a, _, _)| *a == source)
            else {
                // An extant live actor with no canonical state is UNKNOWN, not
                // a positive no-attacker fact. A retired exact generation is absent.
                return Err(crate::foundation::CarrierError::PlanConflict);
            };
            let current = r.player_control_facts(source, *session)?;
            if current.control_loss.is_some() || self.is_dead(source) || state.vitals().health == 0
            {
                return Ok(None);
            }
            if !state.owned_conditions().accepts_time(at) {
                return Err(crate::foundation::CarrierError::PlanConflict);
            }
            if self.has_pending_spell_commit(source, *session) {
                return Err(crate::foundation::CarrierError::PlanConflict);
            }
            r.assert_actor_spell_unreserved(source)?;
            use crate::ability::condition::ConditionValues;
            let percent = match state.owned_conditions().attributes_at(at) {
                Some(ConditionValues::Attributes {
                    damage_dealt_percent,
                    ..
                }) => damage_dealt_percent,
                _ => 100,
            };
            Ok(Some(percent))
        };
        runtime.apply_creature_periodic_turn_with_sources(target, now.get(), facts, &mut lookup)
    }
}
#[cfg(test)]
mod source_creature_dot_owner_tests {
    #![allow(clippy::expect_used, clippy::panic)]
    use super::*;
    use crate::foundation::{
        ApplicationFacts, CharacterId, CommandId, CommandRef, CompiledCreaturePolicies,
        CompiledCreaturePolicy, ConditionDefinition, ConditionSourceKind, ConditionValues,
        CreatureExactRatio, CreatureFlags, CreatureResistance, DotElement, MovementLocalPosition,
    };
    use crate::spell::combat_batch::{
        OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellOccurrenceBinding,
    };
    use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
    fn fixture() -> (
        ChannelRuntimeV1,
        ChannelSpellStates,
        ExactActorRef,
        ExactActorRef,
        GameSessionId,
    ) {
        fixture_with_phase(true)
    }
    fn fixture_with_phase(
        attackable: bool,
    ) -> (
        ChannelRuntimeV1,
        ChannelSpellStates,
        ExactActorRef,
        ExactActorRef,
        GameSessionId,
    ) {
        let (mut runtime, source, session) = super::tests::runtime_with_player(0x41);
        runtime
            .initialize_first_entry_position(source)
            .expect("actual pinned player");
        let policy = CompiledCreaturePolicy {
            definition_key: "creature:source-healing-target".into(),
            definition_revision: "test-source-1".into(),
            display_name: "source healing target".into(),
            maximum_health: 20,
            base_speed: 100,
            outfit_look_type: 1,
            object_look_type: None,
            summonable: false,
            convinceable: false,
            mana_cost: None,
            is_familiar: false,
            condition_immunities: vec![],
            armor: Some(0),
            mitigation: Some(CreatureExactRatio {
                numerator: 0,
                denominator: 1,
            }),
            resistances: vec![],
            damage_immunities: vec![],
            healing_from_damage: vec![CreatureResistance {
                damage_type: "fire".into(),
                percent: CreatureExactRatio {
                    numerator: 100,
                    denominator: 1,
                },
            }],
            flags: CreatureFlags {
                attackable,
                illusionable: false,
                health_hidden: false,
            },
            preferred_distance: Some(1),
            reward_boss: Some(false),
        };
        runtime
            .install_companion_policies(
                CompiledCreaturePolicies::from_active_artifact(
                    runtime.content_pin().server_artifact_digest(),
                    vec![policy],
                )
                .expect("actual loader policy"),
            )
            .expect("fullpin");
        let target = runtime
            .admit_pinned_test_creature(MovementLocalPosition {
                x: 1,
                y: 0,
                floor: 0,
            })
            .expect("physical actor under actual pin");
        runtime
            .install_creature_policy(target, "creature:source-healing-target")
            .expect("current native decoded profile");
        let mut states = ChannelSpellStates::default();
        states
            .initialize(
                &runtime,
                source,
                session,
                super::tests::FACTS,
                (0, 0),
                SemanticTimeMicros::from_micros(0),
            )
            .expect("actual canonical PlayerSpellState");
        let root = GameplayDecisionRoot::from_bytes([8; 32]);
        // TEST_PARAMETERS: Attributes is a native Player combat modifier.
        // Source-player lowering intentionally does not accept this native kind.
        let definition = crate::ability::condition::ConditionDefinition::new(
            "test.source.attacker.buff",
            1,
            crate::ability::condition::ConditionValues::Attributes {
                duration_ms: 5000,
                critical_chance_percent: 0,
                critical_extra_percentage_points: 0,
                damage_dealt_percent: 50,
                incoming_reduction_percent: 0,
            },
        )
        .expect("typed native Player combat buff");
        let facts = ApplicationFacts {
            now: 0,
            base_speed: 100,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &root,
            occurrence: DecisionOccurrenceId::from_bytes([1; 16]),
        };
        let native_facts = crate::ability::condition::ApplicationFacts {
            now: facts.now,
            base_speed: facts.base_speed,
            mana_shield_capacity: facts.mana_shield_capacity,
            target_reentry_protected: facts.target_reentry_protected,
            source_reentry_protected: facts.source_reentry_protected,
            target_is_player: true,
            decision_root: &root,
            occurrence: facts.occurrence,
        };
        let next = states
            .get(&runtime, source, session)
            .expect("sole canonical owner")
            .stage_owned_test_conditions(&[definition], &native_facts)
            .expect("actual native Player buff owner");
        assert!(states.commit(&runtime, source, session, next));
        let wound = OwnerCombatBatch {
            caster: source,
            attacker: CharacterId::decode(&[
                1, 0x90, 0, 0, 0, 5, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 5,
            ])
            .expect("test character"),
            current_lease_generation: 1,
            command: CommandRef::new(session, CommandId::new(1).expect("command")),
            occurrence: SpellOccurrenceBinding {
                id: "source-dot-wound".into(),
                revisions: ["r1", "r1", "r1", "r1", "r1"].map(str::to_owned),
            },
            binding: b"actual-source-wound".to_vec(),
            anchor: None,
            now_ms: 0,
            effects: vec![OwnerCombatEffect {
                target,
                sub_ordinal: 0,
                change: OwnerCombatChange::Damage {
                    target_atom: runtime
                        .creature_spell_target_atom(target)
                        .expect("actual target identity"),
                    magnitude: 15,
                },
            }],
            deferred: None,
        };
        let staged = runtime
            .stage_spell_batch(&wound)
            .expect("native wound stage");
        runtime.commit_spell_batch(staged).expect("native HP");
        let before = runtime.companion_snapshot(target).expect("actual target");
        let mut next = before.state.clone();
        let dot = ConditionDefinition::new(
            "test.source.player.dot",
            1,
            ConditionValues::DamageOverTime {
                element: DotElement::Fire,
                total_min: 30,
                total_max: 30,
                per_tick: 10,
                interval_ms: 1000,
                delayed: true,
            },
        )
        .expect("typed source DOT");
        let facts = ApplicationFacts {
            target_is_player: false,
            occurrence: DecisionOccurrenceId::from_bytes([2; 16]),
            ..facts
        };
        next.conditions
            .apply(&dot, Some(source), ConditionSourceKind::Player, &[], &facts)
            .expect("actual source in native Creature store");
        runtime
            .compare_companion_state(&before, next)
            .expect("current source install");
        (runtime, states, source, target, session)
    }
    #[test]
    fn current_player_buff_comes_from_canonical_owner_dot_heals_before_scaled_damage_and_replays() {
        let (mut runtime, states, source, target, session) = fixture();
        assert!(
            runtime
                .actor_conditions(source, Some(session))
                .expect("native player compatibility store")
                .instances()
                .is_empty()
        );
        let receipt = states
            .tick_source_creature_conditions(
                &mut runtime,
                target,
                crate::foundation::TickFacts::default(),
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(1_000_000),
            )
            .expect("actual owned periodic handler");
        assert_eq!(
            (receipt.health_before, receipt.health_after),
            (5, 10),
            "map10 before50% source buff then damage5"
        );
        let retry = states
            .tick_source_creature_conditions(
                &mut runtime,
                target,
                crate::foundation::TickFacts::default(),
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(1_000_000),
            )
            .expect("same turn retry");
        assert!(retry.ticks.is_empty());
        assert_eq!(retry.health_after, 10);
    }
    #[test]
    fn retired_exact_attacker_cannot_heal_and_pz_tick_uses_real_owner_store_without_hp() {
        let (mut runtime, states, source, target, _) = fixture();
        runtime
            .remove_test_actor(source)
            .expect("retire source generation");
        let first = states
            .tick_source_creature_conditions(
                &mut runtime,
                target,
                crate::foundation::TickFacts {
                    in_protection_zone: true,
                    standing_on_field: None,
                },
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(1_000_000),
            )
            .expect("qualified PZ suppression");
        assert_eq!((first.health_before, first.health_after), (5, 5));
        let second = states
            .tick_source_creature_conditions(
                &mut runtime,
                target,
                crate::foundation::TickFacts::default(),
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(2_000_000),
            )
            .expect("retired attacker resolved absent");
        assert_eq!(
            (second.health_before, second.health_after),
            (5, 0),
            "no fake attacker buff/healing; residual damage remains"
        );
    }
    #[test]
    fn stale_lineage_content_pin_refuses_before_hp_and_condition_cursor_publication() {
        let (mut runtime, states, source, target, session) = fixture();
        let before = runtime.companion_snapshot(target).expect("current target");
        let mut next = before.state.clone();
        next.conditions
            .remove_type(crate::foundation::ConditionType::DamageOverTime(
                DotElement::Fire,
            ));
        let root = GameplayDecisionRoot::from_bytes([12; 32]);
        let facts = ApplicationFacts {
            now: 0,
            base_speed: 100,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: false,
            decision_root: &root,
            occurrence: DecisionOccurrenceId::from_bytes([3; 16]),
        };
        let definition = ConditionDefinition::new(
            "test.source.pin-negative",
            1,
            ConditionValues::DamageOverTime {
                element: DotElement::Fire,
                total_min: 10,
                total_max: 10,
                per_tick: 10,
                interval_ms: 1000,
                delayed: true,
            },
        )
        .expect("typed source recipe");
        let lineage = crate::foundation::condition::ConditionLineage {
            source_character: [3; 16],
            source_session: *session.as_bytes(),
            source_lease_generation: 1,
            source_actor_placement: source.placement_identity(),
            source_scope_generation: source.scope_generation().get(),
            source_content_digest: [9; 32],
            source_catalog_digest: [8; 32],
            source_creation_command: 7,
        };
        next.conditions
            .apply_with_lineage(
                &definition,
                Some(source),
                ConditionSourceKind::Player,
                &[],
                &facts,
                &lineage,
            )
            .expect("retained historical source receipt");
        runtime
            .compare_companion_state(&before, next)
            .expect("real owner source store");
        let before = runtime.companion_snapshot(target).expect("full preimage");
        assert!(
            states
                .tick_source_creature_conditions(
                    &mut runtime,
                    target,
                    crate::foundation::TickFacts::default(),
                    crate::foundation::owner_timer::SemanticTimeMicros::from_micros(1_000_000)
                )
                .is_err()
        );
        assert_eq!(
            runtime
                .companion_snapshot(target)
                .expect("untouched HP/store"),
            before
        );
    }

    #[test]
    fn current_nonattackable_phase_does_not_trigger_pre_immunity_healing() {
        let (mut runtime, states, _, target, _) = fixture_with_phase(false);
        let receipt = states
            .tick_source_creature_conditions(
                &mut runtime,
                target,
                crate::foundation::TickFacts::default(),
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(1_000_000),
            )
            .expect("actual nonattackable phase");
        assert_eq!(
            (receipt.health_before, receipt.health_after),
            (5, 5),
            "phaseattackability precedes healingMap; damageimmunity differs"
        );
    }
}

#[derive(Debug)]
pub(in crate::gameplay_transport) struct CreatureFieldContact {
    pub(in crate::gameplay_transport) source: String,
    pub(in crate::gameplay_transport) definition: crate::ability::condition::ConditionDefinition,
    pub(in crate::gameplay_transport) element: crate::ability::condition::DotElement,
    pub(in crate::gameplay_transport) content: [u8; 32],
}
impl ChannelSpellStates {
    /// Real STEP and field initial HP/DOT are staged before movement. This is an
    /// owner method, not a callback that can mutate the stores during preflight.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::gameplay_transport) fn step_with_creature_field_contact(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        cells: &crate::content::NativeEntryMovementCells,
        actor: ExactActorRef,
        session: GameSessionId,
        now_us: u64,
        direction: crate::movement::CardinalStep,
        blocking: &std::collections::BTreeSet<crate::content::LogicalCell>,
        equipment_delta: Option<i32>,
        proof: Option<crate::movement::source_floor_change::SourceStepProof<'_>>,
        contact: Option<CreatureFieldContact>,
    ) -> actor_movement::StepInChannel {
        let Some(contact) = contact else {
            return actor_movement::step_in_channel_with_source_step(
                runtime,
                self,
                cells,
                actor,
                session,
                now_us,
                direction,
                blocking,
                equipment_delta,
                proof,
            );
        };
        if contact.content != runtime.content_pin().server_artifact_digest()
            || self.has_pending_spell_commit(actor, session)
            || self.is_dead(actor)
            || runtime.assert_actor_spell_unreserved(actor).is_err()
        {
            return Err(crate::movement::MovementError::NotQualified);
        }
        let index = self
            .index(actor, session)
            .ok_or(crate::movement::MovementError::NotQualified)?;
        let before = self
            .get(runtime, actor, session)
            .ok_or(crate::movement::MovementError::NotQualified)?;
        let protected = runtime
            .current_player_reentry_protection(actor, session, now_us)
            .map_err(crate::movement::MovementError::Actor)?;
        let root = GameplayDecisionRoot::from_bytes(contact.content);
        let mut occurrence_bytes = [0; 16];
        let digest = Sha256::digest(contact.source.as_bytes());
        occurrence_bytes.copy_from_slice(&digest[..16]);
        let facts = crate::ability::condition::ApplicationFacts {
            now: now_us,
            base_speed: 1,
            mana_shield_capacity: 0,
            target_reentry_protected: protected,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &root,
            occurrence: DecisionOccurrenceId::from_bytes(occurrence_bytes),
        };
        let (next, _damage) = before
            .stage_creature_field_contact(
                contact.source,
                &contact.definition,
                &facts,
                contact.element,
            )
            .ok_or(crate::movement::MovementError::NotQualified)?;
        let changed = next != *before;
        let death = if next.vitals().health == 0 {
            self.deaths
                .try_reserve(1)
                .map_err(|_| crate::movement::MovementError::NotQualified)?;
            Some((self.mint_death)().ok_or(crate::movement::MovementError::NotQualified)?)
        } else {
            None
        };
        // Existing movement computes speed/ground/duration/current proof before
        // committing its one physical position. It does not mutate Player state.
        let result = actor_movement::step_in_channel_with_source_step(
            runtime,
            self,
            cells,
            actor,
            session,
            now_us,
            direction,
            blocking,
            equipment_delta,
            proof,
        )?;
        if changed {
            self.actors[index].2 = next;
        }
        if let Some(occurrence) = death {
            self.deaths.push((
                actor,
                session,
                PlayerDeath {
                    occurrence,
                    cell: result.0.position(),
                },
            ));
        }
        Ok(result)
    }
}
