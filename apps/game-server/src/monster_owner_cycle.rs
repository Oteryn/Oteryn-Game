//! Bounded owner-timer -> typed AI think adapter. This never grants scope ownership.
//! The Foundation-owned current mutable fence must be supplied independently.
use crate::ai_think::{
    CREATURE_THINK_INTERVAL_MILLIS, ThinkFamily, ThinkOccurrence, ThinkSequenceTracker,
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
    pub(crate) tracker: ThinkSequenceTracker,
    pub(crate) actors: Vec<ExactActorRef>,
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
    pub(crate) fn run_due<R, S>(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut S,
        current: &mut ScopeRuntimeFence,
        clock: &impl OwnerClock,
        mut consume: impl FnMut(
            &mut ChannelRuntimeV1,
            &mut S,
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
                        .saturating_add_micros(CREATURE_THINK_INTERVAL_MILLIS * 1000),
                )?;
            }
        }
        self.prune(runtime);
        Ok(results)
    }
}
