//! Live names and exact session appearance references projected into scene sprites.
use super::*;
use oteryn_session::{EntityDetail, VisibleAppearance};

impl Scene {
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

    pub(crate) fn set_name_visibility(&mut self, visibility: [bool; 3]) {
        self.name_visibility = visibility;
    }
    pub fn name_count(&self) -> usize {
        self.names.len()
    }

    /// Position relative to the actual visible name, fenced by entity generation.
    #[cfg(feature = "bevy-shell")]
    pub(crate) fn typing_position(
        &self,
        indicator: &oteryn_client_ui::player_typing::TypingIndicator,
    ) -> Option<[f32; 2]> {
        let (_, (tile, cells)) =
            self.name_entities
                .iter()
                .zip(&self.names)
                .find(|(entity, _)| {
                    entity.identity == indicator.identity
                        && entity.generation == indicator.generation.get()
                })?;
        let [x, y] = self.view.tile_to_screen(*tile);
        let scale = f32::from(self.label_scale) / 100.;
        let half_width = cells.len() as f32 * 4. * scale;
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
        self.own_draws = None;
        self.names.clear();
        #[cfg(feature = "bevy-shell")]
        self.name_entities.clear();
        if !entries.is_empty() {
            self.cache.begin(self.world.atlas(), root);
        }
        for entry in entries {
            let entity = &entry.entity;
            if !matches!(entity.detail, EntityDetail::Actor { .. }) {
                continue;
            }
            let is_own = self.own_identity == Some(entity.entity.identity);
            if !is_own && entity.position.floor != floor {
                continue;
            }
            let tile = if is_own {
                self.own
            } else {
                TileCoord::new(entity.position.x, entity.position.y)
            };
            let direction = if is_own {
                match self.facing {
                    StepDir::North => 0,
                    StepDir::East | StepDir::NorthEast | StepDir::SouthEast => 1,
                    StepDir::South => 2,
                    StepDir::West | StepDir::NorthWest | StepDir::SouthWest => 3,
                }
            } else {
                match entity.detail {
                    EntityDetail::Actor { direction, .. } => match direction {
                        oteryn_session::StepDirection::North => 0,
                        oteryn_session::StepDirection::East => 1,
                        oteryn_session::StepDirection::South => 2,
                        _ => 3,
                    },
                    _ => 0,
                }
            };
            let draws = self.cache.outfit(entry.look.as_ref(), direction);
            if is_own {
                self.own_draws = Some(draws);
            } else {
                self.actor_draws.push((tile, draws));
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
                if let Some(name) = name.or_else(|| marker.map(|_| tr!("Creature"))) {
                    let label = oteryn_client_ui::forge_creature::label(name, marker);
                    self.names.push((tile, self.cache.name(&label)));
                    #[cfg(feature = "bevy-shell")]
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

#[cfg(feature = "bevy-shell")]
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
