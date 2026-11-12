//! Play-view lifetime assets and atlas cells. Recentring does not reload or repack them.
use super::*;
use oteryn_client_assets::world::{MANIFEST_PATH, MANIFEST_SHA256};
use oteryn_client_assets::{AppearanceIndex, AssetStore, Catalog, SpriteSheets};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};

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
    outfits: BTreeMap<(u32, u32), Vec<Draw>>,
    image: Option<Arc<AtlasImage>>,
    visuals: BTreeMap<(u32, bool, u32, [i32; 2]), Vec<Draw>>,
    packed_cells: usize,
    #[cfg(test)]
    counts: [usize; 3], // asset loads, base unpacks, atlas packs
}

impl std::fmt::Debug for PresentationCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(tr!("PresentationCache"))
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
        self.visuals.clear();
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
    ) -> Vec<Draw> {
        if matches!(look, Some(oteryn_session::Look::EmptyOutfit)) {
            return Vec::new();
        }
        let Some(oteryn_session::Look::Outfit { look_type, .. }) = look else {
            return marker();
        };
        let key = (*look_type, direction);
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
            let entry = index.resolve_outfit(sheets, *look_type, direction).ok()?;
            entry
                .cells
                .iter()
                .map(|cell| {
                    sheets
                        .cell_rgba(cell.sprite_id, cell.cell_x, cell.cell_y)
                        .ok()
                        .map(|rgba| ([cell.offset_x, cell.offset_y], rgba))
                })
                .collect::<Option<Vec<_>>>()
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

include!("cache_visuals.rs");
