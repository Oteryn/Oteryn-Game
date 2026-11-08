//! Source-qualified Primal Menace callbacks under the actual native creature turn.
//! Crystal 201189f the_primal_menace.lua:156-347 and hazard_primal.lua:60-80.
use super::actor_spell::ChannelSpellStates;
use crate::content::{QualifiedNativeEntryRoom, native_gameplay::NativeGameplayState};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, MovementLocalPosition};
use crate::movement::monster_map::CurrentMonsterMap;
use oteryn_gameplay_encounters::bosses::hazard_primal::primal_menace_think::{
    self as primal, PrimalMenaceEffect, PrimalMenaceThinkFacts, PrimalMenaceThinkOwner, WaveKind,
};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

mod deferred;
#[cfg(test)]
mod retention_tests;
use deferred::DeferredCallbacks;

#[derive(Debug)]
struct PendingPodStep {
    source: crate::durability::creature_source_items::PrimalPodPlayerStep,
    actor: ExactActorRef,
    session: crate::foundation::GameSessionId,
    maximum_health: u32,
    hazard: u16,
    applied_events: std::sync::atomic::AtomicUsize,
}

#[derive(Debug, Default)]
pub(super) struct PrimalCallbacks {
    owner: std::sync::Arc<tokio::sync::Mutex<PrimalCallbackState>>,
    expiries: Vec<crate::durability::creature_source_items::PrimalPodExpiry>,
    expiry_attempted: Vec<[u8; 16]>,
    pod_cells: std::collections::BTreeSet<(i32, i32, i16)>,
    steps: Vec<std::sync::Arc<PendingPodStep>>,
    pub(super) pods:
        Vec<std::sync::Arc<crate::durability::creature_source_items::PrimalPodCallback>>,
}

#[derive(Debug, Default)]
struct PrimalCallbackState {
    content: Option<[u8; 32]>,
    boss: Option<ExactActorRef>,
    think: PrimalMenaceThinkOwner,
    deferred: DeferredCallbacks,
    beasts: Vec<(u64, ExactActorRef)>,
    next_id: u64,
}

fn in_room(p: MovementLocalPosition) -> bool {
    p.floor == 15 && (33547..=33570).contains(&p.x) && (32749..=32769).contains(&p.y)
}
fn draw(root: &GameplayDecisionRoot, now: u64, ordinal: &mut u64) -> u32 {
    let mut occurrence = [0; 16];
    occurrence[..8].copy_from_slice(&now.to_be_bytes());
    let limit = u64::MAX - u64::MAX % u64::from(primal::DRAW_PPM);
    for _ in 0..64 {
        let current = *ordinal;
        *ordinal = ordinal.saturating_add(1);
        if let Ok(value) = deterministic_decision_u64(
            root,
            DecisionOccurrenceId::from_bytes(occurrence),
            "primal_waves",
            current,
        ) && value < limit
        {
            return (value % u64::from(primal::DRAW_PPM)) as u32;
        }
    }
    primal::DRAW_PPM
}

/// Runs on every actual native owner turn, including turns after the boss died,
/// so already-scheduled source closures are not silently discarded.
pub(super) async fn drive(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    actors: &[ExactActorRef],
    now: u64,
) -> Result<Option<u64>, ()> {
    runtime.owner_fence().map_err(|_| ())?;
    if room.pinned_activity_source_room()
        != Some(crate::content::bosses::activity_template::PinnedActivitySourceRoom::Primal)
    {
        return Ok(None);
    }
    if !matches!(
        runtime.binding().runtime_scope(),
        crate::foundation::RuntimeScopeRefV1::Instance { .. }
    ) || native.source_digest() != runtime.content_pin().server_artifact_digest()
    {
        return Ok(None);
    }
    let census = runtime.positioned_actor_census().map_err(|_| ())?;
    let retained = states.primal_callbacks.retained_owner();
    let mut owner = retained.lock_owned().await;
    async {
        if owner.content != Some(native.source_digest()) {
            if !native.qualifies_primal_callbacks(runtime.content_pin().server_artifact_digest()) {
                return Ok(None);
            }
            owner.content = Some(native.source_digest());
        }
        if owner.think.centre().is_none() && !census.iter().any(|(_,position,session)|session.is_some()&&in_room(position.position())) {
            return Ok(None);
        }
        if owner.boss.is_none() {
            let bosses = census
                .iter()
                .filter(|(actor, p, session)| {
                    session.is_none()
                        && in_room(p.position())
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
                return Ok(None);
            }
            owner.boss = Some(bosses[0]);
        }
        let boss = owner.boss.ok_or(())?;
        let snapshot = runtime
            .companion_snapshot(boss)
            .ok()
            .filter(|s| s.health > 0 && in_room(s.position));
        let live = snapshot.is_some();
        let root = GameplayDecisionRoot::from_bytes(
            Sha256::new()
                .chain_update(b"srx3:primal:callbacks:v1")
                .chain_update(native.source_digest())
                .chain_update(boss.placement_identity())
                .finalize()
                .into(),
        );
        let mut ordinal = 0;
        // os.time is integral seconds, including the strict > wave/age checks.
        let source_ms = (now / 1_000_000) * 1_000;
        if let Some(snapshot) = snapshot
            && actors.contains(&boss)
        {
            let center = owner.think.centre().unwrap_or(snapshot.position);
            // Load the complete bounded radius under the current Item+Map scope
            // transaction before the pure callback asks about a candidate cell.
            for dx in -5..=5 {
                for dy in -5..=5 {
                    let at = MovementLocalPosition {
                        x: center.x + dx,
                        y: center.y + dy,
                        floor: center.floor,
                    };
                    map.read(runtime, at).await;
                }
            }
            if !map.validate_transaction(runtime).await {
                return Err(());
            }
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
                        .filter(|s| s.health > 0)
                        .map(|s| (*id, s.position))
                })
                .collect::<Vec<_>>();
            let mut next_think=owner.think.clone();
            let effects = next_think.think(
                    &PrimalMenaceThinkFacts {
                        position: snapshot.position,
                        now_ms: source_ms,
                        health: u64::try_from(snapshot.health).map_err(|_| ())?,
                        max_health: u64::try_from(snapshot.maximum_health).map_err(|_| ())?,
                        damage_map_levels: &levels,
                    },
                    || draw(&root, now, &mut ordinal),
                    |at| map.view().source_spawn_admits(at) && runtime.native_summon_cell_free(at),
                    |id| {
                        positions
                            .iter()
                            .find_map(|(key, p)| (*key == id).then_some(*p))
                    },
                )
                .map_err(|_| ())?;
            let pods=effects.iter().filter(|effect|matches!(effect,PrimalMenaceEffect::ScheduledSpawn{kind:WaveKind::Pod,..})).count();
            if states.primal_callbacks.pods.len()+pods>32 || owner.deferred.0.len()+effects.len()-pods>128 {return Err(());}
            let mut pending=Vec::new();
            for mut effect in effects {
                if let PrimalMenaceEffect::ScheduledSpawn{kind:WaveKind::Pod,position,..}=effect {
                    owner.next_id=owner.next_id.checked_add(1).ok_or(())?;
                    let callback=crate::durability::creature_source_items::PrimalPodCallback::from_current_native_wave(
                        runtime,room,native,boss,position,owner.next_id,now,
                    ).map_err(|_|())?;
                    pending.push(std::sync::Arc::new(callback));
                    continue;
                }
                // addEvent starts now; source wave/age clocks use os.time.
                let offset=now/1_000-source_ms;
                match &mut effect {
                    PrimalMenaceEffect::SpawnSparkle{at_ms,..}|PrimalMenaceEffect::ScheduledSpawn{at_ms,..}=>*at_ms=at_ms.saturating_add(offset),
                    PrimalMenaceEffect::ReplaceWithFungosaurus{..}=>{},
                }
                owner.deferred.enqueue_at(effect, now / 1_000)?;
            }
            states.primal_callbacks.pods.extend(pending);
            owner.think=next_think;
        }
        while let Some(mut callback)=owner.deferred.peek_due(now/1_000,live) {
            let applied:Result<(),()>=async {
                match callback.effect {
                    PrimalMenaceEffect::SpawnSparkle{position,..}=>emit_sparkle(room,position),
                    PrimalMenaceEffect::ScheduledSpawn{kind:WaveKind::Pod,..}=>return Err(()),
                    PrimalMenaceEffect::ScheduledSpawn{kind:WaveKind::Beast,position,..}=>{
                        let mut creation_draw=0;
                        let child=primal::pick_beast(draw(&root,callback.id,&mut creation_draw)).ok_or(())?;
                        let plan=prepare_spawn(runtime,native,map,child,position,&root,callback.id,None).await?;
                        if let Some(actor)=spawn_prepared(runtime,states,native,map,child,&plan,now)? {
                            owner.next_id=owner.next_id.checked_add(1).ok_or(())?;
                            let id = owner.next_id;
                            owner.beasts.push((id,actor));
                            owner.think.record_beast(id,source_ms);
                        }
                    }
                    PrimalMenaceEffect::ReplaceWithFungosaurus{beast,position}=>{
                        let old=owner.beasts.iter().position(|(id,_)|*id==beast)
                            .map(|index|(index,owner.beasts[index].1));
                        if callback.replacement_removed || old.is_some_and(|(_,actor)|runtime.contains_live_creature(actor)) {
                            let removed=if callback.replacement_removed {None}else{old.map(|(_,actor)|actor)};
                            let plan=prepare_spawn(runtime,native,map,primal::FUNGOSAURUS_KEY,position,&root,callback.id,removed).await?;
                            if !callback.replacement_removed {
                                let (index,actor)=old.ok_or(())?;
                                runtime.remove_arena_opponent(actor).map_err(|_|())?;
                                owner.beasts.remove(index);
                                callback.replacement_removed=true;
                            }
                            spawn_prepared(runtime,states,native,map,primal::FUNGOSAURUS_KEY,&plan,now)?;
                        }
                    }
                }
                Ok(())
            }.await;
            if applied.is_err() {
                let retained = owner.deferred.0.iter_mut().find(|queued| queued.id == callback.id).ok_or(())?;
                retained.replacement_removed = callback.replacement_removed;
                return Err(());
            }
            owner.deferred.acknowledge(callback.id)?;
        }
        while let Some(expiry) = states.primal_callbacks.expiries.first().copied() {
            if states.primal_callbacks.expiry_attempted.contains(&expiry.source_transaction_id){
                states.primal_callbacks.expiries.remove(0);
                continue;
            }
            if states.primal_callbacks.expiry_attempted.len()>=512{return Err(());}
            if !in_room(expiry.position) || map.read(runtime,expiry.position).await.is_none()
                || map.source_tile_exists(runtime,expiry.position).is_none()
                || !map.validate_transaction(runtime).await {return Err(());}
            let target=native.creature_profiles().records.iter().find(|record|record.profile.target.key==primal::FUNGOSAURUS_KEY)
                .ok_or(())?;
            if let Ok(actor)=runtime.spawn_primal_pod_fungosaurus(expiry.position,|r,p|{
                if !map.view().matches(r) { return None; }
                map.source_tile_exists(r,p)
            }) {
                if runtime.install_creature_policy(actor,primal::FUNGOSAURUS_KEY).is_err()
                    || states.monster_melee.creature_ai.admit_native_creature(runtime,native,actor,&target.profile.target,now).is_err() {
                    runtime.remove_arena_opponent(actor).map_err(|_|())?;
                    return Err(());
                }
                emit_pod_speech(runtime,room,actor);
            }
            states.primal_callbacks.expiry_attempted.push(expiry.source_transaction_id);
            states.primal_callbacks.expiries.remove(0);
        }
        owner
            .beasts
            .retain(|(_, actor)| runtime.contains_live_creature(*actor));
        let due = owner.deferred.next_due().map(|ms| ms.saturating_mul(1_000))
            .into_iter().chain(states.primal_callbacks.pods.iter().map(|pod|pod.due_us())).min();
        Ok(due)
    }
    .await
}

#[allow(clippy::too_many_arguments)]
async fn prepare_spawn(
    runtime: &ChannelRuntimeV1,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    child: &str,
    center: MovementLocalPosition,
    root: &GameplayDecisionRoot,
    creation: u64,
    removed: Option<ExactActorRef>,
) -> Result<oteryn_gameplay_world::movement::source_placement::SourcePlacementPlan, ()> {
    use oteryn_gameplay_world::movement::source_placement::SourcePlacementPlan;
    if !in_room(center)
        || !native
            .creature_profiles()
            .records
            .iter()
            .any(|record| record.profile.target.key == child)
    {
        return Err(());
    }
    let mut occurrence = [0; 16];
    occurrence[..8].copy_from_slice(&creation.to_be_bytes());
    let occurrence = DecisionOccurrenceId::from_bytes(occurrence);
    let mut ordinal = 0;
    let plan = SourcePlacementPlan::prepare(center, false, false, |bound| {
        let limit = u64::MAX - u64::MAX % u64::from(bound);
        for _ in 0..64 {
            let value =
                deterministic_decision_u64(root, occurrence, "primal_source_placement", ordinal);
            ordinal += 1;
            if let Ok(value) = value
                && value < limit
            {
                return (value % u64::from(bound)) as u32;
            }
        }
        bound
    })
    .map_err(|_| ())?;
    for at in plan.candidates().iter().copied() {
        // The qualified selector distinguishes actual captured absence from
        // an unavailable item/map read; a failed read alone cannot do that.
        map.read(runtime, at).await;
        let cell = |at| match removed {
            Some(actor) => map.source_placement_cell_after_removal(runtime, child, at, actor),
            None => map.source_placement_cell(runtime, child, at),
        };
        cell(at).ok_or(())?;
        if plan.select(cell, |_, _| Some(true)).is_ok() {
            if !map.validate_transaction(runtime).await {
                return Err(());
            }
            return Ok(plan);
        }
    }
    Err(())
}
fn spawn_prepared(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    native: &NativeGameplayState,
    map: &CurrentMonsterMap<'_, '_>,
    child: &str,
    plan: &oteryn_gameplay_world::movement::source_placement::SourcePlacementPlan,
    now: u64,
) -> Result<Option<ExactActorRef>, ()> {
    let at = plan
        .select(
            |at| map.source_placement_cell(runtime, child, at),
            |_, _| Some(true),
        )
        .map_err(|_| ())?;
    let Some(at) = at else {
        return Ok(None);
    };
    let record = native
        .creature_profiles()
        .records
        .iter()
        .find(|record| record.profile.target.key == child)
        .ok_or(())?;
    let actor = match runtime.spawn_arena_opponent(child, at, |r, p| {
        if !map.view().matches(r) {
            return None;
        }
        map.source_placement_cell(r, child, p)
            .map(|cell| cell.query_add)
    }) {
        Ok(actor) => actor,
        Err(_) => return Ok(None),
    };
    let registration = runtime
        .install_creature_policy(actor, child)
        .map_err(|_| ())
        .and_then(|()| {
            states
                .monster_melee
                .creature_ai
                .admit_native_creature(runtime, native, actor, &record.profile.target, now)
                .map_err(|_| ())
        });
    if registration.is_err() {
        runtime.remove_arena_opponent(actor).map_err(|_| ())?;
        return Err(());
    }
    Ok(Some(actor))
}

fn emit_sparkle(room: &QualifiedNativeEntryRoom, position: MovementLocalPosition) {
    use crate::content::native_cell_lookup::NativeMovementCollisionIndex;
    use crate::map::boot::BundleCollisionIndexExt;
    if let NativeMovementCollisionIndex::Bundle(collision) = room.movement_cells().index()
        && let (Ok(x), Ok(y), Some(floor)) = (
            u16::try_from(position.x),
            u16::try_from(position.y),
            i8::try_from(position.floor).ok().and_then(i8::checked_neg),
        )
    {
        collision
            .map()
            .facts
            .emit_movement_effect(crate::map::overlay::TilePos { x, y, floor }, 11);
    }
}

impl PrimalCallbacks {
    #[cfg(test)]
    pub(super) fn source_started_for_test(&self) -> bool {
        self.owner
            .try_lock()
            .is_ok_and(|owner| owner.think.centre().is_some())
    }
    fn retained_owner(&self) -> std::sync::Arc<tokio::sync::Mutex<PrimalCallbackState>> {
        std::sync::Arc::clone(&self.owner)
    }
    /// Test DATA only: no constructor, scheduling operation or callback authority.
    #[cfg(test)]
    pub(super) fn deferred_original_deadlines_for_test(&self) -> Result<Vec<(u64, u64)>, ()> {
        let owner = self.owner.try_lock().map_err(|_| ())?;
        Ok(owner.deferred.original_deadlines_for_test())
    }
    pub(super) fn has_pending_step(
        &self,
        actor: ExactActorRef,
        session: crate::foundation::GameSessionId,
    ) -> bool {
        self.steps
            .iter()
            .any(|step| step.actor == actor && step.session == session)
    }
    pub(super) fn has_native_work(&self) -> bool {
        !self.expiries.is_empty()
            || !self.pods.is_empty()
            || !self.steps.is_empty()
            || self
                .owner
                .try_lock()
                .map_or(true, |owner| !owner.deferred.0.is_empty())
    }
    pub(super) fn has_native_due(&self, now: u64) -> bool {
        !self.expiries.is_empty()
            || !self.steps.is_empty()
            || self.pods.iter().any(|pod| pod.due_us() <= now)
            || self.owner.try_lock().map_or(true, |owner| {
                owner
                    .deferred
                    .next_due()
                    .is_some_and(|due| due.saturating_mul(1_000) <= now)
            })
    }
    pub(super) fn next_due_us(&self) -> Option<u64> {
        if !self.expiries.is_empty() || !self.steps.is_empty() {
            return Some(0);
        }
        let deferred = match self.owner.try_lock() {
            Ok(owner) => owner
                .deferred
                .next_due()
                .map(|due| due.saturating_mul(1_000)),
            Err(_) => return Some(0),
        };
        deferred
            .into_iter()
            .chain(self.pods.iter().map(|pod| pod.due_us()))
            .min()
    }
}

impl super::ComposedFreshAdmission<'_, '_, '_> {
    /// A sealed immutable callback crosses SQL. Native runtime/spell guards are
    /// released before the writer independently proves the current Node/scope.
    pub(super) async fn drain_primal_pod_callbacks(&self) {
        let Some(permit) = self.spell_lane_permit().await else {
            return;
        };
        for _ in 0..4 {
            let now = self.owner_now().get();
            let pending = {
                let runtime = self.runtime.lock().await;
                let states = self.states.spell_states.lock().await;
                if runtime.owner_fence().is_err() {
                    return;
                }
                states
                    .primal_callbacks
                    .pods
                    .iter()
                    .find(|pod| pod.due_us() <= now)
                    .map(std::sync::Arc::clone)
            };
            let Some(pending) = pending else {
                return;
            };
            let result = self
                .root
                .commit_primal_pod_callback(&permit, self.character, self.holder, &pending, now)
                .await;
            if let Err(error) = result {
                #[cfg(test)]
                eprintln!("SRX3_PRIMAL_ACTUAL_POD_WRITER_REFUSAL {error:?}");
                #[cfg(not(test))]
                let _ = error;
                return;
            }
            self.states.loop_ground.invalidate();
            let mut states = self.states.spell_states.lock().await;
            let at = pending.position();
            states
                .primal_callbacks
                .pod_cells
                .insert((at.x, at.y, at.floor));
            states
                .primal_callbacks
                .pods
                .retain(|pod| !std::sync::Arc::ptr_eq(pod, &pending));
        }
    }
}

fn emit_pod_speech(
    runtime: &ChannelRuntimeV1,
    room: &QualifiedNativeEntryRoom,
    actor: ExactActorRef,
) {
    use crate::content::native_cell_lookup::NativeMovementCollisionIndex;
    use crate::map::boot::BundleCollisionIndexExt;
    if let NativeMovementCollisionIndex::Bundle(collision) = room.movement_cells().index()
        && let Some(speech) = crate::map::facts::MovementSpeech::capture_monster(
            runtime,
            actor,
            36,
            "The primal pod explode and wild emerges from it.",
        )
    {
        collision.map().facts.emit_movement_speech(speech);
    }
}

impl super::ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn drain_primal_pod_expiries(&self) {
        let Some(permit) = self.spell_lane_permit().await else {
            return;
        };
        let (scope, generation) = {
            let runtime = self.runtime.lock().await;
            if runtime.owner_fence().is_err() {
                return;
            }
            (
                runtime.binding().runtime_scope(),
                runtime.binding().scope_generation().get(),
            )
        };
        if !matches!(scope, crate::foundation::RuntimeScopeRefV1::Instance { .. }) {
            return;
        }
        for _ in 0..4 {
            let ack = {
                self.states
                    .spell_states
                    .lock()
                    .await
                    .primal_callbacks
                    .expiry_attempted
                    .first()
                    .copied()
            };
            if let Some(source) = ack {
                if self
                    .root
                    .acknowledge_primal_pod_expiry(
                        &permit,
                        self.character,
                        self.holder,
                        scope,
                        generation,
                        source,
                    )
                    .await
                    .is_err()
                {
                    return;
                }
                self.states
                    .spell_states
                    .lock()
                    .await
                    .primal_callbacks
                    .expiry_attempted
                    .retain(|id| *id != source);
            }
            let expired = match self
                .root
                .drain_primal_pod_expiry(&permit, self.character, self.holder, scope, generation)
                .await
            {
                Ok(Some(expired)) => expired,
                _ => return,
            };
            self.states.loop_ground.invalidate();
            if !expired.requires_spawn {
                continue;
            }
            let mut states = self.states.spell_states.lock().await;
            if states
                .primal_callbacks
                .expiries
                .iter()
                .any(|pending| pending.source_transaction_id == expired.source_transaction_id)
                || states
                    .primal_callbacks
                    .expiry_attempted
                    .contains(&expired.source_transaction_id)
            {
                return;
            }
            if states.primal_callbacks.expiries.len() >= 4 {
                return;
            }
            states.primal_callbacks.expiries.push(expired);
            states.owner_wake.notify_one();
        }
    }
}

/// This is called only by the already committed STEP work item. A standing
/// player or client item-use request cannot enqueue an entry callback.
pub(super) async fn step_in(
    owner: &super::ComposedFreshAdmission<'_, '_, '_>,
    permit: &crate::durability::spell_owner_commit::SpellLanePermit,
    actor: ExactActorRef,
    session: crate::foundation::GameSessionId,
    committed: crate::foundation::MovementPositionSnapshot,
    now: u64,
) {
    let Some(room) = owner.effective_rooms().gate_field_step_ingress() else {
        return;
    };
    let Some(native) = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
    else {
        return;
    };
    {
        let runtime = owner.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        let at = committed.position();
        if !states
            .primal_callbacks
            .pod_cells
            .contains(&(at.x, at.y, at.floor))
            || states.has_pending_spell_commit(actor, session)
            || states.primal_callbacks.steps.len() >= 5
        {
            return;
        }
        let Some(player) = states.get(&runtime, actor, session) else {
            return;
        };
        let maximum_health = player.vitals().max_health;
        let Some(hazard) = runtime.hazard_level(actor, session) else {
            return;
        };
        let Ok(source) =
            crate::durability::creature_source_items::PrimalPodPlayerStep::from_current_player_step(
                &runtime, room, native, actor, session, committed, now,
            )
        else {
            return;
        };
        states
            .primal_callbacks
            .steps
            .push(std::sync::Arc::new(PendingPodStep {
                source,
                actor,
                session,
                maximum_health,
                hazard,
                applied_events: std::sync::atomic::AtomicUsize::new(0),
            }));
    }
    let _ = owner.resolve_primal_pod_steps(permit).await;
}
impl super::ComposedFreshAdmission<'_, '_, '_> {
    /// The existing lane acquisition resolves this retained source before any
    /// subsequent Item writer. Unknown COMMIT keeps the exact pending actor and
    /// source step; it cannot publish a rejection or expose a successor session.
    pub(super) async fn resolve_primal_pod_steps(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
    ) -> bool {
        for _ in 0..5 {
            let pending = {
                self.states
                    .spell_states
                    .lock()
                    .await
                    .primal_callbacks
                    .steps
                    .first()
                    .map(std::sync::Arc::clone)
            };
            let Some(pending) = pending else {
                return true;
            };
            let result = self
                .root
                .commit_primal_pod_step(permit, self.character, self.holder, &pending.source)
                .await;
            let receipt = match result {
                Ok(receipt) => receipt,
                Err(_) => return false,
            };
            self.states.loop_ground.invalidate();
            let runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            let Some(index) = states
                .primal_callbacks
                .steps
                .iter()
                .position(|step| std::sync::Arc::ptr_eq(step, &pending))
            else {
                continue;
            };
            // Removing the pending marker and installing the effect share the
            // same held physical owners; no heal, handoff or successor intervenes.
            states.primal_callbacks.steps.remove(index);
            let Some(receipt) = receipt else {
                continue;
            };
            let step_unix_ms = receipt.step_unix_ms;
            let applied = pending
                .applied_events
                .load(std::sync::atomic::Ordering::Relaxed);
            for (event, timer) in std::iter::once(receipt.source_timer_unix_ms)
                .chain(receipt.additional_source_timers_unix_ms)
                .enumerate()
                .skip(applied)
            {
                use oteryn_gameplay_encounters::bosses::hazard_primal::primal_pod::{
                    PodStep, step_in,
                };
                if let PodStep::DamageAndRemove { life_drain } = step_in(
                    timer,
                    step_unix_ms,
                    u64::from(pending.maximum_health),
                    pending.hazard,
                    true,
                ) {
                    let Ok(damage) = u32::try_from(life_drain) else {
                        states.primal_callbacks.steps.insert(index, pending.clone());
                        return false;
                    };
                    let now = crate::foundation::owner_timer::SemanticTimeMicros::from_micros(
                        self.owner_now().get(),
                    );
                    let hit = states.apply_creature_damage_kind(
                        &runtime,
                        pending.actor,
                        pending.session,
                        damage,
                        crate::player_lethal::PlayerHitKind::LifeDrain,
                        now,
                        oteryn_protocol_oteryn::analyser::AnalyserDamageSource::Creature(None),
                    );
                    if hit.is_none() && !states.is_dead(pending.actor) {
                        states.primal_callbacks.steps.insert(index, pending.clone());
                        return false;
                    }
                    if let Some(room) = self.effective_rooms().gate_field_step_ingress()
                        && let Ok(at) = runtime.read_actor_position(pending.actor)
                    {
                        emit_pod_damage_effects(room, at.position());
                    }
                    self.states.server_messages.send_current(
                        pending.session,
                        oteryn_protocol_oteryn::zones_nologout::ServerMessage::StatusText(
                            "You stepped too late on the primal pod and it explodes.".into(),
                        ),
                    );
                }
                pending
                    .applied_events
                    .store(event + 1, std::sync::atomic::Ordering::Relaxed);
            }
        }
        true
    }
}
fn emit_pod_damage_effects(room: &QualifiedNativeEntryRoom, position: MovementLocalPosition) {
    use crate::content::native_cell_lookup::NativeMovementCollisionIndex;
    use crate::map::boot::BundleCollisionIndexExt;
    if let NativeMovementCollisionIndex::Bundle(collision) = room.movement_cells().index()
        && let (Ok(x), Ok(y), Some(floor)) = (
            u16::try_from(position.x),
            u16::try_from(position.y),
            i8::try_from(position.floor).ok().and_then(i8::checked_neg),
        )
    {
        let at = crate::map::overlay::TilePos { x, y, floor };
        collision.map().facts.emit_movement_effect(at, 12);
        collision.map().facts.emit_movement_effect(at, 1);
    }
}
