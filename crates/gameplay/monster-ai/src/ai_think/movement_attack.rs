//! Crystal creature.cpp:488-495 and game.cpp:6996-7000: movement may retry a pending
//! extra melee through onAttacking(0). It does not run an AI think or advance a timer.
use super::profile_schedule::NativeAttackFacts;
use super::*;
use crate::foundation::ChannelRuntimeV1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct AttackObservation {
    issuer: MovementPositionSnapshot,
    target: ExactActorRef,
    target_position: MovementPositionSnapshot,
}

impl AttackObservation {
    fn read(runtime: &ChannelRuntimeV1, state: &CreatureAiState) -> Option<Self> {
        let target = state.target?;
        let issuer = runtime.read_actor_position(state.actor).ok()?;
        let target_position = runtime.read_actor_position(target).ok()?;
        let context = runtime.pinned_movement_context();
        (issuer.context() == context && target_position.context() == context).then_some(Self {
            issuer,
            target,
            target_position,
        })
    }

    fn same_translation(self, previous: Self) -> bool {
        self.target == previous.target
            && self.issuer.context() == previous.issuer.context()
            && self.target_position.context() == previous.target_position.context()
            && self.issuer.last_translation_revision()
                == previous.issuer.last_translation_revision()
            && self.target_position.last_translation_revision()
                == previous.target_position.last_translation_revision()
    }

    fn still_visible_from(self, previous: Self) -> bool {
        let issuer = self.issuer.position();
        let target = self.target_position.position();
        issuer.floor == previous.issuer.position().floor
            && target.floor == previous.target_position.position().floor
            && issuer.floor == target.floor
            && targeting::sees(issuer, target)
    }
}

fn changed_pending_target(runtime: &ChannelRuntimeV1, state: &CreatureAiState) -> bool {
    state.profile.is_native()
        && state.schedule.extra_melee_pending()
        && state.attack_observation.is_some_and(|previous| {
            state.target == Some(previous.target)
                && AttackObservation::read(runtime, state)
                    .is_none_or(|current| !current.same_translation(previous))
        })
}

impl CreatureAiTable {
    /// Capture the current held target's complete position snapshots after a full think,
    /// before Movement may commit another step. An already pending movement observation is
    /// retained while an extra chance remains unconsumed, including an out-and-back step.
    /// The position owner's translation stamp distinguishes that from a turn in place.
    pub fn record_native_attack_positions(
        &mut self,
        actor: ExactActorRef,
        runtime: &ChannelRuntimeV1,
    ) {
        if runtime.owner_fence().is_err() {
            return;
        }
        let Some(entry) = self.entry_mut(actor) else {
            return;
        };
        if !entry.state.profile.is_native() {
            return;
        }
        if changed_pending_target(runtime, &entry.state) {
            return;
        }
        entry.state.attack_observation = AttackObservation::read(runtime, &entry.state);
    }

    /// Read-only wake predicate for the production pass's ordinary think deadline. Time
    /// alone and movement of an unrelated actor cannot request an extra melee evaluation.
    #[must_use]
    pub fn has_movement_melee_retry(&self, runtime: &ChannelRuntimeV1) -> bool {
        runtime.owner_fence().is_ok()
            && self.entries.values().any(|entry| {
                runtime.contains_live_creature(entry.state.actor)
                    && changed_pending_target(runtime, &entry.state)
            })
    }

    /// Observe only movement of an issuer or its held target, and select pending native
    /// melee at interval zero. Returned actors have a new source-index selection to commit
    /// immediately through the ordinary melee owner. This shares the existing evaluation
    /// window and per-creature occurrence sequence; it never changes a think deadline, full
    /// ThinkReport, target-change clock, movement goal, defence or summon clock.
    pub fn prepare_movement_melee_retries(
        &mut self,
        runtime: &ChannelRuntimeV1,
        players: &[TargetCandidate],
        now_us: u64,
    ) -> Vec<ExactActorRef> {
        if runtime.owner_fence().is_err() {
            return Vec::new();
        }
        let window = now_us / CREATUREAI0_THINK_WINDOW_MICROS;
        if self.window != Some(window) {
            self.window = Some(window);
            self.window_started = 0;
        }
        let candidates = players
            .iter()
            .map(|candidate| (candidate.actor.placement_identity(), candidate))
            .collect::<BTreeMap<_, _>>();
        let mut selected = Vec::new();
        for entry in self.entries.values_mut() {
            if self.window_started >= CREATUREAI0_THINKS_PER_WINDOW {
                break;
            }
            let state = &mut entry.state;
            if !changed_pending_target(runtime, state) {
                continue;
            }
            let Some(health) = runtime.live_creature_health(state.actor) else {
                continue;
            };
            let Some(previous) = state.attack_observation else {
                continue;
            };
            let current = AttackObservation::read(runtime, state);
            // The observation is consumed once even if reach, eligibility or chance refuses
            // this movement event. Repeated owner polls cannot redraw it.
            state.attack_observation = current;
            self.window_started += 1;
            let Some(current) = current.filter(|current| current.still_visible_from(previous))
            else {
                state.target = None;
                state.schedule.native_target_disappeared();
                continue;
            };
            let position = current.issuer.position();
            let target_position = current.target_position.position();
            let targeting = state.profile.targeting();
            let Some(candidate) =
                candidates
                    .get(&current.target.placement_identity())
                    .filter(|candidate| {
                        candidate.actor == current.target
                            && candidate.position == target_position
                            && targeting::is_eligible(targeting, position, candidate)
                    })
            else {
                continue;
            };
            if 2 + state.profile.entry_count() > CREATUREAI0_EVALUATION_UNITS_PER_THINK {
                continue;
            }
            let forced_target_holds = state.forced_target.is_some_and(|(actor, until)| {
                now_us < until
                    && candidates
                        .get(&actor.placement_identity())
                        .is_some_and(|candidate| {
                            candidate.actor == actor
                                && targeting::is_eligible(targeting, position, candidate)
                        })
            });
            let current_challenge = runtime
                .creature_combat_facts(state.actor, now_us / 1_000)
                .ok()
                .and_then(|facts| facts.monster_ai.and_then(|ai| ai.challenged_to))
                .is_some_and(|(_, until_ms)| now_us / 1_000 < until_ms);
            let fleeing = !state.is_summon
                && !forced_target_holds
                && !current_challenge
                && state.forced_distance.is_none_or(|until| now_us >= until)
                && health <= state.current_flee_health(runtime);
            let Some(sequence) = entry.sequence.checked_add(1) else {
                continue;
            };
            let plan = state.schedule.prepare_movement_melee_retry(
                ThinkOccurrence {
                    actor: state.actor,
                    sequence,
                },
                state.profile.behavior(),
                state.profile.abilities(),
                AttackTarget {
                    actor: candidate.actor,
                    same_floor_distance: Some(
                        u16::try_from(targeting::plane_distance(position, target_position))
                            .unwrap_or(u16::MAX),
                    ),
                },
                NativeAttackFacts {
                    now_us,
                    fleeing,
                    target_disappeared: false,
                },
            );
            entry.sequence = sequence;
            state.update_attack_facing(position, target_position);
            if plan.is_ok_and(|plan| !plan.native_melee_selected.is_empty()) {
                selected.push(state.actor);
            }
        }
        selected
    }
}
