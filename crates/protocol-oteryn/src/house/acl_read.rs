use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HouseAclSelector {
    Guest,
    Subowner,
    Door { x: u16, y: u16, floor: i8 },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HouseAclRead {
    pub house: String,
    pub selector: HouseAclSelector,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HouseAclSnapshot {
    pub revision: u64,
    pub members: Vec<[u8; 16]>,
}

pub fn encode_house_acl_read_query(
    world: [u8; 16],
    town: &str,
    read: &HouseAclRead,
) -> Result<Vec<u8>> {
    uuid(&world)?;
    text(read.house.as_bytes())?;
    let mut nested = Vec::new();
    bytes(&mut nested, 1, read.house.as_bytes());
    let kind = match read.selector {
        HouseAclSelector::Guest => 1,
        HouseAclSelector::Subowner => 2,
        HouseAclSelector::Door { .. } => 3,
    };
    scalar(&mut nested, 2, kind);
    if let HouseAclSelector::Door { x, y, floor } = read.selector {
        if !(-15..=0).contains(&floor) {
            return Err(Error::Malformed);
        }
        let floor = i64::from(floor);
        let mut p = Vec::new();
        scalar(&mut p, 1, u64::from(x));
        scalar(&mut p, 2, u64::from(y));
        scalar(&mut p, 3, ((floor << 1) ^ (floor >> 63)) as u64);
        bytes(&mut nested, 3, &p);
    }
    let mut out = Vec::new();
    bytes(&mut out, 1, &world);
    bytes(&mut out, 2, town.as_bytes());
    bytes(&mut out, 5, &nested);
    decode_house_query(&out)?;
    Ok(out)
}

pub(super) fn decode_read(body: &[u8]) -> Result<HouseAclRead> {
    let (mut at, mut house, mut kind, mut position) = (0, None, None, None);
    while at < body.len() {
        match read_varint(body, &mut at)? {
            10 => set_once(&mut house, text(read_bytes(body, &mut at)?)?)?,
            16 => set_once(&mut kind, read_varint(body, &mut at)?)?,
            26 => {
                let mut p = Vec::new();
                bytes(&mut p, 1, read_bytes(body, &mut at)?);
                scalar(&mut p, 2, 0);
                let HouseAction::Door { x, y, floor, .. } = list(&p, true)? else {
                    return Err(Error::Malformed);
                };
                set_once(&mut position, HouseAclSelector::Door { x, y, floor })?;
            }
            _ => return Err(Error::Malformed),
        }
    }
    let selector = match (kind, position) {
        (Some(1), None) => HouseAclSelector::Guest,
        (Some(2), None) => HouseAclSelector::Subowner,
        (Some(3), Some(position)) => position,
        _ => return Err(Error::Malformed),
    };
    Ok(HouseAclRead {
        house: house.ok_or(Error::Malformed)?,
        selector,
    })
}

pub fn encode_house_acl_snapshot(snapshot: &HouseAclSnapshot) -> Result<Vec<u8>> {
    if snapshot.members.len() > MAX_MEMBERS {
        return Err(Error::LimitExceeded);
    }
    let mut members = Vec::new();
    for (index, member) in snapshot.members.iter().enumerate() {
        uuid(member)?;
        if snapshot.members[..index].contains(member) {
            return Err(Error::Malformed);
        }
        bytes(&mut members, 1, member);
    }
    let mut out = encode_house_result(HouseDisposition::Ok, snapshot.revision);
    bytes(&mut out, 5, &members);
    Ok(out)
}

pub fn decode_house_acl_snapshot(body: &[u8]) -> Result<HouseAclSnapshot> {
    if body.len() > MAX_RESULT_BYTES {
        return Err(Error::LimitExceeded);
    }
    let (mut at, mut disposition, mut revision, mut members) = (0, None, None, None);
    while at < body.len() {
        match read_varint(body, &mut at)? {
            8 => set_once(&mut disposition, read_varint(body, &mut at)?)?,
            16 => set_once(&mut revision, read_varint(body, &mut at)?)?,
            42 => {
                let nested = read_bytes(body, &mut at)?;
                let (mut at, mut decoded) = (0, Vec::new());
                while at < nested.len() {
                    if decoded.len() == MAX_MEMBERS {
                        return Err(Error::LimitExceeded);
                    }
                    if read_varint(nested, &mut at)? != 10 {
                        return Err(Error::Malformed);
                    }
                    let member = uuid(read_bytes(nested, &mut at)?)?;
                    if decoded.contains(&member) {
                        return Err(Error::Malformed);
                    }
                    decoded.push(member);
                }
                set_once(&mut members, decoded)?;
            }
            _ => return Err(Error::Malformed),
        }
    }
    if disposition != Some(HouseDisposition::Ok as u64) {
        return Err(Error::Malformed);
    }
    Ok(HouseAclSnapshot {
        revision: revision.ok_or(Error::Malformed)?,
        members: members.ok_or(Error::Malformed)?,
    })
}
