//! Bounded Bosstiary catalogue extension of existing capability 117, command 133.
pub const PER_PAGE: usize = 16;
pub const SCAN_LIMIT: usize = 64;
pub const MAX_BYTES: usize = 16_384;
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Query {
    pub after: u32,
    pub category: u8,
    pub search: String,
}
impl Query {
    pub fn encode(&self) -> Option<Vec<u8>> {
        if self.category > 3 || self.search.len() > 48 || self.search.contains('\0') {
            return None;
        }
        let mut out = b"BCT1".to_vec();
        out.extend(self.after.to_le_bytes());
        out.push(self.category);
        out.push(self.search.len() as u8);
        out.extend(self.search.as_bytes());
        Some(out)
    }
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let mut r = Reader(bytes);
        if r.take(4)? != b"BCT1" {
            return None;
        }
        let after = r.u32()?;
        let category = r.take(1)?[0];
        let n = usize::from(r.take(1)?[0]);
        if n > 48 {
            return None;
        }
        let search = std::str::from_utf8(r.take(n)?).ok()?.into();
        if !r.0.is_empty() {
            return None;
        }
        let q = Self {
            after,
            category,
            search,
        };
        q.encode()?;
        Some(q)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub key: String,
    pub name: String,
    /// 1 bane, 2 archfoe, 3 nemesis.
    pub category: u8,
    pub kills: u64,
    pub thresholds: [u64; 3],
    /// Cumulative points at prowess, expertise and mastery.
    pub points: [u16; 3],
    pub look: u32,
    pub object: bool,
}
impl Entry {
    pub fn stage(&self) -> usize {
        self.thresholds
            .iter()
            .filter(|&&goal| self.kills >= goal)
            .count()
    }
    pub fn earned_points(&self) -> u16 {
        self.stage().checked_sub(1).map_or(0, |i| self.points[i])
    }
    pub fn category_name(&self) -> &'static str {
        match self.category {
            1 => "Bane",
            2 => "Archfoe",
            3 => "Nemesis",
            _ => "Unknown",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    pub next: Option<u32>,
    pub points: u64,
    pub entries: Vec<Entry>,
}
impl Page {
    pub fn encode(&self) -> Option<Vec<u8>> {
        if self.next == Some(0)
            || self.entries.len() > PER_PAGE
            || self.entries.iter().any(|e| {
                !super::valid_key(&e.key)
                    || e.key.len() > 512
                    || e.name.is_empty()
                    || e.name.len() > 128
                    || e.name.contains('\0')
                    || !(1..=3).contains(&e.category)
                    || e.thresholds[0] == 0
                    || !e.thresholds.windows(2).all(|p| p[0] < p[1])
                    || !e.points.windows(2).all(|p| p[0] < p[1])
            })
            || self
                .entries
                .iter()
                .enumerate()
                .any(|(i, e)| self.entries[..i].iter().any(|p| p.key == e.key))
        {
            return None;
        }
        let mut out = b"BCP1".to_vec();
        out.extend(self.next.unwrap_or(0).to_le_bytes());
        out.extend(self.points.to_le_bytes());
        out.push(self.entries.len() as u8);
        for e in &self.entries {
            text(&mut out, &e.key);
            text(&mut out, &e.name);
            out.push(e.category);
            out.extend(e.kills.to_le_bytes());
            for n in e.thresholds {
                out.extend(n.to_le_bytes());
            }
            for n in e.points {
                out.extend(n.to_le_bytes());
            }
            out.extend(e.look.to_le_bytes());
            out.push(u8::from(e.object));
        }
        (out.len() <= MAX_BYTES).then_some(out)
    }
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > MAX_BYTES {
            return None;
        }
        let mut r = Reader(bytes);
        if r.take(4)? != b"BCP1" {
            return None;
        }
        let next = r.u32()?;
        let points = r.u64()?;
        let count = usize::from(r.take(1)?[0]);
        if count > PER_PAGE {
            return None;
        }
        let mut entries = Vec::new();
        for _ in 0..count {
            let key = r.text(512)?;
            let name = r.text(128)?;
            let category = r.take(1)?[0];
            let kills = r.u64()?;
            let thresholds = [r.u64()?, r.u64()?, r.u64()?];
            let points = [r.u16()?, r.u16()?, r.u16()?];
            let look = r.u32()?;
            let object = match r.take(1)?[0] {
                0 => false,
                1 => true,
                _ => return None,
            };
            entries.push(Entry {
                key,
                name,
                category,
                kills,
                thresholds,
                points,
                look,
                object,
            });
        }
        if !r.0.is_empty() {
            return None;
        }
        let page = Self {
            next: (next != 0).then_some(next),
            points,
            entries,
        };
        page.encode()?;
        Some(page)
    }
}
fn text(out: &mut Vec<u8>, s: &str) {
    out.extend((s.len() as u16).to_le_bytes());
    out.extend(s.as_bytes());
}
struct Reader<'a>(&'a [u8]);
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let (a, b) = self.0.split_at_checked(n)?;
        self.0 = b;
        Some(a)
    }
    fn u16(&mut self) -> Option<u16> {
        Some(u16::from_le_bytes(self.take(2)?.try_into().ok()?))
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }
    fn text(&mut self, max: usize) -> Option<String> {
        let n = usize::from(self.u16()?);
        if n > max {
            return None;
        }
        Some(std::str::from_utf8(self.take(n)?).ok()?.into())
    }
}
#[cfg(test)]
#[path = "catalogue_tests.rs"]
mod tests;
