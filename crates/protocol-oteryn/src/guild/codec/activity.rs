//! Additive GUILD_QUERY section 4. Legacy guild query codecs keep their original sections.
use super::*;

pub const CAPABILITY_GUILD_ACTIVITY_V1: u32 = 657;
pub const ACTIVITY_PAGE_ENTRIES: usize = 30;
const SECTION: u64 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivityCursor {
    pub guild: [u8; 16],
    pub occurred_at_ms: u64,
    pub event: [u8; 16],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivityQuery {
    pub guild: [u8; 16],
    pub before: Option<ActivityCursor>,
}

/// Member-facing operation kinds. Codes follow the journal's committed action kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ActivityKind {
    Invited = 1,
    RevokedInvitation = 2,
    Joined = 3,
    Left = 4,
    Excluded = 5,
    Founded = 6,
    RankChanged = 9,
    LeadershipTransferred = 10,
    DeclinedInvitation = 12,
    Disbanded = 13,
}
impl ActivityKind {
    pub fn from_code(code: u64) -> Option<Self> {
        Some(match code {
            1 => Self::Invited,
            2 => Self::RevokedInvitation,
            3 => Self::Joined,
            4 => Self::Left,
            5 => Self::Excluded,
            6 => Self::Founded,
            9 => Self::RankChanged,
            10 => Self::LeadershipTransferred,
            12 => Self::DeclinedInvitation,
            13 => Self::Disbanded,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityEntry {
    pub occurred_at_ms: u64,
    pub event: [u8; 16],
    pub kind: ActivityKind,
    pub actor: String,
    pub target: String,
    pub previous_rank: Option<u8>,
    pub rank: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityPage {
    pub guild: [u8; 16],
    pub entries: Vec<ActivityEntry>,
    pub next: Option<ActivityCursor>,
}

pub fn is_activity(bytes: &[u8]) -> bool {
    read_varint(bytes, &mut 0).is_ok_and(|tag| tag >> 3 == SECTION)
}

fn cursor(c: ActivityCursor) -> Vec<u8> {
    let mut body = Vec::new();
    put(&mut body, 1, &c.guild);
    num(&mut body, 2, c.occurred_at_ms);
    put(&mut body, 3, &c.event);
    body
}

fn read_activity_cursor(bytes: &[u8], guild: [u8; 16]) -> WireResult<ActivityCursor> {
    let f = fields(bytes, 3)?;
    if uuid(required(&f, 1)?)? != guild {
        return bad();
    }
    let occurred_at_ms = scalar(&f, 2)?;
    if i64::try_from(occurred_at_ms).is_err() {
        return bad();
    }
    Ok(ActivityCursor {
        guild,
        occurred_at_ms,
        event: uuid(required(&f, 3)?)?,
    })
}

fn section(bytes: &[u8]) -> WireResult<&[u8]> {
    let (tag, body) = one(bytes, SECTION)?;
    if tag != SECTION {
        return bad();
    }
    Ok(body)
}

pub fn encode_activity_query(query: &ActivityQuery) -> WireResult<Vec<u8>> {
    let mut body = Vec::new();
    put(&mut body, 1, &query.guild);
    if let Some(before) = query.before {
        put(&mut body, 2, &cursor(before));
    }
    let mut out = Vec::new();
    put(&mut out, SECTION, &body);
    checked(out, decode_activity_query, MAX_GUILD_REQUEST_BYTES)
}

pub fn decode_activity_query(bytes: &[u8]) -> WireResult<ActivityQuery> {
    bounded(bytes, MAX_GUILD_REQUEST_BYTES)?;
    let f = fields(section(bytes)?, 2)?;
    let guild = uuid(required(&f, 1)?)?;
    let before = optional_bytes(&f, 2)?
        .map(|b| read_activity_cursor(b, guild))
        .transpose()?;
    Ok(ActivityQuery { guild, before })
}

pub fn encode_activity_page(page: &ActivityPage) -> WireResult<Vec<u8>> {
    if page.entries.len() > ACTIVITY_PAGE_ENTRIES {
        return Err(GuildWireError::LimitExceeded);
    }
    let mut body = Vec::new();
    put(&mut body, 1, &page.guild);
    for entry in &page.entries {
        let mut bytes = Vec::new();
        num(&mut bytes, 1, entry.occurred_at_ms);
        put(&mut bytes, 2, &entry.event);
        num(&mut bytes, 3, entry.kind as u64);
        put(&mut bytes, 4, entry.actor.as_bytes());
        put(&mut bytes, 5, entry.target.as_bytes());
        if let Some(level) = entry.previous_rank {
            num(&mut bytes, 6, u64::from(level));
        }
        if let Some(level) = entry.rank {
            num(&mut bytes, 7, u64::from(level));
        }
        put(&mut body, 2, &bytes);
    }
    if let Some(next) = page.next {
        put(&mut body, 3, &cursor(next));
    }
    let mut out = Vec::new();
    put(&mut out, SECTION, &body);
    checked(out, decode_activity_page, MAX_GUILD_RESULT_BYTES)
}

pub fn decode_activity_page(bytes: &[u8]) -> WireResult<ActivityPage> {
    bounded(bytes, MAX_GUILD_RESULT_BYTES)?;
    let (body, entries) = super::query_result::repeated(section(bytes)?, 2, ACTIVITY_PAGE_ENTRIES)?;
    let f = fields(&body, 3)?;
    let guild = uuid(required(&f, 1)?)?;
    let entries = entries
        .into_iter()
        .map(read_entry)
        .collect::<WireResult<Vec<_>>>()?;
    let next = optional_bytes(&f, 3)?
        .map(|b| read_activity_cursor(b, guild))
        .transpose()?;
    if entries
        .windows(2)
        .any(|pair| position(&pair[0]) <= position(&pair[1]))
    {
        return bad();
    }
    if let Some(next) = next
        && (entries.len() != ACTIVITY_PAGE_ENTRIES
            || entries.last().map(position) != Some((next.occurred_at_ms, next.event)))
    {
        return bad();
    }
    Ok(ActivityPage {
        guild,
        entries,
        next,
    })
}

fn position(entry: &ActivityEntry) -> (u64, [u8; 16]) {
    (entry.occurred_at_ms, entry.event)
}

fn read_entry(bytes: &[u8]) -> WireResult<ActivityEntry> {
    let f = fields(bytes, 7)?;
    let kind = ActivityKind::from_code(scalar(&f, 3)?).ok_or(GuildWireError::Malformed)?;
    let previous_rank = f[5].map(|_| rank(&f, 6)).transpose()?;
    let next_rank = f[6].map(|_| rank(&f, 7)).transpose()?;
    if matches!(
        kind,
        ActivityKind::RankChanged | ActivityKind::LeadershipTransferred
    ) != (previous_rank.is_some() && next_rank.is_some())
        || previous_rank.is_some() != next_rank.is_some()
    {
        return bad();
    }
    Ok(ActivityEntry {
        occurred_at_ms: scalar(&f, 1)?,
        event: uuid(required(&f, 2)?)?,
        kind,
        actor: text(required(&f, 4)?, 29)?,
        target: text(required(&f, 5)?, 29)?,
        previous_rank,
        rank: next_rank,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_guild_query_bytes_decode_after_activity_extension() -> WireResult<()> {
        let bytes = [0x0a, 0x00];
        assert!(!is_activity(&bytes));
        assert_eq!(decode_guild_query(&bytes)?, GuildQuery::Own);
        assert_eq!(
            decode_guild_query_result(&bytes)?,
            GuildQueryResult::Own(None)
        );
        let mut guild = [1; 16];
        guild[6] = 0x70;
        guild[8] = 0x80;
        let query = ActivityQuery {
            guild,
            before: None,
        };
        assert_eq!(
            decode_activity_query(&encode_activity_query(&query)?)?,
            query
        );
        Ok(())
    }
}
