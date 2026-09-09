use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Cursor, Read, Write};
use std::path::{Path, PathBuf};

pub const SOURCE_ZIP_SHA256: &str =
    "1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f";
pub const SOURCE_CATALOG_SHA256: &str =
    "35639e000c4c108665a091cfbdf699d549d995b37670bc08de575ab6cd380d85";
pub const SOURCE_APPEARANCE_SHA256: &str =
    "dc4f4c01e3701c77877c67895168e4399837046122d6d17e3e608a12a2fed075";
pub const THAIS_FIXTURE_ARTIFACT: &str =
    "sha256:4b340053f72b3522a9fe644c9afdf08d9c7b9b686aec0b57cef764a7cb7dc468";
pub const EXPECTED_SPRITE_SHEETS: usize = 5_084;
pub const EXPECTED_THAIS_SPRITES: usize = 990;
pub const SHEET_SIZE: u32 = 384;

const CIP_HEADER_BYTES: usize = 32;
const LZMA_SIZE_FIELD_START: usize = 5;
const LZMA_SIZE_FIELD_END: usize = 13;
const MAX_CATALOG_BYTES: u64 = 16 * 1024 * 1024;
const MAX_COMPRESSED_SHEET_BYTES: u64 = 2 * 1024 * 1024;
const MAX_DECOMPRESSED_SHEET_BYTES: usize = (SHEET_SIZE as usize * SHEET_SIZE as usize * 4) + 65_536;
const MAX_FIXTURE_MANIFEST_BYTES: u64 = 2 * 1024 * 1024;
const MAX_TILE_LINE_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SpriteLayout {
    Size32x32,
    Size32x64,
    Size64x32,
    Size64x64,
}

impl SpriteLayout {
    pub const fn dimensions(self) -> (u32, u32) {
        match self {
            Self::Size32x32 => (32, 32),
            Self::Size32x64 => (32, 64),
            Self::Size64x32 => (64, 32),
            Self::Size64x64 => (64, 64),
        }
    }

    pub const fn capacity(self) -> u32 {
        let (width, height) = self.dimensions();
        (SHEET_SIZE / width) * (SHEET_SIZE / height)
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Size32x32 => "32x32",
            Self::Size32x64 => "32x64",
            Self::Size64x32 => "64x32",
            Self::Size64x64 => "64x64",
        }
    }
}

impl TryFrom<u32> for SpriteLayout {
    type Error = String;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Size32x32),
            1 => Ok(Self::Size32x64),
            2 => Ok(Self::Size64x32),
            3 => Ok(Self::Size64x64),
            other => Err(format!("unsupported 15.32 spritetype {other}")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SpriteSheetDescriptor {
    pub first_sprite_id: u32,
    pub last_sprite_id: u32,
    pub layout: SpriteLayout,
    pub file: String,
}

#[derive(Debug, Deserialize)]
struct RawCatalogEntry {
    #[serde(rename = "type")]
    kind: Option<String>,
    firstspriteid: Option<u32>,
    lastspriteid: Option<u32>,
    spritetype: Option<u32>,
    file: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SourceIdentity {
    pub zip_sha256: String,
    pub catalog_sha256: String,
    pub appearance_sha256: &'static str,
    pub sprite_sheets: usize,
}

#[derive(Debug, Clone)]
pub struct SpriteCatalog {
    assets_dir: PathBuf,
    sheets: Vec<SpriteSheetDescriptor>,
    source: SourceIdentity,
}

impl SpriteCatalog {
    pub fn load(asset_zip: &Path, assets_dir: &Path) -> Result<Self, String> {
        let zip_sha256 = sha256_file(asset_zip)?;
        if zip_sha256 != SOURCE_ZIP_SHA256 {
            return Err(format!(
                "15.32.zip SHA-256 mismatch: expected {SOURCE_ZIP_SHA256}, got {zip_sha256}"
            ));
        }

        let catalog_path = assets_dir.join("catalog-content.json");
        let catalog_bytes = read_bounded_file(&catalog_path, MAX_CATALOG_BYTES, "asset catalog")?;
        let catalog_sha256 = sha256_bytes(&catalog_bytes);
        if catalog_sha256 != SOURCE_CATALOG_SHA256 {
            return Err(format!(
                "catalog-content.json SHA-256 mismatch: expected {SOURCE_CATALOG_SHA256}, got {catalog_sha256}"
            ));
        }

        let raw: Vec<RawCatalogEntry> = serde_json::from_slice(&catalog_bytes)
            .map_err(|error| format!("parse catalog-content.json: {error}"))?;
        let mut sheets = Vec::new();
        for entry in raw {
            if entry.kind.as_deref() != Some("sprite") {
                continue;
            }
            let first = entry
                .firstspriteid
                .ok_or_else(|| "sprite catalog entry is missing firstspriteid".to_owned())?;
            let last = entry
                .lastspriteid
                .ok_or_else(|| "sprite catalog entry is missing lastspriteid".to_owned())?;
            let layout = SpriteLayout::try_from(
                entry
                    .spritetype
                    .ok_or_else(|| "sprite catalog entry is missing spritetype".to_owned())?,
            )?;
            let file = entry
                .file
                .ok_or_else(|| "sprite catalog entry is missing file".to_owned())?;
            validate_sheet_file_name(&file)?;
            if last < first {
                return Err(format!("sprite sheet {file} has inverted ID range {first}..={last}"));
            }
            let count = last - first + 1;
            if count > layout.capacity() {
                return Err(format!(
                    "sprite sheet {file} range contains {count} sprites but {} carries at most {}",
                    layout.label(),
                    layout.capacity()
                ));
            }
            sheets.push(SpriteSheetDescriptor {
                first_sprite_id: first,
                last_sprite_id: last,
                layout,
                file,
            });
        }
        sheets.sort_by_key(|sheet| (sheet.first_sprite_id, sheet.last_sprite_id));
        if sheets.len() != EXPECTED_SPRITE_SHEETS {
            return Err(format!(
                "expected {EXPECTED_SPRITE_SHEETS} sprite sheets, got {}",
                sheets.len()
            ));
        }
        for pair in sheets.windows(2) {
            if pair[1].first_sprite_id <= pair[0].last_sprite_id {
                return Err(format!(
                    "overlapping sprite ranges: {}..={} and {}..={}",
                    pair[0].first_sprite_id,
                    pair[0].last_sprite_id,
                    pair[1].first_sprite_id,
                    pair[1].last_sprite_id
                ));
            }
        }

        Ok(Self {
            assets_dir: assets_dir.to_path_buf(),
            source: SourceIdentity {
                zip_sha256,
                catalog_sha256,
                appearance_sha256: SOURCE_APPEARANCE_SHA256,
                sprite_sheets: sheets.len(),
            },
            sheets,
        })
    }

    pub fn source_identity(&self) -> &SourceIdentity {
        &self.source
    }

    pub fn sheet_count(&self) -> usize {
        self.sheets.len()
    }

    pub fn locate(&self, sprite_id: u32) -> Option<&SpriteSheetDescriptor> {
        let upper = self
            .sheets
            .partition_point(|sheet| sheet.first_sprite_id <= sprite_id);
        if upper == 0 {
            return None;
        }
        let candidate = &self.sheets[upper - 1];
        (sprite_id <= candidate.last_sprite_id).then_some(candidate)
    }

    fn sheet_path(&self, descriptor: &SpriteSheetDescriptor) -> PathBuf {
        self.assets_dir.join(&descriptor.file)
    }
}

fn validate_sheet_file_name(file: &str) -> Result<(), String> {
    if file.is_empty()
        || file.contains('/')
        || file.contains('\\')
        || file == "."
        || file == ".."
        || !file.ends_with(".bmp.lzma")
    {
        return Err(format!("unsafe or unsupported sprite-sheet file name {file:?}"));
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct DecodedSheet {
    rgba: Vec<u8>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpritePixels {
    pub sprite_source_id: u32,
    pub width: u32,
    pub height: u32,
    #[serde(skip_serializing)]
    pub rgba: Vec<u8>,
}

impl SpritePixels {
    pub fn rgba_sha256(&self) -> String {
        sha256_bytes(&self.rgba)
    }
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct CacheStats {
    pub sprite_requests: u64,
    pub sheet_hits: u64,
    pub sheet_misses: u64,
    pub sheet_decodes: u64,
    pub sheet_evictions: u64,
    pub compressed_bytes_read: u64,
    pub decoded_rgba_bytes: u64,
}

#[derive(Debug)]
struct CachedSheet {
    file: String,
    last_used: u64,
    decoded: DecodedSheet,
}

#[derive(Debug)]
pub struct SpriteDecoder {
    catalog: SpriteCatalog,
    capacity: usize,
    tick: u64,
    cache: Vec<CachedSheet>,
    stats: CacheStats,
}

impl SpriteDecoder {
    pub fn new(catalog: SpriteCatalog, capacity: usize) -> Result<Self, String> {
        if capacity == 0 {
            return Err("sprite-sheet cache capacity must be at least one".to_owned());
        }
        Ok(Self {
            catalog,
            capacity,
            tick: 0,
            cache: Vec::with_capacity(capacity),
            stats: CacheStats::default(),
        })
    }

    pub fn source_identity(&self) -> &SourceIdentity {
        self.catalog.source_identity()
    }

    pub fn stats(&self) -> &CacheStats {
        &self.stats
    }

    pub fn resident_sheets(&self) -> usize {
        self.cache.len()
    }

    pub fn decode_sprite(&mut self, sprite_id: u32) -> Result<SpritePixels, String> {
        self.stats.sprite_requests = self.stats.sprite_requests.saturating_add(1);
        self.tick = self.tick.saturating_add(1);
        let descriptor = self
            .catalog
            .locate(sprite_id)
            .cloned()
            .ok_or_else(|| format!("sprite_source_id {sprite_id} is absent from exact 15.32 catalog"))?;

        let cache_index = if let Some(index) = self
            .cache
            .iter()
            .position(|sheet| sheet.file == descriptor.file)
        {
            self.stats.sheet_hits = self.stats.sheet_hits.saturating_add(1);
            self.cache[index].last_used = self.tick;
            index
        } else {
            self.stats.sheet_misses = self.stats.sheet_misses.saturating_add(1);
            let path = self.catalog.sheet_path(&descriptor);
            let (decoded, compressed_bytes) = decode_cip_sprite_sheet(&path)?;
            self.stats.sheet_decodes = self.stats.sheet_decodes.saturating_add(1);
            self.stats.compressed_bytes_read = self
                .stats
                .compressed_bytes_read
                .saturating_add(compressed_bytes);
            self.stats.decoded_rgba_bytes = self
                .stats
                .decoded_rgba_bytes
                .saturating_add(decoded.rgba.len() as u64);

            let new_sheet = CachedSheet {
                file: descriptor.file.clone(),
                last_used: self.tick,
                decoded,
            };
            if self.cache.len() < self.capacity {
                self.cache.push(new_sheet);
                self.cache.len() - 1
            } else {
                let index = self
                    .cache
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, sheet)| sheet.last_used)
                    .map(|(index, _)| index)
                    .ok_or_else(|| "non-empty sprite cache has no eviction candidate".to_owned())?;
                self.cache[index] = new_sheet;
                self.stats.sheet_evictions = self.stats.sheet_evictions.saturating_add(1);
                index
            }
        };

        extract_sprite(&self.cache[cache_index].decoded, &descriptor, sprite_id)
    }
}

fn decode_cip_sprite_sheet(path: &Path) -> Result<(DecodedSheet, u64), String> {
    let encoded = read_bounded_file(path, MAX_COMPRESSED_SHEET_BYTES, "sprite sheet")?;
    if encoded.len() <= CIP_HEADER_BYTES + LZMA_SIZE_FIELD_END {
        return Err(format!("sprite sheet {} is too small", path.display()));
    }

    let mut lzma_alone = encoded[CIP_HEADER_BYTES..].to_vec();
    if lzma_alone.len() < LZMA_SIZE_FIELD_END {
        return Err(format!("sprite sheet {} has truncated LZMA header", path.display()));
    }
    lzma_alone[LZMA_SIZE_FIELD_START..LZMA_SIZE_FIELD_END].fill(0xFF);

    let mut input = BufReader::new(Cursor::new(lzma_alone));
    let mut output = BoundedWriter::new(MAX_DECOMPRESSED_SHEET_BYTES);
    lzma_rs::lzma_decompress(&mut input, &mut output)
        .map_err(|error| format!("decode {} LZMA1 payload: {error}", path.display()))?;
    let bmp = output.into_inner();
    let rgba = parse_bmp_rgba(&bmp)?;
    Ok((DecodedSheet { rgba }, encoded.len() as u64))
}

fn parse_bmp_rgba(data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() < 54 || &data[0..2] != b"BM" {
        return Err("decoded sprite sheet is not a Windows BMP".to_owned());
    }
    let pixel_offset = read_u32_le(data, 10)? as usize;
    let width = read_i32_le(data, 18)?;
    let signed_height = read_i32_le(data, 22)?;
    let bpp = read_u16_le(data, 28)?;
    let compression = read_u32_le(data, 30)?;
    if width != SHEET_SIZE as i32 || signed_height.unsigned_abs() != SHEET_SIZE {
        return Err(format!(
            "decoded sprite sheet dimensions must be {SHEET_SIZE}x{SHEET_SIZE}, got {width}x{signed_height}"
        ));
    }
    if bpp != 32 {
        return Err(format!("decoded sprite sheet must be 32-bpp, got {bpp}"));
    }
    if compression != 0 && compression != 3 {
        return Err(format!("unsupported sprite-sheet BMP compression {compression}"));
    }

    let row_bytes = SHEET_SIZE as usize * 4;
    let required = pixel_offset
        .checked_add(row_bytes.saturating_mul(SHEET_SIZE as usize))
        .ok_or_else(|| "sprite-sheet BMP byte range overflow".to_owned())?;
    if required > data.len() {
        return Err(format!(
            "sprite-sheet BMP pixel data is truncated: need {required}, got {}",
            data.len()
        ));
    }

    let top_down = signed_height < 0;
    let mut rgba = vec![0_u8; SHEET_SIZE as usize * SHEET_SIZE as usize * 4];
    for y in 0..SHEET_SIZE as usize {
        let source_y = if top_down {
            y
        } else {
            SHEET_SIZE as usize - 1 - y
        };
        for x in 0..SHEET_SIZE as usize {
            let source = pixel_offset + source_y * row_bytes + x * 4;
            let destination = (y * SHEET_SIZE as usize + x) * 4;
            let b = data[source];
            let g = data[source + 1];
            let r = data[source + 2];
            let a = data[source + 3];
            if r == 0xFF && g == 0x00 && b == 0xFF {
                rgba[destination..destination + 4].copy_from_slice(&[0, 0, 0, 0]);
            } else {
                rgba[destination..destination + 4].copy_from_slice(&[r, g, b, a]);
            }
        }
    }
    Ok(rgba)
}

fn extract_sprite(
    sheet: &DecodedSheet,
    descriptor: &SpriteSheetDescriptor,
    sprite_id: u32,
) -> Result<SpritePixels, String> {
    if sprite_id < descriptor.first_sprite_id || sprite_id > descriptor.last_sprite_id {
        return Err(format!(
            "sprite {sprite_id} is outside sheet range {}..={}",
            descriptor.first_sprite_id, descriptor.last_sprite_id
        ));
    }
    let (width, height) = descriptor.layout.dimensions();
    let columns = SHEET_SIZE / width;
    let offset = sprite_id - descriptor.first_sprite_id;
    let row = offset / columns;
    let column = offset % columns;
    if row * height + height > SHEET_SIZE {
        return Err(format!(
            "sprite {sprite_id} exceeds physical {} carrier in {}",
            descriptor.layout.label(),
            descriptor.file
        ));
    }

    let mut rgba = vec![0_u8; width as usize * height as usize * 4];
    let source_row_bytes = SHEET_SIZE as usize * 4;
    let sprite_row_bytes = width as usize * 4;
    for local_y in 0..height as usize {
        let source_y = row as usize * height as usize + local_y;
        let source_x = column as usize * width as usize;
        let source_start = source_y * source_row_bytes + source_x * 4;
        let destination_start = local_y * sprite_row_bytes;
        rgba[destination_start..destination_start + sprite_row_bytes]
            .copy_from_slice(&sheet.rgba[source_start..source_start + sprite_row_bytes]);
    }

    Ok(SpritePixels {
        sprite_source_id: sprite_id,
        width,
        height,
        rgba,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Viewport {
    pub x_min: i64,
    pub x_max: i64,
    pub y_min: i64,
    pub y_max: i64,
    pub floor: i64,
}

impl Viewport {
    pub const THAIS_DENSE: Self = Self {
        x_min: 32_400,
        x_max: 32_439,
        y_min: 32_239,
        y_max: 32_268,
        floor: -7,
    };

    const fn contains(self, x: i64, y: i64, floor: i64) -> bool {
        x >= self.x_min
            && x <= self.x_max
            && y >= self.y_min
            && y <= self.y_max
            && floor == self.floor
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct FixtureSelection {
    pub artifact_digest: String,
    pub viewport: Option<Viewport>,
    pub tile_records_scanned: u64,
    pub selected_tile_records: u64,
    pub presentation_primitives: u64,
    pub unique_sprite_ids: usize,
    #[serde(skip_serializing)]
    pub sprite_ids: Vec<u32>,
}

pub fn select_fixture_sprites(
    fixture_dir: &Path,
    viewport: Option<Viewport>,
) -> Result<FixtureSelection, String> {
    let manifest_path = fixture_dir.join("manifest.json");
    let manifest_bytes = read_bounded_file(
        &manifest_path,
        MAX_FIXTURE_MANIFEST_BYTES,
        "Thais fixture manifest",
    )?;
    let manifest: serde_json::Value = serde_json::from_slice(&manifest_bytes)
        .map_err(|error| format!("parse {}: {error}", manifest_path.display()))?;
    let artifact_digest = value_str(&manifest, &["artifact_digest"])?;
    if artifact_digest != THAIS_FIXTURE_ARTIFACT {
        return Err(format!(
            "Thais fixture artifact mismatch: expected {THAIS_FIXTURE_ARTIFACT}, got {artifact_digest}"
        ));
    }
    let source_zip = value_str(
        &manifest,
        &["asset_catalog_revision", "zip_sha256"],
    )?;
    if source_zip != SOURCE_ZIP_SHA256 {
        return Err(format!(
            "Thais fixture source ZIP mismatch: expected {SOURCE_ZIP_SHA256}, got {source_zip}"
        ));
    }
    require_u64(&manifest, &["counts", "tiles"], 24_311)?;
    require_u64(
        &manifest,
        &["counts", "presentation_records"],
        39_282,
    )?;
    require_u64(
        &manifest,
        &["counts", "resolved_primitives"],
        39_282,
    )?;
    require_u64(
        &manifest,
        &["counts", "unique_sprite_source_ids"],
        EXPECTED_THAIS_SPRITES as u64,
    )?;

    let tiles_path = fixture_dir.join("tiles.jsonl");
    let expected_tiles_sha = value_str(&manifest, &["files", "tiles.jsonl", "sha256"])?;
    let actual_tiles_sha = sha256_file(&tiles_path)?;
    if actual_tiles_sha != expected_tiles_sha {
        return Err(format!(
            "Thais tiles.jsonl SHA-256 mismatch: expected {expected_tiles_sha}, got {actual_tiles_sha}"
        ));
    }

    let file = File::open(&tiles_path)
        .map_err(|error| format!("open {}: {error}", tiles_path.display()))?;
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    let mut tile_records_scanned = 0_u64;
    let mut selected_tile_records = 0_u64;
    let mut presentation_primitives = 0_u64;
    let mut sprite_ids = BTreeSet::new();
    loop {
        line.clear();
        let bytes = reader
            .read_line(&mut line)
            .map_err(|error| format!("read {}: {error}", tiles_path.display()))?;
        if bytes == 0 {
            break;
        }
        if bytes > MAX_TILE_LINE_BYTES {
            return Err(format!("{} contains an oversized tile record", tiles_path.display()));
        }
        tile_records_scanned = tile_records_scanned.saturating_add(1);
        let row: serde_json::Value = serde_json::from_str(&line)
            .map_err(|error| format!("parse {} line {tile_records_scanned}: {error}", tiles_path.display()))?;
        let x = value_i64(&row, &["position", "x"])?;
        let y = value_i64(&row, &["position", "y"])?;
        let floor = value_i64(&row, &["position", "floor"])?;
        if viewport.is_some_and(|candidate| !candidate.contains(x, y, floor)) {
            continue;
        }
        selected_tile_records = selected_tile_records.saturating_add(1);
        let presentations = row
            .get("presentation")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| format!("tile {x},{y},{floor} has no presentation array"))?;
        for presentation in presentations {
            let primitives = presentation
                .get("resolved_primitives")
                .and_then(serde_json::Value::as_array)
                .ok_or_else(|| format!("tile {x},{y},{floor} has malformed resolved_primitives"))?;
            for primitive in primitives {
                let sprite_id = primitive
                    .get("sprite_source_id")
                    .and_then(serde_json::Value::as_u64)
                    .ok_or_else(|| format!("tile {x},{y},{floor} has invalid sprite_source_id"))?;
                let sprite_id = u32::try_from(sprite_id)
                    .map_err(|_| "sprite_source_id exceeds u32".to_owned())?;
                sprite_ids.insert(sprite_id);
                presentation_primitives = presentation_primitives.saturating_add(1);
            }
        }
    }

    if tile_records_scanned != 24_311 {
        return Err(format!(
            "Thais fixture tiles.jsonl must contain 24311 records, got {tile_records_scanned}"
        ));
    }
    match viewport {
        None => {
            if presentation_primitives != 39_282 || sprite_ids.len() != EXPECTED_THAIS_SPRITES {
                return Err(format!(
                    "full Thais selection mismatch: primitives={presentation_primitives}, unique_sprites={}",
                    sprite_ids.len()
                ));
            }
        }
        Some(candidate) if candidate == Viewport::THAIS_DENSE => {
            if presentation_primitives != 2_215 {
                return Err(format!(
                    "dense Thais viewport must resolve 2215 primitives, got {presentation_primitives}"
                ));
            }
        }
        Some(_) => {
            if presentation_primitives == 0 {
                return Err("selected Thais viewport contains no presentation primitives".to_owned());
            }
        }
    }

    Ok(FixtureSelection {
        artifact_digest: artifact_digest.to_owned(),
        viewport,
        tile_records_scanned,
        selected_tile_records,
        presentation_primitives,
        unique_sprite_ids: sprite_ids.len(),
        sprite_ids: sprite_ids.into_iter().collect(),
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct DecodeEvidence {
    pub schema: &'static str,
    pub source: SourceIdentity,
    pub fixture: FixtureSelection,
    pub cache: CacheStats,
    pub resident_sheets: usize,
    pub geometry_counts: BTreeMap<String, u64>,
    pub sprite_id_set_sha256: String,
    pub sprite_rgba_root_sha256: String,
}

pub fn qualify_fixture_decode(
    asset_zip: &Path,
    assets_dir: &Path,
    fixture_dir: &Path,
    viewport: Option<Viewport>,
    cache_capacity: usize,
) -> Result<DecodeEvidence, String> {
    let catalog = SpriteCatalog::load(asset_zip, assets_dir)?;
    let fixture = select_fixture_sprites(fixture_dir, viewport)?;
    let mut decoder = SpriteDecoder::new(catalog, cache_capacity)?;
    let mut geometry_counts = BTreeMap::new();
    let mut id_hasher = Sha256::new();
    let mut rgba_root = Sha256::new();
    rgba_root.update(b"OTERYN-REAL-CONTENT-SPRITE-RGBA-ROOT-V1\0");

    for sprite_id in &fixture.sprite_ids {
        let sprite = decoder.decode_sprite(*sprite_id)?;
        *geometry_counts
            .entry(format!("{}x{}", sprite.width, sprite.height))
            .or_insert(0) += 1;
        id_hasher.update(sprite_id.to_be_bytes());
        rgba_root.update(sprite_id.to_be_bytes());
        rgba_root.update(sprite.width.to_be_bytes());
        rgba_root.update(sprite.height.to_be_bytes());
        let sprite_digest = Sha256::digest(&sprite.rgba);
        rgba_root.update(sprite_digest);
    }

    Ok(DecodeEvidence {
        schema: "oteryn-world-vfx-real-content-decode-evidence-v1",
        source: decoder.source_identity().clone(),
        fixture,
        cache: decoder.stats().clone(),
        resident_sheets: decoder.resident_sheets(),
        geometry_counts,
        sprite_id_set_sha256: hex::encode(id_hasher.finalize()),
        sprite_rgba_root_sha256: hex::encode(rgba_root.finalize()),
    })
}

fn value_at<'a>(root: &'a serde_json::Value, path: &[&str]) -> Result<&'a serde_json::Value, String> {
    let mut current = root;
    for key in path {
        current = current
            .get(*key)
            .ok_or_else(|| format!("missing JSON field {}", path.join(".")))?;
    }
    Ok(current)
}

fn value_str<'a>(root: &'a serde_json::Value, path: &[&str]) -> Result<&'a str, String> {
    value_at(root, path)?
        .as_str()
        .ok_or_else(|| format!("JSON field {} must be a string", path.join(".")))
}

fn value_i64(root: &serde_json::Value, path: &[&str]) -> Result<i64, String> {
    value_at(root, path)?
        .as_i64()
        .ok_or_else(|| format!("JSON field {} must be an integer", path.join(".")))
}

fn require_u64(root: &serde_json::Value, path: &[&str], expected: u64) -> Result<(), String> {
    let actual = value_at(root, path)?
        .as_u64()
        .ok_or_else(|| format!("JSON field {} must be an unsigned integer", path.join(".")))?;
    if actual != expected {
        return Err(format!(
            "JSON field {} mismatch: expected {expected}, got {actual}",
            path.join(".")
        ));
    }
    Ok(())
}

fn read_bounded_file(path: &Path, max_bytes: u64, label: &str) -> Result<Vec<u8>, String> {
    let mut file = File::open(path).map_err(|error| format!("open {label} {}: {error}", path.display()))?;
    let size = file
        .metadata()
        .map_err(|error| format!("stat {label} {}: {error}", path.display()))?
        .len();
    if size > max_bytes {
        return Err(format!(
            "{label} {} exceeds bounded size: {size} > {max_bytes}",
            path.display()
        ));
    }
    let mut bytes = Vec::with_capacity(size as usize);
    file.read_to_end(&mut bytes)
        .map_err(|error| format!("read {label} {}: {error}", path.display()))?;
    Ok(bytes)
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|error| format!("open {}: {error}", path.display()))?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| format!("read {}: {error}", path.display()))?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(hex::encode(digest.finalize()))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn read_u16_le(data: &[u8], offset: usize) -> Result<u16, String> {
    let bytes: [u8; 2] = data
        .get(offset..offset + 2)
        .ok_or_else(|| format!("BMP u16 field at {offset} is truncated"))?
        .try_into()
        .map_err(|_| "BMP u16 conversion failed".to_owned())?;
    Ok(u16::from_le_bytes(bytes))
}

fn read_u32_le(data: &[u8], offset: usize) -> Result<u32, String> {
    let bytes: [u8; 4] = data
        .get(offset..offset + 4)
        .ok_or_else(|| format!("BMP u32 field at {offset} is truncated"))?
        .try_into()
        .map_err(|_| "BMP u32 conversion failed".to_owned())?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_i32_le(data: &[u8], offset: usize) -> Result<i32, String> {
    let bytes: [u8; 4] = data
        .get(offset..offset + 4)
        .ok_or_else(|| format!("BMP i32 field at {offset} is truncated"))?
        .try_into()
        .map_err(|_| "BMP i32 conversion failed".to_owned())?;
    Ok(i32::from_le_bytes(bytes))
}

struct BoundedWriter {
    bytes: Vec<u8>,
    limit: usize,
}

impl BoundedWriter {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            limit,
        }
    }

    fn into_inner(self) -> Vec<u8> {
        self.bytes
    }
}

impl Write for BoundedWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if self.bytes.len().saturating_add(buffer.len()) > self.limit {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "decompressed sprite sheet exceeds bounded output size",
            ));
        }
        self.bytes.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprite_layouts_match_384_sheet_capacity() {
        let cases = [
            (SpriteLayout::Size32x32, (32, 32), 144),
            (SpriteLayout::Size32x64, (32, 64), 72),
            (SpriteLayout::Size64x32, (64, 32), 72),
            (SpriteLayout::Size64x64, (64, 64), 36),
        ];
        for (layout, dimensions, capacity) in cases {
            assert_eq!(layout.dimensions(), dimensions);
            assert_eq!(layout.capacity(), capacity);
        }
    }

    #[test]
    fn sprite_extraction_is_row_major_from_first_id() -> Result<(), String> {
        let descriptor = SpriteSheetDescriptor {
            first_sprite_id: 1_000,
            last_sprite_id: 1_035,
            layout: SpriteLayout::Size64x64,
            file: "sprites-test.bmp.lzma".to_owned(),
        };
        let mut sheet = DecodedSheet {
            rgba: vec![0; SHEET_SIZE as usize * SHEET_SIZE as usize * 4],
        };
        for cell in 0..descriptor.layout.capacity() {
            let columns = SHEET_SIZE / 64;
            let row = cell / columns;
            let column = cell % columns;
            let offset = ((row * 64 * SHEET_SIZE + column * 64) * 4) as usize;
            sheet.rgba[offset] = cell as u8;
            sheet.rgba[offset + 3] = 255;
        }
        let sprite = extract_sprite(&sheet, &descriptor, 1_007)?;
        assert_eq!((sprite.width, sprite.height), (64, 64));
        assert_eq!(sprite.rgba[0], 7);
        assert_eq!(sprite.rgba[3], 255);
        Ok(())
    }

    #[test]
    fn bmp_parser_flips_bottom_up_and_keys_magenta() -> Result<(), String> {
        let row_bytes = SHEET_SIZE as usize * 4;
        let pixel_offset = 54_usize;
        let mut bmp = vec![0_u8; pixel_offset + row_bytes * SHEET_SIZE as usize];
        bmp[0..2].copy_from_slice(b"BM");
        bmp[10..14].copy_from_slice(&(pixel_offset as u32).to_le_bytes());
        bmp[14..18].copy_from_slice(&40_u32.to_le_bytes());
        bmp[18..22].copy_from_slice(&(SHEET_SIZE as i32).to_le_bytes());
        bmp[22..26].copy_from_slice(&(SHEET_SIZE as i32).to_le_bytes());
        bmp[26..28].copy_from_slice(&1_u16.to_le_bytes());
        bmp[28..30].copy_from_slice(&32_u16.to_le_bytes());
        bmp[30..34].copy_from_slice(&0_u32.to_le_bytes());
        for pixel in bmp[pixel_offset..].chunks_exact_mut(4) {
            pixel.copy_from_slice(&[0, 0, 0, 255]);
        }

        let source_top_row = SHEET_SIZE as usize - 1;
        let magenta = pixel_offset + source_top_row * row_bytes;
        bmp[magenta..magenta + 4].copy_from_slice(&[255, 0, 255, 255]);
        let colored = magenta + 4;
        bmp[colored..colored + 4].copy_from_slice(&[30, 20, 10, 255]);

        let rgba = parse_bmp_rgba(&bmp)?;
        assert_eq!(&rgba[0..4], &[0, 0, 0, 0]);
        assert_eq!(&rgba[4..8], &[10, 20, 30, 255]);
        Ok(())
    }

    #[test]
    fn bounded_writer_rejects_overflow() {
        let mut writer = BoundedWriter::new(4);
        assert_eq!(writer.write(&[1, 2, 3, 4]).ok(), Some(4));
        assert!(writer.write(&[5]).is_err());
    }
}
