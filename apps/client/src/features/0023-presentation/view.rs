//! Transient combat overlays use server positions, including the own actor and hidden attackers.
//! Item effects retain their donor colours; fireworks expand into eight emissive sparks.
//! Other magic effects retain the existing hit pulse until official sprites are resolved.
#[path = "resources.rs"]
mod resources;
use super::state::{MAX_VISUAL_CUES, VISUAL_LIFETIME, VisualCue};
use crate::play::PlayView;
use oteryn_renderer::{AtlasImage, BatchError, QuadInstance, SpriteBatch, TileCoord, TileView};
use oteryn_session::{PresentationColor, PresentationEventKind, PresentationValue};
use std::time::Instant;

impl PlayView {
    pub(in crate::play) fn receive_presentation(
        &mut self,
        events: &[PresentationEventKind],
        now: Instant,
    ) {
        self.visuals
            .retain(|cue| now.saturating_duration_since(cue.received) < VISUAL_LIFETIME);
        self.visuals.extend(events.iter().map(|&kind| VisualCue {
            kind,
            received: now,
        }));
        let excess = self.visuals.len().saturating_sub(MAX_VISUAL_CUES);
        self.visuals.drain(..excess);
    }

    pub(in crate::play) fn advance_presentation(&mut self, now: Instant) -> Result<(), BatchError> {
        if self.visuals.is_empty() && !self.tutorial_overlay.enabled {
            return Ok(());
        }
        self.visuals
            .retain(|cue| now.saturating_duration_since(cue.received) < VISUAL_LIFETIME);
        self.project_presentation(now)
    }

    pub(in crate::play) fn project_presentation(&mut self, now: Instant) -> Result<(), BatchError> {
        // Palette pixels remain emissive: overlays must be legible in dark areas.
        let atlas = self.local.world();
        let atlas = atlas.atlas();
        let resource_options = resources::Options::from(self.preferences());
        let effects = self.preferences().show_effects;
        let creature_bars = self.preferences().creature_bars;
        let (view, batch) = self.scene.presentation_base()?;
        resources::draw(batch, view, atlas, self.vitals.as_ref(), self.own, resource_options)?;
        if creature_bars {
            for entity in &self.entities {
                let oteryn_session::EntityDetail::Actor { health_percent, .. } = entity.detail
                else {
                    continue;
                };
                let tile = TileCoord::new(entity.position.x, entity.position.y);
                if entity.position.floor != self.floor || !view.contains(tile) {
                    continue;
                }
                let [x, y] = view.tile_to_screen(tile);
                let at = [x + 2.0, y + view.tile_px() as f32 - 5.0];
                let width = (view.tile_px() as f32 - 4.0).max(1.0);
                pixel(
                    batch,
                    view,
                    atlas,
                    at,
                    [width, 3.0],
                    PresentationColor::DarkRed,
                )?;
                pixel(
                    batch,
                    view,
                    atlas,
                    at,
                    [width * f32::from(health_percent.min(100)) / 100.0, 3.0],
                    PresentationColor::LightGreen,
                )?;
                if let Some(icon) = self
                    .party_icons
                    .iter()
                    .find(|icon| icon.entity == entity.entity)
                {
                    for (offset, value, pattern) in [
                        (0.0, icon.shield, [5_u8, 7, 2]),
                        (8.0, icon.skull, [7_u8, 5, 2]),
                    ] {
                        if value == 0 {
                            continue;
                        }
                        let color = match value {
                            1 | 3 | 5 | 7 | 9 => PresentationColor::Blue,
                            2 | 4 | 6 | 8 | 10 => PresentationColor::Yellow,
                            _ => PresentationColor::Red,
                        };
                        for (row, bits) in pattern.into_iter().enumerate() {
                            for col in 0..3 {
                                if bits & (4 >> col) != 0 {
                                    pixel(
                                        batch,
                                        view,
                                        atlas,
                                        [x + offset + col as f32 * 2.0, y + row as f32 * 2.0],
                                        [2.0, 2.0],
                                        color,
                                    )?;
                                }
                            }
                        }
                    }
                }
            }
        }
        draw_tutorial(batch, view, atlas, &self.tutorial_overlay, now)?;
        if !effects {
            return Ok(());
        }
        let mut lines = std::collections::BTreeMap::new();
        for cue in &self.visuals {
            let age = now.saturating_duration_since(cue.received);
            if age >= VISUAL_LIFETIME {
                continue;
            }
            let progress = age.as_secs_f32() / VISUAL_LIFETIME.as_secs_f32();
            match cue.kind {
                PresentationEventKind::ValueText {
                    position,
                    primary,
                    secondary,
                    ..
                } if position.floor == self.floor => {
                    let tile = TileCoord::new(position.x, position.y);
                    if !view.contains(tile) {
                        continue;
                    }
                    let [x, y] = view.tile_to_screen(tile);
                    let center = x + view.tile_px() as f32 / 2.0;
                    let line = lines.entry((position.x, position.y)).or_insert(0_u32);
                    let top = y - progress * 32.0 - 12.0 * *line as f32;
                    *line += 1 + u32::from(secondary.is_some());
                    number(batch, view, atlas, [center, top], primary)?;
                    if let Some(value) = secondary {
                        number(batch, view, atlas, [center, top - 12.0], value)?;
                    }
                }
                PresentationEventKind::MagicEffect {
                    position,
                    effect_id,
                    ..
                } if position.floor == self.floor && progress < 0.5 => {
                    let tile = TileCoord::new(position.x, position.y);
                    if !view.contains(tile) {
                        continue;
                    }
                    let [x, y] = view.tile_to_screen(tile);
                    let half = view.tile_px() as f32 / 2.0;
                    let radius = 4.0 + progress * 24.0;
                    // Crystal utils_definitions.hpp: FIREWORK_YELLOW/RED/BLUE = 29/30/31.
                    let color = match effect_id {
                        1 | 14 | 30 => PresentationColor::Red,
                        7 => PresentationColor::Orange,
                        16 => PresentationColor::PastelRed,
                        24 => PresentationColor::SkyBlue,
                        27 => PresentationColor::White,
                        28 => PresentationColor::LightGreen,
                        31 => PresentationColor::Blue,
                        _ => PresentationColor::Yellow,
                    };
                    if matches!(effect_id, 29..=31) {
                        for [dx, dy] in [
                            [-1.0, 0.0],
                            [1.0, 0.0],
                            [0.0, -1.0],
                            [0.0, 1.0],
                            [-0.7, -0.7],
                            [-0.7, 0.7],
                            [0.7, -0.7],
                            [0.7, 0.7],
                        ] {
                            pixel(
                                batch,
                                view,
                                atlas,
                                [x + half + dx * radius, y + half + dy * radius],
                                [3.0, 3.0],
                                color,
                            )?;
                        }
                        continue;
                    }
                    for [dx, dy, w, h] in [
                        [-radius, -radius, radius * 2.0, 2.0],
                        [-radius, radius, radius * 2.0, 2.0],
                        [-radius, -radius, 2.0, radius * 2.0],
                        [radius, -radius, 2.0, radius * 2.0],
                    ] {
                        pixel(
                            batch,
                            view,
                            atlas,
                            [x + half + dx, y + half + dy],
                            [w, h],
                            color,
                        )?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}

// Original compact 3 × 5 decimal glyphs, most significant row first.
const DIGITS: [[u8; 5]; 10] = [
    [7, 5, 5, 5, 7],
    [2, 6, 2, 2, 7],
    [7, 1, 7, 4, 7],
    [7, 1, 7, 1, 7],
    [5, 5, 7, 1, 1],
    [7, 4, 7, 1, 7],
    [7, 4, 7, 5, 7],
    [7, 1, 1, 1, 1],
    [7, 5, 7, 5, 7],
    [7, 5, 7, 1, 7],
];

fn number(
    batch: &mut SpriteBatch,
    view: &TileView,
    atlas: &AtlasImage,
    [center, top]: [f32; 2],
    value: PresentationValue,
) -> Result<(), BatchError> {
    let text = value.value.to_string();
    let left = center - (text.len() as f32 * 8.0 - 2.0) / 2.0;
    for (i, digit) in text.bytes().enumerate() {
        for (row, bits) in DIGITS[usize::from(digit - b'0')].iter().enumerate() {
            for column in 0..3 {
                if bits & (4 >> column) != 0 {
                    pixel(
                        batch,
                        view,
                        atlas,
                        [left + (i * 8 + column * 2) as f32, top + row as f32 * 2.0],
                        [2.0, 2.0],
                        value.color,
                    )?;
                }
            }
        }
    }
    Ok(())
}

fn rgb(color: PresentationColor) -> [u8; 3] {
    use PresentationColor::*;
    match color {
        Red => [5, 0, 0],
        LightGreen => [2, 5, 2],
        LightGrey => [4, 4, 4],
        Orange => [5, 3, 0],
        Purple => [4, 1, 5],
        SkyBlue => [0, 4, 5],
        Yellow => [5, 5, 0],
        DarkRed => [3, 0, 0],
        LightBlue => [2, 3, 5],
        Blue => [0, 0, 5],
        PastelRed => [5, 2, 2],
        MayaBlue => [2, 4, 5],
        White => [5, 5, 5],
    }
}

fn pixel(
    batch: &mut SpriteBatch,
    view: &TileView,
    atlas: &AtlasImage,
    position: [f32; 2],
    size: [f32; 2],
    color: PresentationColor,
) -> Result<(), BatchError> {
    let (width, height) = view.viewport_px();
    if position[0] < 0.0
        || position[1] < 0.0
        || position[0] + size[0] > width as f32
        || position[1] + size[1] > height as f32
    {
        return Ok(());
    }
    let [r, g, b] = rgb(color);
    let index = u32::from(r) * 36 + u32::from(g) * 6 + u32::from(b);
    let cell = u32::from(oteryn_renderer::world::PALETTE_CELL);
    let columns = atlas.width() / atlas.cell_px();
    let x = (cell % columns) * atlas.cell_px() + index % atlas.cell_px();
    let y = (cell / columns) * atlas.cell_px() + index / atlas.cell_px();
    let u = (x as f32 + 0.5) / atlas.width() as f32;
    let v = (y as f32 + 0.5) / atlas.height() as f32;
    if batch.len() == oteryn_renderer::MAX_BATCH_QUADS {
        return Ok(());
    }
    batch.push_quad(QuadInstance {
        position,
        size,
        uv: [u, v, u, v],
    })
}

fn draw_tutorial(
    batch: &mut SpriteBatch,
    view: &TileView,
    atlas: &AtlasImage,
    overlay: &crate::play::tutorial_overlay::Overlay,
    now: Instant,
) -> Result<(), BatchError> {
    if !overlay.enabled {
        return Ok(());
    }
    for [x, y] in overlay.points(now) {
        pixel(
            batch,
            view,
            atlas,
            [x as f32, y as f32],
            [3.0, 3.0],
            PresentationColor::Yellow,
        )?;
    }
    if let Some([x, y]) = overlay.marker {
        for (dx, dy) in [(-8, 0), (8, 0), (0, -8), (0, 8)] {
            pixel(
                batch,
                view,
                atlas,
                [(x + dx) as f32, (y + dy) as f32],
                [4.0, 4.0],
                PresentationColor::LightGreen,
            )?;
        }
    }
    for (index, character) in overlay.hint().chars().enumerate() {
        let glyph = oteryn_renderer::name_text::glyph_rgba(character);
        for y in 0..7usize {
            for x in 0..5usize {
                if glyph[(((y * 32 + 16) / 8) * 32 + (x * 32 + 16) / 6) * 4 + 3] == 255 {
                    pixel(
                        batch,
                        view,
                        atlas,
                        [8.0 + index as f32 * 6.0 + x as f32, 8.0 + y as f32],
                        [1.0, 1.0],
                        PresentationColor::White,
                    )?;
                }
            }
        }
    }
    Ok(())
}
