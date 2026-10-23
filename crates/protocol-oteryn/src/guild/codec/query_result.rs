use super::*;
fn summary(s: &GuildSummary) -> Vec<u8> {
    let mut out = Vec::new();
    put(&mut out, 1, &s.guild);
    put(&mut out, 2, s.name.as_bytes());
    num(&mut out, 3, u64::from(s.state));
    num(&mut out, 4, s.revision);
    num(&mut out, 5, u64::from(s.own_rank));
    put(&mut out, 6, s.title.as_bytes());
    put(&mut out, 7, s.message.as_bytes());
    for name in &s.ranks {
        put(&mut out, 8, name.as_bytes());
    }
    out
}
// Repeated fields are parsed separately with count/entry bounds before allocation.
fn repeated(bytes: &[u8], repeated_tag: u64, count: usize) -> WireResult<(Vec<u8>, Vec<&[u8]>)> {
    let mut singular = Vec::new();
    let mut entries = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let start = at;
        let tag = read_varint(bytes, &mut at)?;
        if tag == repeated_tag * 8 + 2 {
            if entries.len() == count {
                return Err(GuildWireError::LimitExceeded);
            }
            let b = read_bytes(bytes, &mut at)?;
            bounded(b, MAX_GUILD_ENTRY_BYTES)?;
            entries.push(b);
        } else {
            match tag & 7 {
                0 => {
                    read_varint(bytes, &mut at)?;
                }
                2 => {
                    read_bytes(bytes, &mut at)?;
                }
                _ => return bad(),
            }
            singular.extend_from_slice(&bytes[start..at]);
        }
    }
    Ok((singular, entries))
}
fn read_summary(bytes: &[u8]) -> WireResult<GuildSummary> {
    let (b, ranks) = repeated(bytes, 8, 20)?;
    let f = fields(&b, 7)?;
    if ranks.len() < 3 {
        return bad();
    }
    let state = scalar(&f, 3)?;
    if state > 4 {
        return bad();
    }
    let s = GuildSummary {
        guild: uuid(required(&f, 1)?)?,
        name: text(required(&f, 2)?, 29)?,
        state: state as u8,
        revision: scalar(&f, 4)?,
        own_rank: rank(&f, 5)?,
        title: text(required(&f, 6)?, 116)?,
        message: text(required(&f, 7)?, 1020)?,
        ranks: ranks
            .into_iter()
            .map(|b| text(b, 128))
            .collect::<WireResult<_>>()?,
    };
    if usize::from(s.own_rank) > s.ranks.len()
        || s.title.chars().count() > 29
        || s.message.chars().count() > 255
        || s.ranks.iter().any(String::is_empty)
    {
        return bad();
    }
    Ok(s)
}
fn member(m: &GuildMember) -> Vec<u8> {
    let mut b = Vec::new();
    put(&mut b, 1, &m.character);
    put(&mut b, 2, m.name.as_bytes());
    num(&mut b, 3, u64::from(m.level));
    put(&mut b, 4, m.title.as_bytes());
    b
}
fn read_member(bytes: &[u8]) -> WireResult<GuildMember> {
    let f = fields(bytes, 4)?;
    let m = GuildMember {
        character: uuid(required(&f, 1)?)?,
        name: text(required(&f, 2)?, 29)?,
        level: rank(&f, 3)?,
        title: text(required(&f, 4)?, 116)?,
    };
    if m.title.chars().count() > 29 {
        return bad();
    }
    Ok(m)
}
fn invitation(i: &GuildInvitation) -> Vec<u8> {
    let mut b = Vec::new();
    put(&mut b, 1, &i.guild);
    put(&mut b, 2, i.name.as_bytes());
    num(&mut b, 3, i.expires_at_unix_ms);
    b
}
pub fn encode_guild_query_result(r: &GuildQueryResult) -> WireResult<Vec<u8>> {
    let mut out = Vec::new();
    let mut b = Vec::new();
    let tag = match r {
        GuildQueryResult::Own(s) => {
            if let Some(s) = s {
                bounded(s.name.as_bytes(), 29)?;
                bounded(s.title.as_bytes(), 116)?;
                bounded(s.message.as_bytes(), 1020)?;
                if s.ranks.len() > 20 {
                    return Err(GuildWireError::LimitExceeded);
                }
                for rank in &s.ranks {
                    bounded(rank.as_bytes(), 128)?;
                }
                b = summary(s);
            }
            1
        }
        GuildQueryResult::Roster {
            guild,
            revision,
            members,
            next,
        } => {
            if members.len() > MAX_GUILD_PAGE_ENTRIES {
                return Err(GuildWireError::LimitExceeded);
            }
            put(&mut b, 1, guild);
            num(&mut b, 2, *revision);
            for m in members {
                bounded(m.name.as_bytes(), 29)?;
                bounded(m.title.as_bytes(), 116)?;
                put(&mut b, 3, &member(m));
            }
            if let Some(c) = next {
                put(&mut b, 4, &cursor(c));
            }
            2
        }
        GuildQueryResult::Invitations(entries) => {
            if entries.len() > 50 {
                return Err(GuildWireError::LimitExceeded);
            }
            for i in entries {
                bounded(i.name.as_bytes(), 29)?;
                put(&mut b, 1, &invitation(i));
            }
            3
        }
    };
    put(&mut out, tag, &b);
    checked(out, decode_guild_query_result, MAX_GUILD_RESULT_BYTES)
}
pub fn decode_guild_query_result(bytes: &[u8]) -> WireResult<GuildQueryResult> {
    bounded(bytes, MAX_GUILD_RESULT_BYTES)?;
    let (tag, b) = one(bytes, 3)?;
    match tag {
        1 => Ok(GuildQueryResult::Own(if b.is_empty() {
            None
        } else {
            Some(read_summary(b)?)
        })),
        2 => {
            let (b, entries) = repeated(b, 3, MAX_GUILD_PAGE_ENTRIES)?;
            let f = fields(&b, 4)?;
            if f[2].is_some() {
                return bad();
            }
            let guild = uuid(required(&f, 1)?)?;
            let members = entries
                .into_iter()
                .map(read_member)
                .collect::<WireResult<Vec<_>>>()?;
            if members
                .windows(2)
                .any(|w| (w[0].level, w[0].character) >= (w[1].level, w[1].character))
            {
                return bad();
            }
            let next = optional_bytes(&f, 4)?
                .map(|b| read_cursor(b, guild))
                .transpose()?;
            if let Some(c) = next
                && (members.len() != MAX_GUILD_PAGE_ENTRIES
                    || members.last().map(|m| (m.level, m.character))
                        != Some((c.level, c.character)))
            {
                return bad();
            }
            Ok(GuildQueryResult::Roster {
                guild,
                revision: scalar(&f, 2)?,
                members,
                next,
            })
        }
        3 => {
            let (rest, entries) = repeated(b, 1, 50)?;
            if !rest.is_empty() {
                return bad();
            }
            let entries = entries
                .into_iter()
                .map(|b| {
                    let f = fields(b, 3)?;
                    Ok(GuildInvitation {
                        guild: uuid(required(&f, 1)?)?,
                        name: text(required(&f, 2)?, 29)?,
                        expires_at_unix_ms: scalar(&f, 3)?,
                    })
                })
                .collect::<WireResult<Vec<_>>>()?;
            if entries.windows(2).any(|w| w[0].guild >= w[1].guild) {
                return bad();
            }
            Ok(GuildQueryResult::Invitations(entries))
        }
        _ => bad(),
    }
}
