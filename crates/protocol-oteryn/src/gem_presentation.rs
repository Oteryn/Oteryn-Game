//! Optional executed Gem modifier presentation. No price, draw or mutation input is added.
/// Reuses the existing Wheel/Atelier capability; no additional capability ID.
pub const CAPABILITY_GEM_PRESENTATION_V1: u32 = crate::wheel::CAPABILITY_WHEEL_ALLOCATION_V1;
pub const COMMAND_TYPE_GEM_PRESENTATION: u32 = 6121;
pub const MAX_REQUEST_BYTES: usize = 16;
pub const MAX_REPLY_BYTES: usize = 65536;
pub const MAX_MODIFIERS: usize = 140;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Request {
    pub nonce: u64,
    pub revision: u64,
}
impl Request {
    pub fn encode(self) -> Option<Vec<u8>> {
        if self.nonce == 0 {
            return None;
        }
        let mut b = self.nonce.to_le_bytes().to_vec();
        b.extend(self.revision.to_le_bytes());
        Some(b)
    }
    pub fn decode(b: &[u8]) -> Option<Self> {
        if b.len() != 16 {
            return None;
        }
        let s = Self {
            nonce: u64::from_le_bytes(b[..8].try_into().ok()?),
            revision: u64::from_le_bytes(b[8..].try_into().ok()?),
        };
        s.encode()?;
        Some(s)
    }
}
/// Every signed value is in thousandths of the explicit unit:0 percent-points,1 flat,2 seconds,3 points.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Effect {
    pub name: String,
    pub target: String,
    pub unit: u8,
    pub values: [i64; 4],
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Modifier {
    pub kind: u8,
    pub id: u16,
    pub name: String,
    pub effects: Vec<Effect>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reply {
    pub request: Request,
    pub catalogue: String,
    pub vocation: String,
    pub modifiers: Vec<Modifier>,
    pub grades_current: bool,
    pub qualified_gems: Vec<[u8; 16]>,
}
fn text_valid(s: &str, empty: bool) -> bool {
    (empty || !s.is_empty()) && s.len() <= 128 && !s.chars().any(char::is_control)
}
impl Reply {
    pub fn encode(&self) -> Option<Vec<u8>> {
        if !text_valid(&self.catalogue, false)
            || !text_valid(&self.vocation, false)
            || self.qualified_gems.len() > 225
            || self.qualified_gems.contains(&[0; 16])
            || self.qualified_gems.windows(2).any(|p| p[0] >= p[1])
            || self.modifiers.is_empty()
            || self.modifiers.len() > MAX_MODIFIERS
        {
            return None;
        }
        let mut b = self.request.encode()?;
        write_text(&mut b, &self.catalogue);
        write_text(&mut b, &self.vocation);
        b.push(u8::from(self.grades_current));
        b.extend((self.qualified_gems.len() as u16).to_le_bytes());
        for id in &self.qualified_gems {
            b.extend(id);
        }
        b.extend((self.modifiers.len() as u16).to_le_bytes());
        let mut previous = None;
        for m in &self.modifiers {
            if m.kind > 1
                || previous.is_some_and(|p| p >= (m.kind, m.id))
                || !text_valid(&m.name, false)
                || m.effects.is_empty()
                || m.effects.len() > 2
            {
                return None;
            }
            previous = Some((m.kind, m.id));
            b.push(m.kind);
            b.extend(m.id.to_le_bytes());
            write_text(&mut b, &m.name);
            b.push(m.effects.len() as u8);
            for e in &m.effects {
                if e.unit > 3 || !text_valid(&e.name, false) || !text_valid(&e.target, true) {
                    return None;
                }
                write_text(&mut b, &e.name);
                write_text(&mut b, &e.target);
                b.push(e.unit);
                for value in e.values {
                    b.extend(value.to_le_bytes());
                }
            }
        }
        (b.len() <= MAX_REPLY_BYTES).then_some(b)
    }
    pub fn decode(b: &[u8]) -> Option<Self> {
        if b.len() > MAX_REPLY_BYTES {
            return None;
        }
        let request = Request::decode(b.get(..16)?)?;
        let mut r = Reader(b.get(16..)?);
        let catalogue = r.text()?;
        let vocation = r.text()?;
        let grades_current = match r.byte()? {
            0 => false,
            1 => true,
            _ => return None,
        };
        let count = usize::from(u16::from_le_bytes(r.take()?));
        if count > 225 {
            return None;
        }
        let mut qualified_gems = Vec::with_capacity(count);
        for _ in 0..count {
            qualified_gems.push(r.take::<16>()?);
        }
        let count = usize::from(u16::from_le_bytes(r.take()?));
        if count > MAX_MODIFIERS {
            return None;
        }
        let mut modifiers = Vec::with_capacity(count);
        for _ in 0..count {
            let kind = r.byte()?;
            let id = u16::from_le_bytes(r.take()?);
            let name = r.text()?;
            let count = r.byte()?;
            if count > 2 {
                return None;
            }
            let mut effects = Vec::with_capacity(usize::from(count));
            for _ in 0..count {
                let name = r.text()?;
                let target = r.text()?;
                let unit = r.byte()?;
                let mut values = [0; 4];
                for v in &mut values {
                    *v = i64::from_le_bytes(r.take()?);
                }
                effects.push(Effect {
                    name,
                    target,
                    unit,
                    values,
                });
            }
            modifiers.push(Modifier {
                kind,
                id,
                name,
                effects,
            });
        }
        if !r.0.is_empty() {
            return None;
        }
        let s = Self {
            request,
            catalogue,
            vocation,
            modifiers,
            grades_current,
            qualified_gems,
        };
        s.encode()?;
        Some(s)
    }
}
fn write_text(b: &mut Vec<u8>, s: &str) {
    b.extend((s.len() as u16).to_le_bytes());
    b.extend(s.as_bytes());
}
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn take<const N: usize>(&mut self) -> Option<[u8; N]> {
        let out = self.0.get(..N)?.try_into().ok()?;
        self.0 = &self.0[N..];
        Some(out)
    }
    fn byte(&mut self) -> Option<u8> {
        Some(self.take::<1>()?[0])
    }
    fn text(&mut self) -> Option<String> {
        let n = usize::from(u16::from_le_bytes(self.take()?));
        if n > 128 {
            return None;
        }
        let s = std::str::from_utf8(self.0.get(..n)?).ok()?.to_owned();
        self.0 = &self.0[n..];
        Some(s)
    }
}
#[cfg(test)]
#[path = "gem_presentation/tests.rs"]
mod tests;
