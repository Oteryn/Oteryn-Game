//! Play-view lifetime assets and atlas cells. Recentring does not reload or repack them.
use super::*;
use oteryn_client_assets::world::{MANIFEST_PATH, MANIFEST_SHA256};
use oteryn_client_assets::{AppearanceIndex, AssetStore, CELL_RGBA_BYTES, Catalog, SpriteSheets};
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
    graphics: Option<Arc<TibiaGraphics>>,
    icon_cells: Option<HudIconCells>,
    attempted: bool,
    pixels: Vec<u8>,
    glyphs: BTreeMap<(char, [u8; 3]), u16>,
    outfits: BTreeMap<OutfitKey, OutfitFrame>,
    outfit_free: Vec<u16>,
    palette: Vec<[u8; 3]>,
    image: Option<Arc<AtlasImage>>,
    visuals: BTreeMap<(u32, bool, u32, [i32; 2]), Vec<Draw>>,
    packed_cells: usize,
    #[cfg(bevy_shell)]
    items: BTreeMap<(u16, u32, u32), Vec<Draw>>,
    #[cfg(test)]
    counts: [usize; 3], // asset loads, base unpacks, atlas packs
    /// Generated bundles carry their own entry pins.
    #[cfg(test)]
    pub(crate) graphics_pins: Option<oteryn_client_assets::tibia_graphics::GraphicsPins>,
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
        self.outfit_free.clear();
        self.visuals.clear();
        self.icon_cells = None;
        #[cfg(bevy_shell)]
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
            #[cfg(not(test))]
            let pins = oteryn_client_assets::tibia_graphics::GraphicsPins::default();
            #[cfg(test)]
            let pins = self.graphics_pins.clone().unwrap_or_default();
            self.graphics = root.as_ref().and_then(|root| {
                TibiaGraphics::load_pinned(&root.join("content/assets/files"), &pins)
                    .ok()
                    .map(Arc::new)
            });
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
        if self.icon_cells.is_none() {
            self.icon_cells = self.append_icons();
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

    #[cfg(test)]
    pub(super) fn outfit(
        &mut self,
        look: Option<&oteryn_session::Look>,
        direction: u32,
    ) -> Vec<Draw> {
        self.animated_outfit(look, direction, (0, None), 0, None)
    }

    pub(super) fn animated_outfit(
        &mut self,
        look: Option<&oteryn_session::Look>,
        direction: u32,
        (elapsed_ms, step_ms): (u64, Option<u32>),
        mount: u32,
        actor: Option<oteryn_session::EntityRef>,
    ) -> Vec<Draw> {
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
        let moving = step_ms.is_some();
        let (phase, mount_phase) = self.assets.as_ref().map_or((0, 0), |assets| {
            let assets = assets.lock().unwrap_or_else(PoisonError::into_inner);
            (
                assets
                    .index
                    .outfit_preview(*look_type, moving)
                    .map_or(0, |a| {
                        outfit_phase(a.phases, &a.phase_ms, elapsed_ms, step_ms)
                    }),
                assets.index.outfit_preview(mount, moving).map_or(0, |a| {
                    outfit_phase(a.phases, &a.phase_ms, elapsed_ms, step_ms)
                }),
            )
        });
        let style = (*look_type, direction, *colours, *addons, mount);
        let key = actor.map_or((None, Some(style)), |actor| (Some(actor), None));
        let stamp = (phase, mount_phase, moving);
        if let Some(frame) = self.outfits.get(&key)
            && frame.stamp == stamp
            && frame.style == style
        {
            return frame.draws.clone();
        }
        // Animation reuses this style's atlas cells rather than retaining every
        // phase of every visible outfit for the lifetime of the connection.
        if self.outfits.len() >= 1024 && !self.outfits.contains_key(&key) {
            return marker();
        }
        let resolved = self.assets.as_ref().and_then(|assets| {
            let mut assets = assets.lock().unwrap_or_else(PoisonError::into_inner);
            let colours = match colours.map(|index| self.palette.get(usize::from(index)).copied()) {
                [Some(head), Some(body), Some(legs), Some(feet)] => Some([head, body, legs, feet]),
                _ => None,
            };
            let mut cells = if mount != 0 {
                outfit_cells(
                    &mut assets,
                    mount,
                    direction,
                    0,
                    None,
                    (mount_phase, moving, false),
                )?
            } else {
                Vec::new()
            };
            cells.extend(outfit_cells(
                &mut assets,
                *look_type,
                direction,
                *addons,
                colours,
                (phase, moving, mount != 0),
            )?);
            Some(cells)
        });
        let (mut slots, retired) = self.outfits.remove(&key).map_or_else(
            || (Vec::new(), Vec::new()),
            |frame| {
                if frame.style == style {
                    (frame.cells, Vec::new())
                } else {
                    (Vec::new(), frame.cells)
                }
            },
        );
        let draws = resolved
            .and_then(|cells| {
                if self.count()
                    + cells
                        .len()
                        .saturating_sub(slots.len() + self.outfit_free.len())
                    > oteryn_renderer::world::MAX_CELLS
                {
                    return None;
                }
                cells
                    .into_iter()
                    .enumerate()
                    .map(|(index, (offset, rgba))| {
                        let cell = if let Some(cell) = slots.get(index).copied() {
                            let at = usize::from(cell) * CELL_RGBA_BYTES;
                            self.pixels[at..at + CELL_RGBA_BYTES].copy_from_slice(&rgba);
                            self.image = None;
                            cell
                        } else {
                            let cell = if let Some(cell) = self.outfit_free.pop() {
                                let at = usize::from(cell) * CELL_RGBA_BYTES;
                                self.pixels[at..at + CELL_RGBA_BYTES].copy_from_slice(&rgba);
                                self.image = None;
                                cell
                            } else {
                                self.append(&rgba)?
                            };
                            slots.push(cell);
                            cell
                        };
                        Some(Draw { cell, offset })
                    })
                    .collect()
            })
            .unwrap_or_else(marker);
        self.outfit_free.extend(retired);
        self.outfits.insert(
            key,
            OutfitFrame {
                style,
                stamp,
                cells: slots,
                draws: draws.clone(),
            },
        );
        draws
    }

    pub(super) fn retain_outfit_actors(&mut self, entries: &[oteryn_session::VisibleAppearance]) {
        let free = &mut self.outfit_free;
        self.outfits.retain(|(actor, _), frame| {
            let retained =
                actor.is_none_or(|actor| entries.iter().any(|entry| entry.entity.entity == actor));
            if !retained {
                free.extend_from_slice(&frame.cells);
            }
            retained
        });
    }

    pub(super) fn name(&mut self, name: &str, health: u8) -> NameLabel {
        let colour = match health {
            93.. => [0, 188, 0],
            61..=92 => [80, 161, 80],
            31..=60 => [161, 161, 0],
            9..=30 => [191, 10, 10],
            4..=8 => [145, 15, 15],
            _ => [133, 12, 12],
        };
        let graphics = self.graphics.clone();
        let mut label = NameLabel::default();
        for character in name.chars() {
            let glyph = graphics
                .as_deref()
                .and_then(|graphics| graphics.font().glyph(character));
            let cell = if let Some(cell) = self.glyphs.get(&(character, colour)) {
                *cell
            } else {
                let pixels = glyph.map_or_else(
                    || {
                        let mut pixels = oteryn_renderer::name_text::glyph_rgba(character);
                        for pixel in pixels.as_chunks_mut::<4>().0 {
                            if pixel[3] == 255 {
                                pixel[..3].copy_from_slice(&colour);
                            }
                        }
                        pixels
                    },
                    |glyph| glyph_cell(glyph, colour),
                );
                let Some(cell) = self.append(&pixels) else {
                    break;
                };
                self.glyphs.insert((character, colour), cell);
                cell
            };
            label.push(cell, glyph);
        }
        label
    }

    pub(super) fn image(&mut self) -> Result<Arc<AtlasImage>, BatchError> {
        let count = self.count();
        if self.image.is_none() || self.packed_cells != count {
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

include!("cache_graphics.rs");
include!("cache_visuals.rs");
include!("cache_outfits.rs");

#[cfg(test)]
mod outfit_tests {
    use super::*;
    #[test]
    fn actor_motion_same_outfit_keeps_independent_pixels_and_recycles_frame_cells()
    -> Result<(), BatchError> {
        fn pixels(cache: &PresentationCache, draws: &[Draw]) -> Vec<u8> {
            draws
                .iter()
                .flat_map(|draw| {
                    let at = usize::from(draw.cell) * CELL_RGBA_BYTES;
                    cache.pixels[at..at + CELL_RGBA_BYTES].iter().copied()
                })
                .collect()
        }
        let mut cache = PresentationCache::default();
        let world = World::builtin()?;
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        cache.begin(world.atlas(), Some(root));
        let look = oteryn_session::Look::Outfit {
            look_type: 128,
            colours: [0; 4],
            addons: 0,
        };
        let actor = |tag| oteryn_session::EntityRef {
            identity: [tag; 16],
            generation: 1,
        };
        let fast = cache.animated_outfit(Some(&look), 2, (100, Some(200)), 0, Some(actor(1)));
        let fast_pixels = pixels(&cache, &fast);
        let slow = cache.animated_outfit(Some(&look), 2, (100, Some(800)), 0, Some(actor(2)));
        assert_eq!(pixels(&cache, &fast), fast_pixels);
        assert_ne!(pixels(&cache, &slow), fast_pixels);
        for elapsed in [0, 50, 100, 150] {
            cache.animated_outfit(Some(&look), 2, (elapsed, Some(200)), 0, Some(actor(1)));
        }
        let cells = cache.count();
        for elapsed in (0..20_000).step_by(50) {
            let draws =
                cache.animated_outfit(Some(&look), 2, (elapsed, Some(200)), 0, Some(actor(1)));
            assert!(draws.iter().any(|draw| draw.cell > MARKER_CELL));
        }
        assert_eq!(cache.count(), cells);
        cache.retain_outfit_actors(&[]);
        cache.animated_outfit(Some(&look), 2, (0, Some(200)), 0, Some(actor(3)));
        assert_eq!(cache.count(), cells);
        Ok(())
    }
    #[test]
    fn spatial_outfit_cache_composes_colours_addons_and_reuses_exact_looks()
    -> Result<(), BatchError> {
        let mut cache = PresentationCache::default();
        let world = World::builtin()?;
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        cache.begin(world.atlas(), Some(root));
        cache.set_palette(&[[255, 0, 0], [0, 0, 255]]);
        let look = |colours, addons| oteryn_session::Look::Outfit {
            look_type: 128,
            colours,
            addons,
        };
        let plain = cache.outfit(Some(&look([0; 4], 0)), 2);
        let coloured = cache.outfit(Some(&look([1; 4], 0)), 2);
        assert_ne!(
            plain[0].cell, coloured[0].cell,
            "colours need distinct atlas cells"
        );
        let pixels = |draws: &[Draw]| {
            draws
                .iter()
                .flat_map(|draw| {
                    let at = usize::from(draw.cell) * CELL_RGBA_BYTES;
                    cache.pixels[at..at + CELL_RGBA_BYTES].iter().copied()
                })
                .collect::<Vec<_>>()
        };
        assert!(
            pixels(&plain) != pixels(&coloured),
            "outfit pixels must use the selected palette colours"
        );
        let added = cache.outfit(Some(&look([1; 4], 3)), 2);
        assert!(
            added.len() > coloured.len(),
            "selected addons must be drawn"
        );
        let count = cache.count();
        let repeated = cache.outfit(Some(&look([1; 4], 3)), 2);
        assert_eq!(repeated[0].cell, added[0].cell);
        assert_eq!(cache.count(), count);
        assert!(
            cache
                .outfit(Some(&oteryn_session::Look::EmptyOutfit), 2)
                .is_empty()
        );
        Ok(())
    }
}

#[cfg(bevy_shell)]
include!("cache_items.rs");
