//! Spell cast wire composition (`OTERYN_PLAYER_SPELL_CAST_WIRE_AND_VITALS_CONTRACT_CANDIDATE_V1`
//! §9 step 2). The typed payload codecs live in `oteryn-protocol-oteryn::actor_spell` (#1254) and
//! are re-exported here, as `world_spatial` does; this module adds the Channel owner's per-actor
//! vitals and cooldowns and the owner work item of one cast.

pub(crate) use oteryn_protocol_oteryn::actor_spell::*;

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
pub(crate) use actor_movement::StepInChannel;
pub(crate) use owner_commit::{PlayerBatchPreflight, commit_owner_batch, stage_player_batch};

use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, SemanticTimeMicros, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

use crate::ability::creature_bite::{CreatureBiteVitals, FlooredDamage};
use crate::ability::{AbilityOccurrence, RevisionSet};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId};
use crate::spell::SpellBook;
use crate::spell::cast::{CastContext, CharacterCastFacts, PlayerSpellState, cast};

/// Spell cast §4 (SPELL-D2): the Channel owner's vitals and cooldowns of its present player
/// actors, runtime-actor-local and never durable. Like the door runtime it sits beside the
/// `ChannelRuntimeV1` and is only used while that runtime is locked (runtime first), within the
/// same owner work item. An entry is keyed by the exact actor (World, Channel, scope generation,
/// slot and slot generation), so it lives exactly as long as that actor stays present: every read
/// and write first proves the actor is still the committed player of the GameSession, and an
/// entry whose actor is gone is unreachable and dropped at the next initialization.
#[derive(Debug, Default)]
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
        let state = self.get(runtime, actor, game_session_id)?;
        let (next, gained) = state.after_mana_gain(amount)?;
        let revision = next.revision();
        if gained == 0 {
            return Some((0, revision));
        }
        self.commit(runtime, actor, game_session_id, next)
            .then_some((gained, revision))
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

/// GAME-AI-01 slice §4.6/§4.7: the vitals owner's side of a creature bite. One floored hit is one
/// compare-committed vitals revision; a hit that removes nothing (health already 1) writes nothing
/// and reports the current revision.
impl CreatureBiteVitals for ChannelSpellStates {
    fn apply_creature_damage(
        &mut self,
        runtime: &ChannelRuntimeV1,
        target: ExactActorRef,
        target_session: GameSessionId,
        magnitude: u32,
        now: crate::foundation::owner_timer::SemanticTimeMicros,
    ) -> Option<(FlooredDamage, u64)> {
        // Durable unknown outcomes retain their exact player before-state. A bite
        // must not invalidate that state, including another caster's reserved target.
        if self.has_pending_spell_commit(target, target_session)
            || runtime.assert_actor_spell_unreserved(target).is_err()
        {
            return None;
        }
        let state = self.get(runtime, target, target_session)?;
        let (next, damage) =
            crate::spell::actor_conditions::stage_creature_hit(state, magnitude, now.get()).ok()?;
        if next.revision() == state.revision() {
            return Some((damage, state.revision()));
        }
        let revision = next.revision();
        self.commit(runtime, target, target_session, next)
            .then_some((damage, revision))
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
        let world = WorldId::decode(&uuid_v7(0x60)).expect("world");
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&uuid_v7(0x61)).expect("channel"),
            NodeId::decode(&uuid_v7(0x62)).expect("node"),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
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
        let (runtime, actor, session) = runtime_with_player(0x35);
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
                &runtime,
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
        let (damage, revision) = states
            .apply_creature_damage(
                &runtime,
                actor,
                session,
                8,
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0),
            )
            .expect("unreserved positive control");
        assert_eq!((damage.applied, damage.health_after, revision), (8, 492, 2));
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
                    &runtime,
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

    /// GAME-AI-01 slice §4.6/§4.7: a creature bite reaches the real vitals owner as one floored
    /// vitals revision, and a hit at health 1 writes nothing.
    #[test]
    fn a_creature_bite_lowers_real_vitals_to_the_floor_of_one() {
        use crate::ability::AiAbilityAdapter;
        use crate::ability::creature_bite::{
            CreatureBiteDefinition, CreatureBiteLedger, ReentryProtection, commit_ai_bite,
        };
        use crate::foundation::MovementLocalPosition;
        use crate::foundation::owner_timer::SemanticTimeMicros as OwnerTime;

        let (mut runtime, actor, session) = runtime_with_player(0x31);
        let at = |x, y| MovementLocalPosition { x, y, floor: 7 };
        runtime
            .initialize_movement_test_position(actor, at(10, 10))
            .expect("player position");
        let creature = runtime.admit_test_creature(at(11, 10)).expect("creature");
        let mut states = ChannelSpellStates::default();
        states
            .initialize(&runtime, actor, session, FACTS, (0, 0), now(0))
            .expect("vitals");
        wound(&mut states, actor, session, 5);
        let mut ledger = CreatureBiteLedger::default();
        let mut bite = |sequence, micros| {
            commit_ai_bite(
                &mut ledger,
                &runtime,
                &mut states,
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
        };
        let first = bite(0, 0).expect("first bite");
        assert_eq!((first.damage.applied, first.damage.health_after), (4, 1));
        assert_eq!(first.vitals_revision, 2);
        let second = bite(1, 2_000_000).expect("second bite");
        assert_eq!((second.damage.applied, second.vitals_revision), (0, 2));
        let (revision, vitals) = observe_vitals(&runtime, &states, actor, session).expect("vitals");
        assert_eq!((revision, vitals.health), (2, 1));
    }
}
