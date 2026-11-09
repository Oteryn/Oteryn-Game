//! Public-template browsing and inverse loot-source pages within command 153.
use super::{Result, WireError};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Query {
    Items {
        after: u32,
        category: u8,
        search: [u8; 64],
        len: u8,
    },
    Drops {
        reference: u32,
        after: u16,
    },
}
pub fn items(after: u32, category: u8, search: &str) -> Result<Query> {
    if search.len() > 64 || search.contains('\0') || category > 64 {
        return Err(WireError::LimitExceeded);
    }
    let mut bytes = [0; 64];
    bytes[..search.len()].copy_from_slice(search.as_bytes());
    Ok(Query::Items {
        after,
        category,
        search: bytes,
        len: search.len() as u8,
    })
}
pub fn search(query: Query) -> Result<String> {
    let Query::Items { search, len, .. } = query else {
        return Err(WireError::Malformed);
    };
    let len = usize::from(len);
    if len > search.len() || search[len..].iter().any(|b| *b != 0) {
        return Err(WireError::Malformed);
    }
    let value = std::str::from_utf8(&search[..len]).map_err(|_| WireError::Malformed)?;
    if value.contains('\0') {
        return Err(WireError::Malformed);
    }
    Ok(value.into())
}
pub fn encode(query: Query) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    match query {
        Query::Items {
            after, category, ..
        } => {
            if category > 64 {
                return Err(WireError::LimitExceeded);
            }
            out.push(1);
            out.extend(after.to_le_bytes());
            out.push(category);
            out.extend(search(query)?.as_bytes());
        }
        Query::Drops { reference, after } if reference > 0 && after <= 1024 => {
            out.push(2);
            out.extend(reference.to_le_bytes());
            out.extend(after.to_le_bytes());
        }
        _ => return Err(WireError::Malformed),
    }
    Ok(out)
}
pub fn decode(bytes: &[u8]) -> Result<Query> {
    if bytes.len() > 70 || bytes.len() < 6 {
        return Err(WireError::Malformed);
    }
    let value = u32::from_le_bytes(bytes[1..5].try_into().map_err(|_| WireError::Malformed)?);
    let query = match bytes[0] {
        1 => items(
            value,
            bytes[5],
            std::str::from_utf8(&bytes[6..]).map_err(|_| WireError::Malformed)?,
        )?,
        2 if bytes.len() == 7 => Query::Drops {
            reference: value,
            after: u16::from_le_bytes([bytes[5], bytes[6]]),
        },
        _ => return Err(WireError::Malformed),
    };
    encode(query)?;
    Ok(query)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn item_catalogue_queries_are_bounded_and_round_trip() -> Result<()> {
        for query in [
            items(10, 2, "silver")?,
            Query::Drops {
                reference: 7,
                after: 10,
            },
        ] {
            assert_eq!(decode(&encode(query)?)?, query);
        }
        assert!(items(0, 0, &"x".repeat(65)).is_err());
        assert!(items(0, 65, "").is_err());
        assert!(decode(&[2, 0, 0, 0, 0, 0, 0]).is_err());
        assert!(decode(&[1, 0, 0, 0, 0, 0, 255]).is_err());
        Ok(())
    }
}
// Bounded public-template discovery under the existing inspection capability.
use super::*;
pub const MAX_SEARCH_BYTES: usize = 48;
pub const MAX_CATEGORY_BYTES: usize = 32;
pub const MAX_CATEGORIES: usize = 64;
pub const SCAN_LIMIT: usize = 256;
pub const SOURCES_PER_PAGE: usize = 10;

/// Fixed capacity keeps the existing inspection request a Copy value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageQuery {
    after: u32,
    search_len: u8,
    search: [u8; MAX_SEARCH_BYTES],
    category_len: u8,
    category: [u8; MAX_CATEGORY_BYTES],
}
impl PageQuery {
    pub fn new(after: u32, search: &str, category: &str) -> Result<Self> {
        if search.len() > MAX_SEARCH_BYTES
            || category.len() > MAX_CATEGORY_BYTES
            || search.contains('\0')
            || category.contains('\0')
        {
            return bad();
        }
        let mut query = Self {
            after,
            search_len: search.len() as u8,
            search: [0; MAX_SEARCH_BYTES],
            category_len: category.len() as u8,
            category: [0; MAX_CATEGORY_BYTES],
        };
        query.search[..search.len()].copy_from_slice(search.as_bytes());
        query.category[..category.len()].copy_from_slice(category.as_bytes());
        Ok(query)
    }
    pub const fn after(self) -> u32 {
        self.after
    }
    pub fn search(&self) -> &str {
        std::str::from_utf8(&self.search[..usize::from(self.search_len)]).unwrap_or("")
    }
    pub fn category(&self) -> &str {
        std::str::from_utf8(&self.category[..usize::from(self.category_len)]).unwrap_or("")
    }
    pub fn matches(&self, item: &InspectedItem) -> bool {
        item.name
            .to_lowercase()
            .contains(&self.search().trim().to_lowercase())
            && (self.category().is_empty()
                || categories(item).any(|value| value.eq_ignore_ascii_case(self.category())))
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    /// Last scanned definition or creature row, according to the request variant.
    pub next: Option<u32>,
    pub categories: Vec<String>,
}
pub fn categories(item: &InspectedItem) -> impl Iterator<Item = &str> {
    item.attributes.iter().filter_map(|(key, value)| {
        (key.eq_ignore_ascii_case("category") || key == "Weapon Type" || key == "Body Position")
            .then(|| value.split(',').next().unwrap_or("").trim())
            .filter(|value| !value.is_empty())
    })
}
pub(super) fn encode_query(query: PageQuery) -> Vec<u8> {
    let mut out = Vec::new();
    push_varint_field(&mut out, 1, u64::from(query.after));
    push_message_field(&mut out, 2, query.search().as_bytes());
    push_message_field(&mut out, 3, query.category().as_bytes());
    out
}
pub(super) fn decode_query(input: &[u8]) -> Result<PageQuery> {
    let (mut after, mut search, mut category) = (None, None, None);
    fields(input, |key, input, cursor| {
        match key {
            8 => set_once(&mut after, read_uint32(input, cursor)?)?,
            18 => set_once(
                &mut search,
                std::str::from_utf8(read_bytes(input, cursor)?)
                    .map_err(|_| WireError::Malformed)?
                    .to_owned(),
            )?,
            26 => set_once(
                &mut category,
                std::str::from_utf8(read_bytes(input, cursor)?)
                    .map_err(|_| WireError::Malformed)?
                    .to_owned(),
            )?,
            _ => return bad(),
        }
        Ok(())
    })?;
    PageQuery::new(
        after.ok_or(WireError::Malformed)?,
        search.as_deref().ok_or(WireError::Malformed)?,
        category.as_deref().ok_or(WireError::Malformed)?,
    )
}
pub(super) fn encode_page(page: &Page) -> Result<Vec<u8>> {
    if page.next == Some(0) || page.categories.len() > MAX_CATEGORIES {
        return bad();
    }
    let mut out = Vec::new();
    if let Some(next) = page.next {
        push_varint_field(&mut out, 1, u64::from(next));
    }
    for (index, category) in page.categories.iter().enumerate() {
        if category.is_empty()
            || category.len() > MAX_CATEGORY_BYTES
            || category.contains('\0')
            || page.categories[..index].contains(category)
        {
            return bad();
        }
        push_message_field(&mut out, 2, category.as_bytes());
    }
    Ok(out)
}
pub(super) fn decode_page(input: &[u8]) -> Result<Page> {
    let mut next = None;
    let mut categories = Vec::new();
    fields(input, |key, input, cursor| {
        match key {
            8 => set_once(&mut next, read_uint32(input, cursor)?)?,
            18 => {
                if categories.len() == MAX_CATEGORIES {
                    return bad();
                }
                categories.push(text(read_bytes(input, cursor)?)?);
            }
            _ => return bad(),
        }
        Ok(())
    })?;
    let page = Page { next, categories };
    encode_page(&page)?;
    Ok(page)
}

pub(super) fn encode_sources(definition: u32, after: u32) -> Vec<u8> {
    let mut out = Vec::new();
    push_varint_field(&mut out, 1, u64::from(definition));
    push_varint_field(&mut out, 2, u64::from(after));
    out
}
pub(super) fn decode_sources(input: &[u8]) -> Result<(u32, u32)> {
    let (mut definition, mut after) = (None, None);
    fields(input, |key, input, cursor| {
        match key {
            8 => set_once(&mut definition, read_uint32(input, cursor)?)?,
            16 => set_once(&mut after, read_uint32(input, cursor)?)?,
            _ => return bad(),
        }
        Ok(())
    })?;
    let definition = definition
        .filter(|value| *value != 0)
        .ok_or(WireError::Malformed)?;
    Ok((definition, after.ok_or(WireError::Malformed)?))
}
