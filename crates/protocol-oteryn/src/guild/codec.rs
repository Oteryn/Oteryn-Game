use super::*;
#[derive(Clone, Copy)]
enum Field<'a> {
    Bytes(&'a [u8]),
    Number(u64),
}
type Fields<'a> = [Option<Field<'a>>; 8];
fn bad<T>() -> WireResult<T> {
    Err(GuildWireError::Malformed)
}
fn bounded(bytes: &[u8], max: usize) -> WireResult<()> {
    if bytes.len() > max {
        return Err(GuildWireError::LimitExceeded);
    }
    Ok(())
}
// Strict protobuf fields: unknown tags, wrong wire types and duplicates fail closed.
fn fields(bytes: &[u8], count: usize) -> WireResult<Fields<'_>> {
    let mut out = [None; 8];
    let mut at = 0;
    while at < bytes.len() {
        let tag = read_varint(bytes, &mut at)?;
        let index = usize::try_from(tag >> 3).map_err(|_| GuildWireError::Malformed)?;
        if index == 0 || index > count {
            return bad();
        }
        let field = match tag & 7 {
            0 => Field::Number(read_varint(bytes, &mut at)?),
            2 => Field::Bytes(read_bytes(bytes, &mut at)?),
            _ => return bad(),
        };
        set_once(&mut out[index - 1], field)?;
    }
    Ok(out)
}
fn required<'a>(f: &Fields<'a>, i: usize) -> WireResult<&'a [u8]> {
    match f[i - 1] {
        Some(Field::Bytes(b)) => Ok(b),
        _ => bad(),
    }
}
fn scalar(f: &Fields<'_>, i: usize) -> WireResult<u64> {
    match f[i - 1] {
        Some(Field::Number(n)) if n > 0 => Ok(n),
        _ => bad(),
    }
}
fn optional_bytes<'a>(f: &Fields<'a>, i: usize) -> WireResult<Option<&'a [u8]>> {
    if f[i - 1].is_none() {
        Ok(None)
    } else {
        required(f, i).map(Some)
    }
}
fn uuid(bytes: &[u8]) -> WireResult<[u8; 16]> {
    let id: [u8; 16] = bytes.try_into().map_err(|_| GuildWireError::Malformed)?;
    if id[6] >> 4 != 7 || id[8] >> 6 != 2 {
        return bad();
    }
    Ok(id)
}
fn rank(f: &Fields<'_>, i: usize) -> WireResult<u8> {
    let n = scalar(f, i)?;
    if n > 20 {
        return bad();
    }
    Ok(n as u8)
}
fn text(bytes: &[u8], max: usize) -> WireResult<String> {
    bounded(bytes, max)?;
    let s = std::str::from_utf8(bytes).map_err(|_| GuildWireError::Malformed)?;
    if s.contains('\0') {
        return bad();
    }
    Ok(s.to_owned())
}
fn put(out: &mut Vec<u8>, i: u64, bytes: &[u8]) {
    push_message_field(out, i, bytes);
}
fn num(out: &mut Vec<u8>, i: u64, n: u64) {
    push_varint_field(out, i, n);
}
fn one(bytes: &[u8], max_tag: u64) -> WireResult<(u64, &[u8])> {
    let mut at = 0;
    let tag = read_varint(bytes, &mut at)?;
    if tag & 7 != 2 || tag >> 3 == 0 || tag >> 3 > max_tag {
        return bad();
    }
    let body = read_bytes(bytes, &mut at)?;
    if at != bytes.len() {
        return bad();
    }
    Ok((tag >> 3, body))
}
fn checked<T>(
    out: Vec<u8>,
    decode: impl FnOnce(&[u8]) -> WireResult<T>,
    max: usize,
) -> WireResult<Vec<u8>> {
    bounded(&out, max)?;
    decode(&out)?;
    Ok(out)
}
pub fn encode_guild_intent(intent: &GuildIntent) -> WireResult<Vec<u8>> {
    let mut b = Vec::new();
    let tag = match intent {
        GuildIntent::Found { name } => {
            bounded(name.as_bytes(), 29)?;
            put(&mut b, 1, name.as_bytes());
            1
        }
        GuildIntent::Invite { guild, name } => {
            bounded(name.as_bytes(), 29)?;
            put(&mut b, 1, guild);
            put(&mut b, 2, name.as_bytes());
            2
        }
        GuildIntent::Kick { guild, character } | GuildIntent::RevokeInvite { guild, character } => {
            put(&mut b, 1, guild);
            put(&mut b, 2, character);
            if matches!(intent, GuildIntent::Kick { .. }) {
                5
            } else {
                7
            }
        }
        GuildIntent::Join { guild } | GuildIntent::Leave { guild } => {
            put(&mut b, 1, guild);
            if matches!(intent, GuildIntent::Join { .. }) {
                3
            } else {
                4
            }
        }
        GuildIntent::SetRank {
            guild,
            character,
            level,
            expected_revision,
        } => {
            put(&mut b, 1, guild);
            put(&mut b, 2, character);
            num(&mut b, 3, u64::from(*level));
            num(&mut b, 4, *expected_revision);
            6
        }
    };
    let mut out = Vec::new();
    put(&mut out, tag, &b);
    checked(out, decode_guild_intent, MAX_GUILD_REQUEST_BYTES)
}
pub fn decode_guild_intent(bytes: &[u8]) -> WireResult<GuildIntent> {
    bounded(bytes, MAX_GUILD_REQUEST_BYTES)?;
    let (tag, b) = one(bytes, 7)?;
    let count = match tag {
        1 | 3 | 4 => 1,
        6 => 4,
        _ => 2,
    };
    let f = fields(b, count)?;
    if tag == 1 {
        return Ok(GuildIntent::Found {
            name: text(required(&f, 1)?, 29)?,
        });
    }
    let guild = uuid(required(&f, 1)?)?;
    if tag == 3 {
        return Ok(GuildIntent::Join { guild });
    }
    if tag == 4 {
        return Ok(GuildIntent::Leave { guild });
    }
    if tag == 2 {
        return Ok(GuildIntent::Invite {
            guild,
            name: text(required(&f, 2)?, 29)?,
        });
    }
    let character = uuid(required(&f, 2)?)?;
    Ok(match tag {
        5 => GuildIntent::Kick { guild, character },
        6 => GuildIntent::SetRank {
            guild,
            character,
            level: rank(&f, 3)?,
            expected_revision: scalar(&f, 4)?,
        },
        7 => GuildIntent::RevokeInvite { guild, character },
        _ => return bad(),
    })
}
fn cursor(c: &GuildRosterCursor) -> Vec<u8> {
    let mut b = Vec::new();
    put(&mut b, 1, &c.guild);
    num(&mut b, 2, u64::from(c.level));
    put(&mut b, 3, &c.character);
    b
}
fn read_cursor(bytes: &[u8], guild: [u8; 16]) -> WireResult<GuildRosterCursor> {
    let f = fields(bytes, 3)?;
    let c = GuildRosterCursor {
        guild: uuid(required(&f, 1)?)?,
        level: rank(&f, 2)?,
        character: uuid(required(&f, 3)?)?,
    };
    if c.guild != guild {
        return bad();
    }
    Ok(c)
}
pub fn encode_guild_query(q: &GuildQuery) -> WireResult<Vec<u8>> {
    let mut b = Vec::new();
    let tag = match q {
        GuildQuery::Own => 1,
        GuildQuery::Invitations => 3,
        GuildQuery::Roster { guild, after } => {
            put(&mut b, 1, guild);
            if let Some(c) = after {
                put(&mut b, 2, &cursor(c));
            }
            2
        }
    };
    let mut out = Vec::new();
    put(&mut out, tag, &b);
    checked(out, decode_guild_query, MAX_GUILD_REQUEST_BYTES)
}
pub fn decode_guild_query(bytes: &[u8]) -> WireResult<GuildQuery> {
    bounded(bytes, MAX_GUILD_REQUEST_BYTES)?;
    let (tag, b) = one(bytes, 3)?;
    if tag != 2 {
        if !b.is_empty() {
            return bad();
        }
        return Ok(if tag == 1 {
            GuildQuery::Own
        } else {
            GuildQuery::Invitations
        });
    }
    let f = fields(b, 2)?;
    let guild = uuid(required(&f, 1)?)?;
    Ok(GuildQuery::Roster {
        guild,
        after: optional_bytes(&f, 2)?
            .map(|b| read_cursor(b, guild))
            .transpose()?,
    })
}
pub fn encode_guild_intent_result(r: &GuildIntentResult) -> WireResult<Vec<u8>> {
    let mut out = Vec::new();
    num(&mut out, 1, u64::from(r.code));
    put(&mut out, 2, &r.guild);
    if let Some(n) = r.revision {
        num(&mut out, 3, n);
    }
    checked(out, decode_guild_intent_result, 128)
}
pub fn decode_guild_intent_result(bytes: &[u8]) -> WireResult<GuildIntentResult> {
    bounded(bytes, 128)?;
    let f = fields(bytes, 3)?;
    let code = scalar(&f, 1)?;
    if code > 14 {
        return bad();
    }
    Ok(GuildIntentResult {
        code: code as u8,
        guild: uuid(required(&f, 2)?)?,
        revision: if f[2].is_some() {
            Some(scalar(&f, 3)?)
        } else {
            None
        },
    })
}
mod query_result;
pub use query_result::{decode_guild_query_result, encode_guild_query_result};
