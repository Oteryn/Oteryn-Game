//! ATTACK-1b: the Channel owner's attack-target state and auto-attack swing timer (ATTACK-0 §3,
//! §4, §9; `docs/architecture/reviews/OTERYN_GAME_ATTACK0_ATTACK_TARGET_AND_AUTO_ATTACK_DECISION_2026-09-30.md`).
//!
//! One entry per (player actor, session): the held creature target, the runtime-only fight modes
//! and the in-fight deadline. A due swing is one `AutoAttack` GAME-ABILITY-01 damage plan
//! committed by the physical owner under the swing identity `(character, lease generation,
//! session, lineage, swing ordinal)`: the lineage is the `CommandRef` of the `ATTACK_TARGET`
//! command that set the target and the ordinal counts the swings of that lineage from 0.
//! Player targets (PvP), distance weapons and creature defence/armor are out of scope.
//!
//! CHASE-1 (RANGED-0 §8): with `chase=CHASE` and a held target the owner steps the player towards
//! the nearest tile next to it, one Movement-owner step at a time on the player's own pacing
//! clock, along a bounded path search (`combat_attack::chase`). A client step wins: it drops the
//! route and chase re-paths at its next 1,000 ms check. `STAND` never moves the player. `secure`
//! has no creature-target effect; its PvP refusal waits for player targets (PVP-RT-1).

use crate::foundation::encode_state_delta;
use oteryn_protocol_oteryn::attack::{
    ActorCombatState, AttackIntentDisposition, ChaseMode, DELTA_TYPE_ACTOR_COMBAT_STATE_V1,
    FightMode as WireFightMode, FightModes, MAX_ATTACK_TARGET_CHANGES_PER_SECOND,
    MAX_FIGHT_MODE_CHANGES_PER_SECOND, STATE_DOMAIN_ACTOR_COMBAT_STATE, encode_actor_combat_state,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, SemanticTimeMicros, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Weak};
use tokio::time::{Duration, Instant};

use super::ComposedFreshAdmission;
use crate::ability::exact_actor_resolution::{ExactActorProposal, resolve_exact_actor};
use crate::ability::{
    AbilityIntent, AbilityOccurrence, CommitGroup, Effect, EffectPlan, ProposalSource, RevisionSet,
};
use crate::combat::charm_effects::{
    CharmAssignment, CharmAttackerFacts, CharmCatalogueRead, CharmEvaluationInput, CharmHitSource,
    CharmHookEvent, CharmStateRead, CharmStateUnavailable, evaluate_charm_hook,
};
use crate::combat_attack::{
    AttackConstants, AttackState, CHASE_WINDOW_MICROS, CREATURE_COST, ChaseDecision,
    ChaseSearchBudget, ChaseState, FightMode, SwingPoll, TargetFacts, TargetRefusal,
    player_fist_formula,
};
use crate::content::{LogicalCell, QualifiedNativeEntryRoom};
use crate::foundation::{
    ChannelRuntimeV1, CommandId, CommandRef, ExactActorRef, GameSessionId, MovementLocalPosition,
};
use crate::movement::CardinalStep;
use crate::movement::interest::{VisibilityPosition, VisibilitySettings};
use crate::spell::combat_execution::actor_atom;
use crate::spell::formula::FormulaInputs;

/// `ATTACK0-RL-01` and `ATTACK0-RL-02`: intents per [`ATTACK_INTENT_WINDOW`].
const MAX_INTENTS_PER_WINDOW: usize = 25;
const _: () = assert!(
    MAX_INTENTS_PER_WINDOW as u32 == MAX_ATTACK_TARGET_CHANGES_PER_SECOND
        && MAX_INTENTS_PER_WINDOW as u32 == MAX_FIGHT_MODE_CHANGES_PER_SECOND
);
pub(crate) const ATTACK_INTENT_WINDOW: Duration = Duration::from_secs(1);
/// How often a connection with domain 10 runs the owner's swing timer and looks for a changed
/// combat state.
pub(crate) const COMBAT_REFRESH: Duration = Duration::from_millis(250);

/// The sliding window of one intent limit: the instants of the last intents. `Copy` so it
/// travels with the session continuity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct IntentWindow([Option<Instant>; MAX_INTENTS_PER_WINDOW]);

impl IntentWindow {
    pub(crate) const EMPTY: Self = Self([None; MAX_INTENTS_PER_WINDOW]);

    /// Whether one more intent fits the window ending at `now`; if so it is recorded.
    pub(crate) fn admit(&mut self, now: Instant) -> bool {
        let free = self.0.iter_mut().find(|slot| {
            !matches!(slot, Some(at) if now.saturating_duration_since(*at) < ATTACK_INTENT_WINDOW)
        });
        let Some(slot) = free else {
            return false;
        };
        *slot = Some(now);
        true
    }
}

/// What one GameSession keeps of domain 10 across connections: the revision (monotonic per
/// GameSession), the last state sent and the two intent windows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CombatContinuity {
    pub(crate) revision: u64,
    pub(crate) sent: Option<ActorCombatState>,
    pub(crate) target_intents: IntentWindow,
    pub(crate) mode_intents: IntentWindow,
}

impl CombatContinuity {
    pub(crate) const FRESH: Self = Self {
        revision: 0,
        sent: None,
        target_intents: IntentWindow::EMPTY,
        mode_intents: IntentWindow::EMPTY,
    };

    /// The domain 10 snapshot above every revision this session has seen; recorded only by
    /// [`Self::snapshot_sent`] once transmitted.
    pub(crate) fn snapshot(&self, state: &ActorCombatState) -> Option<(u64, Vec<u8>)> {
        let revision = self.revision.checked_add(1)?;
        Some((revision, encode_actor_combat_state(state).ok()?))
    }

    pub(crate) fn snapshot_sent(&mut self, revision: u64, state: ActorCombatState) {
        self.revision = revision;
        self.sent = Some(state);
    }

    /// The domain 10 delta frame to `state` at the sequence after `sequence` when it differs
    /// from the last state sent; `Some(None)` when unchanged, `None` on a fault. The revision
    /// advances before the write, so a revision that may have reached the client is never
    /// reused.
    pub(crate) fn delta(
        &mut self,
        generation: u64,
        sequence: u64,
        state: ActorCombatState,
    ) -> Option<Option<(u64, Vec<u8>)>> {
        if self.sent == Some(state) {
            return Some(None);
        }
        let from = self.revision;
        let to = from.checked_add(1)?;
        let delta_sequence = sequence.checked_add(1)?;
        let payload = encode_actor_combat_state(&state).ok()?;
        let frame = encode_state_delta(
            generation,
            delta_sequence,
            STATE_DOMAIN_ACTOR_COMBAT_STATE,
            from,
            to,
            DELTA_TYPE_ACTOR_COMBAT_STATE_V1,
            &payload,
        )
        .ok()?;
        self.revision = to;
        self.sent = Some(state);
        Some(Some((delta_sequence, frame)))
    }
}

/// The attack state of one player actor in one session.
#[derive(Debug, Clone)]
struct ActorAttack {
    actor: ExactActorRef,
    session: GameSessionId,
    state: AttackState<ExactActorRef, CommandRef>,
    modes: FightModes,
    /// The wire reference of the held target, as the client named it.
    target_ref: Option<EntityRef>,
    /// The swing ordinal of the next swing of the held target's lineage.
    next_ordinal: u16,
    /// CHASE-1: the chase route towards the held target (runtime-only).
    chase: ChaseState,
}

impl ActorAttack {
    fn clear_target(&mut self) {
        self.state.clear_target();
        self.target_ref = None;
        self.chase.reset();
    }
}

/// CHASE-1: how soon a step that found its actor's clock held by another connection's step in
/// flight asks again.
const STEP_IN_FLIGHT_RETRY: Duration = Duration::from_millis(50);

/// CHASE-1 (RANGED-0 §8): a player actor's pacing clock.
#[derive(Debug)]
enum StepClock {
    /// A claimed step is in flight. It holds the clock until its claimer records the step's
    /// end, or the claimer's connection is gone (its holder no longer upgrades), never by time.
    InFlight(Weak<()>),
    /// The next step is due at this instant.
    Due(Instant),
}

/// The Channel owner's attack entries. Locked after `runtime` and `spell_states`, never before.
#[derive(Debug, Default)]
pub(crate) struct ChannelAttackStates {
    entries: Vec<ActorAttack>,
    /// The Channel's unsettled creature kills (ARCH-KILL-REWARD-LOGOUT-1 §1.3).
    kills: super::kill_reward::KillSettlementQueue,
    /// CHASE-1: the Channel's player chase searches in the current owner window
    /// (`RANGED0-RL-03`).
    chase_searches: ChaseSearchBudget,
    /// CHASE-1 (RANGED-0 §8): each player actor's pacing clock, owned by the actor rather than
    /// its connection, so a client step and a chase step share one clock and a replaced or
    /// resumed connection keeps it. Only held clocks are kept.
    step_clocks: Vec<(ExactActorRef, StepClock)>,
}

impl ChannelAttackStates {
    pub(crate) const fn kills(&self) -> &super::kill_reward::KillSettlementQueue {
        &self.kills
    }

    /// CHASE-1: when `actor`'s pacing clock may next be claimed, if it is still held at `now`.
    pub(crate) fn step_deadline(&self, actor: ExactActorRef, now: Instant) -> Option<Instant> {
        self.step_clocks
            .iter()
            .find(|(owner, _)| *owner == actor)
            .and_then(|(_, clock)| match clock {
                StepClock::InFlight(holder) => {
                    (holder.strong_count() > 0).then(|| now + STEP_IN_FLIGHT_RETRY)
                }
                StepClock::Due(due) => (now < *due).then_some(*due),
            })
    }

    /// CHASE-1: claim `actor`'s pacing clock at `now` for one step of the connection that owns
    /// `holder`, in one check-and-set, so a replaced connection cannot step between another's
    /// commit and its recorded deadline. `Some` with when to ask again while the clock is held;
    /// otherwise the claim holds it until [`Self::record_step_deadline`] with the same holder.
    pub(crate) fn begin_step(
        &mut self,
        actor: ExactActorRef,
        holder: &Arc<()>,
        now: Instant,
    ) -> Option<Instant> {
        if let Some(due) = self.step_deadline(actor, now) {
            return Some(due);
        }
        self.step_clocks.retain(|(owner, _)| *owner != actor);
        self.step_clocks
            .push((actor, StepClock::InFlight(Arc::downgrade(holder))));
        None
    }

    /// CHASE-1: the step `holder` claimed ended at `now`; `actor`'s pacing clock is due at
    /// `due`, or free with `None`. A record without the matching claim changes nothing.
    pub(crate) fn record_step_deadline(
        &mut self,
        actor: ExactActorRef,
        holder: &Arc<()>,
        now: Instant,
        due: Option<Instant>,
    ) {
        let claimed = Arc::downgrade(holder);
        let Some(index) = self.step_clocks.iter().position(|(owner, clock)| {
            *owner == actor && matches!(clock, StepClock::InFlight(kept) if kept.ptr_eq(&claimed))
        }) else {
            return;
        };
        self.step_clocks.swap_remove(index);
        self.step_clocks.retain(|(_, clock)| match clock {
            StepClock::InFlight(kept) => kept.strong_count() > 0,
            StepClock::Due(kept) => now < *kept,
        });
        if let Some(due) = due.filter(|due| now < *due) {
            self.step_clocks.push((actor, StepClock::Due(due)));
        }
    }
}

/// Why an `ATTACK_TARGET` intent was refused before any owner state changed.
fn refusal_disposition(refusal: TargetRefusal) -> AttackIntentDisposition {
    match refusal {
        TargetRefusal::NotVisible | TargetRefusal::Dead => {
            AttackIntentDisposition::TargetNotVisible
        }
        TargetRefusal::NotAttackableKind => AttackIntentDisposition::TargetNotACreature,
        TargetRefusal::TargetInProtectionZone | TargetRefusal::AttackerInProtectionZone => {
            AttackIntentDisposition::ProtectionZone
        }
        TargetRefusal::ReentryProtected | TargetRefusal::TargetReentryProtected => {
            AttackIntentDisposition::ReentryProtected
        }
    }
}

const fn fight_mode(mode: WireFightMode) -> FightMode {
    match mode {
        WireFightMode::Offensive => FightMode::Offensive,
        WireFightMode::Balanced => FightMode::Balanced,
        WireFightMode::Defensive => FightMode::Defensive,
    }
}

fn in_protection_zone(
    room: &QualifiedNativeEntryRoom,
    position: MovementLocalPosition,
) -> Option<bool> {
    let cells = room.movement_cells();
    cells
        .spell_tiles()
        .lookup(
            cells.scope(),
            LogicalCell {
                x: position.x,
                y: position.y,
                z: i32::from(position.floor),
            },
        )
        .ok()
        .map(|tile| tile.flags().protection_zone)
}

/// Whether an attacker at `from` sees `to` under the reference view. A source world's native
/// floors can lie outside the 0..=15 visibility floors (Thalom is on -8..=-5); there a target on
/// the attacker's own floor is judged on the same plane, and any other floor is not seen.
pub(super) fn sees(from: MovementLocalPosition, to: MovementLocalPosition) -> bool {
    const PLANE: i16 = 7;
    let view = |position: MovementLocalPosition, floor: i16| {
        VisibilityPosition::new(position.x, position.y, floor).ok()
    };
    let (observer, seen) = match (view(from, from.floor), view(to, to.floor)) {
        (Some(observer), Some(seen)) => (Some(observer), Some(seen)),
        _ if from.floor == to.floor => (view(from, PLANE), view(to, PLANE)),
        _ => (None, None),
    };
    match (observer, seen) {
        (Some(observer), Some(seen)) => VisibilitySettings::REFERENCE.can_see(observer, seen),
        _ => false,
    }
}

/// The §3/§4 facts of `target` for `attacker` at `now`. `None` when the attacker's own facts
/// cannot be read; the caller fails closed.
fn target_facts(
    runtime: &ChannelRuntimeV1,
    room: &QualifiedNativeEntryRoom,
    attacker: ExactActorRef,
    session: GameSessionId,
    target: ExactActorRef,
    now: SemanticTimeMicros,
) -> Option<TargetFacts> {
    let attacker_snapshot = runtime.read_actor_position(attacker).ok()?;
    let from = attacker_snapshot.position();
    let attacker_in_protection_zone = in_protection_zone(room, from)?;
    let attacker_reentry_protected = runtime
        .current_player_reentry_protection(attacker, session, now.get())
        .ok()?;
    let absent = TargetFacts {
        present_and_visible: false,
        attackable_kind: false,
        alive: false,
        same_floor: false,
        distance: u32::MAX,
        attacker_in_protection_zone,
        target_in_protection_zone: false,
        attacker_reentry_protected,
        target_reentry_protected: false,
    };
    if !runtime.contains_live_creature(target) {
        return Some(absent);
    }
    let Ok(target_snapshot) = runtime.read_actor_position(target) else {
        return Some(absent);
    };
    let to = target_snapshot.position();
    let visible = target_snapshot.context() == attacker_snapshot.context() && sees(from, to);
    let (attackable_kind, alive) = match runtime.companion_snapshot(target) {
        Ok(snapshot) => (
            snapshot.state.master.is_none() && !snapshot.state.policy.is_familiar,
            snapshot.health > 0,
        ),
        Err(_) => (true, false),
    };
    let dx = (i64::from(to.x) - i64::from(from.x)).unsigned_abs();
    let dy = (i64::from(to.y) - i64::from(from.y)).unsigned_abs();
    Some(TargetFacts {
        present_and_visible: visible,
        attackable_kind,
        alive,
        same_floor: to.floor == from.floor,
        distance: u32::try_from(dx.max(dy)).unwrap_or(u32::MAX),
        attacker_in_protection_zone,
        target_in_protection_zone: in_protection_zone(room, to)?,
        attacker_reentry_protected,
        target_reentry_protected: false,
    })
}

impl ChannelAttackStates {
    fn index(&self, actor: ExactActorRef, session: GameSessionId) -> Option<usize> {
        self.entries
            .iter()
            .position(|entry| entry.actor == actor && entry.session == session)
    }

    /// Drop the entries whose actor is no longer controlled by its session's slot.
    fn prune(&mut self, runtime: &ChannelRuntimeV1) {
        self.entries.retain(|entry| {
            runtime
                .player_control_facts(entry.actor, entry.session)
                .is_ok()
        });
    }

    /// The entry of a currently placed `actor` in `session`, created on first use.
    fn entry(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<&mut ActorAttack> {
        runtime.player_control_facts(actor, session).ok()?;
        let index = match self.index(actor, session) {
            Some(index) => index,
            None => {
                self.prune(runtime);
                // SPAWN-CEILINGS-1 (Codex 4242381774): one entry per controlled player slot,
                // which `prune` keeps, so the carrier's slots bound it; the old
                // `AI01-ACTIVE-ACTORS` 256 refused the 257th player of a Channel built for 500.
                if self.entries.len() >= runtime.actor_slot_capacity() {
                    return None;
                }
                self.entries.push(ActorAttack {
                    actor,
                    session,
                    state: AttackState::default(),
                    modes: FightModes::DEFAULT,
                    target_ref: None,
                    next_ordinal: 0,
                    chase: ChaseState::default(),
                });
                self.entries.len() - 1
            }
        };
        self.entries.get_mut(index)
    }

    /// One `ATTACK_TARGET` intent (§3). `None` stops attacking. A new target starts the lineage
    /// of command `command_id` at ordinal 0; a refused target leaves the held one unchanged.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn set_target(
        &mut self,
        runtime: &ChannelRuntimeV1,
        room: Option<&QualifiedNativeEntryRoom>,
        actor: ExactActorRef,
        session: GameSessionId,
        now: SemanticTimeMicros,
        target: Option<EntityRef>,
        command_id: u64,
    ) -> AttackIntentDisposition {
        let Some(entry) = self.entry(runtime, actor, session) else {
            return AttackIntentDisposition::Rejected;
        };
        let Some(target_ref) = target else {
            entry.clear_target();
            return AttackIntentDisposition::Ok;
        };
        let Some(room) = room else {
            return AttackIntentDisposition::Rejected;
        };
        let Ok(command) = CommandId::new(command_id) else {
            return AttackIntentDisposition::Rejected;
        };
        let visible = runtime.visible_entities();
        let named = |actor: ExactActorRef, generation: u64| {
            actor.placement_identity() == target_ref.identity && generation == target_ref.generation
        };
        if visible
            .players
            .iter()
            .any(|player| named(player.actor, player.generation))
        {
            return AttackIntentDisposition::TargetNotACreature;
        }
        let Some(creature) = visible
            .creatures
            .iter()
            .find(|creature| named(creature.actor, creature.generation))
            .map(|creature| creature.actor)
        else {
            return AttackIntentDisposition::TargetNotVisible;
        };
        let Some(facts) = target_facts(runtime, room, actor, session, creature, now) else {
            return AttackIntentDisposition::Rejected;
        };
        match entry
            .state
            .set_target(creature, CommandRef::new(session, command), &facts)
        {
            Ok(()) => {
                entry.target_ref = Some(target_ref);
                entry.next_ordinal = 0;
                entry.chase.reset();
                AttackIntentDisposition::Ok
            }
            Err(refusal) => refusal_disposition(refusal),
        }
    }

    /// One `FIGHT_MODES` intent: runtime-only, never persisted (§9).
    pub(crate) fn set_modes(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        modes: FightModes,
    ) -> bool {
        match self.entry(runtime, actor, session) {
            Some(entry) => {
                if modes.chase != ChaseMode::Chase {
                    entry.chase.reset();
                }
                entry.modes = modes;
                true
            }
            None => false,
        }
    }

    /// The `ACTOR_COMBAT_STATE` of `actor` at `now`; the defaults when it holds no entry.
    pub(crate) fn combat_state(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        now: SemanticTimeMicros,
    ) -> ActorCombatState {
        match self.index(actor, session).map(|index| &self.entries[index]) {
            Some(entry) => ActorCombatState {
                target: entry.target_ref,
                modes: entry.modes,
                in_fight: entry.state.in_fight(now),
            },
            None => ActorCombatState {
                target: None,
                modes: FightModes::DEFAULT,
                in_fight: false,
            },
        }
    }

    /// The running in-fight deadline of `actor` (`ATTACK0-RL-03`): a logout is refused and a
    /// closed client stays in the world until it ends.
    pub(crate) fn in_fight_until(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        now: SemanticTimeMicros,
    ) -> Option<SemanticTimeMicros> {
        let entry = &self.entries[self.index(actor, session)?];
        entry
            .state
            .in_fight(now)
            .then(|| entry.state.in_fight_until())
            .flatten()
    }

    /// Stop attacking (reconnect, control loss); the in-fight deadline is kept (§3 Reconnect).
    pub(crate) fn clear_target(&mut self, actor: ExactActorRef, session: GameSessionId) {
        if let Some(index) = self.index(actor, session) {
            self.entries[index].clear_target();
        }
    }

    /// Whether a creature may resolve `actor` as its melee target, and if so whether re-entry
    /// protection covers it. A closed client held in the world by its in-fight deadline
    /// (ATTACK0-RL-03) stays a target with no re-entry window, so a hit extends the hold; any
    /// other actor under control loss is no target.
    pub(crate) fn creature_target_protection(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        now: SemanticTimeMicros,
    ) -> Option<bool> {
        match runtime.current_player_reentry_protection(actor, session, now.get()) {
            Ok(protected) => Some(protected),
            Err(_) => {
                runtime
                    .player_control_facts(actor, session)
                    .ok()?
                    .control_loss?;
                self.in_fight_until(actor, session, now)?;
                Some(false)
            }
        }
    }

    /// A creature hit `actor`: the in-fight deadline runs from `now`.
    pub(crate) fn record_hit_taken(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        now: SemanticTimeMicros,
    ) {
        let Some(constants) = AttackConstants::checked_in() else {
            return;
        };
        if let Some(entry) = self.entry(runtime, actor, session) {
            entry.state.record_hit(now, constants.in_fight_micros());
        }
    }
}

impl ChannelAttackStates {
    /// CHASE-1 (RANGED-0 §8): the step the chase of `actor` takes now, if any. Chase runs only
    /// with `chase=CHASE`, a held target that is alive, visible and on the attacker's floor, and
    /// no re-entry protection on the attacker; otherwise its route is dropped. `path_tile` says
    /// whether a cell is a path tile; a creature on it costs [`CREATURE_COST`] more.
    pub(crate) fn chase_decision(
        &mut self,
        runtime: &ChannelRuntimeV1,
        room: &QualifiedNativeEntryRoom,
        actor: ExactActorRef,
        session: GameSessionId,
        now: SemanticTimeMicros,
        path_tile: impl Fn(MovementLocalPosition) -> bool,
    ) -> Option<CardinalStep> {
        let index = self.index(actor, session)?;
        let Self {
            entries,
            chase_searches,
            ..
        } = self;
        let entry = &mut entries[index];
        let chasing = entry.modes.chase == ChaseMode::Chase;
        let Some(target) = entry.state.target().filter(|_| chasing) else {
            entry.chase.reset();
            return None;
        };
        let positions = runtime
            .read_actor_position(actor)
            .ok()
            .zip(runtime.read_actor_position(target).ok())
            .map(|(from, to)| (from.position(), to.position()));
        let facts = target_facts(runtime, room, actor, session, target, now);
        // The target stays held here, so only its route goes (RANGED-0 §8, `RANGED0-RL-08`).
        let (Some((from, to)), Some(facts)) = (positions, facts) else {
            entry.chase.hold();
            return None;
        };
        if !(facts.present_and_visible && facts.alive && facts.same_floor)
            || facts.attacker_reentry_protected
        {
            entry.chase.hold();
            return None;
        }
        // RANGED0-RL-03: a due search first registers in its owner window and is admitted from
        // the next one, so every search due in one window is known before any of them is
        // admitted; those that waited from an earlier window, or from the same one with a lower
        // actor id, keep their places.
        let window = now.get() / CHASE_WINDOW_MICROS;
        let registered = entry
            .chase
            .search_waiting_since()
            .map(|since| since / CHASE_WINDOW_MICROS);
        let own = (registered.unwrap_or(window), actor_order(actor));
        let ahead = entries
            .iter()
            .filter(|other| other.actor != actor || other.session != session)
            .filter_map(|other| {
                let since = other.chase.search_waiting_since()? / CHASE_WINDOW_MICROS;
                ((since, actor_order(other.actor)) < own).then_some(())
            })
            .count();
        let admissible = registered.is_some_and(|since| since < window);
        let entry = &mut entries[index];
        // CREATURE-AI-0 §5.3: +40 over a tile another actor stands on (a creature, an NPC or
        // another player; never the chasing player), and the target's own tile is no path tile.
        let mut occupied: Option<std::collections::BTreeSet<(i32, i32)>> = None;
        let decision = entry.chase.decide(
            now.get(),
            from,
            to,
            || admissible && chase_searches.admit(now.get(), ahead),
            |cell| {
                if cell == to || !path_tile(cell) {
                    return None;
                }
                let occupied = occupied.get_or_insert_with(|| {
                    let visible = runtime.visible_entities();
                    visible
                        .creatures
                        .iter()
                        .chain(&visible.npcs)
                        .chain(&visible.players)
                        .filter(|other| other.actor != actor && other.position.floor == from.floor)
                        .map(|other| (other.position.x, other.position.y))
                        .collect()
                });
                Some(if occupied.contains(&(cell.x, cell.y)) {
                    CREATURE_COST
                } else {
                    0
                })
            },
        );
        match decision {
            ChaseDecision::Step(step) => Some(step),
            ChaseDecision::Hold => None,
        }
    }

    /// CHASE-1: a client step wins over the chase; its route is dropped and it re-paths at its
    /// next 1,000 ms check.
    pub(crate) fn chase_yield_to_client(
        &mut self,
        actor: ExactActorRef,
        session: GameSessionId,
        now: SemanticTimeMicros,
    ) {
        if let Some(index) = self.index(actor, session) {
            self.entries[index].chase.yield_to_client(now.get());
        }
    }

    /// CHASE-1: whether `actor`'s chase has a due search the Channel budget has not admitted
    /// yet (`RANGED0-RL-03`), which its connection retries in the next owner window.
    pub(crate) fn chase_search_waiting(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> bool {
        self.index(actor, session)
            .is_some_and(|index| self.entries[index].chase.search_waiting_since().is_some())
    }

    /// CHASE-1: the chase step the Movement owner just refused at `now` drops the route. When it
    /// refused it as `blocked`, the refused cell also stays out of the next searches; any other
    /// refusal is transient and leaves the cell a path tile.
    pub(crate) fn chase_step_refused(
        &mut self,
        actor: ExactActorRef,
        session: GameSessionId,
        now: SemanticTimeMicros,
        blocked: bool,
    ) {
        if let Some(index) = self.index(actor, session) {
            let chase = &mut self.entries[index].chase;
            if blocked {
                chase.step_blocked(now.get());
            } else {
                chase.step_refused();
            }
        }
    }
}

/// CHASE-1: the actor-id order of `RANGED0-RL-03` within one Channel.
const fn actor_order(actor: ExactActorRef) -> (u32, u64) {
    (actor.actor_local_id(), actor.actor_local_generation())
}

/// CHASE-1: whether a player chase may path over `cell` (CREATURE-AI-0 §5.3 with RANGED-0 §8). It
/// is a Walkable cell of the Movement owner's cells, no closed door blocks it, and it is no
/// teleport, protection-zone or floor-change tile: a chase never changes floors. Unknown tile
/// facts refuse, except on the accepted entry room without a tile document, whose Movement owner
/// admits a step on terrain and doors alone.
pub(super) fn chase_path_tile(
    cells: &crate::content::NativeEntryMovementCells,
    blocking: &dyn crate::map::floor::CellBlocker,
    cell: MovementLocalPosition,
) -> bool {
    let target = LogicalCell {
        x: cell.x,
        y: cell.y,
        z: i32::from(cell.floor),
    };
    let ordinary = match cells.spell_tiles().lookup(cells.scope(), target) {
        Ok(tile) => !tile.flags().protection_zone && !tile.flags().floor_change,
        Err(crate::content::SpellTileLookupError::Unknown) => {
            matches!(
                cells.index(),
                crate::content::native_cell_lookup::NativeMovementCollisionIndex::Entry(_)
            ) && !crate::movement::source_floor_change::is_source_profile(cells)
        }
        Err(_) => false,
    };
    ordinary
        && cells.index().lookup(cells.scope(), target)
            == Ok(crate::content::CollisionClass::Walkable)
        && !blocking.blocks(target)
        && !crate::map::floor::is_teleport(cells, target)
}

/// ATTACK-1b has no Charm state reader: every hook is evaluated against no assignments and its
/// outcomes are discarded. The Charm state wiring is a later stage.
struct NoCharmAssignments([u8; 16]);

impl CharmStateRead for NoCharmAssignments {
    fn character(&self) -> [u8; 16] {
        self.0
    }

    fn assignments_for_race(
        &self,
        _race_key: &str,
    ) -> Result<Vec<CharmAssignment>, CharmStateUnavailable> {
        Ok(Vec::new())
    }
}

/// Evaluate one Charm hook and discard its outcomes (see [`NoCharmAssignments`]).
pub(super) fn run_charm_hook(
    catalogue: &dyn CharmCatalogueRead,
    character: [u8; 16],
    race_key: &str,
    decision_root: &GameplayDecisionRoot,
    occurrence: DecisionOccurrenceId,
    event: CharmHookEvent,
) {
    let _ = evaluate_charm_hook(
        &CharmEvaluationInput {
            character,
            race_key,
            decision_root,
            occurrence,
            event,
        },
        &NoCharmAssignments(character),
        catalogue,
    );
}

/// The decision occurrence of one swing event: unique per attacker, lineage, ordinal and tag.
fn swing_occurrence(
    attacker: ExactActorRef,
    lineage: CommandRef,
    ordinal: u16,
    tag: &[u8],
) -> DecisionOccurrenceId {
    let hash = Sha256::new()
        .chain_update(b"oteryn:auto-attack-swing:v1")
        .chain_update(attacker.placement_identity())
        .chain_update(lineage.game_session_id().as_bytes())
        .chain_update(lineage.command_id().get().to_be_bytes())
        .chain_update(ordinal.to_be_bytes())
        .chain_update(tag)
        .finalize();
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&hash[..16]);
    DecisionOccurrenceId::from_bytes(bytes)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The swing as one Ability plan: a client-proposed damage effect on the held target,
/// committed atomically by the Channel owner under the swing's identity. The target atom is the
/// creature slot's `target_identity` (its definition key), which the owner commit compares.
fn swing_plan(
    attacker: ExactActorRef,
    lineage: CommandRef,
    ordinal: u16,
    magnitude: i64,
    content_digest: &[u8; 32],
    target_identity: &str,
) -> Option<EffectPlan> {
    let issuer = actor_atom(attacker);
    let target_atom = target_identity.to_owned();
    let occurrence_id = format!(
        "auto-attack:{}:{}:{}:{ordinal}",
        hex(&attacker.placement_identity()),
        hex(lineage.game_session_id().as_bytes()),
        lineage.command_id().get(),
    );
    let revisions = RevisionSet::new(
        "ruleset:attack-v1",
        &format!("content:{}", hex(content_digest)),
        "world:attack-v1",
        "formula:player-fist",
        "simulation:v1",
    )
    .ok()?;
    let occurrence = AbilityOccurrence::new(&occurrence_id, revisions).ok()?;
    let proposal = AbilityIntent::resolve(
        ProposalSource::Client,
        &issuer,
        &[&target_atom],
        &[&target_atom],
    )
    .ok()?;
    let effect = Effect::damage(&target_atom, magnitude).ok()?;
    let group = CommitGroup::atomic("channel-owner", &occurrence_id).ok()?;
    EffectPlan::immediate(occurrence, proposal, vec![effect], Vec::new(), group).ok()
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// CHASE-1 (RANGED-0 §8): the chase step of `actor` due now, if any, taken as one ordinary
    /// paced step through the Movement owner, which revalidates it like a client step. A step
    /// that does not move drops the route. `None` when the chase takes no step.
    pub(in crate::gameplay_transport) async fn chase_step_now(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<(super::StepOutcome, Option<Duration>)> {
        use super::FreshAdmissionAuthority as _;
        let room = self.combat_room?;
        let step = {
            // Lock order: runtime, spell states, door, then attack.
            let runtime = self.runtime.lock().await;
            let states = self.spell_states.lock().await;
            // DEATH-2 §4.2: a dead player takes no input, the chase included.
            if states.is_dead(actor) {
                return None;
            }
            // The same closed doors a player step reads (`paced_step`).
            let (none, bundle_doors, legacy);
            let blocking: &dyn crate::map::floor::CellBlocker = if self
                .qualified_room
                .is_some_and(|room| room.source_world().is_some())
            {
                none = std::collections::BTreeSet::<LogicalCell>::new();
                &none
            } else if let Some(map) = self.bundle_map() {
                bundle_doors = crate::map::floor::BundleDoors(map.facts.doors());
                &bundle_doors
            } else {
                legacy = self.door.lock().await.blocking_cells().clone();
                &legacy
            };
            let cells = self.movement_cells;
            let mut attack = self.attack.lock().await;
            // Owner time is read once the locks that serialize chase decisions are held, so the
            // Channel search budget sees its windows in order (`RANGED0-RL-03`).
            let now = self.owner_now();
            attack.chase_decision(&runtime, room, actor, session, now, |cell| {
                chase_path_tile(cells, blocking, cell)
            })?
        };
        let direction = match step {
            CardinalStep::North => super::StepDirection::North,
            CardinalStep::East => super::StepDirection::East,
            CardinalStep::South => super::StepDirection::South,
            CardinalStep::West => super::StepDirection::West,
        };
        let (outcome, duration) = self.paced_step(actor, session, direction).await;
        if outcome.moved_to.is_none() {
            let blocked = outcome.disposition == super::StepDisposition::Blocked;
            // The refused cell's 2 s starts when the refusal returned (`RANGED0-RL-08`).
            let mut attack = self.attack.lock().await;
            let now = self.owner_now();
            attack.chase_step_refused(actor, session, now, blocked);
        }
        Some((outcome, duration))
    }

    /// CHASE-1: the actor-owned pacing deadline of `actor` still ahead of now.
    pub(in crate::gameplay_transport) async fn actor_step_deadline_now(
        &self,
        actor: ExactActorRef,
    ) -> Option<Instant> {
        self.attack
            .lock()
            .await
            .step_deadline(actor, Instant::now())
    }

    /// CHASE-1: claim `actor`'s pacing clock for one step; `Some` when it is still held.
    pub(in crate::gameplay_transport) async fn begin_actor_step_now(
        &self,
        actor: ExactActorRef,
        holder: &Arc<()>,
    ) -> Option<Instant> {
        self.attack
            .lock()
            .await
            .begin_step(actor, holder, Instant::now())
    }

    /// CHASE-1: the claimed step ended; `actor`'s pacing clock is now due at `due`, or free.
    pub(in crate::gameplay_transport) async fn record_actor_step_deadline_now(
        &self,
        actor: ExactActorRef,
        holder: &Arc<()>,
        due: Option<Instant>,
    ) {
        self.attack
            .lock()
            .await
            .record_step_deadline(actor, holder, Instant::now(), due);
    }

    /// CHASE-1: whether `actor`'s chase search waits for a later owner window.
    pub(in crate::gameplay_transport) async fn chase_search_waiting_now(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> bool {
        self.attack
            .lock()
            .await
            .chase_search_waiting(actor, session)
    }

    /// CHASE-1: a client step of `actor` wins over its chase.
    pub(in crate::gameplay_transport) async fn chase_yield_to_client(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) {
        let now = self.owner_now();
        self.attack
            .lock()
            .await
            .chase_yield_to_client(actor, session, now);
    }

    /// One owner pass over every held target: at most one swing per attacker (`DEADLINE_STATE`
    /// catch-up), never a backlog.
    pub(in crate::gameplay_transport) async fn drain_auto_attacks(&self) {
        let Some(room) = self.combat_room else {
            return;
        };
        let Some(content) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return;
        };
        let Some(constants) = AttackConstants::checked_in() else {
            return;
        };
        let now = self.owner_now();
        let mut runtime = self.runtime.lock().await;
        let digest = runtime.content_pin().server_artifact_digest();
        if runtime.owner_fence().is_err()
            || content.source_digest() != digest
            || room.compiled().server_digest() != digest
        {
            return;
        }
        let states = self.spell_states.lock().await;
        let mut attack = self.attack.lock().await;
        attack.prune(&runtime);
        let ChannelAttackStates { entries, kills, .. } = &mut *attack;
        let sink = super::kill_reward::KillSink {
            queue: kills,
            rewards: self.reward_table(),
        };
        let root = GameplayDecisionRoot::from_bytes(digest);
        for entry in entries.iter_mut() {
            let Some(target) = entry.state.target() else {
                continue;
            };
            let Some(player) = states.get(&runtime, entry.actor, entry.session) else {
                continue;
            };
            if states.is_dead(entry.actor) {
                entry.clear_target();
                continue;
            }
            // SPELL-LOCK-2 §1.3: while the attacker's own slot is reserved by a pending spell
            // batch, its swing defers to a later turn; the held target and swing clock stay.
            if runtime.assert_actor_spell_unreserved(entry.actor).is_err() {
                continue;
            }
            let character_facts = player.character_facts();
            let Some(facts) = target_facts(&runtime, room, entry.actor, entry.session, target, now)
            else {
                continue;
            };
            let swing =
                match entry
                    .state
                    .poll_swing(now, constants.attack_interval_micros(), &facts)
                {
                    SwingPoll::Swing(swing) => swing,
                    SwingPoll::Cleared(_) => {
                        entry.target_ref = None;
                        continue;
                    }
                    SwingPoll::NoTarget | SwingPoll::Waiting(_) | SwingPoll::NotDue { .. } => {
                        continue;
                    }
                };
            // The ordinal advances on every swing, a zero draw included, so every swing has its
            // own RNG occurrence; the lineage ends at the last ordinal.
            let ordinal = entry.next_ordinal;
            match ordinal.checked_add(1) {
                Some(next) => entry.next_ordinal = next,
                None => entry.clear_target(),
            }
            let Ok(lease) = runtime
                .borrow_exact_actor_commit()
                .bound_attacker_lease(entry.actor, swing.lineage)
            else {
                entry.clear_target();
                continue;
            };
            let character = *lease.character_id().as_bytes();
            let Ok(snapshot) = runtime.companion_snapshot(target) else {
                entry.clear_target();
                continue;
            };
            let race_key = snapshot.state.policy.definition_key.clone();
            let creature_max_health = u64::try_from(snapshot.maximum_health).unwrap_or(0);
            run_charm_hook(
                self.imported_charms,
                character,
                &race_key,
                &root,
                swing_occurrence(entry.actor, swing.lineage, ordinal, b"damage-calculation"),
                CharmHookEvent::AttackDamageCalculation {
                    source: CharmHitSource::CharacterAttack,
                },
            );
            let mode = constants.fight_mode(fight_mode(entry.modes.fight_mode));
            let inputs = FormulaInputs {
                level: character_facts.level,
                magic_level: character_facts.magic_level,
                base_power: None,
                attack_skill: constants.starting_skill,
                attack_value: constants.fist.attack,
                attack_factor: mode.attack_factor,
                shielding_skill: constants.starting_skill,
                shield_defense: None,
            };
            let Ok((minimum, maximum)) = player_fist_formula(constants).bounds(&inputs) else {
                continue;
            };
            let Ok(draw) = deterministic_decision_u64(
                &root,
                swing_occurrence(entry.actor, swing.lineage, ordinal, b"damage-draw"),
                "attack.damage",
                0,
            ) else {
                continue;
            };
            let span = u64::try_from(maximum.saturating_sub(minimum))
                .unwrap_or(0)
                .saturating_add(1);
            let magnitude = minimum.saturating_add(i64::try_from(draw % span).unwrap_or(0));
            entry
                .state
                .record_hit(swing.at, constants.in_fight_micros());
            if magnitude <= 0 {
                continue;
            }
            let Some(plan) = swing_plan(
                entry.actor,
                swing.lineage,
                ordinal,
                magnitude,
                &digest,
                &race_key,
            ) else {
                continue;
            };
            let Ok(resolved) = resolve_exact_actor(
                &runtime.borrow_exact_actor_lookup(),
                plan.occurrence(),
                ExactActorProposal::client(target),
            ) else {
                entry.clear_target();
                continue;
            };
            // The owner clock of this turn, in ms, stamps the hit's contributor and, for a lethal
            // hit, the death time the kill reward reads (§1.2).
            let committed = crate::ability::commit::commit_exact_owner_swing_damage(
                &mut runtime.borrow_exact_actor_commit_at(now.get() / 1_000),
                &resolved,
                &plan,
                entry.actor,
                swing.lineage,
                ordinal,
            );
            let Ok(result) = committed else {
                entry.clear_target();
                continue;
            };
            run_charm_hook(
                self.imported_charms,
                character,
                &race_key,
                &root,
                swing_occurrence(entry.actor, swing.lineage, ordinal, b"committed-hit"),
                CharmHookEvent::committed_hit(
                    &result,
                    CharmHitSource::CharacterAttack,
                    CharmAttackerFacts {
                        level: character_facts.level,
                        max_health: u64::from(character_facts.max_health),
                        max_mana: u64::from(character_facts.max_mana),
                    },
                    creature_max_health,
                ),
            );
            if result.health_after <= 0 {
                let lethal = crate::combat::CapturedRewardPrincipal {
                    character: lease.character_id(),
                    lease_generation: lease.generation(),
                    session: entry.session,
                    actor: entry.actor,
                };
                super::kill_reward::record_creature_kill(
                    &mut runtime,
                    sink,
                    target,
                    lethal,
                    now.get() / 1_000,
                );
                entry.clear_target();
            }
        }
    }
}

#[cfg(test)]
#[path = "../foundation/channel_owner_auto_attack_tests.rs"]
mod tests;

/// SPELL-TARGET-1: a held target inside the spell range but outside the reference window is
/// out of sight (`TargetIllegal` in ordinary combat).
#[cfg(test)]
mod sees_tests {
    use super::*;

    fn at(x: i32, y: i32) -> MovementLocalPosition {
        MovementLocalPosition { x, y, floor: 7 }
    }

    #[test]
    fn reference_window_bounds_the_held_target() {
        let caster = at(100, 100);
        assert!(sees(caster, at(108, 107)));
        assert!(sees(caster, at(92, 94)));
        assert!(!sees(caster, at(100, 108)), "dy +8 is outside the window");
        assert!(!sees(caster, at(100, 93)), "dy -7 is outside the window");
        assert!(!sees(caster, at(110, 100)), "dx +10 is outside the window");
        assert!(!sees(caster, at(91, 100)), "dx -9 is outside the window");
    }
}
