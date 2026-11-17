//! W21 difficulty selection. A context identifies a server-owned draft, never an instance grant.
use crate::charm_wire::{
    CyclopediaWireError as Error, push_message_field, push_varint_field, read_bytes, read_varint,
};

pub const CAPABILITY_BOSS_DIFFICULTY_V1: u32 = 225;
/// A progress window of a pity boss carries the Moonsilver first-drop pity tracker as
/// fields 11-13 ([`PityTracker`]).
pub const CAPABILITY_BOSS_DIFFICULTY_PITY_V1: u32 = 492;
pub const COMMAND_TYPE_BOSS_DIFFICULTY: u32 = 431;
pub const MAX_REQUEST_BYTES: usize = 1_024;
pub const MAX_RESULT_BYTES: usize = 2_048;
/// Largest sourced cap (Phosphorus); individual boss bounds are below, not scaling formulas.
pub const MAX_DIFFICULTY: u16 = 28;
/// The pity bonus over the base drop chance, in permille of base, never exceeds this (3x base).
pub const MAX_PITY_BONUS_PERMILLE: u16 = 2_000;

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

/// One character's Moonsilver first-drop pity for the window's boss (capability 492).
/// `bonus_permille` is the current bonus over the item's base chance, in permille of base;
/// an inactive tracker (first drop received) has no bonus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PityTracker {
    pub kills_without_drop: u16,
    pub bonus_permille: u16,
    pub tracker_active: bool,
}

/// Per-character victory facts are authoritative. The legacy reply carries no pity fields.
pub fn encode_reply(reply: &Reply) -> Result<Vec<u8>, Error> {
    encode_reply_with_pity(reply, None)
}

/// The reply as a session that selected capability 492 receives it: a progress window
/// (`unlocked_max != 0`) of a pity boss appends fields 11 kills_without_drop, 12
/// bonus_permille and 13 tracker_active. Any other reply takes no tracker.
pub fn encode_reply_with_pity(reply: &Reply, pity: Option<&PityTracker>) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    if let Some(pity) = pity
        && (!matches!(reply, Reply::Window { unlocked_max, .. } if *unlocked_max != 0)
            || pity.bonus_permille > MAX_PITY_BONUS_PERMILLE
            || (!pity.tracker_active && pity.bonus_permille != 0))
    {
        return Err(Error::Malformed);
    }
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
            if let Some(pity) = pity {
                push_varint_field(&mut out, 11, u64::from(pity.kills_without_drop));
                push_varint_field(&mut out, 12, u64::from(pity.bonus_permille));
                push_varint_field(&mut out, 13, u64::from(pity.tracker_active));
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
    let (reply, _) = parse_reply(bytes, false).ok_or(Error::Malformed)?;
    if encode_reply(&reply)? != bytes {
        return Err(Error::Malformed);
    }
    Ok(reply)
}

/// A reply as a session that selected capability 492 receives it: the tracker is present
/// exactly when fields 11-13 are. Noncanonical and partial trackers are rejected.
pub fn decode_reply_with_pity(bytes: &[u8]) -> Result<(Reply, Option<PityTracker>), Error> {
    if bytes.len() > MAX_RESULT_BYTES {
        return Err(Error::LimitExceeded);
    }
    let (reply, pity) = parse_reply(bytes, true).ok_or(Error::Malformed)?;
    if encode_reply_with_pity(&reply, pity.as_ref())? != bytes {
        return Err(Error::Malformed);
    }
    Ok((reply, pity))
}

fn parse_reply(bytes: &[u8], with_pity: bool) -> Option<(Reply, Option<PityTracker>)> {
    let mut at = 0;
    let (mut kind, mut context, mut key, mut revision, mut difficulty, mut refusal) =
        (None, None, None, None, None, None);
    let (mut highest, mut unlocked, mut maximum, mut creature) = (None, None, None, None);
    let (mut kills, mut bonus, mut active) = (None, None, None);
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
            88 if with_pity => kills = Some(u16::try_from(var(bytes, &mut at)?).ok()?),
            96 if with_pity => bonus = Some(u16::try_from(var(bytes, &mut at)?).ok()?),
            104 if with_pity => {
                active = Some(match var(bytes, &mut at)? {
                    0 => false,
                    1 => true,
                    _ => return None,
                })
            }
            _ => return None,
        }
    }
    let pity = match (kills, bonus, active) {
        (Some(kills_without_drop), Some(bonus_permille), Some(tracker_active)) => {
            Some(PityTracker {
                kills_without_drop,
                bonus_permille,
                tracker_active,
            })
        }
        (None, None, None) => None,
        _ => return None,
    };
    let reply = match kind? {
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
    }?;
    Some((reply, pity))
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
