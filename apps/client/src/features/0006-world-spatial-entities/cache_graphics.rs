// Tibia's game font and creature HUD flags from the official graphics bundle in the asset
// root; the builtin glyphs remain where the bundle or one of its characters is missing.
use oteryn_client_assets::tibia_graphics::{
    GUILD_ICONS, Glyph, ICON_PX, SHIELD_ICONS, SKULL_ICONS, TibiaGraphics,
};

/// One name glyph: its atlas cell, offset and size in label pixels at 100 %.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LabelGlyph {
    pub(crate) cell: u16,
    pub(crate) at: [f32; 2],
    pub(crate) size: [f32; 2],
}

/// A laid-out name: bundle glyphs advance by the font's metrics, builtin glyphs by 8 pixels.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct NameLabel {
    pub(crate) glyphs: Vec<LabelGlyph>,
    pub(crate) width: f32,
}

impl NameLabel {
    fn push(&mut self, cell: u16, glyph: Option<&Glyph>) {
        let (at, size, advance) = glyph.map_or(([self.width, 0.0], [8.0, 12.0], 8.0), |glyph| {
            (
                [
                    self.width + f32::from(glyph.x_offset),
                    f32::from(glyph.y_offset),
                ],
                [32.0; 2],
                f32::from(glyph.advance),
            )
        });
        self.glyphs.push(LabelGlyph { cell, at, size });
        self.width += advance;
    }
}

/// Atlas cells of the official skull, party shield and guild emblem icons, by wire value
/// minus one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HudIconCells {
    pub(crate) skulls: [u16; SKULL_ICONS],
    pub(crate) shields: [u16; SHIELD_ICONS],
    pub(crate) guilds: [u16; GUILD_ICONS],
}

/// A glyph painted `colour` at the top left of an otherwise clear cell.
fn glyph_cell(glyph: &Glyph, colour: [u8; 3]) -> Vec<u8> {
    let mut pixels = vec![0; CELL_RGBA_BYTES];
    for y in 0..usize::from(glyph.height) {
        for x in 0..usize::from(glyph.width) {
            if let Some(pixel) = glyph.pixel(x, y) {
                let at = (y * 32 + x) * 4;
                pixels[at..at + 4].copy_from_slice(&Glyph::tint(pixel, colour));
            }
        }
    }
    pixels
}

impl PresentationCache {
    fn append_icons(&mut self) -> Option<HudIconCells> {
        let graphics = self.graphics.clone()?;
        let mut cell = |icon: Option<&[u8]>| {
            let mut pixels = vec![0; CELL_RGBA_BYTES];
            for (row, line) in icon?.as_chunks::<{ ICON_PX * 4 }>().0.iter().enumerate() {
                pixels[row * 32 * 4..row * 32 * 4 + line.len()].copy_from_slice(line);
            }
            self.append(&pixels)
        };
        let mut skulls = [0; SKULL_ICONS];
        for (index, slot) in skulls.iter_mut().enumerate() {
            *slot = cell(graphics.skull(index as u8 + 1))?;
        }
        let mut shields = [0; SHIELD_ICONS];
        for (index, slot) in shields.iter_mut().enumerate() {
            *slot = cell(graphics.shield(index as u8 + 1))?;
        }
        let mut guilds = [0; GUILD_ICONS];
        for (index, slot) in guilds.iter_mut().enumerate() {
            *slot = cell(graphics.guild(index as u8 + 1))?;
        }
        Some(HudIconCells {
            skulls,
            shields,
            guilds,
        })
    }

    /// The bundle font and the icon cells of the current atlas, when the bundle loaded.
    pub(super) fn hud_graphics(&self) -> (Option<Arc<TibiaGraphics>>, Option<HudIconCells>) {
        (self.graphics.clone(), self.icon_cells)
    }
}

