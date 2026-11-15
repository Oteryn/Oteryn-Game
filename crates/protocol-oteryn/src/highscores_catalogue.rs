//! Current authoritative World types and a conjunctive World/type highscores query.
pub use crate::WorldId;
use crate::{
    charm_wire::CyclopediaWireError as Error,
    highscores::{self, HighscoresQuery},
};
pub const CAPABILITY: u32 = 502;
pub const COMMAND_CATALOGUE: u32 = 6241;
pub const COMMAND_QUERY: u32 = 6242;
pub const MAX_WORLDS: usize = 256;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum WorldType {
    Optional = 1,
    Open = 2,
    Hardcore = 3,
    RetroOpen = 4,
    RetroHardcore = 5,
}
impl WorldType {
    pub const ALL: [Self; 5] = [
        Self::Optional,
        Self::Open,
        Self::Hardcore,
        Self::RetroOpen,
        Self::RetroHardcore,
    ];
    pub fn from_id(id: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| *kind as u8 == id)
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Optional => "Optional PvP",
            Self::Open => "Open PvP",
            Self::Hardcore => "Hardcore PvP",
            Self::RetroOpen => "Retro Open PvP",
            Self::RetroHardcore => "Retro Hardcore PvP",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    pub world: WorldId,
    pub world_type: WorldType,
}
pub fn encode_catalogue(entries: &[Entry]) -> Result<Vec<u8>, Error> {
    if entries.len() > MAX_WORLDS
        || entries
            .windows(2)
            .any(|pair| pair[0].world.as_bytes() >= pair[1].world.as_bytes())
    {
        return Err(Error::Malformed);
    }
    let mut out = (entries.len() as u16).to_le_bytes().to_vec();
    for entry in entries {
        out.extend(entry.world.as_bytes());
        out.push(entry.world_type as u8);
    }
    Ok(out)
}
pub fn decode_catalogue(bytes: &[u8]) -> Result<Vec<Entry>, Error> {
    if bytes.len() < 2 {
        return Err(Error::Malformed);
    }
    let count = usize::from(u16::from_le_bytes([bytes[0], bytes[1]]));
    if count > MAX_WORLDS || bytes.len() != 2 + 17 * count {
        return Err(Error::Malformed);
    }
    let mut entries = Vec::with_capacity(count);
    for row in bytes[2..].as_chunks::<17>().0 {
        entries.push(Entry {
            world: WorldId::decode(&row[..16]).map_err(|_| Error::Malformed)?,
            world_type: WorldType::from_id(row[16]).ok_or(Error::Malformed)?,
        });
    }
    if encode_catalogue(&entries)? != bytes {
        return Err(Error::Malformed);
    }
    Ok(entries)
}
pub fn encode_query(
    query: HighscoresQuery,
    world_type: Option<WorldType>,
) -> Result<Vec<u8>, Error> {
    let mut out = vec![world_type.map_or(0, |kind| kind as u8)];
    out.extend(highscores::encode_highscores_query(query)?);
    Ok(out)
}
pub fn decode_query(bytes: &[u8]) -> Result<(HighscoresQuery, Option<WorldType>), Error> {
    let (&id, rest) = bytes.split_first().ok_or(Error::Malformed)?;
    let kind = if id == 0 {
        None
    } else {
        Some(WorldType::from_id(id).ok_or(Error::Malformed)?)
    };
    Ok((highscores::decode_highscores_query(rest)?, kind))
}
