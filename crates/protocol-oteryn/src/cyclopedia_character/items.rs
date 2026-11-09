//! Complete self custody projection, grouped by definition across four locations.
use crate::charm_wire::CyclopediaWireError as Error;
pub const SELECTOR: u8 = 130;
pub const PAGE_SIZE: usize = 24;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum Location {
    Inventory,
    Depot,
    Stash,
    Inbox,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub definition: String,
    pub quantities: [u64; 4],
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    pub offset: u16,
    pub total: u16,
    pub rows: Vec<Row>,
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
            || self.rows.len() != usize::from(self.total - self.offset).min(PAGE_SIZE)
        {
            return Err(Error::Malformed);
        }
        let mut out = request(self.offset);
        out.extend_from_slice(&self.total.to_le_bytes());
        out.push(self.rows.len() as u8);
        let mut previous = "";
        for row in &self.rows {
            if row.definition.is_empty()
                || row.definition.len() > 512
                || row.definition.contains(['\0', '\r', '\n'])
                || row.definition.as_str() <= previous
                || row.quantities == [0; 4]
            {
                return Err(Error::Malformed);
            }
            previous = &row.definition;
            out.extend_from_slice(&(row.definition.len() as u16).to_le_bytes());
            out.extend_from_slice(row.definition.as_bytes());
            for quantity in row.quantities {
                out.extend_from_slice(&quantity.to_le_bytes());
            }
        }
        Ok(out)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() < 6 || bytes.len() > 13110 || bytes[0] != SELECTOR {
            return Err(Error::Malformed);
        }
        let mut page = Self {
            offset: u16::from_le_bytes([bytes[1], bytes[2]]),
            total: u16::from_le_bytes([bytes[3], bytes[4]]),
            rows: Vec::new(),
        };
        let mut tail = &bytes[6..];
        for _ in 0..bytes[5] {
            let header = tail.get(..2).ok_or(Error::Malformed)?;
            let len = usize::from(u16::from_le_bytes([header[0], header[1]]));
            tail = &tail[2..];
            let (key, rest) = tail.split_at_checked(len).ok_or(Error::Malformed)?;
            tail = rest;
            let mut quantities = [0; 4];
            for quantity in &mut quantities {
                let (v, rest) = tail.split_at_checked(8).ok_or(Error::Malformed)?;
                *quantity = u64::from_le_bytes(v.try_into().map_err(|_| Error::Malformed)?);
                tail = rest;
            }
            page.rows.push(Row {
                definition: std::str::from_utf8(key)
                    .map_err(|_| Error::Malformed)?
                    .into(),
                quantities,
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
    fn item_summary_codec_keeps_four_locations_and_rejects_truncation() -> Result<(), Error> {
        let page = Page {
            offset: 0,
            total: 1,
            rows: vec![Row {
                definition: "oteryn:item/sword".into(),
                quantities: [1, 2, 3, 4],
            }],
        };
        let bytes = page.encode()?;
        assert_eq!(Page::decode(&bytes)?, page);
        for end in 0..bytes.len() {
            assert!(Page::decode(&bytes[..end]).is_err());
        }
        Ok(())
    }
}
