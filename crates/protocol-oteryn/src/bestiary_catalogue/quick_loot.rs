//! Opt-in Bestiary detail with Quick Loot definition references for the displayed rows.
use super::{Error, PAGE_SIZE, Request, Result, decode_page, encode_page};
use std::num::NonZeroU32;

pub const CAPABILITY: u32 = 609;
pub const PREFIX: [u8; 2] = [255, 4];
const MAX_REPLY_BYTES: usize = super::MAX_BYTES;

pub fn request(query: Request) -> Result<Vec<u8>> {
    if !(1..=1024).contains(&query.race) || query.after > 8192 || query.category != 0 {
        return Err(Error::Malformed);
    }
    let mut bytes = PREFIX.to_vec();
    bytes.extend(query.after.to_le_bytes());
    bytes.extend(query.race.to_le_bytes());
    Ok(bytes)
}

pub fn decode_request(bytes: &[u8]) -> Result<Request> {
    let [255, 4, a, b, c, d] = *bytes else {
        return Err(Error::Malformed);
    };
    let query = Request {
        after: u16::from_le_bytes([a, b]),
        race: u16::from_le_bytes([c, d]),
        category: 0,
    };
    request(query)?;
    Ok(query)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    pub catalogue: super::Page,
    /// One reference per loot line, in display order. None means the line cannot be edited.
    pub references: Vec<Option<NonZeroU32>>,
}

impl Page {
    pub fn encode(&self) -> Result<Vec<u8>> {
        if self.catalogue.cards.len() != 1
            || self.references.len() > PAGE_SIZE
            || (self.catalogue.next.is_some() && self.references.len() != PAGE_SIZE)
        {
            return Err(Error::Malformed);
        }
        let catalogue = encode_page(&self.catalogue)?;
        let mut bytes = PREFIX.to_vec();
        bytes.extend(
            u16::try_from(catalogue.len())
                .map_err(|_| Error::LimitExceeded)?
                .to_le_bytes(),
        );
        bytes.extend(catalogue);
        for reference in &self.references {
            bytes.extend(reference.map_or(0, NonZeroU32::get).to_le_bytes());
        }
        if bytes.len() > MAX_REPLY_BYTES {
            return Err(Error::LimitExceeded);
        }
        Ok(bytes)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let [255, 4, a, b, tail @ ..] = bytes else {
            return Err(Error::Malformed);
        };
        if bytes.len() > MAX_REPLY_BYTES {
            return Err(Error::LimitExceeded);
        }
        let (catalogue, refs) = tail
            .split_at_checked(usize::from(u16::from_le_bytes([*a, *b])))
            .ok_or(Error::Malformed)?;
        let (refs, rest) = refs.as_chunks::<4>();
        if !rest.is_empty() || refs.len() > PAGE_SIZE {
            return Err(Error::Malformed);
        }
        let page = Self {
            catalogue: decode_page(catalogue)?,
            references: refs
                .iter()
                .map(|r| NonZeroU32::new(u32::from_le_bytes(*r)))
                .collect(),
        };
        page.encode()?;
        Ok(page)
    }
}
