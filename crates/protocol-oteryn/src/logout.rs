//! Logout intent and result typed payloads (LOGOUT-WIRE-1).
//!
//! Schema: `docs/contracts/protocol-oteryn/v1/logout_v1.proto`, from ARCH-KILL-REWARD-LOGOUT-1
//! §1.6. Capability 21 `LOGOUT_V1` (no requirement) and command type 23 `LOGOUT_INTENT` are
//! registered in `PROTOCOL_OTERYN_V1_REGISTRY.json`. The decision proposed capability 19, which
//! was already `WORLD_LIGHT_V1`, and 20 is leased to `CONNECTION_TRACE_V1` (D770).
//!
//! The intent is always empty. Decoding of the result is strict, and encoding refuses the same
//! values as a fault before any byte is emitted: a zero or unknown code, a `retry_after_ms` with
//! any code but `IN_FIGHT`, an `IN_FIGHT` without one or above `ATTACK0-RL-03`, a payload over
//! its byte bound, and unknown or repeated fields all fail closed.

use crate::charm_wire::{
    WireResult, push_nonzero_varint_field, push_varint_field, read_uint32_fields,
};

pub use crate::charm_wire::CyclopediaWireError as LogoutWireError;

/// Registered capability `LOGOUT_V1`: command type 23, no state domain.
pub const CAPABILITY_LOGOUT_V1: u32 = 21;
/// Registered command type `LOGOUT_INTENT` (capability 21).
pub const COMMAND_TYPE_LOGOUT_INTENT: u32 = 23;

/// `LOGOUT-RL-01`: logout commands outstanding per GameSession.
pub const MAX_OUTSTANDING_LOGOUTS: u32 = 1;
/// `ATTACK0-RL-03`: the largest `retry_after_ms`, the whole in-fight deadline.
pub const MAX_LOGOUT_RETRY_AFTER_MS: u32 = 60_000;

/// `LogoutIntentV1`: always empty.
pub const MAX_LOGOUT_INTENT_BYTES: usize = 0;
/// `LogoutResultV1`: the code 1 + 1 and `retry_after_ms` 1 + 3.
pub const MAX_LOGOUT_RESULT_BYTES: usize = 6;

/// `LogoutResultV1`: the result of command type 23.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogoutResult {
    /// The terminal release has committed; the transport closes after this result.
    Accepted,
    /// The actor is in fight for `retry_after_ms` more milliseconds (1..=60000).
    InFight { retry_after_ms: u32 },
    /// A pending durable commit or a retryable release failure; the client may retry.
    Busy,
}

impl LogoutResult {
    /// Whether the command status is `REJECTED`: every code but `ACCEPTED`.
    pub const fn is_rejected(self) -> bool {
        !matches!(self, Self::Accepted)
    }

    const fn code(self) -> u64 {
        match self {
            Self::Accepted => 1,
            Self::InFight { .. } => 2,
            Self::Busy => 3,
        }
    }
}

fn check_retry_after(retry_after_ms: u32) -> WireResult<()> {
    if retry_after_ms == 0 || retry_after_ms > MAX_LOGOUT_RETRY_AFTER_MS {
        return Err(LogoutWireError::Malformed);
    }
    Ok(())
}

/// Encodes a `LogoutIntentV1` (the client side): the empty payload.
pub fn encode_logout_intent() -> Vec<u8> {
    Vec::new()
}

/// Decodes a `LogoutIntentV1`; any byte is refused, and the server answers `REJECTED` with an
/// empty result payload.
pub fn decode_logout_intent(payload: &[u8]) -> WireResult<()> {
    if payload.len() > MAX_LOGOUT_INTENT_BYTES {
        return Err(LogoutWireError::LimitExceeded);
    }
    Ok(())
}

/// Encodes a `LogoutResultV1` (the server side). An `IN_FIGHT` outside 1..=60000 is a fault.
pub fn encode_logout_result(result: LogoutResult) -> WireResult<Vec<u8>> {
    let mut output = Vec::with_capacity(MAX_LOGOUT_RESULT_BYTES);
    push_varint_field(&mut output, 1, result.code());
    if let LogoutResult::InFight { retry_after_ms } = result {
        check_retry_after(retry_after_ms)?;
        push_nonzero_varint_field(&mut output, 2, u64::from(retry_after_ms));
    }
    Ok(output)
}

pub fn decode_logout_result(payload: &[u8]) -> WireResult<LogoutResult> {
    if payload.len() > MAX_LOGOUT_RESULT_BYTES {
        return Err(LogoutWireError::LimitExceeded);
    }
    let [code, retry_after_ms] = read_uint32_fields::<2>(payload)?;
    match (code, retry_after_ms) {
        (1, 0) => Ok(LogoutResult::Accepted),
        (2, retry_after_ms) => {
            check_retry_after(retry_after_ms)?;
            Ok(LogoutResult::InFight { retry_after_ms })
        }
        (3, 0) => Ok(LogoutResult::Busy),
        _ => Err(LogoutWireError::Malformed),
    }
}

#[cfg(test)]
#[path = "logout_tests.rs"]
mod tests;
