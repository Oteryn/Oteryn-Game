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
        if !entries.is_empty() {
            self.cache.begin(self.world.atlas(), root);
        }
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
                if entity.position.floor == floor
                    && let Some(draws) = self.cache.item(appearance_id, entity.position, quantity)
                {
                    self.object_draws
                        .push((entity.position, item_definition_ref, draws));
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
