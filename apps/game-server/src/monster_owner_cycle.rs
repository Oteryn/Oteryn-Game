//! Bounded owner-timer -> typed AI think adapter. This never grants scope ownership.
//! The Foundation-owned current mutable fence must be supplied independently.
use crate::ai_think::{
    D115_THINK_INTERVAL_MILLIS, ThinkFamily, ThinkOccurrence, ThinkSequenceTracker,
    schedule_next_think,
};
use crate::foundation::owner_timer::{
    AI01_PENDING_TIMERS_PER_ACTOR, CatchUpPolicy, FamilyPolicy, OwnerClock, OwnerTimerError,
    OwnerTimerLane, SemanticTimeMicros,
};
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GenerationError, RuntimeScopeRefV1, RuntimeWorkStamp,
    ScopeOwnershipGeneration, ScopeRuntimeFence,
};
use crate::gameplay_transport::actor_spell::ChannelSpellStates;

const AI01_CREATURES_PER_SCOPE: usize = 64; // Accepted AI decision §4.9:16sources×4population.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CycleError {
    StaleOwner,
    StaleActor,
    ActorLimit,
    Timer(OwnerTimerError),
    Ordinal(GenerationError),
}
#[derive(Debug)]
pub(crate) struct MonsterThinkLane {
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
    timers: OwnerTimerLane<ThinkFamily, ThinkOccurrence>,
    tracker: ThinkSequenceTracker,
    actors: Vec<ExactActorRef>,
}
impl MonsterThinkLane {
    pub(crate) fn new(
        runtime: &ChannelRuntimeV1,
        current: &ScopeRuntimeFence,
    ) -> Result<Self, CycleError> {
        let binding = runtime.binding();
        let scope = RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id());
        let generation = binding.scope_generation();
        if !current.is_current_for_scope(scope, generation) {
            return Err(CycleError::StaleOwner);
        }
        let timers = OwnerTimerLane::for_generation(
            scope,
            generation,
            [(
                ThinkFamily,
                FamilyPolicy {
                    max_pending: AI01_PENDING_TIMERS_PER_ACTOR,
                    catch_up: CatchUpPolicy::SkipToLatest,
                },
            )],
        )
        .map_err(CycleError::Timer)?;
        Ok(Self {
            scope,
            generation,
            timers,
            tracker: ThinkSequenceTracker::new(),
            actors: Vec::new(),
        })
    }
    fn current(&self, runtime: &ChannelRuntimeV1, current: &ScopeRuntimeFence) -> bool {
        let b = runtime.binding();
        self.scope == RuntimeScopeRefV1::channel(b.world_id(), b.channel_id())
            && self.generation == b.scope_generation()
            && current.is_current_for_scope(self.scope, self.generation)
    }
    fn prune(&mut self, runtime: &ChannelRuntimeV1) {
        self.actors.retain(|actor| {
            if runtime.contains_live_creature(*actor) {
                true
            } else {
                self.tracker.retire(*actor);
                false
            }
        });
    }
    /// Initial scheduling is its own accepted owner input. Mint its stamp only from
    /// the supplied existing current mutable fence, never from an unrelated receipt.
    pub(crate) fn schedule(
        &mut self,
        runtime: &ChannelRuntimeV1,
        current: &mut ScopeRuntimeFence,
        actor: ExactActorRef,
        due: SemanticTimeMicros,
    ) -> Result<ThinkOccurrence, CycleError> {
        if !self.current(runtime, current) {
            return Err(CycleError::StaleOwner);
        }
        if !runtime.contains_live_creature(actor) {
            return Err(CycleError::StaleActor);
        }
        if self.timers.pending_for_key(ThinkFamily, Some(actor)) >= AI01_PENDING_TIMERS_PER_ACTOR {
            return Err(CycleError::Timer(OwnerTimerError::PendingLimitReached));
        }
        self.prune(runtime);
        if !self.actors.contains(&actor) && self.actors.len() >= AI01_CREATURES_PER_SCOPE {
            return Err(CycleError::ActorLimit);
        }
        let ordinal = current
            .accept_input(self.generation)
            .map_err(CycleError::Ordinal)?;
        let stamp = current.stamp(ordinal);
        self.schedule_current(runtime, current, stamp, actor, due)
    }
    fn schedule_current(
        &mut self,
        runtime: &ChannelRuntimeV1,
        current: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        actor: ExactActorRef,
        due: SemanticTimeMicros,
    ) -> Result<ThinkOccurrence, CycleError> {
        if !self.current(runtime, current) || !current.accepts_stamp(stamp) {
            return Err(CycleError::StaleOwner);
        }
        if !runtime.contains_live_creature(actor) {
            return Err(CycleError::StaleActor);
        }
        self.prune(runtime);
        let fresh = !self.actors.contains(&actor);
        if fresh && self.actors.len() >= AI01_CREATURES_PER_SCOPE {
            return Err(CycleError::ActorLimit);
        }
        let occurrence = schedule_next_think(
            &mut self.timers,
            current,
            stamp,
            &mut self.tracker,
            actor,
            due,
        )
        .map_err(CycleError::Timer)?;
        if fresh {
            self.actors.push(actor);
        }
        Ok(occurrence)
    }
    /// Consumes due owner inputs once in the existing Foundation deterministic order. The
    /// callback must use actual native AI selection/combat facts; absent facts mean refusal.
    /// It receives the current fence and fresh stamp, never a caller-fabricated grant.
    pub(crate) fn run_due<R>(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        current: &mut ScopeRuntimeFence,
        clock: &impl OwnerClock,
        mut consume: impl FnMut(
            &mut ChannelRuntimeV1,
            &mut ChannelSpellStates,
            &ScopeRuntimeFence,
            RuntimeWorkStamp,
            ThinkOccurrence,
            SemanticTimeMicros,
        ) -> R,
    ) -> Result<Vec<R>, CycleError> {
        if !self.current(runtime, current) {
            return Err(CycleError::StaleOwner);
        }
        self.prune(runtime);
        let fired = self.timers.drain_due(clock, current, |actor| {
            runtime.contains_live_creature(actor)
        });
        let mut results = Vec::new();
        for work in fired {
            if !self.current(runtime, current) {
                return Err(CycleError::StaleOwner);
            }
            if !runtime.contains_live_creature(work.occurrence.actor) {
                continue;
            }
            let ordinal = current
                .accept_input(self.generation)
                .map_err(CycleError::Ordinal)?;
            let stamp = current.stamp(ordinal);
            results.push(consume(
                runtime,
                states,
                current,
                stamp,
                work.occurrence,
                work.due,
            ));
            if runtime.contains_live_creature(work.occurrence.actor) {
                // Accepted first-slice think cadence, not a replacement attack/spell interval.
                self.schedule_current(
                    runtime,
                    current,
                    stamp,
                    work.occurrence.actor,
                    work.due
                        .saturating_add_micros(D115_THINK_INTERVAL_MILLIS * 1000),
                )?;
            }
        }
        self.prune(runtime);
        Ok(results)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::{crystal_timer_fixture, owner_timer::VirtualOwnerClock};
    fn owner() -> (
        ChannelRuntimeV1,
        ChannelSpellStates,
        ExactActorRef,
        ScopeRuntimeFence,
        RuntimeWorkStamp,
    ) {
        let (mut runtime, _, _) =
            crate::gameplay_transport::actor_spell::tests::runtime_with_player(0x51);
        let creature = runtime
            .admit_test_creature(crate::foundation::MovementLocalPosition {
                x: 10,
                y: 10,
                floor: 7,
            })
            .expect("creature");
        let b = runtime.binding();
        let (fence, stamp) = crystal_timer_fixture(
            RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
            b.scope_generation(),
        )
        .expect("test-only Foundation grant");
        (
            runtime,
            ChannelSpellStates::default(),
            creature,
            fence,
            stamp,
        )
    }
    #[test]
    fn cycle_due_is_once_typed_and_rearms_without_overdue_catchup() {
        let (mut runtime, mut states, creature, mut fence, _stamp) = owner();
        let mut lane = MonsterThinkLane::new(&runtime, &fence).expect("lane");
        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(0));
        let first = lane
            .schedule(
                &runtime,
                &mut fence,
                creature,
                SemanticTimeMicros::from_micros(10),
            )
            .expect("schedule");
        assert_eq!(first.sequence, 0);
        clock.advance(5_000_000);
        let got = lane
            .run_due(
                &mut runtime,
                &mut states,
                &mut fence,
                &clock,
                |r, _, f, s, o, d| {
                    assert!(r.contains_live_creature(o.actor));
                    assert!(f.accepts_stamp(s));
                    (o.sequence, d.get())
                },
            )
            .expect("dispatch");
        assert_eq!(got, vec![(0, 5_000_000)]);
        assert!(
            lane.run_due(
                &mut runtime,
                &mut states,
                &mut fence,
                &clock,
                |_, _, _, _, _, _| ()
            )
            .expect("retry")
            .is_empty()
        );
        clock.advance(1_000_000);
        assert_eq!(
            lane.run_due(
                &mut runtime,
                &mut states,
                &mut fence,
                &clock,
                |_, _, _, _, o, _| o.sequence
            )
            .expect("next"),
            vec![1]
        );
    }
    #[test]
    fn cycle_refuses_current_fence_of_another_scope_and_changed_generation() {
        let (mut runtime, mut states, creature, mut fence, _stamp) = owner();
        let mut lane = MonsterThinkLane::new(&runtime, &fence).expect("lane");
        lane.schedule(
            &runtime,
            &mut fence,
            creature,
            SemanticTimeMicros::from_micros(0),
        )
        .expect("schedule");
        let b = runtime.binding();
        let other_channel = crate::foundation::ChannelId::decode(&[
            1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 90,
        ])
        .expect("channel");
        let (mut foreign, _) = crystal_timer_fixture(
            RuntimeScopeRefV1::channel(b.world_id(), other_channel),
            b.scope_generation(),
        )
        .expect("other scope grant");
        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(0));
        assert_eq!(
            lane.run_due(
                &mut runtime,
                &mut states,
                &mut foreign,
                &clock,
                |_, _, _, _, _, _| ()
            ),
            Err(CycleError::StaleOwner)
        );
        fence
            .apply_external_grant(ScopeOwnershipGeneration::new(2).expect("new generation"))
            .expect("advance real test fence");
        assert_eq!(
            lane.run_due(
                &mut runtime,
                &mut states,
                &mut fence,
                &clock,
                |_, _, _, _, _, _| ()
            ),
            Err(CycleError::StaleOwner)
        );
        assert!(runtime.contains_live_creature(creature));
    }
    #[test]
    fn cycle_dead_target_never_reaches_consumer_and_tracker_is_pruned() {
        let (mut runtime, mut states, creature, mut fence, _stamp) = owner();
        let mut lane = MonsterThinkLane::new(&runtime, &fence).expect("lane");
        lane.schedule(
            &runtime,
            &mut fence,
            creature,
            SemanticTimeMicros::from_micros(0),
        )
        .expect("schedule");
        runtime.remove_test_actor(creature).expect("removed target");
        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(0));
        assert!(
            lane.run_due(
                &mut runtime,
                &mut states,
                &mut fence,
                &clock,
                |_, _, _, _, _, _| panic!("stale target consumer")
            )
            .expect("drain")
            .is_empty()
        );
        assert!(lane.actors.is_empty());
        assert!(lane.tracker.is_empty());
    }

    #[test]
    fn cycle_duplicate_and_stale_target_refusals_mint_no_work_ordinal() {
        let (mut runtime, _, creature, mut fence, _stamp) = owner();
        let mut lane = MonsterThinkLane::new(&runtime, &fence).expect("lane");
        lane.schedule(
            &runtime,
            &mut fence,
            creature,
            SemanticTimeMicros::from_micros(0),
        )
        .expect("schedule");
        assert_eq!(
            lane.schedule(
                &runtime,
                &mut fence,
                creature,
                SemanticTimeMicros::from_micros(0)
            ),
            Err(CycleError::Timer(OwnerTimerError::PendingLimitReached))
        );
        let generation = runtime.binding().scope_generation();
        assert_eq!(
            fence
                .accept_input(generation)
                .expect("probe after refusal")
                .get(),
            3
        );
        runtime.remove_test_actor(creature).expect("stale target");
        assert_eq!(
            lane.schedule(
                &runtime,
                &mut fence,
                creature,
                SemanticTimeMicros::from_micros(0)
            ),
            Err(CycleError::StaleActor)
        );
        assert_eq!(
            fence
                .accept_input(generation)
                .expect("probe after stale target")
                .get(),
            4
        );
    }
}
