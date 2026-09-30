//! Reader for `OTERYN_WORLD_REGION_B3/v1` region files, the World Project base-map source.
//!
//! It follows the B3 codec of the world-authoring tools (`world_region_codec.py`) byte for
//! byte: a 12-byte header `"OTRB" | version u8 | z u8 | rx u16 | ry u16 | sector_count u16`,
//! a table of `local u8 | offset u32 | length u32` rows, then one zstd frame per sector.

use crate::Error;
use crate::sector::{self, Tile, TileLimits};

const HEADER: usize = 12;
const ENTRY: usize = 9;
const SECTORS_PER_SIDE: u16 = 8;
/// The codec's `MAX_SECTOR_BYTES`.
pub const MAX_SECTOR_BYTES: usize = 16 * 1024 * 1024;
const MAX_FLOOR: u8 = 15;

/// One decoded region: legacy floor `z`, region coordinates and the tiles of every sector.
#[derive(Debug)]
pub struct Region {
    pub z: u8,
    pub rx: u16,
    pub ry: u16,
    /// `(sx, sy, tiles)` in ascending table order; sector coordinates are absolute.
    pub sectors: Vec<(u16, u16, Vec<Tile>)>,
}

fn le16(data: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([data[at], data[at + 1]])
}

fn le32(data: &[u8], at: usize) -> usize {
    u32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]]) as usize
}

/// Decodes a whole region file, checking its layout the way the B3 codec does.
pub fn decode_region(data: &[u8], limits: TileLimits) -> Result<Region, Error> {
    let bad = |what: &str| Err(Error::Format(format!("B3: {what}")));
    if data.len() < HEADER {
        return bad("region file shorter than its header");
    }
    if &data[..4] != b"OTRB" || data[4] != 1 {
        return bad("not an OTERYN_WORLD_REGION_B3/v1 file");
    }
    let (z, rx, ry, count) = (
        data[5],
        le16(data, 6),
        le16(data, 8),
        usize::from(le16(data, 10)),
    );
    let per_region = usize::from(SECTORS_PER_SIDE * SECTORS_PER_SIDE);
    if z > MAX_FLOOR || count == 0 || count > per_region {
        return bad("header floor or sector count out of range");
    }
    if rx > 255 || ry > 255 {
        return bad("region lies outside the u16 coordinate plane");
    }
    let mut expected = HEADER + ENTRY * count;
    if data.len() < expected {
        return bad("sector table runs past the end of the file");
    }
    let mut sectors = Vec::with_capacity(count);
    let mut previous: i32 = -1;
    for row in 0..count {
        let at = HEADER + ENTRY * row;
        let (local, offset, length) = (data[at], le32(data, at + 1), le32(data, at + 5));
        if i32::from(local) <= previous || usize::from(local) >= per_region {
            return bad("sector table must be strictly ascending within 0..63");
        }
        if offset != expected || length == 0 || data.len() - offset < length {
            return bad("sector payloads must be non-empty and contiguous");
        }
        previous = i32::from(local);
        expected += length;
        let payload = zstd::bulk::decompress(&data[offset..offset + length], MAX_SECTOR_BYTES)
            .map_err(|error| Error::Format(format!("B3: sector {local}: {error}")))?;
        let sx = rx * SECTORS_PER_SIDE + u16::from(local) % SECTORS_PER_SIDE;
        let sy = ry * SECTORS_PER_SIDE + u16::from(local) / SECTORS_PER_SIDE;
        sectors.push((sx, sy, sector::decode(&payload, sx, sy, limits)?));
    }
    if expected != data.len() {
        return bad("bytes after the last sector payload");
    }
    Ok(Region { z, rx, ry, sectors })
}
