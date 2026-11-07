//! Crystal protocolgame.cpp:8043,7846: public map appearances and visible effect cues.
use oteryn_client_assets::{
    AppearanceIndex, AssetStore, Catalog, DrawCell, Placement, SpriteSheets,
};
use oteryn_client_runtime::cast_view::Picture;
use oteryn_renderer::world::{
    BLACK_CELL, BUILTIN_CELLS, MARKER_CELL, WorldAtlas, atlas_from_cells, builtin_rgba,
};
use oteryn_renderer::{AtlasImage, SpriteBatch, TileBatch, TileCoord, TileView};
use std::{path::Path, time::Instant};
pub(super) struct Assets {
    source: Option<(AppearanceIndex, SpriteSheets)>,
}
pub(super) struct Drawn {
    pub atlas: AtlasImage,
    pub tiles: TileBatch,
    pub sprites: SpriteBatch,
}
impl Assets {
    pub fn new() -> Self {
        Self {
            source: std::env::var_os(crate::map::ASSET_DIR_ENV)
                .and_then(|root| Self::load(Path::new(&root)).ok()),
        }
    }
    fn load(root: &Path) -> Result<(AppearanceIndex, SpriteSheets), String> {
        use oteryn_client_assets::world::{MANIFEST_PATH, MANIFEST_SHA256};
        let store = AssetStore::open_pinned(
            &root.join("content/assets/files"),
            &root.join(MANIFEST_PATH),
            MANIFEST_SHA256,
        )
        .map_err(|e| e.to_string())?;
        let catalog = Catalog::load(&store).map_err(|e| e.to_string())?;
        let index = AppearanceIndex::load(&store, &catalog).map_err(|e| e.to_string())?;
        Ok((index, SpriteSheets::new(store, catalog)))
    }
    pub fn draw(&mut self, picture: &Picture, now: Instant) -> Result<Drawn, String> {
        let view = TileView::new(TileCoord::new(7, 9), 32, 18, 14).map_err(|e| e.to_string())?;
        let mut draws: Vec<(TileCoord, Vec<DrawCell>)> = Vec::new();
        let mut markers = Vec::new();
        let mut cell_count = 0usize;
        if let Some(camera) = picture.floor.filter(|_| !picture.ended && !picture.slate) {
            let projected = |x: i32, y: i32, floor: i16| {
                let shift = i32::from(floor - camera);
                TileCoord::new(x - shift, y - shift)
            };
            for (&(floor, y, x), appearances) in &picture.tiles {
                let at = projected(i32::from(x), i32::from(y), floor);
                let mut cells = Vec::new();
                if let Some((index, sheets)) = &self.source {
                    for appearance in appearances {
                        if let Ok(entry) = index.resolve(
                            sheets,
                            u32::from(*appearance),
                            Placement::at(i32::from(x), i32::from(y), i32::from(floor)),
                        ) {
                            let remaining =
                                oteryn_renderer::MAX_BATCH_QUADS.saturating_sub(cell_count);
                            let cells_to_add = entry.cells.into_iter().take(remaining);
                            let before = cells.len();
                            cells.extend(cells_to_add);
                            cell_count += cells.len() - before;
                        }
                    }
                }
                if cells.is_empty() {
                    markers.push(at);
                } else {
                    draws.push((at, cells));
                }
            }
            for &(_, x, y, look, _) in &picture.actors {
                let floor = i16::try_from(y / 32).map_err(|_| "Invalid display floor")? - 15;
                let at = projected(i32::from(x), i32::from(y % 32), floor);
                let look = if look == 1 { 128 } else { u32::from(look) };
                let cells = self
                    .source
                    .as_ref()
                    .and_then(|(index, sheets)| index.resolve_outfit(sheets, look, 2).ok())
                    .map(|entry| entry.cells)
                    .unwrap_or_default();
                if cells.is_empty() {
                    markers.push(at);
                } else {
                    draws.push((at, cells));
                }
            }
        }
        let prepared = if let Some((_, sheets)) = &mut self.source {
            Some(WorldAtlas::prepare(
                draws.iter().flat_map(|(_, cells)| cells),
                |sprite, x, y| sheets.cell_rgba(sprite, x, y).map_err(|e| e.to_string()),
            )?)
        } else {
            None
        };
        let mut placements = Vec::new();
        for (at, cells) in &draws {
            for cell in cells {
                if let Some(slot) = prepared.as_ref().and_then(|atlas| atlas.slot(cell)) {
                    placements.push((*at, slot, [cell.offset_x, cell.offset_y]));
                }
            }
        }
        let atlas = match prepared {
            Some(atlas) => atlas.into_image(),
            None => atlas_from_cells(builtin_rgba(), BUILTIN_CELLS).map_err(|e| e.to_string())?,
        };
        let mut sprites = SpriteBatch::new();
        for (at, slot, offset) in placements {
            if sprites.len() == oteryn_renderer::MAX_BATCH_QUADS {
                break;
            }
            sprites
                .push_offset(&view, &atlas, at, slot, offset, 32)
                .map_err(|e| e.to_string())?;
        }
        for at in markers {
            if sprites.len() == oteryn_renderer::MAX_BATCH_QUADS {
                break;
            }
            sprites
                .push(&view, &atlas, at, MARKER_CELL)
                .map_err(|e| e.to_string())?;
        }
        // Public effect types use the same compact visible cues as the playing client.
        for (received, event) in &picture.effects {
            if now.saturating_duration_since(*received) >= std::time::Duration::from_millis(800) {
                continue;
            }
            let position = match event.kind {
                oteryn_session::PresentationEventKind::MagicEffect { position, .. } => position,
                oteryn_session::PresentationEventKind::Missile { to, .. } => to,
                _ => continue,
            };
            if let Some(camera) = picture
                .floor
                .filter(|_| sprites.len() < oteryn_renderer::MAX_BATCH_QUADS)
            {
                let shift = i32::from(position.floor - camera);
                sprites
                    .push(
                        &view,
                        &atlas,
                        TileCoord::new(position.x - shift, position.y - shift),
                        oteryn_renderer::world::TARGET_CELL,
                    )
                    .map_err(|e| e.to_string())?;
            }
        }
        let tiles = TileBatch::from_cells(&view, &atlas, &vec![BLACK_CELL; 18 * 14])
            .map_err(|e| e.to_string())?;
        Ok(Drawn {
            atlas,
            tiles,
            sprites,
        })
    }
}
