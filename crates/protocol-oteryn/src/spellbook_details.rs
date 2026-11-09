//! Independently negotiated spellbook pages; the original catalog wire remains unchanged.
use crate::actor_spell_catalog::{self as catalog, SpellCatalogEntry};
pub const CAPABILITY_SPELLBOOK_DETAILS_V1: u32 = 449;
pub const COMMAND_TYPE_SPELLBOOK_QUERY: u32 = 733;
pub const PAGE_SIZE: usize = 32;
pub const MAX_REPLY_BYTES: usize = 65_536;
/// Vocation bits, in order: none, druid, elder druid, sorcerer, master sorcerer,
/// knight, elite knight, paladin, royal paladin, monk, exalted monk.
pub const VOCATION_MASK: u16 = (1 << 11) - 1;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetailedSpell {
    pub spell: SpellCatalogEntry,
    pub vocations: u16,
    pub premium: bool,
    pub magic_level: u32,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Query {
    pub page: u16,
    /// 0 = all; 1..11 select the corresponding vocation bit.
    pub vocation: u8,
    /// 0 = all, 1 = free, 2 = premium.
    pub premium: u8,
    pub min_level: u32,
    pub max_level: u32,
    /// 0 = all; otherwise a server cooldown-group ordinal 1..16.
    pub group: u8,
    pub search: String,
}
impl Query {
    pub fn valid(&self) -> bool {
        self.page <= 31
            && self.vocation <= 11
            && self.premium <= 2
            && self.group <= 16
            && (self.max_level == 0 || self.min_level <= self.max_level)
            && self.search.len() <= 64
            && !self.search.chars().any(char::is_control)
    }
    pub fn encode(&self) -> Option<Vec<u8>> {
        if !self.valid() {
            return None;
        }
        let mut out = vec![1];
        out.extend(self.page.to_le_bytes());
        out.push(self.vocation);
        out.push(self.premium);
        out.push(self.group);
        out.extend(self.min_level.to_le_bytes());
        out.extend(self.max_level.to_le_bytes());
        out.extend(self.search.as_bytes());
        Some(out)
    }
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if !(14..=78).contains(&bytes.len()) || bytes[0] != 1 {
            return None;
        }
        let value = Self {
            page: u16::from_le_bytes(bytes[1..3].try_into().ok()?),
            vocation: bytes[3],
            premium: bytes[4],
            group: bytes[5],
            min_level: u32::from_le_bytes(bytes[6..10].try_into().ok()?),
            max_level: u32::from_le_bytes(bytes[10..14].try_into().ok()?),
            search: std::str::from_utf8(&bytes[14..]).ok()?.into(),
        };
        value.valid().then_some(value)
    }
    /// A read-only display filter, shared so paging and visible totals agree.
    pub fn matches(&self, entry: &DetailedSpell) -> bool {
        self.valid()
            && (self.vocation == 0 || entry.vocations & (1 << (self.vocation - 1)) != 0)
            && (self.premium == 0 || entry.premium == (self.premium == 2))
            && entry.spell.level >= self.min_level
            && (self.max_level == 0 || entry.spell.level <= self.max_level)
            && (self.group == 0 || entry.spell.cooldown_groups.contains(&u32::from(self.group)))
            && (entry
                .spell
                .name
                .to_lowercase()
                .contains(&self.search.to_lowercase())
                || entry
                    .spell
                    .words
                    .to_lowercase()
                    .contains(&self.search.to_lowercase()))
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    pub page: u16,
    pub total: u16,
    pub entries: Vec<DetailedSpell>,
}
impl Page {
    fn valid(&self) -> bool {
        let total = usize::from(self.total);
        let start = usize::from(self.page) * PAGE_SIZE;
        total <= catalog::MAX_ENTRIES
            && (start < total || (total == 0 && self.page == 0))
            && self.entries.len() == total.saturating_sub(start).min(PAGE_SIZE)
            && self.entries.iter().all(|entry| {
                entry.vocations != 0
                    && entry.vocations & !VOCATION_MASK == 0
                    && (entry.spell.rune_item.is_some() || entry.magic_level == 0)
            })
    }
    pub fn encode(&self) -> Option<Vec<u8>> {
        if !self.valid() {
            return None;
        }
        let entries: Vec<_> = self
            .entries
            .iter()
            .map(|entry| entry.spell.clone())
            .collect();
        let catalog = catalog::encode_catalog(&entries).ok()?;
        let mut out = vec![1];
        out.extend(self.page.to_le_bytes());
        out.extend(self.total.to_le_bytes());
        out.extend(u32::try_from(catalog.len()).ok()?.to_le_bytes());
        out.extend(catalog);
        for entry in &self.entries {
            out.extend(entry.vocations.to_le_bytes());
            out.push(u8::from(entry.premium));
            out.extend(entry.magic_level.to_le_bytes());
        }
        (out.len() <= MAX_REPLY_BYTES).then_some(out)
    }
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if !(9..=MAX_REPLY_BYTES).contains(&bytes.len()) || bytes[0] != 1 {
            return None;
        }
        let length = usize::try_from(u32::from_le_bytes(bytes[5..9].try_into().ok()?)).ok()?;
        let end = 9usize.checked_add(length)?;
        let catalog = catalog::decode_catalog(bytes.get(9..end)?).ok()?;
        if bytes.len() != end.checked_add(catalog.len().checked_mul(7)?)? {
            return None;
        }
        let mut entries = Vec::with_capacity(catalog.len());
        for (spell, metadata) in catalog.into_iter().zip(bytes[end..].as_chunks::<7>().0) {
            let premium = match metadata[2] {
                0 => false,
                1 => true,
                _ => return None,
            };
            entries.push(DetailedSpell {
                spell,
                vocations: u16::from_le_bytes(metadata[..2].try_into().ok()?),
                premium,
                magic_level: u32::from_le_bytes(metadata[3..].try_into().ok()?),
            });
        }
        let page = Self {
            page: u16::from_le_bytes(bytes[1..3].try_into().ok()?),
            total: u16::from_le_bytes(bytes[3..5].try_into().ok()?),
            entries,
        };
        page.valid().then_some(page)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn spellbook_details_query_and_page_are_bounded_and_filter_canonical_metadata()
    -> Result<(), &'static str> {
        let spell = DetailedSpell {
            spell: SpellCatalogEntry {
                cast_index: 1,
                name: "Mending".into(),
                words: "mend".into(),
                level: 12,
                mana: 20,
                mana_percent: false,
                soul: 0,
                learned: true,
                available: true,
                cooldown_groups: vec![2],
                rune_item: None,
            },
            vocations: 2,
            premium: true,
            magic_level: 0,
        };
        let page = Page {
            page: 0,
            total: 1,
            entries: vec![spell.clone()],
        };
        let encoded = page.encode().ok_or("encode")?;
        assert_eq!(Page::decode(&encoded), Some(page));
        assert!(Page::decode(&encoded[..encoded.len() - 1]).is_none());
        let mut invalid = encoded;
        let last = invalid.len() - 7;
        invalid[last] = 0;
        invalid[last + 1] = 0;
        assert!(Page::decode(&invalid).is_none());
        let query = Query {
            vocation: 2,
            premium: 2,
            min_level: 10,
            max_level: 20,
            group: 2,
            search: "MEND".into(),
            ..Default::default()
        };
        assert_eq!(
            Query::decode(&query.encode().ok_or("query")?),
            Some(query.clone())
        );
        assert!(query.matches(&spell));
        assert!(
            !Query {
                vocation: 6,
                ..query.clone()
            }
            .matches(&spell)
        );
        assert!(
            !Query {
                premium: 1,
                ..query.clone()
            }
            .matches(&spell)
        );
        assert!(
            Query {
                search: "x".repeat(65),
                ..query.clone()
            }
            .encode()
            .is_none()
        );
        assert!(
            Query {
                min_level: 30,
                max_level: 20,
                ..query
            }
            .encode()
            .is_none()
        );
        assert!(
            Page {
                page: 1,
                total: 1,
                entries: vec![]
            }
            .encode()
            .is_none()
        );
        Ok(())
    }
}
