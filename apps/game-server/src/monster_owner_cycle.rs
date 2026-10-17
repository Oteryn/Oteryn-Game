//! Bounded owner-timer -> typed AI think adapter. This never grants scope ownership.
//! The Foundation-owned current mutable fence must be supplied independently.
use crate::ai_think::CreatureAiState;
use crate::ai_think::behaviour_profile::OwnerProfileError;
use crate::ai_think::profile_schedule::{MAX_SUMMON_ENTRIES, ProfileScheduleInput, SchedulePlan};
use crate::ai_think::{
    CREATURE_THINK_INTERVAL_MILLIS, ThinkFamily, ThinkOccurrence, ThinkSequenceTracker,
    schedule_next_think,
};
use crate::foundation::owner_timer::{
    AI01_PENDING_TIMERS_PER_ACTOR, CatchUpPolicy, FamilyPolicy, OwnerClock, OwnerTimerError,
    OwnerTimerLane, SemanticTimeMicros,
};
use crate::foundation::{
    CarrierError, ChannelRuntimeV1, ExactActorRef, GenerationError, MovementLocalPosition,
    RuntimeScopeRefV1, RuntimeWorkStamp, ScopeOwnershipGeneration, ScopeRuntimeFence,
};
use crate::gameplay_transport::actor_spell::ChannelSpellStates;
use crate::monster_summon::{
    NativeSummonCatalog, NativeSummonOwner, SummonError, SummonLocationPolicy, SummonReceipt,
};
use std::future::{Future, poll_fn};
use std::pin::pin;
use std::task::Poll;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MonsterProfileError {
    Owner(OwnerProfileError),
    Summon(SummonError),
}

/// Explicit detached owner adapter; not installed in the legacy live think loop.
/// Source catalog and profile must come from the active compatible content owner;
/// target selection comes from the think, path and counts from current native owners.
/// The shared `ai_think.rs` report/optional-facts seam must be integrated before
/// activating this path. It neither invents content nor replaces native realization.
pub(crate) fn prepare_monster_profile(
    runtime: &ChannelRuntimeV1,
    current: &ScopeRuntimeFence,
    stamp: RuntimeWorkStamp,
    state: &mut CreatureAiState,
    input: ProfileScheduleInput<'_>,
    catalog: &NativeSummonCatalog,
    policy: &mut dyn SummonLocationPolicy,
) -> Result<SchedulePlan, MonsterProfileError> {
    let binding = runtime.binding();
    if !current.is_current_for_scope(
        RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id()),
        binding.scope_generation(),
    ) || !current.accepts_stamp(stamp)
    {
        return Err(MonsterProfileError::Owner(OwnerProfileError::StaleOwner));
    }
    if !runtime.contains_live_creature(input.occurrence.actor)
        || input
            .target
            .is_some_and(|target| runtime.read_actor_position(target.actor).is_err())
    {
        return Err(MonsterProfileError::Owner(OwnerProfileError::StaleActor));
    }
    let target_with_path = match input.target {
        Some(target) => match policy.current_target_reachable(
            runtime,
            input.occurrence.actor,
            target.actor,
            stamp,
        ) {
            Some(true) => Some(target.actor),
            Some(false) => None,
            None => return Err(MonsterProfileError::Summon(SummonError::MissingMap)),
        },
        None => None,
    };
    let facts = catalog.facts(runtime, input.occurrence, target_with_path);
    state
        .prepare_owner_profile(runtime, current, stamp, input, Some(&facts))
        .map_err(MonsterProfileError::Owner)
}

/// Resolve in authored entry order through the existing physical owner. Each
/// execution independently rechecks live caps, role, target/path, terrain/content,
/// fence/stamp and replay; a proposal or prepared count cannot grant admission.
/// The future caller must register newly-created actors with AI/combat/movement
/// using the existing admission/rollback hooks; this adapter does not fabricate it.
pub(crate) fn resolve_monster_profile_summons(
    runtime: &mut ChannelRuntimeV1,
    current: &ScopeRuntimeFence,
    stamp: RuntimeWorkStamp,
    owner: &mut NativeSummonOwner,
    catalog: &NativeSummonCatalog,
    plan: &SchedulePlan,
    policy: &mut dyn SummonLocationPolicy,
) -> Result<Vec<Result<SummonReceipt, SummonError>>, MonsterProfileError> {
    let binding = runtime.binding();
    if !current.is_current_for_scope(
        RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id()),
        binding.scope_generation(),
    ) || !current.accepts_stamp(stamp)
    {
        return Err(MonsterProfileError::Owner(OwnerProfileError::StaleOwner));
    }
    if plan.summon_proposals.len() > MAX_SUMMON_ENTRIES {
        return Err(MonsterProfileError::Owner(OwnerProfileError::Schedule(
            crate::ai_think::profile_schedule::ScheduleError::TooManyEntries,
        )));
    }
    Ok(plan
        .summon_proposals
        .iter()
        .map(|proposal| owner.execute(runtime, current, stamp, catalog, proposal, policy))
        .collect())
}

/// Session-free body for the World owner's per-Channel respawn job. Foundation
/// remains the only owner of spawn points, occurrence deadlines and window budgets.
/// [`MonsterRespawnJob::drive`] is its wake loop; installing that loop in the World
/// composition root (`node/serve`, D1239) is a separate integration step.
#[derive(Debug)]
pub(crate) struct MonsterRespawnJob {
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
    last_observed_micros: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RespawnJobError {
    StaleOwner,
    ClockRegressed,
    Carrier(CarrierError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RespawnJobReport {
    pub(crate) warnings: Vec<MovementLocalPosition>,
    pub(crate) admitted: Vec<ExactActorRef>,
    pub(crate) blocked: usize,
    pub(crate) retried: usize,
    pub(crate) skipped: usize,
    /// Next native owner-window boundary, not another spawn occurrence/deadline.
    /// None means the semantic clock cannot represent another complete boundary.
    pub(crate) next_wake: Option<SemanticTimeMicros>,
}

impl MonsterRespawnJob {
    pub(crate) fn new(
        runtime: &ChannelRuntimeV1,
        current: &ScopeRuntimeFence,
    ) -> Result<Self, RespawnJobError> {
        let binding = runtime.binding();
        let job = Self {
            scope: RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id()),
            generation: binding.scope_generation(),
            last_observed_micros: None,
        };
        job.check_current(runtime, Some(current))?;
        Ok(job)
    }

    /// The production install: the Channel runtime's own current-owner fence
    /// (`ChannelRuntimeV1::owner_fence`) is the authority, consulted under the runtime
    /// lock on every pass. The fence cannot be lent out across that lock, so no
    /// separate fence is supplied; a retired or replaced owner ends the job.
    pub(crate) fn new_owned(runtime: &ChannelRuntimeV1) -> Result<Self, RespawnJobError> {
        let binding = runtime.binding();
        let job = Self {
            scope: RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id()),
            generation: binding.scope_generation(),
            last_observed_micros: None,
        };
        job.check_current(runtime, None)?;
        Ok(job)
    }

    fn check_current(
        &self,
        runtime: &ChannelRuntimeV1,
        current: Option<&ScopeRuntimeFence>,
    ) -> Result<(), RespawnJobError> {
        let binding = runtime.binding();
        if self.scope != RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id())
            || self.generation != binding.scope_generation()
            || current.is_some_and(|fence| !fence.is_current_for_scope(self.scope, self.generation))
            || runtime.owner_fence().is_err()
        {
            return Err(RespawnJobError::StaleOwner);
        }
        Ok(())
    }

    /// Runs without a player, session, spell state or gameplay transport admission.
    /// Repeated calls retain Foundation's one occurrence per point and shared window
    /// budget. Late calls use the observed owner time once; no catch-up loop is added.
    pub(crate) fn run(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        current: &ScopeRuntimeFence,
        clock: &impl OwnerClock,
    ) -> Result<RespawnJobReport, RespawnJobError> {
        self.run_with(runtime, Some(current), clock)
    }

    fn run_with(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        current: Option<&ScopeRuntimeFence>,
        clock: &impl OwnerClock,
    ) -> Result<RespawnJobReport, RespawnJobError> {
        self.check_current(runtime, current)?;
        let now = clock.now().get();
        if self.last_observed_micros.is_some_and(|last| now < last) {
            return Err(RespawnJobError::ClockRegressed);
        }
        // Record even a refused native pass: it may have progressed other points
        // before returning its error, and a retry cannot move their clock backwards.
        self.last_observed_micros = Some(now);
        let report = runtime
            .drive_spawn_respawns(now, |observer, cell| {
                use crate::movement::interest::{VisibilityPosition, VisibilitySettings};
                match (
                    VisibilityPosition::new(observer.x, observer.y, observer.floor),
                    VisibilityPosition::new(cell.x, cell.y, cell.floor),
                ) {
                    (Ok(observer), Ok(cell)) => {
                        VisibilitySettings::REFERENCE.can_see(observer, cell)
                    }
                    _ => false,
                }
            })
            .map_err(RespawnJobError::Carrier)?;
        // The wake loop below owns the actual wakeup; this pass retains no timer.
        Ok(RespawnJobReport {
            warnings: report.warnings,
            admitted: report.admitted,
            blocked: report.blocked,
            retried: report.retried,
            skipped: report.skipped,
            next_wake: next_owner_window(now),
        })
    }

    /// Session-free World wake loop for one Channel: one native pass per owner window,
    /// each under one short runtime lock that is never held across a wait. Wakes are
    /// deterministic owner-window boundaries of `wake`'s clock; a late wake runs once
    /// at the observed time and never bursts. A refused native pass leaves its
    /// occurrences pending and is retried at the next window. A stale scope, a replaced
    /// generation or a retired runtime ends the loop; it never revives the owner.
    pub(crate) async fn drive(
        self,
        runtime: &tokio::sync::Mutex<ChannelRuntimeV1>,
        current: &ScopeRuntimeFence,
        wake: &impl RespawnWake,
        shutdown: &oteryn_foundation::CancellationToken,
    ) -> RespawnDriveEnd {
        self.drive_with(runtime, Some(current), wake, shutdown)
            .await
    }

    /// [`Self::drive`] under the runtime's own current-owner fence ([`Self::new_owned`]).
    pub(crate) async fn drive_owned(
        self,
        runtime: &tokio::sync::Mutex<ChannelRuntimeV1>,
        wake: &impl RespawnWake,
        shutdown: &oteryn_foundation::CancellationToken,
    ) -> RespawnDriveEnd {
        self.drive_with(runtime, None, wake, shutdown).await
    }

    async fn drive_with(
        mut self,
        runtime: &tokio::sync::Mutex<ChannelRuntimeV1>,
        current: Option<&ScopeRuntimeFence>,
        wake: &impl RespawnWake,
        shutdown: &oteryn_foundation::CancellationToken,
    ) -> RespawnDriveEnd {
        loop {
            if shutdown.is_cancelled() {
                return RespawnDriveEnd::Shutdown;
            }
            let pass = {
                let mut runtime = runtime.lock().await;
                self.run_with(&mut runtime, current, wake)
            };
            let next = match pass {
                Ok(report) => report.next_wake,
                Err(RespawnJobError::StaleOwner) => return RespawnDriveEnd::StaleOwner,
                // No catch-up: resume at the window after the latest observed owner time.
                Err(RespawnJobError::ClockRegressed | RespawnJobError::Carrier(_)) => {
                    self.last_observed_micros.and_then(next_owner_window)
                }
            };
            let Some(next) = next else {
                return RespawnDriveEnd::ClockExhausted;
            };
            let mut stopping = pin!(shutdown.cancelled());
            let mut sleeping = pin!(wake.sleep_until(next));
            let stopped = poll_fn(|context| {
                if stopping.as_mut().poll(context).is_ready() {
                    return Poll::Ready(true);
                }
                sleeping.as_mut().poll(context).map(|()| false)
            })
            .await;
            if stopped {
                return RespawnDriveEnd::Shutdown;
            }
        }
    }
}

/// The World install point for one Channel: starts the session-free respawn wake loop
/// under the runtime's own current-owner fence, on `wake`'s clock. `serve_gameplay`
/// passes the wake built from the same `clock_origin` the session-driven pass reads, so
/// both paths advance one timeline. Ends on shutdown, a retired owner or an exhausted clock.
pub(crate) async fn run_world_respawns(
    runtime: &tokio::sync::Mutex<ChannelRuntimeV1>,
    wake: &impl RespawnWake,
    shutdown: &oteryn_foundation::CancellationToken,
) -> RespawnDriveEnd {
    let job = {
        let runtime = runtime.lock().await;
        MonsterRespawnJob::new_owned(&runtime)
    };
    match job {
        Ok(job) => job.drive_owned(runtime, wake, shutdown).await,
        Err(_) => RespawnDriveEnd::StaleOwner,
    }
}

/// CREATURE-AI-0 §7 / native `spawn::OWNER_WINDOW_MICROS` (50 ms): the next complete
/// owner-window boundary after `now`, or `None` when semantic time cannot represent it.
fn next_owner_window(now: u64) -> Option<SemanticTimeMicros> {
    let window = crate::ai_think::CREATUREAI0_THINK_WINDOW_MICROS;
    (now / window)
        .checked_add(1)
        .and_then(|next| next.checked_mul(window))
        .map(SemanticTimeMicros::from_micros)
}

/// Why [`MonsterRespawnJob::drive`] returned. None of these is retried by the loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RespawnDriveEnd {
    /// Shutdown was requested; no further native pass starts.
    Shutdown,
    /// The scope, its generation or the runtime's owner cycle is no longer current.
    StaleOwner,
    /// The owner clock cannot represent another owner-window boundary.
    ClockExhausted,
}

/// The Channel owner's clock together with its wakeup. The World composition root must
/// supply the same clock origin the gameplay seam reads for this Channel: a second
/// origin would put this loop's observations on another time line than the
/// session-driven pass over the same runtime.
pub(crate) trait RespawnWake: OwnerClock {
    /// Completes once this clock has reached `at` or later.
    fn sleep_until(&self, at: SemanticTimeMicros) -> impl Future<Output = ()>;
}

/// The production wake: a process-local monotonic origin, never a client or wall clock.
#[derive(Debug, Clone, Copy)]
pub(crate) struct MonotonicRespawnWake {
    origin: std::time::Instant,
}

impl MonotonicRespawnWake {
    pub(crate) const fn from_origin(origin: std::time::Instant) -> Self {
        Self { origin }
    }
}

impl OwnerClock for MonotonicRespawnWake {
    fn now(&self) -> SemanticTimeMicros {
        SemanticTimeMicros::from_micros(
            u64::try_from(self.origin.elapsed().as_micros()).unwrap_or(u64::MAX),
        )
    }
}

impl RespawnWake for MonotonicRespawnWake {
    fn sleep_until(&self, at: SemanticTimeMicros) -> impl Future<Output = ()> {
        let deadline = self
            .origin
            .checked_add(std::time::Duration::from_micros(at.get()));
        async move {
            match deadline {
                Some(deadline) => {
                    tokio::time::sleep_until(tokio::time::Instant::from_std(deadline)).await;
                }
                None => std::future::pending().await,
            }
        }
    }
}

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
                        .saturating_add_micros(CREATURE_THINK_INTERVAL_MILLIS * 1000),
                )?;
            }
        }
        self.prune(runtime);
        Ok(results)
    }
}
#[cfg(test)]
#[allow(clippy::expect_used)]
mod loop_monster_summons {
    use super::*;
    use crate::ability::RevisionSet;
    use crate::ai_think::behaviour_profile::{CreatureBehaviourProfile, ProfileRefusal};
    use crate::ai_think::profile_schedule::{AttackTarget, ScheduleError};
    use crate::content::ProjectV2BehaviorAuthoring;
    use crate::foundation::{MovementLocalPosition, WorldId, crystal_timer_fixture};
    use oteryn_simulation_determinism::GameplayDecisionRoot;
    use std::collections::BTreeMap;

    struct Map {
        reachable: Option<bool>,
        admits: Option<bool>,
        reads: usize,
    }
    impl Map {
        fn known() -> Self {
            Self {
                reachable: Some(true),
                admits: Some(true),
                reads: 0,
            }
        }
    }
    impl SummonLocationPolicy for Map {
        fn current_target_reachable(
            &mut self,
            _: &ChannelRuntimeV1,
            _: ExactActorRef,
            _: ExactActorRef,
            _: RuntimeWorkStamp,
        ) -> Option<bool> {
            self.reads += 1;
            self.reachable
        }
        fn current_cell_admits(
            &mut self,
            _: &ChannelRuntimeV1,
            _: ExactActorRef,
            _: &crate::content::ProjectV2DefinitionRef,
            _: MovementLocalPosition,
            _: RuntimeWorkStamp,
        ) -> Option<bool> {
            self.reads += 1;
            self.admits
        }
    }
    fn setup(
        interval: u64,
    ) -> (
        ChannelRuntimeV1,
        ScopeRuntimeFence,
        ExactActorRef,
        ExactActorRef,
        NativeSummonCatalog,
        CreatureBehaviourProfile,
    ) {
        let mut id = [0; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        id[15] = 1;
        let (runtime, fence, parent, target, catalog, proposal) =
            crate::monster_summon::summon_execution_fixture_for_world(
                WorldId::decode(&id).expect("fixture world"),
            );
        // The native fixture supplies physical source specs, not production content authority.
        let behavior: ProjectV2BehaviorAuthoring = serde_json::from_value(serde_json::json!({
            "movement":{"can_walk":true,"pass_through":false,"pushable":false,"push_items":false,"push_creatures":false,"walks_on_energy":false,"walks_on_fire":false,"walks_on_poison":false},
            "targeting":{"hostile":true,"can_target":true,"sense_invisible":false,"target_distance_tiles":1,"static_attack_chance_ppm":1000000,"flee_health":0},
            "summons":{"max_summons":2,"entries":[{"creature":proposal.creature,"interval_ms":interval,"chance_ppm":1000000,"count":2}]}
        })).expect("authored native behavior fixture");
        let profile =
            CreatureBehaviourProfile::project_with_summons(Some(&behavior), &BTreeMap::new())
                .expect("explicit projection");
        (runtime, fence, parent, target, catalog, profile)
    }
    fn state(
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        profile: &CreatureBehaviourProfile,
    ) -> CreatureAiState {
        CreatureAiState::new(
            actor,
            profile.clone(),
            runtime
                .read_actor_position(actor)
                .expect("current position")
                .position(),
        )
    }
    fn stamp(runtime: &ChannelRuntimeV1, current: &mut ScopeRuntimeFence) -> RuntimeWorkStamp {
        let ordinal = current
            .accept_input(runtime.binding().scope_generation())
            .expect("current owner input");
        current.stamp(ordinal)
    }
    fn revisions() -> RevisionSet {
        RevisionSet::new(
            "rules-r1",
            "content-r1",
            "policy-r1",
            "definition-r1",
            "sim-r1",
        )
        .expect("revisions")
    }
    fn input<'a>(
        profile: &'a CreatureBehaviourProfile,
        actor: ExactActorRef,
        target: ExactActorRef,
        sequence: u64,
        revisions: &'a RevisionSet,
        root: &'a GameplayDecisionRoot,
    ) -> ProfileScheduleInput<'a> {
        ProfileScheduleInput {
            occurrence: ThinkOccurrence { actor, sequence },
            behavior: profile.behavior(),
            abilities: profile.abilities(),
            target: Some(AttackTarget {
                actor: target,
                same_floor_distance: Some(5),
            }),
            revisions,
            root,
        }
    }

    #[test]
    fn explicit_projection_keeps_summons_without_activating_legacy_think() {
        let (_, _, _, _, _, profile) = setup(1000);
        let original = profile.behavior();
        let legacy = CreatureBehaviourProfile::project(Some(original), profile.abilities())
            .expect("legacy projection");
        assert!(legacy.behavior().summons.is_none());
        assert!(profile.behavior().summons.is_some());
        assert_eq!(legacy.targeting(), profile.targeting());
        for (entries, maximum, accepted) in
            [(8, 16, true), (9, 16, false), (1, 17, false), (0, 1, false)]
        {
            let mut behavior = original.clone();
            let summons = behavior.summons.as_mut().expect("summons");
            summons.entries.resize(entries, summons.entries[0].clone());
            summons.max_summons = maximum;
            assert_eq!(
                CreatureBehaviourProfile::project_with_summons(
                    Some(&behavior),
                    profile.abilities()
                )
                .is_ok(),
                accepted
            );
        }
        assert_eq!(
            CreatureBehaviourProfile::project_with_summons(None, profile.abilities()),
            Err(ProfileRefusal::Missing)
        );
    }

    #[test]
    fn native_schedule_due_retry_and_changed_path_keep_one_clock() {
        let (runtime, mut fence, parent, target, catalog, profile) = setup(2000);
        let mut state = state(&runtime, parent, &profile);
        let stamp = stamp(&runtime, &mut fence);
        let revisions = revisions();
        let root = GameplayDecisionRoot::from_bytes([9; 32]);
        let mut map = Map::known();
        let first = prepare_monster_profile(
            &runtime,
            &fence,
            stamp,
            &mut state,
            input(&profile, parent, target, 0, &revisions, &root),
            &catalog,
            &mut map,
        )
        .expect("first think");
        assert!(first.summon_proposals.is_empty());
        assert_eq!(first.defence_ticks_after_ms, 1000);
        let second_input = input(&profile, parent, target, 1, &revisions, &root);
        let second = prepare_monster_profile(
            &runtime,
            &fence,
            stamp,
            &mut state,
            second_input,
            &catalog,
            &mut map,
        )
        .expect("due think");
        assert_eq!(second.summon_proposals.len(), 1);
        assert_eq!(
            prepare_monster_profile(
                &runtime,
                &fence,
                stamp,
                &mut state,
                second_input,
                &catalog,
                &mut map
            ),
            Ok(second)
        );
        map.reachable = Some(false);
        assert_eq!(
            prepare_monster_profile(
                &runtime,
                &fence,
                stamp,
                &mut state,
                second_input,
                &catalog,
                &mut map
            ),
            Err(MonsterProfileError::Owner(OwnerProfileError::Schedule(
                ScheduleError::OccurrenceConflict
            )))
        );
    }

    #[test]
    fn existing_native_owner_realizes_once_and_rechecks_live_cap() {
        let (mut runtime, mut fence, parent, target, catalog, profile) = setup(1000);
        let mut state = state(&runtime, parent, &profile);
        let stamp = stamp(&runtime, &mut fence);
        let revisions = revisions();
        let root = GameplayDecisionRoot::from_bytes([9; 32]);
        let mut map = Map::known();
        let mut owner = NativeSummonOwner::default();
        for sequence in 0..2 {
            let plan = prepare_monster_profile(
                &runtime,
                &fence,
                stamp,
                &mut state,
                input(&profile, parent, target, sequence, &revisions, &root),
                &catalog,
                &mut map,
            )
            .expect("prepared");
            let result = resolve_monster_profile_summons(
                &mut runtime,
                &fence,
                stamp,
                &mut owner,
                &catalog,
                &plan,
                &mut map,
            )
            .expect("dispatch");
            assert!(result[0].as_ref().expect("admitted").newly_created);
            let replay = resolve_monster_profile_summons(
                &mut runtime,
                &fence,
                stamp,
                &mut owner,
                &catalog,
                &plan,
                &mut map,
            )
            .expect("retry");
            assert!(!replay[0].as_ref().expect("same child").newly_created);
            assert_eq!(
                runtime.native_summon_count(parent, None),
                sequence as u32 + 1
            );
        }
        let full = prepare_monster_profile(
            &runtime,
            &fence,
            stamp,
            &mut state,
            input(&profile, parent, target, 2, &revisions, &root),
            &catalog,
            &mut map,
        )
        .expect("cap checked");
        assert!(full.summon_proposals.is_empty());
        assert_eq!(runtime.native_summon_count(parent, None), 2);
    }

    #[test]
    fn unknown_path_refuses_before_advancing_schedule_and_known_blocked_path_emits_none() {
        let (runtime, mut fence, parent, target, catalog, profile) = setup(2000);
        let mut state = state(&runtime, parent, &profile);
        let stamp = stamp(&runtime, &mut fence);
        let revisions = revisions();
        let root = GameplayDecisionRoot::from_bytes([9; 32]);
        let mut map = Map::known();
        map.reachable = None;
        let first_input = input(&profile, parent, target, 0, &revisions, &root);
        assert_eq!(
            prepare_monster_profile(
                &runtime,
                &fence,
                stamp,
                &mut state,
                first_input,
                &catalog,
                &mut map
            ),
            Err(MonsterProfileError::Summon(SummonError::MissingMap))
        );
        map.reachable = Some(true);
        let first = prepare_monster_profile(
            &runtime,
            &fence,
            stamp,
            &mut state,
            first_input,
            &catalog,
            &mut map,
        )
        .expect("same first occurrence");
        assert_eq!(first.evaluated_defence_ticks_ms, 1000);
        map.reachable = Some(false);
        let blocked = prepare_monster_profile(
            &runtime,
            &fence,
            stamp,
            &mut state,
            input(&profile, parent, target, 1, &revisions, &root),
            &catalog,
            &mut map,
        )
        .expect("no path");
        assert!(blocked.summon_proposals.is_empty());
    }

    #[test]
    fn foreign_current_scope_and_stale_actor_never_reach_policy() {
        let (mut runtime, mut fence, parent, target, catalog, profile) = setup(1000);
        let mut state = state(&runtime, parent, &profile);
        let stamp = stamp(&runtime, &mut fence);
        let revisions = revisions();
        let root = GameplayDecisionRoot::from_bytes([9; 32]);
        let mut map = Map::known();
        let mut id = *runtime.binding().channel_id().as_bytes();
        id[15] ^= 0x40;
        let (foreign, _) = crystal_timer_fixture(
            RuntimeScopeRefV1::channel(
                runtime.binding().world_id(),
                crate::foundation::ChannelId::decode(&id).expect("foreign channel"),
            ),
            runtime.binding().scope_generation(),
        )
        .expect("independent foreign fence");
        assert_eq!(
            prepare_monster_profile(
                &runtime,
                &foreign,
                stamp,
                &mut state,
                input(&profile, parent, target, 0, &revisions, &root),
                &catalog,
                &mut map
            ),
            Err(MonsterProfileError::Owner(OwnerProfileError::StaleOwner))
        );
        assert_eq!(map.reads, 0);
        runtime.remove_test_actor(target).expect("stale target");
        assert_eq!(
            prepare_monster_profile(
                &runtime,
                &fence,
                stamp,
                &mut state,
                input(&profile, parent, target, 0, &revisions, &root),
                &catalog,
                &mut map
            ),
            Err(MonsterProfileError::Owner(OwnerProfileError::StaleActor))
        );
        assert_eq!(map.reads, 0);
    }

    #[test]
    fn no_free_tile_refuses_without_creating_or_linking_an_actor() {
        let (mut runtime, mut fence, parent, target, catalog, profile) = setup(1000);
        let mut state = state(&runtime, parent, &profile);
        let stamp = stamp(&runtime, &mut fence);
        let revisions = revisions();
        let root = GameplayDecisionRoot::from_bytes([9; 32]);
        let mut map = Map::known();
        let plan = prepare_monster_profile(
            &runtime,
            &fence,
            stamp,
            &mut state,
            input(&profile, parent, target, 0, &revisions, &root),
            &catalog,
            &mut map,
        )
        .expect("proposal");
        map.admits = Some(false);
        assert_eq!(
            resolve_monster_profile_summons(
                &mut runtime,
                &fence,
                stamp,
                &mut NativeSummonOwner::default(),
                &catalog,
                &plan,
                &mut map
            ),
            Ok(vec![Err(SummonError::NoFreeTile)])
        );
        assert_eq!(runtime.native_summon_count(parent, None), 0);
    }

    #[test]
    fn changed_generation_refuses_preparation_and_resolution_without_policy_reads() {
        let (mut runtime, mut fence, parent, target, catalog, profile) = setup(1000);
        let mut state = state(&runtime, parent, &profile);
        let stamp = stamp(&runtime, &mut fence);
        let revisions = revisions();
        let root = GameplayDecisionRoot::from_bytes([9; 32]);
        let mut map = Map::known();
        let plan = prepare_monster_profile(
            &runtime,
            &fence,
            stamp,
            &mut state,
            input(&profile, parent, target, 0, &revisions, &root),
            &catalog,
            &mut map,
        )
        .expect("prepared at original generation");
        fence
            .apply_external_grant(ScopeOwnershipGeneration::new(2).expect("replacement generation"))
            .expect("replace actual current fence");
        map.reads = 0;
        assert_eq!(
            prepare_monster_profile(
                &runtime,
                &fence,
                stamp,
                &mut state,
                input(&profile, parent, target, 1, &revisions, &root),
                &catalog,
                &mut map
            ),
            Err(MonsterProfileError::Owner(OwnerProfileError::StaleOwner))
        );
        assert_eq!(
            resolve_monster_profile_summons(
                &mut runtime,
                &fence,
                stamp,
                &mut NativeSummonOwner::default(),
                &catalog,
                &plan,
                &mut map
            ),
            Err(MonsterProfileError::Owner(OwnerProfileError::StaleOwner))
        );
        assert_eq!(map.reads, 0);
        assert_eq!(runtime.native_summon_count(parent, None), 0);
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
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
        let mut stale_consumer_called = false;
        assert!(
            lane.run_due(
                &mut runtime,
                &mut states,
                &mut fence,
                &clock,
                |_, _, _, _, _, _| {
                    stale_consumer_called = true;
                }
            )
            .expect("drain")
            .is_empty()
        );
        assert!(
            !stale_consumer_called,
            "stale actor must never reach callback"
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

#[cfg(test)]
#[allow(clippy::expect_used)]
mod loop_monster_respawns {
    use super::*;
    use crate::foundation::owner_timer::VirtualOwnerClock;
    use crate::foundation::{ChannelContentPin, ChannelId, NodeId, WorldId, crystal_timer_fixture};

    fn setup() -> (ChannelRuntimeV1, ScopeRuntimeFence, MonsterRespawnJob) {
        let world = WorldId::decode(&[1, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1])
            .expect("world");
        let channel = ChannelId::decode(&[1, 0, 0, 0, 0, 2, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 2])
            .expect("channel");
        let node =
            NodeId::decode(&[1, 0, 0, 0, 0, 3, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 3]).expect("node");
        let room = crate::content::qualify_native_entry_room(world).expect("entry room");
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            channel,
            node,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            ChannelContentPin::test(world),
        )
        .expect("runtime");
        let source = room.spawn_source().expect("fixture spawn source");
        assert_eq!(
            runtime.realize_activation_spawns(vec![source.activation_facts()]),
            Ok(2)
        );
        let (current, _) = crystal_timer_fixture(
            RuntimeScopeRefV1::channel(world, channel),
            ScopeOwnershipGeneration::new(1).expect("generation"),
        )
        .expect("independently supplied current test fence");
        let job = MonsterRespawnJob::new(&runtime, &current).expect("job");
        (runtime, current, job)
    }

    fn at(micros: u64) -> VirtualOwnerClock {
        VirtualOwnerClock::new(SemanticTimeMicros::from_micros(micros))
    }

    #[test]
    fn world_job_respawns_without_a_session_on_the_native_schedule() {
        let (mut runtime, current, mut job) = setup();
        assert!(
            runtime
                .positioned_melee_census()
                .expect("census")
                .iter()
                .all(|(_, _, session)| session.is_none())
        );
        let (point, cell, live) = runtime.spawn_point_creatures()[0];
        let original = live.expect("live rat");
        assert!(runtime.kill_creature_for_test(original));
        let first = job
            .run(&mut runtime, &current, &at(0))
            .expect("observe gone");
        assert!(first.warnings.is_empty());
        assert!(first.admitted.is_empty());
        assert_eq!(
            first.next_wake,
            Some(SemanticTimeMicros::from_micros(50_000))
        );
        let due = runtime
            .pending_respawn(point)
            .expect("one native occurrence")
            .due_micros;
        assert!(
            job.run(&mut runtime, &current, &at(due - 1))
                .expect("not due")
                .warnings
                .is_empty()
        );
        let warned = job.run(&mut runtime, &current, &at(due)).expect("warning");
        assert_eq!(warned.warnings, vec![cell]);
        let admit_at = runtime
            .pending_respawn(point)
            .expect("native admission deadline")
            .due_micros;
        assert_eq!(admit_at, due + 4_200_000);
        assert!(
            job.run(&mut runtime, &current, &at(admit_at - 1))
                .expect("not due")
                .admitted
                .is_empty()
        );
        let admitted = job
            .run(&mut runtime, &current, &at(admit_at))
            .expect("admit");
        assert_eq!(admitted.admitted.len(), 1);
        assert_ne!(admitted.admitted[0], original);
        assert_eq!(runtime.pending_respawn(point), None);
        let replay = job
            .run(&mut runtime, &current, &at(admit_at))
            .expect("repeat wake");
        assert!(replay.admitted.is_empty());
        assert!(replay.warnings.is_empty());
        assert_eq!(
            runtime.spawn_point_creatures()[0].2,
            Some(admitted.admitted[0])
        );
    }

    #[test]
    fn repeated_runs_replay_identically_with_late_owner_wakes() {
        let run = || {
            let (mut runtime, current, mut job) = setup();
            let (point, _, live) = runtime.spawn_point_creatures()[0];
            assert!(runtime.kill_creature_for_test(live.expect("live rat")));
            let mut reports = vec![job.run(&mut runtime, &current, &at(123)).expect("gone")];
            let due = runtime.pending_respawn(point).expect("due").due_micros;
            // A late wake performs one native warning, never bursts through admission.
            reports.push(
                job.run(&mut runtime, &current, &at(due + 12_345))
                    .expect("late warning"),
            );
            reports.push(
                job.run(&mut runtime, &current, &at(due + 12_345))
                    .expect("same wake"),
            );
            let admit_at = runtime
                .pending_respawn(point)
                .expect("admit deadline")
                .due_micros;
            reports.push(
                job.run(&mut runtime, &current, &at(admit_at + 678))
                    .expect("late admission"),
            );
            (reports, runtime.spawn_point_creatures())
        };
        let first = run();
        assert_eq!(first.0.iter().map(|r| r.warnings.len()).sum::<usize>(), 1);
        assert_eq!(first.0.iter().map(|r| r.admitted.len()).sum::<usize>(), 1);
        assert_eq!(first, run());
    }

    #[test]
    fn foreign_scope_or_replaced_generation_refuses_before_native_mutation() {
        let (mut runtime, mut current, mut job) = setup();
        let (point, _, live) = runtime.spawn_point_creatures()[0];
        assert!(runtime.kill_creature_for_test(live.expect("live rat")));
        job.run(&mut runtime, &current, &at(0)).expect("gone");
        let pending = runtime.pending_respawn(point).expect("pending");
        let before = runtime.spawn_point_creatures();
        let binding = runtime.binding();
        let foreign_world =
            WorldId::decode(&[1, 0, 0, 0, 0, 9, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1])
                .expect("foreign world");
        let foreign_channel =
            ChannelId::decode(&[1, 0, 0, 0, 0, 9, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 2])
                .expect("foreign channel");
        for scope in [
            RuntimeScopeRefV1::channel(foreign_world, binding.channel_id()),
            RuntimeScopeRefV1::channel(binding.world_id(), foreign_channel),
        ] {
            let (foreign, _) = crystal_timer_fixture(scope, binding.scope_generation())
                .expect("foreign live grant");
            assert_eq!(
                job.run(&mut runtime, &foreign, &at(pending.due_micros)),
                Err(RespawnJobError::StaleOwner)
            );
            assert_eq!(runtime.pending_respawn(point), Some(pending));
            assert_eq!(runtime.spawn_point_creatures(), before);
            assert_eq!(job.last_observed_micros, Some(0));
        }
        current
            .apply_external_grant(ScopeOwnershipGeneration::new(2).expect("successor"))
            .expect("replace grant");
        assert_eq!(
            job.run(&mut runtime, &current, &at(pending.due_micros)),
            Err(RespawnJobError::StaleOwner)
        );
        assert_eq!(runtime.pending_respawn(point), Some(pending));
        assert_eq!(runtime.spawn_point_creatures(), before);
        assert_eq!(job.last_observed_micros, Some(0));
    }

    #[test]
    fn retired_runtime_cannot_be_revived_by_a_separate_matching_fence() {
        let (mut runtime, current, mut job) = setup();
        runtime.retire_owner_cycle();
        assert_eq!(
            job.run(&mut runtime, &current, &at(0)),
            Err(RespawnJobError::StaleOwner)
        );
        assert!(matches!(
            MonsterRespawnJob::new(&runtime, &current),
            Err(RespawnJobError::StaleOwner)
        ));
        assert_eq!(job.last_observed_micros, None);
    }

    #[test]
    fn clock_regression_preserves_pending_occurrence_and_valid_retry() {
        let (mut runtime, current, mut job) = setup();
        let (point, _, live) = runtime.spawn_point_creatures()[0];
        assert!(runtime.kill_creature_for_test(live.expect("live rat")));
        job.run(&mut runtime, &current, &at(1_000)).expect("gone");
        let pending = runtime.pending_respawn(point).expect("pending");
        let before = runtime.spawn_point_creatures();
        assert_eq!(
            job.run(&mut runtime, &current, &at(999)),
            Err(RespawnJobError::ClockRegressed)
        );
        assert_eq!(runtime.pending_respawn(point), Some(pending));
        assert_eq!(runtime.spawn_point_creatures(), before);
        assert_eq!(job.last_observed_micros, Some(1_000));
        assert_eq!(
            job.run(&mut runtime, &current, &at(pending.due_micros))
                .expect("valid retry")
                .warnings
                .len(),
            1
        );
    }

    /// A virtual owner clock whose wait completes at once at the requested boundary.
    /// It requests shutdown on the first wait after `stop_at` has been observed and can
    /// retire the runtime on its first wait, through the same lock the loop takes.
    struct VirtualWake<'a> {
        now: std::cell::Cell<u64>,
        sleeps: std::cell::RefCell<Vec<u64>>,
        stop_at: u64,
        shutdown: &'a oteryn_foundation::CancellationToken,
        retire: Option<&'a tokio::sync::Mutex<ChannelRuntimeV1>>,
    }

    impl<'a> VirtualWake<'a> {
        fn new(
            start: u64,
            stop_at: u64,
            shutdown: &'a oteryn_foundation::CancellationToken,
        ) -> Self {
            Self {
                now: std::cell::Cell::new(start),
                sleeps: std::cell::RefCell::default(),
                stop_at,
                shutdown,
                retire: None,
            }
        }
    }

    impl OwnerClock for VirtualWake<'_> {
        fn now(&self) -> SemanticTimeMicros {
            SemanticTimeMicros::from_micros(self.now.get())
        }
    }

    impl RespawnWake for VirtualWake<'_> {
        fn sleep_until(&self, at: SemanticTimeMicros) -> impl Future<Output = ()> {
            if let Some(runtime) = self.retire.filter(|_| self.sleeps.borrow().is_empty()) {
                // The loop never holds the runtime across a wait.
                runtime
                    .try_lock()
                    .expect("runtime is free while the loop waits")
                    .retire_owner_cycle();
            }
            if self.now.get() >= self.stop_at {
                self.shutdown.cancel();
            }
            self.sleeps.borrow_mut().push(at.get());
            self.now.set(self.now.get().max(at.get()));
            std::future::ready(())
        }
    }

    fn block_on<F: Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .expect("test runtime")
            .block_on(future)
    }

    /// The native admission time of a rat killed at time 0, read from a separate setup.
    fn admission_time() -> u64 {
        let (mut runtime, current, mut job) = setup();
        let (point, _, live) = runtime.spawn_point_creatures()[0];
        assert!(runtime.kill_creature_for_test(live.expect("live rat")));
        job.run(&mut runtime, &current, &at(0)).expect("gone");
        runtime.pending_respawn(point).expect("due").due_micros + 4_200_000
    }

    #[test]
    fn world_drive_respawns_without_a_session_on_owner_window_wakes() {
        let admit_at = admission_time();
        let drive = || {
            let (mut runtime, current, job) = setup();
            let (point, _, live) = runtime.spawn_point_creatures()[0];
            let original = live.expect("live rat");
            assert!(runtime.kill_creature_for_test(original));
            let runtime = tokio::sync::Mutex::new(runtime);
            let shutdown = oteryn_foundation::CancellationToken::new();
            let wake = VirtualWake::new(0, admit_at, &shutdown);
            let end = block_on(job.drive(&runtime, &current, &wake, &shutdown));
            assert_eq!(end, RespawnDriveEnd::Shutdown);
            let runtime = runtime.into_inner();
            assert_eq!(runtime.pending_respawn(point), None);
            let replacement = runtime.spawn_point_creatures()[0].2.expect("respawned");
            assert_ne!(replacement, original);
            (wake.sleeps.into_inner(), runtime.spawn_point_creatures())
        };
        let (sleeps, creatures) = drive();
        let window = crate::ai_think::CREATUREAI0_THINK_WINDOW_MICROS;
        // One wake per owner window up to and including the native admission pass.
        assert_eq!(
            sleeps,
            (1..=admit_at / window + 1)
                .map(|n| n * window)
                .collect::<Vec<_>>()
        );
        assert_eq!((sleeps, creatures), drive());
    }

    #[test]
    fn world_drive_stops_on_a_retired_owner_and_on_shutdown_before_any_pass() {
        let (mut runtime, current, job) = setup();
        let (point, _, live) = runtime.spawn_point_creatures()[0];
        assert!(runtime.kill_creature_for_test(live.expect("live rat")));
        let runtime = tokio::sync::Mutex::new(runtime);
        let shutdown = oteryn_foundation::CancellationToken::new();
        let mut wake = VirtualWake::new(0, u64::MAX, &shutdown);
        wake.retire = Some(&runtime);
        assert_eq!(
            block_on(job.drive(&runtime, &current, &wake, &shutdown)),
            RespawnDriveEnd::StaleOwner
        );
        assert_eq!(wake.sleeps.into_inner(), vec![50_000]);
        // The first pass observed the death; the retired owner never warned or admitted.
        let runtime = runtime.into_inner();
        assert!(
            runtime
                .pending_respawn(point)
                .is_some_and(|p| p.due_micros > 50_000)
        );
        assert_eq!(runtime.spawn_point_creatures()[0].2, None);

        let (mut runtime, current, job) = setup();
        let (point, _, live) = runtime.spawn_point_creatures()[0];
        assert!(runtime.kill_creature_for_test(live.expect("live rat")));
        let runtime = tokio::sync::Mutex::new(runtime);
        let shutdown = oteryn_foundation::CancellationToken::new();
        shutdown.cancel();
        let wake = VirtualWake::new(0, u64::MAX, &shutdown);
        assert_eq!(
            block_on(job.drive(&runtime, &current, &wake, &shutdown)),
            RespawnDriveEnd::Shutdown
        );
        assert!(wake.sleeps.into_inner().is_empty());
        assert_eq!(runtime.into_inner().pending_respawn(point), None);
    }

    #[test]
    fn world_drive_ends_when_semantic_time_has_no_further_window() {
        let (runtime, current, job) = setup();
        let runtime = tokio::sync::Mutex::new(runtime);
        let shutdown = oteryn_foundation::CancellationToken::new();
        let wake = VirtualWake::new(u64::MAX, u64::MAX, &shutdown);
        assert_eq!(
            block_on(job.drive(&runtime, &current, &wake, &shutdown)),
            RespawnDriveEnd::ClockExhausted
        );
        assert!(wake.sleeps.into_inner().is_empty());
    }

    #[test]
    fn monotonic_wake_reads_its_origin_and_completes_past_deadlines() {
        let origin = std::time::Instant::now();
        let wake = MonotonicRespawnWake::from_origin(origin);
        let first = wake.now();
        assert!(wake.now() >= first);
        block_on(wake.sleep_until(SemanticTimeMicros::from_micros(0)));
        block_on(wake.sleep_until(first));
        assert!(wake.now() >= first);
    }

    #[test]
    fn next_wake_never_wraps_at_the_end_of_semantic_time() {
        let (mut runtime, current, mut job) = setup();
        assert_eq!(
            job.run(&mut runtime, &current, &at(u64::MAX))
                .expect("last time")
                .next_wake,
            None
        );
    }
}
