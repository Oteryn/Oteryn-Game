//! ATTACK-1b: the Channel owner's attack-target state and auto-attack swing timer (ATTACK-0 §3,
//! §4, §9; `docs/architecture/reviews/OTERYN_GAME_ATTACK0_ATTACK_TARGET_AND_AUTO_ATTACK_DECISION_2026-09-30.md`).
//!
//! One entry per (player actor, session): the held creature target, the runtime-only fight modes
//! and the in-fight deadline. A due swing is one `AutoAttack` GAME-ABILITY-01 damage plan
//! committed by the physical owner under the swing identity `(character, lease generation,
//! session, lineage, swing ordinal)`: the lineage is the `CommandRef` of the `ATTACK_TARGET`
//! command that set the target and the ordinal counts the swings of that lineage from 0.
//! ATTACK-2: the swing uses the held melee weapon's attack (fists otherwise), and the creature
//! meets it with its block budget, its source defence and its armor (§4, §5).
//! Player targets use the same physical batch with current PvP legality and durable harm.
//!
//! CHASE-1 (RANGED-0 §8): with `chase=CHASE` and a held target the owner steps the player towards
//! the nearest tile next to it, one Movement-owner step at a time on the player's own pacing
//! clock, along a bounded path search (`combat_attack::chase`). A client step wins: it drops the
//! route and chase re-paths at its next 1,000 ms check. `STAND` never moves the player. `secure`
//! has no creature-target effect; its PvP refusal waits for player targets (PVP-RT-1).

mod attack_chase_search;

use crate::foundation::encode_state_delta;
use oteryn_protocol_oteryn::attack::{
    ActorCombatState, AttackIntentDisposition, ChaseMode, DELTA_TYPE_ACTOR_COMBAT_STATE_V1,
    FightMode as WireFightMode, FightModes, MAX_ATTACK_TARGET_CHANGES_PER_SECOND,
    MAX_FIGHT_MODE_CHANGES_PER_SECOND, STATE_DOMAIN_ACTOR_COMBAT_STATE, encode_actor_combat_state,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, SemanticTimeMicros,
};
#[cfg(test)]
use sha2::{Digest, Sha256};
use std::sync::{Arc, Weak};
use tokio::time::{Duration, Instant};

use super::ComposedFreshAdmission;
use crate::ability::exact_actor_resolution::{ExactActorProposal, resolve_exact_actor};
use crate::ability::{
    AbilityIntent, AbilityOccurrence, CommitGroup, Effect, EffectPlan, ProposalSource, RevisionSet,
};
use crate::combat_attack::{
    AttackConstants, AttackState, BlockBudget, CREATURE_COST, ChaseDecision, ChaseSearchBudget,
    ChaseState, FightMode, SwingPoll, TargetFacts, TargetRefusal, chase_search_order,
    player_fist_formula, player_melee_formula,
};
use crate::content::native_gameplay::NativeGameplayState;
use crate::content::{LogicalCell, QualifiedNativeEntryRoom};
use crate::durability::character_equipment::EquippedItem;
use crate::foundation::{
    ChannelRuntimeV1, CommandId, CommandRef, ExactActorRef, GameSessionId, MovementLocalPosition,
};
use crate::movement::CardinalStep;
use crate::movement::interest::{VisibilityPosition, VisibilitySettings};
use crate::spell::combat_execution::actor_atom;
use crate::spell::formula::FormulaInputs;
use oteryn_gameplay_progression::charm_effects::{
    CharmAttackerFacts, CharmHitSource, CharmHookEvent,
};

/// `ATTACK0-RL-01` and `ATTACK0-RL-02`: intents per [`ATTACK_INTENT_WINDOW`].
const MAX_INTENTS_PER_WINDOW: usize = 25;
const _: () = assert!(
    MAX_INTENTS_PER_WINDOW as u32 == MAX_ATTACK_TARGET_CHANGES_PER_SECOND
        && MAX_INTENTS_PER_WINDOW as u32 == MAX_FIGHT_MODE_CHANGES_PER_SECOND
);
pub const ATTACK_INTENT_WINDOW: Duration = Duration::from_secs(1);
/// How often a connection with domain 10 runs the owner's swing timer and looks for a changed
/// combat state.
pub const COMBAT_REFRESH: Duration = Duration::from_millis(250);

/// The sliding window of one intent limit: the instants of the last intents. `Copy` so it
/// travels with the session continuity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntentWindow([Option<Instant>; MAX_INTENTS_PER_WINDOW]);

impl IntentWindow {
    pub const EMPTY: Self = Self([None; MAX_INTENTS_PER_WINDOW]);

    /// Whether one more intent fits the window ending at `now`; if so it is recorded.
    pub fn admit(&mut self, now: Instant) -> bool {
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
pub struct CombatContinuity {
    pub revision: u64,
    pub sent: Option<ActorCombatState>,
    pub target_intents: IntentWindow,
    pub mode_intents: IntentWindow,
}

impl CombatContinuity {
    pub const FRESH: Self = Self {
        revision: 0,
        sent: None,
        target_intents: IntentWindow::EMPTY,
        mode_intents: IntentWindow::EMPTY,
    };

    /// The domain 10 snapshot above every revision this session has seen; recorded only by
    /// [`Self::snapshot_sent`] once transmitted.
    pub fn snapshot(&self, state: &ActorCombatState) -> Option<(u64, Vec<u8>)> {
        let revision = self.revision.checked_add(1)?;
        Some((revision, encode_actor_combat_state(state).ok()?))
    }

    pub fn snapshot_sent(&mut self, revision: u64, state: ActorCombatState) {
        self.revision = revision;
        self.sent = Some(state);
    }

    /// The domain 10 delta frame to `state` at the sequence after `sequence` when it differs
    /// from the last state sent; `Some(None)` when unchanged, `None` on a fault. The revision
    /// advances before the write, so a revision that may have reached the client is never
    /// reused.
    pub fn delta(
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

pub use oteryn_gameplay_combat_rules::resolution::WEAPON_REFRESH_MICROS;

pub use oteryn_gameplay_combat_rules::resolution::SwingWeapon;

/// The swing weapon read from the equipment owner under `content_digest` at `read_at`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WeaponFacts {
    weapon: SwingWeapon,
    content_digest: [u8; 32],
    read_at: SemanticTimeMicros,
}

/// The swing weapon of equipped hands, resolved by the pure Combat Rules owner.
pub fn swing_weapon(content: &NativeGameplayState, items: &[EquippedItem]) -> Option<SwingWeapon> {
    oteryn_gameplay_combat_rules::resolution::swing_weapon(content, |slot| {
        items.iter().find(|item| item.slot == slot).map(|item| {
            (
                item.definition.production_key.as_str(),
                item.definition.revision_ref.as_str(),
            )
        })
    })
}

pub use oteryn_gameplay_social_rules::shared_experience::SharedExperienceActivityObservation;
use oteryn_gameplay_social_rules::shared_experience::{self, SharedExperienceActivity};

/// The attack state of one player actor in one session.
#[derive(Debug, Clone)]
struct ActorAttack {
    actor: ExactActorRef,
    session: GameSessionId,
    state: AttackState<ExactActorRef, CommandRef>,
    modes: FightModes,
    /// The wire reference of the held target, as the client named it.
    target_ref: Option<EntityRef>,
    /// When the held target was set; a `pacified` applied since then clears it.
    target_set_at: SemanticTimeMicros,
    /// The swing ordinal of the next swing of the held target's lineage.
    next_ordinal: u16,
    /// CHASE-1: the chase route towards the held target (runtime-only).
    chase: ChaseState,
    /// The last swing weapon read; a swing waits while it is missing or stale.
    weapon: Option<WeaponFacts>,
    /// Bumped by every equipment move, so a read that raced one is not stored.
    weapon_epoch: u64,
    shared_experience_activity: Option<SharedExperienceActivity>,
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
pub struct ChannelAttackStates {
    entries: Vec<ActorAttack>,
    /// The block budget of each live creature a player has hit (ATTACK-0 §5), created at its
    /// first hit.
    blocks: Vec<(ExactActorRef, BlockBudget)>,
    /// The Channel's unsettled creature kills (ARCH-KILL-REWARD-LOGOUT-1 §1.3).
    kills: super::kill_reward::KillSettlementQueue,
    /// CHASE-1: the Channel's player chase searches in the current owner window
    /// (`RANGED0-RL-03`).
    chase_searches: ChaseSearchBudget,
    pub(super) path_budget: crate::movement::path_budget::ChannelPathBudget,
    /// CHASE-1 (RANGED-0 §8): each player actor's pacing clock, owned by the actor rather than
    /// its connection, so a client step and a chase step share one clock and a replaced or
    /// resumed connection keeps it. Only held clocks are kept.
    step_clocks: Vec<(ExactActorRef, StepClock)>,
}

impl ChannelAttackStates {
    pub const fn kills(&self) -> &super::kill_reward::KillSettlementQueue {
        &self.kills
    }

    /// CHASE-1: when `actor`'s pacing clock may next be claimed, if it is still held at `now`.
    pub fn step_deadline(&self, actor: ExactActorRef, now: Instant) -> Option<Instant> {
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
    pub fn begin_step(
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
    pub fn record_step_deadline(
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

/// CONDITIONS-0 §3.5: the actor's runtime condition slot holds a current `pacified`. An
/// actor without a slot holds no condition there.
fn pacified_slot(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
    now: SemanticTimeMicros,
) -> bool {
    runtime
        .actor_conditions(actor, Some(session))
        .is_ok_and(|store| store.attack_target_gate(now.get()).is_err())
}

/// ATTACK-0 §4 (as amended by #2106): a `pacified` was applied to `actor`, in its player store
/// or its runtime condition slot, at or after `since`, whether it is still current or not.
fn pacified_applied_since(
    runtime: &ChannelRuntimeV1,
    states: &super::actor_spell::ChannelSpellStates,
    actor: ExactActorRef,
    session: GameSessionId,
    since: SemanticTimeMicros,
) -> bool {
    states.get(runtime, actor, session).is_some_and(|player| {
        player
            .owned_conditions()
            .pacified_applied_since(since.get())
    }) || runtime
        .actor_conditions(actor, Some(session))
        .is_ok_and(|store| store.pacified_applied_since(since.get()))
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

pub use oteryn_gameplay_combat_rules::dispatch_target::in_protection_zone as in_protection_zone_rule;

fn in_protection_zone(
    room: &QualifiedNativeEntryRoom,
    position: MovementLocalPosition,
) -> Option<bool> {
    in_protection_zone_rule(
        room,
        LogicalCell {
            x: position.x,
            y: position.y,
            z: i32::from(position.floor),
        },
    )
}

/// Whether an attacker at `from` sees `to` under the reference view. A source world's native
/// floor sees as the Tibia floor of its negation (`VisibilityPosition`).
pub(super) fn sees(from: MovementLocalPosition, to: MovementLocalPosition) -> bool {
    match (
        VisibilityPosition::new(from.x, from.y, from.floor),
        VisibilityPosition::new(to.x, to.y, to.floor),
    ) {
        (Ok(observer), Ok(seen)) => VisibilitySettings::REFERENCE.can_see(observer, seen),
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
    if runtime
        .visible_entities()
        .players
        .iter()
        .any(|p| p.actor == target)
        && target != attacker
    {
        let to = runtime.read_actor_position(target).ok()?.position();
        return Some(TargetFacts {
            present_and_visible: sees(from, to),
            attackable_kind: true,
            alive: true,
            same_floor: to.floor == from.floor,
            distance: from.x.abs_diff(to.x).max(from.y.abs_diff(to.y)),
            attacker_in_protection_zone,
            target_in_protection_zone: in_protection_zone(room, to)?,
            attacker_reentry_protected: false,
            target_reentry_protected: false,
        });
    }
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

    /// Drop the entries whose actor is no longer controlled by its session's slot, and the
    /// block budgets of creatures no longer live.
    fn prune(&mut self, runtime: &ChannelRuntimeV1) {
        self.entries.retain(|entry| {
            runtime
                .player_control_facts(entry.actor, entry.session)
                .is_ok()
        });
        self.blocks
            .retain(|(creature, _)| runtime.contains_live_creature(*creature));
    }

    /// The equipment epoch of `actor` when it holds a target whose swing weapon is missing,
    /// read under another digest or older than [`WEAPON_REFRESH_MICROS`]; `None` when no read
    /// is due.
    fn weapon_read_due(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        digest: &[u8; 32],
        now: SemanticTimeMicros,
    ) -> Option<u64> {
        let entry = &self.entries[self.index(actor, session)?];
        entry.state.target()?;
        let fresh = entry.weapon.is_some_and(|facts| {
            &facts.content_digest == digest
                && now
                    .elapsed_since(facts.read_at)
                    .is_ok_and(|age| age < WEAPON_REFRESH_MICROS)
        });
        (!fresh).then_some(entry.weapon_epoch)
    }

    /// Store a swing weapon read that began at `epoch`; dropped when a move came since.
    fn store_weapon(
        &mut self,
        actor: ExactActorRef,
        session: GameSessionId,
        epoch: u64,
        facts: WeaponFacts,
    ) {
        if let Some(index) = self.index(actor, session) {
            let entry = &mut self.entries[index];
            if entry.weapon_epoch == epoch {
                entry.weapon = Some(facts);
            }
        }
    }

    /// An equipment move of `actor`: its swing weapon must be read again.
    pub fn forget_weapon(&mut self, actor: ExactActorRef, session: GameSessionId) {
        if let Some(index) = self.index(actor, session) {
            let entry = &mut self.entries[index];
            entry.weapon = None;
            entry.weapon_epoch = entry.weapon_epoch.wrapping_add(1);
        }
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
                    target_set_at: SemanticTimeMicros::from_micros(0),
                    next_ordinal: 0,
                    chase: ChaseState::default(),
                    weapon: None,
                    weapon_epoch: 0,
                    shared_experience_activity: None,
                });
                self.entries.len() - 1
            }
        };
        self.entries.get_mut(index)
    }

    /// One `ATTACK_TARGET` intent (§3). `None` stops attacking. A new target starts the lineage
    /// of command `command_id` at ordinal 0; a refused target leaves the held one unchanged.
    #[allow(clippy::too_many_arguments)]
    pub fn set_target(
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
        // CONDITIONS-0 §3.5: `pacified` refuses setting a target before every other check.
        if pacified_slot(runtime, actor, session, now) {
            return AttackIntentDisposition::Rejected;
        }
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
            .any(|p| p.actor == actor && named(p.actor, p.generation))
        {
            return AttackIntentDisposition::TargetNotACreature;
        }
        let Some(creature) = visible
            .creatures
            .iter()
            .find(|c| named(c.actor, c.generation))
            .map(|c| c.actor)
            .or_else(|| {
                visible
                    .players
                    .iter()
                    .find(|p| named(p.actor, p.generation))
                    .map(|p| p.actor)
            })
        else {
            return AttackIntentDisposition::TargetNotVisible;
        };
        let Some(facts) = target_facts(runtime, room, actor, session, creature, now) else {
            return AttackIntentDisposition::Rejected;
        };
        let previous = entry.state.target();
        match entry
            .state
            .set_target(creature, CommandRef::new(session, command), &facts)
        {
            Ok(()) => {
                entry.target_ref = Some(target_ref);
                entry.target_set_at = now;
                entry.next_ordinal = 0;
                // RANGED0-RL-08: only a target change clears the route and the refused cells;
                // reselecting the held target keeps them for their TTL.
                if previous != Some(creature) {
                    entry.chase.reset();
                }
                AttackIntentDisposition::Ok
            }
            Err(refusal) => refusal_disposition(refusal),
        }
    }

    /// One `FIGHT_MODES` intent: runtime-only, never persisted (§9).
    pub fn set_modes(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        modes: FightModes,
    ) -> bool {
        match self.entry(runtime, actor, session) {
            Some(entry) => {
                // RANGED0-RL-08: any mode change clears the route and the refused cells.
                if modes != entry.modes || modes.chase != ChaseMode::Chase {
                    entry.chase.reset();
                }
                entry.modes = modes;
                true
            }
            None => false,
        }
    }

    /// The `ACTOR_COMBAT_STATE` of `actor` at `now`; the defaults when it holds no entry.
    pub fn combat_state(
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

    pub(super) fn party_attack_state(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<&AttackState<ExactActorRef, CommandRef>> {
        self.index(actor, session).map(|i| &self.entries[i].state)
    }

    /// The running in-fight deadline of `actor` (`ATTACK0-RL-03`): a logout is refused and a
    /// closed client stays in the world until it ends.
    pub fn in_fight_until(
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

    /// ATTACK-0 §4 (as amended by #2106): applying `pacified` clears the held target. The
    /// condition appliers run under `runtime` and `spell_states` without this owner, so the clear
    /// lands under the attack guard before the held target is read, sent or swung: the caster's
    /// own cast completion and every reader settle it first. The condition store keeps the
    /// latest application time past the gate's expiry, so a target held across a `pacified` that
    /// ended before any reader ran is cleared as well. Returns whether a target was cleared.
    pub fn settle_pacified_target(
        &mut self,
        runtime: &ChannelRuntimeV1,
        states: &super::actor_spell::ChannelSpellStates,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> bool {
        let Some(index) = self.index(actor, session) else {
            return false;
        };
        let entry = &mut self.entries[index];
        if entry.state.target().is_none()
            || !pacified_applied_since(runtime, states, actor, session, entry.target_set_at)
        {
            return false;
        }
        entry.clear_target();
        true
    }

    /// Stop attacking (reconnect, control loss); the in-fight deadline is kept (§3 Reconnect).
    pub fn clear_target(&mut self, actor: ExactActorRef, session: GameSessionId) {
        if let Some(index) = self.index(actor, session) {
            self.entries[index].clear_target();
        }
    }

    /// Whether a creature may resolve `actor` as its melee target, and if so whether re-entry
    /// protection covers it. A closed client held in the world by its in-fight deadline
    /// (ATTACK0-RL-03) stays a target with no re-entry window, so a hit extends the hold; any
    /// other actor under control loss is no target.
    pub fn creature_target_protection(
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
    pub fn record_hit_taken(
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
    /// with `chase=CHASE`, a held target that is alive, visible and on the attacker's floor,
    /// neither side in a protection zone, and no re-entry protection on the attacker; otherwise
    /// its route is dropped. `path_tile` says whether a cell is a path tile of the Channel; a
    /// creature on it costs [`CREATURE_COST`] more. The route comes from the Channel's search
    /// drain, never from a search of this call's own.
    pub fn chase_decision(
        &mut self,
        runtime: &ChannelRuntimeV1,
        room: &QualifiedNativeEntryRoom,
        actor: ExactActorRef,
        session: GameSessionId,
        now: SemanticTimeMicros,
        path_tile: impl Fn(MovementLocalPosition) -> bool,
    ) -> Option<CardinalStep> {
        let index = self.index(actor, session)?;
        let entry = &mut self.entries[index];
        let chasing = entry.modes.chase == ChaseMode::Chase;
        let Some(target) = entry.state.target().filter(|_| chasing) else {
            entry.chase.reset();
            return None;
        };
        // The target stays held here, so only its route goes (RANGED-0 §8, `RANGED0-RL-08`).
        let Some((from, to)) = chase_ends(runtime, room, actor, session, target, now) else {
            entry.chase.hold();
            return None;
        };
        entry.chase.prepare(now.get(), from, to)?;
        self.drain_chase_searches(runtime, room, now, &path_tile);
        match self.entries[index].chase.next_step(from) {
            ChaseDecision::Step(step) => Some(step),
            ChaseDecision::Hold => None,
        }
    }

    /// CHASE-1: a client step wins over the chase; its route is dropped and it re-paths at its
    /// next 1,000 ms check.
    pub fn chase_yield_to_client(
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
    pub fn chase_search_waiting(&self, actor: ExactActorRef, session: GameSessionId) -> bool {
        self.index(actor, session)
            .is_some_and(|index| self.entries[index].chase.search_waiting_since().is_some())
    }

    /// CHASE-1: the chase step the Movement owner just refused at `now` drops the route. When it
    /// refused it as `blocked`, the refused cell also stays out of the next searches; any other
    /// refusal is transient and leaves the cell a path tile.
    pub fn chase_step_refused(
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
/// CHASE-1: where the chase of `actor` towards `target` runs from and to, while it may run: the
/// target alive, visible and on the attacker's floor, and, like the swing (ATTACK-0 §4), neither
/// side in a protection zone and no re-entry protection on the attacker. It never walks the
/// player out of a protection zone, nor toward a creature that became a companion.
fn chase_ends(
    runtime: &ChannelRuntimeV1,
    room: &QualifiedNativeEntryRoom,
    actor: ExactActorRef,
    session: GameSessionId,
    target: ExactActorRef,
    now: SemanticTimeMicros,
) -> Option<(MovementLocalPosition, MovementLocalPosition)> {
    let from = runtime.read_actor_position(actor).ok()?.position();
    let to = runtime.read_actor_position(target).ok()?.position();
    let facts = target_facts(runtime, room, actor, session, target, now)?;
    let runs = facts.present_and_visible
        && facts.attackable_kind
        && facts.alive
        && facts.same_floor
        && !facts.attacker_reentry_protected
        && !facts.attacker_in_protection_zone
        && !facts.target_in_protection_zone;
    runs.then_some((from, to))
}

const fn actor_order(actor: ExactActorRef) -> (u32, u64) {
    (actor.actor_local_id(), actor.actor_local_generation())
}

pub use oteryn_gameplay_combat_rules::dispatch_target::chase_path_tile as chase_path_tile_rule;

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
    chase_path_tile_rule(
        cells,
        target,
        crate::movement::source_floor_change::is_source_profile(cells),
    ) && !blocking.blocks(target)
        && !crate::map::floor::is_teleport(cells, target)
}

pub use oteryn_gameplay_combat_rules::charm_hook::run_charm_hook;

/// Translate the runtime swing identity into plain deterministic rule inputs.
fn swing_occurrence(
    attacker: ExactActorRef,
    lineage: CommandRef,
    ordinal: u16,
    tag: &[u8],
) -> DecisionOccurrenceId {
    oteryn_gameplay_combat_rules::resolution::swing_occurrence(
        attacker.placement_identity(),
        (
            *lineage.game_session_id().as_bytes(),
            lineage.command_id().get(),
        ),
        ordinal,
        tag,
    )
}

#[allow(clippy::too_many_arguments)]
fn swing_draw(
    root: &GameplayDecisionRoot,
    attacker: ExactActorRef,
    lineage: CommandRef,
    ordinal: u16,
    tag: &[u8],
    key: &str,
    minimum: i64,
    maximum: i64,
) -> Option<i64> {
    oteryn_gameplay_combat_rules::resolution::swing_draw(
        root,
        attacker.placement_identity(),
        (
            *lineage.game_session_id().as_bytes(),
            lineage.command_id().get(),
        ),
        ordinal,
        tag,
        key,
        minimum,
        maximum,
    )
}

#[allow(clippy::too_many_arguments)]
fn creature_mitigated(
    root: &GameplayDecisionRoot,
    constants: &AttackConstants,
    attacker: ExactActorRef,
    lineage: CommandRef,
    ordinal: u16,
    damage: i64,
    blocked: bool,
    defense: u32,
    armor: u32,
) -> Option<i64> {
    oteryn_gameplay_combat_rules::resolution::creature_mitigated(
        root,
        constants,
        attacker.placement_identity(),
        (
            *lineage.game_session_id().as_bytes(),
            lineage.command_id().get(),
        ),
        ordinal,
        damage,
        blocked,
        defense,
        armor,
    )
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
    formula: &str,
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
        formula,
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
        let room = self.rooms.gate_chase_step()?;
        let step = {
            // Lock order: runtime, spell states, door, then attack.
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            // DEATH-2 §4.2: a dead player takes no input, the chase included.
            if states.is_dead(actor) {
                return None;
            }
            // The same closed doors a player step reads (`paced_step`).
            let (none, bundle_doors, legacy);
            let blocking: &dyn crate::map::floor::CellBlocker = if self
                .rooms
                .gate_source_world()
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
            let mut attack = self.states.attack.lock().await;
            // Owner time is read once the locks that serialize chase decisions are held, so the
            // Channel search budget sees its windows in order (`RANGED0-RL-03`).
            let now = self.owner_now();
            // ATTACK-0 §4: a `pacified` applied over the held target clears it before the chase
            // reads it.
            attack.settle_pacified_target(&runtime, &states, actor, session);
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
            let mut attack = self.states.attack.lock().await;
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
        self.states
            .attack
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
        self.states
            .attack
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
        self.states
            .attack
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
        self.states
            .attack
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
        self.states
            .attack
            .lock()
            .await
            .chase_yield_to_client(actor, session, now);
    }
    /// Read the swing weapon of this connection's `actor` from the equipment owner when its
    /// held target's swings need one ([`ChannelAttackStates::weapon_read_due`]). A failed read
    /// stores nothing, so the swings wait (fail closed).
    pub(in crate::gameplay_transport) async fn refresh_attack_weapon(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) {
        let Some(content) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return;
        };
        let now = self.owner_now();
        let digest = self
            .runtime
            .lock()
            .await
            .content_pin()
            .server_artifact_digest();
        let Some(epoch) = self
            .states
            .attack
            .lock()
            .await
            .weapon_read_due(actor, session, &digest, now)
        else {
            return;
        };
        let Some(equipment) = super::equipment_move::read_current_equipment(self, session).await
        else {
            return;
        };
        if equipment.content_digest != digest || content.source_digest() != digest {
            return;
        }
        let Some(weapon) = swing_weapon(content, &equipment.items) else {
            return;
        };
        self.states.attack.lock().await.store_weapon(
            actor,
            session,
            epoch,
            WeaponFacts {
                weapon,
                content_digest: digest,
                read_at: now,
            },
        );
    }

    /// One owner pass over every held target: at most one swing per attacker (`DEADLINE_STATE`
    /// catch-up), never a backlog.
    pub(in crate::gameplay_transport) async fn drain_auto_attacks(&self) {
        let Some(room) = self.rooms.gate_auto_attack() else {
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
        let Some(mut pvp_permit) = self.spell_lane_permit().await else {
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
        let mut states = self.states.spell_states.lock().await;
        let objects = self.door.lock().await;
        let mut attack = self.states.attack.lock().await;
        attack.prune(&runtime);
        let ChannelAttackStates {
            entries,
            blocks,
            kills,
            ..
        } = &mut *attack;
        let mut activity_cells: Vec<_> = entries
            .iter()
            .filter_map(|e| e.shared_experience_activity)
            .collect();
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
            // CONDITIONS-0 §3.5: applying `pacified` clears a running target.
            if pacified_applied_since(
                &runtime,
                &states,
                entry.actor,
                entry.session,
                entry.target_set_at,
            ) {
                entry.clear_target();
                continue;
            }
            // SPELL-LOCK-2 §1.3: while the attacker's own slot is reserved by a pending spell
            // batch, its swing defers to a later turn; the held target and swing clock stay.
            if states.has_pending_spell_commit(entry.actor, entry.session)
                || runtime.assert_actor_spell_unreserved(entry.actor).is_err()
            {
                continue;
            }
            // ATTACK-2: a swing waits for a swing weapon read under this pin within one attack
            // interval; the attacker's own connection refreshes it (`refresh_attack_weapon`).
            let Some(weapon) = entry.weapon.filter(|facts| {
                facts.content_digest == digest
                    && now
                        .elapsed_since(facts.read_at)
                        .is_ok_and(|age| age <= constants.attack_interval_micros())
            }) else {
                continue;
            };
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
            let mode = constants.fight_mode(fight_mode(entry.modes.fight_mode));
            let (formula, attack_value, revision) = match weapon.weapon {
                SwingWeapon::Fists => (
                    player_fist_formula(constants),
                    constants.fist.attack,
                    "formula:player-fist",
                ),
                SwingWeapon::Melee { attack } => (
                    player_melee_formula(constants),
                    attack,
                    "formula:player-melee",
                ),
            };
            let inputs = FormulaInputs {
                level: character_facts.level,
                magic_level: character_facts.magic_level,
                base_power: None,
                attack_skill: constants.starting_skill,
                attack_value,
                attack_factor: mode.attack_factor,
                shielding_skill: constants.starting_skill,
                shield_defense: None,
            };
            let Ok((minimum, maximum)) = formula.bounds(&inputs) else {
                continue;
            };
            let Some(damage) = swing_draw(
                &root,
                entry.actor,
                swing.lineage,
                ordinal,
                b"damage-draw",
                "attack.damage",
                minimum,
                maximum,
            ) else {
                continue;
            };
            if runtime.contains_live_creature(target) {
                entry
                    .state
                    .record_hit(swing.at, constants.in_fight_micros());
            }
            if damage <= 0 {
                continue;
            }
            if !runtime.contains_live_creature(target) {
                let Some(black) = self
                    .pvp_target_legal(
                        &pvp_permit,
                        &runtime,
                        &states,
                        room,
                        &objects,
                        entry.actor,
                        target,
                        swing.lineage,
                        entry.modes.secure,
                    )
                    .await
                else {
                    entry.clear_target();
                    continue;
                };
                let Ok(raw) = u32::try_from(damage) else {
                    entry.clear_target();
                    continue;
                };
                let magnitude =
                    i64::from(oteryn_gameplay_combat_rules::pvp::pvp_damage(raw, black));
                let atom = actor_atom(target);
                let Some(plan) = swing_plan(
                    entry.actor,
                    swing.lineage,
                    ordinal,
                    magnitude,
                    &digest,
                    &atom,
                    revision,
                ) else {
                    continue;
                };
                let batch = crate::spell::combat_batch::OwnerCombatBatch {
                    caster: entry.actor,
                    attacker: lease.character_id(),
                    current_lease_generation: lease.generation(),
                    command: swing.lineage,
                    occurrence: plan.occurrence().clone().into(),
                    binding: b"oteryn:pvp-melee:v1".to_vec(),
                    anchor: None,
                    now_us: Some(now),
                    now_ms: now.get() / 1000,
                    effects: vec![crate::spell::combat_batch::OwnerCombatEffect {
                        target,
                        sub_ordinal: ordinal,
                        change: crate::spell::combat_batch::OwnerCombatChange::Damage {
                            target_atom: atom,
                            magnitude,
                        },
                    }],
                    deferred: None,
                };
                if !self
                    .pvp_swing(
                        &mut pvp_permit,
                        &mut runtime,
                        &mut states,
                        room,
                        &objects,
                        batch,
                        ordinal,
                        entry.modes.secure,
                        black,
                    )
                    .await
                {
                    entry.clear_target();
                }
                continue;
            }
            let Ok(snapshot) = runtime.companion_snapshot(target) else {
                entry.clear_target();
                continue;
            };
            let race_key = snapshot.state.policy.definition_key.clone();
            // The creature's content defence and armor (ATTACK-0 §5, `monster.cpp:242-251`);
            // a creature without them takes no hit (fail closed).
            let (Some(defense), Some(armor)) =
                (snapshot.state.policy.defense, snapshot.state.policy.armor)
            else {
                continue;
            };
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
            let index = match blocks.iter().position(|(creature, _)| *creature == target) {
                Some(index) => index,
                None => {
                    if blocks.len() >= runtime.actor_slot_capacity() {
                        continue;
                    }
                    // A creature's budget refills from spawn, not from the first hit on it.
                    blocks.push((target, BlockBudget::settled(&constants.block, swing.at)));
                    blocks.len() - 1
                }
            };
            let blocked = blocks[index].1.try_block(&constants.block, swing.at);
            let Some(magnitude) = creature_mitigated(
                &root,
                constants,
                entry.actor,
                swing.lineage,
                ordinal,
                damage,
                blocked,
                defense,
                armor,
            ) else {
                continue;
            };
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
                revision,
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
            if result.applied
                && result.health_after < result.health_before
                && aggressive_monster(&runtime, content, target)
            {
                entry.record_shared_experience_activity(&runtime, now);
                if let Some(cell) = entry.shared_experience_activity {
                    if let Some(old) = activity_cells
                        .iter_mut()
                        .find(|old| old.character == cell.character)
                    {
                        *old = cell;
                    } else {
                        activity_cells.push(cell);
                    }
                }
            }
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
                    super::kill_reward::KillSink {
                        queue: kills,
                        rewards: self.reward_table(),
                        activity: &activity_cells,
                    },
                    target,
                    lethal,
                    now.get() / 1_000,
                    Some(now),
                );
                entry.clear_target();
            }
        }
    }
}

#[cfg(test)]
#[path = "channel_owner_auto_attack_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "attack_gate_tests.rs"]
mod gate_tests;

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

/// ATTACK-2: the swing weapon of the production Item policies, and the creature's block,
/// defence and armor (`player.cpp:405-464`, `creature.cpp:958-990`).
#[cfg(test)]
#[allow(clippy::expect_used)]
mod weapon_tests {
    use super::*;
    use crate::durability::item_mint::TypedDefinitionRef;

    fn item(slot: u8, tibia_id: u32) -> EquippedItem {
        EquippedItem {
            slot,
            item_instance_id: [slot; 16],
            definition: TypedDefinitionRef {
                family: "Item".into(),
                production_key: format!("oteryn:item.tibia.i{tibia_id}"),
                revision_ref: "definition-r1".into(),
            },
            quantity: 1,
            state_revision: 1,
        }
    }

    /// The production generation's Item policies, loaded once for every case.
    fn production() -> crate::content::ContentActivationController {
        let world = crate::foundation::WorldId::decode(&uuid(1)).expect("world id");
        crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None).0
    }

    fn weapon_of(
        controller: &crate::content::ContentActivationController,
        items: &[(u8, u32)],
    ) -> Option<SwingWeapon> {
        let content = controller
            .active()
            .and_then(|active| active.native_gameplay())
            .expect("the production generation");
        let items: Vec<_> = items
            .iter()
            .map(|(slot, tibia_id)| item(*slot, *tibia_id))
            .collect();
        swing_weapon(content, &items)
    }

    #[test]
    fn a_held_melee_weapon_swings_with_its_attack_and_anything_else_with_fists() {
        let production = production();
        let weapon_of = |items: &[(u8, u32)]| weapon_of(&production, items);
        // Sword (3264, attack 14), staff (3289, club, attack 10), wooden shield (3412),
        // spear (3277, distance), arrow (3447, ammunition), wand of darkness (25760).
        assert_eq!(weapon_of(&[]), Some(SwingWeapon::Fists));
        assert_eq!(
            weapon_of(&[(5, 3264)]),
            Some(SwingWeapon::Melee { attack: 14 })
        );
        assert_eq!(
            weapon_of(&[(6, 3289)]),
            Some(SwingWeapon::Melee { attack: 10 })
        );
        assert_eq!(
            weapon_of(&[(6, 3412), (5, 3264)]),
            Some(SwingWeapon::Melee { attack: 14 }),
            "a shield is skipped for the other hand"
        );
        assert_eq!(
            weapon_of(&[(6, 3289), (5, 3264)]),
            Some(SwingWeapon::Melee { attack: 10 }),
            "the left hand is read first"
        );
        assert_eq!(weapon_of(&[(6, 3412)]), Some(SwingWeapon::Fists));
        assert_eq!(weapon_of(&[(5, 3277)]), Some(SwingWeapon::Fists));
        assert_eq!(weapon_of(&[(5, 25760)]), Some(SwingWeapon::Fists));
        assert_eq!(
            weapon_of(&[(6, 3447), (5, 3264)]),
            Some(SwingWeapon::Melee { attack: 14 }),
            "ammunition is skipped"
        );
        assert_eq!(
            weapon_of(&[(10, 3264)]),
            Some(SwingWeapon::Fists),
            "only the hands are read"
        );
        // A hand item without Known weapon facts or policy fails closed: poison field (105)
        // has `weapon: UNKNOWN`; 1 names no policy.
        assert_eq!(weapon_of(&[(5, 105)]), None);
        assert_eq!(weapon_of(&[(6, 1)]), None);
    }

    fn uuid(tag: u8) -> [u8; 16] {
        [1, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag]
    }
}

/// The active content owner, the exact physical definition/revision, and the master relation
/// qualify aggression. Attackable alone is not hostility; summons/familiars are not monsters.
fn aggressive_monster(
    runtime: &ChannelRuntimeV1,
    content: &NativeGameplayState,
    actor: ExactActorRef,
) -> bool {
    if content.source_digest() != runtime.content_pin().server_artifact_digest() {
        return false;
    }
    let Ok(snapshot) = runtime.companion_snapshot_including_dead(actor) else {
        return false;
    };
    if snapshot.state.master.is_some() || snapshot.state.policy.is_familiar {
        return false;
    }
    content
        .creature_profiles()
        .records
        .iter()
        .find(|record| {
            record.profile.target.key == snapshot.state.policy.definition_key
                && record.profile.target.revision == snapshot.state.policy.definition_revision
        })
        .is_some_and(|record| {
            content.qualifies_current_creature_profile(content.source_digest(), &record.profile)
                && matches!(record.behavior.as_ref().map(|profile| &profile.data),
                Some(crate::content::ProjectV2AuthoringProfileData::Behavior(behavior))
                    if behavior.targeting.hostile && behavior.targeting.can_target)
        })
}

impl ActorAttack {
    fn record_shared_experience_activity(
        &mut self,
        runtime: &ChannelRuntimeV1,
        at: SemanticTimeMicros,
    ) -> bool {
        if runtime.owner_fence().is_err()
            || !runtime
                .player_control_facts(self.actor, self.session)
                .is_ok_and(|control| control.control_loss.is_none())
        {
            return false;
        }
        let Ok(lease) = runtime.bound_attacker_lease(self.actor, self.session) else {
            return false;
        };
        let next = SharedExperienceActivity {
            character: *lease.character_id().as_bytes(),
            lease_generation: lease.generation(),
            at,
        };
        shared_experience::record_shared_experience_activity(
            &mut self.shared_experience_activity,
            next,
        )
    }
}

#[allow(
    dead_code,
    reason = "D #2172 integrates these native activity APIs in its separate draft"
)]
impl ChannelAttackStates {
    /// Retained event data for death capture; reading never refreshes an activity clock.
    pub(super) fn shared_experience_cells(&self) -> Vec<SharedExperienceActivity> {
        self.entries
            .iter()
            .filter_map(|e| e.shared_experience_activity)
            .collect()
    }

    /// Preallocate the existing per-player entry BEFORE physical spell installation. No
    /// activity is granted here; phase 2's receipt recorder never allocates or creates it.
    pub fn prepare_shared_experience_record(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> bool {
        self.entry(runtime, actor, session).is_some()
    }

    /// Native activity data only. D must independently resolve current strong membership,
    /// presence, range, levels and leader PZ at the kill; this read grants no authority.
    pub fn shared_experience_active(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        now: SemanticTimeMicros,
    ) -> bool {
        self.committed_shared_experience_activity(runtime, actor, session, now)
            .is_some_and(shared_experience::shared_experience_active)
    }

    /// Read retained native data without renewing the event time. The caller supplies
    /// its independently observed semantic clock; future events refuse this observation.
    pub fn committed_shared_experience_activity(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        observed_at: SemanticTimeMicros,
    ) -> Option<SharedExperienceActivityObservation> {
        let entry = self
            .index(actor, session)
            .map(|index| &self.entries[index])?;
        let activity = entry.shared_experience_activity?;
        if runtime.owner_fence().is_err()
            || !runtime
                .player_control_facts(actor, session)
                .is_ok_and(|control| control.control_loss.is_none())
        {
            return None;
        }
        let lease = runtime.bound_attacker_lease(actor, session).ok()?;
        shared_experience::committed_shared_experience_activity(
            activity,
            lease.character_id().as_bytes(),
            lease.generation(),
            observed_at,
        )
    }

    /// Call under the SAME runtime -> states -> attack guards after successful native owner
    /// install, using that attempt's actual batch and transaction-qualified SourcePartyWorld.
    /// Captured party data describes membership when the committed heal happened; it never
    /// authorizes current membership at a later kill. Replay cannot extend the activity clock.
    pub fn record_committed_shared_experience_batch(
        &mut self,
        runtime: &ChannelRuntimeV1,
        content: &NativeGameplayState,
        batch: &crate::foundation::runtime_actor_spell_types::OwnerCombatBatch,
        receipt: &crate::foundation::runtime_actor_spell_types::CombatBatchReceipt,
        party: Option<&super::party_spell_owner::SourcePartyWorld>,
    ) -> bool {
        use crate::foundation::runtime_actor_spell_types::OwnerCombatChange;
        if !receipt.applied
            || batch.caster != receipt.caster
            || batch.command != receipt.command
            || content.source_digest() != runtime.content_pin().server_artifact_digest()
        {
            return false;
        }
        let session = batch.command.game_session_id();
        let Ok(lease) = runtime.bound_attacker_lease(batch.caster, session) else {
            return false;
        };
        if lease.character_id() != batch.attacker
            || lease.generation() != batch.current_lease_generation
        {
            return false;
        }
        let eligible = receipt.effects.iter().any(|effect| {
            let Some(health) = effect.health.filter(|health| health.applied) else {
                return false;
            };
            let Some(proposed) = batch.effects.iter().find(|proposed| {
                proposed.target == effect.target && proposed.sub_ordinal == effect.sub_ordinal
            }) else {
                return false;
            };
            match &proposed.change {
                OwnerCombatChange::Damage { .. } | OwnerCombatChange::DamageWithHealing { .. }
                    if health.health_after < health.health_before =>
                {
                    aggressive_monster(runtime, content, effect.target)
                }
                OwnerCombatChange::Heal { .. } if health.health_after > health.health_before => {
                    let Some(party) = party else {
                        return false;
                    };
                    let source = party.source();
                    source.party_id.is_some()
                        && source.world == *runtime.binding().world_id().as_bytes()
                        && source.character == *lease.character_id().as_bytes()
                        && source
                            .members
                            .iter()
                            .any(|member| member.character == source.character)
                        && party
                            .actor_for_id(u64::from(effect.target.actor_local_id()))
                            .is_some_and(|(actor, target_session)| {
                                actor == effect.target
                                    && runtime
                                        .bound_attacker_lease(actor, target_session)
                                        .is_ok_and(|target| {
                                            heal_credits_party_member(
                                                source,
                                                target.character_id().as_bytes(),
                                            )
                                        })
                            })
                }
                _ => false,
            }
        });
        if !eligible {
            return false;
        }
        let Some(original_event_at) = batch.now_us else {
            return false;
        };
        if original_event_at.get() / 1000 != batch.now_ms {
            return false;
        }
        let Some(index) = self.index(batch.caster, session) else {
            return false;
        };
        self.entries[index].record_shared_experience_activity(runtime, original_event_at)
    }
}

/// Shared experience credits a heal only when it heals ANOTHER member of the caster's
/// captured party. A self-heal never counts, including an area heal such as Mass Healing
/// that reaches only the caster.
fn heal_credits_party_member(
    source: &crate::foundation::PartySnapshot,
    target_character: &[u8; 16],
) -> bool {
    shared_experience::heal_credits_party_member(
        &source.character,
        source.members.iter().map(|member| &member.character),
        target_character,
    )
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod shared_experience_activity_tests {
    use super::*;
    use crate::foundation::{CharacterId, CharacterLease};

    #[test]
    fn activity_uses_current_native_lease_and_two_minute_window_without_future_credit() {
        let (mut runtime, actor, session) =
            super::super::actor_spell::tests::runtime_with_player(0x71);
        let mut raw = [1; 16];
        raw[6] = 0x70;
        raw[8] = 0x80;
        let character = CharacterId::decode(&raw).expect("character");
        runtime
            .bind_attacker_lease(
                actor,
                session,
                CharacterLease::new(character, 7).expect("lease"),
            )
            .expect("bind");
        let mut states = ChannelAttackStates::default();
        assert!(states.prepare_shared_experience_record(&runtime, actor, session));
        let at = SemanticTimeMicros::from_micros(2_000_000);
        let index = states.index(actor, session).expect("entry");
        assert!(states.entries[index].record_shared_experience_activity(&runtime, at));
        assert!(!states.shared_experience_active(
            &runtime,
            actor,
            session,
            SemanticTimeMicros::from_micros(1_999_999)
        ));
        assert!(states.shared_experience_active(
            &runtime,
            actor,
            session,
            SemanticTimeMicros::from_micros(122_000_000)
        ));
        assert!(!states.shared_experience_active(
            &runtime,
            actor,
            session,
            SemanticTimeMicros::from_micros(122_000_001)
        ));
        let mut successor_bytes = [3; 16];
        successor_bytes[6] = 0x70;
        successor_bytes[8] = 0x80;
        let successor = GameSessionId::decode(&successor_bytes).expect("successor session");
        runtime
            .rebind_player_session(
                actor,
                session,
                successor,
                CharacterLease::new(character, 8).expect("new lease"),
            )
            .expect("native successor rebind");
        assert!(
            !states.shared_experience_active(&runtime, actor, session, at),
            "retained activity is not new lease evidence"
        );
    }

    #[test]
    fn unbound_actor_has_no_activity_and_out_of_order_event_does_not_rewind_it() {
        let (mut runtime, actor, session) =
            super::super::actor_spell::tests::runtime_with_player(0x72);
        let mut states = ChannelAttackStates::default();
        let entry = states.entry(&runtime, actor, session).expect("entry");
        assert!(
            !entry.record_shared_experience_activity(&runtime, SemanticTimeMicros::from_micros(20))
        );
        let mut raw = [2; 16];
        raw[6] = 0x70;
        raw[8] = 0x80;
        runtime
            .bind_attacker_lease(
                actor,
                session,
                CharacterLease::new(CharacterId::decode(&raw).expect("character"), 1)
                    .expect("lease"),
            )
            .expect("bind");
        assert!(
            entry.record_shared_experience_activity(&runtime, SemanticTimeMicros::from_micros(20))
        );
        assert!(
            !entry.record_shared_experience_activity(&runtime, SemanticTimeMicros::from_micros(19))
        );
        assert!(states.shared_experience_active(
            &runtime,
            actor,
            session,
            SemanticTimeMicros::from_micros(20)
        ));
        for observed in [21, 22, 120_000_021] {
            let activity = states
                .committed_shared_experience_activity(
                    &runtime,
                    actor,
                    session,
                    SemanticTimeMicros::from_micros(observed),
                )
                .expect("current native observation");
            assert_eq!(
                activity.original_event_at,
                SemanticTimeMicros::from_micros(20)
            );
            assert_eq!(
                activity.observed_at,
                SemanticTimeMicros::from_micros(observed)
            );
        }
        assert!(!states.shared_experience_active(
            &runtime,
            actor,
            session,
            SemanticTimeMicros::from_micros(120_000_021),
        ));
    }
}
