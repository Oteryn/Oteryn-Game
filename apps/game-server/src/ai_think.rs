//! CREATURE-AI-1 (`OTERYN_GAME_CREATURE_AI1_TARGETING_AND_THINK_PACKET_2026-10-04.md` §1, §2.1):
//! the creature think. One think per creature per `CREATURE_THINK_INTERVAL_MILLIS`, over the
//! owner's facts and the creature's `CreatureBehaviourProfile` (§1.2), in Canary's order:
//! override expiry, target maintenance, search, the timed target change, the fleeing strategy
//! draw, one cardinal step (§1.1), then the profile's attack and defence entries through
//! `profile_schedule` (§2.1 item 5). The melee entry is never proposed here: ATTACK-1's swing
//! (`ai_monster_melee`) swings it at this think's target (§1.5).
//!
//! The think holds no profile, health or chance literal: every behaviour value is a profile
//! field. It mutates only its own `CreatureAiState`; the owner (the Channel carrier) commits the
//! step through Movement and issues the proposals through Ability, and either may refuse. A
//! refusal changes only the next think.
//!
//! Randomness is `oteryn_simulation_determinism::deterministic_decision_u64` bound to the think
//! occurrence (`profile_schedule::decision_occurrence`, creature placement and think sequence)
//! with one purpose per draw family (`AI_TARGET_SEARCH`, `AI_TARGET_CHANGE`, `AI_ATTACK`,
//! `AI_DEFENCE`, `AI_WANDER`) and the entry or selection index. A retry of the same sequence
//! returns the recorded report and never redraws; a lower sequence is refused.
//!
//! Bounds (`RESOURCE_LIMITS_REGISTRY.json`): `CREATUREAI0-RL-05` thinks per Channel window
//! (carrier), `CREATUREAI0-RL-06` evaluation units per think (here: a think over the bound ends
//! with zero mutation), `CREATUREAI0-RL-07` proposals per think (refused at profile admission,
//! `behaviour_profile.rs`) and `CREATUREAI0-RL-18` overrides per creature (here: two slots, the
//! newer replaces the older). `AI01-PENDING-TIMERS-PER-ACTOR` stays 1: one pending think.
//!
//! This lives as a sibling of `ai/`, not inside it: `tests/ai_bootstrap.rs` path-includes
//! `ai/mod.rs` as its own standalone crate root without `foundation`/`content`/`movement`, so the
//! modules that need them (`behaviour_profile`, `targeting`) are declared here with `#[path]`.
//!
//! Timer binding (AI-1 `owner_timer.rs`, `#162` binding items (a)/(b)): `ThinkSequenceTracker`
//! derives every `ThinkOccurrence` sequence monotonically per actor, and `schedule_next_think`
//! forwards only a stamp the caller's fence issued.

#[path = "ai/behaviour_profile.rs"]
pub mod behaviour_profile;
mod channel_owner;
pub mod profile_schedule;
#[path = "ai/targeting.rs"]
pub mod targeting;

use std::collections::BTreeMap;

use crate::ability::RevisionSet;
use crate::ai::AiProvenance;
#[cfg(test)]
use crate::content::static_cell_engine::EngineeringStaticCellIndex;
use crate::foundation::owner_timer::{
    AI01_PENDING_TIMERS_PER_ACTOR, OwnerTimerError, OwnerTimerLane, SemanticTimeMicros, TimerFamily,
};
use crate::foundation::{
    CurrentOwnerMovementPosition, ExactActorRef, MovementLocalPosition, MovementPositionSnapshot,
    RuntimeWorkStamp, ScopeRuntimeFence,
};
use crate::movement::{CardinalStep, MovementEngineeringSelection, MovementError, step_cardinal};
use behaviour_profile::CreatureBehaviourProfile;
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use profile_schedule::{
    AttackTarget, ProfileAbilityProposal, ProfileScheduleState, ScheduleError, decision_occurrence,
};
use targeting::{Strategy, TargetCandidate};

/// §1.3: one think per creature per second (Canary's `EVENT_CREATURE_THINK_INTERVAL`).
pub const CREATURE_THINK_INTERVAL_MILLIS: u64 = 1_000;
/// `CREATUREAI0-RL-05`: thinks one Channel starts per window.
pub const CREATUREAI0_THINKS_PER_WINDOW: usize = 1_024;
/// `CREATUREAI0-RL-05`'s window.
pub const CREATUREAI0_THINK_WINDOW_MICROS: u64 = 50_000;
/// `CREATUREAI0-RL-06`: evaluation units of one think (1 + perceived + eligible + entries).
pub const CREATUREAI0_EVALUATION_UNITS_PER_THINK: usize = 128;
/// `CREATUREAI0-RL-18`: override slots per creature.
pub const CREATUREAI0_OVERRIDES_PER_CREATURE: usize = 2;
/// §2.1 item 1: creatures one player move or admission may wake (VIS-1 candidates).
pub const CREATUREAI1_WAKE_CANDIDATES: usize = 1_024;

const PPM: u64 = 1_000_000;
const PURPOSE_TARGET_SEARCH: &str = "AI_TARGET_SEARCH";
const PURPOSE_TARGET_CHANGE: &str = "AI_TARGET_CHANGE";
const PURPOSE_WANDER: &str = "AI_WANDER";
const CARDINALS: [CardinalStep; 4] = [
    CardinalStep::North,
    CardinalStep::East,
    CardinalStep::South,
    CardinalStep::West,
];

/// §1.6: an owner-side override. Each kind has one slot; a newer override replaces the older
/// one of its kind, so a creature holds at most `CREATUREAI0-RL-18` (2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatureAiOverride {
    /// Holds `actor` as the target until `until_us` (no search, no timed change, no flee) while
    /// it stays eligible.
    ForcedTarget { actor: ExactActorRef, until_us: u64 },
    /// Treats `target_distance` as 1 until `until_us` (no flee).
    ForcedDistanceOne { until_us: u64 },
}

/// The owner's facts for one think. `players` are the Channel's player candidates with the
/// owner's eligibility facts; perception filters and orders them.
#[derive(Clone, Copy)]
pub struct CreatureThinkFacts<'a> {
    pub position: MovementLocalPosition,
    pub health: u64,
    pub has_active_condition: bool,
    pub players: &'a [TargetCandidate],
    pub now_us: u64,
    pub root: &'a GameplayDecisionRoot,
    pub revisions: &'a RevisionSet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepKind {
    Chase,
    Flee,
    Wander,
}

/// §1.1: the one cardinal step a think proposes; the owner commits it through Movement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureStep {
    pub kind: StepKind,
    pub step: CardinalStep,
    pub destination: MovementLocalPosition,
}

/// What one think decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThinkReport {
    pub sequence: u64,
    pub target: Option<ExactActorRef>,
    /// The timed target change (§2.1 item 4) fired this think.
    pub target_changed: bool,
    pub step: Option<CreatureStep>,
    /// Due and drawn non-melee entries, at most `CREATUREAI0-RL-07`.
    pub proposals: Vec<ProfileAbilityProposal>,
    pub refusal: Option<ScheduleError>,
    pub fleeing: bool,
    /// §2.1 item 1: nothing perceived, no target and no active condition; no next think.
    pub idle: bool,
    /// `CREATUREAI0-RL-06` refused this think; nothing changed.
    pub over_budget: bool,
    pub units: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThinkError {
    OccurrenceSuperseded,
    Draw,
}

/// One creature's think state. Ephemeral: confers no Channel owner authority.
pub struct CreatureAiState {
    actor: ExactActorRef,
    profile: CreatureBehaviourProfile,
    anchor: MovementLocalPosition,
    target: Option<ExactActorRef>,
    forced_target: Option<(ExactActorRef, u64)>,
    forced_distance: Option<u64>,
    change_ticks_ms: u64,
    wander_ticks_ms: u64,
    schedule: ProfileScheduleState,
    last: Option<ThinkReport>,
}

impl std::fmt::Debug for CreatureAiState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CreatureAiState")
            .field("actor", &self.actor)
            .field("target", &self.target)
            .field("forced_target", &self.forced_target)
            .field("forced_distance", &self.forced_distance)
            .finish_non_exhaustive()
    }
}

impl CreatureAiState {
    /// `anchor` is the spawn or admission cell wander stays near (§1.4).
    #[must_use]
    pub fn new(
        actor: ExactActorRef,
        profile: CreatureBehaviourProfile,
        anchor: MovementLocalPosition,
    ) -> Self {
        Self {
            actor,
            profile,
            anchor,
            target: None,
            forced_target: None,
            forced_distance: None,
            change_ticks_ms: 0,
            wander_ticks_ms: 0,
            schedule: ProfileScheduleState::new(actor),
            last: None,
        }
    }

    #[must_use]
    pub const fn actor(&self) -> ExactActorRef {
        self.actor
    }

    #[must_use]
    pub const fn profile(&self) -> &CreatureBehaviourProfile {
        &self.profile
    }

    #[must_use]
    pub const fn target(&self) -> Option<ExactActorRef> {
        self.target
    }

    /// The overrides this creature holds, at most `CREATUREAI0-RL-18`.
    #[must_use]
    pub fn override_count(&self) -> usize {
        usize::from(self.forced_target.is_some()) + usize::from(self.forced_distance.is_some())
    }

    /// §1.6: the one owner-side setter. The newer override replaces the older of its kind.
    pub fn set_override(&mut self, value: CreatureAiOverride) {
        match value {
            CreatureAiOverride::ForcedTarget { actor, until_us } => {
                self.forced_target = Some((actor, until_us));
            }
            CreatureAiOverride::ForcedDistanceOne { until_us } => {
                self.forced_distance = Some(until_us);
            }
        }
    }

    /// One think. `admits` is the owner's Movement admission of a destination cell.
    pub fn think(
        &mut self,
        sequence: u64,
        facts: &CreatureThinkFacts<'_>,
        admits: impl Fn(MovementLocalPosition) -> bool,
    ) -> Result<ThinkReport, ThinkError> {
        if let Some(last) = &self.last {
            if sequence < last.sequence {
                return Err(ThinkError::OccurrenceSuperseded);
            }
            if sequence == last.sequence {
                return Ok(last.clone());
            }
        }
        let position = facts.position;
        let targeting = self.profile.targeting();
        let perceived = targeting::perceive(position, facts.players);
        let eligible = perceived
            .iter()
            .filter(|candidate| targeting::is_eligible(targeting, position, candidate))
            .copied()
            .collect::<Vec<_>>();
        let units = 1 + perceived.len() + eligible.len() + self.profile.entry_count();
        if units > CREATUREAI0_EVALUATION_UNITS_PER_THINK {
            return Ok(ThinkReport {
                sequence,
                target: self.target,
                target_changed: false,
                step: None,
                proposals: Vec::new(),
                refusal: None,
                fleeing: false,
                idle: false,
                over_budget: true,
                units,
            });
        }

        let occurrence = ThinkOccurrence {
            actor: self.actor,
            sequence,
        };
        let draw_occurrence = decision_occurrence(occurrence);
        let draw = |purpose: &str, index: u64| {
            deterministic_decision_u64(facts.root, draw_occurrence, purpose, index)
                .map_err(|_| ThinkError::Draw)
        };
        let find =
            |actor: ExactActorRef| eligible.iter().find(|candidate| candidate.actor == actor);
        let strategy_search = |index: u64| -> Result<Option<ExactActorRef>, ThinkError> {
            let strategy = targeting::pick_strategy(
                targeting.strategy_weights,
                draw(PURPOSE_TARGET_SEARCH, index)?,
            );
            Ok(targeting::select(
                strategy,
                position,
                &eligible,
                draw(PURPOSE_TARGET_SEARCH, index + 1)?,
            ))
        };

        // Overrides expire at their deadline; a forced target holds only while eligible.
        let forced_target = self
            .forced_target
            .filter(|(actor, until)| facts.now_us < *until && find(*actor).is_some());
        let forced_distance = self.forced_distance.filter(|until| facts.now_us < *until);
        let override_holds = forced_target.is_some() || forced_distance.is_some();
        let attack_distance = if forced_distance.is_some() {
            1
        } else {
            u64::from(targeting.target_distance_tiles.max(1))
        };
        let in_range = |actor: ExactActorRef| {
            find(actor).is_some_and(|candidate| {
                targeting::plane_distance(position, candidate.position) <= attack_distance
            })
        };

        // Maintenance: an ineligible target is dropped.
        let mut target = forced_target
            .map(|(actor, _)| actor)
            .or(self.target)
            .filter(|actor| find(*actor).is_some());
        // Search when there is no target or it cannot be attacked from here.
        if forced_target.is_none()
            && target.is_none_or(|actor| !in_range(actor))
            && let Some(found) = strategy_search(0)?
        {
            target = Some(found);
        }
        // The timed change.
        let mut change_ticks_ms = self.change_ticks_ms;
        let mut target_changed = false;
        if let Some(change) = targeting.change_target
            && change.interval_ms > 0
            && target.is_some()
            && !override_holds
        {
            change_ticks_ms = change_ticks_ms.saturating_add(CREATURE_THINK_INTERVAL_MILLIS);
            if change_ticks_ms >= change.interval_ms {
                change_ticks_ms = 0;
                if draw(PURPOSE_TARGET_CHANGE, 0)? % PPM < u64::from(change.chance_ppm) {
                    target_changed = true;
                    let strategy = if attack_distance <= 1 {
                        Strategy::Random
                    } else {
                        Strategy::Nearest
                    };
                    if let Some(found) = targeting::select(
                        strategy,
                        position,
                        &eligible,
                        draw(PURPOSE_TARGET_SEARCH, 2)?,
                    ) {
                        target = Some(found);
                    }
                }
            }
        }
        // §2.1 item 6: flee while hurt and no override holds; the strategy draw picks whom
        // to flee from when the target cannot be attacked from here.
        let fleeing = !override_holds && facts.health <= targeting.flee_health;
        if fleeing
            && target.is_none_or(|actor| !in_range(actor))
            && let Some(found) = strategy_search(4)?
        {
            target = Some(found);
        }

        // §1.1, §1.4: one cardinal step.
        let target_position = target.and_then(find).map(|candidate| candidate.position);
        let mut wander_ticks_ms = self.wander_ticks_ms;
        let step = match target_position {
            Some(at) if fleeing => {
                let current = targeting::plane_distance(position, at);
                // The farthest admitted cell; ties keep the first in `CARDINALS` order.
                CARDINALS
                    .into_iter()
                    .enumerate()
                    .map(|(order, step)| (order, step, apply_step(position, step)))
                    .filter(|(_, _, destination)| {
                        targeting::plane_distance(*destination, at) > current
                            && admits(*destination)
                    })
                    .max_by_key(|(order, _, destination)| {
                        (
                            targeting::plane_distance(*destination, at),
                            std::cmp::Reverse(*order),
                        )
                    })
                    .map(|(_, step, destination)| CreatureStep {
                        kind: StepKind::Flee,
                        step,
                        destination,
                    })
            }
            Some(at) if targeting::plane_distance(position, at) > attack_distance => {
                step_towards(position, at)
                    .map(|step| (step, apply_step(position, step)))
                    .filter(|(_, destination)| admits(*destination))
                    .map(|(step, destination)| CreatureStep {
                        kind: StepKind::Chase,
                        step,
                        destination,
                    })
            }
            Some(_) => None,
            None => match self.profile.wander() {
                Some(wander) if !perceived.is_empty() && wander.interval_ms > 0 => {
                    wander_ticks_ms =
                        wander_ticks_ms.saturating_add(CREATURE_THINK_INTERVAL_MILLIS);
                    if wander_ticks_ms >= wander.interval_ms {
                        wander_ticks_ms = 0;
                        let index = usize::try_from(draw(PURPOSE_WANDER, 0)? % 4)
                            .map_err(|_| ThinkError::Draw)?;
                        let step = CARDINALS[index];
                        let destination = apply_step(position, step);
                        (chebyshev_distance(destination, self.anchor)
                            .is_some_and(|distance| distance <= i64::from(wander.radius_tiles))
                            && admits(destination))
                        .then_some(CreatureStep {
                            kind: StepKind::Wander,
                            step,
                            destination,
                        })
                    } else {
                        None
                    }
                }
                _ => None,
            },
        };

        // §2.1 item 5: the profile's non-melee entries at this think's target.
        let attack_target = target.zip(target_position).map(|(actor, at)| AttackTarget {
            actor,
            same_floor_distance: (at.floor == position.floor).then(|| {
                u16::try_from(targeting::plane_distance(position, at)).unwrap_or(u16::MAX)
            }),
        });
        let (proposals, refusal) = match self.schedule.prepare(
            occurrence,
            self.profile.behavior(),
            self.profile.abilities(),
            attack_target,
            facts.revisions,
            facts.root,
        ) {
            Ok(plan) => (plan.proposals, None),
            Err(error) => (Vec::new(), Some(error)),
        };

        let report = ThinkReport {
            sequence,
            target,
            target_changed,
            step,
            proposals,
            refusal,
            fleeing,
            idle: perceived.is_empty() && target.is_none() && !facts.has_active_condition,
            over_budget: false,
            units,
        };
        self.target = target;
        self.forced_target = forced_target.or(self
            .forced_target
            .filter(|(_, until)| facts.now_us < *until));
        self.forced_distance = forced_distance;
        self.change_ticks_ms = change_ticks_ms;
        self.wander_ticks_ms = wander_ticks_ms;
        self.last = Some(report.clone());
        Ok(report)
    }
}

/// One scheduled creature: its think state, its one pending think (`None` while idle) and the
/// sequence of its last think.
#[derive(Debug)]
struct CreatureAiEntry {
    state: CreatureAiState,
    due_us: Option<u64>,
    sequence: u64,
}

/// The Channel's creature thinks (§2.1 item 1, `CREATUREAI0-RL-05`). Each creature holds at most
/// one pending think (`AI01-PENDING-TIMERS-PER-ACTOR`); an idle creature holds none until a
/// player move or admission wakes it. At most `CREATUREAI0_THINKS_PER_WINDOW` thinks start per
/// `CREATUREAI0_THINK_WINDOW_MICROS` window; the rest stay due, in (due, placement) order, for
/// the next window. Ephemeral: confers no Channel owner authority.
#[derive(Debug, Default)]
pub struct CreatureAiTable {
    entries: BTreeMap<[u8; 16], CreatureAiEntry>,
    window: Option<u64>,
    window_started: usize,
}

impl CreatureAiTable {
    /// Admits `state` with its first think due at `now_us`. A creature already present is
    /// replaced.
    pub fn admit(&mut self, state: CreatureAiState, now_us: u64) {
        self.entries.insert(
            state.actor().placement_identity(),
            CreatureAiEntry {
                state,
                due_us: Some(now_us),
                sequence: 0,
            },
        );
    }

    /// Forgets a creature (death, despawn). Returns whether it was present.
    pub fn remove(&mut self, actor: ExactActorRef) -> bool {
        self.entries.remove(&actor.placement_identity()).is_some()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[must_use]
    pub fn state(&self, actor: ExactActorRef) -> Option<&CreatureAiState> {
        self.entry(actor).map(|entry| &entry.state)
    }

    /// The creature's pending think, `None` while idle or absent.
    #[must_use]
    pub fn due(&self, actor: ExactActorRef) -> Option<u64> {
        self.entry(actor).and_then(|entry| entry.due_us)
    }

    /// The number of pending thinks; at most one per creature.
    #[must_use]
    pub fn pending(&self) -> usize {
        self.entries
            .values()
            .filter(|entry| entry.due_us.is_some())
            .count()
    }

    /// §1.6. Returns whether the creature is present.
    pub fn set_override(&mut self, actor: ExactActorRef, value: CreatureAiOverride) -> bool {
        self.entry_mut(actor)
            .map(|entry| entry.state.set_override(value))
            .is_some()
    }

    /// §2.1 item 1: a player at `player` moved or was admitted. Each idle creature whose
    /// current position (`position_of`, the owner's read) the player sees gets a think due at
    /// `now_us`; at most `CREATUREAI1_WAKE_CANDIDATES`, nearest first. Returns the woken.
    pub fn wake(
        &mut self,
        player: MovementLocalPosition,
        position_of: impl Fn(ExactActorRef) -> Option<MovementLocalPosition>,
        now_us: u64,
    ) -> Vec<ExactActorRef> {
        let mut seen = self
            .entries
            .iter()
            .filter(|(_, entry)| entry.due_us.is_none())
            .filter_map(|(identity, entry)| {
                let position = position_of(entry.state.actor())?;
                targeting::sees(player, position)
                    .then(|| (targeting::plane_distance(player, position), *identity))
            })
            .collect::<Vec<_>>();
        seen.sort_unstable();
        seen.truncate(CREATUREAI1_WAKE_CANDIDATES);
        seen.into_iter()
            .filter_map(|(_, identity)| {
                let entry = self.entries.get_mut(&identity)?;
                entry.due_us = Some(now_us);
                Some(entry.state.actor())
            })
            .collect()
    }

    /// The thinks to start at `now_us` under `CREATUREAI0-RL-05`, in (due, placement) order.
    /// Each is counted against the window when returned.
    pub fn take_due(&mut self, now_us: u64) -> Vec<ExactActorRef> {
        let window = now_us / CREATUREAI0_THINK_WINDOW_MICROS;
        if self.window != Some(window) {
            self.window = Some(window);
            self.window_started = 0;
        }
        let mut due = self
            .entries
            .iter()
            .filter_map(|(identity, entry)| {
                entry
                    .due_us
                    .filter(|due| *due <= now_us)
                    .map(|due| (due, *identity, entry.state.actor()))
            })
            .collect::<Vec<_>>();
        due.sort_by_key(|(due, identity, _)| (*due, *identity));
        due.truncate(CREATUREAI0_THINKS_PER_WINDOW.saturating_sub(self.window_started));
        self.window_started += due.len();
        due.into_iter().map(|(_, _, actor)| actor).collect()
    }

    /// Runs one taken think of `actor`: the next sequence, then the next think one interval
    /// later, or none when the think is idle. A failed think keeps its sequence and retries one
    /// interval later.
    pub fn think(
        &mut self,
        actor: ExactActorRef,
        facts: &CreatureThinkFacts<'_>,
        admits: impl Fn(MovementLocalPosition) -> bool,
    ) -> Option<Result<ThinkReport, ThinkError>> {
        let entry = self.entry_mut(actor)?;
        let sequence = entry.sequence.saturating_add(1);
        let report = entry.state.think(sequence, facts, admits);
        let next = facts
            .now_us
            .saturating_add(CREATURE_THINK_INTERVAL_MILLIS.saturating_mul(1_000));
        entry.due_us = Some(next);
        if let Ok(report) = &report {
            entry.sequence = sequence;
            if report.idle {
                entry.due_us = None;
            }
        }
        Some(report)
    }

    fn entry(&self, actor: ExactActorRef) -> Option<&CreatureAiEntry> {
        self.entries
            .get(&actor.placement_identity())
            .filter(|entry| entry.state.actor() == actor)
    }

    fn entry_mut(&mut self, actor: ExactActorRef) -> Option<&mut CreatureAiEntry> {
        self.entries
            .get_mut(&actor.placement_identity())
            .filter(|entry| entry.state.actor() == actor)
    }
}

/// A caller-supplied stable identity for a perceived player. `ExactActorRef`'s fields and
/// constructor are private to `foundation` by design ("never decoded from a client handle"), so
/// this module cannot derive a numeric identity from one itself; the caller (the future
/// production owner-snapshot builder, or a test) supplies one however it has a persistent player
/// identity available.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PerceivedPlayerId(u64);

impl PerceivedPlayerId {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// §4.6's future Ability boundary and §4.7's HP floor are not wired yet (AI-4, spell P3b-2), so
/// bite cooldown/chance arrive as caller-supplied facts rather than being read from a live
/// vitals/cooldown store that does not exist yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttackReadiness {
    pub off_cooldown: bool,
    pub chance_percent: u8,
}

/// §4.4's re-entry protection note ("a player inside the active four-second PvE re-entry
/// protection window ... is not a legal attack target. The creature may still perceive and
/// chase that player") arrives as `legal_attack_target`: the composition root's own facts
/// arrival, since no protection-window fact source is wired yet either.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PerceivedPlayer {
    pub id: PerceivedPlayerId,
    pub position: MovementLocalPosition,
    pub legal_attack_target: bool,
}

/// One creature's owner snapshot for one think (§4.4). `decision_root`/`occurrence` bind the
/// think's randomness to this exact think occurrence; `path_work_id` is the chase tier's
/// `PathRequest::work_id`.
#[derive(Clone)]
pub struct CreatureThinkInput {
    pub provenance: AiProvenance,
    pub decision_root: GameplayDecisionRoot,
    pub occurrence: DecisionOccurrenceId,
    pub path_work_id: u64,
    pub position: MovementLocalPosition,
    pub home: MovementLocalPosition,
    pub attack: AttackReadiness,
}

/// §4.4's per-think resolution: at most one action is proposed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThinkOutcome {
    /// A successful bite-chance draw against an adjacent, legal, off-cooldown target. Never
    /// executed by this module (AI-4 issues it through Ability).
    AttackIntent(PerceivedPlayerId),
    ChaseStep(CardinalStep),
    WanderStep(CardinalStep),
    Idle,
}

pub(crate) fn chebyshev_distance(
    from: MovementLocalPosition,
    to: MovementLocalPosition,
) -> Option<i64> {
    if from.floor != to.floor {
        return None;
    }
    let dx = i64::from(to.x) - i64::from(from.x);
    let dy = i64::from(to.y) - i64::from(from.y);
    Some(dx.abs().max(dy.abs()))
}

const fn step_delta(step: CardinalStep) -> (i32, i32) {
    match step {
        CardinalStep::North => (0, -1),
        CardinalStep::East => (1, 0),
        CardinalStep::South => (0, 1),
        CardinalStep::West => (-1, 0),
    }
}

fn apply_step(position: MovementLocalPosition, step: CardinalStep) -> MovementLocalPosition {
    let (dx, dy) = step_delta(step);
    MovementLocalPosition {
        x: position.x.saturating_add(dx),
        y: position.y.saturating_add(dy),
        floor: position.floor,
    }
}

/// One greedy cardinal step reducing distance to `to` (§4.5: production pathfinding is out of
/// scope, §7; the "path" this decision proposes is always exactly one step). Ties prefer the
/// axis with the larger remaining delta, then the horizontal axis: a fixed, deterministic
/// tie-break, not a claim of an optimal or Reference route.
pub(crate) fn step_towards(
    from: MovementLocalPosition,
    to: MovementLocalPosition,
) -> Option<CardinalStep> {
    if from.floor != to.floor {
        return None;
    }
    let dx = i64::from(to.x) - i64::from(from.x);
    let dy = i64::from(to.y) - i64::from(from.y);
    if dx == 0 && dy == 0 {
        return None;
    }
    Some(if dx.abs() >= dy.abs() {
        if dx > 0 {
            CardinalStep::East
        } else {
            CardinalStep::West
        }
    } else if dy > 0 {
        CardinalStep::South
    } else {
        CardinalStep::North
    })
}

/// What actually happened after a `ThinkOutcome` was applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThinkAction {
    /// Surfaced only; AI-4 issues it through Ability. Out of this module's mutation surface.
    AttackIntentSurfaced(PerceivedPlayerId),
    Moved(MovementLocalPosition),
    Idle,
}

/// §4.4/§4.5: chase and wander act through the real Movement owner (`crate::movement::
/// step_cardinal`, the same revalidating/committing path a player's step takes); attack is only
/// surfaced. A rejected movement proposal (blocked, stale snapshot, scope mismatch, ...) mutates
/// nothing (`step_cardinal`'s own contract) and is returned as an error for the caller to log and
/// proceed to scheduling the next think, per §4.4: "the owner rejects every proposal it cannot
/// legally apply; a rejection changes nothing except scheduling the next think."
pub fn act_step(
    outcome: ThinkOutcome,
    owner: &mut CurrentOwnerMovementPosition<'_>,
    actor: ExactActorRef,
    expected: MovementPositionSnapshot,
    selection: &MovementEngineeringSelection<'_>,
    index: &(impl crate::content::native_cell_lookup::NativeStaticCellLookup + ?Sized),
) -> Result<ThinkAction, MovementError> {
    match outcome {
        ThinkOutcome::AttackIntent(target) => Ok(ThinkAction::AttackIntentSurfaced(target)),
        ThinkOutcome::Idle => Ok(ThinkAction::Idle),
        ThinkOutcome::ChaseStep(step) | ThinkOutcome::WanderStep(step) => {
            let snapshot = step_cardinal(owner, actor, expected, selection, index, step)?;
            Ok(ThinkAction::Moved(snapshot.position()))
        }
    }
}

/// §4.2: the AI think timer family. Its registered ceiling is AI-1's
/// `AI01_PENDING_TIMERS_PER_ACTOR` (`RESOURCE_LIMITS_REGISTRY.json`, AI-RL-06); this is the
/// first concrete `TimerFamily` checked against it outside `owner_timer`'s own tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThinkFamily;

impl TimerFamily for ThinkFamily {
    fn registered_maximum(self) -> usize {
        AI01_PENDING_TIMERS_PER_ACTOR
    }
}

/// §4.2: "a think occurrence is (creature `ExactActorRef`, per-actor think sequence number)."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThinkOccurrence {
    pub actor: ExactActorRef,
    pub sequence: u64,
}

/// Binding item (a): the only place a `ThinkOccurrence`'s sequence is derived from. It advances
/// monotonically per actor and never re-issues a value, so `(actor, sequence)` can never repeat
/// across this tracker's lifetime. `ExactActorRef` derives no `Hash` (its fields are private to
/// `foundation`), so this is a small linear table rather than a hash map -- bounded by the D57
/// envelope (64 live creatures), not a concern at that size.
///
/// Bounded across respawns (Codex P2 on `#1196`): each respawn's new actor-local generation is a
/// distinct `ExactActorRef` (`foundation`'s own doc: "never decoded from a client handle", so
/// this module cannot itself compare two refs' generations to tell "the same slot, a newer
/// generation" from "an unrelated actor" -- `ExactActorRef`'s fields are private to
/// `foundation`). Without `retire`, a dead generation's entry would never be removed and the
/// table would grow one stale entry per past generation forever. `retire` is the bounded
/// alternative: the owner (which does hold generation identity) calls it on that actor's death
/// or despawn, so the entry count tracks currently live/pending actors, never total history.
#[derive(Debug, Default)]
pub struct ThinkSequenceTracker {
    next: Vec<(ExactActorRef, u64)>,
}

impl ThinkSequenceTracker {
    #[must_use]
    pub const fn new() -> Self {
        Self { next: Vec::new() }
    }

    pub fn next_occurrence(&mut self, actor: ExactActorRef) -> ThinkOccurrence {
        if let Some(entry) = self
            .next
            .iter_mut()
            .find(|(candidate, _)| *candidate == actor)
        {
            let sequence = entry.1;
            entry.1 = entry.1.saturating_add(1);
            ThinkOccurrence { actor, sequence }
        } else {
            self.next.push((actor, 1));
            ThinkOccurrence { actor, sequence: 0 }
        }
    }

    /// Removes `actor`'s entry, if any. The owner calls this exactly once per actor death or
    /// despawn (before or independent of a respawn's fresh `ExactActorRef`), which is what
    /// bounds this tracker's size to currently live/pending actors. A safe no-op (returns
    /// `false`, changes nothing) for an actor that was never scheduled or was already retired.
    pub fn retire(&mut self, actor: ExactActorRef) -> bool {
        let before = self.next.len();
        self.next.retain(|(candidate, _)| *candidate != actor);
        self.next.len() != before
    }

    /// The number of actors this tracker currently holds a sequence for. Bounded by the D57
    /// envelope (64) as long as the owner retires every dead/despawned actor.
    #[must_use]
    pub fn len(&self) -> usize {
        self.next.len()
    }

    /// Whether this tracker currently holds no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.next.is_empty()
    }
}

/// Binding item (b): inherited from `OwnerTimerLane::schedule` itself, never re-implemented
/// here. `scheduling_stamp` must already have been issued by `owner_fence` for the current
/// resolution; this wrapper fabricates no stamp of its own and forwards exactly what the
/// caller's fence vouches for, so `schedule`'s own `accepts_stamp` check is the sole enforcement
/// point. See the module doc for why this cannot be exercised end-to-end from this task's own
/// tests (`ScopeRuntimeFence` has no accessible constructor outside `foundation`).
pub fn schedule_next_think(
    lane: &mut OwnerTimerLane<ThinkFamily, ThinkOccurrence>,
    owner_fence: &ScopeRuntimeFence,
    scheduling_stamp: RuntimeWorkStamp,
    tracker: &mut ThinkSequenceTracker,
    actor: ExactActorRef,
    due: SemanticTimeMicros,
) -> Result<ThinkOccurrence, OwnerTimerError> {
    let occurrence = tracker.next_occurrence(actor);
    lane.schedule(
        owner_fence,
        scheduling_stamp,
        ThinkFamily,
        occurrence,
        Some(actor),
        due,
    )?;
    Ok(occurrence)
}

#[cfg(test)]
mod think_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::static_cell_engine::{
        EngineeringCollisionClaim, EngineeringStaticCellClaim, EngineeringStaticCellScope,
    };
    use crate::content::{
        CollisionClass, ContentLockBinding, ContentLockEntry, CoordinateFrameRef, LogicalCell,
        MapRevisionRef, ProductionAtom, ProductionKey, Sha256HexDigest,
    };
    use crate::foundation::owner_timer::{CatchUpPolicy, FamilyPolicy};
    use crate::foundation::{MovementActorFixture, RuntimeScopeRefV1, ScopeOwnershipGeneration};
    use std::error::Error;

    fn position(x: i32, y: i32) -> MovementLocalPosition {
        MovementLocalPosition { x, y, floor: 7 }
    }

    #[test]
    fn think_sequence_tracker_never_repeats_and_is_independent_per_actor() {
        let actor_a = ExactActorRef::transport_fixture(world_id(), channel_id(1));
        let actor_b = ExactActorRef::transport_fixture(world_id(), channel_id(2));
        let mut tracker = ThinkSequenceTracker::new();
        let first = tracker.next_occurrence(actor_a);
        let second = tracker.next_occurrence(actor_a);
        let third = tracker.next_occurrence(actor_b);
        assert_eq!(first.sequence, 0);
        assert_eq!(second.sequence, 1);
        assert_ne!(
            (first.actor, first.sequence),
            (second.actor, second.sequence)
        );
        assert_eq!(third.sequence, 0);
        assert_eq!(third.actor, actor_b);
    }

    #[test]
    fn think_sequence_tracker_sequence_is_monotonic_within_one_generation() {
        let actor = ExactActorRef::transport_fixture(world_id(), channel_id(1));
        let mut tracker = ThinkSequenceTracker::new();
        let first = tracker.next_occurrence(actor);
        let second = tracker.next_occurrence(actor);
        let third = tracker.next_occurrence(actor);
        assert_eq!([first.sequence, second.sequence, third.sequence], [0, 1, 2]);
        assert_eq!(
            tracker.len(),
            1,
            "still one entry: the same live generation"
        );
    }

    #[test]
    fn think_sequence_tracker_stays_bounded_across_many_respawn_generations_of_one_slot() {
        // Codex P2 on #1196: without `retire`, this tracker would grow one stale entry per past
        // generation forever. Each iteration here stands in for one respawn's fresh
        // `ExactActorRef` (a distinct generation is, by construction, a distinct ref) of what is
        // conceptually the same spawn slot: schedule its think, then retire it on death, exactly
        // as the owner would across many respawn cycles of one cell.
        let mut tracker = ThinkSequenceTracker::new();
        for generation_seed in 1_u8..=200 {
            let actor = ExactActorRef::transport_fixture(world_id(), channel_id(generation_seed));
            let occurrence = tracker.next_occurrence(actor);
            assert_eq!(
                occurrence.sequence, 0,
                "a fresh generation starts a new sequence"
            );
            assert_eq!(
                tracker.len(),
                1,
                "at most one entry while this generation is live"
            );
            assert!(
                tracker.retire(actor),
                "retire removes the dead generation's entry"
            );
        }
        assert!(
            tracker.is_empty(),
            "no stale entries survive 200 respawn generations"
        );
    }

    #[test]
    fn think_sequence_tracker_retire_is_a_safe_no_op_for_an_untracked_or_already_retired_actor() {
        let tracked = ExactActorRef::transport_fixture(world_id(), channel_id(1));
        let untracked = ExactActorRef::transport_fixture(world_id(), channel_id(2));
        let mut tracker = ThinkSequenceTracker::new();
        assert!(!tracker.retire(untracked), "nothing was ever scheduled");

        tracker.next_occurrence(tracked);
        assert!(tracker.retire(tracked));
        assert!(
            !tracker.retire(tracked),
            "retiring an already-retired actor changes nothing"
        );
        assert!(tracker.is_empty());
    }

    #[test]
    fn think_family_registered_maximum_matches_ai01_pending_timers_per_actor() {
        assert_eq!(
            ThinkFamily.registered_maximum(),
            AI01_PENDING_TIMERS_PER_ACTOR
        );
    }

    #[test]
    fn think_family_lane_construction_admits_the_maximum_and_rejects_maximum_plus_one() {
        let scope = RuntimeScopeRefV1::channel(world_id(), channel_id(9));
        let generation = generation(1);
        let at_maximum = OwnerTimerLane::<ThinkFamily, ThinkOccurrence>::for_generation(
            scope,
            generation,
            [(
                ThinkFamily,
                FamilyPolicy {
                    max_pending: AI01_PENDING_TIMERS_PER_ACTOR,
                    catch_up: CatchUpPolicy::SkipToLatest,
                },
            )],
        );
        assert!(at_maximum.is_ok());

        let over_maximum = OwnerTimerLane::<ThinkFamily, ThinkOccurrence>::for_generation(
            scope,
            generation,
            [(
                ThinkFamily,
                FamilyPolicy {
                    max_pending: AI01_PENDING_TIMERS_PER_ACTOR + 1,
                    catch_up: CatchUpPolicy::SkipToLatest,
                },
            )],
        );
        assert_eq!(
            over_maximum.err(),
            Some(OwnerTimerError::FamilyCapExceedsRegisteredMaximum)
        );
    }

    fn generation(value: u64) -> ScopeOwnershipGeneration {
        match ScopeOwnershipGeneration::new(value) {
            Ok(generation) => generation,
            Err(_) => unreachable!("test-fixed nonzero generation"),
        }
    }

    fn uuid_v7(seed: u8) -> [u8; 16] {
        let mut bytes = [0_u8; 16];
        bytes[0] = 1;
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        bytes[15] = seed;
        bytes
    }

    fn world_id() -> crate::foundation::WorldId {
        match crate::foundation::WorldId::decode(&uuid_v7(1)) {
            Ok(id) => id,
            Err(_) => unreachable!("test-fixed UUIDv7 WorldId"),
        }
    }

    fn channel_id(seed: u8) -> crate::foundation::ChannelId {
        match crate::foundation::ChannelId::decode(&uuid_v7(seed)) {
            Ok(id) => id,
            Err(_) => unreachable!("test-fixed UUIDv7 ChannelId"),
        }
    }

    fn cell_scope(
        for_world: crate::foundation::WorldId,
    ) -> Result<EngineeringStaticCellScope, Box<dyn Error>> {
        Ok(EngineeringStaticCellScope {
            world_id: for_world,
            coordinate_frame: CoordinateFrameRef::new("ai-think-engineering-frame")?,
            map_revision: MapRevisionRef::new("ai-think-engineering-map")?,
            generation_digest: [9; 32],
            content_lock: ContentLockBinding {
                revision_digest_token: ProductionAtom::new("lock", "ai-think-engineering-lock")?,
                entries: vec![ContentLockEntry::exact(
                    ProductionKey::new("engineering:ai-think")?,
                    ProductionAtom::new("revision", "ai-think-engineering-r1")?,
                    Sha256HexDigest::new(&"b".repeat(64))?,
                )],
            },
        })
    }

    fn walkable_index(
        scope: &EngineeringStaticCellScope,
        cells: &[(i32, i32, i32)],
    ) -> Result<EngineeringStaticCellIndex, Box<dyn Error>> {
        Ok(EngineeringStaticCellIndex::from_claims(
            cells
                .iter()
                .map(|(x, y, z)| EngineeringStaticCellClaim {
                    scope: scope.clone(),
                    cell: LogicalCell {
                        x: *x,
                        y: *y,
                        z: *z,
                    },
                    collision: EngineeringCollisionClaim::Qualified(CollisionClass::Walkable),
                })
                .collect(),
        )?)
    }

    #[test]
    fn act_step_chase_moves_a_creature_actor_through_the_real_movement_owner()
    -> Result<(), Box<dyn Error>> {
        // §4.5: creature slots are Movement-capable through the same owner turn a player's step
        // uses; this exercises exactly that path for AI-3's own composed step.
        let mut creature = MovementActorFixture::new(position(4, 5), true)?;
        let scope = cell_scope(creature.world_id())?;
        let index = walkable_index(&scope, &[(5, 5, 7)])?;
        let actor = creature.actor();
        let before = creature.current_position()?;
        let selection = MovementEngineeringSelection {
            owner_context: before.context(),
            content_scope: &scope,
        };
        let outcome = act_step(
            ThinkOutcome::ChaseStep(CardinalStep::East),
            &mut creature.borrow_position(),
            actor,
            before,
            &selection,
            &index,
        )?;
        assert_eq!(outcome, ThinkAction::Moved(position(5, 5)));
        assert_eq!(creature.current_position()?.position(), position(5, 5));
        Ok(())
    }

    #[test]
    fn act_step_wander_moves_a_creature_actor_through_the_real_movement_owner()
    -> Result<(), Box<dyn Error>> {
        let mut creature = MovementActorFixture::new(position(4, 5), true)?;
        let scope = cell_scope(creature.world_id())?;
        let index = walkable_index(&scope, &[(4, 4, 7)])?;
        let actor = creature.actor();
        let before = creature.current_position()?;
        let selection = MovementEngineeringSelection {
            owner_context: before.context(),
            content_scope: &scope,
        };
        let outcome = act_step(
            ThinkOutcome::WanderStep(CardinalStep::North),
            &mut creature.borrow_position(),
            actor,
            before,
            &selection,
            &index,
        )?;
        assert_eq!(outcome, ThinkAction::Moved(position(4, 4)));
        Ok(())
    }

    #[test]
    fn act_step_never_moves_for_attack_intent_or_idle() -> Result<(), Box<dyn Error>> {
        let mut creature = MovementActorFixture::new(position(4, 5), true)?;
        let scope = cell_scope(creature.world_id())?;
        let index = walkable_index(&scope, &[(5, 5, 7)])?;
        let actor = creature.actor();
        let before = creature.current_position()?;
        let selection = MovementEngineeringSelection {
            owner_context: before.context(),
            content_scope: &scope,
        };
        let idle = act_step(
            ThinkOutcome::Idle,
            &mut creature.borrow_position(),
            actor,
            before,
            &selection,
            &index,
        )?;
        assert_eq!(idle, ThinkAction::Idle);
        let attack = act_step(
            ThinkOutcome::AttackIntent(PerceivedPlayerId::new(1)),
            &mut creature.borrow_position(),
            actor,
            before,
            &selection,
            &index,
        )?;
        assert_eq!(
            attack,
            ThinkAction::AttackIntentSurfaced(PerceivedPlayerId::new(1))
        );
        assert_eq!(creature.current_position()?, before, "no movement mutation");
        Ok(())
    }

    #[test]
    fn act_step_propagates_a_blocked_movement_error_without_mutating() -> Result<(), Box<dyn Error>>
    {
        let mut creature = MovementActorFixture::new(position(4, 5), true)?;
        let scope = cell_scope(creature.world_id())?;
        let index = EngineeringStaticCellIndex::from_claims(vec![EngineeringStaticCellClaim {
            scope: scope.clone(),
            cell: LogicalCell { x: 5, y: 5, z: 7 },
            collision: EngineeringCollisionClaim::Qualified(CollisionClass::Blocked),
        }])?;
        let actor = creature.actor();
        let before = creature.current_position()?;
        let selection = MovementEngineeringSelection {
            owner_context: before.context(),
            content_scope: &scope,
        };
        let result = act_step(
            ThinkOutcome::ChaseStep(CardinalStep::East),
            &mut creature.borrow_position(),
            actor,
            before,
            &selection,
            &index,
        );
        assert_eq!(result, Err(MovementError::Blocked));
        assert_eq!(creature.current_position()?, before);
        Ok(())
    }
}
