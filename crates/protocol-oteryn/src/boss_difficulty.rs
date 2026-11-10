//! W21 difficulty selection. A context identifies a server-owned draft, never an instance grant.
use crate::charm_wire::{
    CyclopediaWireError as Error, push_message_field, push_varint_field, read_bytes, read_varint,
};

pub const CAPABILITY_BOSS_DIFFICULTY_V1: u32 = 225;
pub const COMMAND_TYPE_BOSS_DIFFICULTY: u32 = 431;
pub const MAX_REQUEST_BYTES: usize = 1_024;
pub const MAX_RESULT_BYTES: usize = 2_048;
/// Largest sourced cap (Phosphorus); individual boss bounds are below, not scaling formulas.
pub const MAX_DIFFICULTY: u16 = 28;

/// Owner-supplied TibiaWiki Boss Difficulty System / Updates/15.30.a30dad.
/// No scaling or special-drop parameters are inferred from these limits.
pub fn boss_maximum(key: &str) -> Option<u16> {
    match key {
        "oteryn:boss.mimar_haffar" | "oteryn:creature.mimar_haffar" => Some(25),
        "oteryn:boss.maior_domus" | "oteryn:creature.maior_domus" => Some(15),
        "oteryn:boss.phosphorus" | "oteryn:creature.phosphorus" => Some(28),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    Open { boss_key: String },
    OpenWithProgress { boss_key: String },
    Start { context: [u8; 16], difficulty: u16 },
    Cancel { context: [u8; 16] },
    Preview { context: [u8; 16], difficulty: u16 },
}

impl Request {
    pub const fn context(&self) -> Option<[u8; 16]> {
        match self {
            Self::Open { .. } | Self::OpenWithProgress { .. } => None,
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

/// Actions 0/1/2 retain Crystal's Start/Cancel/Preview meaning. Action 3 is the legacy
/// draft; action 4 opts into individual progress and native Start.
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
        (Some(4), Some(boss_key), None, None) => Ok(Request::OpenWithProgress { boss_key }),
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
        Request::Open { boss_key } | Request::OpenWithProgress { boss_key } => {
            push_varint_field(
                &mut out,
                1,
                if matches!(request, Request::OpenWithProgress { .. }) {
                    4
                } else {
                    3
                },
            );
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
    DifficultyLocked = 7,
    AdmissionRefused = 8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    Window {
        context: [u8; 16],
        boss_key: String,
        boss_creature_key: String,
        content_revision: String,
        selected_difficulty: u16,
        personal_highest: u16,
        unlocked_max: u16,
        maximum_difficulty: u16,
    },
    Closed {
        context: [u8; 16],
    },
    Refused {
        context: Option<[u8; 16]>,
        reason: Refusal,
    },
}

/// Per-character victory facts are authoritative. No unsourced pity figures are sent.
pub fn encode_reply(reply: &Reply) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    match reply {
        Reply::Window {
            context,
            boss_key,
            boss_creature_key,
            content_revision,
            selected_difficulty,
            personal_highest,
            unlocked_max,
            maximum_difficulty,
        } => {
            if *context == [0; 16]
                || !valid_key(boss_key)
                || !valid_key(boss_creature_key)
                || content_revision.is_empty()
                || content_revision.len() > 512
                || *maximum_difficulty > MAX_DIFFICULTY
                || *selected_difficulty > *maximum_difficulty
                || *personal_highest > *maximum_difficulty
                || *unlocked_max > *maximum_difficulty
            {
                return Err(Error::Malformed);
            }
            push_varint_field(&mut out, 1, 0);
            push_message_field(&mut out, 2, context);
            push_message_field(&mut out, 3, boss_key.as_bytes());
            push_message_field(&mut out, 4, content_revision.as_bytes());
            push_varint_field(&mut out, 5, u64::from(*selected_difficulty));
            push_varint_field(&mut out, 6, u64::from(*maximum_difficulty));
            if *unlocked_max != 0 {
                push_varint_field(&mut out, 8, u64::from(*personal_highest));
                push_varint_field(&mut out, 9, u64::from(*unlocked_max));
                push_message_field(&mut out, 10, boss_creature_key.as_bytes());
            }
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

/// Reject noncanonical, repeated, unknown and out-of-range reply fields.
pub fn decode_reply(bytes: &[u8]) -> Result<Reply, Error> {
    if bytes.len() > MAX_RESULT_BYTES {
        return Err(Error::LimitExceeded);
    }
    let reply = parse_reply(bytes).ok_or(Error::Malformed)?;
    if encode_reply(&reply)? != bytes {
        return Err(Error::Malformed);
    }
    Ok(reply)
}
fn parse_reply(bytes: &[u8]) -> Option<Reply> {
    let mut at = 0;
    let (mut kind, mut context, mut key, mut revision, mut difficulty, mut refusal) =
        (None, None, None, None, None, None);
    let (mut highest, mut unlocked, mut maximum, mut creature) = (None, None, None, None);
    while at < bytes.len() {
        let tag = var(bytes, &mut at)?;
        match tag {
            8 => kind = Some(var(bytes, &mut at)?),
            18 => context = Some(take(bytes, &mut at)?.try_into().ok()?),
            26 => key = Some(String::from_utf8(take(bytes, &mut at)?.to_vec()).ok()?),
            34 => revision = Some(String::from_utf8(take(bytes, &mut at)?.to_vec()).ok()?),
            40 => difficulty = Some(u16::try_from(var(bytes, &mut at)?).ok()?),
            48 => {
                maximum = Some(u16::try_from(var(bytes, &mut at)?).ok()?);
            }
            56 => {
                refusal = Some(match var(bytes, &mut at)? {
                    1 => Refusal::NoAcceptedProfile,
                    2 => Refusal::ContextClosed,
                    3 => Refusal::ContextMissing,
                    4 => Refusal::ContentChanged,
                    5 => Refusal::BossUnavailable,
                    6 => Refusal::ReplayConflict,
                    7 => Refusal::DifficultyLocked,
                    8 => Refusal::AdmissionRefused,
                    _ => return None,
                })
            }
            64 => highest = Some(u16::try_from(var(bytes, &mut at)?).ok()?),
            72 => unlocked = Some(u16::try_from(var(bytes, &mut at)?).ok()?),
            82 => creature = Some(String::from_utf8(take(bytes, &mut at)?.to_vec()).ok()?),
            _ => return None,
        }
    }
    match kind? {
        0 => Some(Reply::Window {
            context: context?,
            boss_creature_key: creature.unwrap_or_else(|| key.clone().unwrap_or_default()),
            boss_key: key?,
            content_revision: revision?,
            selected_difficulty: difficulty?,
            personal_highest: highest.unwrap_or(0),
            unlocked_max: unlocked.unwrap_or(0),
            maximum_difficulty: maximum?,
        }),
        1 => Some(Reply::Closed { context: context? }),
        2 => Some(Reply::Refused {
            context,
            reason: refusal?,
        }),
        _ => None,
    }
}
fn var(b: &[u8], a: &mut usize) -> Option<u64> {
    let (mut v, mut s) = (0, 0);
    loop {
        let x = *b.get(*a)?;
        *a += 1;
        v |= u64::from(x & 127) << s;
        if x & 128 == 0 {
            return Some(v);
        }
        s += 7;
        if s > 63 {
            return None;
        }
    }
}
fn take<'a>(b: &'a [u8], a: &mut usize) -> Option<&'a [u8]> {
    let n = usize::try_from(var(b, a)?).ok()?;
    let end = a.checked_add(n)?;
    let x = b.get(*a..end)?;
    *a = end;
    Some(x)
}

#[cfg(test)]
#[path = "boss_difficulty_tests.rs"]
mod tests;
