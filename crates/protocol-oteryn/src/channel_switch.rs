//! chsw / GAME-CHANNEL-01 §7-§10: in-game Channel switch (`channel-switch-v1.md`).
//!
//! `CHANNEL_SWITCH_STATUS` reads the locks and the cooldown that apply to any switch now.
//! `CHANNEL_SWITCH_REQUEST` names one target Channel of the current World. An accepted request
//! is the safe exit from the current Channel: the server has released the session and closes the
//! transport after the result, and the client enters the target by a fresh admission. A switch is
//! never a teleport; the final admission revalidates the cooldown and the locks.

/// Registered capability `CHANNEL_SWITCH_V1`.
pub const CAPABILITY_CHANNEL_SWITCH_V1: u32 = 512;
/// Registered command type `CHANNEL_SWITCH_STATUS` (capability 512): empty request.
pub const COMMAND_TYPE_CHANNEL_SWITCH_STATUS: u32 = 6247;
/// Registered command type `CHANNEL_SWITCH_REQUEST` (capability 512): the target ChannelId.
pub const COMMAND_TYPE_CHANNEL_SWITCH_REQUEST: u32 = 6248;
/// Five minutes between voluntary switches, timed by the server clock (owner, 2026-11-17).
pub const COOLDOWN_MS: u32 = 300_000;
/// The request: one 16-byte ChannelId.
pub const REQUEST_BYTES: usize = 16;
/// Both results: one lock byte and the cooldown left, big-endian milliseconds.
pub const RESULT_BYTES: usize = 5;

/// Why a switch cannot start now (§8). `None` with no cooldown left means a switch may start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwitchLock {
    None = 0,
    /// In fight, or a PvP logout block.
    Combat = 1,
    /// A red or black skull cannot leave for a Channel that is not Open PvP.
    Skull = 2,
    /// An open direct trade.
    Trade = 3,
    /// An unresolved item, currency or value commit.
    ValueTransaction = 4,
    /// Boss, raid or event participation (an instance scope).
    Event = 5,
    /// A house, instance or travel transition in progress.
    Transition = 6,
    /// A pending checkpoint, handoff or authority transition; retry shortly.
    Checkpoint = 7,
    /// The target is not a current Channel of this World.
    Destination = 8,
    /// The cooldown since the last switch is still running.
    Cooldown = 9,
    /// The target is the current Channel.
    SameChannel = 10,
    /// A tile where logging out is not allowed.
    NoLogout = 11,
}

impl SwitchLock {
    pub fn from_code(code: u8) -> Option<Self> {
        Some(match code {
            0 => Self::None,
            1 => Self::Combat,
            2 => Self::Skull,
            3 => Self::Trade,
            4 => Self::ValueTransaction,
            5 => Self::Event,
            6 => Self::Transition,
            7 => Self::Checkpoint,
            8 => Self::Destination,
            9 => Self::Cooldown,
            10 => Self::SameChannel,
            11 => Self::NoLogout,
            _ => return None,
        })
    }

    pub fn code(self) -> u8 {
        self as u8
    }
}

/// The result of both commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwitchStatus {
    pub lock: SwitchLock,
    /// 0..=`COOLDOWN_MS`; non-zero only with `lock` `None` or `Cooldown`.
    pub cooldown_left_ms: u32,
}

impl SwitchStatus {
    pub const READY: Self = Self {
        lock: SwitchLock::None,
        cooldown_left_ms: 0,
    };

    fn valid(self) -> bool {
        self.cooldown_left_ms <= COOLDOWN_MS
            && (self.cooldown_left_ms == 0
                || matches!(self.lock, SwitchLock::None | SwitchLock::Cooldown))
            && (self.lock != SwitchLock::Cooldown || self.cooldown_left_ms > 0)
    }
}

/// `None` unless the status request payload is empty.
pub fn decode_status_request(payload: &[u8]) -> Option<()> {
    payload.is_empty().then_some(())
}

pub fn encode_request(target_channel: [u8; 16]) -> Vec<u8> {
    target_channel.to_vec()
}

/// The target ChannelId, or `None` for any other length.
pub fn decode_request(payload: &[u8]) -> Option<[u8; 16]> {
    payload.try_into().ok()
}

/// `None` for a status the wire refuses (a cooldown above five minutes, or with another lock).
pub fn encode_status(status: SwitchStatus) -> Option<Vec<u8>> {
    if !status.valid() {
        return None;
    }
    let mut bytes = Vec::with_capacity(RESULT_BYTES);
    bytes.push(status.lock.code());
    bytes.extend_from_slice(&status.cooldown_left_ms.to_be_bytes());
    Some(bytes)
}

pub fn decode_status(payload: &[u8]) -> Option<SwitchStatus> {
    let [lock, a, b, c, d]: [u8; RESULT_BYTES] = payload.try_into().ok()?;
    let status = SwitchStatus {
        lock: SwitchLock::from_code(lock)?,
        cooldown_left_ms: u32::from_be_bytes([a, b, c, d]),
    };
    status.valid().then_some(status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_round_trips_and_refuses_values_outside_the_wire() {
        for status in [
            SwitchStatus::READY,
            SwitchStatus {
                lock: SwitchLock::Cooldown,
                cooldown_left_ms: COOLDOWN_MS,
            },
            SwitchStatus {
                lock: SwitchLock::None,
                cooldown_left_ms: 1,
            },
            SwitchStatus {
                lock: SwitchLock::Skull,
                cooldown_left_ms: 0,
            },
        ] {
            let bytes = encode_status(status);
            assert_eq!(bytes.as_ref().map(Vec::len), Some(RESULT_BYTES));
            assert_eq!(bytes.as_deref().and_then(decode_status), Some(status));
        }
        for refused in [
            SwitchStatus {
                lock: SwitchLock::Cooldown,
                cooldown_left_ms: 0,
            },
            SwitchStatus {
                lock: SwitchLock::None,
                cooldown_left_ms: COOLDOWN_MS + 1,
            },
            SwitchStatus {
                lock: SwitchLock::Trade,
                cooldown_left_ms: 5,
            },
        ] {
            assert_eq!(encode_status(refused), None);
        }
        assert_eq!(decode_status(&[12, 0, 0, 0, 0]), None);
        assert_eq!(decode_status(&[0, 0, 0, 0]), None);
    }

    #[test]
    fn requests_carry_exactly_one_channel_id() {
        let target = [7; 16];
        assert_eq!(decode_request(&encode_request(target)), Some(target));
        assert_eq!(decode_request(&[7; 15]), None);
        assert_eq!(decode_request(&[7; 17]), None);
        assert_eq!(decode_status_request(&[]), Some(()));
        assert_eq!(decode_status_request(&[0]), None);
    }
}
