//! Live-map composition state retained across play-view rebuilds and redraws.
use crate::input::Targetable;
use crate::map_draw::{MapActor, MapDraw};
use crate::map_view::MapView;
use crate::world::{MARKER_CELL, TARGET_CELL, builtin_rgba};
use oteryn_renderer::{
    AtlasImage, AtlasPage, BatchError, CellKey, PAGE_CELL_PX, PAGE_CELLS_PER_ROW, PlacedCell,
    SpriteFrame, SpritePageUpload, TileView,
};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};

#[derive(Debug, Clone)]
pub(crate) struct StreamedScene {
    map: MapView,
    actors: Vec<MapActor>,
    draw: Arc<Mutex<MapDraw>>,
    page: AtlasPage,
    atlas: Arc<AtlasImage>,
    frame: SpriteFrame,
    // Rebuilds can precede presentation. Keep every pending slot's latest bytes.
    pending: BTreeMap<u16, SpritePageUpload>,
}

impl StreamedScene {
    pub(crate) fn compose(
        map: &MapView,
        actors: Vec<MapActor>,
        draw: Arc<Mutex<MapDraw>>,
        previous: Option<Self>,
        view: &TileView,
        target: Option<Targetable>,
    ) -> Result<Self, BatchError> {
        let mut state = match previous {
            Some(mut state) => {
                state.map = map.clone();
                state.actors = actors;
                state
            }
            None => Self {
                map: map.clone(),
                actors,
                draw,
                page: AtlasPage::new(),
                atlas: Arc::new(page_atlas()?),
                frame: SpriteFrame::new(16),
                pending: BTreeMap::new(),
            },
        };
        state.rebuild(view, target)?;
        Ok(state)
    }

    pub(crate) fn rebuild(
        &mut self,
        view: &TileView,
        target: Option<Targetable>,
    ) -> Result<(), BatchError> {
        let mut draw = self.draw.lock().unwrap_or_else(PoisonError::into_inner);
        let mut frame = draw.frame(&self.map, &self.actors, &mut self.page, view)?;
        if let Some(target) = target {
            frame.push_entry(
                &mut self.page,
                view,
                target.tile,
                &[PlacedCell {
                    key: CellKey {
                        sprite_id: 0,
                        cell_x: TARGET_CELL,
                        cell_y: 0,
                    },
                    offset_x: 0,
                    offset_y: 0,
                }],
            );
        }
        frame.validate_batches()?;
        let placeholder = builtin_cell(MARKER_CELL);
        for &(key, slot) in frame.uploads() {
            let rgba = draw.cell_rgba(key).unwrap_or_else(|_| placeholder.clone());
            self.pending.insert(slot, SpritePageUpload { slot, rgba });
        }
        self.frame = frame;
        Ok(())
    }

    pub(crate) fn reset_page(
        &mut self,
        view: &TileView,
        target: Option<Targetable>,
    ) -> Result<(), BatchError> {
        self.page = AtlasPage::new();
        self.pending.clear();
        self.rebuild(view, target)
    }

    pub(crate) fn atlas(&self) -> &AtlasImage {
        &self.atlas
    }
    pub(crate) fn frame(&self) -> &SpriteFrame {
        &self.frame
    }
    pub(crate) fn take_uploads(&mut self) -> Vec<SpritePageUpload> {
        std::mem::take(&mut self.pending).into_values().collect()
    }
}

fn builtin_cell(cell: u16) -> Vec<u8> {
    let bytes = (PAGE_CELL_PX * PAGE_CELL_PX * 4) as usize;
    builtin_rgba()[usize::from(cell) * bytes..(usize::from(cell) + 1) * bytes].to_vec()
}

fn page_atlas() -> Result<AtlasImage, BatchError> {
    let (width, height) = AtlasPage::texture_size();
    let mut rgba = vec![0; (width * height * 4) as usize];
    let placeholder = builtin_cell(MARKER_CELL);
    let line_bytes = (PAGE_CELL_PX * 4) as usize;
    for row in 0..PAGE_CELL_PX as usize {
        let at = ((PAGE_CELLS_PER_ROW * PAGE_CELL_PX) as usize + row) * width as usize * 4;
        rgba[at..at + line_bytes]
            .copy_from_slice(&placeholder[row * line_bytes..(row + 1) * line_bytes]);
    }
    AtlasImage::new(
        PAGE_CELL_PX,
        PAGE_CELLS_PER_ROW,
        PAGE_CELLS_PER_ROW + 1,
        rgba,
    )
}

#[cfg(test)]
mod tests;
