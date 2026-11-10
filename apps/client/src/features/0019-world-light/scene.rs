//! Floor separation follows the actual culled painter ranges sent to the renderer.
use super::*;
type Dimming = Vec<(std::ops::Range<usize>, u8)>;
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
    pub(super) fn world_sprites(&self) -> Result<(SpriteBatch, Dimming), BatchError> {
        let (atlas, origin) = (self.atlas(), self.view.origin());
        let mut sprites = SpriteBatch::new();
        let mut dimmed = Vec::new();
        let limit = MAX_BATCH_QUADS - usize::from(self.target.is_some() && self.target_frame);
        for floor in self.world.draw_floors() {
            let own_floor = floor == self.world.center().2;
            let start = sprites.len();
            {
                let emit = &mut |(x, y), draw: Draw| {
                    if sprites.len() >= limit {
                        return Ok(());
                    }
                    sprites
                        .push_offset(
                            &self.view,
                            atlas,
                            TileCoord::new(x, y),
                            draw.cell,
                            draw.offset,
                            oteryn_client_domain::world::CELL_PX,
                        )
                        .map(|_| ())
                };
                visit_scene_entities(
                    (origin.x, origin.y),
                    self.anchor.map(|anchor| (anchor.x, anchor.y)),
                    (
                        (self.own.x, self.own.y),
                        if own_floor {
                            self.own_draws
                                .as_deref()
                                .unwrap_or_else(|| self.world.player(self.facing))
                        } else {
                            &[]
                        },
                    ),
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
                    &mut *emit,
                )?;
            }
            if floor < self.world.center().2 && self.floor_separator > 0 && sprites.len() > start {
                dimmed.push((start..sprites.len(), self.floor_separator));
            }
        }
        if let Some(target) = self.target.filter(|_| self.target_frame) {
            sprites.push(&self.view, atlas, target.tile, crate::map::TARGET_CELL)?;
        }
        Ok((sprites, dimmed))
    }
}
