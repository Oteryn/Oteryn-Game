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
    ChannelRuntimeV1, ExactActorRef, GenerationError, RuntimeScopeRefV1, RuntimeWorkStamp,
    ScopeOwnershipGeneration, ScopeRuntimeFence,
};
use crate::gameplay_transport::actor_spell::ChannelSpellStates;
use crate::monster_summon::{
    NativeSummonCatalog, NativeSummonOwner, SummonError, SummonLocationPolicy, SummonReceipt,
};

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
