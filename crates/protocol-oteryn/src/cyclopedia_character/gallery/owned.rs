//! The character's available appearances, with sources backed by the ownership record.
use super::{Error, Kind, PAGE_SIZE};

pub const CAPABILITY: u32 = 611;
pub const SELECTOR: u8 = 136;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Source {
    Starter = 1,
    Quest = 2,
    Store = 4,
    Npc = 8,
    Taming = 16,
    Item = 32,
    TaskBoard = 64,
    FamiliarGrant = 128,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Query {
    pub offset: u16,
    /// Choosing a source returns only outfits with that acquisition source.
    pub outfit_source: Option<Source>,
}

impl Query {
    pub fn encode(self) -> [u8; 4] {
        let [a, b] = self.offset.to_le_bytes();
        [
            SELECTOR,
            a,
            b,
            self.outfit_source.map_or(0, |source| source as u8),
        ]
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        let [SELECTOR, a, b, source] = *bytes else {
            return Err(Error::Malformed);
        };
        let outfit_source = match source {
            0 => None,
            1 => Some(Source::Starter),
            2 => Some(Source::Quest),
            4 => Some(Source::Store),
            8 => Some(Source::Npc),
            16 => Some(Source::Taming),
            32 => Some(Source::Item),
            64 => Some(Source::TaskBoard),
            128 => Some(Source::FamiliarGrant),
            _ => return Err(Error::Malformed),
        };
        Ok(Self {
            offset: u16::from_le_bytes([a, b]),
            outfit_source,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    pub gallery: super::Page,
    /// An acquisition-source bitmask per entry. Every entry is owned.
    pub sources: Vec<u8>,
}

impl Page {
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        if self.sources.len() != self.gallery.entries.len()
            || self
                .gallery
                .entries
                .iter()
                .zip(&self.sources)
                .any(|(entry, mask)| {
                    if entry.kind == Kind::Familiar {
                        *mask != Source::FamiliarGrant as u8
                    } else {
                        *mask == 0 || mask & Source::FamiliarGrant as u8 != 0
                    }
                })
        {
            return Err(Error::Malformed);
        }
        let gallery = self.gallery.encode()?;
        let mut bytes = vec![SELECTOR];
        bytes.extend(
            u16::try_from(gallery.len())
                .map_err(|_| Error::Malformed)?
                .to_le_bytes(),
        );
        bytes.extend(gallery);
        bytes.extend(&self.sources);
        Ok(bytes)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        let [SELECTOR, a, b, tail @ ..] = bytes else {
            return Err(Error::Malformed);
        };
        let (gallery, sources) = tail
            .split_at_checked(usize::from(u16::from_le_bytes([*a, *b])))
            .ok_or(Error::Malformed)?;
        if sources.len() > PAGE_SIZE {
            return Err(Error::Malformed);
        }
        let page = Self {
            gallery: super::Page::decode(gallery)?,
            sources: sources.to_vec(),
        };
        page.encode()?;
        Ok(page)
    }
}
