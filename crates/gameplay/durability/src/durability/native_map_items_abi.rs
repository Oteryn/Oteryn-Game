//! Neutral source-map initialization ABI. No spell/player/death cause.
#![allow(
    dead_code,
    reason = "spell import candidate; awaits its production owner caller"
)]
use super::item_mint::GroundPlacement;
pub type NativeMapOwnerBinding = oteryn_gameplay_items::native_map_items_abi::NativeMapOwnerBinding<
    crate::foundation::WorldId,
    crate::foundation::ChannelId,
>;
pub type NativeMapItemPlacement =
    oteryn_gameplay_items::native_map_items_abi::NativeMapItemPlacement<GroundPlacement>;
pub mod map_initializer_seal {
    pub trait Sealed {}
}
/// Actual active map + current physical scope owner is the sole runtime producer.
/// No caller-selected source IDs, player command, loot cause or property allocation.
pub trait NativeMapInitializationProof: map_initializer_seal::Sealed {
    fn current_binding(&self) -> Result<NativeMapOwnerBinding, &'static str>;
    fn placements(&self) -> &[NativeMapItemPlacement];
}
pub use oteryn_gameplay_items::native_map_items_abi::MaterializedNativeMapItem;
