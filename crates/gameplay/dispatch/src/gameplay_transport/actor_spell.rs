//! Spell cast wire composition (`OTERYN_PLAYER_SPELL_CAST_WIRE_AND_VITALS_CONTRACT_CANDIDATE_V1`
//! §9 step 2). The typed payload codecs live in `oteryn-protocol-oteryn::actor_spell` (#1254) and
//! are re-exported here, as `world_spatial` does; this module adds the Channel owner's per-actor
//! vitals and cooldowns and the owner work item of one cast.

pub use oteryn_protocol_oteryn::actor_spell::*;
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
#[cfg(test)]
use crate::foundation::MovementLocalPosition;
pub use crate::monster_combat_lane::native_conditions::{
    NativeConditionAdmission, NativeConditionApplication, NativeConditionError,
};
#[cfg(test)]
pub use crate::monster_combat_lane::native_invisibility::{
    CreatureVision, InvisibleTileCombatPolicy,
};
pub use crate::monster_combat_lane::native_invisibility::{SelfInvisibleSource, SelfSpeedSource};

#[path = "actor_movement.rs"]
mod actor_movement;
#[path = "familiar_cast.rs"]
mod familiar_cast;
#[path = "movement_equipment.rs"]
mod movement_equipment;
pub(in crate::gameplay_transport) use familiar_cast::{FamiliarActorEnd, FamiliarLogoutSave};
#[cfg(test)]
pub(in crate::gameplay_transport) use familiar_cast::{
    FamiliarFenceRead, familiar_end_proven, familiar_end_read,
};
#[path = "familiar_defense.rs"]
mod familiar_defense;
#[path = "field_step_ingress.rs"]
mod field_step_ingress;
#[path = "native_combat_cast.rs"]
pub(in crate::gameplay_transport) mod native_combat_cast;
#[path = "parameter_cast.rs"]
mod parameter_cast;
#[path = "spell_timer_callbacks.rs"]
mod spell_timer_callbacks;
#[path = "world_item_cast.rs"]
mod world_item_cast;
pub(in crate::gameplay_transport) use native_combat_cast::ordinary_combat::due_chain_presentations as prepare_ordinary_due_presentations;
pub(in crate::gameplay_transport) use native_combat_cast::ordinary_combat::prepare_due as prepare_ordinary_due_from_owners;
pub(in crate::gameplay_transport) use native_combat_cast::{
    NativeCastDispatch, replay_with_rune, rune_item_cast::current_revision as current_rune_revision,
};
#[path = "spell_periodic.rs"]
mod spell_periodic;
#[path = "training_save.rs"]
mod training_save;
pub(in crate::gameplay_transport) use training_save::TrainingSave;
#[path = "actor_spell_commit.rs"]
mod owner_commit;
#[path = "stance_cast.rs"]
mod stance_cast;
pub use owner_commit::{
    CheckedOwnerBatch, PlayerBatchPreflight, check_owner_batch, commit_owner_batch,
    install_owner_batch, stage_player_batch,
};

use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, SemanticTimeMicros, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

use crate::ability::creature_bite::{CreatureBiteVitals, CreatureHit};
use crate::ability::{AbilityOccurrence, RevisionSet};
use crate::durability::character_death::PlayerDeathOccurrence;
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
///
/// DEATH-2 (Reference first player death decision §4.2, §4.5): the vitals owner also keeps each
/// present player's death, from the lethal write until its respawn. A dead player takes no
/// command, no damage and no credit, and its death record goes with its entry.
#[derive(Debug)]
pub struct ChannelSpellStates {
    pub(in crate::gameplay_transport) owner_wake: std::sync::Arc<tokio::sync::Notify>,
    pub(in crate::gameplay_transport) actors: Vec<(ExactActorRef, GameSessionId, PlayerSpellState)>,
    /// One real Channel-owner lane; no timer is restored from client intent.
    pub spell_timers: Option<crate::spell::delayed_execution::SpellTimerOwner>,
    pub pending_familiars: Vec<
        PendingSpellMarker<
            oteryn_protocol_oteryn::actor_spell::SpellCastIntent,
            familiar_cast::PreparedFamiliarCast,
        >,
    >,
    pub pending_familiar_lifecycle: Vec<familiar_cast::PreparedFamiliarLifecycle>,
    pub pending_familiar_logouts: Vec<familiar_cast::PreparedFamiliarLogout>,
    /// ITEM-USE-1c: the item-use gate of each present actor (cooldowns, the one use in flight).
    pub(in crate::gameplay_transport) item_uses: super::item_use::ItemUseGates,
    pub pending_native: Vec<
        PendingSpellMarker<
            native_combat_cast::NativeMarkerIntent,
            native_combat_cast::PendingNativeCast,
        >,
    >,
    pub pending_world_items: Vec<
        PendingSpellMarker<
            oteryn_protocol_oteryn::actor_spell::SpellCastIntent,
            world_item_cast::PreparedWorldItemCast,
        >,
    >,
    pub pending_parameters: Vec<
        PendingSpellMarker<
            oteryn_protocol_oteryn::actor_spell_v2::ParameterSpellCastIntent,
            parameter_cast::PreparedParameterCast,
        >,
    >,
    pub presentations: Option<super::spell_presentations::SpellPresentationOwner>,
    /// One bounded Channel pass per second, shared by all present actors.
    pub(in crate::gameplay_transport) next_item_deadline_pass_us: u64,
    pub(in crate::gameplay_transport) source_map_initialized:
        Option<crate::durability::native_map_items_abi::NativeMapOwnerBinding>,
    pub(in crate::gameplay_transport) next_map_initialization_pass_us: u64,
    pub(in crate::gameplay_transport) next_party_deadline_pass_us: u64,
    pub(in crate::gameplay_transport) monster_summons: crate::monster_summon::NativeSummonOwner,
    pub(in crate::gameplay_transport) monster_melee: crate::ai_monster_melee::MonsterMeleeOwner,
    pub(in crate::gameplay_transport) next_monster_ai_pass_us: u64,
    pub(in crate::gameplay_transport) monster_ai_sequence: u64,
    pub(in crate::gameplay_transport) monster_ai_cursor: usize,
    pub(in crate::gameplay_transport) pvp_locks: Vec<(
        ExactActorRef,
        GameSessionId,
        oteryn_gameplay_combat_rules::pvp::CombatLock,
    )>,
    pub(in crate::gameplay_transport) deaths: Vec<(ExactActorRef, GameSessionId, PlayerDeath)>,
    mint_death: fn() -> Option<PlayerDeathOccurrence>,
    source_secondary_memos: Vec<actor_conditions::NativeSecondaryMemo>,
    source_damage: Vec<NativeSourceMemo<crate::player_lethal::PlayerDamageReceipt>>,
    source_mana: Vec<NativeSourceMemo<crate::player_lethal::PlayerManaDrainReceipt>>,
    source_heal: Vec<NativeSourceMemo<crate::player_lethal::PlayerHealReceipt>>,
}

/// The pending marker of one committing writer's attempt (ARCH-SPELL-LOCK-2 §1.4). It holds the
/// attempt's identity only, so `has_pending_spell_commit` reads true for the caster from the start
/// of its pass to the end that installs or releases the batch. `attempt` is `None` while the pass
/// holds the attempt, or while the lane holds it in `unresolved`; the marker stays in both cases.
#[derive(Debug)]
pub struct PendingSpellMarker<I, T> {
    pub actor: ExactActorRef,
    pub session: GameSessionId,
    pub command: crate::foundation::CommandRef,
    pub intent: I,
    pub attempt: Option<T>,
}

impl<I, T> PendingSpellMarker<I, T> {
    pub fn is_for(&self, actor: ExactActorRef, session: GameSessionId) -> bool {
        self.actor == actor && self.session == session
    }

    /// Puts an attempt back into the caster's marker, creating the marker if none exists.
    pub fn restore(
        markers: &mut Vec<Self>,
        actor: ExactActorRef,
        session: GameSessionId,
        command: crate::foundation::CommandRef,
        intent: I,
        attempt: T,
    ) {
        match markers.iter_mut().find(|m| m.is_for(actor, session)) {
            Some(marker) => marker.attempt = Some(attempt),
            None => markers.push(Self {
                actor,
                session,
                command,
                intent,
                attempt: Some(attempt),
            }),
        }
    }

    /// The end of a writer's pass: an attempt the pass still holds goes back into the marker.
    /// Without one, the marker stays only while the lane holds the attempt in `unresolved`; an
    /// installed, released or never prepared attempt removes it.
    pub fn settle(
        markers: &mut Vec<Self>,
        actor: ExactActorRef,
        session: GameSessionId,
        command: crate::foundation::CommandRef,
        intent: I,
        leftover: Option<T>,
        parked: bool,
    ) {
        match leftover {
            Some(attempt) => Self::restore(markers, actor, session, command, intent, attempt),
            None if !parked => markers.retain(|m| !m.is_for(actor, session)),
            None => {
                if !markers.iter().any(|m| m.is_for(actor, session)) {
                    markers.push(Self {
                        actor,
                        session,
                        command,
                        intent,
                        attempt: None,
                    });
                }
            }
        }
    }

    /// The original of a resolving attempt from its caster's marker (§1.6): the command id and
    /// the intent, only while the marker holds the attempt. A marker without one is vacant (its
    /// writer pass was cancelled after it installed, released or never prepared the attempt) and
    /// is removed, since the resolution holds the lane and no pass or parked record owns it.
    pub fn take_original(
        markers: &mut Vec<Self>,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<(u64, I)>
    where
        I: Clone,
    {
        let original = markers
            .iter()
            .find(|m| m.is_for(actor, session) && m.attempt.is_some())
            .map(|m| (m.command.command_id().get(), m.intent.clone()));
        if original.is_none() {
            markers.retain(|m| !m.is_for(actor, session));
        }
        original
    }

    /// Takes the attempt out of the caster's marker, leaving the marker in place.
    pub fn take_attempt(
        markers: &mut [Self],
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<T> {
        markers
            .iter_mut()
            .find(|m| m.is_for(actor, session))
            .and_then(|m| m.attempt.take())
    }
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
            item_uses: Default::default(),
            pending_native: Default::default(),
            pending_world_items: Default::default(),
            pending_parameters: Default::default(),
            presentations: Default::default(),
            next_item_deadline_pass_us: Default::default(),
            source_map_initialized: Default::default(),
            next_map_initialization_pass_us: Default::default(),
            next_party_deadline_pass_us: Default::default(),
            monster_melee: Default::default(),
            monster_summons: Default::default(),
            next_monster_ai_pass_us: Default::default(),
            monster_ai_sequence: Default::default(),
            monster_ai_cursor: Default::default(),
            pvp_locks: Vec::new(),
            deaths: Vec::new(),
            mint_death: mint_player_death_occurrence,
            source_secondary_memos: Vec::new(),
            source_damage: Vec::new(),
            source_mana: Vec::new(),
            source_heal: Vec::new(),
        }
    }
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
    pub fn has_pending_spell_commit(&self, actor: ExactActorRef, session: GameSessionId) -> bool {
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
    pub fn apply_current_premium(
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
    pub fn drain_spell_timers(
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
    pub fn load_owned_build(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        build: &crate::durability::character_build::DurableBuildState,
    ) -> bool {
        self.get_mut(runtime, actor, session)
            .is_some_and(|state| state.load_owned_build(build).is_ok())
    }
    pub fn load_owned_stance(
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
    pub fn initialize(
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
    pub fn resume(
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
    pub fn detach(
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
    pub fn tick(
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
    pub fn tick_in_environment(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        cells: &crate::content::NativeEntryMovementCells,
        actor: ExactActorRef,
        session: GameSessionId,
        now: SemanticTimeMicros,
    ) -> Option<(u64, ActorVitals)> {
        use oteryn_gameplay_foundation::foundation::condition::{TickFacts, TickKind};
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
    pub fn monk_save_values(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        now: SemanticTimeMicros,
    ) -> Option<(u8, u64)> {
        self.get(runtime, actor, game_session_id)?
            .monk_save_values(now)
    }

    /// CHAR-VITALS-1: the (health, mana) a last-position write saves for the committed player
    /// of `game_session_id`: `Some(None)` when both are at their maxima, and `None` when it has
    /// no spell state, so the caller keeps the vitals already stored rather than saving the
    /// maxima. A dead actor is never captured (its caller checks [`Self::is_dead`] first).
    pub fn saved_vitals(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
    ) -> Option<Option<(u32, u32)>> {
        self.get(runtime, actor, game_session_id)
            .map(PlayerSpellState::saved_vitals)
    }

    /// CHAR-VITALS-1: [`Self::initialize`] in the first-entry owner step, which also restores the
    /// `saved` (health, mana) of the Character's last-position row when this call creates the
    /// actor's state. A state that already existed (a retry, a same-GameSession reconnect) keeps
    /// its vitals exactly, as [`Self::initialize`] keeps them.
    #[allow(
        clippy::too_many_arguments,
        reason = "the initialize inputs plus the saved vitals of the admission"
    )]
    pub fn initialize_admitted(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        facts: CharacterCastFacts,
        monk: (u8, u64),
        now: SemanticTimeMicros,
        saved: Option<(u32, u32)>,
        rest: Option<[u32; 4]>,
    ) -> Option<&PlayerSpellState> {
        let created = self.index(actor, game_session_id).is_none();
        self.initialize(runtime, actor, game_session_id, facts, monk, now)?;
        let index = self.index(actor, game_session_id)?;
        if created && let Some((health, mana)) = saved {
            self.actors
                .get_mut(index)?
                .2
                .restore_saved_vitals(health, mana);
        }
        if created
            && let Some(rest) = rest
            && self
                .actors
                .get_mut(index)?
                .2
                .restore_bed_rest(rest, now.get())
                .is_none()
        {
            self.actors.remove(index);
            return None;
        }
        self.actors.get(index).map(|(_, _, state)| state)
    }

    /// Whether `actor` of `session` holds training state a forced checkpoint would write.
    pub fn has_training_to_flush(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
    ) -> bool {
        self.get(runtime, actor, game_session_id)
            .is_some_and(|state| state.has_training_to_flush())
    }

    /// Commit a resolved health credit under the Channel owner's runtime lock.
    /// The occurrence owner suppresses replay before calling this method.
    /// Zero/full-pool credit writes nothing; stale bindings and exhaustion refuse.
    pub fn apply_health_gain(
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
    pub fn apply_mana_gain(
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

    /// True while no spell commit is pending for the committed player and no durable spell holds
    /// the actor reserved: nothing then stages a successor of its spell state.
    pub(in crate::gameplay_transport) fn spell_idle(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
    ) -> bool {
        !self.has_pending_spell_commit(actor, game_session_id)
            && runtime.assert_actor_spell_unreserved(actor).is_ok()
    }

    /// Compare-commit the fed time of one eaten food (ITEM-USE-1c) as the direct successor
    /// revision. `None` when the actor is dead, stale, or eating is refused; nothing changes then.
    pub(in crate::gameplay_transport) fn apply_food(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        regeneration_seconds: u16,
        now_ms: u64,
    ) -> Option<u64> {
        if self.is_dead(actor) {
            return None;
        }
        let next = self
            .get(runtime, actor, game_session_id)?
            .after_food(regeneration_seconds, now_ms)?;
        let revision = next.revision();
        self.commit(runtime, actor, game_session_id, next)
            .then_some(revision)
    }

    /// DEATH-2 §4.2: true from the lethal write until the respawn. A dead player takes no input.
    pub fn is_dead(&self, actor: ExactActorRef) -> bool {
        self.deaths.iter().any(|(dead, _, _)| *dead == actor)
    }

    /// The death of the committed player of `game_session_id`, while it waits for its respawn.
    pub fn player_death(
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
    pub fn respawn(
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
    pub fn read_owned_player_state_test_snapshot(
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
pub const SPELL_RL_01_CAST_INPUTS_PER_WORK_CYCLE: usize = 1;

/// The owner's result of one cast: its disposition and, only when it committed, the actor's new
/// `ACTOR_VITALS` revision and value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellCastOutcome {
    pub disposition: SpellCastDisposition,
    pub vitals: Option<(u64, ActorVitals)>,
}

impl SpellCastOutcome {
    pub const fn rejected() -> Self {
        Self {
            disposition: SpellCastDisposition::Rejected,
            vitals: None,
        }
    }
}

/// The admitted actor's current `ACTOR_VITALS` revision and value, or `None` while it has no
/// Character cast facts.
pub fn observe_vitals(
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    actor: ExactActorRef,
    game_session_id: GameSessionId,
) -> Option<(u64, ActorVitals)> {
    let state = states.get(runtime, actor, game_session_id)?;
    Some((state.revision(), state.vitals()))
}

/// CONDITIONS-0 §3.5 at the transport: once the cast's spell is identified, a gate answers
/// `REJECTED` before every dispatcher pre-check and every SPELL-D1 `resolve_cast` check; only
/// the CHAT-0 §6 chat-mute pre-gate runs earlier. A gate in the player's spell state or in its
/// runtime condition slot refuses alike, as at attack admission. The refusal is the typed gate
/// in §3.5 precedence across both, kept server-side (§7); the caller answers `REJECTED`.
/// `None` lets the cast go on.
pub fn condition_gated_cast(
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    book: &SpellBook,
    actor: ExactActorRef,
    game_session_id: GameSessionId,
    intent: &SpellCastIntent,
    now: SemanticTimeMicros,
) -> Option<oteryn_gameplay_domain::domain::condition::ConditionGate> {
    let (spell, _) = book.source_indexed(intent.spell)?;
    let state = states.get(runtime, actor, game_session_id)?;
    let slot = runtime
        .actor_conditions(actor, Some(game_session_id))
        .ok()
        .and_then(|store| crate::spell::cast::store_condition_gate(store, spell, now).err());
    let own = crate::spell::cast::condition_gate(state, spell, now).err();
    own.into_iter().chain(slot).min()
}

/// One cast of `intent` by the admitted actor, as one Channel-owner work item under the runtime
/// lock the caller holds (and `states` beside it): read the actor's state, resolve, plan, and compare-commit the next state
/// with its anchor payment (SPELL-D3). `command_id` binds the occurrence and its draws, so the
/// outcome never depends on anything the client chooses beyond its intent. An actor without
/// Character cast facts, a stale binding or an unknown spell is `REJECTED` with no effect.
#[allow(clippy::too_many_arguments)]
pub fn cast_in_channel(
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

#[cfg(any(test, feature = "test-support"))]
#[allow(clippy::expect_used)]
#[path = "actor_spell_tests.rs"]
pub mod tests;
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
#[path = "actor_spell_source_native_vitals_tests.rs"]
mod source_native_vitals_tests;
pub use crate::monster_combat_lane::native_ticks::{NativeOwnedPlayerTickReceipt, PlayerDeath};
impl ChannelSpellStates {
    /// Independently qualified map/field facts are supplied by the same source owner turn.
    /// Creature provenance is frozen by native source application, never reacquired from a
    /// dead caster. Player/unknown provenance must use the separate actual PvP legality owner.
    #[allow(
        dead_code,
        reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
    )]
    pub fn tick_source_player_conditions(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
        _occurrence: DecisionOccurrenceId,
        facts: oteryn_gameplay_foundation::foundation::condition::TickFacts,
        now: crate::foundation::owner_timer::SemanticTimeMicros,
    ) -> Option<NativeOwnedPlayerTickReceipt> {
        use oteryn_gameplay_foundation::foundation::condition::{ConditionSourceKind, TickKind};
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
                // CONDITIONS-0 §3.5: the creature provenance of `Damage`; the mana shield never
                // absorbs agony. A refused occurrence was used up in the protection zone.
                TickKind::Agony { amount, refused } => {
                    if !refused {
                        if tick.provenance.source_kind != ConditionSourceKind::Creature
                            || !tick
                                .provenance
                                .source
                                .as_deref()
                                .is_some_and(|s| s.starts_with("creature:"))
                        {
                            return None;
                        }
                        let (successor, hit) = crate::spell::actor_conditions::stage_agony_hit(
                            &next,
                            amount,
                            now.get(),
                        )
                        .ok()?;
                        damage = damage.checked_add(hit.applied)?;
                        next = successor;
                    }
                }
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
#[path = "actor_spell_owned_source_tick_tests.rs"]
mod owned_source_tick_tests;

#[cfg(test)]
impl ChannelSpellStates {
    /// Component fixture seam inside the real owner: exact live creature provenance,
    /// native Player condition staging and existing session-checked CAS publication.
    /// This creates no production grant or mutable PlayerSpellState accessor.
    pub fn install_owned_source_condition_fixture(
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
    pub fn owned_source_condition_fixture_snapshot(
        &self,
        runtime: &ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
    ) -> Option<oteryn_gameplay_foundation::foundation::condition::ConditionStore<String>> {
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
    pub fn tick_source_creature_conditions(
        &self,
        runtime: &mut ChannelRuntimeV1,
        target: ExactActorRef,
        facts: crate::foundation::TickFacts,
        now: crate::foundation::owner_timer::SemanticTimeMicros,
    ) -> Result<
        crate::foundation::CreaturePeriodicReceipt,
        oteryn_gameplay_domain::domain::carrier_values::CarrierError,
    > {
        let mut lookup = |r: &ChannelRuntimeV1, source: ExactActorRef, at: u64| {
            if !r.borrow_exact_actor_lookup().contains(source) {
                return Ok(None);
            }
            let position = r.read_actor_position(source)?;
            if position.context() != r.pinned_movement_context() {
                return Err(oteryn_gameplay_domain::domain::carrier_values::CarrierError::PositionContextMismatch);
            }
            let Some((_, session, state)) = self.actors.iter().find(|(a, _, _)| *a == source)
            else {
                // An extant live actor with no canonical state is UNKNOWN, not
                // a positive no-attacker fact. A retired exact generation is absent.
                return Err(
                    oteryn_gameplay_domain::domain::carrier_values::CarrierError::PlanConflict,
                );
            };
            let current = r.player_control_facts(source, *session)?;
            if current.control_loss.is_some() || self.is_dead(source) || state.vitals().health == 0
            {
                return Ok(None);
            }
            if !state.owned_conditions().accepts_time(at) {
                return Err(
                    oteryn_gameplay_domain::domain::carrier_values::CarrierError::PlanConflict,
                );
            }
            if self.has_pending_spell_commit(source, *session) {
                return Err(
                    oteryn_gameplay_domain::domain::carrier_values::CarrierError::PlanConflict,
                );
            }
            r.assert_actor_spell_unreserved(source)?;
            use oteryn_gameplay_domain::domain::condition::ConditionValues;
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
#[path = "actor_spell_source_creature_dot_owner_tests.rs"]
mod source_creature_dot_owner_tests;

#[derive(Debug)]
pub(in crate::gameplay_transport) struct CreatureFieldContact {
    pub(in crate::gameplay_transport) source: String,
    pub(in crate::gameplay_transport) definition:
        oteryn_gameplay_foundation::foundation::condition::ConditionDefinition,
    pub(in crate::gameplay_transport) element:
        oteryn_gameplay_domain::domain::condition::DotElement,
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
        blocking: &dyn crate::map::floor::CellBlocker,
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
        let facts = oteryn_gameplay_foundation::foundation::condition::ApplicationFacts {
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
