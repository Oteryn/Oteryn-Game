//! Opt-in public atlas pages: command 72, request ff 01 + area:u32 + floor:i8 + after:u16.
use crate::charm_wire::CyclopediaWireError as Error;
type Result<T> = std::result::Result<T, Error>;
pub const AREA_PAGE: usize = 16;
pub const RUN_PAGE: usize = 1024;
pub const MAX_AREAS: usize = 1024;
pub const MAX_RUNS: usize = 262_144;
pub const MAX_NAME: usize = 256;
pub const MAX_REPLY: usize = 24 + RUN_PAGE * 10;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Request {
    pub area: u32,
    pub floor: i8,
    pub after: u16,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Area {
    pub id: u32,
    pub parent: u32,
    pub name: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AreaPage {
    pub parent: u32,
    pub after: u16,
    pub next: Option<u16>,
    pub areas: Vec<Area>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Run {
    pub y: u16,
    pub begin: u16,
    pub end: u16,
    pub subarea: u32,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeometryPage {
    pub area: u32,
    pub floor: i8,
    pub page: u16,
    pub next: Option<u16>,
    pub total: u32,
    pub bounds: [u16; 4],
    pub runs: Vec<Run>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    Areas(AreaPage),
    Geometry(GeometryPage),
}
pub fn encode_request(request: Request) -> Result<Vec<u8>> {
    if request.floor == 1 {
        if usize::from(request.after) > MAX_AREAS {
            return Err(Error::LimitExceeded);
        }
    } else if request.area == 0
        || !(-15..=0).contains(&request.floor)
        || usize::from(request.after) >= MAX_RUNS / RUN_PAGE
    {
        return Err(Error::Malformed);
    }
    let mut out = vec![255, 1];
    out.extend(request.area.to_le_bytes());
    out.push(request.floor as u8);
    out.extend(request.after.to_le_bytes());
    Ok(out)
}
pub fn decode_request(body: &[u8]) -> Result<Request> {
    if body.len() != 9 || body[..2] != [255, 1] {
        return Err(Error::Malformed);
    }
    let request = Request {
        area: u32::from_le_bytes(body[2..6].try_into().map_err(|_| Error::Malformed)?),
        floor: body[6] as i8,
        after: u16::from_le_bytes([body[7], body[8]]),
    };
    encode_request(request)?;
    Ok(request)
}
fn validate(reply: &Reply) -> Result<()> {
    match reply {
        Reply::Areas(page) => {
            if page.areas.len() > AREA_PAGE
                || usize::from(page.after) + page.areas.len() > MAX_AREAS
                || page.next.is_some_and(|n| {
                    page.areas.is_empty()
                        || usize::from(n) != usize::from(page.after) + page.areas.len()
                })
            {
                return Err(Error::LimitExceeded);
            }
            if page.areas.windows(2).any(|w| w[0].id >= w[1].id) {
                return Err(Error::Malformed);
            }
            for area in &page.areas {
                if area.id == 0
                    || area.id == area.parent
                    || area.parent != page.parent
                    || area.name.is_empty()
                    || area.name.len() > MAX_NAME
                    || area.name.contains('\0')
                {
                    return Err(Error::Malformed);
                }
            }
        }
        Reply::Geometry(page) => {
            let offset = usize::from(page.page) * RUN_PAGE;
            let total = usize::try_from(page.total).map_err(|_| Error::LimitExceeded)?;
            if page.area == 0
                || !(-15..=0).contains(&page.floor)
                || total > MAX_RUNS
                || (offset >= total && total > 0)
                || (total == 0 && page.page != 0)
                || page.runs.len() != (total - offset).min(RUN_PAGE)
            {
                return Err(Error::LimitExceeded);
            }
            let next = if offset + page.runs.len() < total {
                Some(page.page.checked_add(1).ok_or(Error::LimitExceeded)?)
            } else {
                None
            };
            if page.next != next
                || (total == 0 && page.bounds != [0; 4])
                || page.bounds[0] > page.bounds[2]
                || page.bounds[1] > page.bounds[3]
            {
                return Err(Error::Malformed);
            }
            for run in &page.runs {
                if run.subarea == 0
                    || run.begin >= run.end
                    || run.begin < page.bounds[0]
                    || run.end - 1 > page.bounds[2]
                    || run.y < page.bounds[1]
                    || run.y > page.bounds[3]
                {
                    return Err(Error::Malformed);
                }
            }
            if page
                .runs
                .windows(2)
                .any(|w| w[0].y > w[1].y || (w[0].y == w[1].y && w[0].end > w[1].begin))
            {
                return Err(Error::Malformed);
            }
        }
    }
    Ok(())
}
pub fn encode_reply(reply: &Reply) -> Result<Vec<u8>> {
    validate(reply)?;
    let mut out = Vec::new();
    match reply {
        Reply::Areas(page) => {
            out.push(1);
            out.extend(page.parent.to_le_bytes());
            out.extend(page.after.to_le_bytes());
            out.extend(page.next.unwrap_or(0).to_le_bytes());
            out.push(page.areas.len() as u8);
            for area in &page.areas {
                out.extend(area.id.to_le_bytes());
                out.extend(area.parent.to_le_bytes());
                out.extend((area.name.len() as u16).to_le_bytes());
                out.extend(area.name.as_bytes());
            }
        }
        Reply::Geometry(page) => {
            out.push(2);
            out.extend(page.area.to_le_bytes());
            out.push(page.floor as u8);
            out.extend(page.page.to_le_bytes());
            out.extend(page.next.unwrap_or(0).to_le_bytes());
            out.extend(page.total.to_le_bytes());
            for value in page.bounds {
                out.extend(value.to_le_bytes());
            }
            out.extend((page.runs.len() as u16).to_le_bytes());
            for run in &page.runs {
                out.extend(run.y.to_le_bytes());
                out.extend(run.begin.to_le_bytes());
                out.extend(run.end.to_le_bytes());
                out.extend(run.subarea.to_le_bytes());
            }
        }
    }
    if out.len() > MAX_REPLY {
        return Err(Error::LimitExceeded);
    }
    Ok(out)
}
struct Reader<'a>(&'a [u8]);
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let value = self.0.get(..n).ok_or(Error::Malformed)?;
        self.0 = &self.0[n..];
        Ok(value)
    }
    fn byte(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn short(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(
            self.take(2)?.try_into().map_err(|_| Error::Malformed)?,
        ))
    }
    fn number(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().map_err(|_| Error::Malformed)?,
        ))
    }
    fn next(&mut self) -> Result<Option<u16>> {
        Ok(match self.short()? {
            0 => None,
            n => Some(n),
        })
    }
}
pub fn decode_reply(body: &[u8]) -> Result<Reply> {
    if body.len() > MAX_REPLY {
        return Err(Error::LimitExceeded);
    }
    let mut reader = Reader(body);
    let reply = match reader.byte()? {
        1 => {
            let parent = reader.number()?;
            let after = reader.short()?;
            let next = reader.next()?;
            let count = usize::from(reader.byte()?);
            if count > AREA_PAGE {
                return Err(Error::LimitExceeded);
            }
            let mut areas = Vec::with_capacity(count);
            for _ in 0..count {
                let id = reader.number()?;
                let parent = reader.number()?;
                let length = usize::from(reader.short()?);
                if length == 0 || length > MAX_NAME {
                    return Err(Error::LimitExceeded);
                }
                let name = std::str::from_utf8(reader.take(length)?)
                    .map_err(|_| Error::Malformed)?
                    .into();
                areas.push(Area { id, parent, name });
            }
            Reply::Areas(AreaPage {
                parent,
                after,
                next,
                areas,
            })
        }
        2 => {
            let area = reader.number()?;
            let floor = reader.byte()? as i8;
            let page = reader.short()?;
            let next = reader.next()?;
            let total = reader.number()?;
            let bounds = [
                reader.short()?,
                reader.short()?,
                reader.short()?,
                reader.short()?,
            ];
            let count = usize::from(reader.short()?);
            if count > RUN_PAGE {
                return Err(Error::LimitExceeded);
            }
            let mut runs = Vec::with_capacity(count);
            for _ in 0..count {
                runs.push(Run {
                    y: reader.short()?,
                    begin: reader.short()?,
                    end: reader.short()?,
                    subarea: reader.number()?,
                });
            }
            Reply::Geometry(GeometryPage {
                area,
                floor,
                page,
                next,
                total,
                bounds,
                runs,
            })
        }
        _ => return Err(Error::Malformed),
    };
    if !reader.0.is_empty() {
        return Err(Error::Malformed);
    }
    validate(&reply)?;
    Ok(reply)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn atlas_wire_bounds_roundtrip_and_truncation() -> Result<()> {
        let request = Request {
            area: 7,
            floor: -7,
            after: 0,
        };
        assert_eq!(decode_request(&encode_request(request)?)?, request);
        let reply = Reply::Geometry(GeometryPage {
            area: 7,
            floor: -7,
            page: 0,
            next: None,
            total: RUN_PAGE as u32,
            bounds: [1, 0, 2, 1023],
            runs: (0..1024)
                .map(|y| Run {
                    y,
                    begin: 1,
                    end: 3,
                    subarea: 8,
                })
                .collect(),
        });
        let bytes = encode_reply(&reply)?;
        assert_eq!(bytes.len(), MAX_REPLY);
        assert_eq!(decode_reply(&bytes)?, reply);
        for length in [0, 1, 23, bytes.len() - 1] {
            assert!(decode_reply(&bytes[..length]).is_err());
        }
        assert!(decode_reply(&vec![0; MAX_REPLY + 1]).is_err());
        let mut bad = reply.clone();
        if let Reply::Geometry(page) = &mut bad {
            page.runs[1] = page.runs[0];
        }
        assert!(encode_reply(&bad).is_err());
        let names = Reply::Areas(AreaPage {
            parent: 0,
            after: 0,
            next: None,
            areas: (1..=16)
                .map(|id| Area {
                    id,
                    parent: 0,
                    name: "n".repeat(MAX_NAME),
                })
                .collect(),
        });
        assert_eq!(decode_reply(&encode_reply(&names)?)?, names);
        let mut invalid = names;
        if let Reply::Areas(page) = &mut invalid {
            page.areas[0].name.push('x');
        }
        assert!(encode_reply(&invalid).is_err());
        let mut full = reply;
        if let Reply::Geometry(page) = &mut full {
            page.total = MAX_RUNS as u32;
            page.next = Some(1);
        }
        assert_eq!(decode_reply(&encode_reply(&full)?)?, full);
        if let Reply::Geometry(page) = &mut full {
            page.total += 1;
        }
        assert!(encode_reply(&full).is_err());
        assert!(
            encode_request(Request {
                area: 0,
                floor: 1,
                after: 1025
            })
            .is_err()
        );
        Ok(())
    }
}
