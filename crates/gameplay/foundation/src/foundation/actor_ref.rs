//! Kernel actor identities shared by the carrier and owner timer.

use super::{ChannelId, ScopeOwnershipGeneration, WorldId};

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorRef {
    pub world_id: WorldId,
    pub channel_id: ChannelId,
    pub(crate) scope_generation: ScopeOwnershipGeneration,
    pub actor_local_id: ActorLocalId,
    pub actor_local_generation: ActorLocalGeneration,
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
        Self {
            world_id,
            channel_id,
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

/// Inseparable semantic actor handle issued by this one Channel carrier.
/// Its internals are exposed within the crate for carrier qualification tests.
/// It is never decoded from a client handle or derived from the fixture Ability `TargetId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExactActorRef(pub ActorRef);

impl ExactActorRef {
    pub const fn world_id(self) -> WorldId {
        self.0.world_id
    }
    pub const fn channel_id(self) -> ChannelId {
        self.0.channel_id
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
        use sha2::{Digest, Sha256};
        let ActorRef {
            world_id,
            channel_id,
            scope_generation,
            actor_local_id,
            actor_local_generation,
        } = self.0;
        let digest = Sha256::new()
            .chain_update(b"oteryn:runtime-player-placement:v1")
            .chain_update(world_id.as_bytes())
            .chain_update(channel_id.as_bytes())
            .chain_update(scope_generation.get().to_be_bytes())
            .chain_update(actor_local_id.0.to_be_bytes())
            .chain_update(actor_local_generation.0.to_be_bytes())
            .finalize();
        let mut identity = [0_u8; 16];
        identity.copy_from_slice(&digest[..16]);
        identity
    }
}

impl ExactActorRef {
    /// Transport test fixture: an actor reference that names no runtime slot.
    #[cfg(any(test, feature = "test-support"))]
    #[allow(clippy::expect_used, dead_code)]
    pub fn transport_fixture(world_id: WorldId, channel_id: ChannelId) -> Self {
        Self(ActorRef {
            world_id,
            channel_id,
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
        use sha2::{Digest, Sha256};
        let ActorRef {
            world_id,
            channel_id,
            scope_generation,
            actor_local_id,
            actor_local_generation,
        } = self.0;
        let digest = Sha256::new()
            .chain_update(b"oteryn:runtime-corpse-placement:v1")
            .chain_update(world_id.as_bytes())
            .chain_update(channel_id.as_bytes())
            .chain_update(scope_generation.get().to_be_bytes())
            .chain_update(actor_local_id.0.to_be_bytes())
            .chain_update(actor_local_generation.0.to_be_bytes())
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
