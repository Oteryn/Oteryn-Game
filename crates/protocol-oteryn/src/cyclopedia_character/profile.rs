//! A coherent, consent-controlled identity, badge and equipment snapshot.
use super::{Badge, EquippedItem, encode_badges};
use crate::charm_wire::CyclopediaWireError as Error;
pub const SELECTOR: u8 = 129;
pub const MAX_ITEMS: usize = 32;
pub const MAX_BYTES: usize = 17_000;
pub fn request(target: Option<[u8; 16]>) -> Vec<u8> {
    let mut out = vec![SELECTOR];
    if let Some(target) = target {
        out.extend_from_slice(&target);
    }
    out
}
pub fn decode_request(bytes: &[u8]) -> Result<Option<[u8; 16]>, Error> {
    match bytes {
        [SELECTOR] => Ok(None),
        [SELECTOR, rest @ ..] if rest.len() == 16 => {
            Ok(Some(rest.try_into().map_err(|_| Error::Malformed)?))
        }
        _ => Err(Error::Malformed),
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub character: [u8; 16],
    pub name: String,
    pub badges: Vec<Badge>,
    pub equipment: Vec<EquippedItem>,
}
impl Profile {
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        if !(2..=29).contains(&self.name.len())
            || self.name.contains(['\0', '\r', '\n'])
            || self.equipment.len() > MAX_ITEMS
        {
            return Err(Error::Malformed);
        }
        let mut out = vec![SELECTOR];
        out.extend_from_slice(&self.character);
        out.push(self.name.len() as u8);
        out.extend_from_slice(self.name.as_bytes());
        out.extend_from_slice(&encode_badges(&self.badges).map_err(|_| Error::Malformed)?);
        out.push(self.equipment.len() as u8);
        let mut slots = Vec::new();
        for item in &self.equipment {
            if item.definition.is_empty()
                || item.definition.len() > 512
                || item.definition.contains(['\0', '\r', '\n'])
                || item.quantity == 0
                || slots.contains(&item.slot)
            {
                return Err(Error::Malformed);
            }
            slots.push(item.slot);
            out.push(item.slot);
            out.extend_from_slice(&(item.definition.len() as u16).to_le_bytes());
            out.extend_from_slice(item.definition.as_bytes());
            out.extend_from_slice(&item.quantity.to_le_bytes());
        }
        if out.len() > MAX_BYTES {
            return Err(Error::LimitExceeded);
        }
        Ok(out)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() > MAX_BYTES {
            return Err(Error::LimitExceeded);
        }
        let mut r = Reader(bytes);
        if r.byte()? != SELECTOR {
            return Err(Error::Malformed);
        }
        let character = r.take(16)?.try_into().map_err(|_| Error::Malformed)?;
        let len = usize::from(r.byte()?);
        let name = r.text(len)?;
        let count = r.byte()?;
        let mut badges = Vec::new();
        for _ in 0..count {
            badges.push(Badge::from_id(r.byte()?).ok_or(Error::Malformed)?);
        }
        let count = r.byte()?;
        let mut equipment = Vec::new();
        for _ in 0..count {
            let slot = r.byte()?;
            let len = u16::from_le_bytes(r.take(2)?.try_into().map_err(|_| Error::Malformed)?);
            let definition = r.text(usize::from(len))?;
            let quantity = u32::from_le_bytes(r.take(4)?.try_into().map_err(|_| Error::Malformed)?);
            equipment.push(EquippedItem {
                slot,
                definition,
                quantity,
            });
        }
        let value = Self {
            character,
            name,
            badges,
            equipment,
        };
        if !r.0.is_empty() || value.encode()?.as_slice() != bytes {
            return Err(Error::Malformed);
        }
        Ok(value)
    }
}
struct Reader<'a>(&'a [u8]);
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let value = self.0.get(..n).ok_or(Error::Malformed)?;
        self.0 = &self.0[n..];
        Ok(value)
    }
    fn byte(&mut self) -> Result<u8, Error> {
        Ok(self.take(1)?[0])
    }
    fn text(&mut self, n: usize) -> Result<String, Error> {
        Ok(std::str::from_utf8(self.take(n)?)
            .map_err(|_| Error::Malformed)?
            .into())
    }
}
#[cfg(test)]
#[path = "profile_tests.rs"]
mod tests;
