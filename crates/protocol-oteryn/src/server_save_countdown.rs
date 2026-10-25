//! PARITY-EVENT-CALLBACKS-1 server-save countdown notices. Snapshot is empty state; a delta
//! carries one notice. Coordinator decision, NIGHT-W1: the server only reads the planned reset
//! Control publishes and never triggers, moves or cancels it.
pub use crate::charm_wire::CyclopediaWireError as ServerSaveCountdownError;
use crate::charm_wire::{WireResult, push_varint_field, read_varint};

pub const CAPABILITY_SERVER_SAVE_COUNTDOWN_V1: u32 = 75;
pub const STATE_DOMAIN_SERVER_SAVE_COUNTDOWN: u32 = 40;
pub const SNAPSHOT_TYPE_SERVER_SAVE_COUNTDOWN_V1: u32 = 1;
pub const DELTA_TYPE_SERVER_SAVE_COUNTDOWN_V1: u32 = 1;
pub const SERVER_SAVE_COUNTDOWN_SNAPSHOT: &[u8] = &[0x08, 1];
/// Canary `GLOBAL_SERVER_SAVE_NOTIFY_DURATION` default: one notice per minute from 5 to 1.
pub const SERVER_SAVE_COUNTDOWN_MAX_MINUTES: u8 = 5;
const MAX_PAYLOAD_BYTES: usize = 16;

/// One countdown notice for the planned reset that ends `ending_epoch`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ServerSaveCountdown {
    pub ending_epoch: u64,
    pub minutes_remaining: u8,
}

const fn valid_minutes(minutes: u8) -> bool {
    minutes >= 1 && minutes <= SERVER_SAVE_COUNTDOWN_MAX_MINUTES
}

pub fn encode_server_save_countdown(notice: ServerSaveCountdown) -> WireResult<Vec<u8>> {
    if !valid_minutes(notice.minutes_remaining) {
        return Err(ServerSaveCountdownError::Malformed);
    }
    let mut bytes = Vec::new();
    // Field 1 is written even for epoch zero, so both fields are always present.
    push_varint_field(&mut bytes, 1, notice.ending_epoch);
    push_varint_field(&mut bytes, 2, u64::from(notice.minutes_remaining));
    Ok(bytes)
}

pub fn decode_server_save_countdown(bytes: &[u8]) -> WireResult<ServerSaveCountdown> {
    if bytes.len() > MAX_PAYLOAD_BYTES {
        return Err(ServerSaveCountdownError::LimitExceeded);
    }
    let mut cursor = 0;
    if read_varint(bytes, &mut cursor)? != 0x08 {
        return Err(ServerSaveCountdownError::Malformed);
    }
    let ending_epoch = read_varint(bytes, &mut cursor)?;
    if read_varint(bytes, &mut cursor)? != 0x10 {
        return Err(ServerSaveCountdownError::Malformed);
    }
    let minutes_remaining = u8::try_from(read_varint(bytes, &mut cursor)?)
        .map_err(|_| ServerSaveCountdownError::Malformed)?;
    if !valid_minutes(minutes_remaining) || cursor != bytes.len() {
        return Err(ServerSaveCountdownError::Malformed);
    }
    Ok(ServerSaveCountdown {
        ending_epoch,
        minutes_remaining,
    })
}

#[cfg(test)]
#[path = "server_save_countdown_tests.rs"]
mod tests;
