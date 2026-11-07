// Test-only oracle over real server look/palette and independently pinned official cells.
struct SourceOutfitPixels {
    index: oteryn_client_assets::AppearanceIndex,
    sheets: oteryn_client_assets::SpriteSheets,
}
impl SourceOutfitPixels {
    fn load() -> Result<Self, HarePlayError> {
        use oteryn_client_assets::world::{MANIFEST_PATH, MANIFEST_SHA256};
        use oteryn_client_assets::{AppearanceIndex, AssetStore, Catalog, SpriteSheets};
        let root = std::path::PathBuf::from(
            std::env::var_os(crate::map::ASSET_DIR_ENV)
                .ok_or("actual outfit pixels require pinned OTERYN_ASSET_DIR")?,
        );
        let store = AssetStore::open_pinned(
            &root.join("content/assets/files"),
            &root.join(MANIFEST_PATH),
            MANIFEST_SHA256,
        )?;
        let catalog = Catalog::load(&store)?;
        let index = AppearanceIndex::load(&store, &catalog)?;
        Ok(Self {
            index,
            sheets: SpriteSheets::new(store, catalog),
        })
    }
    fn check(
        &mut self,
        view: &PlayView,
        identity: [u8; 16],
        own: bool,
        palette: &[[u8; 3]],
    ) -> Result<Vec<String>, HarePlayError> {
        use oteryn_session::{EntityDetail, Look, StepDirection};
        let entry = view
            .captured_appearance(identity)
            .ok_or("actual rendered outfit entity")?;
        let Some(Look::Outfit {
            look_type,
            colours,
            addons,
        }) = entry.look.clone()
        else {
            return Err("actual source outfit required".into());
        };
        let direction = if own {
            match view.facing() {
                crate::input::StepDir::North => 0,
                crate::input::StepDir::East => 1,
                crate::input::StepDir::South => 2,
                crate::input::StepDir::West => 3,
            }
        } else {
            match entry.entity.detail {
                EntityDetail::Actor { direction, .. } => match direction {
                    StepDirection::North => 0,
                    StepDirection::East => 1,
                    StepDirection::South => 2,
                    StepDirection::West => 3,
                },
                _ => return Err("actual source actor detail".into()),
            }
        };
        let info = self
            .index
            .outfit(look_type)
            .ok_or("actual source outfit exists in current official assets")?
            .clone();
        let tile =
            oteryn_renderer::TileCoord::new(entry.entity.position.x, entry.entity.position.y);
        assert_eq!(entry.entity.position.floor, view.map_center().2);
        assert!(view.scene().view().contains(tile));
        let at = view.scene().view().tile_to_screen(tile);
        let scale = view.scene().view().tile_px() as f32 / 32.;
        let mut missing = Vec::new();
        let mut opaque = 0;
        for pattern in 0..info.pattern_height {
            if pattern != 0 && (pattern > 2 || addons & (1 << (pattern - 1)) == 0) {
                continue;
            }
            let index = ((pattern * info.pattern_width + direction % info.pattern_width)
                * info.layers) as usize;
            let sprite = *info
                .sprite_ids
                .get(index)
                .ok_or("actual official base/addon sprite")?;
            if sprite == 0 {
                continue;
            }
            let mask = if info.layers > 1 {
                *info
                    .sprite_ids
                    .get(index + 1)
                    .ok_or("actual official mask sprite")?
            } else {
                0
            };
            let (width, height) = self.sheets.layout(sprite)?.cells();
            for cy in 0..height {
                for cx in 0..width {
                    let mut expected = self.sheets.cell_rgba(sprite, cx, cy)?;
                    if mask != 0 {
                        let masks = self.sheets.cell_rgba(mask, cx, cy)?;
                        for (pixel, mask) in expected
                            .as_chunks_mut::<4>()
                            .0
                            .iter_mut()
                            .zip(masks.as_chunks::<4>().0.iter())
                        {
                            if pixel[3] == 0 || mask[3] == 0 {
                                continue;
                            }
                            let part = match mask[..3] {
                                [255, 255, 0] => Some(0),
                                [255, 0, 0] => Some(1),
                                [0, 255, 0] => Some(2),
                                [0, 0, 255] => Some(3),
                                _ => None,
                            };
                            if let Some(part) = part {
                                let rgb = palette
                                    .get(usize::from(colours[part]))
                                    .ok_or("actual captured palette index")?;
                                for channel in 0..3 {
                                    pixel[channel] =
                                        ((u16::from(pixel[channel]) * u16::from(rgb[channel]))
                                            / 255) as u8;
                                }
                            }
                        }
                    }
                    if !expected.as_chunks::<4>().0.iter().any(|p| p[3] != 0) {
                        continue;
                    }
                    opaque += 1;
                    let offset = [
                        (cx as i32 + 1 - width as i32) * 32 - info.displacement_x as i32,
                        (cy as i32 + 1 - height as i32) * 32 - info.displacement_y as i32,
                    ];
                    let position = [
                        at[0] + offset[0] as f32 * scale,
                        at[1] + offset[1] as f32 * scale,
                    ];
                    if !source_quad_pixels(view.scene(), position, &expected) {
                        missing.push(format!("look{look_type} pattern{pattern} sprite{sprite} cell{cx},{cy} sourceColours{colours:?} addons{addons} officialRGBA absent"));
                    }
                }
            }
        }
        assert!(opaque > 0, "actual official outfit must have pixels");
        eprintln!(
            "SRV7_SOURCE_OUTFIT_PIXELS look={look_type} colours={colours:?} addons={addons} opaque_cells={opaque} failures={}",
            missing.len()
        );
        Ok(missing)
    }
}
fn source_quad_pixels(scene: &crate::scene::Scene, position: [f32; 2], expected: &[u8]) -> bool {
    let atlas = scene.atlas();
    scene
        .sprites()
        .instances()
        .iter()
        .filter(|q| q.position == position)
        .any(|q| {
            let x = (q.uv[0] * atlas.width() as f32).round() as usize;
            let y = (q.uv[1] * atlas.height() as f32).round() as usize;
            let w = ((q.uv[2] - q.uv[0]) * atlas.width() as f32).round() as usize;
            let h = ((q.uv[3] - q.uv[1]) * atlas.height() as f32).round() as usize;
            w == 32
                && h == 32
                && (0..32).all(|row| {
                    let start = ((y + row) * atlas.width() as usize + x) * 4;
                    atlas.rgba().get(start..start + 128) == expected.get(row * 128..row * 128 + 128)
                })
        })
}
