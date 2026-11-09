//! Soulseal fight catalogue and session-bound entry; capability 72, command 6120.
pub const COMMAND_TYPE: u32 = 6120;
pub const PAGE_SIZE: usize = 20;
pub const MAX_REQUEST_BYTES: usize = 20;
pub const MAX_REPLY_BYTES: usize = 8192;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Malformed,
    Limit,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    Catalogue { after: u16 },
    Enter { authorization: [u8; 16], race: u16 },
}
impl Request {
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        let mut out = vec![1];
        match self {
            Self::Catalogue { after } if *after <= 4096 => {
                out.push(0);
                out.extend(after.to_le_bytes());
            }
            Self::Enter {
                authorization,
                race,
            } if (1..=4096).contains(race) && *authorization != [0; 16] => {
                out.push(1);
                out.extend(authorization);
                out.extend(race.to_le_bytes());
            }
            _ => return Err(Error::Limit),
        }
        Ok(out)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        let request = match bytes {
            [1, 0, a, b] => Self::Catalogue {
                after: u16::from_le_bytes([*a, *b]),
            },
            [1, 1, rest @ ..] if rest.len() == 18 => Self::Enter {
                authorization: rest[..16].try_into().map_err(|_| Error::Malformed)?,
                race: u16::from_le_bytes([rest[16], rest[17]]),
            },
            _ => return Err(Error::Malformed),
        };
        request.encode()?;
        Ok(request)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Catalogue,
    Started,
    Refused,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryRules {
    pub required_level: u32,
    pub time_limit_seconds: u32,
    pub authorization_ms: u32,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fight {
    pub race: u16,
    pub name: String,
    pub stars: u8,
    pub cost: u16,
}
impl Fight {
    pub const fn cost(&self) -> u16 {
        self.cost
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    pub status: Status,
    pub balance: Option<u64>,
    pub authorization: Option<[u8; 16]>,
    pub next: Option<u16>,
    pub total: u16,
    pub rules: EntryRules,
    pub entries: Vec<Fight>,
    pub reason: String,
}
fn string(out: &mut Vec<u8>, text: &str, max: usize) -> Result<(), Error> {
    if text.len() > max || text.contains('\0') {
        return Err(Error::Limit);
    }
    out.extend((text.len() as u16).to_le_bytes());
    out.extend(text.as_bytes());
    Ok(())
}
impl Reply {
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        if self.entries.len() > PAGE_SIZE
            || self.total > 4096
            || self.entries.len() > usize::from(self.total)
            || self.next == Some(0)
            || self
                .next
                .is_some_and(|n| n > 4096 || self.entries.last().is_none_or(|e| e.race != n))
            || self.entries.windows(2).any(|p| p[0].race >= p[1].race)
            || self.authorization == Some([0; 16])
            || self.rules.required_level == 0
            || self.rules.time_limit_seconds == 0
            || self.rules.authorization_ms == 0
            || (self.status != Status::Catalogue
                && (!self.entries.is_empty()
                    || self.next.is_some()
                    || self.authorization.is_some()))
            || (self.status != Status::Refused && self.balance.is_none())
        {
            return Err(Error::Malformed);
        }
        let status = match self.status {
            Status::Catalogue => 0,
            Status::Started => 1,
            Status::Refused => 2,
        };
        let mut out = vec![1, status, u8::from(self.balance.is_some())];
        if let Some(balance) = self.balance {
            out.extend(balance.to_le_bytes());
        }
        out.push(u8::from(self.authorization.is_some()));
        if let Some(auth) = self.authorization {
            out.extend(auth);
        }
        out.extend(self.next.unwrap_or(0).to_le_bytes());
        out.extend(self.total.to_le_bytes());
        for n in [
            self.rules.required_level,
            self.rules.time_limit_seconds,
            self.rules.authorization_ms,
        ] {
            out.extend(n.to_le_bytes());
        }
        string(&mut out, &self.reason, 256)?;
        out.push(self.entries.len() as u8);
        for entry in &self.entries {
            if !(1..=4096).contains(&entry.race)
                || entry.stars > 5
                || entry.cost != (u16::from(entry.stars) + 1) * 10
                || entry.name.is_empty()
            {
                return Err(Error::Malformed);
            }
            out.extend(entry.race.to_le_bytes());
            out.push(entry.stars);
            out.extend(entry.cost.to_le_bytes());
            string(&mut out, &entry.name, 128)?;
        }
        if out.len() > MAX_REPLY_BYTES {
            return Err(Error::Limit);
        }
        Ok(out)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() > MAX_REPLY_BYTES {
            return Err(Error::Limit);
        }
        let mut r = Reader(bytes);
        if r.u8()? != 1 {
            return Err(Error::Malformed);
        }
        let status = match r.u8()? {
            0 => Status::Catalogue,
            1 => Status::Started,
            2 => Status::Refused,
            _ => return Err(Error::Malformed),
        };
        let balance = match r.u8()? {
            0 => None,
            1 => Some(u64::from_le_bytes(
                r.take(8)?.try_into().map_err(|_| Error::Malformed)?,
            )),
            _ => return Err(Error::Malformed),
        };
        let authorization = match r.u8()? {
            0 => None,
            1 => Some(r.take(16)?.try_into().map_err(|_| Error::Malformed)?),
            _ => return Err(Error::Malformed),
        };
        let next = match r.u16()? {
            0 => None,
            n => Some(n),
        };
        let total = r.u16()?;
        let rules = EntryRules {
            required_level: r.u32()?,
            time_limit_seconds: r.u32()?,
            authorization_ms: r.u32()?,
        };
        let reason = r.text(256)?;
        let count = r.u8()?;
        if usize::from(count) > PAGE_SIZE {
            return Err(Error::Limit);
        }
        let mut entries = Vec::with_capacity(usize::from(count));
        for _ in 0..count {
            entries.push(Fight {
                race: r.u16()?,
                stars: r.u8()?,
                cost: r.u16()?,
                name: r.text(128)?,
            });
        }
        if !r.0.is_empty() {
            return Err(Error::Malformed);
        }
        let reply = Self {
            status,
            balance,
            authorization,
            next,
            total,
            rules,
            entries,
            reason,
        };
        reply.encode()?;
        Ok(reply)
    }
}
struct Reader<'a>(&'a [u8]);
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let (head, tail) = self.0.split_at_checked(n).ok_or(Error::Malformed)?;
        self.0 = tail;
        Ok(head)
    }
    fn u8(&mut self) -> Result<u8, Error> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, Error> {
        Ok(u16::from_le_bytes(
            self.take(2)?.try_into().map_err(|_| Error::Malformed)?,
        ))
    }
    fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().map_err(|_| Error::Malformed)?,
        ))
    }
    fn text(&mut self, max: usize) -> Result<String, Error> {
        let len = usize::from(self.u16()?);
        if len > max {
            return Err(Error::Limit);
        }
        let text = std::str::from_utf8(self.take(len)?).map_err(|_| Error::Malformed)?;
        if text.contains('\0') {
            return Err(Error::Malformed);
        }
        Ok(text.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn t114_soulseal_requests_and_catalogue_round_trip_without_ambiguous_frames()
    -> Result<(), Error> {
        for request in [
            Request::Catalogue { after: 0 },
            Request::Enter {
                authorization: [7; 16],
                race: 3,
            },
        ] {
            let bytes = request.encode()?;
            assert_eq!(Request::decode(&bytes)?, request);
            for end in 0..bytes.len() {
                assert!(Request::decode(&bytes[..end]).is_err());
            }
            let mut trailing = bytes;
            trailing.push(0);
            assert!(Request::decode(&trailing).is_err());
        }
        let reply = Reply {
            status: Status::Catalogue,
            balance: Some(80),
            authorization: Some([7; 16]),
            next: None,
            total: 1,
            entries: vec![Fight {
                race: 3,
                name: "Grove watcher".into(),
                stars: 2,
                cost: 30,
            }],
            reason: String::new(),
            rules: EntryRules {
                required_level: 8,
                time_limit_seconds: 600,
                authorization_ms: 10000,
            },
        };
        assert_eq!(Reply::decode(&reply.encode()?)?, reply);
        assert_eq!(reply.entries[0].cost(), 30);
        Ok(())
    }
}
