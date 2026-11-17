//! Ephemeral source-creature cue receipts in the existing BundleFacts owner.
//! A captured cue can be published after its caster's administrative retirement,
//! while the exact scope generation and content/map/frame binding remain current.
use super::*;
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, RuntimeScopeRefV1, RuntimeWorkStamp, ScopeOwnershipGeneration,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatureVisualKind {
    Effect {
        position: TilePos,
        effect: u16,
    },
    Missile {
        from: TilePos,
        to: TilePos,
        missile: u16,
    },
}

/// Opaque, non-Clone capture. Moving it into the map outbox consumes that receipt.
#[derive(Debug)]
pub struct CreatureVisualCue {
    kind: CreatureVisualKind,
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
    stamp: RuntimeWorkStamp,
    activation: u64,
    server: [u8; 32],
    client: [u8; 32],
    map: [u8; 32],
    frame: [u8; 32],
}
impl CreatureVisualCue {
    fn capture(
        runtime: &ChannelRuntimeV1,
        stamp: RuntimeWorkStamp,
        actor: ExactActorRef,
        kind: CreatureVisualKind,
    ) -> Option<Self> {
        if !runtime.contains_live_creature(actor) {
            return None;
        }
        Self::capture_binding(runtime, stamp, actor, kind)
    }
    fn capture_binding(
        runtime: &ChannelRuntimeV1,
        stamp: RuntimeWorkStamp,
        actor: ExactActorRef,
        kind: CreatureVisualKind,
    ) -> Option<Self> {
        let binding = runtime.binding();
        let fence = runtime.owner_fence().ok()?;
        if !fence.accepts_stamp(stamp) {
            return None;
        }
        let position = runtime.read_actor_position(actor).ok()?;
        if position.context() != runtime.pinned_movement_context() {
            return None;
        }
        let pin = runtime.content_pin();
        Some(Self {
            kind,
            scope: binding.runtime_scope(),
            generation: binding.scope_generation(),
            stamp,
            activation: pin.activation_sequence(),
            server: pin.server_artifact_digest(),
            client: pin.client_artifact_digest(),
            map: pin.map_revision_digest(),
            frame: pin.frame_binding_digest(),
        })
    }
    pub fn effect(
        runtime: &ChannelRuntimeV1,
        stamp: RuntimeWorkStamp,
        actor: ExactActorRef,
        position: TilePos,
        effect: u16,
    ) -> Option<Self> {
        if effect == 0
            || crate::movement::interest::tibia_floor(i16::from(position.floor)).is_none()
        {
            return None;
        }
        Self::capture(
            runtime,
            stamp,
            actor,
            CreatureVisualKind::Effect { position, effect },
        )
    }
    /// A source callback can observe its current named Creature at HP0 until
    /// canonical onDeath unregisters it. This grants only a bound visual cue.
    pub fn effect_from_registered_source(
        runtime: &ChannelRuntimeV1,
        stamp: RuntimeWorkStamp,
        actor: ExactActorRef,
        position: TilePos,
        effect: u16,
    ) -> Option<Self> {
        if effect == 0
            || crate::movement::interest::tibia_floor(i16::from(position.floor)).is_none()
            || !runtime.source_creature_is_registered(actor).ok()?
        {
            return None;
        }
        Self::capture_binding(
            runtime,
            stamp,
            actor,
            CreatureVisualKind::Effect { position, effect },
        )
    }
    pub fn missile(
        runtime: &ChannelRuntimeV1,
        stamp: RuntimeWorkStamp,
        actor: ExactActorRef,
        from: TilePos,
        to: TilePos,
        missile: u16,
    ) -> Option<Self> {
        if missile == 0
            || [from, to].iter().any(|position| {
                crate::movement::interest::tibia_floor(i16::from(position.floor)).is_none()
            })
        {
            return None;
        }
        Self::capture(
            runtime,
            stamp,
            actor,
            CreatureVisualKind::Missile { from, to, missile },
        )
    }
    pub(super) fn current(&self, runtime: &ChannelRuntimeV1) -> bool {
        let pin = runtime.content_pin();
        runtime.owner_fence().is_ok_and(|fence| {
            fence.is_current_for_scope(self.scope, self.generation)
                && fence.accepts_stamp(self.stamp)
        }) && pin.activation_sequence() == self.activation
            && pin.server_artifact_digest() == self.server
            && pin.client_artifact_digest() == self.client
            && pin.map_revision_digest() == self.map
            && pin.frame_binding_digest() == self.frame
    }
    pub(super) fn into_kind(self) -> CreatureVisualKind {
        self.kind
    }
}
