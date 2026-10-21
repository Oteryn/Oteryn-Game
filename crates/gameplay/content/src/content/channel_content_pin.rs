use oteryn_gameplay_domain::domain::carrier_values::LocalPosition;
use oteryn_protocol_oteryn::WorldId;

/// The exact active Content generation a Channel runtime is created with (#935 "Activation issuer
/// and current pin"): scope World, activation sequence, pair digests and the source-qualified
/// frame binding digest. In production it is minted only by the Content activation path from an
/// activated native entry generation; it is fixed for the runtime's lifetime.
#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "test-support"), derive(Clone))]
pub struct ChannelContentPin {
    world_id: WorldId,
    activation_sequence: u64,
    server_artifact_digest: [u8; 32],
    client_artifact_digest: [u8; 32],
    frame_binding_digest: [u8; 32],
    map_revision_digest: [u8; 32],
    /// The first-entry start cell the active generation attests as Walkable.
    entry_start: LocalPosition,
}

impl ChannelContentPin {
    pub const fn entry_start(&self) -> LocalPosition {
        self.entry_start
    }

    pub const fn map_revision_digest(&self) -> [u8; 32] {
        self.map_revision_digest
    }
    pub const fn from_activation(
        world_id: WorldId,
        activation_sequence: u64,
        server_artifact_digest: [u8; 32],
        client_artifact_digest: [u8; 32],
        frame_binding_digest: [u8; 32],
        map_revision_digest: [u8; 32],
        entry_start: (i32, i32, i16),
    ) -> Self {
        Self {
            world_id,
            activation_sequence,
            server_artifact_digest,
            client_artifact_digest,
            frame_binding_digest,
            map_revision_digest,
            entry_start: LocalPosition {
                x: entry_start.0,
                y: entry_start.1,
                floor: entry_start.2,
            },
        }
    }

    /// Synthetic pin for runtime tests that do not exercise Content activation.
    #[cfg(any(test, feature = "test-support"))]
    pub const fn test(world_id: WorldId) -> Self {
        Self::from_activation(world_id, 1, [1; 32], [2; 32], [3; 32], [4; 32], (0, 0, 0))
    }

    pub const fn world_id(&self) -> WorldId {
        self.world_id
    }

    pub const fn activation_sequence(&self) -> u64 {
        self.activation_sequence
    }

    pub const fn server_artifact_digest(&self) -> [u8; 32] {
        self.server_artifact_digest
    }

    pub const fn client_artifact_digest(&self) -> [u8; 32] {
        self.client_artifact_digest
    }

    pub const fn frame_binding_digest(&self) -> [u8; 32] {
        self.frame_binding_digest
    }
}
