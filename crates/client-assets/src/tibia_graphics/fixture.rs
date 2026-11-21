//! A generated bundle with the layout of the official one, for tests that cannot ship the
//! original artwork: an indexed 2-bit outlined font page, a zlib-compressed font description
//! and RGBA flag strips, in a format-3 Qt resource bundle.

use super::{GRAPHICS_FILE, GUILD_ICONS, GraphicsPins, ICON_PX, SHIELD_ICONS, SKULL_ICONS};
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::Path;

/// Characters the generated font covers; everything else is absent from it.
pub const CHARACTERS: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz -";
/// Every generated glyph's advance and outlined size.
pub const ADVANCE: u8 = 6;
pub const GLYPH_WIDTH: u8 = 7;
pub const GLYPH_HEIGHT: u8 = 9;
/// Every generated glyph's offset from the pen.
pub const X_OFFSET: i8 = -1;
pub const Y_OFFSET: i8 = 2;
const PAGE_WIDTH: usize = 128;
const PAGE_HEIGHT: usize = 64;

/// The RGBA colour of generated icon `index` (zero-based) of a strip.
#[must_use]
pub fn icon_color(skull: bool, index: usize) -> [u8; 4] {
    let base = if skull { 20 } else { 120 };
    [
        base + index as u8 * 9,
        200 - index as u8 * 7,
        33 + index as u8 * 3,
        255,
    ]
}

/// The RGBA colour of generated guild emblem `index` (zero-based).
#[must_use]
pub fn guild_color(index: usize) -> [u8; 4] {
    [
        60 + index as u8 * 11,
        30 + index as u8 * 13,
        210 - index as u8 * 5,
        255,
    ]
}

/// Writes the generated bundle into `dir` and returns its entry pins.
pub fn write(dir: &Path) -> std::io::Result<GraphicsPins> {
    let (bytes, pins) = bundle()?;
    std::fs::write(dir.join(GRAPHICS_FILE), bytes)?;
    Ok(pins)
}

/// The generated bundle and its entry pins.
pub fn bundle() -> std::io::Result<(Vec<u8>, GraphicsPins)> {
    let metrics = font_metrics();
    let page = font_page()?;
    let skulls = strip(|index| icon_color(true, index), SKULL_ICONS)?;
    let shields = strip(|index| icon_color(false, index), SHIELD_ICONS)?;
    let guilds = strip(guild_color, GUILD_ICONS)?;
    let pins = GraphicsPins {
        font_metrics: sha256(metrics.as_bytes()),
        font_page: sha256(&page),
        skulls: sha256(&skulls),
        shields: sha256(&shields),
        guilds: sha256(&guilds),
    };
    let mut compressed = (metrics.len() as u32).to_be_bytes().to_vec();
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(metrics.as_bytes())?;
    compressed.extend(encoder.finish()?);
    let files: [(&'static str, u16, Vec<u8>); 5] = [
        ("Verdana10px.fnt", 1, compressed),
        ("Verdana10px.png", 0, page),
        ("playerkiller.png", 0, skulls),
        ("party.png", 0, shields),
        ("guild.png", 0, guilds),
    ];
    let [
        font_metrics,
        font_page,
        skull_strip,
        shield_strip,
        guild_strip,
    ] = files;
    // Breadth-first nodes: 0 root, 1 fonts, 2 images, 3-4 font files, 5 creatures, 6 hud,
    // 7 flags, 8-10 flag strips. A directory names its first child and child count.
    let nodes = [
        Node::Directory("", 1, 2),
        Node::Directory("fonts", 3, 2),
        Node::Directory("images", 5, 1),
        Node::File(font_metrics),
        Node::File(font_page),
        Node::Directory("creatures", 6, 1),
        Node::Directory("hud", 7, 1),
        Node::Directory("flags", 8, 3),
        Node::File(skull_strip),
        Node::File(shield_strip),
        Node::File(guild_strip),
    ];
    let (mut tree, mut data, mut names) = (Vec::new(), Vec::new(), Vec::new());
    for node in nodes {
        let label = match &node {
            Node::Directory(label, ..) | Node::File((label, ..)) => *label,
        };
        let units: Vec<u16> = label.encode_utf16().collect();
        tree.extend((names.len() as u32).to_be_bytes());
        names.extend((units.len() as u16).to_be_bytes());
        names.extend(0_u32.to_be_bytes());
        names.extend(units.iter().flat_map(|unit| unit.to_be_bytes()));
        match node {
            Node::Directory(_, first, count) => {
                tree.extend(2_u16.to_be_bytes());
                tree.extend(count.to_be_bytes());
                tree.extend(first.to_be_bytes());
            }
            Node::File((_, flags, payload)) => {
                tree.extend(flags.to_be_bytes());
                tree.extend([0_u8; 4]);
                tree.extend((data.len() as u32).to_be_bytes());
                data.extend((payload.len() as u32).to_be_bytes());
                data.extend(payload);
            }
        }
        tree.extend(0_u64.to_be_bytes());
    }
    let header = 24_u32;
    let tree_at = header;
    let data_at = tree_at + tree.len() as u32;
    let names_at = data_at + data.len() as u32;
    let mut bytes = b"qres".to_vec();
    for word in [3, tree_at, data_at, names_at, 0] {
        bytes.extend(word.to_be_bytes());
    }
    bytes.extend(tree);
    bytes.extend(data);
    bytes.extend(names);
    Ok((bytes, pins))
}

enum Node {
    Directory(&'static str, u32, u32),
    File((&'static str, u16, Vec<u8>)),
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn glyph_origin(index: usize) -> (usize, usize) {
    let per_row = PAGE_WIDTH / usize::from(GLYPH_WIDTH);
    (
        index % per_row * usize::from(GLYPH_WIDTH),
        index / per_row * usize::from(GLYPH_HEIGHT),
    )
}

fn font_metrics() -> String {
    let mut text = format!(
        "info face=\"Verdana\" size=12 bold=1\r\ncommon lineHeight=12 base=10 scaleW={PAGE_WIDTH} scaleH={PAGE_HEIGHT} pages=1\r\npage id=0 file=\"/fonts/Verdana10px.png\"\r\nchars count={}\r\n",
        CHARACTERS.chars().count()
    );
    for (index, character) in CHARACTERS.chars().enumerate() {
        let (x, y) = glyph_origin(index);
        text.push_str(&format!(
            "char id={} x={x} y={y} width={GLYPH_WIDTH} height={GLYPH_HEIGHT} xoffset={X_OFFSET} yoffset={Y_OFFSET} xadvance={ADVANCE} page=0 chnl=15\r\n",
            u32::from(character)
        ));
    }
    text
}

/// The palette index of generated glyph pixel `(x, y)`: 0 clear, 1 outline, 2 grey, 3 white.
#[must_use]
pub fn glyph_index(character: char, x: usize, y: usize) -> u8 {
    let (width, height) = (usize::from(GLYPH_WIDTH), usize::from(GLYPH_HEIGHT));
    if x == 0 || y == 0 || x + 1 == width || y + 1 == height {
        return u8::from((x + y).is_multiple_of(2));
    }
    if (x * 7 + y * 3 + u32::from(character) as usize).is_multiple_of(5) {
        2
    } else {
        3
    }
}

fn font_page() -> std::io::Result<Vec<u8>> {
    let mut indices = vec![0_u8; PAGE_WIDTH * PAGE_HEIGHT];
    for (index, character) in CHARACTERS.chars().enumerate() {
        let (left, top) = glyph_origin(index);
        for y in 0..usize::from(GLYPH_HEIGHT) {
            for x in 0..usize::from(GLYPH_WIDTH) {
                indices[(top + y) * PAGE_WIDTH + left + x] = glyph_index(character, x, y);
            }
        }
    }
    let packed: Vec<u8> = indices
        .as_chunks::<4>()
        .0
        .iter()
        .map(|&[a, b, c, d]| a << 6 | b << 4 | c << 2 | d)
        .collect();
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, PAGE_WIDTH as u32, PAGE_HEIGHT as u32);
    encoder.set_color(png::ColorType::Indexed);
    encoder.set_depth(png::BitDepth::Two);
    encoder.set_palette(vec![0, 0, 0, 0, 0, 0, 192, 192, 192, 255, 255, 255]);
    encoder.set_trns(vec![0]);
    let mut writer = encoder.write_header().map_err(std::io::Error::other)?;
    writer
        .write_image_data(&packed)
        .map_err(std::io::Error::other)?;
    writer.finish().map_err(std::io::Error::other)?;
    Ok(out)
}

fn strip(color: fn(usize) -> [u8; 4], count: usize) -> std::io::Result<Vec<u8>> {
    let width = ICON_PX * count;
    let mut rgba = vec![0_u8; width * ICON_PX * 4];
    for y in 0..ICON_PX {
        for x in 0..width {
            // Corners stay clear, as in the round official icons.
            let (column, row) = (x % ICON_PX, y);
            if (column == 0 || column + 1 == ICON_PX) && (row == 0 || row + 1 == ICON_PX) {
                continue;
            }
            let at = (y * width + x) * 4;
            rgba[at..at + 4].copy_from_slice(&color(x / ICON_PX));
        }
    }
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, width as u32, ICON_PX as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(std::io::Error::other)?;
    writer
        .write_image_data(&rgba)
        .map_err(std::io::Error::other)?;
    writer.finish().map_err(std::io::Error::other)?;
    Ok(out)
}
