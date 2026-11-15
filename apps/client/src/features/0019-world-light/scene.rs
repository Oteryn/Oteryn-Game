//! Floor separation follows the actual culled painter ranges sent to the renderer.
use super::*;
type FloorQuads = Vec<(std::ops::Range<usize>, i16)>;
type Dimming = Vec<(std::ops::Range<usize>, u8)>;
type WorldSprites = (
    SpriteBatch,
    Dimming,
    FloorQuads,
    Option<std::ops::Range<usize>>,
);
impl Scene {
    pub(crate) fn has_roof(&self) -> bool {
        let Some(anchor) = self.anchor else {
            return false;
        };
        self.world.draw_floors().into_iter().any(|floor| {
            floor > self.world.center().2
                && self
                    .world
                    .floor_tile(floor, self.own.x + anchor.x, self.own.y + anchor.y)
                    .is_some_and(|tile| !tile.under.is_empty() || !tile.over.is_empty())
        })
    }
    pub(crate) fn set_floor_separator(&mut self, value: u8) -> Result<(), BatchError> {
        if self.floor_separator != value {
            self.floor_separator = value;
            self.rebuild_sprites()?;
        }
        Ok(())
    }
    /// The sprites in painter order, the dimmed floor ranges, the per-floor quad ranges and the
    /// quads of the player's own outfit (which a scrolling camera leaves in place).
    pub(super) fn world_sprites(&self) -> Result<WorldSprites, BatchError> {
        let (atlas, origin) = (self.atlas(), self.view.origin());
        let mut sprites = SpriteBatch::new();
        let mut dimmed = Vec::new();
        let mut player = None::<std::ops::Range<usize>>;
        let mut floor_quads = Vec::new();
        let limit = MAX_BATCH_QUADS - usize::from(self.target.is_some() && self.target_frame);
        #[cfg(feature = "bevy-shell")]
        let floors = self.world.all_draw_floors();
        #[cfg(not(feature = "bevy-shell"))]
        let floors = self.world.draw_floors();
        for floor in floors {
            let own_floor = floor == self.world.center().2;
            let start = sprites.len();
            let own_draws: &[Draw] = if own_floor {
                self.own_draws
                    .as_deref()
                    .unwrap_or_else(|| self.world.player(self.facing))
            } else {
                &[]
            };
            {
                let emit = &mut |(x, y), draw: Draw| {
                    if sprites.len() >= limit {
                        return Ok(());
                    }
                    let at = sprites.len();
                    let side = self.view.tile_px() as f32;
                    let scale = side / oteryn_client_domain::world::CELL_PX as f32;
                    let [sx, sy] = self.view.tile_to_screen(TileCoord::new(x, y));
                    let position = [
                        sx + draw.offset[0] as f32 * scale,
                        sy + draw.offset[1] as f32 * scale,
                    ];
                    let (w, h) = self.view.viewport_px();
                    let pushed = position[0] + side > -side - self.scroll[0].max(0.)
                        && position[1] + side > -side - self.scroll[1].max(0.)
                        && position[0] < w as f32 + side - self.scroll[0].min(0.)
                        && position[1] < h as f32 + side - self.scroll[1].min(0.);
                    if pushed {
                        sprites.push_quad(oteryn_renderer::QuadInstance {
                            position,
                            size: [side; 2],
                            uv: atlas.uv_rect(draw.cell)?,
                        })?;
                    }
                    // Elevation shifts a draw's offset, so the own outfit is told by its cell.
                    if pushed
                        && (x, y) == (self.own.x, self.own.y)
                        && own_draws.iter().any(|own| own.cell == draw.cell)
                    {
                        player = Some(player.take().map_or(at..at + 1, |seen| seen.start..at + 1));
                    }
                    Ok(())
                };
                visit_scene_entities(
                    (origin.x, origin.y),
                    self.anchor.map(|anchor| (anchor.x, anchor.y)),
                    ((self.own.x, self.own.y), own_draws),
                    |x, y| self.world.floor_tile(floor, x, y),
                    |x, y| {
                        let tile = TileCoord::new(x, y);
                        if !own_floor {
                            return Vec::new();
                        }
                        let has_actor = self.actor_draws.iter().any(|(at, _)| *at == tile);
                        // Object markers remain below actors. Entity fallback glyphs are replaced.
                        let marker = self.visible.iter().any(|v| {
                            v.tile == tile
                                && ((v.kind == TargetKind::Object && (v.glyph || v.item.is_none()))
                                    || (v.glyph && !has_actor))
                        });
                        marker
                            .then_some(Draw {
                                cell: MARKER_CELL,
                                offset: [0, 0],
                            })
                            .into_iter()
                            .chain(
                                self.actor_draws
                                    .iter()
                                    .filter(|(at, _)| *at == tile)
                                    .flat_map(|(_, draws)| draws.iter().copied()),
                            )
                            .collect()
                    },
                    self.scroll.map(|v| v / self.view.tile_px() as f32),
                    &mut *emit,
                )?;
            }
            floor_quads.push((start..sprites.len(), floor));
            if floor < self.world.center().2 && self.floor_separator > 0 && sprites.len() > start {
                dimmed.push((start..sprites.len(), self.floor_separator));
            }
        }
        if let Some(target) = self.target.filter(|_| self.target_frame) {
            sprites.push(&self.view, atlas, target.tile, crate::map::TARGET_CELL)?;
        }
        Ok((sprites, dimmed, floor_quads, player))
    }
}

#[cfg(feature = "bevy-shell")]
impl Scene {
    pub(crate) fn floor_quad_ranges(&self) -> &[(std::ops::Range<usize>, i16)] {
        &self.floor_quads
    }

    pub(crate) fn first_visible_floor(&self) -> i16 {
        self.world.first_visible_floor
    }
}

/// OTClient's default floor fading interval is 500 ms; reversals retain current opacity.
#[cfg(any(feature = "bevy-shell", test))]
pub(crate) fn fade_alpha(alpha: f32, visible: bool, elapsed_seconds: f32) -> f32 {
    let step = elapsed_seconds / 0.5;
    if visible {
        (alpha + step).min(1.0)
    } else {
        (alpha - step).max(0.0)
    }
}

#[cfg(test)]
mod floorfade_tests {
    #[test]
    fn floor_fade_alpha_over_time() {
        assert_eq!(super::fade_alpha(1.0, false, 0.0), 1.0);
        assert_eq!(super::fade_alpha(1.0, false, 0.25), 0.5);
        assert_eq!(super::fade_alpha(1.0, false, 0.5), 0.0);
        assert_eq!(super::fade_alpha(0.0, true, 0.25), 0.5);
        assert_eq!(super::fade_alpha(0.5, true, 0.125), 0.75);
    }
}
