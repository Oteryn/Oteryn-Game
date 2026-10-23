//! Dormant SPECTATE1 observer payloads. No Platform admission profile is defined here.
//! The outer candidate family is refused before player admission; capability 33 is never offered.
pub use crate::charm_wire::CyclopediaWireError as SpectateWireError;
use crate::charm_wire::{push_message_field, push_varint_field, read_bytes, read_varint, set_once};

pub const CAPABILITY_SPECTATE_V1: u32 = 33;
pub const OBSERVER_MESSAGE_TYPE: u64 = 15;
pub const COMMAND_TYPE_OBSERVE_GRANT_SET: u32 = 34;
pub const COMMAND_TYPE_OBSERVE_GRANT_REVOKE: u32 = 35;
pub const STATE_DOMAIN_OBSERVE_TARGET: u32 = 25;
pub const CODE_BYTES: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Audience {
    Code = 1,
    Contacts = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EndReason {
    Revoked = 1,
    TargetGone = 2,
    TargetMoved = 3,
    Overload = 4,
    NotObservable = 5,
    ObserverStopped = 6,
}

impl TryFrom<u64> for EndReason {
    type Error = SpectateWireError;
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Revoked),
            2 => Ok(Self::TargetGone),
            3 => Ok(Self::TargetMoved),
            4 => Ok(Self::Overload),
            5 => Ok(Self::NotObservable),
            6 => Ok(Self::ObserverStopped),
            _ => Err(SpectateWireError::Malformed),
        }
    }
}

/// A frame carries an existing world-tier payload verbatim, never a command result/envelope.
/// Projection qualification is owned by the social owner before constructing this value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpectateMessage {
    GrantSet(Audience),
    GrantRevoke,
    TargetState {
        active: bool,
        count: u8,
    },
    Stop,
    Frame {
        domain: u32,
        payload_type: u32,
        snapshot: bool,
        payload: Vec<u8>,
    },
    End(EndReason),
    NotAvailable,
    NotAllowed,
    Keepalive,
}

mod audit;
pub use audit::{ObservationKind, SpectateObservation};
mod codec;
mod envelope;
pub use envelope::ObserverEnvelope;

#[cfg(test)]
#[path = "spectate/spectate_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "spectate/frame_tests.rs"]
mod frame_tests;
