//! E12 bounded identity selection and appearance management, selected separately from command220.
pub const CAPABILITY_HIRELING_MANAGEMENT_V1: u32 = 250;
pub const COMMAND_TYPE_HIRELING_MANAGEMENT: u32 = 470;
pub const MAX_HIRELINGS: usize = 10;
pub const MAX_NAME_BYTES: usize = 18;
pub const MAX_REPLY_BYTES: usize =
    2 + MAX_HIRELINGS * (16 + 1 + MAX_NAME_BYTES + 1 + 2 + 4 + 1 + 2 + 1 + 5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Request {
    List,
    ReturnToLamp {
        hireling: [u8; 16],
    },
    Outfit {
        hireling: [u8; 16],
        look_type: u16,
        colors: [u8; 4],
    },
    /// Current ephemeral actor bindings; these are never retained in durable receipts.
    Projection,
}

impl Request {
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        match bytes {
            [0] => Some(Self::List),
            [3] => Some(Self::Projection),
            [1, rest @ ..] if rest.len() == 16 => Some(Self::ReturnToLamp {
                hireling: identity(rest)?,
            }),
            [2, rest @ ..] if rest.len() == 22 => {
                let look_type = u16::from_be_bytes(rest[16..18].try_into().ok()?);
                if look_type == 0 {
                    return None;
                }
                Some(Self::Outfit {
                    hireling: identity(&rest[..16])?,
                    look_type,
                    colors: rest[18..22].try_into().ok()?,
                })
            }
            _ => None,
        }
    }
    pub fn encode(self) -> Vec<u8> {
        match self {
            Self::List => vec![0],
            Self::Projection => vec![3],
            Self::ReturnToLamp { hireling } => {
                let mut out = vec![1];
                out.extend(hireling);
                out
            }
            Self::Outfit {
                hireling,
                look_type,
                colors,
            } => {
                let mut out = vec![2];
                out.extend(hireling);
                out.extend(look_type.to_be_bytes());
                out.extend(colors);
                out
            }
        }
    }
}

fn identity(bytes: &[u8]) -> Option<[u8; 16]> {
    let id: [u8; 16] = bytes.try_into().ok()?;
    (id[6] >> 4 == 7 && id[8] & 0xc0 == 0x80).then_some(id)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Refusal {
    Stale = 1,
    NotOwner = 2,
    NotActive = 3,
    OutfitNotOwned = 4,
    InventoryFull = 5,
    Capacity = 6,
    NotInHouse = 7,
    Conflict = 8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub hireling: [u8; 16],
    pub name: String,
    /// Crystal uses1 for male and2 for female.
    pub sex: u8,
    pub look_type: u16,
    pub colors: [u8; 4],
    pub skills: u8,
    /// Bit0 is Banker, through bit8 Dragon. Citizen is always available.
    pub outfits: u16,
    pub position: Option<(u16, u16, i8)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HirelingActorBinding {
    pub hireling: [u8; 16],
    pub entity: crate::world_spatial_entities::EntityRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    State(Vec<Entry>),
    Committed,
    Refused(Refusal),
    Projection(Vec<HirelingActorBinding>),
}

impl Reply {
    pub fn encode(&self) -> Option<Vec<u8>> {
        let mut out = Vec::new();
        match self {
            Self::Committed => out.push(1),
            Self::Refused(reason) => out.extend([2, *reason as u8]),
            Self::Projection(entries) => {
                if entries.len() > MAX_HIRELINGS {
                    return None;
                }
                let mut identities = std::collections::BTreeSet::new();
                let mut actors = std::collections::BTreeSet::new();
                out.extend([3, entries.len() as u8]);
                for binding in entries {
                    if identity(&binding.hireling).is_none()
                        || binding.entity.generation == 0
                        || !identities.insert(binding.hireling)
                        || !actors.insert(binding.entity.identity)
                    {
                        return None;
                    }
                    let entity = crate::world_spatial_entities::encode_ref(&binding.entity);
                    out.extend(binding.hireling);
                    out.push(u8::try_from(entity.len()).ok()?);
                    out.extend(entity);
                }
            }
            Self::State(entries) => {
                if entries.len() > MAX_HIRELINGS {
                    return None;
                }
                out.extend([0, entries.len() as u8]);
                for e in entries {
                    if identity(&e.hireling).is_none()
                        || !(3..=MAX_NAME_BYTES).contains(&e.name.len())
                        || !e.name.is_ascii()
                        || !(1..=2).contains(&e.sex)
                        || e.look_type == 0
                        || e.skills & !15 != 0
                        || e.outfits & !511 != 0
                    {
                        return None;
                    }
                    out.extend(e.hireling);
                    out.push(e.name.len() as u8);
                    out.extend(e.name.as_bytes());
                    out.push(e.sex);
                    out.extend(e.look_type.to_be_bytes());
                    out.extend(e.colors);
                    out.push(e.skills);
                    out.extend(e.outfits.to_be_bytes());
                    match e.position {
                        None => out.push(0),
                        Some((x, y, floor)) => {
                            if !(-15..=0).contains(&floor) {
                                return None;
                            }
                            out.push(1);
                            out.extend(x.to_be_bytes());
                            out.extend(y.to_be_bytes());
                            out.push(floor as u8);
                        }
                    }
                }
            }
        }
        (out.len() <= MAX_REPLY_BYTES).then_some(out)
    }

    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > MAX_REPLY_BYTES {
            return None;
        }
        let (&tag, mut tail) = bytes.split_first()?;
        match tag {
            3 => {
                let count = take(&mut tail, 1)?[0] as usize;
                if count > MAX_HIRELINGS {
                    return None;
                }
                let mut entries = Vec::with_capacity(count);
                for _ in 0..count {
                    let hireling = identity(take(&mut tail, 16)?)?;
                    let length = take(&mut tail, 1)?[0] as usize;
                    let entity =
                        crate::world_spatial_entities::decode_ref(take(&mut tail, length)?).ok()?;
                    entries.push(HirelingActorBinding { hireling, entity });
                }
                if !tail.is_empty() {
                    return None;
                }
                let reply = Self::Projection(entries);
                reply.encode()?;
                Some(reply)
            }
            1 if tail.is_empty() => Some(Self::Committed),
            2 => {
                let reason = match tail {
                    [1] => Refusal::Stale,
                    [2] => Refusal::NotOwner,
                    [3] => Refusal::NotActive,
                    [4] => Refusal::OutfitNotOwned,
                    [5] => Refusal::InventoryFull,
                    [6] => Refusal::Capacity,
                    [7] => Refusal::NotInHouse,
                    [8] => Refusal::Conflict,
                    _ => return None,
                };
                Some(Self::Refused(reason))
            }
            0 => {
                let count = take(&mut tail, 1)?[0] as usize;
                if count > MAX_HIRELINGS {
                    return None;
                }
                let mut entries = Vec::with_capacity(count);
                for _ in 0..count {
                    let hireling = identity(take(&mut tail, 16)?)?;
                    let length = take(&mut tail, 1)?[0] as usize;
                    let name = std::str::from_utf8(take(&mut tail, length)?)
                        .ok()?
                        .to_owned();
                    let sex = take(&mut tail, 1)?[0];
                    let look_type = u16::from_be_bytes(take(&mut tail, 2)?.try_into().ok()?);
                    let colors = take(&mut tail, 4)?.try_into().ok()?;
                    let skills = take(&mut tail, 1)?[0];
                    let outfits = u16::from_be_bytes(take(&mut tail, 2)?.try_into().ok()?);
                    let position = match take(&mut tail, 1)?[0] {
                        0 => None,
                        1 => Some((
                            u16::from_be_bytes(take(&mut tail, 2)?.try_into().ok()?),
                            u16::from_be_bytes(take(&mut tail, 2)?.try_into().ok()?),
                            take(&mut tail, 1)?[0] as i8,
                        )),
                        _ => return None,
                    };
                    entries.push(Entry {
                        hireling,
                        name,
                        sex,
                        look_type,
                        colors,
                        skills,
                        outfits,
                        position,
                    });
                }
                if !tail.is_empty() {
                    return None;
                }
                let reply = Self::State(entries);
                reply.encode()?;
                Some(reply)
            }
            _ => None,
        }
    }
}
fn take<'a>(tail: &mut &'a [u8], len: usize) -> Option<&'a [u8]> {
    if tail.len() < len {
        return None;
    }
    let (head, rest) = tail.split_at(len);
    *tail = rest;
    Some(head)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn id() -> [u8; 16] {
        let mut id = [1; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        id
    }
    #[test]
    fn management_commands_round_trip_and_reject_unknown_trailing_and_bad_identity() {
        for request in [
            Request::List,
            Request::Projection,
            Request::ReturnToLamp { hireling: id() },
            Request::Outfit {
                hireling: id(),
                look_type: 1109,
                colors: [1, 2, 3, 4],
            },
        ] {
            let bytes = request.encode();
            assert_eq!(Request::decode(&bytes), Some(request));
            for len in 0..bytes.len() {
                assert_eq!(Request::decode(&bytes[..len]), None)
            }
            let mut trailing = bytes;
            trailing.push(0);
            assert_eq!(Request::decode(&trailing), None);
        }
        assert_eq!(Request::decode(&[4]), None);
        assert_eq!(
            Request::decode(&Request::ReturnToLamp { hireling: [0; 16] }.encode()),
            None
        );
    }
    #[test]
    fn projection_reuses_entity_refs_and_rejects_stale_duplicate_and_overbound_bindings()
    -> Result<(), String> {
        let entry = HirelingActorBinding {
            hireling: id(),
            entity: crate::world_spatial_entities::EntityRef {
                identity: [9; 16],
                generation: u64::MAX,
            },
        };
        let reply = Reply::Projection(vec![entry.clone()]);
        let encoded = reply.encode().ok_or("projection encoding")?;
        assert_eq!(Reply::decode(&encoded), Some(reply));
        for len in 0..encoded.len() {
            assert_eq!(Reply::decode(&encoded[..len]), None)
        }
        assert!(Reply::Projection(vec![entry.clone(); 2]).encode().is_none());
        assert!(
            Reply::Projection(vec![entry.clone(); 11])
                .encode()
                .is_none()
        );
        let mut stale = entry;
        stale.entity.generation = 0;
        assert!(Reply::Projection(vec![stale]).encode().is_none());
        assert_eq!(Reply::decode(&[3, 11]), None);
        let full = Reply::Projection(
            (0..MAX_HIRELINGS)
                .map(|index| {
                    let mut hireling = id();
                    hireling[15] = index as u8;
                    let mut identity = [9; 16];
                    identity[15] = index as u8;
                    HirelingActorBinding {
                        hireling,
                        entity: crate::world_spatial_entities::EntityRef {
                            identity,
                            generation: u64::MAX,
                        },
                    }
                })
                .collect(),
        );
        let full_bytes = full.encode().ok_or("full projection encoding")?;
        assert!(full_bytes.len() <= MAX_REPLY_BYTES);
        assert_eq!(Reply::decode(&full_bytes), Some(full));
        Ok(())
    }
    #[test]
    fn bounded_state_replies_keep_identity_appearance_skills_and_active_position()
    -> Result<(), String> {
        let reply = Reply::State(vec![Entry {
            hireling: id(),
            name: "Alice".into(),
            sex: 2,
            look_type: 1109,
            colors: [1, 2, 3, 4],
            skills: 15,
            outfits: 511,
            position: Some((100, 200, -7)),
        }]);
        let bytes = reply.encode().ok_or("encode reply")?;
        assert_eq!(Reply::decode(&bytes), Some(reply));
        for len in 0..bytes.len() {
            assert_eq!(Reply::decode(&bytes[..len]), None)
        }
        assert_eq!(Reply::decode(&[0, 11]), None);
        assert_eq!(Reply::decode(&[1, 0]), None);
        assert_eq!(Reply::decode(&[2, 0]), None);
        for refusal in [
            Refusal::Stale,
            Refusal::NotOwner,
            Refusal::NotActive,
            Refusal::OutfitNotOwned,
            Refusal::InventoryFull,
            Refusal::Capacity,
            Refusal::NotInHouse,
            Refusal::Conflict,
        ] {
            let reply = Reply::Refused(refusal);
            assert_eq!(
                Reply::decode(&reply.encode().ok_or("encode reply")?),
                Some(reply)
            );
        }
        Ok(())
    }
}
