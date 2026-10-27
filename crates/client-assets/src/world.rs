//! Bounded, hash-pinned immutable placement index and B3 region decoding.

use crate::text_sha256;
pub use oteryn_world_bundle::sector::Budget;
use oteryn_world_bundle::sector::{self, TileLimits};
use serde::Deserialize;
use std::io::Read;
use std::path::Path;

/// The sha256 of `imports/official/client-assets/15.30/manifest.json`. The manifest pins every
/// asset file, so the client pins the manifest instead of trusting the one in the asset root.
pub const MANIFEST_SHA256: &str =
    "febaff9f4bd7e0f8a029736e446a81f1626805e895e7c2268018e0a9a8493fe4";
pub const MANIFEST_PATH: &str = "imports/official/client-assets/15.30/manifest.json";
/// The sha256 of `content/world/placements/index.json`, which pins every region file. A
/// regenerated placement index must update it; a test compares it with the repository.
pub const PLACEMENT_INDEX_SHA256: &str =
    "ad24e9b7a3fee668feb5eed466458af329d61487db7a4ef7753e2618f4b377c4";
pub const PLACEMENT_INDEX_PATH: &str = "content/world/placements/index.json";
const MAX_INDEX_BYTES: u64 = 8 * 1024 * 1024;
pub const MAX_REGION_BYTES: u64 = 32 * 1024 * 1024;
const MAX_SECTOR_BYTES: usize = 16 * 1024 * 1024;
pub const REGION_TILES: i32 = 256;
/// The side of one sector: a region is 8 x 8 sectors.
pub const SECTOR_TILES: i32 = REGION_TILES / 8;
#[derive(Deserialize)]
pub struct PlacementIndex {
    pub palette: Vec<PaletteEntry>,
    pub regions: Vec<RegionEntry>,
}

#[derive(Deserialize)]
pub struct PaletteEntry {
    pub source_item_id: u32,
}

#[derive(Deserialize)]
pub struct RegionEntry {
    pub path: String,
    pub sha256: String,
}

/// Reads the placement index, refusing one that does not match [`PLACEMENT_INDEX_SHA256`].
pub fn read_placement_index(root: &Path) -> Result<PlacementIndex, String> {
    let bytes = read_capped(&root.join(PLACEMENT_INDEX_PATH), MAX_INDEX_BYTES)?;
    if text_sha256(&bytes) != PLACEMENT_INDEX_SHA256 {
        return Err("placement index: sha256 does not match the pinned digest".to_owned());
    }
    serde_json::from_slice(&bytes).map_err(|error| format!("placement index: {error}"))
}

/// The palette entry an item names; an index outside the palette fails the whole map closed.
pub fn palette_entry<'a>(
    palette: &'a [PaletteEntry],
    index: u32,
    region: &str,
) -> Result<&'a PaletteEntry, String> {
    usize::try_from(index)
        .ok()
        .and_then(|index| palette.get(index))
        .ok_or_else(|| format!("{region}: palette index {index} outside the placement index"))
}

pub fn read_capped(path: &Path, max: u64) -> Result<Vec<u8>, String> {
    let file = std::fs::File::open(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut bytes = Vec::new();
    file.take(max + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    if bytes.len() as u64 > max {
        return Err(format!("{}: larger than {max} bytes", path.display()));
    }
    Ok(bytes)
}

/// Decodes an `OTERYN_WORLD_REGION_B3/v1` file the way `world-bundle-compiler`'s `b3` reader
/// does: a 12-byte header, `local u8 | offset u32 | length u32` rows, one zstd frame per sector.
/// The header must name `region` (`z, rx, ry`), the region the file was looked up for. Only
/// sectors that overlap `window` (`lo_x, hi_x, lo_y, hi_y`, inclusive) are decompressed.
pub fn decode_region(
    data: &[u8],
    region: (u8, i32, i32),
    window: (i32, i32, i32, i32),
    budget: &mut Budget,
) -> Result<Vec<sector::Tile>, String> {
    const HEADER: usize = 12;
    const ROW: usize = 9;
    let le16 = |at: usize| u16::from_le_bytes([data[at], data[at + 1]]);
    let le32 = |at: usize| {
        u32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]]) as usize
    };
    if data.len() < HEADER || &data[..4] != b"OTRB" || data[4] != 1 {
        return Err("not an OTERYN_WORLD_REGION_B3/v1 file".into());
    }
    let (rx, ry, count) = (le16(6), le16(8), usize::from(le16(10)));
    if count == 0 || count > 64 || rx > 255 || ry > 255 {
        return Err("header out of range".into());
    }
    if (data[5], i32::from(rx), i32::from(ry)) != region {
        return Err("header names another region".into());
    }
    let mut expected = HEADER + ROW * count;
    if data.len() < expected {
        return Err("sector table runs past the end of the file".into());
    }
    let limits = TileLimits {
        max_entries: 4096,
        max_text_bytes: 4096,
    };
    let mut tiles = Vec::new();
    let mut previous = -1;
    for row in 0..count {
        let at = HEADER + ROW * row;
        let (local, offset, length) = (data[at], le32(at + 1), le32(at + 5));
        if i32::from(local) <= previous || local >= 64 {
            return Err("sector table must be strictly ascending within 0..63".into());
        }
        if offset != expected || length == 0 || data.len() - offset < length {
            return Err("sector payloads must be non-empty and contiguous".into());
        }
        previous = i32::from(local);
        expected += length;
        let (sx, sy) = (rx * 8 + u16::from(local) % 8, ry * 8 + u16::from(local) / 8);
        let (x0, y0) = (i32::from(sx) * SECTOR_TILES, i32::from(sy) * SECTOR_TILES);
        let (lo_x, hi_x, lo_y, hi_y) = window;
        if x0 > hi_x || x0 + SECTOR_TILES <= lo_x || y0 > hi_y || y0 + SECTOR_TILES <= lo_y {
            continue;
        }
        let frame = &data[offset..offset + length];
        let capacity = match zstd::zstd_safe::get_frame_content_size(frame) {
            Ok(Some(size)) if size <= MAX_SECTOR_BYTES as u64 => size as usize,
            Ok(Some(_)) => return Err("sector payload over the cap".into()),
            _ => MAX_SECTOR_BYTES,
        };
        let payload = zstd::bulk::decompress(frame, capacity)
            .map_err(|error| format!("sector {local}: {error}"))?;
        let mut decoded = sector::decode(&payload, (sx, sy), limits, budget)
            .map_err(|error| format!("sector {local}: {error:?}"))?;
        // Zones and item texts are not drawn and not charged to the budget. Dropping them per
        // sector keeps a region's retained memory to the charged tiles and entries.
        for tile in &mut decoded {
            tile.zones = Vec::new();
            for item in &mut tile.items {
                item.attrs.text = None;
                item.attrs.description = None;
            }
        }
        tiles.extend(decoded);
    }
    if expected != data.len() {
        return Err("bytes after the last sector payload".into());
    }
    Ok(tiles)
}

#[cfg(test)]
mod tests;
