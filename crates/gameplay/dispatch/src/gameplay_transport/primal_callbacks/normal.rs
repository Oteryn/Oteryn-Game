//! Private continuation of the original Primal source/native owner turn.
use super::*;
use crate::ai_think::profile_schedule::ProfileAbilityProposal;
use crate::ai_think::summon_plan::ReservedNativeThinkBatch;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug, Default)]
struct RetainedNativeResults {
    proposals: Vec<ProfileAbilityProposal>,
    actors: Vec<ExactActorRef>,
    overflow: bool,
}

/// Original opaque reservation, never recreated from actor IDs or a later clock.
pub(in crate::gameplay_transport) struct PendingNormalPrimal {
    batch: Arc<ReservedNativeThinkBatch>,
    source_known: AtomicBool,
    ai_known: AtomicBool,
    due_known: AtomicBool,
    results: Mutex<RetainedNativeResults>,
}
impl std::fmt::Debug for PendingNormalPrimal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingNormalPrimal")
            .field("source_known", &self.source_known.load(Ordering::Acquire))
            .field("ai_known", &self.ai_known.load(Ordering::Acquire))
            .finish_non_exhaustive()
    }
}
impl PendingNormalPrimal {
    fn prepare(batch: ReservedNativeThinkBatch) -> Result<Arc<Self>, ()> {
        let count = batch.occurrences().len();
        let bound = count
            .checked_mul(
                crate::ai_think::profile_schedule::MAX_ATTACK_ENTRIES
                    + crate::ai_think::profile_schedule::MAX_DEFENCE_ENTRIES,
            )
            .ok_or(())?;
        let mut results = RetainedNativeResults::default();
        results.proposals.try_reserve_exact(bound).map_err(|_| ())?;
        results.actors.try_reserve_exact(count).map_err(|_| ())?;
        Ok(Arc::new(Self {
            batch: Arc::new(batch),
            source_known: AtomicBool::new(false),
            ai_known: AtomicBool::new(false),
            due_known: AtomicBool::new(false),
            results: Mutex::new(results),
        }))
    }
    pub(in crate::gameplay_transport) fn batch(&self) -> &ReservedNativeThinkBatch {
        &self.batch
    }
    pub(in crate::gameplay_transport) fn source_known(&self) -> bool {
        self.source_known.load(Ordering::Acquire)
    }
    pub(in crate::gameplay_transport) fn source_ready(&self) -> bool {
        self.source_known() && self.due_known.load(Ordering::Acquire)
    }
    pub(in crate::gameplay_transport) fn ai_known(&self) -> bool {
        self.ai_known.load(Ordering::Acquire)
    }
    pub(in crate::gameplay_transport) fn finish_ai(&self) {
        self.ai_known.store(true, Ordering::Release);
    }
    pub(in crate::gameplay_transport) fn record_report(
        &self,
        actor: ExactActorRef,
        proposals: impl Iterator<Item = ProfileAbilityProposal>,
    ) {
        let mut results = self
            .results
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if results.actors.contains(&actor) {
            return;
        }
        for proposal in proposals {
            if results.proposals.len() == results.proposals.capacity() {
                results.overflow = true;
                return;
            }
            results.proposals.push(proposal);
        }
        if results.actors.len() == results.actors.capacity() {
            results.overflow = true;
            return;
        }
        results.actors.push(actor);
    }
    pub(in crate::gameplay_transport) fn results(
        &self,
    ) -> Result<(Vec<ProfileAbilityProposal>, Vec<ExactActorRef>), ()> {
        let results = self
            .results
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !self.ai_known() || results.overflow {
            return Err(());
        }
        let mut proposals = Vec::new();
        proposals
            .try_reserve_exact(results.proposals.len())
            .map_err(|_| ())?;
        proposals.extend(results.proposals.iter().cloned());
        let mut actors = Vec::new();
        actors
            .try_reserve_exact(results.actors.len())
            .map_err(|_| ())?;
        actors.extend_from_slice(&results.actors);
        Ok((proposals, actors))
    }
}

impl PrimalCallbacks {
    pub(in crate::gameplay_transport) fn normal_pending(&self) -> Option<Arc<PendingNormalPrimal>> {
        self.normal.as_ref().map(Arc::clone)
    }
    pub(in crate::gameplay_transport) fn prepare_normal(
        &mut self,
        batch: ReservedNativeThinkBatch,
    ) -> Result<(), ()> {
        if self.normal.is_some() {
            return Err(());
        }
        self.normal = Some(PendingNormalPrimal::prepare(batch)?);
        Ok(())
    }
    pub(in crate::gameplay_transport) fn complete_normal(
        &mut self,
        expected: &Arc<PendingNormalPrimal>,
    ) -> Result<(), ()> {
        if !expected.ai_known()
            || !self
                .normal
                .as_ref()
                .is_some_and(|pending| Arc::ptr_eq(pending, expected))
        {
            return Err(());
        }
        self.normal = None;
        Ok(())
    }
}

impl super::super::ComposedFreshAdmission<'_, '_, '_> {
    /// The lane stays held; every volatile native guard ends before SQL begins.
    pub(in crate::gameplay_transport) async fn finish_primal_normal_source(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
    ) -> Result<(), ()> {
        let room = self.rooms().gate_monster_ai().ok_or(())?;
        let native = self
            .active_generation
            .and_then(|active| active.native_gameplay())
            .ok_or(())?;
        let (pending, plan, scope, generation) = {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.try_lock().map_err(|_| ())?;
            let pending = states.primal_callbacks.normal_pending().ok_or(())?;
            if !pending.batch().current_for(&runtime) {
                return Err(());
            }
            if pending.source_known() {
                drop(states);
                drop(runtime);
                return self.finish_primal_normal_deadlines(permit, &pending).await;
            }
            if self
                .states
                .attack
                .try_lock()
                .map_err(|_| ())?
                .kills()
                .core_head()?
                .is_some()
            {
                return Err(());
            }
            if room.pinned_activity_source_room()
                != Some(crate::content::bosses::activity_template::PinnedActivitySourceRoom::Primal)
                || !matches!(
                    runtime.binding().runtime_scope(),
                    crate::foundation::RuntimeScopeRefV1::Instance { .. }
                )
                || room.compiled().server_digest() != native.source_digest()
                || native.source_digest() != runtime.content_pin().server_artifact_digest()
                || !native.qualifies_primal_callbacks(native.source_digest())
            {
                return Err(());
            }
            let owner = states.primal_callbacks.owner.try_lock().map_err(|_| ())?;
            let census = runtime.positioned_actor_census().map_err(|_| ())?;
            let boss = owner.boss.or_else(|| {
                census.iter().find_map(|(actor, position, session)| {
                    (session.is_none()
                        && in_room(position.position())
                        && runtime.matches_live_creature_identity(
                            *actor,
                            primal::PRIMAL_MENACE_KEY.as_bytes(),
                        ))
                    .then_some(*actor)
                })
            });
            let Some(boss) = boss else {
                pending.source_known.store(true, Ordering::Release);
                drop(owner);
                drop(states);
                drop(runtime);
                return self.finish_primal_normal_deadlines(permit, &pending).await;
            };
            if !pending
                .batch()
                .occurrences()
                .iter()
                .any(|occurrence| occurrence.actor == boss)
            {
                pending.source_known.store(true, Ordering::Release);
                drop(owner);
                drop(states);
                drop(runtime);
                return self.finish_primal_normal_deadlines(permit, &pending).await;
            }
            let Some(snapshot) = source_snapshot_or_skip(&runtime, boss, &pending)? else {
                // Source onThink does not run for a dead/retired boss. Keep
                // the SAME reservation: live minions still own their thinks.
                pending.source_known.store(true, Ordering::Release);
                drop(owner);
                drop(states);
                drop(runtime);
                return self.finish_primal_normal_deadlines(permit, &pending).await;
            };
            let center = owner.think.centre().unwrap_or(snapshot.position);
            let mut positions = Vec::new();
            positions.try_reserve_exact(121).map_err(|_| ())?;
            // Fresh getSpawnPosition reads exactly radius5. This phase only
            // records scheduled work; each later admission proves its own
            // SourceSpawnPlan candidates, including any fallback neighbours.
            for dx in -5..=5 {
                for dy in -5..=5 {
                    positions.push(MovementLocalPosition {
                        x: center.x.checked_add(dx).ok_or(())?,
                        y: center.y.checked_add(dy).ok_or(())?,
                        floor: center.floor,
                    });
                }
            }
            let objects = self.door.try_lock().map_err(|_| ())?;
            let plan = crate::movement::monster_map::PreparedMonsterMapRead::prepare(
                &runtime, room, &objects, &positions,
            )
            .ok_or(())?;
            (
                pending,
                plan,
                runtime.binding().runtime_scope(),
                runtime.binding().scope_generation().get(),
            )
        };
        let mut context = (self, permit, plan, room, native, Arc::clone(&pending));
        self.root
            .try_issue_semantic_pass()
            .map_err(|_| ())?
            .run_with_context(&mut context, move |holder, deadline, context| {
                Box::pin(async move {
                    use crate::durability::spell_item_transaction::{
                        assert_spell_item_scope_in_transaction, begin_spell_owner_transaction,
                    };
                    let (admission, permit, plan, room, native, pending) = context;
                    let mut tx = begin_spell_owner_transaction(holder, deadline).await?;
                    let authority = assert_spell_item_scope_in_transaction(
                        &mut tx,
                        permit,
                        admission.root,
                        admission.character,
                        admission.holder,
                        scope,
                        generation,
                    )
                    .await
                    .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                    #[cfg(test)]
                    NORMAL_MAP_READ_EPOCH.fetch_add(1, Ordering::Relaxed);
                    let read = plan
                        .read(&mut tx, &authority)
                        .await
                        .ok_or(crate::durability::DurabilityError::Unavailable)?;
                    {
                        let mut runtime = admission.runtime.lock().await;
                        let mut states = admission
                            .states
                            .spell_states
                            .try_lock()
                            .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                        let objects = admission
                            .door
                            .try_lock()
                            .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                        if !pending.batch().current_for(&runtime)
                            || !states
                                .primal_callbacks
                                .normal
                                .as_ref()
                                .is_some_and(|current| Arc::ptr_eq(current, pending))
                            || admission
                                .states
                                .attack
                                .try_lock()
                                .map_err(|_| crate::durability::DurabilityError::Unavailable)?
                                .kills()
                                .core_head()
                                .map_err(|_| crate::durability::DurabilityError::Unavailable)?
                                .is_some()
                        {
                            return Err(crate::durability::DurabilityError::Unavailable);
                        }
                        let view = read
                            .into_current_view(&runtime, room, &objects, native)
                            .ok_or(crate::durability::DurabilityError::Unavailable)?;
                        if !pending.source_known() {
                            publish_source(&mut runtime, &mut states, room, native, &view, pending)
                                .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                        }
                    }
                    tx.rollback()
                        .await
                        .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                    Ok(())
                })
            })
            .await
            .map_err(|_| ())?;
        self.finish_primal_normal_deadlines(permit, &pending).await
    }
    async fn finish_primal_normal_deadlines(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        pending: &Arc<PendingNormalPrimal>,
    ) -> Result<(), ()> {
        for _ in 0..256 {
            if self
                .drain_primal_retained_through(permit, None, pending.batch().observed_at_us())
                .await?
            {
                pending.due_known.store(true, Ordering::Release);
                return Ok(());
            }
        }
        Err(())
    }
}

fn publish_source(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &crate::movement::monster_map::MonsterMap,
    pending: &PendingNormalPrimal,
) -> Result<(), ()> {
    let now = pending.batch().observed_at_us();
    let census = runtime.positioned_actor_census().map_err(|_| ())?;
    let retained = states.primal_callbacks.retained_owner();
    let mut owner = retained.try_lock().map_err(|_| ())?;
    if owner
        .content
        .is_some_and(|content| content != native.source_digest())
    {
        return Err(());
    }
    if owner.think.centre().is_none()
        && !census
            .iter()
            .any(|(_, position, session)| session.is_some() && in_room(position.position()))
    {
        pending.source_known.store(true, Ordering::Release);
        return Ok(());
    }
    let boss = match owner.boss {
        Some(actor) => actor,
        None => {
            let bosses = census
                .iter()
                .filter(|(actor, position, session)| {
                    session.is_none()
                        && in_room(position.position())
                        && runtime.matches_live_creature_identity(
                            *actor,
                            primal::PRIMAL_MENACE_KEY.as_bytes(),
                        )
                        && runtime
                            .companion_snapshot(*actor)
                            .is_ok_and(|snapshot| snapshot.state.master.is_none())
                        && runtime.native_summon_master(*actor).is_none()
                })
                .map(|(actor, ..)| *actor)
                .collect::<Vec<_>>();
            if bosses.len() != 1 {
                return Err(());
            }
            bosses[0]
        }
    };
    let Some(snapshot) = source_snapshot_or_skip(runtime, boss, pending)? else {
        // Recheck after the SQL wait as well; a cancelled/resumed turn may
        // observe a real retirement after its immutable plan was prepared.
        pending.source_known.store(true, Ordering::Release);
        return Ok(());
    };
    if !in_room(snapshot.position) {
        return Err(());
    }
    let root = GameplayDecisionRoot::from_bytes(
        Sha256::new()
            .chain_update(b"srx3:primal:callbacks:v1")
            .chain_update(native.source_digest())
            .chain_update(boss.placement_identity())
            .finalize()
            .into(),
    );
    let source_ms = (now / 1_000_000) * 1000;
    let shares = runtime
        .borrow_combat_death()
        .current_session_damage_shares(boss)
        .map_err(|_| ())?;
    let levels = shares
        .iter()
        .filter(|share| share.total > 0)
        .filter_map(|share| {
            let session = share.session?;
            census.iter().find_map(|(actor, _, bound)| {
                if *bound != Some(session) {
                    return None;
                }
                let lease = runtime.bound_attacker_lease(*actor, session).ok()?;
                (lease.character_id() == share.character
                    && Some(lease.generation()) == share.lease_generation)
                    .then(|| runtime.hazard_level(*actor, session))
                    .flatten()
            })
        })
        .collect::<Vec<_>>();
    let positions = owner
        .beasts
        .iter()
        .filter_map(|(id, actor)| {
            runtime
                .companion_snapshot(*actor)
                .ok()
                .filter(|snapshot| snapshot.health > 0)
                .map(|snapshot| (*id, snapshot.position))
        })
        .collect::<Vec<_>>();
    let center = owner.think.centre().unwrap_or(snapshot.position);
    let radius = i32::try_from(primal::SPAWN_RADIUS_TILES).map_err(|_| ())?;
    let registered = runtime
        .source_registered_tile_actor_positions()
        .map_err(|_| ())?;
    let mut candidate_facts = std::collections::BTreeMap::new();
    for dx in -radius..=radius {
        for dy in -radius..=radius {
            let at = MovementLocalPosition {
                x: center.x.checked_add(dx).ok_or(())?,
                y: center.y.checked_add(dy).ok_or(())?,
                floor: center.floor,
            };
            let surface = map.source_spawn_surface_admits(runtime, at).ok_or(())?;
            let occupied = registered.contains(&at);
            candidate_facts.insert((at.x, at.y, at.floor), surface && !occupied);
        }
    }
    let unavailable_candidate = std::cell::Cell::new(false);
    let mut next_think = owner.think.clone();
    let mut ordinal = 0;
    let effects = next_think
        .think(
            &PrimalMenaceThinkFacts {
                position: snapshot.position,
                now_ms: source_ms,
                health: u64::try_from(snapshot.health).map_err(|_| ())?,
                max_health: u64::try_from(snapshot.maximum_health).map_err(|_| ())?,
                damage_map_levels: &levels,
            },
            || draw(&root, now, &mut ordinal),
            |at| match candidate_facts.get(&(at.x, at.y, at.floor)) {
                Some(clear) => *clear,
                None => {
                    unavailable_candidate.set(true);
                    false
                }
            },
            |id| {
                positions
                    .iter()
                    .find_map(|(key, position)| (*key == id).then_some(*position))
            },
        )
        .map_err(|_| ())?;
    if unavailable_candidate.get() {
        return Err(());
    }
    let pods = effects
        .iter()
        .filter(|effect| {
            matches!(
                effect,
                PrimalMenaceEffect::ScheduledSpawn {
                    kind: WaveKind::Pod,
                    ..
                }
            )
        })
        .count();
    if states.primal_callbacks.pods.len() + pods > 32
        || owner.deferred.0.len() + effects.len() - pods > 128
    {
        return Err(());
    }
    let mut pod_callbacks = Vec::new();
    pod_callbacks.try_reserve_exact(pods).map_err(|_| ())?;
    let mut deferred = owner.deferred.clone();
    let mut next_id = owner.next_id;
    for mut effect in effects {
        if let PrimalMenaceEffect::ScheduledSpawn {
            kind: WaveKind::Pod,
            position,
            ..
        } = effect
        {
            next_id = next_id.checked_add(1).ok_or(())?;
            let callback=crate::durability::creature_source_items::PrimalPodCallback::from_current_native_wave(
                runtime,room,native,boss,position,next_id,now).map_err(|_|())?;
            pod_callbacks.push(Arc::new(callback));
            continue;
        }
        let offset = now / 1000 - source_ms;
        match &mut effect {
            PrimalMenaceEffect::SpawnSparkle { at_ms, .. }
            | PrimalMenaceEffect::ScheduledSpawn { at_ms, .. } => {
                *at_ms = at_ms.saturating_add(offset)
            }
            PrimalMenaceEffect::ReplaceWithFungosaurus { .. } => {}
        }
        deferred.enqueue_at(effect, now / 1000)?;
    }
    states
        .primal_callbacks
        .pods
        .try_reserve(pod_callbacks.len())
        .map_err(|_| ())?;
    owner.content = Some(native.source_digest());
    owner.boss = Some(boss);
    owner.think = next_think;
    owner.deferred = deferred;
    owner.next_id = next_id;
    states.primal_callbacks.pods.extend(pod_callbacks);
    pending.source_known.store(true, Ordering::Release);
    Ok(())
}

fn source_snapshot_or_skip(
    runtime: &ChannelRuntimeV1,
    boss: ExactActorRef,
    pending: &PendingNormalPrimal,
) -> Result<Option<oteryn_gameplay_entities::CompanionSnapshot>, ()> {
    let snapshot = current_source_snapshot(runtime, boss)?;
    if snapshot.is_none() {
        pending.source_known.store(true, Ordering::Release);
    }
    Ok(snapshot)
}

fn current_source_snapshot(
    runtime: &ChannelRuntimeV1,
    boss: ExactActorRef,
) -> Result<Option<oteryn_gameplay_entities::CompanionSnapshot>, ()> {
    runtime.owner_fence().map_err(|_| ())?;
    if boss.runtime_scope() != runtime.binding().runtime_scope()
        || boss.scope_generation() != runtime.binding().scope_generation()
    {
        return Err(());
    }
    match runtime.companion_snapshot_including_dead(boss) {
        Ok(snapshot) if snapshot.health > 0 => Ok(Some(snapshot)),
        Ok(_) => Ok(None),
        Err(
            oteryn_gameplay_domain::domain::carrier_values::CarrierError::StaleActorGeneration
            | oteryn_gameplay_domain::domain::carrier_values::CarrierError::NotCreature,
        ) => Ok(None),
        Err(_) => Err(()),
    }
}

#[cfg(test)]
mod lifecycle_controls {
    use super::*;
    use crate::content::channel_content_pin::ChannelContentPin;
    use crate::content::creature_policy::{
        CompiledCreaturePolicies, CompiledCreaturePolicy, CreatureFlags,
    };
    use crate::foundation::{ChannelId, NodeId, WorldId};
    use oteryn_gameplay_entities::OwnerDamageCommand;
    type TestResult<T = ()> = Result<T, String>;
    fn id(seed: u8) -> [u8; 16] {
        let mut id = [seed; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        id
    }
    // This is the ordinary carrier's lifecycle fixture, NOT a native Primal
    // source/Room/Event proof. Production source qualification stays outside
    // this private DATA classifier and is exercised by the real PG caller.
    fn runtime(scope: u8) -> TestResult<(ChannelRuntimeV1, ExactActorRef)> {
        let world = WorldId::decode(&id(1)).map_err(|error| format!("world: {error:?}"))?;
        let pin = ChannelContentPin::from_activation(
            world,
            1,
            [1; 32],
            [2; 32],
            [3; 32],
            [4; 32],
            (0, 0, 0),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&id(scope)).map_err(|error| format!("scope: {error:?}"))?,
            NodeId::decode(&id(3)).map_err(|error| format!("node: {error:?}"))?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            pin,
        )
        .map_err(|error| format!("runtime: {error:?}"))?;
        let policies = CompiledCreaturePolicies::from_active_artifact(
            runtime.content_pin().server_artifact_digest(),
            vec![CompiledCreaturePolicy {
                definition_key: "fixture:source-lifecycle".into(),
                definition_revision: "fixture:1".into(),
                display_name: "lifecycle fixture".into(),
                maximum_health: 20,
                forge_difficulty: None,
                base_speed: 110,
                outfit_look_type: 21,
                object_look_type: None,
                summonable: false,
                convinceable: false,
                mana_cost: None,
                is_familiar: false,
                condition_immunities: vec![],
                defense: Some(1),
                armor: Some(1),
                mitigation: None,
                resistances: vec![],
                damage_immunities: vec![],
                healing_from_damage: vec![],
                preferred_distance: Some(1),
                reward_boss: Some(false),
                blocked_by_nearby_players: false,
                flags: CreatureFlags {
                    attackable: true,
                    illusionable: false,
                    health_hidden: false,
                },
            }],
        )
        .map_err(|error| format!("fixture policy: {error:?}"))?;
        runtime
            .install_companion_policies(policies)
            .map_err(|error| format!("install: {error:?}"))?;
        let actor = runtime
            .admit_pinned_monster_lab_creature(
                MovementLocalPosition {
                    x: 100,
                    y: 100,
                    floor: 0,
                },
                "fixture:source-lifecycle",
                20,
            )
            .map_err(|error| format!("actor: {error:?}"))?;
        runtime
            .install_creature_policy(actor, "fixture:source-lifecycle")
            .map_err(|error| format!("policy: {error:?}"))?;
        Ok((runtime, actor))
    }
    fn pending(runtime: &mut ChannelRuntimeV1) -> TestResult<Arc<PendingNormalPrimal>> {
        let mut table = crate::ai_think::CreatureAiTable::default();
        let batch = table
            .reserve_current_thinks(runtime, 0)
            .ok_or("actual empty owner reservation")?;
        PendingNormalPrimal::prepare(batch).map_err(|_| "retained owner batch".into())
    }
    #[test]
    fn cancelled_private_phase_treats_actual_hp0_and_retirement_as_known_no_fresh_think()
    -> TestResult {
        use std::future::Future;
        let (mut runtime, actor) = runtime(2)?;
        let phase = pending(&mut runtime)?;
        let mut canonical = PrimalCallbacks {
            normal: Some(Arc::clone(&phase)),
            ..PrimalCallbacks::default()
        };
        assert!(
            source_snapshot_or_skip(&runtime, actor, &phase)
                .map_err(|_| "live source snapshot")?
                .is_some()
        );
        assert!(!phase.source_known());
        let retained = Arc::clone(&phase);
        let mut cancelled = Box::pin(async move {
            let _lease = retained;
            std::future::pending::<()>().await;
        });
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(matches!(
            cancelled.as_mut().poll(&mut context),
            std::task::Poll::Pending
        ));
        assert_eq!(Arc::strong_count(&phase), 3);
        drop(cancelled);
        assert_eq!(Arc::strong_count(&phase), 2);
        let hit = runtime
            .borrow_exact_actor_commit_at_us(1000)
            .commit_damage(
                actor,
                OwnerDamageCommand {
                    target: b"fixture:source-lifecycle",
                    occurrence: b"lifecycle-actual-lethal",
                    binding: b"lifecycle-actual-lethal\0source-lifecycle",
                    damage: 20,
                },
            )
            .map_err(|error| format!("actual lethal: {error:?}"))?;
        assert_eq!(hit.health_after, 0);
        assert!(
            source_snapshot_or_skip(&runtime, actor, &phase)
                .map_err(|_| "actual HP0 classification")?
                .is_none()
        );
        assert!(phase.source_known());
        assert!(
            !phase.ai_known(),
            "source no-op cannot wholesale-complete original native batch"
        );
        assert!(
            !phase.source_ready(),
            "retained due callbacks still have independent work"
        );
        assert_eq!(phase.batch().observed_at_us(), 0);
        assert!(Arc::ptr_eq(
            &canonical
                .normal_pending()
                .ok_or("canonical original phase")?,
            &phase
        ));
        assert!(
            canonical.complete_normal(&phase).is_err(),
            "known source no-op cannot clear an unfinished original AI reservation"
        );
        let retired = pending(&mut runtime)?;
        runtime
            .remove_arena_opponent(actor)
            .map_err(|error| format!("retire: {error:?}"))?;
        assert!(
            source_snapshot_or_skip(&runtime, actor, &retired)
                .map_err(|_| "retired incarnation")?
                .is_none()
        );
        assert!(retired.source_known());
        assert!(!retired.ai_known());
        Ok(())
    }
    #[test]
    fn private_phase_never_turns_wrong_scope_or_retired_owner_into_known_empty() -> TestResult {
        let (mut first, actor) = runtime(2)?;
        let (other, _) = runtime(5)?;
        let phase = pending(&mut first)?;
        assert!(source_snapshot_or_skip(&other, actor, &phase).is_err());
        assert!(!phase.source_known());
        first.retire_owner_cycle();
        assert!(source_snapshot_or_skip(&first, actor, &phase).is_err());
        assert!(!phase.source_known());
        Ok(())
    }
}
