//! The sector payload grammar shared by `OTERYN_WORLD_REGION_B3/v1` and the World Bundle.
//!
//! One payload holds the tiles of one 32x32 sector. All integers are LEB128 varints unless
//! noted. The grammar is the B3 codec's (`world_region_codec.py`); the bundle reuses it and
//! changes only what the palette index and the teleport floor byte mean (format document §5).

use crate::Error;

pub const SECTOR_SIZE: u16 = 32;
const SECTOR_TILES: usize = 32 * 32;
const DEPTH_BIT: u64 = 1;
const COUNT_BIT: u64 = 1 << 1;
const ACTION_BIT: u64 = 1 << 2;
const UNIQUE_BIT: u64 = 1 << 3;
const DOOR_BIT: u64 = 1 << 4;
const TEXT_BIT: u64 = 1 << 5;
const CHARGES_BIT: u64 = 1 << 6;
const DESCRIPTION_BIT: u64 = 1 << 7;
const TELEPORT_BIT: u64 = 1 << 8;
const DEPOT_BIT: u64 = 1 << 9;
const KNOWN_MASK: u64 = (1 << 10) - 1;

/// Item attributes in mask-bit order. Presence is exact: a present zero stays present.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Attrs {
    pub count: Option<u8>,
    pub action: Option<u16>,
    pub unique: Option<u16>,
    pub door: Option<u8>,
    pub text: Option<String>,
    pub charges: Option<u16>,
    pub description: Option<String>,
    /// `(x, y, floor byte)`: legacy `z` in B3, the native floor as `i8` in a bundle.
    pub teleport: Option<(u16, u16, u8)>,
    pub depot: Option<u16>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub palette: u32,
    /// 0 for a top-level entry; container contents follow their container one level deeper.
    pub depth: u8,
    pub attrs: Attrs,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tile {
    pub x: u16,
    pub y: u16,
    pub flags: u32,
    pub house: u32,
    pub zones: Vec<u16>,
    pub items: Vec<Item>,
}

/// Bounds a decoder enforces on one payload before it allocates for it.
#[derive(Clone, Copy, Debug)]
pub struct TileLimits {
    pub max_entries: usize,
    pub max_text_bytes: usize,
}

/// What one whole decode (all sectors of a bundle or of a source) may still produce.
#[derive(Clone, Copy, Debug)]
pub struct Budget {
    pub tiles: usize,
    pub entries: usize,
}

impl Budget {
    fn take(&mut self, tiles: usize, entries: usize) -> Result<(), Error> {
        match (
            self.tiles.checked_sub(tiles),
            self.entries.checked_sub(entries),
        ) {
            (Some(t), Some(e)) => {
                (self.tiles, self.entries) = (t, e);
                Ok(())
            }
            _ => Err(Error::Limit(
                "decoded tiles or entries over the bundle total".into(),
            )),
        }
    }
}

fn put(out: &mut Vec<u8>, mut value: u64) {
    while value >= 0x80 {
        out.push((value & 0x7F) as u8 | 0x80);
        value >>= 7;
    }
    out.push(value as u8);
}

fn put_text(out: &mut Vec<u8>, text: &str) {
    put(out, text.len() as u64);
    out.extend_from_slice(text.as_bytes());
}

fn mask_of(item: &Item) -> u64 {
    let a = &item.attrs;
    let bits = [
        (item.depth != 0, DEPTH_BIT),
        (a.count.is_some(), COUNT_BIT),
        (a.action.is_some(), ACTION_BIT),
        (a.unique.is_some(), UNIQUE_BIT),
        (a.door.is_some(), DOOR_BIT),
        (a.text.is_some(), TEXT_BIT),
        (a.charges.is_some(), CHARGES_BIT),
        (a.description.is_some(), DESCRIPTION_BIT),
        (a.teleport.is_some(), TELEPORT_BIT),
        (a.depot.is_some(), DEPOT_BIT),
    ];
    bits.iter().filter(|(on, _)| *on).map(|(_, bit)| bit).sum()
}

/// Encodes tiles of one sector. The caller supplies them sorted strictly ascending by (y, x).
pub fn encode(tiles: &[Tile]) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    put(&mut out, tiles.len() as u64);
    let mut previous: i64 = -1;
    for tile in tiles {
        let index = i64::from(tile.y % SECTOR_SIZE) * 32 + i64::from(tile.x % SECTOR_SIZE);
        if index <= previous {
            return Err(Error::Format(
                "sector tiles must be strictly ascending".into(),
            ));
        }
        put(&mut out, (index - previous - 1) as u64);
        previous = index;
        let control = (tile.items.len() as u64) << 3
            | u64::from(!tile.zones.is_empty()) << 2
            | u64::from(tile.house != 0) << 1
            | u64::from(tile.flags != 0);
        put(&mut out, control);
        if tile.flags != 0 {
            put(&mut out, tile.flags.into());
        }
        if tile.house != 0 {
            put(&mut out, tile.house.into());
        }
        if !tile.zones.is_empty() {
            put(&mut out, tile.zones.len() as u64);
            tile.zones
                .iter()
                .for_each(|zone| put(&mut out, (*zone).into()));
        }
        for item in &tile.items {
            let mask = mask_of(item);
            put(
                &mut out,
                u64::from(item.palette) << 1 | u64::from(mask != 0),
            );
            if mask == 0 {
                continue;
            }
            put(&mut out, mask);
            let a = &item.attrs;
            if item.depth != 0 {
                out.push(item.depth);
            }
            a.count.iter().for_each(|v| out.push(*v));
            [a.action, a.unique]
                .iter()
                .flatten()
                .for_each(|v| put(&mut out, (*v).into()));
            a.door.iter().for_each(|v| out.push(*v));
            a.text.iter().for_each(|v| put_text(&mut out, v));
            a.charges.iter().for_each(|v| put(&mut out, (*v).into()));
            a.description.iter().for_each(|v| put_text(&mut out, v));
            if let Some((x, y, floor)) = a.teleport {
                put(&mut out, x.into());
                put(&mut out, y.into());
                out.push(floor);
            }
            a.depot.iter().for_each(|v| put(&mut out, (*v).into()));
        }
    }
    Ok(out)
}

struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl Reader<'_> {
    fn byte(&mut self) -> Result<u8, Error> {
        let byte = *self.buf.get(self.pos).ok_or_else(truncated)?;
        self.pos += 1;
        Ok(byte)
    }

    fn varint(&mut self) -> Result<u64, Error> {
        let (mut value, mut shift) = (0u64, 0u32);
        loop {
            let byte = self.byte()?;
            // Canonical LEB128 only: no bits past 64 and no redundant zero high group.
            if (shift == 63 && byte > 1) || (shift > 0 && byte == 0) {
                return Err(Error::Format("varint is not canonical".into()));
            }
            value |= u64::from(byte & 0x7F) << shift;
            if byte < 0x80 {
                return Ok(value);
            }
            shift += 7;
        }
    }

    fn bounded(&mut self, max: u64, what: &str) -> Result<u64, Error> {
        let value = self.varint()?;
        if value > max {
            return Err(Error::Format(format!("{what} {value} out of range")));
        }
        Ok(value)
    }

    fn u16(&mut self, what: &str) -> Result<u16, Error> {
        Ok(self.bounded(0xFFFF, what)? as u16)
    }

    fn text(&mut self, limits: TileLimits) -> Result<String, Error> {
        let length = self.varint()?;
        if length > limits.max_text_bytes as u64 {
            return Err(Error::Limit(format!("text of {length} bytes")));
        }
        let length = length as usize;
        let end = self
            .pos
            .checked_add(length)
            .filter(|end| *end <= self.buf.len());
        let raw = self
            .buf
            .get(self.pos..end.ok_or_else(truncated)?)
            .ok_or_else(truncated)?;
        self.pos += length;
        String::from_utf8(raw.to_vec()).map_err(|_| Error::Format("text is not UTF-8".into()))
    }
}

fn truncated() -> Error {
    Error::Format("sector payload is truncated".into())
}

/// Decodes one payload of sector `(sx, sy)`, charging `budget` before anything is reserved.
/// Rejects trailing bytes and unknown attribute bits.
pub fn decode(
    payload: &[u8],
    (sx, sy): (u16, u16),
    limits: TileLimits,
    budget: &mut Budget,
) -> Result<Vec<Tile>, Error> {
    if sx >= 2048 || sy >= 2048 {
        return Err(Error::Format(
            "sector outside the u16 coordinate plane".into(),
        ));
    }
    let mut r = Reader {
        buf: payload,
        pos: 0,
    };
    let count = r.bounded(SECTOR_TILES as u64, "sector tile count")? as usize;
    budget.take(count, 0)?;
    let mut tiles = Vec::with_capacity(count);
    let mut previous: i64 = -1;
    for _ in 0..count {
        let index = previous + 1 + r.bounded(SECTOR_TILES as u64, "tile delta")? as i64;
        if index >= SECTOR_TILES as i64 {
            return Err(Error::Format("tile index outside the sector".into()));
        }
        previous = index;
        let control = r.varint()?;
        let entries = (control >> 3) as usize;
        if entries > limits.max_entries {
            return Err(Error::Limit(format!("tile holds {entries} entries")));
        }
        budget.take(0, entries)?;
        let mut tile = Tile {
            x: sx * SECTOR_SIZE + (index & 31) as u16,
            y: sy * SECTOR_SIZE + (index >> 5) as u16,
            flags: 0,
            house: 0,
            zones: Vec::new(),
            items: Vec::with_capacity(entries),
        };
        if control & 1 != 0 {
            tile.flags = r.bounded(0xFFFF_FFFF, "tile flags")? as u32;
        }
        if control & 2 != 0 {
            tile.house = r.bounded(0xFFFF_FFFF, "house id")? as u32;
        }
        if control & 4 != 0 {
            let zones = r.bounded(0xFFFF, "tile zone count")?;
            for _ in 0..zones {
                tile.zones.push(r.u16("tile zone id")?);
            }
        }
        if (control & 1 != 0 && tile.flags == 0)
            || (control & 2 != 0 && tile.house == 0)
            || (control & 4 != 0 && tile.zones.is_empty())
        {
            return Err(Error::Format("present tile field is zero or empty".into()));
        }
        let mut depth_limit = 0u8;
        for _ in 0..entries {
            let word = r.bounded(0xFFFF_FFFF, "item palette word")?;
            let mut item = Item {
                palette: (word >> 1) as u32,
                depth: 0,
                attrs: Attrs::default(),
            };
            if word & 1 != 0 {
                let mask = r.varint()?;
                if mask == 0 || mask & !KNOWN_MASK != 0 {
                    return Err(Error::Format(
                        "item attribute mask is empty or unknown".into(),
                    ));
                }
                let a = &mut item.attrs;
                if mask & DEPTH_BIT != 0 {
                    item.depth = r.byte()?;
                    if item.depth == 0 {
                        return Err(Error::Format("depth attribute must be non-zero".into()));
                    }
                }
                if mask & COUNT_BIT != 0 {
                    a.count = Some(r.byte()?);
                }
                if mask & ACTION_BIT != 0 {
                    a.action = Some(r.u16("action")?);
                }
                if mask & UNIQUE_BIT != 0 {
                    a.unique = Some(r.u16("unique")?);
                }
                if mask & DOOR_BIT != 0 {
                    a.door = Some(r.byte()?);
                }
                if mask & TEXT_BIT != 0 {
                    a.text = Some(r.text(limits)?);
                }
                if mask & CHARGES_BIT != 0 {
                    a.charges = Some(r.u16("charges")?);
                }
                if mask & DESCRIPTION_BIT != 0 {
                    a.description = Some(r.text(limits)?);
                }
                if mask & TELEPORT_BIT != 0 {
                    a.teleport = Some((r.u16("teleport x")?, r.u16("teleport y")?, r.byte()?));
                }
                if mask & DEPOT_BIT != 0 {
                    a.depot = Some(r.u16("depot")?);
                }
            }
            if item.depth > depth_limit {
                return Err(Error::Format("item depth skips a container level".into()));
            }
            depth_limit = item.depth.saturating_add(1);
            tile.items.push(item);
        }
        tiles.push(tile);
    }
    if r.pos != payload.len() {
        return Err(Error::Format(
            "bytes after the last tile of a sector".into(),
        ));
    }
    Ok(tiles)
}
