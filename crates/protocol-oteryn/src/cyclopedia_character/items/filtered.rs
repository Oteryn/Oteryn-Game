//! Capability 506: name-aware holdings filtering before server paging.
use super::{Location, Page as HoldingsPage};
use crate::charm_wire::CyclopediaWireError as Error;
pub const CAPABILITY: u32 = 506;
pub const SELECTOR: u8 = 135;
pub const MAX_SEARCH_BYTES: usize = 96;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    pub offset: u16,
    pub location: Option<Location>,
    pub search: String,
}
impl Query {
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        if self.search.len() > MAX_SEARCH_BYTES || self.search.contains(['\0', '\r', '\n']) {
            return Err(Error::Malformed);
        }
        let mut bytes = vec![SELECTOR];
        bytes.extend(self.offset.to_le_bytes());
        bytes.push(self.location.map_or(0, |location| location as u8 + 1));
        bytes.push(self.search.len() as u8);
        bytes.extend(self.search.as_bytes());
        Ok(bytes)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() < 5 || bytes[0] != SELECTOR || bytes.len() != 5 + usize::from(bytes[4]) {
            return Err(Error::Malformed);
        }
        let location = match bytes[3] {
            0 => None,
            1 => Some(Location::Inventory),
            2 => Some(Location::Depot),
            3 => Some(Location::Stash),
            4 => Some(Location::Inbox),
            _ => return Err(Error::Malformed),
        };
        let query = Self {
            offset: u16::from_le_bytes([bytes[1], bytes[2]]),
            location,
            search: std::str::from_utf8(&bytes[5..])
                .map_err(|_| Error::Malformed)?
                .into(),
        };
        if query.encode()? != bytes {
            return Err(Error::Malformed);
        }
        Ok(query)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    pub holdings: HoldingsPage,
    /// Names correspond exactly to the ordered holdings rows; unknown definitions retain their key.
    pub names: Vec<String>,
}
impl Page {
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        if self.names.len() != self.holdings.rows.len() {
            return Err(Error::Malformed);
        }
        let page = self.holdings.encode()?;
        let mut bytes = vec![SELECTOR];
        bytes.extend((page.len() as u16).to_le_bytes());
        bytes.extend(page);
        for name in &self.names {
            if name.is_empty() || name.len() > 512 || name.contains(['\0', '\r', '\n']) {
                return Err(Error::Malformed);
            }
            bytes.extend((name.len() as u16).to_le_bytes());
            bytes.extend(name.as_bytes());
        }
        Ok(bytes)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() < 3 || bytes.len() > 25500 || bytes[0] != SELECTOR {
            return Err(Error::Malformed);
        }
        let size = usize::from(u16::from_le_bytes([bytes[1], bytes[2]]));
        let (page, mut rest) = bytes[3..].split_at_checked(size).ok_or(Error::Malformed)?;
        let holdings = HoldingsPage::decode(page)?;
        let mut names = Vec::with_capacity(holdings.rows.len());
        for _ in &holdings.rows {
            let (length, tail) = rest.split_at_checked(2).ok_or(Error::Malformed)?;
            let size = usize::from(u16::from_le_bytes([length[0], length[1]]));
            let (name, tail) = tail.split_at_checked(size).ok_or(Error::Malformed)?;
            names.push(
                std::str::from_utf8(name)
                    .map_err(|_| Error::Malformed)?
                    .into(),
            );
            rest = tail;
        }
        let page = Self { holdings, names };
        if !rest.is_empty() || page.encode()? != bytes {
            return Err(Error::Malformed);
        }
        Ok(page)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn holdings_search_selector_does_not_shadow_blessings_dialog() {
        assert_ne!(
            super::SELECTOR,
            crate::cyclopedia_character::blessings_dialog::SELECTOR
        );
    }
}
