//! Neutral source-map initialization ABI. No spell/player/death cause.
#![allow(
    dead_code,
    reason = "spell import candidate; awaits its production owner caller"
)]
use super::item_mint::{GroundPlacement, TypedDefinitionRef};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeMapOwnerBinding {
    pub world: crate::foundation::WorldId,
    pub channel: crate::foundation::ChannelId,
    pub scope_generation: u64,
    pub content_digest: [u8; 32],
    pub map_digest: [u8; 32],
    pub frame_digest: [u8; 32],
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeMapItemPlacement {
    pub placement_key: String,
    pub definition: TypedDefinitionRef,
    pub quantity: u32,
    pub ground: GroundPlacement,
    pub source_binding: Vec<u8>,
    pub source_attributes: serde_json::Value,
    pub blocks_movement: bool,
    pub blocks_projectile: bool,
    pub immovable_block_solid: bool,
    pub owner_kind: Option<String>,
}
pub mod map_initializer_seal {
    pub trait Sealed {}
}
/// Actual active map + current physical scope owner is the sole runtime producer.
/// No caller-selected source IDs, player command, loot cause or property allocation.
pub trait NativeMapInitializationProof: map_initializer_seal::Sealed {
    fn current_binding(&self) -> Result<NativeMapOwnerBinding, &'static str>;
    fn placements(&self) -> &[NativeMapItemPlacement];
}
#[derive(Debug, Clone)]
pub struct MaterializedNativeMapItem {
    pub placement_key: String,
    pub transaction_id: [u8; 16],
    pub item_instance_id: [u8; 16],
    pub replayed: bool,
}
