//! W21 difficulty selection. A context identifies a server-owned draft, never an instance grant.
use crate::charm_wire::{
    CyclopediaWireError as Error, push_message_field, push_varint_field, read_bytes, read_varint,
};

pub const CAPABILITY_BOSS_DIFFICULTY_V1: u32 = 225;
pub const COMMAND_TYPE_BOSS_DIFFICULTY: u32 = 431;
pub const MAX_REQUEST_BYTES: usize = 1_024;
pub const MAX_RESULT_BYTES: usize = 2_048;
/// Crystal 201189f protocolgame.cpp:11848-11852. This is a choice bound, not a scaling formula.
pub const MAX_DIFFICULTY: u16 = 25;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    Open { boss_key: String },
    Start { context: [u8; 16], difficulty: u16 },
    Cancel { context: [u8; 16] },
    Preview { context: [u8; 16], difficulty: u16 },
}

impl Request {
    pub const fn context(&self) -> Option<[u8; 16]> {
        match self {
            Self::Open { .. } => None,
            Self::Start { context, .. }
            | Self::Cancel { context }
            | Self::Preview { context, .. } => Some(*context),
        }
    }
}

pub const fn clamp_difficulty(difficulty: u16) -> u16 {
    if difficulty > MAX_DIFFICULTY {
        MAX_DIFFICULTY
    } else {
        difficulty
    }
}

fn valid_key(value: &str) -> bool {
    value.len() <= crate::boss_encounter::MAX_BOSS_KEY_BYTES
        && value
            .split_once(':')
            .is_some_and(|(namespace, local)| !namespace.is_empty() && !local.is_empty())
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b':' | b'_' | b'.' | b'-' | b'/'))
}

/// Actions 0/1/2 retain Crystal's Start/Cancel/Preview meaning. Action 3 requests a window.
/// Unknown, repeated, truncated and action-incompatible fields are rejected.
pub fn decode_request(payload: &[u8]) -> Result<Request, Error> {
    if payload.len() > MAX_REQUEST_BYTES {
        return Err(Error::LimitExceeded);
    }
    let (mut action, mut key, mut context, mut difficulty) = (None, None, None, None);
    let mut at = 0;
    while at < payload.len() {
        match read_varint(payload, &mut at)? {
            8 if action.is_none() => action = Some(read_varint(payload, &mut at)?),
            18 if key.is_none() => {
                let value = std::str::from_utf8(read_bytes(payload, &mut at)?)
                    .map_err(|_| Error::Malformed)?;
                if !valid_key(value) {
                    return Err(Error::Malformed);
                }
                key = Some(value.to_owned());
            }
            26 if context.is_none() => {
                let value: [u8; 16] = read_bytes(payload, &mut at)?
                    .try_into()
                    .map_err(|_| Error::Malformed)?;
                if value == [0; 16] {
                    return Err(Error::Malformed);
                }
                context = Some(value);
            }
            32 if difficulty.is_none() => {
                difficulty = Some(
                    u16::try_from(read_varint(payload, &mut at)?).map_err(|_| Error::Malformed)?,
                );
            }
            _ => return Err(Error::Malformed),
        }
    }
    match (action, key, context, difficulty) {
        (Some(3), Some(boss_key), None, None) => Ok(Request::Open { boss_key }),
        (Some(0), None, Some(context), Some(difficulty)) => Ok(Request::Start {
            context,
            difficulty: clamp_difficulty(difficulty),
        }),
        (Some(1), None, Some(context), None) => Ok(Request::Cancel { context }),
        (Some(2), None, Some(context), Some(difficulty)) => Ok(Request::Preview {
            context,
            difficulty: clamp_difficulty(difficulty),
        }),
        _ => Err(Error::Malformed),
    }
}

pub fn encode_request(request: &Request) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    match request {
        Request::Open { boss_key } => {
            push_varint_field(&mut out, 1, 3);
            push_message_field(&mut out, 2, boss_key.as_bytes());
        }
        Request::Start {
            context,
            difficulty,
        }
        | Request::Preview {
            context,
            difficulty,
        } => {
            push_varint_field(
                &mut out,
                1,
                if matches!(request, Request::Start { .. }) {
                    0
                } else {
                    2
                },
            );
            push_message_field(&mut out, 3, context);
            push_varint_field(&mut out, 4, u64::from(*difficulty));
        }
        Request::Cancel { context } => {
            push_varint_field(&mut out, 1, 1);
            push_message_field(&mut out, 3, context);
        }
    }
    decode_request(&out)?;
    Ok(out)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Refusal {
    NoAcceptedProfile = 1,
    ContextClosed = 2,
    ContextMissing = 3,
    ContentChanged = 4,
    BossUnavailable = 5,
    ReplayConflict = 6,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    Window {
        context: [u8; 16],
        boss_key: String,
        content_revision: String,
        selected_difficulty: u16,
    },
    Closed {
        context: [u8; 16],
    },
    Refused {
        context: Option<[u8; 16]>,
        reason: Refusal,
    },
}

/// Only known window facts are sent. Uncaptured race/group/personal-highest, banners,
/// bad-luck and modifier figures are absent rather than copied from the donor's god test.
pub fn encode_reply(reply: &Reply) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    match reply {
        Reply::Window {
            context,
            boss_key,
            content_revision,
            selected_difficulty,
        } => {
            if *context == [0; 16]
                || !valid_key(boss_key)
                || content_revision.is_empty()
                || content_revision.len() > 512
                || *selected_difficulty > MAX_DIFFICULTY
            {
                return Err(Error::Malformed);
            }
            push_varint_field(&mut out, 1, 0);
            push_message_field(&mut out, 2, context);
            push_message_field(&mut out, 3, boss_key.as_bytes());
            push_message_field(&mut out, 4, content_revision.as_bytes());
            push_varint_field(&mut out, 5, u64::from(*selected_difficulty));
            push_varint_field(&mut out, 6, u64::from(MAX_DIFFICULTY));
        }
        Reply::Closed { context } => {
            if *context == [0; 16] {
                return Err(Error::Malformed);
            }
            push_varint_field(&mut out, 1, 1);
            push_message_field(&mut out, 2, context);
        }
        Reply::Refused { context, reason } => {
            push_varint_field(&mut out, 1, 2);
            if let Some(context) = context {
                if *context == [0; 16] {
                    return Err(Error::Malformed);
                }
                push_message_field(&mut out, 2, context);
            }
            push_varint_field(&mut out, 7, *reason as u64);
        }
    }
    if out.len() > MAX_RESULT_BYTES {
        return Err(Error::LimitExceeded);
    }
    Ok(out)
}

#[cfg(test)]
#[path = "boss_difficulty_tests.rs"]
mod tests;
