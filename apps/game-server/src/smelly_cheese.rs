//! Source-specific two-pulse scheduler on the existing physical-owner timer lane.
//! This does not implement a generic Encounter engine, target lookup, damage or presentation.
use crate::foundation::owner_timer::{
    CatchUpPolicy, FamilyPolicy, FiredTimer, OwnerClock, OwnerTimerError, OwnerTimerLane,
    SemanticTimeMicros, TimerFamily,
};
use crate::foundation::{
    CarrierError, ChannelRuntimeV1, ExactActorRef, RuntimeScopeRefV1, RuntimeWorkStamp,
    ScopeOwnershipGeneration, ScopeRuntimeFence,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SmellyPulse {
    Damage500,
    Presentation700,
}
impl TimerFamily for SmellyPulse {
    // Proposed resource row: one pending pulse of each kind per live caster, two total.
    // Source attack interval2000ms exceeds both deadlines; root must register the row.
    fn registered_maximum(self) -> usize {
        1
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SmellyCastOccurrence {
    pub caster: ExactActorRef,
    pub sequence: u64,
}

#[derive(Debug)]
pub(crate) enum SmellyScheduleError {
    Owner(CarrierError),
    Timer(OwnerTimerError),
    InvalidSequence,
    Superseded,
    Capacity,
}
/// One drained native timer input. Deliberately neither Clone nor Copy; only drain
/// constructs it. A consumer takes ownership, so source A cannot be replayed after B.
#[derive(Debug)]
pub(crate) struct SmellyDuePulse {
    pulse: FiredTimer<SmellyPulse, SmellyCastOccurrence>,
}
impl SmellyDuePulse {
    pub(crate) fn kind(&self) -> SmellyPulse {
        self.pulse.family
    }
    pub(crate) fn caster(&self) -> ExactActorRef {
        self.pulse.occurrence.caster
    }
    pub(crate) fn sequence(&self) -> u64 {
        self.pulse.occurrence.sequence
    }
}
pub(crate) struct SmellyCheeseTimers {
    lane: OwnerTimerLane<SmellyPulse, SmellyCastOccurrence>,
    accepted: Vec<SmellyCastOccurrence>,
}
impl SmellyCheeseTimers {
    pub(crate) fn new(
        scope: RuntimeScopeRefV1,
        generation: ScopeOwnershipGeneration,
    ) -> Result<Self, OwnerTimerError> {
        Ok(Self {
            accepted: Vec::new(),
            lane: OwnerTimerLane::for_generation(
                scope,
                generation,
                [SmellyPulse::Damage500, SmellyPulse::Presentation700].map(|family| {
                    (
                        family,
                        FamilyPolicy {
                            max_pending: 1,
                            catch_up: CatchUpPolicy::DeadlineState,
                        },
                    )
                }),
            )?,
        })
    }
    /// Source-bound ordered cast admission. This is the production scheduling entry point.
    /// Completed cast retries cannot schedule another HP pulse; old sequences fail closed.
    pub(crate) fn schedule_once(
        &mut self,
        runtime: &ChannelRuntimeV1,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        occurrence: SmellyCastOccurrence,
        now: SemanticTimeMicros,
    ) -> Result<bool, SmellyScheduleError> {
        if !fence.is_current_for_scope(self.lane.scope(), self.lane.generation())
            || !fence.accepts_stamp(stamp)
        {
            return Err(SmellyScheduleError::Timer(
                OwnerTimerError::StaleOwnerGeneration,
            ));
        }
        // Accepted AI Think occurrence begins at zero; this source adapter keeps its namespace.
        // Exact live source identity and position are read from physical carrier, not metadata.
        runtime
            .smelly_unambiguous_targets(occurrence.caster)
            .map_err(SmellyScheduleError::Owner)?;
        self.accepted
            .retain(|old| runtime.contains_live_creature(old.caster));
        if let Some(old) = self
            .accepted
            .iter()
            .find(|old| old.caster == occurrence.caster)
        {
            if occurrence.sequence < old.sequence {
                return Err(SmellyScheduleError::Superseded);
            }
            if occurrence.sequence == old.sequence {
                return Ok(false);
            }
        }
        if !self
            .accepted
            .iter()
            .any(|old| old.caster == occurrence.caster)
        {
            use crate::foundation::{AI01_SPAWN_POPULATION_MAX, AI01_SPAWN_SOURCES_PER_SCOPE_MAX};
            if self.accepted.len() >= AI01_SPAWN_SOURCES_PER_SCOPE_MAX * AI01_SPAWN_POPULATION_MAX {
                return Err(SmellyScheduleError::Capacity);
            }
            self.accepted
                .try_reserve(1)
                .map_err(|_| SmellyScheduleError::Capacity)?;
        }
        self.schedule(fence, stamp, occurrence, now)
            .map_err(SmellyScheduleError::Timer)?;
        if let Some(old) = self
            .accepted
            .iter_mut()
            .find(|old| old.caster == occurrence.caster)
        {
            *old = occurrence;
        } else {
            self.accepted.push(occurrence);
        }
        Ok(true)
    }
    /// Caller has already resolved the exact source-specific cast under the Channel owner.
    /// No successful cast is reported if either deadline fails to schedule.
    fn schedule(
        &mut self,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        occurrence: SmellyCastOccurrence,
        now: SemanticTimeMicros,
    ) -> Result<(), OwnerTimerError> {
        self.lane.schedule(
            fence,
            stamp,
            SmellyPulse::Damage500,
            occurrence,
            Some(occurrence.caster),
            now.saturating_add_micros(500_000),
        )?;
        if let Err(error) = self.lane.schedule(
            fence,
            stamp,
            SmellyPulse::Presentation700,
            occurrence,
            Some(occurrence.caster),
            now.saturating_add_micros(700_000),
        ) {
            self.lane.cancel(SmellyPulse::Damage500, occurrence);
            return Err(error);
        }
        Ok(())
    }
    /// Stale owner and retired actor purging are the existing timer owner's checks.
    /// Each drained pulse is consumed exactly once; target/presentation consumers resolve
    /// current native world state in the same owner work item before they mutate anything.
    pub(crate) fn drain(
        &mut self,
        clock: &impl OwnerClock,
        fence: &ScopeRuntimeFence,
        caster_is_current: impl FnMut(ExactActorRef) -> bool,
    ) -> Vec<SmellyDuePulse> {
        self.lane
            .drain_due(clock, fence, caster_is_current)
            .into_iter()
            .map(|pulse| SmellyDuePulse { pulse })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::foundation::owner_timer::{OwnerClock, SemanticTimeMicros, VirtualOwnerClock};
    use crate::foundation::{ChannelId, WorldId};
    use crate::foundation::{
        ExactActorRef, RuntimeScopeRefV1, RuntimeWorkStamp, ScopeOwnershipGeneration,
        ScopeRuntimeFence,
    };
    use crate::smelly_cheese::{SmellyCastOccurrence, SmellyCheeseTimers, SmellyPulse};
    fn uuid(seed: u8) -> [u8; 16] {
        let mut v = [0; 16];
        v[0] = 1;
        v[6] = 0x70;
        v[8] = 0x80;
        v[15] = seed;
        v
    }
    fn setup() -> (
        SmellyCheeseTimers,
        ScopeRuntimeFence,
        RuntimeWorkStamp,
        SmellyCastOccurrence,
    ) {
        let world = WorldId::decode(&uuid(1)).unwrap();
        let channel = ChannelId::decode(&uuid(2)).unwrap();
        let generation = ScopeOwnershipGeneration::new(1).unwrap();
        let scope = RuntimeScopeRefV1::channel(world, channel);
        let (fence, stamp) = crate::foundation::crystal_timer_fixture(scope, generation).unwrap();
        // Existing Foundation test constructor; timer tests do not claim real actor admission.
        let caster = ExactActorRef::transport_fixture(world, channel);
        (
            SmellyCheeseTimers::new(scope, generation).unwrap(),
            fence,
            stamp,
            SmellyCastOccurrence {
                caster,
                sequence: 1,
            },
        )
    }
    #[test]
    fn separate_exact_deadlines_have_no_early_or_duplicate_pulses() {
        let (mut lane, fence, stamp, occ) = setup();
        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(0));
        lane.schedule(&fence, stamp, occ, clock.now()).unwrap();
        clock.advance(499_999);
        assert!(lane.drain(&clock, &fence, |_| true).is_empty());
        clock.advance(1);
        let first = lane.drain(&clock, &fence, |_| true);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].kind(), SmellyPulse::Damage500);
        clock.advance(199_999);
        assert!(lane.drain(&clock, &fence, |_| true).is_empty());
        clock.advance(1);
        let second = lane.drain(&clock, &fence, |_| true);
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].kind(), SmellyPulse::Presentation700);
        assert!(lane.drain(&clock, &fence, |_| true).is_empty());
    }
    #[test]
    fn retired_caster_cancels_both_even_before_deadline() {
        let (mut lane, fence, stamp, occ) = setup();
        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(0));
        lane.schedule(&fence, stamp, occ, clock.now()).unwrap();
        assert!(lane.drain(&clock, &fence, |_| false).is_empty());
        clock.advance(1_000_000);
        assert!(lane.drain(&clock, &fence, |_| true).is_empty());
    }
}
