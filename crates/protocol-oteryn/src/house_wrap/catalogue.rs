//! Bounded native furniture selection. UUIDs and revisions stay inside the codec.
use super::{Action, Request};

pub const CATALOGUE_PAGE_SIZE: usize = 8;
const MAX_PAGE: u16 = 252; // House limit 2,000 plus the supported 20-slot backpack.

pub fn encode_catalogue_query(page: u16) -> Option<Vec<u8>> {
    (page <= MAX_PAGE).then(|| [vec![3], page.to_be_bytes().to_vec()].concat())
}
pub fn decode_catalogue_query(bytes: &[u8]) -> Option<u16> {
    let [3, hi, lo] = bytes else { return None };
    let page = u16::from_be_bytes([*hi, *lo]);
    (page <= MAX_PAGE).then_some(page)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogueEntry {
    pub item: [u8; 16],
    pub revision: u64,
    pub action: Action,
    pub name: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Catalogue {
    pub page: u16,
    pub more: bool,
    pub acl_revision: u64,
    pub position: [i32; 3],
    pub entries: Vec<CatalogueEntry>,
}
impl Catalogue {
    pub fn request(&self, index: usize) -> Option<Request> {
        let entry = self.entries.get(index)?;
        Some(Request {
            action: entry.action,
            item: entry.item,
            expected_revision: entry.revision,
            acl_revision: self.acl_revision,
            position: self.position,
        })
    }
    pub fn encode(&self) -> Option<Vec<u8>> {
        if self.page > MAX_PAGE
            || self.acl_revision == 0
            || self.entries.len() > CATALOGUE_PAGE_SIZE
        {
            return None;
        }
        let mut out = vec![7];
        out.extend(self.page.to_be_bytes());
        out.push(u8::from(self.more));
        out.extend(self.acl_revision.to_be_bytes());
        for value in self.position {
            out.extend(value.to_be_bytes());
        }
        out.push(u8::try_from(self.entries.len()).ok()?);
        for entry in &self.entries {
            if entry.item == [0; 16]
                || entry.revision == 0
                || entry.name.is_empty()
                || entry.name.len() > 120
                || entry.name.contains('\0')
            {
                return None;
            }
            out.extend(entry.item);
            out.extend(entry.revision.to_be_bytes());
            out.push(entry.action as u8);
            out.push(u8::try_from(entry.name.len()).ok()?);
            out.extend(entry.name.as_bytes());
        }
        Some(out)
    }
    pub fn decode(mut bytes: &[u8]) -> Option<Self> {
        fn take<'a>(bytes: &mut &'a [u8], len: usize) -> Option<&'a [u8]> {
            let (head, tail) = bytes.split_at_checked(len)?;
            *bytes = tail;
            Some(head)
        }
        if bytes.len() > 2048 || take(&mut bytes, 1)? != [7] {
            return None;
        }
        let page = u16::from_be_bytes(take(&mut bytes, 2)?.try_into().ok()?);
        let more = match take(&mut bytes, 1)? {
            [0] => false,
            [1] => true,
            _ => return None,
        };
        let acl_revision = u64::from_be_bytes(take(&mut bytes, 8)?.try_into().ok()?);
        let mut position = [0; 3];
        for value in &mut position {
            *value = i32::from_be_bytes(take(&mut bytes, 4)?.try_into().ok()?);
        }
        let count = usize::from(*take(&mut bytes, 1)?.first()?);
        if count > CATALOGUE_PAGE_SIZE {
            return None;
        }
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            let item = take(&mut bytes, 16)?.try_into().ok()?;
            let revision = u64::from_be_bytes(take(&mut bytes, 8)?.try_into().ok()?);
            let action = match take(&mut bytes, 1)? {
                [1] => Action::Wrap,
                [2] => Action::Unwrap,
                _ => return None,
            };
            let len = usize::from(*take(&mut bytes, 1)?.first()?);
            let name = std::str::from_utf8(take(&mut bytes, len)?).ok()?.to_owned();
            if entries
                .iter()
                .any(|entry: &CatalogueEntry| entry.item == item)
            {
                return None;
            }
            entries.push(CatalogueEntry {
                item,
                revision,
                action,
                name,
            });
        }
        let result = Self {
            page,
            more,
            acl_revision,
            position,
            entries,
        };
        (bytes.is_empty() && result.encode().is_some()).then_some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_furniture_catalogue_is_bounded_exact_and_utf8()
    -> Result<(), Box<dyn std::error::Error>> {
        let catalogue = Catalogue {
            page: 0,
            more: false,
            acl_revision: 1,
            position: [1, 2, 7],
            entries: vec![CatalogueEntry {
                item: [1; 16],
                revision: 1,
                action: Action::Wrap,
                name: "oven żółw".into(),
            }],
        };
        let bytes = catalogue.encode().ok_or("fixture")?;
        assert_eq!(Catalogue::decode(&bytes), Some(catalogue.clone()));
        for end in 0..bytes.len() {
            assert!(Catalogue::decode(&bytes[..end]).is_none());
        }
        let mut extra = bytes;
        extra.push(0);
        assert!(Catalogue::decode(&extra).is_none());
        let mut duplicate = catalogue.clone();
        duplicate.entries.push(duplicate.entries[0].clone());
        assert!(Catalogue::decode(&duplicate.encode().ok_or("fixture")?).is_none());
        let mut too_many = catalogue;
        too_many.entries = vec![too_many.entries[0].clone(); CATALOGUE_PAGE_SIZE + 1];
        assert!(too_many.encode().is_none());
        assert_eq!(decode_catalogue_query(&[3, 0, 0]), Some(0));
        assert!(decode_catalogue_query(&[3, 0, 0, 0]).is_none());
        assert!(encode_catalogue_query(253).is_none());
        Ok(())
    }
}
