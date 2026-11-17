//! Live names and exact session appearance references projected into scene sprites.
use super::*;
use oteryn_session::{EntityDetail, VisibleAppearance};

impl Scene {
    pub(crate) fn item_multi_use(&mut self, id: u16) -> bool {
        self.cache.begin(
            self.world.atlas(),
            std::env::var_os(crate::map::ASSET_DIR_ENV).map(std::path::PathBuf::from),
        );
        self.cache.item_multi_use(id)
    }

    pub(crate) fn set_outfit_palette(&mut self, palette: &[[u8; 3]]) -> bool {
        self.cache.set_palette(palette)
    }

    pub(crate) fn set_label_scale(&mut self, percentage: u16) {
        self.label_scale = percentage;
    }
    pub(crate) fn own_identity(&self) -> Option<[u8; 16]> {
        self.own_identity
    }
    pub(crate) fn set_name_options(&mut self, marks: bool, npc_names: bool) {
        self.name_marks = marks;
        self.npc_names = npc_names;
    }

    /// Tibia's game font and the atlas cells of its HUD flags, once the bundle loaded.
    pub(crate) fn hud_graphics(
        &self,
    ) -> (
        Option<Arc<oteryn_client_assets::tibia_graphics::TibiaGraphics>>,
        Option<super::HudIconCells>,
    ) {
        self.cache.hud_graphics()
    }

    pub(crate) fn set_name_visibility(&mut self, visibility: [bool; 3]) {
        self.name_visibility = visibility;
    }
    pub fn name_count(&self) -> usize {
        self.names.len()
    }

    /// Position relative to the actual visible name, fenced by entity generation.
    #[cfg(bevy_shell)]
    pub(crate) fn typing_position(
        &self,
        indicator: &oteryn_client_ui::player_typing::TypingIndicator,
    ) -> Option<[f32; 2]> {
        let (entity, (tile, label)) =
            self.name_entities
                .iter()
                .zip(&self.names)
                .find(|(entity, _)| {
                    entity.identity == indicator.identity
                        && entity.generation == indicator.generation.get()
                })?;
        let [x, y] = self.view.tile_to_screen(*tile);
        let [dx, dy] = self.actor_offset(*entity);
        let (x, y) = (x + dx, y + dy);
        let scale = f32::from(self.label_scale) / 100.;
        let half_width = label.width / 2. * scale;
        let left = (x + self.view.tile_px() as f32 / 2. - half_width).max(0.);
        Some([left + half_width + 2., (y - 14. * scale).max(0.) - 4.])
    }

    pub(crate) fn set_own_identity(&mut self, identity: [u8; 16]) {
        self.own_identity = Some(identity);
    }

    /// Recenter while retaining assets, outfit/glyph cells and the packed atlas.
    pub(crate) fn recenter_with(
        &mut self,
        world: Arc<World>,
        anchor: Option<TileCoord>,
        own: TileCoord,
        facing: StepDir,
        markers: &[TileCoord],
        entities: &[Targetable],
    ) -> Result<(), BatchError> {
        let same_world = Arc::ptr_eq(&world, &self.world);
        let mut next = Self::centered_with(world, anchor, own, facing, markers, entities)?;
        next.cache = std::mem::take(&mut self.cache);
        if !same_world {
            next.cache.reset_atlas();
        }
        next.own_identity = self.own_identity;
        next.drome_hazards = std::mem::take(&mut self.drome_hazards);
        next.actor_clocks = self.actor_clocks.clone();
        *self = next;
        Ok(())
    }

    pub fn set_presentations(
        &mut self,
        entries: &[VisibleAppearance],
        floor: i16,
    ) -> Result<(), BatchError> {
        self.set_presentations_from(
            entries,
            floor,
            std::env::var_os(crate::map::ASSET_DIR_ENV).map(std::path::PathBuf::from),
        )
    }

    fn set_presentations_from(
        &mut self,
        entries: &[VisibleAppearance],
        floor: i16,
        root: Option<std::path::PathBuf>,
    ) -> Result<(), BatchError> {
        self.actor_draws.clear();
        self.object_draws.clear();
        self.own_draws = None;
        self.names.clear();
        self.name_entities.clear();
        if !entries.is_empty() {
            self.cache.begin(self.world.atlas(), root);
        }
        self.cache.retain_outfit_actors(entries);
        self.actor_clocks
            .entries
            .retain(|entity, _| entries.iter().any(|entry| entry.entity.entity == *entity));
        for entry in entries {
            let entity = oteryn_client_ui::native_creature_icons::project(&entry.entity);
            if let EntityDetail::ObjectWithAppearance {
                item_definition_ref,
                quantity,
                appearance_id,
                item_handle,
            } = entity.detail
            {
                if !matches!(
                    entity.kind,
                    oteryn_session::EntityKind::Corpse | oteryn_session::EntityKind::GroundItem
                ) || entity.entity.generation != 0
                    || item_definition_ref == 0
                    || quantity == 0
                    || item_handle.is_none()
                {
                    continue;
                }
                let shift = i32::from(entity.position.floor) - i32::from(floor);
                let draw_floor = if shift == 0 {
                    i32::from(floor)
                } else {
                    i32::from(self.world.center().2) + shift
                };
                if let (Ok(draw_floor), Some(x), Some(y)) = (
                    i16::try_from(draw_floor),
                    entity.position.x.checked_sub(shift),
                    entity.position.y.checked_sub(shift),
                ) && (shift == 0 || self.world.draw_floors().contains(&draw_floor))
                    && let Some(draws) =
                        self.cache
                            .world_item(appearance_id, entity.position, quantity)
                {
                    let projected = oteryn_session::ActorPosition {
                        x,
                        y,
                        floor: draw_floor,
                    };
                    self.object_draws
                        .push((projected, item_definition_ref, draws));
                }
                continue;
            }
            if !matches!(
                entity.detail,
                EntityDetail::Actor { .. } | EntityDetail::ActorWithIcons { .. }
            ) {
                continue;
            }
            let is_own = self.own_identity == Some(entity.entity.identity);
            let draw_floor = self
                .world
                .center()
                .2
                .saturating_add(entity.position.floor.saturating_sub(floor));
            if !is_own
                && entity.position.floor != floor
                && !self.world.draw_floors().contains(&draw_floor)
            {
                continue;
            }
            let tile = if is_own {
                self.own
            } else {
                let shift = i32::from(entity.position.floor) - i32::from(floor);
                TileCoord::new(
                    entity.position.x.saturating_sub(shift),
                    entity.position.y.saturating_sub(shift),
                )
            };
            let direction = if is_own {
                oteryn_client_ui::diagonal_movement::facing(self.facing)
            } else {
                match entity.detail {
                    EntityDetail::Actor { direction, .. }
                    | EntityDetail::ActorWithIcons { direction, .. } => match direction {
                        oteryn_session::StepDirection::North => 0,
                        oteryn_session::StepDirection::East => 1,
                        oteryn_session::StepDirection::South => 2,
                        _ => 3,
                    },
                    _ => 0,
                }
            };
            let (gait, offset) = self.actor_clocks.sample(entity, is_own);
            let mount = entity.motion.map_or(0, |motion| motion.mount_look_type);
            let mut draws = self.cache.animated_outfit(
                entry.look.as_ref(),
                direction,
                gait,
                mount,
                Some(entity.entity),
            );
            if !is_own {
                for draw in &mut draws {
                    draw.offset[0] += offset[0].round() as i32;
                    draw.offset[1] += offset[1].round() as i32;
                }
            }
            if is_own {
                self.own_draws = Some(draws);
            } else {
                self.actor_draws.push(ActorDraw {
                    floor: draw_floor,
                    tile,
                    draws,
                });
            }
            if entity.position.floor != floor {
                continue;
            }
            let name_group = if is_own {
                0
            } else if entity.kind == oteryn_session::EntityKind::Player {
                1
            } else {
                2
            };
            let visible = if entity.kind == oteryn_session::EntityKind::Npc {
                self.npc_names
            } else {
                self.name_visibility[name_group]
            };
            if self.view.contains(tile) && visible {
                let mut marker = None;
                oteryn_client_ui::forge_creature::protocol::replace(
                    &mut marker,
                    entity.kind == oteryn_session::EntityKind::Creature && self.name_marks,
                    entry.forge_stacks,
                );
                let name = entity
                    .display_name
                    .as_ref()
                    .and_then(|name| name.as_str().ok());
                let mut label = name
                    .or_else(|| marker.map(|_| tr!("Creature")))
                    .map(|name| oteryn_client_ui::forge_creature::label(name, marker))
                    .unwrap_or_default();
                let icons = oteryn_client_ui::native_creature_icons::labels(entity).join(" ");
                if !icons.is_empty() {
                    if !label.is_empty() {
                        label.push(' ');
                    }
                    label.push_str(&icons);
                }
                if !label.is_empty() {
                    self.names.push((
                        tile,
                        self.cache.name(
                            &label,
                            match entity.detail {
                                EntityDetail::Actor { health_percent, .. }
                                | EntityDetail::ActorWithIcons { health_percent, .. } => {
                                    health_percent
                                }
                                _ => 100,
                            },
                        ),
                    ));
                    self.name_entities.push(entity.entity);
                }
            }
        }
        if !entries.is_empty() {
            self.presentation_atlas = Some(self.cache.image()?);
        }
        self.tiles = TileBatch::from_cells(
            &self.view,
            self.atlas(),
            &vec![BLACK_CELL; (SCENE_COLUMNS * SCENE_ROWS) as usize],
        )?;
        self.rebuild_sprites()
    }
}

#[cfg(test)]
#[path = "tests/scene.rs"]
mod tests;

#[cfg(bevy_shell)]
impl Scene {
    pub(crate) fn item_icon(&self, item: &oteryn_session::ItemEntry) -> Option<&[Draw]> {
        self.cache.item(item)
    }

    pub(crate) fn prepare_item_icons<'a>(
        &mut self,
        items: impl Iterator<Item = &'a oteryn_session::ItemEntry>,
    ) -> Result<bool, BatchError> {
        let mut items = items.filter(|item| item.appearance_id != 0).peekable();
        if items.peek().is_none() {
            return Ok(false);
        }
        // begin retains the already loaded asset index and sprite sheets.
        self.cache.begin(
            self.world.atlas(),
            std::env::var_os(crate::map::ASSET_DIR_ENV).map(std::path::PathBuf::from),
        );
        for item in items {
            self.cache.prepare_item(item);
        }
        let image = self.cache.image()?;
        if self
            .presentation_atlas
            .as_ref()
            .is_none_or(|old| !Arc::ptr_eq(old, &image))
        {
            self.presentation_atlas = Some(image);
            self.lit_atlas = None;
            self.tiles = oteryn_renderer::TileBatch::from_cells(
                &self.view,
                self.atlas(),
                &vec![
                    crate::map::BLACK_CELL;
                    (crate::scene::SCENE_COLUMNS * crate::scene::SCENE_ROWS) as usize
                ],
            )?;
            self.rebuild_sprites()?;
            return Ok(true);
        }
        Ok(false)
    }
}
