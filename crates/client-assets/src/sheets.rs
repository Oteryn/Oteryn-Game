use crate::catalog::{Catalog, SpriteLayout, SpriteSheetDescriptor};
use crate::error::AssetError;
use crate::store::{AssetStore, MAX_COMPRESSED_SHEET_BYTES, MAX_DECOMPRESSED_SHEET_BYTES};
use std::io::{self, BufReader, Cursor, Write};

/// Edge of one drawn cell in pixels.
pub const CELL_PX: u32 = 32;
/// Bytes of one RGBA cell.
pub const CELL_RGBA_BYTES: usize = (CELL_PX * CELL_PX * 4) as usize;
/// Decoded sheets kept in memory, least recently used first out (about 37 MiB).
pub const MAX_RESIDENT_SHEETS: usize = 64;

const SHEET_PX: usize = 384;
const CIP_HEADER_BYTES: usize = 32;
const LZMA_SIZE_FIELD: std::ops::Range<usize> = 5..13;

struct ResidentSheet {
    file: String,
    last_used: u64,
    rgba: Vec<u8>,
}

/// Sprite id to sheet and layout. Sheets are verified and decoded on demand.
pub struct SpriteSheets {
    store: AssetStore,
    catalog: Catalog,
    tick: u64,
    resident: Vec<ResidentSheet>,
    decodes: u64,
}

impl SpriteSheets {
    #[must_use]
    pub fn new(store: AssetStore, catalog: Catalog) -> Self {
        Self {
            store,
            catalog,
            tick: 0,
            resident: Vec::with_capacity(MAX_RESIDENT_SHEETS),
            decodes: 0,
        }
    }

    /// Layout of a sprite, from the catalogue alone (no sheet is decoded).
    pub fn layout(&self, sprite_id: u32) -> Result<SpriteLayout, AssetError> {
        self.catalog
            .locate(sprite_id)
            .map(|sheet| sheet.layout)
            .ok_or(AssetError::UnknownSprite { sprite_id })
    }

    #[must_use]
    pub fn resident_sheets(&self) -> usize {
        self.resident.len()
    }

    /// Sheets decoded since creation, evicted ones included.
    #[must_use]
    pub const fn decode_count(&self) -> u64 {
        self.decodes
    }

    /// True when the sheet file is in memory.
    #[must_use]
    pub fn is_resident(&self, file: &str) -> bool {
        self.resident.iter().any(|sheet| sheet.file == file)
    }

    /// The sheet that holds `sprite_id`.
    #[must_use]
    pub fn sheet_of(&self, sprite_id: u32) -> Option<&SpriteSheetDescriptor> {
        self.catalog.locate(sprite_id)
    }

    /// The 32x32 RGBA cell `(cell_x, cell_y)` of a sprite; magenta is transparent.
    pub fn cell_rgba(
        &mut self,
        sprite_id: u32,
        cell_x: u32,
        cell_y: u32,
    ) -> Result<Vec<u8>, AssetError> {
        let descriptor = self
            .catalog
            .locate(sprite_id)
            .cloned()
            .ok_or(AssetError::UnknownSprite { sprite_id })?;
        let (cells_x, cells_y) = descriptor.layout.cells();
        if cell_x >= cells_x || cell_y >= cells_y {
            return Err(AssetError::Malformed {
                file: descriptor.file,
                reason: format!("cell ({cell_x}, {cell_y}) is outside the sprite"),
            });
        }
        let index = self.load_sheet(&descriptor)?;
        let (width, height) = descriptor.layout.dimensions();
        let columns = SHEET_PX as u32 / width;
        let slot = sprite_id - descriptor.first_sprite_id;
        let origin_x = (slot % columns) * width + cell_x * CELL_PX;
        let origin_y = (slot / columns) * height + cell_y * CELL_PX;
        if origin_y + CELL_PX > SHEET_PX as u32 {
            return Err(AssetError::Malformed {
                file: descriptor.file,
                reason: format!("sprite {sprite_id} lies outside the sheet"),
            });
        }
        let sheet = &self.resident[index].rgba;
        let mut cell = Vec::with_capacity(CELL_RGBA_BYTES);
        for row in 0..CELL_PX {
            let start = ((origin_y + row) as usize * SHEET_PX + origin_x as usize) * 4;
            cell.extend_from_slice(&sheet[start..start + (CELL_PX as usize) * 4]);
        }
        Ok(cell)
    }

    fn load_sheet(&mut self, descriptor: &SpriteSheetDescriptor) -> Result<usize, AssetError> {
        self.tick = self.tick.saturating_add(1);
        if let Some(index) = self
            .resident
            .iter()
            .position(|sheet| sheet.file == descriptor.file)
        {
            self.resident[index].last_used = self.tick;
            return Ok(index);
        }
        let bytes = self
            .store
            .read_verified(&descriptor.file, MAX_COMPRESSED_SHEET_BYTES)?;
        let rgba = decode_sheet(&descriptor.file, &bytes)?;
        self.decodes = self.decodes.saturating_add(1);
        let sheet = ResidentSheet {
            file: descriptor.file.clone(),
            last_used: self.tick,
            rgba,
        };
        if self.resident.len() < MAX_RESIDENT_SHEETS {
            self.resident.push(sheet);
            return Ok(self.resident.len() - 1);
        }
        let oldest = self
            .resident
            .iter()
            .enumerate()
            .min_by_key(|(_, sheet)| sheet.last_used)
            .map_or(0, |(index, _)| index);
        self.resident[oldest] = sheet;
        Ok(oldest)
    }
}

/// Strips the 32-byte CIP header, decodes the LZMA1 payload (size field unknown) and reads the
/// BMP as 384x384 RGBA with magenta as transparent. Ported from the World VFX experiment.
fn decode_sheet(file: &str, encoded: &[u8]) -> Result<Vec<u8>, AssetError> {
    let malformed = |reason: String| AssetError::Malformed {
        file: file.to_owned(),
        reason,
    };
    if encoded.len() <= CIP_HEADER_BYTES + LZMA_SIZE_FIELD.end {
        return Err(malformed("sheet is too small".into()));
    }
    let mut lzma_alone = encoded[CIP_HEADER_BYTES..].to_vec();
    lzma_alone[LZMA_SIZE_FIELD].fill(0xFF);
    let mut input = BufReader::new(Cursor::new(lzma_alone));
    let mut output = BoundedWriter {
        bytes: Vec::new(),
        limit: MAX_DECOMPRESSED_SHEET_BYTES,
    };
    lzma_rs::lzma_decompress(&mut input, &mut output)
        .map_err(|error| malformed(format!("LZMA1 payload: {error}")))?;
    bmp_to_rgba(&output.bytes).map_err(malformed)
}

fn bmp_to_rgba(data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() < 54 || &data[0..2] != b"BM" {
        return Err("decoded sheet is not a Windows BMP".into());
    }
    let u32_at = |at: usize| -> Result<u32, String> {
        data.get(at..at + 4)
            .and_then(|bytes| bytes.try_into().ok())
            .map(u32::from_le_bytes)
            .ok_or_else(|| "truncated BMP header".to_owned())
    };
    let pixel_offset = u32_at(10)? as usize;
    let width = u32_at(18)? as i32;
    let signed_height = u32_at(22)? as i32;
    let bpp = u16::from_le_bytes([data[28], data[29]]);
    let compression = u32_at(30)?;
    if width != SHEET_PX as i32 || signed_height.unsigned_abs() as usize != SHEET_PX {
        return Err(format!(
            "sheet must be {SHEET_PX}x{SHEET_PX}, got {width}x{signed_height}"
        ));
    }
    if bpp != 32 || (compression != 0 && compression != 3) {
        return Err(format!(
            "unsupported BMP bpp {bpp} or compression {compression}"
        ));
    }
    if compression == 3 {
        let masks = [u32_at(54)?, u32_at(58)?, u32_at(62)?, u32_at(66)?];
        if masks != [0x00FF_0000, 0x0000_FF00, 0x0000_00FF, 0xFF00_0000] {
            return Err(format!("unsupported BMP channel masks {masks:?}"));
        }
    }
    let row_bytes = SHEET_PX * 4;
    let end = pixel_offset
        .checked_add(row_bytes * SHEET_PX)
        .ok_or_else(|| "BMP byte range overflow".to_owned())?;
    if end > data.len() {
        return Err("BMP pixel data is truncated".into());
    }
    let top_down = signed_height < 0;
    let mut rgba = vec![0_u8; SHEET_PX * row_bytes];
    for y in 0..SHEET_PX {
        let source_y = if top_down { y } else { SHEET_PX - 1 - y };
        for x in 0..SHEET_PX {
            let source = pixel_offset + source_y * row_bytes + x * 4;
            let (b, g, r, a) = (
                data[source],
                data[source + 1],
                data[source + 2],
                data[source + 3],
            );
            let pixel = if r == 0xFF && g == 0 && b == 0xFF {
                [0, 0, 0, 0]
            } else {
                [r, g, b, a]
            };
            let destination = y * row_bytes + x * 4;
            rgba[destination..destination + 4].copy_from_slice(&pixel);
        }
    }
    Ok(rgba)
}

struct BoundedWriter {
    bytes: Vec<u8>,
    limit: usize,
}

impl Write for BoundedWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if self.bytes.len().saturating_add(buffer.len()) > self.limit {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "decompressed sheet exceeds its bound",
            ));
        }
        self.bytes.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
