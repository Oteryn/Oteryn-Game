//! The registered source GlobalEvent belongs to the actual scope owner, not a
//! boss or a Player. Deadlines use its monotonic execution clock, never SQL UTC.
use super::ComposedFreshAdmission;
use crate::content::native_soul_war_global_events::QualifiedSpiteGlobalEvent;
use crate::foundation::{ChannelRuntimeV1, RuntimeScopeRefV1};

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
impl SpiteGlobalState {
    pub(super) fn next_due_us(&self) -> Option<u64> {
        self.registration
            .as_ref()
            .map(|registration| registration.next_us)
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
        });
        Ok(())
    }
}
impl ComposedFreshAdmission<'_, '_, '_> {
    /// Called once at the real ServingChannel construction boundary. A fixture
    /// may call this same port with the instant captured at its real bootstrap.
    pub(super) async fn register_world_spite_global(&self, registered_us: u64) -> Result<bool, ()> {
        if registered_us > self.owner_now().get() {
            return Err(());
        }
        let Some(room) = self.effective_rooms().gate_monster_ai() else {
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
    pub(super) async fn drive_spite_global_due(&self, now_us: u64) -> Result<bool, ()> {
        let runtime = self.runtime.lock().await;
        let states = self.states.spell_states.try_lock().map_err(|_| ())?;
        let Some(registration) = &states.spite_global.registration else {
            return Ok(true);
        };
        if !registration.current_for(&runtime) {
            return Err(());
        }
        let cutoff = runtime
            .next_source_creature_death()
            .map_err(|_| ())?
            .and_then(|notice| notice.enqueued_at_us())
            .map_or(now_us, |at| at.min(now_us));
        if registration.next_us > cutoff {
            return Ok(true);
        }
        // Neutral until an actual registered owner pass reaches its14s effect.
        Ok(false)
    }
}
