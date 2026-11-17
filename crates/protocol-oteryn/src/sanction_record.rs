//! N009: the admitted Character's own Platform sanction record (`sanction-record-v1.md`).
//! Platform owns every decision; this is a bounded read of Game's projection.
pub const CAPABILITY_SANCTION_RECORD_V1: u32 = 497;
pub const COMMAND_TYPE_SANCTION_RECORD_QUERY: u32 = 6240;
pub const MAX_RECORDS: usize = 32;
pub const MAX_REASON_BYTES: usize = 120;
const HEADER_BYTES: usize = 2;
const RECORD_FIXED_BYTES: usize = 22;
pub const MAX_REPLY_BYTES: usize =
    HEADER_BYTES + MAX_RECORDS * (RECORD_FIXED_BYTES + MAX_REASON_BYTES);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SanctionKind {
    Mute = 1,
    Jail = 2,
    CharacterBan = 3,
    AccountBan = 4,
    Namelock = 5,
    Notation = 6,
}
impl SanctionKind {
    pub fn from_code(code: u8) -> Option<Self> {
        Some(match code {
            1 => Self::Mute,
            2 => Self::Jail,
            3 => Self::CharacterBan,
            4 => Self::AccountBan,
            5 => Self::Namelock,
            6 => Self::Notation,
            _ => return None,
        })
    }
    pub fn code(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SanctionEntry {
    pub kind: SanctionKind,
    /// `false`: account-wide; `true`: the asking Character.
    pub character_scope: bool,
    pub revoked: bool,
    pub reason_code: u16,
    pub starts_at_ms: i64,
    /// `None`: permanent, or no end for the kind.
    pub ends_at_ms: Option<i64>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SanctionRecord {
    /// More records exist than [`MAX_RECORDS`]; the oldest are omitted.
    pub more: bool,
    pub entries: Vec<SanctionEntry>,
}

fn valid_entry(entry: &SanctionEntry) -> bool {
    let scope = match entry.kind {
        SanctionKind::AccountBan => !entry.character_scope,
        SanctionKind::Notation => true,
        _ => entry.character_scope,
    };
    let end = match (entry.kind, entry.ends_at_ms) {
        (SanctionKind::Mute | SanctionKind::Jail, None) => false,
        (SanctionKind::Namelock | SanctionKind::Notation, Some(_)) => false,
        (_, Some(end)) => end > entry.starts_at_ms,
        (_, None) => true,
    };
    scope
        && end
        && entry.reason_code > 0
        && entry.starts_at_ms > 0
        && (1..=MAX_REASON_BYTES).contains(&entry.reason.len())
        && !entry.reason.chars().any(char::is_control)
}

fn valid_record(record: &SanctionRecord) -> bool {
    record.entries.len() <= MAX_RECORDS
        && (!record.more || record.entries.len() == MAX_RECORDS)
        && record.entries.iter().all(valid_entry)
        && record
            .entries
            .windows(2)
            .all(|pair| pair[0].starts_at_ms >= pair[1].starts_at_ms)
}

/// The query names nothing: it always reads the admitted session's own Character.
pub fn encode_query() -> Vec<u8> {
    Vec::new()
}
pub fn decode_query(bytes: &[u8]) -> Option<()> {
    bytes.is_empty().then_some(())
}

pub fn encode_reply(record: &SanctionRecord) -> Option<Vec<u8>> {
    if !valid_record(record) {
        return None;
    }
    let mut bytes = vec![
        u8::try_from(record.entries.len()).ok()?,
        u8::from(record.more),
    ];
    for entry in &record.entries {
        bytes.push(entry.kind.code());
        bytes.push(u8::from(entry.character_scope));
        bytes.push(u8::from(entry.revoked));
        bytes.extend(entry.reason_code.to_le_bytes());
        bytes.extend(entry.starts_at_ms.to_le_bytes());
        bytes.extend(entry.ends_at_ms.unwrap_or(0).to_le_bytes());
        bytes.push(u8::try_from(entry.reason.len()).ok()?);
        bytes.extend(entry.reason.as_bytes());
    }
    (bytes.len() <= MAX_REPLY_BYTES).then_some(bytes)
}

fn flag(byte: u8) -> Option<bool> {
    match byte {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    }
}

pub fn decode_reply(bytes: &[u8]) -> Option<SanctionRecord> {
    if bytes.len() > MAX_REPLY_BYTES {
        return None;
    }
    let (header, mut rest) = bytes.split_at_checked(HEADER_BYTES)?;
    let count = usize::from(header[0]);
    let more = flag(header[1])?;
    let mut entries = Vec::with_capacity(count.min(MAX_RECORDS));
    for _ in 0..count {
        let (fixed, tail) = rest.split_at_checked(RECORD_FIXED_BYTES)?;
        let reason_len = usize::from(fixed[21]);
        let (reason, tail) = tail.split_at_checked(reason_len)?;
        let ends = i64::from_le_bytes(fixed[13..21].try_into().ok()?);
        entries.push(SanctionEntry {
            kind: SanctionKind::from_code(fixed[0])?,
            character_scope: flag(fixed[1])?,
            revoked: flag(fixed[2])?,
            reason_code: u16::from_le_bytes(fixed[3..5].try_into().ok()?),
            starts_at_ms: i64::from_le_bytes(fixed[5..13].try_into().ok()?),
            ends_at_ms: (ends != 0).then_some(ends),
            reason: std::str::from_utf8(reason).ok()?.to_owned(),
        });
        rest = tail;
    }
    let record = SanctionRecord { more, entries };
    (rest.is_empty() && valid_record(&record)).then_some(record)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(kind: SanctionKind, starts: i64, ends: Option<i64>) -> SanctionEntry {
        SanctionEntry {
            kind,
            character_scope: kind != SanctionKind::AccountBan,
            revoked: false,
            reason_code: 7,
            starts_at_ms: starts,
            ends_at_ms: ends,
            reason: "Offensive statement".into(),
        }
    }

    #[test]
    fn sanction_record_round_trips_and_rejects_truncation_trailing_and_bad_shapes()
    -> Result<(), &'static str> {
        assert_eq!(decode_query(&encode_query()), Some(()));
        assert!(decode_query(&[0]).is_none());
        let mut revoked = entry(SanctionKind::AccountBan, 1_000, None);
        revoked.revoked = true;
        let record = SanctionRecord {
            more: false,
            entries: vec![
                entry(SanctionKind::Mute, 5_000, Some(6_000)),
                entry(SanctionKind::Notation, 4_000, None),
                revoked,
            ],
        };
        let bytes = encode_reply(&record).ok_or("encode")?;
        assert_eq!(bytes.len(), 2 + 3 * (22 + 19));
        assert_eq!(decode_reply(&bytes), Some(record));
        for end in 0..bytes.len() {
            assert!(decode_reply(&bytes[..end]).is_none());
        }
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(decode_reply(&trailing).is_none());
        let mut bad_kind = bytes.clone();
        bad_kind[2] = 7;
        assert!(decode_reply(&bad_kind).is_none());
        let mut bad_flag = bytes;
        bad_flag[1] = 2;
        assert!(decode_reply(&bad_flag).is_none());
        // Shape rules: a mute needs an end, a namelock has none, an account ban is account-wide,
        // newest first, positive reason code, bounded control-free reason.
        for bad in [
            entry(SanctionKind::Mute, 5, None),
            entry(SanctionKind::Namelock, 5, Some(6)),
            entry(SanctionKind::CharacterBan, 5, Some(5)),
            SanctionEntry {
                character_scope: true,
                ..entry(SanctionKind::AccountBan, 5, None)
            },
            SanctionEntry {
                reason_code: 0,
                ..entry(SanctionKind::Jail, 5, Some(6))
            },
            SanctionEntry {
                reason: "a\nb".into(),
                ..entry(SanctionKind::Jail, 5, Some(6))
            },
            SanctionEntry {
                reason: "x".repeat(MAX_REASON_BYTES + 1),
                ..entry(SanctionKind::Jail, 5, Some(6))
            },
        ] {
            let one = SanctionRecord {
                more: false,
                entries: vec![bad],
            };
            assert!(encode_reply(&one).is_none());
        }
        let ascending = SanctionRecord {
            more: false,
            entries: vec![
                entry(SanctionKind::Jail, 1, Some(2)),
                entry(SanctionKind::Jail, 3, Some(4)),
            ],
        };
        assert!(encode_reply(&ascending).is_none());
        Ok(())
    }

    #[test]
    fn sanction_record_reply_is_bounded_at_thirty_two_maximal_records() -> Result<(), &'static str>
    {
        let full = SanctionRecord {
            more: true,
            entries: (0..MAX_RECORDS)
                .map(|_| SanctionEntry {
                    reason: "r".repeat(MAX_REASON_BYTES),
                    ..entry(SanctionKind::CharacterBan, 9, None)
                })
                .collect(),
        };
        let bytes = encode_reply(&full).ok_or("full")?;
        assert_eq!(bytes.len(), MAX_REPLY_BYTES);
        assert_eq!(MAX_REPLY_BYTES, 4546);
        assert_eq!(decode_reply(&bytes), Some(full.clone()));
        let mut over = full;
        over.entries.push(entry(SanctionKind::Notation, 1, None));
        assert!(encode_reply(&over).is_none());
        let short = SanctionRecord {
            more: true,
            entries: vec![entry(SanctionKind::Notation, 1, None)],
        };
        assert!(encode_reply(&short).is_none());
        Ok(())
    }
}
