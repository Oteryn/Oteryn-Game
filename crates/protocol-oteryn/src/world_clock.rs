//! World clock payloads (LOOP-CLOCK-1).
//!
//! Schema: `docs/contracts/protocol-oteryn/v1/world_clock_v1.proto`, from WORLD-INTERACTION-0 §10.2
//! and §11. Capability 19 `WORLD_LIGHT_V1` and state domain 18 `WORLD_CLOCK` are registered in
//! `PROTOCOL_OTERYN_V1_REGISTRY.json`, leased by the #1622 control plane. The capability has no
//! command type, and the snapshot and the delta carry the same message.
//!
//! Decoding is strict, and encoding refuses the same values as a server fault before any byte is
//! emitted. The following all fail closed: a Tibian minute of 1,440 or more, a light level or color
//! above 255, a payload over its bound, and unknown or repeated fields.

pub use crate::charm_wire::CyclopediaWireError as WorldClockWireError;
use crate::charm_wire::{WireResult, push_nonzero_varint_field, read_uint32_fields};

/// Registered capability `WORLD_LIGHT_V1`: no command type, state domain 18, requires 6.
pub const CAPABILITY_WORLD_LIGHT_V1: u32 = 19;
/// The capability `WORLD_LIGHT_V1` requires: 6 `WORLD_SPATIAL_ENTITIES`, the world state it rides on.
pub const CAPABILITY_WORLD_LIGHT_V1_REQUIRES: [u32; 1] =
    [crate::world_spatial_entities::CAPABILITY_WORLD_SPATIAL_ENTITIES];
/// Registered state domain `WORLD_CLOCK` (capability 19).
pub const STATE_DOMAIN_WORLD_CLOCK: u32 = 18;
/// Delta type 1 `WORLD_CLOCK_DELTA_V1`.
pub const DELTA_TYPE_WORLD_CLOCK_V1: u32 = 1;
/// Snapshot type 1 `WORLD_CLOCK_SNAPSHOT_V1`.
pub const SNAPSHOT_TYPE_WORLD_CLOCK_V1: u32 = 1;

/// `WORLDINT0-RL-16`: Tibian minutes in a Tibian day.
pub const TIBIAN_MINUTES_PER_DAY: u32 = 1_440;
/// At most 9 bytes: a 2-byte varint and two 2-byte varints, each with a 1-byte tag.
pub const MAX_WORLD_CLOCK_BYTES: usize = 9;

const _: () = assert!(MAX_WORLD_CLOCK_BYTES <= crate::MAX_STATE_DELTA_PAYLOAD_BYTES);

/// `WorldClockV1`: the Tibian minute of the day and the World light at that minute.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WorldClock {
    pub tibian_minute: u16,
    pub light_level: u8,
    pub light_color: u8,
}

/// Encodes the snapshot (type 1) or delta (type 1) payload. A minute outside the day is a server
/// fault.
pub fn encode_world_clock(clock: WorldClock) -> WireResult<Vec<u8>> {
    if u32::from(clock.tibian_minute) >= TIBIAN_MINUTES_PER_DAY {
        return Err(WorldClockWireError::Malformed);
    }
    let mut output = Vec::with_capacity(MAX_WORLD_CLOCK_BYTES);
    push_nonzero_varint_field(&mut output, 1, u64::from(clock.tibian_minute));
    push_nonzero_varint_field(&mut output, 2, u64::from(clock.light_level));
    push_nonzero_varint_field(&mut output, 3, u64::from(clock.light_color));
    Ok(output)
}

pub fn decode_world_clock(payload: &[u8]) -> WireResult<WorldClock> {
    if payload.len() > MAX_WORLD_CLOCK_BYTES {
        return Err(WorldClockWireError::LimitExceeded);
    }
    let [minute, level, color] = read_uint32_fields::<3>(payload)?;
    if minute >= TIBIAN_MINUTES_PER_DAY {
        return Err(WorldClockWireError::Malformed);
    }
    Ok(WorldClock {
        tibian_minute: u16::try_from(minute).map_err(|_| WorldClockWireError::Malformed)?,
        light_level: u8::try_from(level).map_err(|_| WorldClockWireError::Malformed)?,
        light_color: u8::try_from(color).map_err(|_| WorldClockWireError::Malformed)?,
    })
}

#[cfg(test)]
#[path = "world_clock_tests.rs"]
mod tests;
