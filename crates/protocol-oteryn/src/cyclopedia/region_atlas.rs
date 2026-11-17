//! Bounded, read-only atlas geometry from the admitted world's native subarea masks.
use super::MapWireError;
pub const RUNS_PER_PAGE: usize = 64;
pub const MAX_AREAS: usize = RUNS_PER_PAGE * 2;
pub const MAX_BYTES: usize = 40_000;
const _: () = assert!(MAX_BYTES <= crate::MAX_COMMAND_RESULT_PAYLOAD_BYTES);
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Area {
    pub id: u32,
    /// Zero for a region; otherwise the native parent region id.
    pub parent: u32,
    pub name: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Run {
    pub area: u32,
    pub x: u16,
    /// Half-open horizontal endpoint, preserving exact admitted coverage.
    pub end: u16,
    pub y: u16,
    pub floor: i8,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    pub next: Option<u32>,
    pub areas: Vec<Area>,
    pub runs: Vec<Run>,
}
pub fn encode(page: &Page) -> Result<Vec<u8>, MapWireError> {
    if page.runs.len() > RUNS_PER_PAGE
        || page.areas.len() > MAX_AREAS
        || page.next == Some(0)
        || !page.areas.windows(2).all(|p| p[0].id < p[1].id)
        || page.areas.iter().any(|a| {
            a.id == 0
                || a.name.is_empty()
                || a.name.len() > 256
                || a.name.contains('\0')
                || a.parent != 0 && !page.areas.iter().any(|p| p.id == a.parent && p.parent == 0)
        })
        || page.runs.iter().any(|r| {
            r.x >= r.end
                || !(-15..=0).contains(&r.floor)
                || !page.areas.iter().any(|a| a.id == r.area && a.parent != 0)
        })
    {
        return Err(MapWireError);
    }
    let mut out = b"ATL1".to_vec();
    out.extend_from_slice(&page.next.unwrap_or(0).to_le_bytes());
    out.extend_from_slice(&(page.areas.len() as u16).to_le_bytes());
    out.extend_from_slice(&(page.runs.len() as u16).to_le_bytes());
    for area in &page.areas {
        out.extend_from_slice(&area.id.to_le_bytes());
        out.extend_from_slice(&area.parent.to_le_bytes());
        out.extend_from_slice(&(area.name.len() as u16).to_le_bytes());
        out.extend_from_slice(area.name.as_bytes());
    }
    for run in &page.runs {
        out.extend_from_slice(&run.area.to_le_bytes());
        for n in [run.x, run.end, run.y] {
            out.extend_from_slice(&n.to_le_bytes());
        }
        out.push(run.floor as u8);
    }
    if out.len() > MAX_BYTES {
        return Err(MapWireError);
    }
    Ok(out)
}
struct Reader<'a>(&'a [u8]);
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], MapWireError> {
        let (a, b) = self.0.split_at_checked(n).ok_or(MapWireError)?;
        self.0 = b;
        Ok(a)
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
}
pub fn decode(bytes: &[u8]) -> Result<Page, MapWireError> {
    if bytes.len() > MAX_BYTES {
        return Err(MapWireError);
    }
    let mut r = Reader(bytes);
    if r.take(4)? != b"ATL1" {
        return Err(MapWireError);
    }
    let next = r.u32()?;
    let area_count = usize::from(r.u16()?);
    let run_count = usize::from(r.u16()?);
    if area_count > MAX_AREAS || run_count > RUNS_PER_PAGE {
        return Err(MapWireError);
    }
    let mut areas = Vec::new();
    let mut runs = Vec::new();
    for _ in 0..area_count {
        let id = r.u32()?;
        let parent = r.u32()?;
        let len = usize::from(r.u16()?);
        if len > 256 {
            return Err(MapWireError);
        }
        let name = std::str::from_utf8(r.take(len)?)
            .map_err(|_| MapWireError)?
            .to_owned();
        areas.push(Area { id, parent, name });
    }
    for _ in 0..run_count {
        runs.push(Run {
            area: r.u32()?,
            x: r.u16()?,
            end: r.u16()?,
            y: r.u16()?,
            floor: r.take(1)?[0] as i8,
        });
    }
    if !r.0.is_empty() {
        return Err(MapWireError);
    }
    let page = Page {
        next: (next != 0).then_some(next),
        areas,
        runs,
    };
    encode(&page)?;
    Ok(page)
}
#[cfg(test)]
#[path = "atlas_tests.rs"]
mod tests;
