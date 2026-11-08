//! Live names and exact session appearance references projected into scene sprites.
use super::*;
use oteryn_session::{EntityDetail, VisibleAppearance};

impl Scene {
    pub fn name_count(&self) -> usize {
        self.names.len()
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
                oteryn_client_ui::diagonal_movement::facing(self.facing)
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
            if self.view.contains(tile) {
                let mut marker = None;
                oteryn_client_ui::forge_creature::protocol::replace(
                    &mut marker,
                    entity.kind == oteryn_session::EntityKind::Creature,
                    entry.forge_stacks,
                );
                let name = entity
                    .display_name
                    .as_ref()
                    .and_then(|name| name.as_str().ok());
                if let Some(name) = name.or_else(|| marker.map(|_| "Creature")) {
                    let label = oteryn_client_ui::forge_creature::label(name, marker);
                    self.names.push((tile, self.cache.name(&label)));
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
