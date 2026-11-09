//! Self appearance gallery. Familiar entries describe the active content catalogue,
//! never an account unlock; outfit and mount entries are confirmed available choices.
use crate::charm_wire::CyclopediaWireError as Error;
pub const SELECTOR: u8 = 129;
pub const PAGE_SIZE: usize = 24;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Kind {
    Outfit = 1,
    Mount = 2,
    Familiar = 3,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub kind: Kind,
    pub key: String,
    pub look_type: u32,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    pub offset: u16,
    pub total: u16,
    pub entries: Vec<Entry>,
}
pub fn request(offset: u16) -> Vec<u8> {
    let mut out = vec![SELECTOR];
    out.extend_from_slice(&offset.to_le_bytes());
    out
}
pub fn decode_request(bytes: &[u8]) -> Result<u16, Error> {
    if bytes.len() != 3 || bytes[0] != SELECTOR {
        return Err(Error::Malformed);
    }
    Ok(u16::from_le_bytes([bytes[1], bytes[2]]))
}
impl Page {
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        if self.offset > self.total
            || self.entries.len()
                != usize::from(self.total.saturating_sub(self.offset)).min(PAGE_SIZE)
            || self.entries.len() > PAGE_SIZE
            || usize::from(self.offset) + self.entries.len() > usize::from(self.total)
        {
            return Err(Error::Malformed);
        }
        let mut out = request(self.offset);
        out.extend_from_slice(&self.total.to_le_bytes());
        out.push(self.entries.len() as u8);
        for entry in &self.entries {
            if entry.key.is_empty()
                || entry.key.len() > 255
                || entry.key.contains(['\0', '\r', '\n'])
                || entry.look_type == 0
            {
                return Err(Error::Malformed);
            }
            out.push(entry.kind as u8);
            out.extend_from_slice(&entry.look_type.to_le_bytes());
            out.push(entry.key.len() as u8);
            out.extend_from_slice(entry.key.as_bytes());
        }
        Ok(out)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() < 6 || bytes[0] != SELECTOR || bytes.len() > 6270 {
            return Err(Error::Malformed);
        }
        let mut page = Self {
            offset: u16::from_le_bytes([bytes[1], bytes[2]]),
            total: u16::from_le_bytes([bytes[3], bytes[4]]),
            entries: Vec::new(),
        };
        let mut tail = &bytes[6..];
        for _ in 0..bytes[5] {
            if tail.len() < 6 {
                return Err(Error::Malformed);
            }
            let kind = match tail[0] {
                1 => Kind::Outfit,
                2 => Kind::Mount,
                3 => Kind::Familiar,
                _ => return Err(Error::Malformed),
            };
            let look_type =
                u32::from_le_bytes(tail[1..5].try_into().map_err(|_| Error::Malformed)?);
            let len = usize::from(tail[5]);
            tail = &tail[6..];
            let (key, rest) = tail.split_at_checked(len).ok_or(Error::Malformed)?;
            tail = rest;
            page.entries.push(Entry {
                kind,
                key: std::str::from_utf8(key)
                    .map_err(|_| Error::Malformed)?
                    .into(),
                look_type,
            });
        }
        if !tail.is_empty() || page.encode()? != bytes {
            return Err(Error::Malformed);
        }
        Ok(page)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gallery_wire_rejects_every_truncation_and_foreign_kind() -> Result<(), Error> {
        let page = Page {
            offset: 0,
            total: 1,
            entries: vec![Entry {
                kind: Kind::Mount,
                key: "oteryn:mount/horse".into(),
                look_type: 1,
            }],
        };
        let bytes = page.encode()?;
        assert_eq!(Page::decode(&bytes)?, page);
        for end in 0..bytes.len() {
            assert!(Page::decode(&bytes[..end]).is_err());
        }
        let mut bad = bytes;
        bad[6] = 4;
        assert!(Page::decode(&bad).is_err());
        Ok(())
    }
}
