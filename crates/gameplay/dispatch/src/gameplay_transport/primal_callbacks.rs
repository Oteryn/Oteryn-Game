//! Source-qualified Primal Menace callbacks under the actual native creature turn.
//! Crystal 201189f the_primal_menace.lua:156-347 and hazard_primal.lua:60-80.
use super::actor_spell::ChannelSpellStates;
use crate::content::{QualifiedNativeEntryRoom, native_gameplay::NativeGameplayState};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, MovementLocalPosition};
use oteryn_gameplay_encounters::bosses::hazard_primal::primal_menace_think::{
    self as primal, PrimalMenaceEffect, PrimalMenaceThinkFacts, PrimalMenaceThinkOwner, WaveKind,
};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

#[cfg(test)]
static NORMAL_MAP_READ_EPOCH: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
#[cfg(test)]
pub(super) fn normal_map_read_epoch_for_test() -> u64 {
    NORMAL_MAP_READ_EPOCH.load(std::sync::atomic::Ordering::Relaxed)
}

mod deferred;
mod normal;
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
    observed_us: u64,
    applied_events: std::sync::atomic::AtomicUsize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PodExecution {
    source: [u8; 16],
    started_us: u64,
    expiry_us: u64,
    created: bool,
}

#[derive(Debug, Default)]
pub(super) struct PrimalCallbacks {
    owner: std::sync::Arc<tokio::sync::Mutex<PrimalCallbackState>>,
    expiries: Vec<crate::durability::creature_source_items::PrimalPodExpiry>,
    expiry_attempted: Vec<[u8; 16]>,
    pod_executions: Vec<PodExecution>,
    normal: Option<std::sync::Arc<normal::PendingNormalPrimal>>,
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

fn source_spawn_plan(
    native: &NativeGameplayState,
    child: &str,
    center: MovementLocalPosition,
    root: &GameplayDecisionRoot,
    creation: u64,
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
    Ok(plan)
}
fn spawn_in_view(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    native: &NativeGameplayState,
    map: &crate::movement::monster_map::MonsterMap,
    child: &str,
    plan: &oteryn_gameplay_world::movement::source_placement::SourcePlacementPlan,
    now: u64,
) -> Result<Option<ExactActorRef>, ()> {
    let at = plan
        .select(
            |at| map.source_placement_cell(runtime, native, child, at, None),
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
        if !map.matches(r) {
            return None;
        }
        map.source_placement_cell(r, native, child, p, None)
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
    #[cfg(test)]
    pub(super) fn pod_execution_deadlines_for_test(&self) -> Vec<([u8; 16], u64, u64, bool)> {
        self.pod_executions
            .iter()
            .map(|execution| {
                (
                    execution.source,
                    execution.started_us,
                    execution.expiry_us,
                    execution.created,
                )
            })
            .collect()
    }
    #[cfg(test)]
    pub(super) fn queued_pod_expiry_sources_for_test(&self) -> Vec<[u8; 16]> {
        self.expiries
            .iter()
            .map(|expiry| expiry.source_transaction_id)
            .collect()
    }
    pub(super) fn has_pending_step(
        &self,
        actor: ExactActorRef,
        session: crate::foundation::GameSessionId,
    ) -> bool {
        self.normal
            .as_ref()
            .is_some_and(|pending| !pending.source_ready())
            || self
                .steps
                .iter()
                .any(|step| step.actor == actor && step.session == session)
    }
    pub(super) fn has_native_work(&self) -> bool {
        self.normal.is_some()
            || !self.expiries.is_empty()
            || !self.pod_executions.is_empty()
            || !self.pods.is_empty()
            || !self.steps.is_empty()
            || self
                .owner
                .try_lock()
                .map_or(true, |owner| !owner.deferred.0.is_empty())
    }
    pub(super) fn has_native_due(&self, now: u64) -> bool {
        self.normal.is_some()
            || !self.expiries.is_empty()
            || !self.steps.is_empty()
            || self
                .pod_executions
                .iter()
                .any(|execution| execution.created && execution.expiry_us <= now)
            || self.pods.iter().any(|pod| pod.due_us() <= now)
            || self.owner.try_lock().map_or(true, |owner| {
                owner
                    .deferred
                    .next_due()
                    .is_some_and(|due| due.saturating_mul(1_000) <= now)
            })
    }
    pub(super) fn next_due_us(&self) -> Option<u64> {
        if self.normal.is_some() {
            return Some(0);
        }
        self.next_retained_due_us()
    }
    /// Original deadlines for source ordering; a locked owner is unavailable,
    /// while the ordinary scheduler keeps its separate immediate retry hint.
    pub(super) fn next_source_due_time(&self) -> Result<Option<u64>, ()> {
        let owner = self.owner.try_lock().map_err(|_| ())?;
        let deferred = owner
            .deferred
            .original_next_due()
            .map(|due| due.checked_mul(1_000).ok_or(()))
            .transpose()?;
        for expiry in &self.expiries {
            if !self.pod_executions.iter().any(|execution| {
                execution.source == expiry.source_transaction_id && execution.created
            }) {
                return Err(());
            }
        }
        Ok(deferred
            .into_iter()
            .chain(
                self.normal
                    .as_ref()
                    .filter(|pending| !pending.source_ready())
                    .map(|pending| pending.batch().observed_at_us()),
            )
            .chain(self.steps.iter().map(|step| step.observed_us))
            .chain(self.pods.iter().map(|pod| pod.due_us()))
            .chain(
                self.pod_executions
                    .iter()
                    .filter(|execution| execution.created)
                    .map(|execution| execution.expiry_us),
            )
            .min())
    }
    pub(in crate::gameplay_transport) fn next_retained_due_us(&self) -> Option<u64> {
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
            .chain(
                self.pod_executions
                    .iter()
                    .filter(|execution| execution.created)
                    .map(|execution| execution.expiry_us),
            )
            .min()
    }
}

fn selected_primal_source_due(
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    native: &NativeGameplayState,
    now: u64,
) -> Result<Option<u64>, ()> {
    if let Some(pending) = states.primal_callbacks.normal_pending() {
        if !pending.batch().current_for(runtime) {
            return Err(());
        }
        // This is the continuation of an actual reserved Think. Its existing
        // source preparation must finish before standalone callbacks can run.
        if !pending.source_ready() {
            return Ok(None);
        }
    }
    let Some(due) = states.primal_callbacks.next_source_due_time()? else {
        return Ok(None);
    };
    if due > now
        || super::source_death::source_work_cutoff(runtime, states)?
            .is_some_and(|cutoff| due > cutoff)
        || super::source_death::next_source_work(runtime, states, native)?
            != Some((super::source_death::SourceWorkFamily::Detached, due))
    {
        return Ok(None);
    }
    Ok(Some(due))
}

impl super::ComposedFreshAdmission<'_, '_, '_> {
    /// Run only an actual retained Primal deadline selected by the shared
    /// source arbiter. A fresh turn needs no synthetic native Think reservation.
    pub(in crate::gameplay_transport) async fn drain_selected_primal_source_work(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        now: u64,
    ) -> Result<bool, ()> {
        let room = self.rooms().gate_monster_ai().ok_or(())?;
        if room.pinned_activity_source_room()
            != Some(crate::content::bosses::activity_template::PinnedActivitySourceRoom::Primal)
        {
            return Ok(false);
        }
        let native = self
            .active_generation
            .and_then(|active| active.native_gameplay())
            .ok_or(())?;
        let (due, step) = {
            let runtime = self.runtime.lock().await;
            runtime.owner_fence().map_err(|_| ())?;
            if native.source_digest() != runtime.content_pin().server_artifact_digest()
                || room.compiled().server_digest() != native.source_digest()
                || !native.qualifies_primal_callbacks(native.source_digest())
            {
                return Err(());
            }
            let states = self.states.spell_states.try_lock().map_err(|_| ())?;
            if states.source_death_action.is_some()
                || self
                    .states
                    .attack
                    .try_lock()
                    .map_err(|_| ())?
                    .kills()
                    .core_head()?
                    .is_some()
            {
                return Ok(false);
            }
            let Some(due) = selected_primal_source_due(&runtime, &states, native, now)? else {
                return Ok(false);
            };
            let step = states
                .primal_callbacks
                .steps
                .iter()
                .find(|step| step.observed_us == due)
                .map(std::sync::Arc::clone);
            (due, step)
        };
        if let Some(step) = step {
            self.resolve_one_primal_pod_step(permit, &step, now, Some(due))
                .await?;
        } else {
            self.drain_primal_retained_through(permit, None, due, now)
                .await?;
        }
        Ok(true)
    }

    /// A sealed immutable callback crosses SQL. Native runtime/spell guards are
    /// released before the writer independently proves the current Node/scope.
    #[cfg(test)]
    pub(super) async fn drain_primal_pod_callbacks(&self) {
        self.drain_primal_pod_callbacks_limited(4).await;
    }
    pub(super) async fn drain_primal_pod_callback_unit(&self) {
        self.drain_primal_pod_callbacks_limited(1).await;
    }
    async fn drain_primal_pod_callbacks_limited(&self, limit: usize) {
        let cutoff = match self.states.attack.lock().await.kills().core_head() {
            Ok(Some((_, _, Some(clock)))) => clock.get(),
            Ok(None) => u64::MAX,
            Ok(Some((_, _, None))) | Err(()) => return,
        };
        let Some(permit) = self.spell_lane_permit().await else {
            return;
        };
        for _ in 0..limit {
            if self
                .drain_primal_pod_creates_with_permit(&permit, cutoff, self.owner_now().get())
                .await
                .is_err()
            {
                return;
            }
        }
    }
    #[cfg(test)]
    pub(super) async fn drain_primal_pod_creates_through(&self, cutoff: u64) -> Result<(), ()> {
        let permit = self.spell_lane_permit().await.ok_or(())?;
        self.drain_primal_pod_creates_with_permit(&permit, cutoff, self.owner_now().get())
            .await
    }
    async fn drain_primal_pod_creates_with_permit(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        cutoff: u64,
        now: u64,
    ) -> Result<(), ()> {
        for _ in 0..1 {
            let pending = {
                let runtime = self.runtime.lock().await;
                let mut states = self.states.spell_states.lock().await;
                if runtime.owner_fence().is_err() {
                    return Err(());
                }
                let pending = states
                    .primal_callbacks
                    .pods
                    .iter()
                    .filter(|pod| pod.due_us() <= now.min(cutoff))
                    .min_by_key(|pod| pod.due_us())
                    .map(std::sync::Arc::clone);
                if let Some(pending) = &pending {
                    let source = pending.source_transaction_id();
                    let state = &mut states.primal_callbacks;
                    if !state
                        .pod_executions
                        .iter()
                        .any(|execution| execution.source == source)
                    {
                        if state.pod_executions.len() >= 512 {
                            return Err(());
                        }
                        state.pod_executions.try_reserve(1).map_err(|_| ())?;
                        state
                            .expiries
                            .try_reserve(4_usize.saturating_sub(state.expiries.len()))
                            .map_err(|_| ())?;
                        state
                            .expiry_attempted
                            .try_reserve(512_usize.saturating_sub(state.expiry_attempted.len()))
                            .map_err(|_| ())?;
                        // First ACTUAL create invocation, before uncertain SQL.
                        // This is native timer DATA, never a caller SQL epoch.
                        state.pod_executions.push(PodExecution {
                            source,
                            started_us: now,
                            expiry_us: now.checked_add(4_000_000).ok_or(())?,
                            created: false,
                        });
                    }
                }
                pending
            };
            let Some(pending) = pending else {
                return Ok(());
            };
            let result = self
                .root
                .commit_primal_pod_callback(permit, self.character, self.holder, &pending, now)
                .await;
            if let Err(error) = result {
                #[cfg(test)]
                eprintln!("SRX3_PRIMAL_ACTUAL_POD_WRITER_REFUSAL {error:?}");
                #[cfg(not(test))]
                let _ = error;
                return Err(());
            }
            self.states.loop_ground.invalidate();
            let mut states = self.states.spell_states.lock().await;
            states
                .primal_callbacks
                .pod_executions
                .iter_mut()
                .find(|execution| execution.source == pending.source_transaction_id())
                .ok_or(())?
                .created = true;
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
        Ok(())
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
        let cutoff = match self.states.attack.lock().await.kills().core_head() {
            Ok(Some((_, _, Some(clock)))) => clock.get(),
            Ok(None) => u64::MAX,
            _ => return,
        };
        let _ = self.drain_primal_expiry_through(cutoff).await;
    }
    async fn drain_primal_expiry_through(&self, cutoff: u64) -> Result<(), ()> {
        let permit = self.spell_lane_permit().await.ok_or(())?;
        self.drain_primal_expiry_with_permit(&permit, cutoff, self.owner_now().get())
            .await
    }
    async fn drain_primal_expiry_with_permit(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        cutoff: u64,
        now: u64,
    ) -> Result<(), ()> {
        let selected = {
            let states = self.states.spell_states.lock().await;
            states
                .primal_callbacks
                .pod_executions
                .iter()
                .filter(|execution| execution.created && execution.expiry_us <= now.min(cutoff))
                .min_by_key(|execution| (execution.expiry_us, execution.source))
                .copied()
        };
        let Some(selected) = selected else {
            return Ok(());
        };
        let (scope, generation, attempted) = {
            let runtime = self.runtime.lock().await;
            runtime.owner_fence().map_err(|_| ())?;
            let states = self.states.spell_states.lock().await;
            (
                runtime.binding().runtime_scope(),
                runtime.binding().scope_generation().get(),
                states
                    .primal_callbacks
                    .expiry_attempted
                    .contains(&selected.source),
            )
        };
        if attempted {
            self.root
                .acknowledge_primal_pod_expiry(
                    permit,
                    self.character,
                    self.holder,
                    scope,
                    generation,
                    selected.source,
                )
                .await
                .map_err(|_| ())?;
            let mut states = self.states.spell_states.lock().await;
            states
                .primal_callbacks
                .expiry_attempted
                .retain(|source| *source != selected.source);
            states
                .primal_callbacks
                .pod_executions
                .retain(|execution| *execution != selected);
            return Ok(());
        }
        {
            let states = self.states.spell_states.lock().await;
            if states
                .primal_callbacks
                .expiries
                .iter()
                .any(|expiry| expiry.source_transaction_id == selected.source)
            {
                return Ok(());
            }
        }
        // Native original eligibility AND unchanged independent SQL due gate.
        let expired = self
            .root
            .drain_primal_pod_expiry_for_source(
                permit,
                self.character,
                self.holder,
                scope,
                generation,
                selected.source,
                self.active_generation
                    .and_then(|generation| generation.native_gameplay())
                    .and_then(|native| native.item_profile_source())
                    .ok_or(())?,
            )
            .await
            .map_err(|_| ())?
            .ok_or(())?;
        self.states.loop_ground.invalidate();
        let mut states = self.states.spell_states.lock().await;
        if !states.primal_callbacks.pod_executions.contains(&selected) {
            return Err(());
        }
        if !expired.requires_spawn {
            states
                .primal_callbacks
                .pod_executions
                .retain(|execution| *execution != selected);
            return Ok(());
        }
        if !states
            .primal_callbacks
            .expiries
            .iter()
            .any(|expiry| expiry.source_transaction_id == selected.source)
        {
            if states.primal_callbacks.expiries.len() >= 4 {
                return Err(());
            }
            states.primal_callbacks.expiries.push(expired);
        }
        states.owner_wake.notify_one();
        Ok(())
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
                observed_us: now,
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
            if self
                .resolve_one_primal_pod_step(permit, &pending, self.owner_now().get(), None)
                .await
                .is_err()
            {
                return false;
            }
        }
        true
    }

    async fn resolve_one_primal_pod_step(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        pending: &std::sync::Arc<PendingPodStep>,
        execution_now: u64,
        selected_due: Option<u64>,
    ) -> Result<(), ()> {
        let receipt = self
            .root
            .commit_primal_pod_step(permit, self.character, self.holder, &pending.source)
            .await
            .map_err(|_| ())?;
        self.states.loop_ground.invalidate();
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        if let Some(due) = selected_due {
            runtime.owner_fence().map_err(|_| ())?;
            let native = self
                .active_generation
                .and_then(|active| active.native_gameplay())
                .ok_or(())?;
            if states.source_death_action.is_some()
                || self
                    .states
                    .attack
                    .try_lock()
                    .map_err(|_| ())?
                    .kills()
                    .core_head()?
                    .is_some()
                || selected_primal_source_due(&runtime, &states, native, execution_now)?
                    != Some(due)
            {
                return Err(());
            }
        }
        let Some(index) = states
            .primal_callbacks
            .steps
            .iter()
            .position(|step| std::sync::Arc::ptr_eq(step, pending))
        else {
            return Ok(());
        };
        // Removing the pending marker and installing the effect share the
        // same held physical owners; no heal, handoff or successor intervenes.
        states.primal_callbacks.steps.remove(index);
        let Some(receipt) = receipt else {
            return Ok(());
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
            use oteryn_gameplay_encounters::bosses::hazard_primal::primal_pod::{PodStep, step_in};
            if let PodStep::DamageAndRemove { life_drain } = step_in(
                timer,
                step_unix_ms,
                u64::from(pending.maximum_health),
                pending.hazard,
                true,
            ) {
                let Ok(damage) = u32::try_from(life_drain) else {
                    states
                        .primal_callbacks
                        .steps
                        .insert(index, std::sync::Arc::clone(pending));
                    return Err(());
                };
                let now =
                    crate::foundation::owner_timer::SemanticTimeMicros::from_micros(execution_now);
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
                    states
                        .primal_callbacks
                        .steps
                        .insert(index, std::sync::Arc::clone(pending));
                    return Err(());
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
        Ok(())
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

impl super::ComposedFreshAdmission<'_, '_, '_> {
    /// One retained original source deadline, before this exact core prefix.
    pub(super) async fn drain_primal_core_prefix(
        &self,
        target: ExactActorRef,
        cutoff: u64,
    ) -> Result<(), ()> {
        let permit = self.spell_lane_permit().await.ok_or(())?;
        self.drain_primal_core_prefix_with_permit(&permit, target, cutoff)
            .await
    }
    pub(in crate::gameplay_transport) async fn drain_primal_core_prefix_with_permit(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        target: ExactActorRef,
        cutoff: u64,
    ) -> Result<(), ()> {
        let expected = self
            .states
            .attack
            .lock()
            .await
            .kills()
            .core_head()?
            .ok_or(())?;
        if expected.0 != target || expected.2.map(|clock| clock.get()) != Some(cutoff) {
            return Err(());
        }
        if self
            .drain_primal_retained_through(permit, Some(expected), cutoff, self.owner_now().get())
            .await?
        {
            Ok(())
        } else {
            Err(())
        }
    }
    async fn drain_primal_retained_through(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        expected_head: Option<(
            ExactActorRef,
            crate::foundation::CreatureDeathOccurrenceKey,
            Option<oteryn_simulation_determinism::SemanticTimeMicros>,
        )>,
        cutoff: u64,
        now: u64,
    ) -> Result<bool, ()> {
        let room = self.rooms().gate_monster_ai().ok_or(())?;
        if room.pinned_activity_source_room()
            != Some(crate::content::bosses::activity_template::PinnedActivitySourceRoom::Primal)
        {
            return Ok(true);
        }
        let native = self
            .active_generation
            .and_then(|active| active.native_gameplay())
            .ok_or(())?;
        if self.states.attack.lock().await.kills().core_head()? != expected_head {
            return Err(());
        }
        let (callback, boss, child, plan, removed, map_plan, scope, generation) = {
            let runtime = self.runtime.lock().await;
            runtime.owner_fence().map_err(|_| ())?;
            if native.source_digest() != runtime.content_pin().server_artifact_digest()
                || room.compiled().server_digest() != native.source_digest()
                || !native.qualifies_primal_callbacks(native.source_digest())
            {
                return Err(());
            }
            let states = self.states.spell_states.try_lock().map_err(|_| ())?;
            let mut owner = states.primal_callbacks.owner.try_lock().map_err(|_| ())?;
            let deferred_due = owner
                .deferred
                .original_next_due()
                .map(|due| due.saturating_mul(1000));
            let pod_due = states
                .primal_callbacks
                .pods
                .iter()
                .map(|pod| pod.due_us())
                .min();
            let expiry = states
                .primal_callbacks
                .pod_executions
                .iter()
                .filter(|execution| execution.created && execution.expiry_us <= now.min(cutoff))
                .min_by_key(|execution| (execution.expiry_us, execution.source))
                .copied();
            if let Some(expiry) = expiry
                && deferred_due.is_none_or(|due| expiry.expiry_us <= due)
                && pod_due.is_none_or(|due| expiry.expiry_us <= due)
            {
                drop(owner);
                drop(states);
                drop(runtime);
                self.drain_primal_expiry_with_permit(permit, expiry.expiry_us, now)
                    .await?;
                self.publish_primal_expiry_native_with_permit(permit, expiry.source, now)
                    .await?;
                return Ok(false);
            }
            if pod_due.is_some_and(|due| {
                due <= now.min(cutoff) && deferred_due.is_none_or(|other| due <= other)
            }) {
                let due = pod_due.ok_or(())?;
                drop(owner);
                drop(states);
                drop(runtime);
                self.drain_primal_pod_creates_with_permit(permit, due, now)
                    .await?;
                return Ok(false); // re-read exact core head and all original deadlines
            }
            // Known default owner before the first real Think has no retained
            // source work. This is distinct from an unavailable owner lease.
            let Some(boss) = owner.boss else {
                return if owner.deferred.0.is_empty() && states.primal_callbacks.pods.is_empty() {
                    Ok(true)
                } else {
                    Err(())
                };
            };
            if owner.content != Some(native.source_digest()) {
                return Err(());
            }
            let registered = primal_parent_registered(&runtime, boss)?;
            let Some(callback) =
                owner
                    .deferred
                    .peek_due_registered_through(now / 1000, cutoff / 1000, registered)
            else {
                return if states
                    .primal_callbacks
                    .pods
                    .iter()
                    .any(|pod| pod.due_us() <= now.min(cutoff))
                    || states.primal_callbacks.expiries.iter().any(|expiry| {
                        !states
                            .primal_callbacks
                            .pod_executions
                            .iter()
                            .any(|execution| execution.source == expiry.source_transaction_id)
                    }) {
                    Err(())
                } else {
                    Ok(true)
                };
            };
            if let PrimalMenaceEffect::SpawnSparkle { position, .. } = callback.effect {
                if self
                    .states
                    .attack
                    .try_lock()
                    .map_err(|_| ())?
                    .kills()
                    .core_head()?
                    != expected_head
                {
                    return Err(());
                }
                emit_sparkle(room, position);
                owner.deferred.acknowledge(callback.id)?;
                return Ok(false); // one deadline, then re-evaluate the canonical prefix
            }
            let root = GameplayDecisionRoot::from_bytes(
                Sha256::new()
                    .chain_update(b"srx3:primal:callbacks:v1")
                    .chain_update(native.source_digest())
                    .chain_update(boss.placement_identity())
                    .finalize()
                    .into(),
            );
            let (child, position, removed) = match callback.effect {
                PrimalMenaceEffect::ScheduledSpawn {
                    kind: WaveKind::Beast,
                    position,
                    ..
                } => {
                    let mut draw_ordinal = 0;
                    (
                        primal::pick_beast(draw(&root, callback.id, &mut draw_ordinal)).ok_or(())?,
                        position,
                        None,
                    )
                }
                PrimalMenaceEffect::ReplaceWithFungosaurus { beast, position } => {
                    let removed = if callback.replacement_removed {
                        None
                    } else {
                        owner
                            .beasts
                            .iter()
                            .find(|(id, _)| *id == beast)
                            .map(|(_, actor)| *actor)
                    };
                    if !callback.replacement_removed
                        && removed.is_none_or(|actor| !runtime.contains_live_creature(actor))
                    {
                        owner.deferred.acknowledge(callback.id)?;
                        return Ok(false);
                    }
                    (primal::FUNGOSAURUS_KEY, position, removed)
                }
                _ => return Err(()),
            };
            let plan = source_spawn_plan(native, child, position, &root, callback.id)?;
            let objects = self.door.try_lock().map_err(|_| ())?;
            let map_plan = crate::movement::monster_map::PreparedMonsterMapRead::prepare(
                &runtime,
                room,
                &objects,
                plan.candidates(),
            )
            .ok_or(())?;
            (
                callback,
                boss,
                child,
                plan,
                removed,
                map_plan,
                runtime.binding().runtime_scope(),
                runtime.binding().scope_generation().get(),
            )
        };
        let mut context = (self, &permit, map_plan, room, native);
        self.root
            .try_issue_semantic_pass()
            .map_err(|_| ())?
            .run_with_context(&mut context, move |holder, deadline, context| {
                Box::pin(async move {
                    use crate::durability::spell_item_transaction::{
                        assert_spell_item_scope_in_transaction, begin_spell_owner_transaction,
                    };
                    let (admission, permit, map_plan, room, native) = context;
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
                    let read = map_plan
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
                        let view = read
                            .into_current_view(&runtime, room, &objects, native)
                            .ok_or(crate::durability::DurabilityError::Unavailable)?;
                        let retained = states.primal_callbacks.retained_owner();
                        let mut owner = retained
                            .try_lock()
                            .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                        if admission
                            .states
                            .attack
                            .try_lock()
                            .map_err(|_| crate::durability::DurabilityError::Unavailable)?
                            .kills()
                            .core_head()
                            .map_err(|_| crate::durability::DurabilityError::Unavailable)?
                            != expected_head
                            || owner.boss != Some(boss)
                            || owner.content != Some(native.source_digest())
                            || owner
                                .deferred
                                .peek_due_through(now / 1000, cutoff / 1000)
                                .map_err(|_| crate::durability::DurabilityError::Unavailable)?
                                != Some(callback)
                        {
                            return Err(crate::durability::DurabilityError::Unavailable);
                        }
                        let result: Result<(), ()> = (|| {
                            if !primal_parent_registered(&runtime, boss)?
                                && matches!(
                                    callback.effect,
                                    PrimalMenaceEffect::ScheduledSpawn {
                                        kind: WaveKind::Beast,
                                        ..
                                    }
                                )
                            {
                                return owner.deferred.acknowledge(callback.id);
                            }
                            // Qualify complete candidate facts and current occupancy before
                            // source remove; no owner guard crosses a SQL await below.
                            plan.select(
                                |at| {
                                    view.source_placement_cell(&runtime, native, child, at, removed)
                                },
                                |_, _| Some(true),
                            )
                            .map_err(|_| ())?;
                            owner.beasts.try_reserve(1).map_err(|_| ())?;
                            if let Some(actor) = removed {
                                runtime.remove_arena_opponent(actor).map_err(|_| ())?;
                                owner.beasts.retain(|(_, old)| *old != actor);
                                owner
                                    .deferred
                                    .0
                                    .iter_mut()
                                    .find(|work| work.id == callback.id)
                                    .ok_or(())?
                                    .replacement_removed = true;
                            }
                            if let Some(actor) = spawn_in_view(
                                &mut runtime,
                                &mut states,
                                native,
                                &view,
                                child,
                                &plan,
                                now,
                            )? && matches!(
                                callback.effect,
                                PrimalMenaceEffect::ScheduledSpawn {
                                    kind: WaveKind::Beast,
                                    ..
                                }
                            ) {
                                owner.next_id = owner.next_id.checked_add(1).ok_or(())?;
                                let id = owner.next_id;
                                owner.beasts.push((id, actor));
                                owner.think.record_beast(id, (now / 1_000_000) * 1000);
                            }
                            owner.deferred.acknowledge(callback.id)
                        })();
                        result.map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                    }
                    tx.rollback()
                        .await
                        .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                    Ok(())
                })
            })
            .await
            .map_err(|_| ())?;
        Ok(false) // re-evaluate before any queue take or next original deadline
    }
}

impl super::ComposedFreshAdmission<'_, '_, '_> {
    async fn publish_primal_expiry_native_with_permit(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        source: [u8; 16],
        now: u64,
    ) -> Result<(), ()> {
        let room = self.rooms().gate_monster_ai().ok_or(())?;
        let native = self
            .active_generation
            .and_then(|active| active.native_gameplay())
            .ok_or(())?;
        let (expiry, execution, map_plan, scope, generation) = {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.try_lock().map_err(|_| ())?;
            if states.primal_callbacks.expiry_attempted.contains(&source) {
                return Ok(());
            }
            let Some(expiry) = states
                .primal_callbacks
                .expiries
                .iter()
                .find(|expiry| expiry.source_transaction_id == source)
                .copied()
            else {
                return Ok(());
            };
            let execution = states
                .primal_callbacks
                .pod_executions
                .iter()
                .find(|execution| execution.source == source)
                .copied()
                .ok_or(())?;
            if !execution.created || execution.expiry_us > now {
                return Err(());
            }
            let objects = self.door.try_lock().map_err(|_| ())?;
            let plan = crate::movement::monster_map::PreparedMonsterMapRead::prepare(
                &runtime,
                room,
                &objects,
                &[expiry.position],
            )
            .ok_or(())?;
            (
                expiry,
                execution,
                plan,
                runtime.binding().runtime_scope(),
                runtime.binding().scope_generation().get(),
            )
        };
        let mut context = (self, &permit, map_plan, room, native);
        self.root
            .try_issue_semantic_pass()
            .map_err(|_| ())?
            .run_with_context(&mut context, move |holder, deadline, context| {
                Box::pin(async move {
                    use crate::durability::spell_item_transaction::{
                        assert_spell_item_scope_in_transaction, begin_spell_owner_transaction,
                    };
                    let (owner, permit, map_plan, room, native) = context;
                    let mut tx = begin_spell_owner_transaction(holder, deadline).await?;
                    let authority = assert_spell_item_scope_in_transaction(
                        &mut tx,
                        permit,
                        owner.root,
                        owner.character,
                        owner.holder,
                        scope,
                        generation,
                    )
                    .await
                    .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                    let read = map_plan
                        .read(&mut tx, &authority)
                        .await
                        .ok_or(crate::durability::DurabilityError::Unavailable)?;
                    {
                        let mut runtime = owner.runtime.lock().await;
                        let mut states = owner
                            .states
                            .spell_states
                            .try_lock()
                            .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                        let objects = owner
                            .door
                            .try_lock()
                            .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                        let view = read
                            .into_current_view(&runtime, room, &objects, native)
                            .ok_or(crate::durability::DurabilityError::Unavailable)?;
                        if !states.primal_callbacks.pod_executions.contains(&execution)
                            || !states
                                .primal_callbacks
                                .expiries
                                .iter()
                                .any(|pending| pending.source_transaction_id == source)
                            || states.primal_callbacks.expiry_attempted.contains(&source)
                        {
                            return Err(crate::durability::DurabilityError::Unavailable);
                        }
                        // A concurrently earlier core prefix cannot authorize this later
                        // expiry merely because its SQL due condition is already true.
                        if let Some((_, _, clock)) = owner
                            .states
                            .attack
                            .try_lock()
                            .map_err(|_| crate::durability::DurabilityError::Unavailable)?
                            .kills()
                            .core_head()
                            .map_err(|_| crate::durability::DurabilityError::Unavailable)?
                            && clock.is_none_or(|clock| execution.expiry_us > clock.get())
                        {
                            return Err(crate::durability::DurabilityError::Unavailable);
                        }
                        let record = native
                            .creature_profiles()
                            .records
                            .iter()
                            .find(|record| record.profile.target.key == primal::FUNGOSAURUS_KEY)
                            .ok_or(crate::durability::DurabilityError::Unavailable)?;
                        let exists = view
                            .source_placement_cell(
                                &runtime,
                                native,
                                primal::FUNGOSAURUS_KEY,
                                expiry.position,
                                None,
                            )
                            .ok_or(crate::durability::DurabilityError::Unavailable)?
                            .exists;
                        if !exists {
                            return Err(crate::durability::DurabilityError::Unavailable);
                        }
                        if let Ok(actor) = runtime
                            .spawn_primal_pod_fungosaurus(expiry.position, |runtime, _| {
                                view.matches(runtime).then_some(exists)
                            })
                        {
                            if runtime
                                .install_creature_policy(actor, primal::FUNGOSAURUS_KEY)
                                .is_err()
                                || states
                                    .monster_melee
                                    .creature_ai
                                    .admit_native_creature(
                                        &runtime,
                                        native,
                                        actor,
                                        &record.profile.target,
                                        now,
                                    )
                                    .is_err()
                            {
                                runtime
                                    .remove_arena_opponent(actor)
                                    .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                                return Err(crate::durability::DurabilityError::Unavailable);
                            }
                            emit_pod_speech(&runtime, room, actor);
                        }
                        states.primal_callbacks.expiry_attempted.push(source);
                        states
                            .primal_callbacks
                            .expiries
                            .retain(|pending| pending.source_transaction_id != source);
                    }
                    tx.rollback()
                        .await
                        .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                    Ok(())
                })
            })
            .await
            .map_err(|_| ())
    }
}

fn primal_parent_registered(runtime: &ChannelRuntimeV1, boss: ExactActorRef) -> Result<bool, ()> {
    runtime.owner_fence().map_err(|_| ())?;
    if boss.runtime_scope() != runtime.binding().runtime_scope()
        || boss.scope_generation() != runtime.binding().scope_generation()
    {
        return Err(());
    }
    match runtime.companion_snapshot_including_dead(boss) {
        Ok(_) => runtime.source_creature_is_registered(boss).map_err(|_| ()),
        Err(
            oteryn_gameplay_domain::domain::carrier_values::CarrierError::StaleActorGeneration
            | oteryn_gameplay_domain::domain::carrier_values::CarrierError::NotCreature,
        ) => Ok(false),
        Err(_) => Err(()),
    }
}
