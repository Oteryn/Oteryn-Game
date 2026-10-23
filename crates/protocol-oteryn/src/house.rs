//! HOUSE-WIRE-1: strict, bounded management requests. World/House data never grants authority.
use crate::charm_wire::CyclopediaWireError as Error;
use crate::charm_wire::{
    push_message_field as bytes, push_varint_field as scalar, read_bytes, read_varint, set_once,
};

pub const CAPABILITY_HOUSE_V1: u32 = 49;
pub const COMMAND_TYPE_HOUSE_INTENT: u32 = 56;
pub const COMMAND_TYPE_HOUSE_QUERY: u32 = 57;
pub const MAX_INTENT_BYTES: usize = 4168;
pub const MAX_QUERY_BYTES: usize = 1056;
pub const MAX_RESULT_BYTES: usize = 32768;
pub const MAX_MEMBERS: usize = 200;
pub const PAGE_SIZE: usize = 20;
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HouseIntent {
    pub world: [u8; 16],
    pub house: String,
    pub action: HouseAction,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HouseAction {
    Bid(u64),
    MoveOut(u64),
    CancelMoveOut,
    Acl {
        subowners: bool,
        revision: u64,
        members: Vec<[u8; 16]>,
    },
    Door {
        x: u16,
        y: u16,
        floor: i8,
        revision: u64,
        members: Vec<[u8; 16]>,
    },
    Kick([u8; 16]),
    Leave,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HouseQuery {
    pub world: [u8; 16],
    pub town: String,
    pub cursor: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HouseSummary {
    pub house: String,
    pub state: u8,
    pub rent: u64,
    pub revision: u64,
    pub acl_revision: u64,
    pub owned: bool,
    pub move_out_ms: Option<u64>,
    pub price: u64,
    pub ends_ms: Option<u64>,
    pub own_maximum: Option<u64>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HouseDisposition {
    Ok = 1,
    Rejected = 2,
    Conflict = 3,
}

fn text(value: &[u8]) -> Result<String> {
    if value.is_empty() || value.len() > 512 {
        return Err(Error::LimitExceeded);
    }
    std::str::from_utf8(value)
        .map(str::to_owned)
        .map_err(|_| Error::Malformed)
}
fn uuid(value: &[u8]) -> Result<[u8; 16]> {
    let id: [u8; 16] = value.try_into().map_err(|_| Error::Malformed)?;
    if id[6] >> 4 != 7 || id[8] & 0xc0 != 0x80 {
        return Err(Error::Malformed);
    }
    Ok(id)
}
fn target(body: &[u8]) -> Result<([u8; 16], String)> {
    let (mut world, mut house, mut c) = (None, None, 0);
    while c < body.len() {
        match read_varint(body, &mut c)? {
            10 => set_once(&mut world, uuid(read_bytes(body, &mut c)?)?)?,
            18 => set_once(&mut house, text(read_bytes(body, &mut c)?)?)?,
            _ => return Err(Error::Malformed),
        }
    }
    Ok((
        world.ok_or(Error::Malformed)?,
        house.ok_or(Error::Malformed)?,
    ))
}
fn single(body: &[u8]) -> Result<u64> {
    let mut c = 0;
    if read_varint(body, &mut c)? != 8 {
        return Err(Error::Malformed);
    }
    let n = read_varint(body, &mut c)?;
    if c != body.len() {
        return Err(Error::Malformed);
    }
    Ok(n)
}
fn list(body: &[u8], door: bool) -> Result<HouseAction> {
    let (mut c, mut kind, mut revision, mut position) = (0, None, None, None);
    let mut members = Vec::new();
    while c < body.len() {
        match read_varint(body, &mut c)? {
            8 if !door => set_once(&mut kind, read_varint(body, &mut c)?)?,
            10 if door => {
                let p = read_bytes(body, &mut c)?;
                let (mut at, mut x, mut y, mut floor) = (0, None, None, None);
                while at < p.len() {
                    let key = read_varint(p, &mut at)?;
                    let n = read_varint(p, &mut at)?;
                    match key {
                        8 => set_once(&mut x, u16::try_from(n).map_err(|_| Error::Malformed)?)?,
                        16 => set_once(&mut y, u16::try_from(n).map_err(|_| Error::Malformed)?)?,
                        24 if n <= 30 => set_once(&mut floor, ((n >> 1) as i8) ^ -((n & 1) as i8))?,
                        _ => return Err(Error::Malformed),
                    }
                }
                let floor = floor.ok_or(Error::Malformed)?;
                if !(-15..=0).contains(&floor) {
                    return Err(Error::Malformed);
                }
                set_once(
                    &mut position,
                    (
                        x.ok_or(Error::Malformed)?,
                        y.ok_or(Error::Malformed)?,
                        floor,
                    ),
                )?;
            }
            16 => set_once(&mut revision, read_varint(body, &mut c)?)?,
            26 => {
                if members.len() == MAX_MEMBERS {
                    return Err(Error::LimitExceeded);
                }
                let id = uuid(read_bytes(body, &mut c)?)?;
                if members.contains(&id) {
                    return Err(Error::Malformed);
                }
                members.push(id);
            }
            _ => return Err(Error::Malformed),
        }
    }
    let revision = revision.ok_or(Error::Malformed)?;
    if door {
        let (x, y, floor) = position.ok_or(Error::Malformed)?;
        Ok(HouseAction::Door {
            x,
            y,
            floor,
            revision,
            members,
        })
    } else {
        let subowners = match kind {
            Some(1) => false,
            Some(2) => true,
            _ => return Err(Error::Malformed),
        };
        Ok(HouseAction::Acl {
            subowners,
            revision,
            members,
        })
    }
}
pub fn decode_house_intent(body: &[u8]) -> Result<HouseIntent> {
    if body.len() > MAX_INTENT_BYTES {
        return Err(Error::LimitExceeded);
    }
    let (mut c, mut binding, mut action) = (0, None, None);
    while c < body.len() {
        let key = read_varint(body, &mut c)?;
        let nested = read_bytes(body, &mut c)?;
        if key == 66 {
            set_once(&mut binding, target(nested)?)?;
            continue;
        }
        let value = match key {
            10 => HouseAction::Bid(single(nested)?),
            18 => HouseAction::MoveOut(single(nested)?),
            26 if nested.is_empty() => HouseAction::CancelMoveOut,
            34 => list(nested, false)?,
            42 => list(nested, true)?,
            50 => {
                let mut at = 0;
                if read_varint(nested, &mut at)? != 10 {
                    return Err(Error::Malformed);
                }
                let id = uuid(read_bytes(nested, &mut at)?)?;
                if at != nested.len() {
                    return Err(Error::Malformed);
                }
                HouseAction::Kick(id)
            }
            58 if nested.is_empty() => HouseAction::Leave,
            _ => return Err(Error::Malformed),
        };
        set_once(&mut action, value)?;
    }
    let (world, house) = binding.ok_or(Error::Malformed)?;
    Ok(HouseIntent {
        world,
        house,
        action: action.ok_or(Error::Malformed)?,
    })
}
pub fn decode_house_query(body: &[u8]) -> Result<HouseQuery> {
    if body.len() > MAX_QUERY_BYTES {
        return Err(Error::LimitExceeded);
    }
    let (mut c, mut world, mut town, mut cursor) = (0, None, None, None);
    while c < body.len() {
        match read_varint(body, &mut c)? {
            10 => set_once(&mut world, uuid(read_bytes(body, &mut c)?)?)?,
            18 => set_once(&mut town, text(read_bytes(body, &mut c)?)?)?,
            26 => {
                let raw = read_bytes(body, &mut c)?;
                set_once(
                    &mut cursor,
                    if raw.is_empty() {
                        None
                    } else {
                        Some(text(raw)?)
                    },
                )?;
            }
            _ => return Err(Error::Malformed),
        }
    }
    let town = town.ok_or(Error::Malformed)?;
    if !town
        .strip_prefix("oteryn:content.area.city.")
        .is_some_and(|s| {
            !s.is_empty()
                && s.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        })
    {
        return Err(Error::Malformed);
    }
    Ok(HouseQuery {
        world: world.ok_or(Error::Malformed)?,
        town,
        cursor: cursor.flatten(),
    })
}
/// A terminal, known result; ambiguous commits disconnect and are retried by CommandRef.
pub fn encode_house_result(disposition: HouseDisposition, revision: u64) -> Vec<u8> {
    let mut out = Vec::new();
    scalar(&mut out, 1, disposition as u64);
    scalar(&mut out, 2, revision);
    out
}
pub fn encode_house_page(rows: &[HouseSummary], cursor: Option<&str>) -> Result<Vec<u8>> {
    if rows.len() > PAGE_SIZE {
        return Err(Error::LimitExceeded);
    }
    let mut out = Vec::new();
    scalar(&mut out, 1, HouseDisposition::Ok as u64);
    for row in rows {
        if row.house.len() > 512 || !(1..=6).contains(&row.state) {
            return Err(Error::Malformed);
        }
        let mut b = Vec::new();
        bytes(&mut b, 1, row.house.as_bytes());
        scalar(&mut b, 2, row.state.into());
        scalar(&mut b, 3, row.rent);
        scalar(&mut b, 4, row.revision);
        scalar(&mut b, 5, u64::from(row.owned));
        scalar(&mut b, 10, row.acl_revision);
        if let Some(due) = row.move_out_ms {
            scalar(&mut b, 6, due);
        }
        scalar(&mut b, 7, row.price);
        if let Some(end) = row.ends_ms {
            scalar(&mut b, 8, end);
        }
        if let Some(maximum) = row.own_maximum {
            scalar(&mut b, 9, maximum);
        }
        bytes(&mut out, 3, &b);
    }
    if let Some(cursor) = cursor {
        if cursor.len() > 512 {
            return Err(Error::LimitExceeded);
        }
        bytes(&mut out, 4, cursor.as_bytes());
    }
    if out.len() > MAX_RESULT_BYTES {
        return Err(Error::LimitExceeded);
    }
    Ok(out)
}
#[cfg(test)]
#[path = "house_tests.rs"]
mod tests;
