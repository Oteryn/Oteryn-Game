//! Neutral source-map placement, binding and materialization values.

use oteryn_gameplay_domain::domain::item_definition::TypedDefinitionRef;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeMapOwnerBinding<World, Channel> {
    pub world: World,
    pub channel: Channel,
    pub scope_generation: u64,
    pub content_digest: [u8; 32],
    pub map_digest: [u8; 32],
    pub frame_digest: [u8; 32],
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeMapItemPlacement<Placement> {
    pub placement_key: String,
    pub definition: TypedDefinitionRef,
    pub quantity: u32,
    pub ground: Placement,
    pub source_binding: Vec<u8>,
    pub source_attributes: serde_json::Value,
    pub blocks_movement: bool,
    pub blocks_projectile: bool,
    pub immovable_block_solid: bool,
    pub owner_kind: Option<String>,
}
#[derive(Debug, Clone)]
pub struct MaterializedNativeMapItem {
    pub placement_key: String,
    pub transaction_id: [u8; 16],
    pub item_instance_id: [u8; 16],
    pub replayed: bool,
}
