//! Play-view lifetime assets and atlas cells. Recentring does not reload or repack them.
#[path = "item_cache.rs"]
mod item_cache;
use super::*;
use oteryn_client_assets::world::{MANIFEST_PATH, MANIFEST_SHA256};
use oteryn_client_assets::{AppearanceIndex, AssetStore, Catalog, SpriteSheets};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};

type OutfitCacheKey = (u32, u32, u8, [u16; 4], [[u8; 3]; 4]);

struct Assets {
    index: AppearanceIndex,
    sheets: SpriteSheets,
}

#[derive(Clone, Default)]
pub(super) struct PresentationCache {
    assets: Option<Arc<Mutex<Assets>>>,
    attempted: bool,
    pixels: Vec<u8>,
    glyphs: BTreeMap<char, u16>,
    outfits: BTreeMap<OutfitCacheKey, Vec<Draw>>,
    items: BTreeMap<u32, Vec<Draw>>,
    image: Option<Arc<AtlasImage>>,
    packed_cells: usize,
    #[cfg(test)]
    counts: [usize; 3], // asset loads, base unpacks, atlas packs
}

impl std::fmt::Debug for PresentationCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PresentationCache")
            .field("glyphs", &self.glyphs.len())
            .field("outfits", &self.outfits.len())
            .finish_non_exhaustive()
    }
}

impl PresentationCache {
    /// Map changes invalidate slots, but retain the verified index and decoded sheets.
    pub(super) fn reset_atlas(&mut self) {
        self.pixels.clear();
        self.glyphs.clear();
        self.outfits.clear();
        self.items.clear();
        self.image = None;
        self.packed_cells = 0;
    }

    pub(super) fn begin(&mut self, base: &AtlasImage, root: Option<PathBuf>) {
        if self.pixels.is_empty() {
            self.pixels = unpack(base);
            #[cfg(test)]
            {
                self.counts[1] += 1;
            }
        }
        if !self.attempted {
            self.attempted = true;
            self.assets = root.and_then(|root| {
                #[cfg(test)]
                {
                    self.counts[0] += 1;
                }
                let store = AssetStore::open_pinned(
                    &root.join("content/assets/files"),
                    &root.join(MANIFEST_PATH),
                    MANIFEST_SHA256,
                )
                .ok()?;
                let catalog = Catalog::load(&store).ok()?;
                let index = AppearanceIndex::load(&store, &catalog).ok()?;
                Some(Arc::new(Mutex::new(Assets {
                    index,
                    sheets: SpriteSheets::new(store, catalog),
                })))
            });
        }
    }

    fn count(&self) -> usize {
        self.pixels.len() / (32 * 32 * 4)
    }

    fn append(&mut self, pixels: &[u8]) -> Option<u16> {
        let count = self.count();
        if count >= oteryn_renderer::world::MAX_CELLS {
            return None;
        }
        self.pixels.extend_from_slice(pixels);
        Some(count as u16)
    }

    pub(super) fn outfit(
        &mut self,
        look: Option<&oteryn_session::Look>,
        direction: u32,
        palette: Option<&[[u8; 3]]>,
    ) -> Vec<Draw> {
        if let Some(oteryn_session::Look::Item { definition_ref }) = look {
            return self.item(*definition_ref);
        }
        if matches!(look, Some(oteryn_session::Look::EmptyOutfit)) {
            return Vec::new();
        }
        let Some(oteryn_session::Look::Outfit {
            look_type,
            colours,
            addons,
        }) = look
        else {
            return marker();
        };
        let mut rgb = [[255; 3]; 4];
        for (part, index) in colours.iter().enumerate() {
            match palette.and_then(|palette| palette.get(usize::from(*index))) {
                Some(value) => rgb[part] = *value,
                None if *index == 0 => {}
                None => return marker(),
            }
        }
        let key = (*look_type, direction, *addons, *colours, rgb);
        if let Some(draws) = self.outfits.get(&key) {
            return draws.clone();
        }
        // Bound even negative cache entries when appearances change over a long session.
        if self.outfits.len() >= 1024 {
            return marker();
        }
        let resolved = self.assets.as_ref().and_then(|assets| {
            let mut assets = assets.lock().unwrap_or_else(PoisonError::into_inner);
            let Assets { index, sheets } = &mut *assets;
            Some(
                index
                    .resolve_outfit_rgba(sheets, *look_type, direction, *addons, rgb)
                    .ok()?
                    .into_iter()
                    .map(|cell| (cell.offset, cell.rgba))
                    .collect::<Vec<_>>(),
            )
        });
        let draws = resolved
            .and_then(|cells| {
                if self.count() + cells.len() > oteryn_renderer::world::MAX_CELLS {
                    return None;
                }
                cells
                    .into_iter()
                    .map(|(offset, rgba)| self.append(&rgba).map(|cell| Draw { cell, offset }))
                    .collect()
            })
            .unwrap_or_else(marker);
        self.outfits.insert(key, draws.clone());
        draws
    }

    pub(super) fn name(&mut self, name: &str) -> Vec<u16> {
        let mut cells = Vec::new();
        for character in name.chars() {
            let cell = if let Some(cell) = self.glyphs.get(&character) {
                *cell
            } else {
                let Some(cell) = self.append(&oteryn_renderer::name_text::glyph_rgba(character))
                else {
                    break;
                };
                self.glyphs.insert(character, cell);
                cell
            };
            cells.push(cell);
        }
        cells
    }

    pub(super) fn image(&mut self) -> Result<Arc<AtlasImage>, BatchError> {
        let count = self.count();
        if self.packed_cells != count {
            self.image = Some(Arc::new(oteryn_renderer::world::atlas_from_cells(
                self.pixels.clone(),
                count,
            )?));
            self.packed_cells = count;
            #[cfg(test)]
            {
                self.counts[2] += 1;
            }
        }
        self.image.clone().ok_or(BatchError::InvalidAtlas)
    }

    #[cfg(test)]
    pub(super) fn counts(&self) -> [usize; 3] {
        self.counts
    }
}

fn marker() -> Vec<Draw> {
    vec![Draw {
        cell: MARKER_CELL,
        offset: [0, 0],
    }]
}

fn unpack(atlas: &AtlasImage) -> Vec<u8> {
    let px = atlas.cell_px() as usize;
    let columns = atlas.width() as usize / px;
    let mut cells = Vec::new();
    for index in 0..atlas.cell_count() as usize {
        for y in 0..px {
            let at =
                (((index / columns) * px + y) * atlas.width() as usize + index % columns * px) * 4;
            cells.extend_from_slice(&atlas.rgba()[at..at + px * 4]);
        }
    }
    cells
}
