//! Canonical opaque placement encoding shared by production and native PG roots.

/// D1241 canonical opaque placement bytes, extracted exactly from death_reward:
/// signed x i32BE || signed y i32BE || floor i16BE. No frame conversion or clamp.
pub fn encode_spatial_position(position: crate::foundation::MovementLocalPosition) -> Vec<u8> {
    let mut spatial_position = Vec::with_capacity(10);
    spatial_position.extend_from_slice(&position.x.to_be_bytes());
    spatial_position.extend_from_slice(&position.y.to_be_bytes());
    spatial_position.extend_from_slice(&position.floor.to_be_bytes());
    spatial_position
}
