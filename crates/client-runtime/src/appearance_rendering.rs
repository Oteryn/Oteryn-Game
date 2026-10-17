//! Unregistered APP-4 native asset consumer: static first frame group, phase 0, z=0.
//! Selection/unlocks/sex/fallback and observed actor state stay with their owners.
use oteryn_client_assets::{
    AppearanceIndex, AssetError, AssetStore, CELL_PX, Catalog, MAX_ENTRY_CELLS, SpriteSheets,
};
use oteryn_renderer::{AtlasImage, BatchError};
use std::path::Path;
use std::sync::Arc;

/// CONTENT_APPEARANCE §4.2: palette VALUES come from content, not this renderer.
const PALETTE_LEN: usize = 133;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OutfitDirection {
    North,
    East,
    South,
    West,
}
impl OutfitDirection {
    fn pattern(self, width: u32) -> Result<u32, AppearanceRenderError> {
        match (width, self) {
            (1, Self::South) => Ok(0),
            (4.., Self::North) => Ok(0),
            (4.., Self::East) => Ok(1),
            (4.., Self::South) => Ok(2),
            (4.., Self::West) => Ok(3),
            _ => Err(AppearanceRenderError::UnsupportedGrammar),
        }
    }
}

/// Already resolved presentation DATA; neither a server selection nor an unlock grant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OutfitPixels {
    pub look_type: u32,
    pub direction: OutfitDirection,
    pub colours: [u16; 4],
    pub addons: u8,
}

#[derive(Debug)]
pub enum AppearanceRenderError {
    Asset(AssetError),
    Atlas(BatchError),
    UnknownLook,
    InvalidPalette,
    UnsupportedGrammar,
    InvalidAddons,
    EmptyOutfit,
}
impl From<AssetError> for AppearanceRenderError {
    fn from(value: AssetError) -> Self {
        Self::Asset(value)
    }
}
impl From<BatchError> for AppearanceRenderError {
    fn from(value: BatchError) -> Self {
        Self::Atlas(value)
    }
}
impl std::fmt::Display for AppearanceRenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "outfit presentation: {self:?}")
    }
}
impl std::error::Error for AppearanceRenderError {}

/// Identity of this static first-group/phase0/z0 composition. The asset pin and resolved
/// RGB values prevent equal look/colour indices from aliasing across source or palette changes.
/// Scene adoption must namespace this key separately from raw sprite and builtin keys, and
/// pair it with the cell coordinates. It does not authorize a selected or observed appearance.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OutfitVariantKey {
    asset_manifest_sha256: Arc<str>,
    look: OutfitPixels,
    colours_rgb: [[u8; 3]; 4],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposedOutfit {
    pub variant_key: OutfitVariantKey,
    /// Native atlas, with source 32px cells and straight RGBA pixels.
    pub atlas: AtlasImage,
    /// Top-left relative to the native tile, retaining source displacement/anchoring.
    pub offset: [i32; 2],
}

/// Owns the actual native verified index and native bounded sheet LRU. No second decoder,
/// raster cache or client-authored content fallback. A failed compose returns no old pixels.
pub struct PinnedOutfitAssets {
    index: AppearanceIndex,
    sheets: SpriteSheets,
    asset_manifest_sha256: Arc<str>,
}
impl PinnedOutfitAssets {
    pub fn open(
        asset_dir: &Path,
        manifest: &Path,
        manifest_sha256: &str,
    ) -> Result<Self, AppearanceRenderError> {
        let store = AssetStore::open_pinned(asset_dir, manifest, manifest_sha256)?;
        let catalog = Catalog::load(&store)?;
        let index = AppearanceIndex::load(&store, &catalog)?;
        Ok(Self {
            index,
            sheets: SpriteSheets::new(store, catalog),
            asset_manifest_sha256: Arc::from(manifest_sha256),
        })
    }

    /// Render only native first-group phase0/z0. Palette RGB is actual content DATA;
    /// no HSI table or entitlement/catalogue is manufactured here. Source order is
    /// [phase][z][y][x][layer], as exposed by the native AppearanceIndex.
    pub fn compose(
        &mut self,
        look: OutfitPixels,
        palette: &[[u8; 3]],
    ) -> Result<ComposedOutfit, AppearanceRenderError> {
        let colours = resolve_colours(look.colours, palette)?;
        let appearance = self
            .index
            .outfit(look.look_type)
            .ok_or(AppearanceRenderError::UnknownLook)?;
        let x = look.direction.pattern(appearance.pattern_width)?;
        let rows = enabled_rows(appearance.pattern_height, appearance.layers, look.addons)?;
        if appearance.pattern_depth == 0 || appearance.phases == 0 {
            return Err(AppearanceRenderError::UnsupportedGrammar);
        }
        // Preflight every selected sprite/layout before any pixel decode. Native dimensions
        // are at most64x64; each row shares the same bottom-right/source displacement anchor.
        let mut sprites = Vec::with_capacity(rows.len());
        let (mut columns, mut lines) = (0, 0);
        for y in rows {
            let index = (y as usize)
                .checked_mul(appearance.pattern_width as usize)
                .and_then(|v| v.checked_add(x as usize))
                .and_then(|v| v.checked_mul(appearance.layers as usize))
                .ok_or(AppearanceRenderError::UnsupportedGrammar)?;
            let base = *appearance
                .sprite_ids
                .get(index)
                .ok_or(AppearanceRenderError::UnsupportedGrammar)?;
            let mask = if appearance.layers == 2 {
                *appearance
                    .sprite_ids
                    .get(index + 1)
                    .ok_or(AppearanceRenderError::UnsupportedGrammar)?
            } else {
                0
            };
            let base_size = if base == 0 {
                None
            } else {
                Some(self.sheets.layout(base)?.cells())
            };
            let mask_size = if mask == 0 {
                None
            } else {
                Some(self.sheets.layout(mask)?.cells())
            };
            if base_size.is_some() && mask_size.is_some() && base_size != mask_size {
                return Err(AppearanceRenderError::UnsupportedGrammar);
            }
            if let Some((w, h)) = base_size.or(mask_size) {
                if (w * h) as usize * appearance.layers as usize > MAX_ENTRY_CELLS {
                    return Err(AppearanceRenderError::UnsupportedGrammar);
                }
                columns = columns.max(w);
                lines = lines.max(h);
                sprites.push((base, mask, w, h));
            }
        }
        if columns == 0 || lines == 0 {
            return Err(AppearanceRenderError::EmptyOutfit);
        }
        let width = columns * CELL_PX;
        let height = lines * CELL_PX;
        if (columns * lines) as usize > MAX_ENTRY_CELLS {
            return Err(AppearanceRenderError::UnsupportedGrammar);
        }
        let mut rgba = vec![0; (width * height * 4) as usize];
        for (base, mask, w, h) in sprites {
            // Every base row is alpha-over, then that row's exact opaque colour mask
            // multiplies the composed RGB. Addon rows are traversed in ascending y.
            for (sprite, tint) in [(base, false), (mask, true)] {
                if sprite == 0 {
                    continue;
                }
                for cy in 0..h {
                    for cx in 0..w {
                        let cell = self.sheets.cell_rgba(sprite, cx, cy)?;
                        for py in 0..CELL_PX {
                            for px in 0..CELL_PX {
                                let src = ((py * CELL_PX + px) * 4) as usize;
                                let dest = ((((lines - h) * CELL_PX + cy * CELL_PX + py) * width
                                    + (columns - w) * CELL_PX
                                    + cx * CELL_PX
                                    + px)
                                    * 4) as usize;
                                let pixel = &cell[src..src + 4];
                                let target = &mut rgba[dest..dest + 4];
                                if tint {
                                    tint_mask(target, pixel, &colours);
                                } else {
                                    alpha_over(target, pixel);
                                }
                            }
                        }
                    }
                }
            }
        }
        let dx = i32::try_from(appearance.displacement_x)
            .map_err(|_| AppearanceRenderError::UnsupportedGrammar)?;
        let dy = i32::try_from(appearance.displacement_y)
            .map_err(|_| AppearanceRenderError::UnsupportedGrammar)?;
        Ok(ComposedOutfit {
            variant_key: OutfitVariantKey {
                asset_manifest_sha256: Arc::clone(&self.asset_manifest_sha256),
                look,
                colours_rgb: colours,
            },
            atlas: AtlasImage::new(CELL_PX, columns, lines, rgba)?,
            offset: [
                CELL_PX as i32 - width as i32 - dx,
                CELL_PX as i32 - height as i32 - dy,
            ],
        })
    }

    pub fn sheet_decode_count(&self) -> u64 {
        self.sheets.decode_count()
    }
}

fn resolve_colours(
    indices: [u16; 4],
    palette: &[[u8; 3]],
) -> Result<[[u8; 3]; 4], AppearanceRenderError> {
    if palette.len() != PALETTE_LEN {
        return Err(AppearanceRenderError::InvalidPalette);
    }
    let mut result = [[0; 3]; 4];
    for (out, index) in result.iter_mut().zip(indices) {
        *out = *palette
            .get(usize::from(index))
            .ok_or(AppearanceRenderError::InvalidPalette)?;
    }
    Ok(result)
}
fn enabled_rows(height: u32, layers: u32, addons: u8) -> Result<Vec<u32>, AppearanceRenderError> {
    if !(1..=9).contains(&height) || !(1..=2).contains(&layers) {
        return Err(AppearanceRenderError::UnsupportedGrammar);
    }
    let allowed = (1_u16 << (height - 1)) - 1;
    if u16::from(addons) & !allowed != 0 {
        return Err(AppearanceRenderError::InvalidAddons);
    }
    Ok((0..height)
        .filter(|y| *y == 0 || u16::from(addons) & (1_u16 << (y - 1)) != 0)
        .collect())
}
fn tint_mask(target: &mut [u8], mask: &[u8], colours: &[[u8; 3]; 4]) {
    let role = match mask {
        [255, 255, 0, 255] => 0,
        [255, 0, 0, 255] => 1,
        [0, 255, 0, 255] => 2,
        [0, 0, 255, 255] => 3,
        _ => return,
    };
    for (channel, tint) in target[..3].iter_mut().zip(colours[role]) {
        *channel = (u16::from(*channel) * u16::from(tint) / 255) as u8;
    }
}
fn alpha_over(target: &mut [u8], source: &[u8]) {
    let sa = u32::from(source[3]);
    let da = u32::from(target[3]);
    let alpha = sa * 255 + da * (255 - sa);
    if alpha == 0 {
        return;
    }
    for i in 0..3 {
        target[i] =
            ((u32::from(source[i]) * sa * 255 + u32::from(target[i]) * da * (255 - sa) + alpha / 2)
                / alpha) as u8;
    }
    target[3] = ((alpha + 127) / 255) as u8;
}

#[cfg(test)]
#[path = "appearance_rendering_tests.rs"]
mod tests;
