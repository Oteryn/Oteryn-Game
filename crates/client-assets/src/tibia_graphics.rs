//! Tibia's own game font and creature HUD flag artwork, read at runtime from the official
//! client's `graphics_resources.rcc` (a Qt resource bundle) placed in the asset directory.
//!
//! Only the entries named below are read, each checked against its pinned `sha256`; the bundle
//! itself is never committed or cached on disk. A missing bundle, a missing entry or a
//! mismatch is an error and the caller keeps its builtin glyphs.

use crate::error::AssetError;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::{Cursor, Read};
use std::path::Path;

/// File name of the bundle inside the asset directory, as the official client ships it.
pub const GRAPHICS_FILE: &str = "graphics_resources.rcc";
/// The 15.x bundle is 6,785,684 bytes.
pub const MAX_GRAPHICS_BYTES: u64 = 16 * 1024 * 1024;
/// Edge of one creature HUD flag icon in pixels.
pub const ICON_PX: usize = 11;
/// Skull icons, wire values `1..=6` (yellow, green, white, red, black, orange).
pub const SKULL_ICONS: usize = 6;
/// Party shield icons, wire values `1..=11`.
pub const SHIELD_ICONS: usize = 11;

const FONT_METRICS: &str = "/fonts/Verdana10px.fnt";
const FONT_PAGE: &str = "/fonts/Verdana10px.png";
const SKULLS: &str = "/images/creatures/hud/flags/playerkiller.png";
const SHIELDS: &str = "/images/creatures/hud/flags/party.png";
const MAX_ENTRY_BYTES: usize = 1024 * 1024;
const MAX_GLYPH_PX: u32 = 32;

/// The `sha256` of every bundle entry read, over its decompressed bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphicsPins {
    pub font_metrics: String,
    pub font_page: String,
    pub skulls: String,
    pub shields: String,
}

impl Default for GraphicsPins {
    /// The entries of the official 15.x client bundle.
    fn default() -> Self {
        Self {
            font_metrics: "7798333509369c7745db9144981e0242922007c30dd6dd04791d663f1dd372cb".into(),
            font_page: "4a9f6c1db76b4403428a08c03ee81440f42a780e0126b3086b47890e9bc2d2d7".into(),
            skulls: "9815b59a8203eeafb13a1dce0bd746adabca9c490f0ff3d32ee1bb50e10be4ce".into(),
            shields: "62a936b1bbebf64a1f3cb20afa97a5a5f658fbc3cfd6d623d6ad830ae2788de7".into(),
        }
    }
}

/// One outlined glyph as RGBA: a grey fill inside a black outline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Glyph {
    pub width: u8,
    pub height: u8,
    pub x_offset: i8,
    pub y_offset: i8,
    pub advance: u8,
    pub rgba: Vec<u8>,
}

impl Glyph {
    /// The pixel at `(x, y)`, `None` where the glyph is transparent.
    #[must_use]
    pub fn pixel(&self, x: usize, y: usize) -> Option<[u8; 4]> {
        if x >= usize::from(self.width) || y >= usize::from(self.height) {
            return None;
        }
        let at = (y * usize::from(self.width) + x) * 4;
        let pixel: [u8; 4] = self.rgba.get(at..at + 4)?.try_into().ok()?;
        (pixel[3] != 0).then_some(pixel)
    }

    /// The fill (grey in the bundle) painted `color`; the black outline stays black.
    #[must_use]
    pub const fn tint(pixel: [u8; 4], color: [u8; 3]) -> [u8; 4] {
        if pixel[0] == 0 {
            [0, 0, 0, pixel[3]]
        } else {
            [color[0], color[1], color[2], pixel[3]]
        }
    }
}

/// The bitmap font the official client draws its game text with (AngelCode BMFont).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameFont {
    line_height: u8,
    glyphs: BTreeMap<char, Glyph>,
}

impl GameFont {
    #[must_use]
    pub const fn line_height(&self) -> u8 {
        self.line_height
    }

    /// The glyph for `character`, `None` where the font has none.
    #[must_use]
    pub fn glyph(&self, character: char) -> Option<&Glyph> {
        self.glyphs.get(&character)
    }
}

/// The verified game font and creature HUD flags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TibiaGraphics {
    font: GameFont,
    skulls: Vec<Vec<u8>>,
    shields: Vec<Vec<u8>>,
}

impl TibiaGraphics {
    /// Reads [`GRAPHICS_FILE`] from `dir` against the official entry pins.
    pub fn load(dir: &Path) -> Result<Self, AssetError> {
        Self::load_pinned(dir, &GraphicsPins::default())
    }

    /// Reads [`GRAPHICS_FILE`] from `dir`, refusing any entry that does not match `pins`.
    pub fn load_pinned(dir: &Path, pins: &GraphicsPins) -> Result<Self, AssetError> {
        let path = dir.join(GRAPHICS_FILE);
        let io_error = |error: std::io::Error| AssetError::Io {
            file: GRAPHICS_FILE.to_owned(),
            message: error.to_string(),
        };
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .map_err(io_error)?
            .take(MAX_GRAPHICS_BYTES.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(io_error)?;
        if bytes.len() as u64 > MAX_GRAPHICS_BYTES {
            return Err(AssetError::TooLarge {
                file: GRAPHICS_FILE.to_owned(),
                limit: MAX_GRAPHICS_BYTES,
            });
        }
        let bundle = Bundle::parse(&bytes)?;
        let font = parse_font(
            &bundle.entry(FONT_METRICS, &pins.font_metrics)?,
            &decode_png(FONT_PAGE, &bundle.entry(FONT_PAGE, &pins.font_page)?)?,
        )?;
        Ok(Self {
            font,
            skulls: icons(SKULLS, &bundle.entry(SKULLS, &pins.skulls)?, SKULL_ICONS)?,
            shields: icons(
                SHIELDS,
                &bundle.entry(SHIELDS, &pins.shields)?,
                SHIELD_ICONS,
            )?,
        })
    }

    #[must_use]
    pub const fn font(&self) -> &GameFont {
        &self.font
    }

    /// The [`ICON_PX`] square RGBA skull for wire value `1..=6`.
    #[must_use]
    pub fn skull(&self, value: u8) -> Option<&[u8]> {
        self.skulls
            .get(usize::from(value).checked_sub(1)?)
            .map(Vec::as_slice)
    }

    /// The [`ICON_PX`] square RGBA party shield for wire value `1..=11`.
    #[must_use]
    pub fn shield(&self, value: u8) -> Option<&[u8]> {
        self.shields
            .get(usize::from(value).checked_sub(1)?)
            .map(Vec::as_slice)
    }
}

fn malformed(file: &str, reason: impl Into<String>) -> AssetError {
    AssetError::Malformed {
        file: file.to_owned(),
        reason: reason.into(),
    }
}

/// A parsed Qt `rcc` bundle (format versions 1 to 3).
struct Bundle<'a> {
    bytes: &'a [u8],
    tree: usize,
    data: usize,
    names: usize,
    node_bytes: usize,
}

impl<'a> Bundle<'a> {
    fn parse(bytes: &'a [u8]) -> Result<Self, AssetError> {
        if bytes.get(..4) != Some(b"qres") {
            return Err(malformed(GRAPHICS_FILE, "not a Qt resource bundle"));
        }
        let word =
            |at: usize| be_u32(bytes, at).ok_or_else(|| malformed(GRAPHICS_FILE, "short header"));
        let version = word(4)?;
        if !(1..=3).contains(&version) {
            return Err(malformed(
                GRAPHICS_FILE,
                format!("format version {version}"),
            ));
        }
        Ok(Self {
            bytes,
            tree: word(8)? as usize,
            data: word(12)? as usize,
            names: word(16)? as usize,
            node_bytes: if version == 1 { 14 } else { 22 },
        })
    }

    fn node(&self, index: usize) -> Option<&'a [u8]> {
        let at = self.tree.checked_add(index.checked_mul(self.node_bytes)?)?;
        self.bytes.get(at..at.checked_add(self.node_bytes)?)
    }

    fn name_is(&self, offset: u32, expected: &str) -> bool {
        let Some(at) = self.names.checked_add(offset as usize) else {
            return false;
        };
        let Some(length) = be_u16(self.bytes, at) else {
            return false;
        };
        let start = at + 6;
        let Some(raw) = self.bytes.get(start..start + usize::from(length) * 2) else {
            return false;
        };
        let units = raw
            .as_chunks::<2>()
            .0
            .iter()
            .map(|unit| u16::from_be_bytes(*unit));
        char::decode_utf16(units)
            .map(|unit| unit.ok())
            .eq(expected.chars().map(Some))
    }

    /// The decompressed bytes of the file at `path`, checked against `sha256`.
    fn entry(&self, path: &str, sha256: &str) -> Result<Vec<u8>, AssetError> {
        let missing = || AssetError::NotInManifest {
            file: path.to_owned(),
        };
        let mut node = self.node(0).ok_or_else(missing)?;
        for part in path.split('/').filter(|part| !part.is_empty()) {
            let flags = be_u16(node, 4).ok_or_else(missing)?;
            if flags & 2 == 0 {
                return Err(missing());
            }
            let count = be_u32(node, 6).ok_or_else(missing)? as usize;
            let first = be_u32(node, 10).ok_or_else(missing)? as usize;
            if count > self.bytes.len() / self.node_bytes {
                return Err(malformed(GRAPHICS_FILE, "directory larger than the bundle"));
            }
            node = (first..first.saturating_add(count))
                .filter_map(|index| self.node(index))
                .find(|child| be_u32(child, 0).is_some_and(|name| self.name_is(name, part)))
                .ok_or_else(missing)?;
        }
        let flags = be_u16(node, 4).ok_or_else(missing)?;
        let offset = be_u32(node, 10).ok_or_else(missing)? as usize;
        let at = self.data.checked_add(offset).ok_or_else(missing)?;
        let size = be_u32(self.bytes, at).ok_or_else(missing)? as usize;
        if flags & 2 != 0 || size > MAX_ENTRY_BYTES {
            return Err(malformed(path, "not a bounded file entry"));
        }
        let raw = self.bytes.get(at + 4..at + 4 + size).ok_or_else(missing)?;
        let bytes = match flags & 5 {
            0 => raw.to_vec(),
            // qCompress: a big-endian length, then a zlib stream.
            1 => {
                let stream = raw.get(4..).ok_or_else(missing)?;
                let mut bytes = Vec::new();
                flate2::read::ZlibDecoder::new(stream)
                    .take(MAX_ENTRY_BYTES as u64 + 1)
                    .read_to_end(&mut bytes)
                    .map_err(|error| malformed(path, error.to_string()))?;
                if bytes.len() > MAX_ENTRY_BYTES {
                    return Err(malformed(path, "inflates past its bound"));
                }
                bytes
            }
            _ => return Err(malformed(path, "unsupported compression")),
        };
        let digest: String = Sha256::digest(&bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        if !digest.eq_ignore_ascii_case(sha256) {
            return Err(AssetError::HashMismatch {
                file: path.to_owned(),
            });
        }
        Ok(bytes)
    }
}

fn be_u16(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes(
        bytes.get(at..at.checked_add(2)?)?.try_into().ok()?,
    ))
}

fn be_u32(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(
        bytes.get(at..at.checked_add(4)?)?.try_into().ok()?,
    ))
}

/// A decoded RGBA image.
struct Image {
    width: usize,
    height: usize,
    rgba: Vec<u8>,
}

fn decode_png(file: &str, bytes: &[u8]) -> Result<Image, AssetError> {
    let fail = |error: png::DecodingError| malformed(file, error.to_string());
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::ALPHA | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(fail)?;
    let (width, height) = (reader.info().width, reader.info().height);
    if width > 1024 || height > 1024 {
        return Err(malformed(file, "image larger than 1024 pixels"));
    }
    let mut buffer = vec![
        0;
        reader
            .output_buffer_size()
            .ok_or_else(|| malformed(file, "size"))?
    ];
    let frame = reader.next_frame(&mut buffer).map_err(fail)?;
    buffer.truncate(frame.buffer_size());
    let rgba = match frame.color_type {
        png::ColorType::Rgba => buffer,
        png::ColorType::GrayscaleAlpha => buffer
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|&[grey, alpha]| [grey, grey, grey, alpha])
            .collect(),
        other => return Err(malformed(file, format!("unexpected colour type {other:?}"))),
    };
    Ok(Image {
        width: width as usize,
        height: height as usize,
        rgba,
    })
}

/// Splits a one-row strip of `count` square icons.
fn icons(file: &str, bytes: &[u8], count: usize) -> Result<Vec<Vec<u8>>, AssetError> {
    let image = decode_png(file, bytes)?;
    if image.height != ICON_PX || image.width != ICON_PX * count {
        return Err(malformed(file, "unexpected icon strip size"));
    }
    Ok((0..count)
        .map(|icon| {
            (0..ICON_PX)
                .flat_map(|y| {
                    let at = (y * image.width + icon * ICON_PX) * 4;
                    image.rgba[at..at + ICON_PX * 4].iter().copied()
                })
                .collect()
        })
        .collect())
}

/// Parses the text form of an AngelCode BMFont with one page.
fn parse_font(metrics: &[u8], page: &Image) -> Result<GameFont, AssetError> {
    let fail = |reason: &str| malformed(FONT_METRICS, reason);
    let text = std::str::from_utf8(metrics).map_err(|_| fail("not UTF-8"))?;
    let mut line_height = None;
    let mut glyphs = BTreeMap::new();
    for line in text.lines() {
        let mut words = line.split_whitespace();
        let tag = words.next();
        let fields: BTreeMap<&str, &str> = words.filter_map(|word| word.split_once('=')).collect();
        let number = |key: &str| -> Result<i32, AssetError> {
            fields
                .get(key)
                .and_then(|value| value.parse().ok())
                .ok_or_else(|| fail("missing numeric field"))
        };
        match tag {
            Some("common") => {
                if number("pages")? != 1 {
                    return Err(fail("more than one page"));
                }
                line_height = Some(u8::try_from(number("lineHeight")?).map_err(|_| fail("line"))?);
            }
            Some("page") if fields.get("file") != Some(&"\"/fonts/Verdana10px.png\"") => {
                return Err(fail("unexpected page file"));
            }
            Some("char") => {
                let character = u32::try_from(number("id")?)
                    .ok()
                    .and_then(char::from_u32)
                    .ok_or_else(|| fail("character id"))?;
                let [x, y, width, height] = [
                    number("x")?,
                    number("y")?,
                    number("width")?,
                    number("height")?,
                ]
                .map(|value| u32::try_from(value).unwrap_or(u32::MAX));
                if number("page")? != 0
                    || width > MAX_GLYPH_PX
                    || height > MAX_GLYPH_PX
                    || x.saturating_add(width) as usize > page.width
                    || y.saturating_add(height) as usize > page.height
                {
                    return Err(fail("glyph outside the page"));
                }
                let small = |key: &str| -> Result<i8, AssetError> {
                    i8::try_from(number(key)?).map_err(|_| fail("glyph offset"))
                };
                let rgba = (y..y + height)
                    .flat_map(|row| {
                        let at = (row as usize * page.width + x as usize) * 4;
                        page.rgba[at..at + width as usize * 4].iter().copied()
                    })
                    .collect();
                glyphs.insert(
                    character,
                    Glyph {
                        width: width as u8,
                        height: height as u8,
                        x_offset: small("xoffset")?,
                        y_offset: small("yoffset")?,
                        advance: u8::try_from(number("xadvance")?)
                            .map_err(|_| fail("glyph advance"))?,
                        rgba,
                    },
                );
            }
            _ => {}
        }
    }
    let line_height = line_height.ok_or_else(|| fail("no common line"))?;
    if glyphs.is_empty() {
        return Err(fail("no glyphs"));
    }
    Ok(GameFont {
        line_height,
        glyphs,
    })
}

#[cfg(any(test, feature = "test-support"))]
pub mod fixture;

#[cfg(test)]
#[path = "tibia_graphics_tests.rs"]
mod tests;
