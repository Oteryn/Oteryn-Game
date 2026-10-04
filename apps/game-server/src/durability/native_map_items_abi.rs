//! Neutral source-map initialization ABI. No spell/player/death cause.
#![allow(
    dead_code,
    reason = "spell import candidate; awaits its production owner caller"
)]
use super::item_mint::{GroundPlacement, TypedDefinitionRef};
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeMapOwnerBinding {
    pub(crate) world: crate::foundation::WorldId,
    pub(crate) channel: crate::foundation::ChannelId,
    pub(crate) scope_generation: u64,
    pub(crate) content_digest: [u8; 32],
    pub(crate) map_digest: [u8; 32],
    pub(crate) frame_digest: [u8; 32],
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeMapItemPlacement {
    pub(crate) placement_key: String,
    pub(crate) definition: TypedDefinitionRef,
    pub(crate) quantity: u32,
    pub(crate) ground: GroundPlacement,
    pub(crate) source_binding: Vec<u8>,
    pub(crate) source_attributes: serde_json::Value,
    pub(crate) blocks_movement: bool,
    pub(crate) blocks_projectile: bool,
    pub(crate) immovable_block_solid: bool,
    pub(crate) owner_kind: Option<String>,
}
pub(crate) mod map_initializer_seal {
    pub(crate) trait Sealed {}
}
/// Actual active map + current physical scope owner is the sole runtime producer.
/// No caller-selected source IDs, player command, loot cause or property allocation.
pub(crate) trait NativeMapInitializationProof: map_initializer_seal::Sealed {
    fn current_binding(&self) -> Result<NativeMapOwnerBinding, &'static str>;
    fn placements(&self) -> &[NativeMapItemPlacement];
}
#[derive(Debug, Clone)]
pub(crate) struct MaterializedNativeMapItem {
    pub(crate) placement_key: String,
    pub(crate) transaction_id: [u8; 16],
    pub(crate) item_instance_id: [u8; 16],
    pub(crate) replayed: bool,
}
