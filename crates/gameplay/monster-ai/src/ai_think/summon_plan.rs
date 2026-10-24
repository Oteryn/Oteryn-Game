//! One targeting pass followed by the native attack/defence/summon schedule.
pub mod summon_path;
use super::profile_schedule::{MonsterSummonFacts, ProfileScheduleInput};
use super::*;
use crate::foundation::ChannelRuntimeV1;

pub trait MonsterThinkReader {
    /// Resolve only the selected target under the current Map/Item transaction. Unknown
    /// must return None, which refuses before the defence/summon clock advances.
    fn summon_facts(
        &mut self,
        runtime: &ChannelRuntimeV1,
        occurrence: ThinkOccurrence,
        target: Option<ExactActorRef>,
        route: Option<&[MovementLocalPosition]>,
    ) -> impl Future<Output = Option<MonsterSummonFacts>> + Send;
}

pub(super) struct PreparedCreatureThink {
    report: ThinkReport,
    attack_target: Option<AttackTarget>,
    forced_target: Option<(ExactActorRef, u64)>,
    forced_distance: Option<u64>,
    change_ticks_ms: u64,
}

impl CreatureAiState {
    pub(super) fn select_think(
        &self,
        sequence: u64,
        facts: &CreatureThinkFacts<'_>,
    ) -> Result<PreparedCreatureThink, ThinkError> {
        if self
            .last
            .as_ref()
            .is_some_and(|last| sequence < last.sequence)
        {
            return Err(ThinkError::OccurrenceSuperseded);
        }
        if let Some(last) = &self.last
            && sequence == last.sequence
        {
            let attack_target = last.target.and_then(|actor| {
                let at = facts
                    .players
                    .iter()
                    .find(|candidate| candidate.actor == actor)?
                    .position;
                Some(AttackTarget {
                    actor,
                    same_floor_distance: (at.floor == facts.position.floor).then(|| {
                        u16::try_from(targeting::plane_distance(facts.position, at))
                            .unwrap_or(u16::MAX)
                    }),
                })
            });
            return Ok(PreparedCreatureThink {
                report: last.clone(),
                attack_target,
                forced_target: self.forced_target,
                forced_distance: self.forced_distance,
                change_ticks_ms: self.change_ticks_ms,
            });
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
            return Ok(PreparedCreatureThink {
                report: ThinkReport {
                    sequence,
                    target: self.target,
                    target_changed: false,
                    goal: MovementGoal::Stand,
                    proposals: Vec::new(),
                    summon_proposals: Vec::new(),
                    refusal: None,
                    fleeing: false,
                    idle: false,
                    over_budget: true,
                    units,
                },
                attack_target: None,
                forced_target: self.forced_target,
                forced_distance: self.forced_distance,
                change_ticks_ms: self.change_ticks_ms,
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
        // CREATURE-AI-0 §4.3: the ordinary search takes the nearest eligible candidate when
        // there is no target or it cannot be attacked from here, and never while an override
        // holds.
        if !override_holds
            && (target.is_none() || self.unreachable)
            && let Some(found) = targeting::select(
                Strategy::Nearest,
                position,
                &eligible,
                draw(PURPOSE_TARGET_SEARCH, 0)?,
            )
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

        let target_position = target.and_then(find).map(|candidate| candidate.position);
        let goal = self.choose_goal(
            position,
            target.zip(target_position),
            fleeing,
            attack_distance,
            !perceived.is_empty(),
        );

        // §2.1 item 5: the profile's non-melee entries at this think's target.
        let attack_target = target.zip(target_position).map(|(actor, at)| AttackTarget {
            actor,
            same_floor_distance: (at.floor == position.floor).then(|| {
                u16::try_from(targeting::plane_distance(position, at)).unwrap_or(u16::MAX)
            }),
        });

        Ok(PreparedCreatureThink {
            report: ThinkReport {
                sequence,
                target,
                target_changed,
                goal,
                proposals: Vec::new(),
                summon_proposals: Vec::new(),
                refusal: None,
                fleeing,
                idle: perceived.is_empty()
                    && target.is_none()
                    && !facts.has_active_condition
                    && !matches!(goal, MovementGoal::WalkBack { .. }),
                over_budget: false,
                units,
            },
            attack_target,
            forced_target: forced_target.or(self
                .forced_target
                .filter(|(_, until)| facts.now_us < *until)),
            forced_distance,
            change_ticks_ms,
        })
    }

    fn finish_think(
        &mut self,
        mut selected: PreparedCreatureThink,
        facts: &CreatureThinkFacts<'_>,
        summons: Option<&MonsterSummonFacts>,
    ) -> ThinkReport {
        if selected.report.over_budget {
            return selected.report;
        }
        selected.report.proposals.clear();
        selected.report.summon_proposals.clear();
        selected.report.refusal = None;
        let occurrence = ThinkOccurrence {
            actor: self.actor,
            sequence: selected.report.sequence,
        };
        match self.schedule.prepare_with_summons(
            ProfileScheduleInput {
                occurrence,
                behavior: self.profile.behavior(),
                abilities: self.profile.abilities(),
                target: selected.attack_target,
                revisions: facts.revisions,
                root: facts.root,
            },
            summons,
        ) {
            Ok(plan) => {
                selected.report.proposals = plan.proposals;
                selected.report.summon_proposals = plan.summon_proposals;
            }
            Err(error) => selected.report.refusal = Some(error),
        }
        self.target = selected.report.target;
        self.unreachable = false;
        self.forced_target = selected.forced_target;
        self.forced_distance = selected.forced_distance;
        self.change_ticks_ms = selected.change_ticks_ms;
        self.last = Some(selected.report.clone());
        selected.report
    }

    pub(super) async fn think_current(
        &mut self,
        sequence: u64,
        facts: &CreatureThinkFacts<'_>,
        runtime: &ChannelRuntimeV1,
        reader: &mut impl MonsterThinkReader,
        route_hint: Option<(ExactActorRef, Vec<MovementLocalPosition>)>,
    ) -> Result<ThinkReport, ThinkError> {
        let selected = self.select_think(sequence, facts)?;
        let summons = if self.profile.behavior().summons.is_some() && !selected.report.over_budget {
            reader
                .summon_facts(
                    runtime,
                    ThinkOccurrence {
                        actor: self.actor,
                        sequence,
                    },
                    selected.report.target,
                    route_hint
                        .as_ref()
                        .filter(|(target, _)| Some(*target) == selected.report.target)
                        .map(|(_, route)| route.as_slice()),
                )
                .await
        } else {
            None
        };
        Ok(self.finish_think(selected, facts, summons.as_ref()))
    }

    pub(super) fn think_legacy(
        &mut self,
        sequence: u64,
        facts: &CreatureThinkFacts<'_>,
    ) -> Result<ThinkReport, ThinkError> {
        if let Some(last) = &self.last {
            if sequence < last.sequence {
                return Err(ThinkError::OccurrenceSuperseded);
            }
            if sequence == last.sequence {
                return Ok(last.clone());
            }
        }
        let selected = self.select_think(sequence, facts)?;
        Ok(self.finish_think(selected, facts, None))
    }
}

impl CreatureAiTable {
    /// Production one-pass thinking. Awaited Map reads precede sequence/deadline publication.
    #[allow(clippy::too_many_arguments)]
    pub async fn run_due_current_thinks<R: MonsterThinkReader>(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        now_us: u64,
        root: &GameplayDecisionRoot,
        revisions: &RevisionSet,
        players: &[TargetCandidate],
        reader: &mut R,
        mut consume: impl FnMut(&mut ChannelRuntimeV1, &mut Self, ExactActorRef, &ThinkReport, &mut R),
    ) -> Vec<(ExactActorRef, ThinkReport)> {
        if runtime.owner_fence().is_err() {
            return Vec::new();
        }
        let (taken, _) = self.take_due_bounded(now_us);
        let mut reports = Vec::new();
        for actor in taken {
            let Some(health) = runtime.live_creature_health(actor) else {
                self.remove(actor);
                continue;
            };
            let Ok(position) = runtime.read_actor_position(actor) else {
                self.remove(actor);
                continue;
            };
            let route = self.retained_summon_route(runtime, actor);
            let Some(entry) = self.entry_mut(actor) else {
                continue;
            };
            let facts = CreatureThinkFacts {
                position: position.position(),
                health,
                has_active_condition: runtime
                    .actor_conditions(actor, None)
                    .is_ok_and(|store| !store.instances().is_empty()),
                players,
                now_us,
                root,
                revisions,
            };
            let sequence = entry.sequence.saturating_add(1);
            if let Ok(report) = entry
                .state
                .think_current(sequence, &facts, runtime, reader, route)
                .await
            {
                entry.sequence = sequence;
                entry.due_us = (!report.idle)
                    .then_some(now_us.saturating_add(CREATURE_THINK_INTERVAL_MILLIS * 1000));
                consume(runtime, self, actor, &report, reader);
                reports.push((actor, report));
            }
        }
        reports
    }
}
