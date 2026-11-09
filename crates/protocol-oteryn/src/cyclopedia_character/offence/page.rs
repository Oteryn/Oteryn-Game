use super::*;
use crate::cyclopedia_character::{CharacterTab, TabView, combat::CombatStats};
pub const MAX_PAGE_BYTES: usize = 32_000;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Query {
    pub page: u8,
    pub sources: u8,
    pub by_source: bool,
    pub target: Option<[u8; 16]>,
}
impl Default for Query {
    fn default() -> Self {
        Self {
            page: 0,
            sources: 7,
            by_source: false,
            target: None,
        }
    }
}
impl Query {
    pub fn encode(self) -> Result<Vec<u8>, Error> {
        if !(1..=7).contains(&self.sources) {
            return Err(Error::Malformed);
        }
        let mut out = vec![
            SELECTOR,
            self.page,
            self.sources | u8::from(self.by_source) << 3,
        ];
        if let Some(target) = self.target {
            out.extend_from_slice(&target);
        }
        Ok(out)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        if !matches!(bytes.len(), 3 | 19) || bytes[0] != SELECTOR || bytes[2] > 15 {
            return Err(Error::Malformed);
        }
        let query = Self {
            page: bytes[1],
            sources: bytes[2] & 7,
            by_source: bytes[2] & 8 != 0,
            target: if bytes.len() == 19 {
                Some(bytes[3..].try_into().map_err(|_| Error::Malformed)?)
            } else {
                None
            },
        };
        if query.encode()?.as_slice() != bytes {
            return Err(Error::Malformed);
        }
        Ok(query)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    pub character: [u8; 16],
    pub query: Query,
    pub total: u16,
    pub stats: CombatStats,
    pub facts: Option<Snapshot>,
}
impl Page {
    pub fn from_snapshot(
        character: [u8; 16],
        mut query: Query,
        stats: CombatStats,
        facts: Option<Snapshot>,
    ) -> Result<Self, Error> {
        query.target = None;
        let (total, facts) = if let Some(mut facts) = facts {
            facts.encode()?;
            facts.rows.retain(|row| {
                query.sources & facts.sources[usize::from(row.source)].kind.bit() != 0
            });
            if query.by_source {
                facts.rows.sort_by(|a, b| {
                    (a.source, &a.spell, a.effect).cmp(&(b.source, &b.spell, b.effect))
                });
            } else {
                facts.rows.sort_by(|a, b| {
                    (&a.spell, a.source, a.effect).cmp(&(&b.spell, b.source, b.effect))
                });
            }
            let total = u16::try_from(facts.rows.len()).map_err(|_| Error::LimitExceeded)?;
            let start = usize::from(query.page) * PAGE_SIZE;
            if start >= facts.rows.len() && query.page > 0 {
                return Err(Error::Malformed);
            }
            facts.rows = facts.rows.into_iter().skip(start).take(PAGE_SIZE).collect();
            (total, Some(facts))
        } else {
            (0, None)
        };
        let value = Self {
            character,
            query,
            total,
            stats,
            facts,
        };
        value.encode()?;
        Ok(value)
    }
    pub fn has_more(&self) -> bool {
        usize::from(self.query.page) * PAGE_SIZE + self.facts.as_ref().map_or(0, |f| f.rows.len())
            < usize::from(self.total)
    }
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        let filter = self.query.encode()?[2];
        if self.query.target.is_some()
            || self.stats.tab() != CharacterTab::Offence
            || usize::from(self.total) > MAX_ROWS
        {
            return Err(Error::Malformed);
        }
        let start = usize::from(self.query.page) * PAGE_SIZE;
        let expected = usize::from(self.total).saturating_sub(start).min(PAGE_SIZE);
        if (self.query.page > 0 && start >= usize::from(self.total))
            || self.facts.as_ref().is_some_and(|f| {
                f.rows.len() != expected
                    || f.rows.iter().any(|r| {
                        f.sources
                            .get(usize::from(r.source))
                            .is_none_or(|s| self.query.sources & s.kind.bit() == 0)
                    })
            })
            || (self.facts.is_none() && (self.total != 0 || self.query.page != 0))
        {
            return Err(Error::Malformed);
        }
        let stats = TabView::Combat(self.stats.clone())
            .encode()
            .map_err(|_| Error::Malformed)?;
        let mut out = vec![SELECTOR, self.query.page, filter];
        out.extend_from_slice(&self.character);
        out.extend_from_slice(&self.total.to_le_bytes());
        out.extend_from_slice(&(stats.len() as u16).to_le_bytes());
        out.extend_from_slice(&stats);
        out.push(u8::from(self.facts.is_some()));
        if let Some(facts) = &self.facts {
            out.extend_from_slice(&facts.encode()?);
        }
        if out.len() > MAX_PAGE_BYTES {
            return Err(Error::LimitExceeded);
        }
        Ok(out)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() > MAX_PAGE_BYTES {
            return Err(Error::LimitExceeded);
        }
        let mut r = Reader(bytes);
        if r.byte()? != SELECTOR {
            return Err(Error::Malformed);
        }
        let query = Query::decode(&[SELECTOR, r.byte()?, r.byte()?])?;
        let character = r.take(16)?.try_into().map_err(|_| Error::Malformed)?;
        let total = r.u16()?;
        let n = usize::from(r.u16()?);
        let stats = CombatStats::decode(r.take(n)?).map_err(|_| Error::Malformed)?;
        let facts = match r.byte()? {
            0 => None,
            1 => {
                let facts = Snapshot::decode(r.0)?;
                r.0 = &[];
                Some(facts)
            }
            _ => return Err(Error::Malformed),
        };
        let value = Self {
            character,
            query,
            total,
            stats,
            facts,
        };
        if !r.0.is_empty() || value.encode()?.as_slice() != bytes {
            return Err(Error::Malformed);
        }
        Ok(value)
    }
}
