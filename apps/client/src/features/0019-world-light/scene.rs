//! Floor separation follows the actual culled painter ranges sent to the renderer.
use super::*;
type FloorQuads = Vec<(std::ops::Range<usize>, i16)>;
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
    pub(super) fn world_sprites(&self) -> Result<(SpriteBatch, Dimming, FloorQuads), BatchError> {
        let (atlas, origin) = (self.atlas(), self.view.origin());
        let mut sprites = SpriteBatch::new();
        let mut dimmed = Vec::new();
        let mut floor_quads = Vec::new();
        let limit = MAX_BATCH_QUADS - usize::from(self.target.is_some() && self.target_frame);
        #[cfg(feature = "bevy-shell")]
        let floors = self.world.all_draw_floors();
        #[cfg(not(feature = "bevy-shell"))]
        let floors = self.world.draw_floors();
        for floor in floors {
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
                                && ((v.kind == TargetKind::Object
                                    && !v.item.is_some_and(|item| {
                                        self.object_draws.iter().any(|(position, reference, _)| {
                                            position.x == tile.x
                                                && position.y == tile.y
                                                && *reference == item.definition()
                                        })
                                    })
                                    && (v.glyph || v.item.is_none()))
                                    || (v.glyph && v.kind != TargetKind::Object && !has_actor))
                        });
                        marker
                            .then_some(Draw {
                                cell: MARKER_CELL,
                                offset: [0, 0],
                            })
                            .into_iter()
                            .chain(
                                self.object_draws
                                    .iter()
                                    .filter(|(position, _, _)| {
                                        position.x == tile.x && position.y == tile.y
                                    })
                                    .flat_map(|(_, _, draws)| draws.iter().copied()),
                            )
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
            floor_quads.push((start..sprites.len(), floor));
            if floor < self.world.center().2 && self.floor_separator > 0 && sprites.len() > start {
                dimmed.push((start..sprites.len(), self.floor_separator));
            }
        }
        if let Some(target) = self.target.filter(|_| self.target_frame) {
            sprites.push(&self.view, atlas, target.tile, crate::map::TARGET_CELL)?;
        }
        Ok((sprites, dimmed, floor_quads))
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
