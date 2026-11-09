//! T079: bounded overview of admitted geometry, never movement or discovery authority.
use super::{MapResult, MapWireError, decode_result, encode_result};
pub const WIDTH: usize = 64;
pub const HEIGHT: usize = 40;
pub const MAX_ATLAS_BYTES: usize = 32_768;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Query {
    pub floor: i8,
    pub x: u16,
    pub y: u16,
    pub zoom: u8,
    pub page: u16,
}
impl Default for Query {
    fn default() -> Self {
        Self {
            floor: 127,
            x: u16::MAX,
            y: u16::MAX,
            zoom: 0,
            page: 0,
        }
    }
}
impl Query {
    pub fn encode(self) -> Result<Vec<u8>, MapWireError> {
        if (self.floor != 127 && !(-15..=0).contains(&self.floor)) || self.zoom > 8 {
            return Err(MapWireError);
        }
        let mut b = vec![128, self.floor as u8];
        b.extend(self.x.to_le_bytes());
        b.extend(self.y.to_le_bytes());
        b.push(self.zoom);
        b.extend(self.page.to_le_bytes());
        Ok(b)
    }
    pub fn decode(b: &[u8]) -> Result<Self, MapWireError> {
        if b.len() != 9 || b[0] != 128 {
            return Err(MapWireError);
        }
        let q = Self {
            floor: b[1] as i8,
            x: u16::from_le_bytes([b[2], b[3]]),
            y: u16::from_le_bytes([b[4], b[5]]),
            zoom: b[6],
            page: u16::from_le_bytes([b[7], b[8]]),
        };
        q.encode()?;
        Ok(q)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Area {
    pub id: u32,
    pub parent: u32,
    pub name: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Marker {
    pub kind: u8,
    pub x: u16,
    pub y: u16,
    pub label: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Atlas {
    pub query: Query,
    pub digest: [u8; 32],
    pub floors: u16,
    pub progress: Option<MapResult>,
    /// Canonical first intersecting admitted subarea per overview cell; zero is unknown.
    pub cells: Vec<u32>,
    pub areas: Vec<Area>,
    pub markers: Vec<Marker>,
    pub marker_total: u32,
}
impl Atlas {
    pub fn encode(&self) -> Result<Vec<u8>, MapWireError> {
        let query = self.query.encode()?;
        if self.query.floor == 127
            || self.cells.len() != WIDTH * HEIGHT
            || self.areas.len() > 64
            || self.markers.len() > 64
            || self.progress.as_ref().is_some_and(|p| p.code != 0)
            || self.floors & (1u16 << (-self.query.floor as u8)) == 0
        {
            return Err(MapWireError);
        }
        let progress = self
            .progress
            .as_ref()
            .map(encode_result)
            .transpose()?
            .unwrap_or_default();
        let start = u32::from(self.query.page) * 64;
        if self.marker_total > 65536
            || (start > 0 && start >= self.marker_total)
            || self.markers.len() != self.marker_total.saturating_sub(start).min(64) as usize
        {
            return Err(MapWireError);
        }
        let mut b = query;
        b.extend(self.digest);
        b.extend(self.floors.to_le_bytes());
        b.push(progress.len() as u8);
        b.extend(progress);
        b.extend(self.marker_total.to_le_bytes());
        for cell in &self.cells {
            b.extend(cell.to_le_bytes());
        }
        b.push(self.areas.len() as u8);
        for (i, a) in self.areas.iter().enumerate() {
            if a.id == 0 || self.areas[..i].iter().any(|old| old.id == a.id) {
                return Err(MapWireError);
            }
            b.extend(a.id.to_le_bytes());
            b.extend(a.parent.to_le_bytes());
            text(&mut b, &a.name)?;
        }
        b.push(self.markers.len() as u8);
        for m in &self.markers {
            if !(1..=3).contains(&m.kind) {
                return Err(MapWireError);
            }
            b.push(m.kind);
            b.extend(m.x.to_le_bytes());
            b.extend(m.y.to_le_bytes());
            text(&mut b, &m.label)?;
        }
        if b.len() > MAX_ATLAS_BYTES {
            return Err(MapWireError);
        }
        Ok(b)
    }
    pub fn decode(b: &[u8]) -> Result<Self, MapWireError> {
        if b.len() > MAX_ATLAS_BYTES {
            return Err(MapWireError);
        }
        let mut r = Reader(b);
        let query = Query::decode(r.take(9)?)?;
        let digest = r.take(32)?.try_into().map_err(|_| MapWireError)?;
        let floors = r.u16()?;
        let n = usize::from(r.byte()?);
        let progress = if n == 0 {
            None
        } else {
            Some(decode_result(r.take(n)?)?)
        };
        let marker_total = r.u32()?;
        let mut cells = Vec::with_capacity(WIDTH * HEIGHT);
        for _ in 0..WIDTH * HEIGHT {
            cells.push(r.u32()?);
        }
        let n = r.byte()?;
        if n > 64 {
            return Err(MapWireError);
        }
        let mut areas = Vec::new();
        for _ in 0..n {
            areas.push(Area {
                id: r.u32()?,
                parent: r.u32()?,
                name: r.text()?,
            });
        }
        let n = r.byte()?;
        if n > 64 {
            return Err(MapWireError);
        }
        let mut markers = Vec::new();
        for _ in 0..n {
            markers.push(Marker {
                kind: r.byte()?,
                x: r.u16()?,
                y: r.u16()?,
                label: r.text()?,
            });
        }
        let a = Self {
            query,
            digest,
            floors,
            progress,
            cells,
            areas,
            markers,
            marker_total,
        };
        if !r.0.is_empty() || a.encode()?.as_slice() != b {
            return Err(MapWireError);
        }
        Ok(a)
    }
}
fn text(b: &mut Vec<u8>, s: &str) -> Result<(), MapWireError> {
    if s.is_empty() || s.len() > 128 || s.chars().any(char::is_control) {
        return Err(MapWireError);
    }
    b.push(s.len() as u8);
    b.extend(s.as_bytes());
    Ok(())
}
struct Reader<'a>(&'a [u8]);
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], MapWireError> {
        let b = self.0.get(..n).ok_or(MapWireError)?;
        self.0 = &self.0[n..];
        Ok(b)
    }
    fn byte(&mut self) -> Result<u8, MapWireError> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, MapWireError> {
        Ok(u16::from_le_bytes(
            self.take(2)?.try_into().map_err(|_| MapWireError)?,
        ))
    }
    fn u32(&mut self) -> Result<u32, MapWireError> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().map_err(|_| MapWireError)?,
        ))
    }
    fn text(&mut self) -> Result<String, MapWireError> {
        let n = usize::from(self.byte()?);
        Ok(std::str::from_utf8(self.take(n)?)
            .map_err(|_| MapWireError)?
            .into())
    }
}
