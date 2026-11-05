//! Live names and exact session appearance references projected into scene sprites.
use super::*;
use oteryn_client_assets::world::{MANIFEST_PATH, MANIFEST_SHA256};
use oteryn_client_assets::{AppearanceIndex, AssetStore, Catalog, SpriteSheets};
use oteryn_session::{EntityDetail, VisibleAppearance};
use std::collections::BTreeMap;

impl Scene {
    pub fn name_count(&self) -> usize {
        self.names.len()
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
        let entries: Vec<_> = entries
            .iter()
            .filter(|entry| entry.entity.position.floor == floor)
            .collect();
        self.actor_draws.clear();
        self.names.clear();
        if entries.is_empty() {
            return self.rebuild_sprites();
        }
        let mut pixels = unpack(self.world.atlas());
        let mut count = self.world.atlas().cell_count() as usize;
        let mut glyphs = BTreeMap::new();
        let mut assets = root.and_then(|root| {
            let store = AssetStore::open_pinned(
                &root.join("content/assets/files"),
                &root.join(MANIFEST_PATH),
                MANIFEST_SHA256,
            )
            .ok()?;
            let catalog = Catalog::load(&store).ok()?;
            let index = AppearanceIndex::load(&store, &catalog).ok()?;
            Some((index, SpriteSheets::new(store, catalog)))
        });
        for entry in entries {
            let entity = &entry.entity;
            let tile = TileCoord::new(entity.position.x, entity.position.y);
            // Explicit EmptyOutfit must suppress the actor, including the local fallback.
            let draws = if matches!(entry.look, Some(oteryn_session::Look::EmptyOutfit)) {
                Some(Vec::new())
            } else if let (
                Some(oteryn_session::Look::Outfit { look_type, .. }),
                EntityDetail::Actor { direction, .. },
                Some((index, sheets)),
            ) = (&entry.look, entity.detail, assets.as_mut())
            {
                let direction = match direction {
                    oteryn_session::StepDirection::North => 0,
                    oteryn_session::StepDirection::East => 1,
                    oteryn_session::StepDirection::South => 2,
                    _ => 3,
                };
                index
                    .resolve_outfit(sheets, *look_type, direction)
                    .ok()
                    .and_then(|resolved| {
                        let mut draws = Vec::new();
                        for cell in resolved.cells {
                            if count >= oteryn_renderer::world::MAX_CELLS {
                                return None;
                            }
                            let rgba = sheets
                                .cell_rgba(cell.sprite_id, cell.cell_x, cell.cell_y)
                                .ok()?;
                            draws.push(Draw {
                                cell: count as u16,
                                offset: [cell.offset_x, cell.offset_y],
                            });
                            pixels.extend(rgba);
                            count += 1;
                        }
                        Some(draws)
                    })
            } else {
                None
            };
            self.actor_draws.push((
                tile,
                draws.unwrap_or_else(|| {
                    vec![Draw {
                        cell: MARKER_CELL,
                        offset: [0, 0],
                    }]
                }),
            ));
            if let Some(name) = entity
                .display_name
                .and_then(|name| name.as_str().ok().map(str::to_owned))
            {
                let mut cells = Vec::new();
                for character in name.chars() {
                    let cell = if let Some(cell) = glyphs.get(&character) {
                        *cell
                    } else {
                        if count >= oteryn_renderer::world::MAX_CELLS {
                            break;
                        }
                        let cell = count as u16;
                        pixels.extend(oteryn_renderer::name_text::glyph_rgba(character));
                        count += 1;
                        glyphs.insert(character, cell);
                        cell
                    };
                    cells.push(cell);
                }
                if self.view.contains(tile) {
                    self.names.push((tile, cells));
                }
            }
        }
        self.presentation_atlas = Some(oteryn_renderer::world::atlas_from_cells(pixels, count)?);
        self.tiles = TileBatch::from_cells(
            &self.view,
            self.atlas(),
            &vec![BLACK_CELL; (SCENE_COLUMNS * SCENE_ROWS) as usize],
        )?;
        self.rebuild_sprites()
    }
}

fn unpack(atlas: &AtlasImage) -> Vec<u8> {
    let px = atlas.cell_px() as usize;
    let columns = atlas.width() as usize / px;
    let mut cells = Vec::new();
    for index in 0..atlas.cell_count() as usize {
        for y in 0..px {
            let at =
                (((index / columns) * px + y) * atlas.width() as usize + index % columns * px) * 4;
            cells.extend_from_slice(&atlas.rgba()[at..at + px * 4]);
        }
    }
    cells
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn live_outfit_uses_server_look_and_empty_outfit_suppresses_pixels() -> Result<(), BatchError> {
        let tile = TileCoord::new(100, 200);
        let mut scene =
            Scene::centered_on(Arc::new(World::builtin()?), None, tile, StepDir::South, &[])?;
        let mut entry = VisibleAppearance {
            entity: oteryn_session::WorldSpatialEntity {
                display_name: Some(
                    oteryn_session::EntityDisplayName::new("Citizen")
                        .map_err(|_| BatchError::InvalidView)?,
                ),
                kind: oteryn_session::EntityKind::Player,
                entity: oteryn_session::EntityRef {
                    identity: [1; 16],
                    generation: 1,
                },
                position: oteryn_session::ActorPosition {
                    x: 100,
                    y: 200,
                    floor: 0,
                },
                detail: EntityDetail::Actor {
                    direction: oteryn_session::StepDirection::South,
                    appearance_ref: 1,
                    health_percent: 100,
                },
            },
            look: Some(oteryn_session::Look::Outfit {
                look_type: 128,
                colours: [0; 4],
                addons: 0,
            }),
        };
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        scene.set_presentations_from(&[entry.clone()], 0, Some(root))?;
        assert!(
            scene.actor_draws[0]
                .1
                .iter()
                .any(|draw| draw.cell > MARKER_CELL)
        );
        assert_eq!(scene.name_count(), 1);
        entry.look = Some(oteryn_session::Look::EmptyOutfit);
        scene.set_presentations_from(&[entry.clone()], 0, None)?;
        assert!(scene.actor_draws[0].1.is_empty());
        entry.entity.position.floor = -1;
        scene.set_presentations_from(&[entry], 0, None)?;
        assert_eq!(scene.name_count(), 0);
        Ok(())
    }
}
