//! Kernel actor identities shared by the carrier and owner timer.

use super::{ChannelId, RuntimeScopeRefV1, ScopeOwnershipGeneration, WorldId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorLocalId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorLocalGeneration(pub u64);

/// Carrier-issued actor identity. Its scope generation stays private to the kernel;
/// carrier lookup independently validates its current scope and slot generation:
///
/// ```
/// use oteryn_gameplay_foundation::foundation::ActorRef;
/// fn local_id(actor: ActorRef) -> u32 { actor.actor_local_id.0 }
/// ```
///
/// ```compile_fail
/// use oteryn_gameplay_foundation::foundation::ActorRef;
/// fn forge(actor: ActorRef) -> ActorRef { ActorRef { ..actor } }
/// ```
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ActorRef {
    pub runtime_scope: RuntimeScopeRefV1,
    pub(crate) scope_generation: ScopeOwnershipGeneration,
    pub actor_local_id: ActorLocalId,
    pub actor_local_generation: ActorLocalGeneration,
}

// Native cast intents and retained owner bindings include this Debug representation.
// Preserve the original Channel shape until those consumers adopt a versioned encoding.
impl std::fmt::Debug for ActorRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out = f.debug_struct("ActorRef");
        match self.runtime_scope {
            RuntimeScopeRefV1::Channel {
                world_id,
                channel_id,
            } => {
                out.field("world_id", &world_id)
                    .field("channel_id", &channel_id);
            }
            RuntimeScopeRefV1::Instance { .. } => {
                out.field("runtime_scope", &self.runtime_scope);
            }
        }
        out.field("scope_generation", &self.scope_generation)
            .field("actor_local_id", &self.actor_local_id)
            .field("actor_local_generation", &self.actor_local_generation)
            .finish()
    }
}

impl ActorRef {
    /// Records an owner-issued identity; every carrier lookup still validates its live scope
    /// and slot generation before granting access or committing a mutation.
    pub const fn from_owner_parts(
        world_id: WorldId,
        channel_id: ChannelId,
        scope_generation: ScopeOwnershipGeneration,
        actor_local_id: ActorLocalId,
        actor_local_generation: ActorLocalGeneration,
    ) -> Self {
        Self::from_scoped_owner_parts(
            RuntimeScopeRefV1::channel(world_id, channel_id),
            scope_generation,
            actor_local_id,
            actor_local_generation,
        )
    }

    /// Records a typed owner-issued identity. This grants no admission: carrier lookups
    /// must still validate scope, ownership generation and slot generation.
    pub const fn from_scoped_owner_parts(
        runtime_scope: RuntimeScopeRefV1,
        scope_generation: ScopeOwnershipGeneration,
        actor_local_id: ActorLocalId,
        actor_local_generation: ActorLocalGeneration,
    ) -> Self {
        Self {
            runtime_scope,
            scope_generation,
            actor_local_id,
            actor_local_generation,
        }
    }
}

#[cfg(any(test, feature = "test-support"))]
impl ActorRef {
    /// Test builds only: restamp the scope generation, so a test can present the carrier
    /// with a reference from a scope it does not own.
    pub fn set_scope_generation(&mut self, scope_generation: ScopeOwnershipGeneration) {
        self.scope_generation = scope_generation;
    }
}

/// Inseparable semantic actor handle issued by one runtime carrier.
/// Its internals are exposed within the crate for carrier qualification tests.
/// It is never decoded from a client handle or derived from the fixture Ability `TargetId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExactActorRef(pub ActorRef);

impl ExactActorRef {
    pub const fn world_id(self) -> WorldId {
        self.0.runtime_scope.world_id()
    }
    pub const fn runtime_scope(self) -> RuntimeScopeRefV1 {
        self.0.runtime_scope
    }

    /// Channel-only consumers must refuse an Instance rather than use origin routing.
    pub const fn channel_id(self) -> Option<ChannelId> {
        match self.runtime_scope() {
            RuntimeScopeRefV1::Channel { channel_id, .. } => Some(channel_id),
            RuntimeScopeRefV1::Instance { .. } => None,
        }
    }
    pub const fn scope_generation(self) -> ScopeOwnershipGeneration {
        self.0.scope_generation
    }
    pub const fn actor_local_id(self) -> u32 {
        self.0.actor_local_id.0
    }
    pub const fn actor_local_generation(self) -> u64 {
        self.0.actor_local_generation.0
    }
    /// Stable opaque placement identity: a domain-separated digest of the exact actor
    /// reference in its runtime scope. It carries no position and reveals no slot layout.
    pub fn placement_identity(self) -> [u8; 16] {
        self.identity(
            b"oteryn:runtime-player-placement:v1",
            b"oteryn:instance-player-placement:v1",
        )
    }
}

impl ExactActorRef {
    /// Transport test fixture: an actor reference that names no runtime slot.
    #[cfg(any(test, feature = "test-support"))]
    #[allow(clippy::expect_used, dead_code)]
    pub fn transport_fixture(world_id: WorldId, channel_id: ChannelId) -> Self {
        Self(ActorRef {
            runtime_scope: RuntimeScopeRefV1::channel(world_id, channel_id),
            scope_generation: ScopeOwnershipGeneration::new(1).expect("generation"),
            actor_local_id: ActorLocalId(1),
            actor_local_generation: ActorLocalGeneration(1),
        })
    }
}

impl ExactActorRef {
    /// MAP-ITEM-REF-1 Part B: the stable opaque identity of this actor's corpse, separate from
    /// [`Self::placement_identity`] so a corpse never shares an identity with the actor it was.
    pub fn corpse_identity(self) -> [u8; 16] {
        self.identity(
            b"oteryn:runtime-corpse-placement:v1",
            b"oteryn:instance-corpse-placement:v1",
        )
    }

    fn identity(self, channel_domain: &[u8], instance_domain: &[u8]) -> [u8; 16] {
        use sha2::{Digest, Sha256};
        let (domain, local_scope_id) = match self.runtime_scope() {
            RuntimeScopeRefV1::Channel { channel_id, .. } => {
                (channel_domain, *channel_id.as_bytes())
            }
            RuntimeScopeRefV1::Instance { instance_id, .. } => (instance_domain, instance_id),
        };
        let digest = Sha256::new()
            .chain_update(domain)
            .chain_update(self.world_id().as_bytes())
            .chain_update(local_scope_id)
            .chain_update(self.scope_generation().get().to_be_bytes())
            .chain_update(self.actor_local_id().to_be_bytes())
            .chain_update(self.actor_local_generation().to_be_bytes())
            .finalize();
        let mut identity = [0_u8; 16];
        identity.copy_from_slice(&digest[..16]);
        identity
    }
}

impl ExactActorRef {
    /// Current native actor slot ordinal for private source chain selection, never a wire ID.
    pub fn chain_local_actor_id(self) -> u64 {
        u64::from(self.0.actor_local_id.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boss_activity_actor_identity_is_typed_and_preserves_channel_v1() -> Result<(), String> {
        let mut id = [1_u8; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        let world = WorldId::decode(&id).map_err(|error| format!("{error:?}"))?;
        let channel = ChannelId::decode(&id).map_err(|error| format!("{error:?}"))?;
        let generation = ScopeOwnershipGeneration::new(3).map_err(|error| format!("{error:?}"))?;
        let actor = ExactActorRef(ActorRef::from_owner_parts(
            world,
            channel,
            generation,
            ActorLocalId(4),
            ActorLocalGeneration(5),
        ));
        assert_eq!(
            actor.runtime_scope(),
            RuntimeScopeRefV1::channel(world, channel)
        );
        assert_eq!(actor.channel_id(), Some(channel));
        // Frozen pre-scope ActorRef field labels/order used by native intent bindings.
        let legacy_debug = format!(
            "ActorRef {{ world_id: {world:?}, channel_id: {channel:?}, scope_generation: {generation:?}, actor_local_id: ActorLocalId(4), actor_local_generation: ActorLocalGeneration(5) }}"
        );
        assert_eq!(format!("{:?}", actor.0), legacy_debug);
        assert_eq!(
            format!("{actor:?}"),
            format!("ExactActorRef({legacy_debug})")
        );
        // Frozen Channel v1 vectors from the pre-scope byte encoding.
        assert_eq!(
            actor.placement_identity(),
            [
                0x8f, 0x29, 0xc1, 0xe3, 0x83, 0xcc, 0x19, 0xce, 0xd4, 0x6b, 0x10, 0xcb, 0x26, 0xc5,
                0xe0, 0xcd
            ]
        );
        assert_eq!(
            actor.corpse_identity(),
            [
                0x03, 0x7a, 0x01, 0xbf, 0x2a, 0x8d, 0x3d, 0xbd, 0x69, 0xbb, 0x17, 0x7f, 0x3d, 0xc7,
                0xd9, 0xcf
            ]
        );
        let instance = ExactActorRef(ActorRef::from_scoped_owner_parts(
            RuntimeScopeRefV1::instance(world, id).map_err(|error| format!("{error:?}"))?,
            generation,
            ActorLocalId(4),
            ActorLocalGeneration(5),
        ));
        assert_eq!(instance.world_id(), world);
        assert_eq!(instance.channel_id(), None);
        assert_ne!(actor, instance);
        assert_ne!(actor.placement_identity(), instance.placement_identity());
        assert_ne!(actor.corpse_identity(), instance.corpse_identity());
        assert_ne!(instance.placement_identity(), instance.corpse_identity());
        // Every component of Instance identity participates in both opaque digests.
        let mut other = id;
        other[15] += 1;
        for changed in [
            ActorRef {
                runtime_scope: RuntimeScopeRefV1::instance(world, other)
                    .map_err(|error| format!("{error:?}"))?,
                ..instance.0
            },
            ActorRef {
                runtime_scope: RuntimeScopeRefV1::instance(
                    WorldId::decode(&other).map_err(|error| format!("{error:?}"))?,
                    id,
                )
                .map_err(|error| format!("{error:?}"))?,
                ..instance.0
            },
            ActorRef {
                scope_generation: ScopeOwnershipGeneration::new(4)
                    .map_err(|error| format!("{error:?}"))?,
                ..instance.0
            },
            ActorRef {
                actor_local_id: ActorLocalId(5),
                ..instance.0
            },
            ActorRef {
                actor_local_generation: ActorLocalGeneration(6),
                ..instance.0
            },
        ] {
            let changed = ExactActorRef(changed);
            assert_ne!(instance, changed);
            assert_ne!(instance.placement_identity(), changed.placement_identity());
            assert_ne!(instance.corpse_identity(), changed.corpse_identity());
        }
        Ok(())
    }
}
