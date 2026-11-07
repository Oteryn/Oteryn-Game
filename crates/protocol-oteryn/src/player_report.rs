//! PARITY-REPORT-1 player bug and rule-violation reports. Strict bounded codec; the server
//! owns identity, position, rate and queue truth. Canary report enums keep their ordinals.
use crate::charm_wire::{
    CyclopediaWireError as Error, push_message_field, push_varint_field, read_bytes, read_varint,
    set_once,
};

pub const CAPABILITY_PLAYER_REPORT_V1: u32 = 76;
pub const COMMAND_TYPE_PLAYER_REPORT: u32 = 93;
/// REPORT1-RL-01: UTF-8 comment bytes.
pub const MAX_COMMENT_BYTES: usize = 1024;
/// REPORT1-RL-02: reported character name bytes, the character-name bound.
pub const MAX_TARGET_BYTES: usize = 29;
pub const MAX_REQUEST_BYTES: usize = 1088;
pub const MAX_REPLY_BYTES: usize = 24;
pub const KIND_BUG: u8 = 1;
pub const KIND_RULE_VIOLATION: u8 = 2;
/// Canary BUG_CATEGORY_MAP; only a map bug carries the client map position.
pub const BUG_CATEGORY_MAP: u8 = 0;
pub const MAX_BUG_CATEGORY: u8 = 3;
/// Canary REPORT_REASON_NAMEINAPPROPRIATE..REPORT_REASON_SERVICEAGREEMENT.
pub const MAX_REPORT_REASON: u8 = 20;
/// Canary REPORT_TYPE_NAME, REPORT_TYPE_STATEMENT, REPORT_TYPE_BOT.
pub const MAX_REPORT_ACTION: u8 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerReportRequest {
    /// 1 bug, 2 rule violation; any other value is refused by the owner.
    pub kind: u8,
    /// Bug category; zero for a rule violation.
    pub category: u8,
    /// Rule-violation reason; zero for a bug.
    pub reason: u8,
    /// Rule-violation report type (name, statement, bot); zero for a bug.
    pub action: u8,
    /// Reported character name; empty for a bug.
    pub target: String,
    pub comment: String,
    /// Client map position of a map bug; zero otherwise.
    pub map_x: u16,
    pub map_y: u16,
    pub map_floor: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PlayerReportCode {
    Accepted = 1,
    /// Unknown kind, reason, action or category, or a field set the kind does not carry.
    Invalid = 2,
    SelfReport = 3,
    /// Canary `hasPendingReport`: same reporter, reported character and report type.
    AlreadyPending = 4,
    RateLimited = 5,
}

impl PlayerReportCode {
    pub fn from_u8(code: u8) -> Option<Self> {
        Some(match code {
            1 => Self::Accepted,
            2 => Self::Invalid,
            3 => Self::SelfReport,
            4 => Self::AlreadyPending,
            5 => Self::RateLimited,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerReportReply {
    pub code: PlayerReportCode,
    /// Server-minted queue number; nonzero exactly when accepted.
    pub report: u64,
    /// Seconds until the next report is admitted; nonzero exactly when rate limited.
    pub wait_seconds: u32,
}

/// Character-name syntax, identical to the installed `game_character_is_name`.
pub fn valid_character_name(name: &str) -> bool {
    (2..=MAX_TARGET_BYTES).contains(&name.len())
        && name
            .split(' ')
            .all(|word| !word.is_empty() && word.bytes().all(|b| b.is_ascii_alphabetic()))
}

/// The semantic shape a kind admits. Codec bounds hold before this is reached.
pub fn validate_player_report(r: &PlayerReportRequest) -> Result<(), PlayerReportCode> {
    let no_map = r.map_x == 0 && r.map_y == 0 && r.map_floor == 0;
    let valid = match r.kind {
        KIND_BUG => {
            r.reason == 0
                && r.action == 0
                && r.target.is_empty()
                && r.category <= MAX_BUG_CATEGORY
                && (r.category == BUG_CATEGORY_MAP || no_map)
        }
        KIND_RULE_VIOLATION => {
            r.category == 0
                && no_map
                && r.reason <= MAX_REPORT_REASON
                && r.action <= MAX_REPORT_ACTION
                && valid_character_name(&r.target)
        }
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(PlayerReportCode::Invalid)
    }
}

pub fn encode_player_report_request(r: &PlayerReportRequest) -> Result<Vec<u8>, Error> {
    if r.target.len() > MAX_TARGET_BYTES || r.comment.len() > MAX_COMMENT_BYTES {
        return Err(Error::LimitExceeded);
    }
    let mut out = Vec::new();
    for (field, value) in [(1, r.kind), (2, r.category), (3, r.reason), (4, r.action)] {
        push_varint_field(&mut out, field, u64::from(value));
    }
    push_message_field(&mut out, 5, r.target.as_bytes());
    push_message_field(&mut out, 6, r.comment.as_bytes());
    push_varint_field(&mut out, 7, u64::from(r.map_x));
    push_varint_field(&mut out, 8, u64::from(r.map_y));
    push_varint_field(&mut out, 9, u64::from(r.map_floor));
    Ok(out)
}

fn text(bytes: &[u8], at: &mut usize, cap: usize) -> Result<String, Error> {
    let raw = read_bytes(bytes, at)?;
    if raw.len() > cap {
        return Err(Error::LimitExceeded);
    }
    Ok(std::str::from_utf8(raw)
        .map_err(|_| Error::Malformed)?
        .to_owned())
}

pub fn decode_player_report_request(bytes: &[u8]) -> Result<PlayerReportRequest, Error> {
    if bytes.len() > MAX_REQUEST_BYTES {
        return Err(Error::LimitExceeded);
    }
    let mut small = [None; 4];
    let mut position = [None; 3];
    let (mut target, mut comment) = (None, None);
    let mut at = 0;
    while at < bytes.len() {
        match read_varint(bytes, &mut at)? {
            tag @ (8 | 16 | 24 | 32) => set_once(
                &mut small[(tag >> 3) as usize - 1],
                u8::try_from(read_varint(bytes, &mut at)?).map_err(|_| Error::Malformed)?,
            )?,
            42 => set_once(&mut target, text(bytes, &mut at, MAX_TARGET_BYTES)?)?,
            50 => set_once(&mut comment, text(bytes, &mut at, MAX_COMMENT_BYTES)?)?,
            tag @ (56 | 64 | 72) => {
                let value = read_varint(bytes, &mut at)?;
                let limit = if tag == 72 {
                    u64::from(u8::MAX)
                } else {
                    u64::from(u16::MAX)
                };
                if value > limit {
                    return Err(Error::Malformed);
                }
                set_once(&mut position[(tag >> 3) as usize - 7], value as u16)?;
            }
            _ => return Err(Error::Malformed),
        }
    }
    let field = |slot: Option<u8>| slot.ok_or(Error::Malformed);
    let r = PlayerReportRequest {
        kind: field(small[0])?,
        category: field(small[1])?,
        reason: field(small[2])?,
        action: field(small[3])?,
        target: target.ok_or(Error::Malformed)?,
        comment: comment.ok_or(Error::Malformed)?,
        map_x: position[0].ok_or(Error::Malformed)?,
        map_y: position[1].ok_or(Error::Malformed)?,
        map_floor: position[2].ok_or(Error::Malformed)? as u8,
    };
    encode_player_report_request(&r)?;
    Ok(r)
}

pub fn encode_player_report_reply(r: &PlayerReportReply) -> Result<Vec<u8>, Error> {
    if (r.code == PlayerReportCode::Accepted) != (r.report != 0)
        || r.report > i64::MAX as u64
        || (r.code == PlayerReportCode::RateLimited) != (r.wait_seconds != 0)
    {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    push_varint_field(&mut out, 1, r.code as u64);
    push_varint_field(&mut out, 2, r.report);
    push_varint_field(&mut out, 3, u64::from(r.wait_seconds));
    Ok(out)
}

pub fn decode_player_report_reply(bytes: &[u8]) -> Result<PlayerReportReply, Error> {
    if bytes.len() > MAX_REPLY_BYTES {
        return Err(Error::LimitExceeded);
    }
    let mut fields = [None; 3];
    let mut at = 0;
    while at < bytes.len() {
        match read_varint(bytes, &mut at)? {
            tag @ (8 | 16 | 24) => set_once(
                &mut fields[(tag >> 3) as usize - 1],
                read_varint(bytes, &mut at)?,
            )?,
            _ => return Err(Error::Malformed),
        }
    }
    let [Some(code), Some(report), Some(wait)] = fields else {
        return Err(Error::Malformed);
    };
    let r = PlayerReportReply {
        code: u8::try_from(code)
            .ok()
            .and_then(PlayerReportCode::from_u8)
            .ok_or(Error::Malformed)?,
        report,
        wait_seconds: u32::try_from(wait).map_err(|_| Error::Malformed)?,
    };
    encode_player_report_reply(&r)?;
    Ok(r)
}

#[cfg(test)]
#[path = "player_report_tests.rs"]
mod tests;
