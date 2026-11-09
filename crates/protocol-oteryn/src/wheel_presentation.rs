//! Optional current-owner Wheel graph/perk and draft preview, never a mutation.
use crate::wheel::WheelVessels;
/// Uses the existing Wheel/Atelier capability; no extra capability ID.
pub const CAPABILITY_WHEEL_PRESENTATION_V1: u32 = crate::wheel::CAPABILITY_WHEEL_ALLOCATION_V1;
pub const COMMAND_TYPE_WHEEL_PRESENTATION: u32 = 6119;
pub const MAX_REQUEST_BYTES: usize = 164;
pub const MAX_REPLY_BYTES: usize = 131072;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub nonce: u64,
    pub wheel_revision: u64,
    pub atelier_revision: u64,
    pub points: [u16; 36],
    pub vessels: WheelVessels,
}
impl Request {
    pub fn encode(&self) -> Option<Vec<u8>> {
        if self.nonce == 0 {
            return None;
        }
        let mut b = self.nonce.to_le_bytes().to_vec();
        b.extend(self.wheel_revision.to_le_bytes());
        b.extend(self.atelier_revision.to_le_bytes());
        for p in self.points {
            b.extend(p.to_le_bytes());
        }
        for id in self.vessels {
            b.push(u8::from(id.is_some()));
            if let Some(id) = id {
                if id == [0; 16] {
                    return None;
                }
                b.extend(id);
            }
        }
        Some(b)
    }
    pub fn decode(b: &[u8]) -> Option<Self> {
        if b.len() > MAX_REQUEST_BYTES {
            return None;
        }
        let mut r = Reader(b);
        let nonce = r.u64()?;
        let wheel_revision = r.u64()?;
        let atelier_revision = r.u64()?;
        let mut points = [0; 36];
        for p in &mut points {
            *p = r.u16()?;
        }
        let mut vessels = [None; 4];
        for id in &mut vessels {
            *id = match r.byte()? {
                0 => None,
                1 => Some(r.take()?),
                _ => return None,
            };
        }
        if !r.0.is_empty() {
            return None;
        }
        let s = Self {
            nonce,
            wheel_revision,
            atelier_revision,
            points,
            vessels,
        };
        s.encode()?;
        Some(s)
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub domain: u8,
    pub capacity: u16,
    pub minimum_points: u32,
    pub unlock_from: Vec<u8>,
    pub name: String,
    pub per_point: Vec<String>,
    pub full_slot: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stage {
    pub minimum_points: u32,
    pub effects: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Revelation {
    pub domain: u8,
    pub name: String,
    pub stages: Vec<Stage>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reply {
    pub request: Request,
    pub catalogue: String,
    pub vocation: String,
    pub available: u32,
    pub unlock_points: u32,
    pub removal_permitted: bool,
    pub eligible: bool,
    pub draft_valid: bool,
    pub reason: String,
    pub nodes: Vec<Node>,
    pub revelations: Vec<Revelation>,
    pub current: Vec<String>,
    pub draft: Vec<String>,
}
fn text(b: &mut Vec<u8>, s: &str) -> Option<()> {
    if s.len() > 256 || s.chars().any(char::is_control) {
        return None;
    }
    b.extend((s.len() as u16).to_le_bytes());
    b.extend(s.as_bytes());
    Some(())
}
fn lines(b: &mut Vec<u8>, v: &[String]) -> Option<()> {
    if v.len() > 128 {
        return None;
    }
    b.push(v.len() as u8);
    for s in v {
        text(b, s)?;
    }
    Some(())
}
#[path = "wheel_presentation/codec.rs"]
mod codec;

struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn bytes(&mut self, n: usize) -> Option<&[u8]> {
        let (a, b) = self.0.split_at_checked(n)?;
        self.0 = b;
        Some(a)
    }
    fn take<const N: usize>(&mut self) -> Option<[u8; N]> {
        self.bytes(N)?.try_into().ok()
    }
    fn byte(&mut self) -> Option<u8> {
        Some(self.take::<1>()?[0])
    }
    fn boolean(&mut self) -> Option<bool> {
        match self.byte()? {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        }
    }
    fn u16(&mut self) -> Option<u16> {
        Some(u16::from_le_bytes(self.take()?))
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take()?))
    }
    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.take()?))
    }
    fn text(&mut self) -> Option<String> {
        let n = usize::from(self.u16()?);
        if n > 256 {
            return None;
        }
        Some(std::str::from_utf8(self.bytes(n)?).ok()?.to_owned())
    }
    fn lines(&mut self) -> Option<Vec<String>> {
        let n = self.byte()?;
        if n > 128 {
            return None;
        }
        let mut out = Vec::with_capacity(usize::from(n));
        for _ in 0..n {
            out.push(self.text()?);
        }
        Some(out)
    }
}
#[cfg(test)]
#[path = "wheel_presentation/tests.rs"]
mod tests;
