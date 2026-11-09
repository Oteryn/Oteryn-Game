//! Store browse view; each product retains its own immutable purchase quote.
use super::*;
pub const COMMAND_TYPE: u32 = 6118;
pub const MAX_CATEGORIES: usize = 64;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    pub category: String,
    pub search: String,
    pub sort: u8,
    pub cursor: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Category {
    pub key: String,
    pub parent: String,
    pub title: String,
}
/// 0 service, 1 item, 2 outfit, 3 mount. Two look IDs represent male/female views.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Artwork {
    pub kind: u8,
    pub looks: [u32; 2],
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Product {
    pub category: String,
    pub revision: String,
    pub offer: StoreOffer,
    pub artwork: Artwork,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    pub categories: Vec<Category>,
    pub products: Vec<Product>,
    pub next: String,
    pub total: u64,
    pub transferable: u64,
    pub transferable_known: bool,
}
fn blob(out: &mut Vec<u8>, bytes: &[u8]) -> Result<(), StoreWireError> {
    let n = u16::try_from(bytes.len()).map_err(|_| StoreWireError::LimitExceeded)?;
    out.extend(n.to_le_bytes());
    out.extend(bytes);
    Ok(())
}
fn string(out: &mut Vec<u8>, s: &str, max: usize) -> Result<(), StoreWireError> {
    if s.len() > max || s.chars().any(char::is_control) {
        return Err(StoreWireError::Malformed);
    }
    blob(out, s.as_bytes())
}
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Reader<'a> {
    fn take<const N: usize>(&mut self) -> Result<[u8; N], StoreWireError> {
        let b = self
            .bytes
            .get(self.at..self.at + N)
            .ok_or(StoreWireError::Malformed)?;
        self.at += N;
        b.try_into().map_err(|_| StoreWireError::Malformed)
    }
    fn blob(&mut self) -> Result<&'a [u8], StoreWireError> {
        let n = usize::from(u16::from_le_bytes(self.take()?));
        let b = self
            .bytes
            .get(self.at..self.at + n)
            .ok_or(StoreWireError::Malformed)?;
        self.at += n;
        Ok(b)
    }
    fn text(&mut self) -> Result<String, StoreWireError> {
        Ok(std::str::from_utf8(self.blob()?)
            .map_err(|_| StoreWireError::Malformed)?
            .to_owned())
    }
    fn end(&self) -> Result<(), StoreWireError> {
        if self.at == self.bytes.len() {
            Ok(())
        } else {
            Err(StoreWireError::Malformed)
        }
    }
}
impl Query {
    pub fn encode(&self) -> Result<Vec<u8>, StoreWireError> {
        if !valid_key(&self.category) || self.sort > 4 {
            return Err(StoreWireError::Malformed);
        }
        let mut b = b"SBQ1".to_vec();
        string(&mut b, &self.category, 128)?;
        string(&mut b, &self.search, 128)?;
        b.push(self.sort);
        string(&mut b, &self.cursor, 128)?;
        Ok(b)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, StoreWireError> {
        if bytes.len() > MAX_STORE_REQUEST_BYTES {
            return Err(StoreWireError::LimitExceeded);
        }
        let mut r = Reader { bytes, at: 0 };
        if r.take::<4>()? != *b"SBQ1" {
            return Err(StoreWireError::Malformed);
        }
        let q = Self {
            category: r.text()?,
            search: r.text()?,
            sort: r.take::<1>()?[0],
            cursor: r.text()?,
        };
        r.end()?;
        q.encode()?;
        Ok(q)
    }
}
impl Page {
    pub fn encode(&self) -> Result<Vec<u8>, StoreWireError> {
        if self.categories.len() > MAX_CATEGORIES
            || self.products.len() > MAX_STORE_PAGE_ENTRIES
            || self.transferable > self.total
            || (!self.transferable_known && self.transferable != 0)
        {
            return Err(StoreWireError::LimitExceeded);
        }
        let mut b = b"SBR1".to_vec();
        b.extend(self.total.to_le_bytes());
        b.extend(self.transferable.to_le_bytes());
        b.push(u8::from(self.transferable_known));
        b.push(self.categories.len() as u8);
        for (i, c) in self.categories.iter().enumerate() {
            if !valid_key(&c.key)
                || self.categories[..i].iter().any(|p| p.key == c.key)
                || (!c.parent.is_empty() && !self.categories[..i].iter().any(|p| p.key == c.parent))
                || !valid_text(&c.title, 256)
            {
                return Err(StoreWireError::Malformed);
            }
            string(&mut b, &c.key, 128)?;
            string(&mut b, &c.parent, 128)?;
            string(&mut b, &c.title, 256)?;
        }
        b.push(self.products.len() as u8);
        for p in &self.products {
            if !self.categories.iter().any(|c| c.key == p.category)
                || p.artwork.kind > 3
                || (p.artwork.kind == 0 && p.artwork.looks != [0; 2])
                || (p.artwork.kind != 0 && p.artwork.looks.contains(&0))
            {
                return Err(StoreWireError::Malformed);
            }
            string(&mut b, &p.category, 128)?;
            blob(
                &mut b,
                &encode_catalogue_page(&StoreCataloguePage {
                    catalog_revision: p.revision.clone(),
                    offers: vec![p.offer.clone()],
                    next_page_cursor: String::new(),
                    total_coins: 0,
                    transferable_coins: 0,
                })?,
            )?;
            b.push(p.artwork.kind);
            for look in p.artwork.looks {
                b.extend(look.to_le_bytes());
            }
        }
        string(&mut b, &self.next, 128)?;
        if b.len() > MAX_STORE_RESULT_BYTES {
            return Err(StoreWireError::LimitExceeded);
        }
        Ok(b)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, StoreWireError> {
        if bytes.len() > MAX_STORE_RESULT_BYTES {
            return Err(StoreWireError::LimitExceeded);
        }
        let mut r = Reader { bytes, at: 0 };
        if r.take::<4>()? != *b"SBR1" {
            return Err(StoreWireError::Malformed);
        }
        let total = u64::from_le_bytes(r.take()?);
        let transferable = u64::from_le_bytes(r.take()?);
        let known = r.take::<1>()?[0];
        if known > 1 {
            return Err(StoreWireError::Malformed);
        }
        let n = r.take::<1>()?[0];
        if usize::from(n) > MAX_CATEGORIES {
            return Err(StoreWireError::LimitExceeded);
        }
        let mut categories = Vec::new();
        for _ in 0..n {
            categories.push(Category {
                key: r.text()?,
                parent: r.text()?,
                title: r.text()?,
            });
        }
        let n = r.take::<1>()?[0];
        if usize::from(n) > MAX_STORE_PAGE_ENTRIES {
            return Err(StoreWireError::LimitExceeded);
        }
        let mut products = Vec::new();
        for _ in 0..n {
            let category = r.text()?;
            let mut quote = decode_catalogue_page(r.blob()?)?;
            if quote.offers.len() != 1
                || !quote.next_page_cursor.is_empty()
                || quote.total_coins != 0
                || quote.transferable_coins != 0
            {
                return Err(StoreWireError::Malformed);
            }
            let artwork = Artwork {
                kind: r.take::<1>()?[0],
                looks: [u32::from_le_bytes(r.take()?), u32::from_le_bytes(r.take()?)],
            };
            products.push(Product {
                category,
                revision: quote.catalog_revision,
                offer: quote.offers.remove(0),
                artwork,
            });
        }
        let page = Self {
            categories,
            products,
            next: r.text()?,
            total,
            transferable,
            transferable_known: known == 1,
        };
        r.end()?;
        page.encode()?;
        Ok(page)
    }
}
#[cfg(test)]
#[path = "browse/tests.rs"]
mod tests;
