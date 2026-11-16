use super::*;
use crate::map::Draw;
use oteryn_session::ActorPosition;

pub(super) fn sprites(
    batch: &mut SpriteBatch,
    view: &TileView,
    atlas: &AtlasImage,
    position: ActorPosition,
    draws: &[Draw],
) -> Result<(), BatchError> {
    let tile = TileCoord::new(position.x, position.y);
    let at = view.tile_to_screen(tile);
    at_screen(batch, view, atlas, at, draws)
}
fn at_screen(
    batch: &mut SpriteBatch,
    view: &TileView,
    atlas: &AtlasImage,
    at: [f32; 2],
    draws: &[Draw],
) -> Result<(), BatchError> {
    let scale = view.tile_px() as f32 / 32.0;
    let (width, height) = view.viewport_px();
    for draw in draws {
        let position = [
            at[0] + draw.offset[0] as f32 * scale,
            at[1] + draw.offset[1] as f32 * scale,
        ];
        let size = [view.tile_px() as f32; 2];
        if position[0] + size[0] <= 0.0
            || position[1] + size[1] <= 0.0
            || position[0] >= width as f32
            || position[1] >= height as f32
        {
            continue;
        }
        if batch.len() == oteryn_renderer::MAX_BATCH_QUADS {
            break;
        }
        batch.push_quad(QuadInstance {
            position,
            size,
            uv: atlas.uv_rect(draw.cell)?,
        })?;
    }
    Ok(())
}
pub(super) fn missile(
    batch: &mut SpriteBatch,
    view: &TileView,
    atlas: &AtlasImage,
    from: ActorPosition,
    to: ActorPosition,
    elapsed: f32,
    frame: Option<&[Draw]>,
) -> Result<(), BatchError> {
    let a = view.tile_to_screen(TileCoord::new(from.x, from.y));
    let b = view.tile_to_screen(TileCoord::new(to.x, to.y));
    let distance = ((a[0] - b[0]).hypot(a[1] - b[1]) / view.tile_px() as f32).sqrt();
    let duration = distance * 0.15;
    if elapsed >= duration {
        return Ok(());
    }
    let progress = (elapsed / duration).clamp(0.0, 1.0);
    let at = [
        a[0] + (b[0] - a[0]) * progress,
        a[1] + (b[1] - a[1]) * progress,
    ];
    if let Some(draws) = frame {
        at_screen(batch, view, atlas, at, draws)
    } else {
        pixel(batch, view, atlas, at, [3.0; 2], PresentationColor::Yellow)
    }
}

pub(super) fn words(
    batch: &mut SpriteBatch,
    view: &TileView,
    atlas: &AtlasImage,
    font: Option<&GameFont>,
    position: ActorPosition,
    text: &str,
    progress: f32,
) -> Result<(), BatchError> {
    let tile = TileCoord::new(position.x, position.y);
    if !view.contains(tile) {
        return Ok(());
    }
    let [left, top] = view.tile_to_screen(tile);
    if let Some(font) = font {
        let at = [left + view.tile_px() as f32 / 2.0, top - 12.0 - progress * 20.0];
        return game_text(batch, view, atlas, font, text, at, rgb(PresentationColor::Orange));
    }
    // Tibia does not cap spell words; the catalogue wire bounds them.
    let text: Vec<_> = text.chars().collect();
    for (index, character) in text.iter().enumerate() {
        let glyph = oteryn_renderer::name_text::glyph_rgba(*character);
        for y in 0..7usize {
            for x in 0..5usize {
                let at = (((y * 32 + 16) / 8) * 32 + (x * 32 + 16) / 6) * 4 + 3;
                if glyph[at] != 0 {
                    pixel(
                        batch,
                        view,
                        atlas,
                        [
                            left + view.tile_px() as f32 / 2.0 - text.len() as f32 * 3.0
                                + index as f32 * 6.0
                                + x as f32,
                            top - 12.0 - progress * 20.0 + y as f32,
                        ],
                        [1.0; 2],
                        PresentationColor::Orange,
                    )?;
                }
            }
        }
    }
    Ok(())
}
impl PlayView {
    pub(in crate::play) fn sync_spell_words(&mut self, link: &crate::play::PlayLink) {
        self.spell_words.clear();
        if !self
            .visuals
            .iter()
            .any(|cue| matches!(cue.kind, PresentationEventKind::SpellWords { .. }))
        {
            return;
        }
        let panel = link.spell_catalog_panel();
        if !panel.selected {
            return;
        }
        for cue in &self.visuals {
            if let PresentationEventKind::SpellWords { spell, .. } = cue.kind
                && let Some(entry) = panel
                    .catalog
                    .entries()
                    .iter()
                    .find(|entry| entry.cast_index == spell)
            {
                self.spell_words
                    .insert(spell, entry.words.clone());
            }
        }
    }
}

#[cfg(test)]
mod timing_tests {
    use super::*;
    #[test]
    fn missile_uses_square_root_distance_and_zero_paths_vanish() -> Result<(), BatchError> {
        let world = crate::map::World::builtin()?;
        let view = TileView::new(TileCoord::new(0, 0), 32, 15, 11)?;
        let from = ActorPosition { x: 1, y: 1, floor: 0 };
        for frame in [None, Some(&[Draw { cell: crate::map::MARKER_CELL, offset: [0, 0] }][..])] {
            for (distance, elapsed, visible) in [(0, 0.0, false), (1, 0.1, true), (1, 0.15, false), (4, 0.25, true), (4, 0.3, false)] {
                let mut batch = SpriteBatch::new();
                missile(&mut batch, &view, world.atlas(), from, ActorPosition { x: from.x + distance, ..from }, elapsed, frame)?;
                assert_eq!(!batch.is_empty(), visible, "distance {distance}, elapsed {elapsed}");
            }
        }
        Ok(())
    }
}
