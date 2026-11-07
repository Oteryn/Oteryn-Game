//! Bounded private boss reward access. Death ids select entitlements, never another character.
use crate::charm_wire::CyclopediaWireError as Error;

pub const CAPABILITY_BOSS_REWARDS_V1: u32 = 224;
pub const COMMAND_TYPE_BOSS_REWARD_BROWSE: u32 = 428;
pub const COMMAND_TYPE_BOSS_REWARD_TAKE: u32 = 429;
pub const COMMAND_TYPE_BOSS_REWARD_COLLECT_ALL: u32 = 430;
pub const MAX_REQUEST_BYTES: usize = 19;
pub const MAX_RESULT_BYTES: usize = 1_024;
pub const MAX_PAGE_ROWS: usize = 64;
pub const MAX_REWARD_ORDINAL: u16 = 1_000;
const VERSION: u8 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BrowseRequest {
    pub death: [u8; 16],
    pub after_ordinal: u16,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TakeRequest {
    pub death: [u8; 16],
    pub ordinal: u16,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CollectAllRequest {
    pub death: Option<[u8; 16]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum BossRewardStatus {
    Ok = 0,
    Empty = 1,
    Expired = 2,
    NoRoom = 3,
    OutOfReach = 4,
    Busy = 5,
    Rejected = 6,
    Unavailable = 7,
    ContentUnavailable = 8,
}

impl TryFrom<u8> for BossRewardStatus {
    type Error = Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Ok),
            1 => Ok(Self::Empty),
            2 => Ok(Self::Expired),
            3 => Ok(Self::NoRoom),
            4 => Ok(Self::OutOfReach),
            5 => Ok(Self::Busy),
            6 => Ok(Self::Rejected),
            7 => Ok(Self::Unavailable),
            8 => Ok(Self::ContentUnavailable),
            _ => Err(Error::Malformed),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RewardRow {
    pub ordinal: u16,
    pub definition_reference: u32,
    pub quantity: u32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowseReply {
    pub status: BossRewardStatus,
    pub death: [u8; 16],
    pub expires_at_unix_ms: u64,
    pub rows: Vec<RewardRow>,
    pub more: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CollectReply {
    pub status: BossRewardStatus,
    pub moved: u16,
    pub remaining: u32,
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8], max: usize) -> Result<Self, Error> {
        if bytes.len() > max {
            return Err(Error::LimitExceeded);
        }
        let mut value = Self { bytes, at: 0 };
        if value.byte()? != VERSION {
            return Err(Error::Malformed);
        }
        Ok(value)
    }
    fn array<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        let end = self.at.checked_add(N).ok_or(Error::Malformed)?;
        let value = self.bytes.get(self.at..end).ok_or(Error::Malformed)?;
        self.at = end;
        value.try_into().map_err(|_| Error::Malformed)
    }
    fn byte(&mut self) -> Result<u8, Error> {
        Ok(self.array::<1>()?[0])
    }
    fn u16(&mut self) -> Result<u16, Error> {
        Ok(u16::from_le_bytes(self.array()?))
    }
    fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.array()?))
    }
    fn u64(&mut self) -> Result<u64, Error> {
        Ok(u64::from_le_bytes(self.array()?))
    }
    fn finish(self) -> Result<(), Error> {
        if self.at == self.bytes.len() {
            Ok(())
        } else {
            Err(Error::Malformed)
        }
    }
}

fn valid_death(death: [u8; 16]) -> Result<[u8; 16], Error> {
    if death == [0; 16] {
        Err(Error::Malformed)
    } else {
        Ok(death)
    }
}

pub fn decode_browse_request(bytes: &[u8]) -> Result<BrowseRequest, Error> {
    let mut r = Reader::new(bytes, MAX_REQUEST_BYTES)?;
    let death = valid_death(r.array()?)?;
    let after_ordinal = r.u16()?;
    r.finish()?;
    if after_ordinal > MAX_REWARD_ORDINAL {
        return Err(Error::Malformed);
    }
    Ok(BrowseRequest {
        death,
        after_ordinal,
    })
}
pub fn encode_browse_request(value: &BrowseRequest) -> Result<Vec<u8>, Error> {
    valid_death(value.death)?;
    if value.after_ordinal > MAX_REWARD_ORDINAL {
        return Err(Error::Malformed);
    }
    let mut out = vec![VERSION];
    out.extend(value.death);
    out.extend(value.after_ordinal.to_le_bytes());
    Ok(out)
}
pub fn decode_take_request(bytes: &[u8]) -> Result<TakeRequest, Error> {
    let value = decode_browse_request(bytes)?;
    if value.after_ordinal == 0 {
        return Err(Error::Malformed);
    }
    Ok(TakeRequest {
        death: value.death,
        ordinal: value.after_ordinal,
    })
}
pub fn encode_take_request(value: &TakeRequest) -> Result<Vec<u8>, Error> {
    if value.ordinal == 0 {
        return Err(Error::Malformed);
    }
    encode_browse_request(&BrowseRequest {
        death: value.death,
        after_ordinal: value.ordinal,
    })
}
pub fn decode_collect_all_request(bytes: &[u8]) -> Result<CollectAllRequest, Error> {
    let mut r = Reader::new(bytes, 17)?;
    let death = r.array()?;
    r.finish()?;
    Ok(CollectAllRequest {
        death: (death != [0; 16]).then_some(death),
    })
}
pub fn encode_collect_all_request(value: &CollectAllRequest) -> Result<Vec<u8>, Error> {
    if let Some(death) = value.death {
        valid_death(death)?;
    }
    let mut out = vec![VERSION];
    out.extend(value.death.unwrap_or([0; 16]));
    Ok(out)
}

fn validate_rows(rows: &[RewardRow]) -> Result<(), Error> {
    if rows.len() > MAX_PAGE_ROWS {
        return Err(Error::LimitExceeded);
    }
    let mut previous = 0;
    for row in rows {
        if row.ordinal <= previous
            || row.ordinal > MAX_REWARD_ORDINAL
            || row.definition_reference == 0
            || row.quantity == 0
        {
            return Err(Error::Malformed);
        }
        previous = row.ordinal;
    }
    Ok(())
}
pub fn encode_browse_reply(value: &BrowseReply) -> Result<Vec<u8>, Error> {
    valid_death(value.death)?;
    validate_rows(&value.rows)?;
    let mut out = vec![VERSION, value.status as u8];
    out.extend(value.death);
    out.extend(value.expires_at_unix_ms.to_le_bytes());
    out.push(u8::try_from(value.rows.len()).map_err(|_| Error::LimitExceeded)?);
    for row in &value.rows {
        out.extend(row.ordinal.to_le_bytes());
        out.extend(row.definition_reference.to_le_bytes());
        out.extend(row.quantity.to_le_bytes());
    }
    out.push(u8::from(value.more));
    Ok(out)
}
pub fn decode_browse_reply(bytes: &[u8]) -> Result<BrowseReply, Error> {
    let mut r = Reader::new(bytes, MAX_RESULT_BYTES)?;
    let status = BossRewardStatus::try_from(r.byte()?)?;
    let death = valid_death(r.array()?)?;
    let expires_at_unix_ms = r.u64()?;
    let count = usize::from(r.byte()?);
    if count > MAX_PAGE_ROWS {
        return Err(Error::LimitExceeded);
    }
    let mut rows = Vec::with_capacity(count);
    for _ in 0..count {
        rows.push(RewardRow {
            ordinal: r.u16()?,
            definition_reference: r.u32()?,
            quantity: r.u32()?,
        });
    }
    validate_rows(&rows)?;
    let more = match r.byte()? {
        0 => false,
        1 => true,
        _ => return Err(Error::Malformed),
    };
    r.finish()?;
    Ok(BrowseReply {
        status,
        death,
        expires_at_unix_ms,
        rows,
        more,
    })
}
pub fn encode_collect_reply(value: &CollectReply) -> Result<Vec<u8>, Error> {
    let mut out = vec![VERSION, value.status as u8];
    out.extend(value.moved.to_le_bytes());
    out.extend(value.remaining.to_le_bytes());
    Ok(out)
}
pub fn decode_collect_reply(bytes: &[u8]) -> Result<CollectReply, Error> {
    let mut r = Reader::new(bytes, 8)?;
    let value = CollectReply {
        status: BossRewardStatus::try_from(r.byte()?)?,
        moved: r.u16()?,
        remaining: r.u32()?,
    };
    r.finish()?;
    Ok(value)
}

#[cfg(test)]
#[path = "boss_rewards_tests.rs"]
mod tests;
