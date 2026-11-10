//! The registered source GlobalEvent belongs to the actual scope owner, not a
//! boss or a Player. Deadlines use its monotonic execution clock, never SQL UTC.
use super::ComposedFreshAdmission;
use crate::content::native_soul_war_global_events::QualifiedSpiteGlobalEvent;
use crate::content::{QualifiedNativeEntryRoom, native_gameplay::NativeGameplayState};
use crate::durability::creature_source_items::{
    PreparedSpiteGlobalWrite, SpiteGlobalFireCallback, SpiteGlobalFireOutcome,
};
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::{ChannelRuntimeV1, RuntimeScopeRefV1};
use oteryn_gameplay_entities::{CompanionSnapshot, SourceActorReservation};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};
use std::sync::Arc;

#[derive(Debug, Default)]
pub(super) struct SpiteGlobalState {
    registration: Option<Registration>,
}
#[derive(Debug)]
struct Registration {
    proof: QualifiedSpiteGlobalEvent,
    scope: RuntimeScopeRefV1,
    generation: u64,
    pins: [[u8; 32]; 4],
    activation: u64,
    registered_us: u64,
    next_us: u64,
    tick: u64,
    expiry: Option<Arc<SpiteGlobalFireCallback>>,
    pending: Option<Pending>,
}
#[derive(Debug, Clone)]
struct Pending {
    callback: Arc<SpiteGlobalFireCallback>,
    prepared: Arc<PreparedSpiteGlobalWrite>,
    due_us: u64,
    next_create: Option<(u64, u64)>,
    expiry_target: Option<Arc<ExpiryTarget>>,
}
#[derive(Debug)]
struct ExpiryTarget {
    reservation: SourceActorReservation,
    target: Option<(CompanionSnapshot, u32)>,
}
impl Pending {
    fn current_for(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.callback.current_for(runtime)
            && self.prepared.current_for(runtime)
            && self.expiry_target.as_ref().is_none_or(|target| {
                runtime
                    .validate_source_global_action(&target.reservation, self.prepared.as_ref())
                    .is_ok()
            })
    }
}
impl Registration {
    fn current_for(&self, runtime: &ChannelRuntimeV1) -> bool {
        let pin = runtime.content_pin();
        runtime.owner_fence().is_ok()
            && self.scope == runtime.binding().runtime_scope()
            && self.generation == runtime.binding().scope_generation().get()
            && self.activation == pin.activation_sequence()
            && self.proof.content() == pin.server_artifact_digest()
            && self.pins
                == [
                    pin.server_artifact_digest(),
                    pin.client_artifact_digest(),
                    pin.frame_binding_digest(),
                    runtime.map_revision_digest(),
                ]
    }
}
impl Registration {
    fn next_due_us(&self) -> u64 {
        if let Some(pending) = &self.pending {
            return pending.due_us;
        }
        self.expiry
            .as_ref()
            .map_or(self.next_us, |expiry| expiry.due_us().min(self.next_us))
    }
    fn capture_due(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        now_us: u64,
    ) -> Result<Pending, ()> {
        if let Some(pending) = &self.pending {
            return pending
                .current_for(runtime)
                .then(|| pending.clone())
                .ok_or(());
        }
        let (callback, due_us, next_create) = if let Some(expiry) = &self.expiry
            && expiry.due_us() <= self.next_us
        {
            (
                expiry
                    .prepare_current_expiry(runtime, room, now_us)
                    .map_err(|_| ())?,
                expiry.due_us(),
                None,
            )
        } else {
            // A successful creation's five-second callback always precedes its
            // next fourteen-second occurrence. Never discard that continuation.
            if self.expiry.is_some() {
                return Err(());
            }
            let next_us = now_us
                .checked_add(self.proof.interval_ms().checked_mul(1_000).ok_or(())?)
                .ok_or(())?;
            let next_tick = self.tick.checked_add(1).ok_or(())?;
            let index = self.selected_anchor(runtime)?;
            (
                SpiteGlobalFireCallback::from_current_registration(
                    runtime,
                    room,
                    native,
                    &self.proof,
                    self.tick,
                    index,
                    now_us,
                )
                .map_err(|_| ())?,
                self.next_us,
                Some((next_us, next_tick)),
            )
        };
        let prepared = Arc::new(callback.prepare_current_write(runtime).map_err(|_| ())?);
        let pending = Pending {
            callback: Arc::new(callback),
            prepared,
            due_us,
            next_create,
            expiry_target: None,
        };
        self.pending = Some(pending.clone());
        Ok(pending)
    }
    fn selected_anchor(&self, runtime: &ChannelRuntimeV1) -> Result<u8, ()> {
        let binding = runtime.binding();
        let scope = match binding.runtime_scope() {
            RuntimeScopeRefV1::Channel { channel_id, .. } => *channel_id.as_bytes(),
            RuntimeScopeRefV1::Instance { instance_id, .. } => instance_id,
        };
        let mut digest = Sha256::new();
        digest.update(b"oteryn:registered-spite-global-selection:v1");
        digest.update(binding.world_id().as_bytes());
        digest.update(scope);
        digest.update(self.generation.to_be_bytes());
        digest.update(self.activation.to_be_bytes());
        digest.update(self.registered_us.to_be_bytes());
        digest.update(self.proof.body_digest());
        for pin in self.pins {
            digest.update(pin);
        }
        let root = GameplayDecisionRoot::from_bytes(digest.finalize().into());
        let mut occurrence = [0; 16];
        occurrence[..8].copy_from_slice(&self.registered_us.to_be_bytes());
        occurrence[8..].copy_from_slice(&self.tick.to_be_bytes());
        let draw = deterministic_decision_u64(
            &root,
            DecisionOccurrenceId::from_bytes(occurrence),
            "spite_registered_global_anchor",
            0,
        )
        .map_err(|_| ())?;
        // Four divides the complete u64 range, so every literal anchor has the
        // same probability without a retry budget or a biased fallback.
        Ok((draw % 4) as u8)
    }
}

fn source_cutoff(runtime: &ChannelRuntimeV1, now_us: u64) -> Result<u64, ()> {
    Ok(
        match runtime.next_source_creature_death().map_err(|_| ())? {
            Some(notice) => notice.enqueued_at_us().ok_or(())?.min(now_us),
            None => now_us,
        },
    )
}

impl SpiteGlobalState {
    pub(super) fn next_due_us(&self) -> Option<u64> {
        self.registration.as_ref().map(Registration::next_due_us)
    }
    /// A source Instance owns the same registered event in its independent
    /// physical scope. Unlike XML clearing, this clock does not wait for a lever.
    pub(super) fn register_source_instance(
        &mut self,
        runtime: &ChannelRuntimeV1,
        native: &NativeGameplayState,
        room: &QualifiedNativeEntryRoom,
        registered_us: u64,
    ) -> Result<(), ()> {
        use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
        if room.pinned_activity_source_room() != Some(PinnedActivitySourceRoom::GoshnarsSpite) {
            return Ok(());
        }
        let pin = runtime.content_pin();
        if !matches!(
            runtime.binding().runtime_scope(),
            RuntimeScopeRefV1::Instance { .. }
        ) || runtime.owner_fence().is_err()
            || pin.server_artifact_digest() != native.source_digest()
            || room.compiled().server_digest() != pin.server_artifact_digest()
            || room.compiled().client_digest() != pin.client_artifact_digest()
            || room.frame_binding().digest() != pin.frame_binding_digest()
            || room.map_revision_digest() != runtime.map_revision_digest()
        {
            return Err(());
        }
        let proof = native.qualify_spite_global_event(room).ok_or(())?;
        self.register(runtime, proof, registered_us)
    }
    fn register(
        &mut self,
        runtime: &ChannelRuntimeV1,
        proof: QualifiedSpiteGlobalEvent,
        registered_us: u64,
    ) -> Result<(), ()> {
        if let Some(old) = &self.registration {
            return (old.current_for(runtime)
                && old.proof == proof
                && old.registered_us == registered_us)
                .then_some(())
                .ok_or(());
        }
        let pin = runtime.content_pin();
        let next_us = registered_us
            .checked_add(proof.interval_ms().checked_mul(1_000).ok_or(())?)
            .ok_or(())?;
        self.registration = Some(Registration {
            proof,
            scope: runtime.binding().runtime_scope(),
            generation: runtime.binding().scope_generation().get(),
            pins: [
                pin.server_artifact_digest(),
                pin.client_artifact_digest(),
                pin.frame_binding_digest(),
                runtime.map_revision_digest(),
            ],
            activation: pin.activation_sequence(),
            registered_us,
            next_us,
            tick: 1,
            expiry: None,
            pending: None,
        });
        Ok(())
    }
}
impl ComposedFreshAdmission<'_, '_, '_> {
    fn spite_global_room(&self) -> Option<&QualifiedNativeEntryRoom> {
        self.effective_rooms().gate_monster_ai()
    }
    /// Called once at the real ServingChannel construction boundary. A fixture
    /// may call this same port with the instant captured at its real bootstrap.
    pub(super) async fn register_world_spite_global(&self, registered_us: u64) -> Result<bool, ()> {
        if registered_us > self.owner_now().get() {
            return Err(());
        }
        let Some(room) = self.spite_global_room() else {
            return Ok(false);
        };
        let Some(active) = self.active_generation else {
            return Ok(false);
        };
        let Some(native) = active.native_gameplay() else {
            return Ok(false);
        };
        let runtime = self.runtime.lock().await;
        if !matches!(
            runtime.binding().runtime_scope(),
            RuntimeScopeRefV1::Channel { .. }
        ) {
            return Ok(false);
        }
        let Some(world) = active.qualify_published_world_context(room, runtime.content_pin())
        else {
            return Ok(false);
        };
        let Some(proof) = native.qualify_spite_global_event_in_published_world(
            room,
            runtime.content_pin(),
            &world,
        ) else {
            return Ok(false);
        };
        let mut states = self.states.spell_states.try_lock().map_err(|_| ())?;
        states
            .spite_global
            .register(&runtime, proof, registered_us)?;
        Ok(true)
    }
    /// Older timers run before the copied death task. This port never creates a
    /// registration or advances a fresh Think while draining a retained cutoff.
    pub(super) async fn drive_spite_global_due(
        &self,
        permit: &SpellLanePermit,
        now_us: u64,
    ) -> Result<bool, ()> {
        let mut pending = {
            let mut runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.try_lock().map_err(|_| ())?;
            let Some(registration) = &mut states.spite_global.registration else {
                return Ok(true);
            };
            if !registration.current_for(&runtime) {
                return Err(());
            }
            if registration.next_due_us() > source_cutoff(&runtime, now_us)? {
                return Ok(true);
            }
            let room = self.spite_global_room().ok_or(())?;
            let native = self
                .active_generation
                .and_then(|active| active.native_gameplay())
                .ok_or(())?;
            registration.capture_due(&mut runtime, room, native, now_us)?
        };
        if pending.callback.is_expiry() && pending.expiry_target.is_none() {
            self.reserve_spite_expiry(permit, &mut pending).await?;
        }
        // No runtime or callback-state guard enters this future. An error may
        // mean an unknown COMMIT; retain the same callback, choice and target.
        let outcome = self
            .root
            .commit_spite_global_fire(
                permit,
                self.character,
                self.holder,
                &pending.callback,
                &pending.prepared,
            )
            .await
            .map_err(|_| ())?;
        if matches!(&outcome, SpiteGlobalFireOutcome::Committed(_)) {
            self.states.loop_ground.invalidate();
        }
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.try_lock().map_err(|_| ())?;
        let registration = states.spite_global.registration.as_mut().ok_or(())?;
        if !registration.current_for(&runtime)
            || !pending.current_for(&runtime)
            || !registration
                .pending
                .as_ref()
                .is_some_and(|current| Arc::ptr_eq(&current.callback, &pending.callback))
        {
            return Err(());
        }
        if let Some((next_us, next_tick)) = pending.next_create {
            if pending.callback.is_expiry() || registration.expiry.is_some() {
                return Err(());
            }
            match outcome {
                SpiteGlobalFireOutcome::Committed(receipt) if !receipt.removed => {
                    registration.expiry = Some(Arc::clone(&pending.callback));
                }
                SpiteGlobalFireOutcome::CreateNoSpace => {}
                _ => return Err(()),
            }
            registration.next_us = next_us;
            registration.tick = next_tick;
        } else {
            let SpiteGlobalFireOutcome::Committed(receipt) = outcome else {
                return Err(());
            };
            let target = pending.expiry_target.as_ref().ok_or(())?;
            runtime
                .complete_spite_global_action(
                    &target.reservation,
                    pending.prepared.as_ref(),
                    target
                        .target
                        .as_ref()
                        .map(|(snapshot, defense)| (snapshot, *defense)),
                    receipt.removed,
                )
                .map_err(|_| ())?;
            registration.expiry = None;
        }
        // No fallible operation separates the runtime publication from clearing
        // this exact pending occurrence. A replay cannot add defense twice.
        registration.pending = None;
        let drained = registration.next_due_us() > source_cutoff(&runtime, now_us)?;
        states.owner_wake.notify_one();
        Ok(drained)
    }

    async fn reserve_spite_expiry(
        &self,
        permit: &SpellLanePermit,
        pending: &mut Pending,
    ) -> Result<(), ()> {
        let plan = {
            let runtime = self.runtime.lock().await;
            if !pending.current_for(&runtime) {
                return Err(());
            }
            self.prepare_source_names(&runtime, &["Goshnar's Spite"])?
        };
        let read = self.read_source_names(permit, &plan).await?;
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.try_lock().map_err(|_| ())?;
        let registration = states.spite_global.registration.as_mut().ok_or(())?;
        if !registration.current_for(&runtime) || !pending.current_for(&runtime) {
            return Err(());
        }
        let current = registration.pending.as_mut().ok_or(())?;
        if !Arc::ptr_eq(&current.callback, &pending.callback) || current.expiry_target.is_some() {
            return Err(());
        }
        let names = self.seal_source_names(&runtime, read)?;
        let named = super::source_names::creature_named(&names, "Goshnar's Spite")?;
        let target = if let Some(named) = named {
            if named.role != super::source_names::SourceNameRole::Creature {
                return Err(());
            }
            let native = self
                .active_generation
                .and_then(|active| active.native_gameplay())
                .ok_or(())?;
            if super::soul_war_callbacks::source_actor(&runtime, native, named.actor).as_deref()
                != Some("oteryn:creature.goshnar_s_spite")
            {
                return Err(());
            }
            let snapshot = runtime
                .companion_snapshot_including_dead(named.actor)
                .map_err(|_| ())?;
            let defense = runtime
                .effective_creature_defense(&snapshot)
                .map_err(|_| ())?;
            defense.checked_add(10).ok_or(())?;
            Some((snapshot, defense))
        } else {
            None
        };
        let actors = target.as_ref().map(|(snapshot, _)| snapshot.actor);
        let reservation = runtime
            .reserve_source_global_action(pending.prepared.as_ref(), actors.as_slice())
            .map_err(|_| ())?;
        let target = Arc::new(ExpiryTarget {
            reservation,
            target,
        });
        current.expiry_target = Some(Arc::clone(&target));
        pending.expiry_target = Some(target);
        Ok(())
    }
}
