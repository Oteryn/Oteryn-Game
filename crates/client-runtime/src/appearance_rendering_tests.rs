use super::*;
use oteryn_renderer::{SpriteBatch, TileCoord, TileView};
use std::error::Error;
use std::path::PathBuf;
type TestResult = Result<(), Box<dyn Error>>;
// Exact native client fixture pin (apps/client/src/world.rs at this task HEAD).
const MANIFEST_SHA256: &str = "febaff9f4bd7e0f8a029736e446a81f1626805e895e7c2268018e0a9a8493fe4";
fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn assets() -> Result<PinnedOutfitAssets, AppearanceRenderError> {
    let r = fixture_root();
    PinnedOutfitAssets::open(
        &r.join("content/assets/files"),
        &r.join("imports/official/client-assets/15.30/manifest.json"),
        MANIFEST_SHA256,
    )
}

#[test]
fn native_variant_cache_separates_palette_direction_and_addons() -> TestResult {
    use std::collections::HashMap;
    let mut subject = assets()?;
    let palette = [[255; 3]; 133];
    let original = subject.compose(look(), &palette)?;
    let mut changed_palette = palette;
    changed_palette[0] = [0; 3];
    let recoloured = subject.compose(look(), &changed_palette)?;
    assert_ne!(original.atlas.rgba(), recoloured.atlas.rgba());
    let turned = subject.compose(
        OutfitPixels {
            direction: OutfitDirection::East,
            ..look()
        },
        &palette,
    )?;
    let addon = subject.compose(
        OutfitPixels {
            addons: 1,
            ..look()
        },
        &palette,
    )?;
    let mut cache = HashMap::new();
    for frame in [&original, &recoloured, &turned, &addon] {
        assert!(
            cache
                .insert(frame.variant_key.clone(), frame.atlas.clone())
                .is_none()
        );
    }
    assert_eq!(cache.len(), 4);
    assert_eq!(cache.get(&original.variant_key), Some(&original.atlas));
    assert_eq!(cache.get(&recoloured.variant_key), Some(&recoloured.atlas));
    // A second verified asset instance has the same identity; an unused palette entry does
    // not change any selected RGB value or the composed pixels.
    let mut reopened = assets()?;
    assert_eq!(
        reopened.compose(look(), &palette)?.variant_key,
        original.variant_key
    );
    let mut unused_change = palette;
    unused_change[3] = [3, 4, 5];
    let unchanged = subject.compose(look(), &unused_change)?;
    assert_eq!(unchanged.variant_key, original.variant_key);
    assert_eq!(unchanged.atlas, original.atlas);
    Ok(())
}
fn look() -> OutfitPixels {
    OutfitPixels {
        look_type: 128,
        direction: OutfitDirection::South,
        colours: [0, 1, 2, 132],
        addons: 0,
    }
}

#[test]
fn semantic_palette_uses_supplied_content_values_and_all_bounds() -> TestResult {
    // Synthetic palette tests DATA semantics, not an admitted Tibia palette.
    let mut palette = [[1, 2, 3]; 133];
    palette[132] = [4, 5, 6];
    assert_eq!(
        resolve_colours([132, 0, 0, 132], &palette)?,
        [[4, 5, 6], [1, 2, 3], [1, 2, 3], [4, 5, 6]]
    );
    assert!(matches!(
        resolve_colours([133, 0, 0, 0], &palette),
        Err(AppearanceRenderError::InvalidPalette)
    ));
    assert!(resolve_colours([0; 4], &palette[..132]).is_err());
    Ok(())
}
#[test]
fn semantic_direction_addon_and_layer_grammar() -> TestResult {
    assert_eq!(OutfitDirection::South.pattern(1)?, 0);
    assert!(OutfitDirection::North.pattern(1).is_err());
    for width in [0, 2, 3] {
        assert!(OutfitDirection::South.pattern(width).is_err());
    }
    for (i, d) in [
        OutfitDirection::North,
        OutfitDirection::East,
        OutfitDirection::South,
        OutfitDirection::West,
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(d.pattern(4)?, i as u32);
    }
    assert_eq!(enabled_rows(3, 2, 2)?, [0, 2]);
    assert_eq!(enabled_rows(3, 2, 3)?, [0, 1, 2]);
    assert_eq!(enabled_rows(9, 1, 255)?.len(), 9);
    assert!(matches!(
        enabled_rows(3, 2, 4),
        Err(AppearanceRenderError::InvalidAddons)
    ));
    for (h, l) in [(0, 1), (10, 1), (3, 0), (3, 3)] {
        assert!(enabled_rows(h, l, 0).is_err());
    }
    Ok(())
}
#[test]
fn semantic_exact_opaque_masks_select_four_independent_roles() {
    let colours = [[10, 20, 30], [40, 50, 60], [70, 80, 90], [100, 110, 120]];
    for (i, mask) in [
        [255, 255, 0, 255],
        [255, 0, 0, 255],
        [0, 255, 0, 255],
        [0, 0, 255, 255],
    ]
    .into_iter()
    .enumerate()
    {
        let mut rgba = [255, 255, 255, 137];
        tint_mask(&mut rgba, &mask, &colours);
        assert_eq!(&rgba[..3], &colours[i]);
        assert_eq!(rgba[3], 137);
    }
    for mask in [
        [255, 255, 0, 254],
        [255, 255, 0, 0],
        [254, 0, 0, 255],
        [0, 0, 0, 255],
    ] {
        let mut rgba = [17, 31, 47, 255];
        tint_mask(&mut rgba, &mask, &colours);
        assert_eq!(rgba, [17, 31, 47, 255]);
    }
    let mut rgba = [200, 100, 50, 128];
    tint_mask(&mut rgba, &[255, 255, 0, 255], &[[128, 64, 32]; 4]);
    assert_eq!(rgba, [100, 25, 6, 128]);
}
#[test]
fn semantic_alpha_is_ordered_and_transparency_is_preserved() {
    // Synthetic byte controls for standard straight-RGBA alpha-over.
    let mut rgba = [0, 0, 255, 255];
    alpha_over(&mut rgba, &[255, 0, 0, 128]);
    assert_eq!(rgba, [128, 0, 127, 255]);
    alpha_over(&mut rgba, &[1, 2, 3, 0]);
    assert_eq!(rgba, [128, 0, 127, 255]);
    let mut clear = [0; 4];
    alpha_over(&mut clear, &[12, 34, 56, 128]);
    assert_eq!(clear, [12, 34, 56, 128]);
    alpha_over(&mut clear, &[65, 43, 21, 255]);
    assert_eq!(clear, [65, 43, 21, 255]);
}

#[test]
fn native_white_palette_matches_actual_native_base_resolver_four_directions() -> TestResult {
    let r = fixture_root();
    let store = AssetStore::open_pinned(
        &r.join("content/assets/files"),
        &r.join("imports/official/client-assets/15.30/manifest.json"),
        MANIFEST_SHA256,
    )?;
    let catalog = Catalog::load(&store)?;
    let index = AppearanceIndex::load(&store, &catalog)?;
    let mut sheets = SpriteSheets::new(store, catalog);
    let mut subject = assets()?;
    // White is a synthetic control palette: tint is identity, allowing an independent
    // full-pixel comparison against the actual existing native base-only resolver.
    for (d, direction) in [
        OutfitDirection::North,
        OutfitDirection::East,
        OutfitDirection::South,
        OutfitDirection::West,
    ]
    .into_iter()
    .enumerate()
    {
        let rendered = subject.compose(
            OutfitPixels {
                direction,
                ..look()
            },
            &[[255; 3]; 133],
        )?;
        let native = index.resolve_outfit(&sheets, 128, d as u32)?;
        let mut expected = vec![0; rendered.atlas.rgba().len()];
        let width = rendered.atlas.width() as usize;
        for cell in native.cells {
            let pixels = sheets.cell_rgba(cell.sprite_id, cell.cell_x, cell.cell_y)?;
            let x = usize::try_from(cell.offset_x - rendered.offset[0])?;
            let y = usize::try_from(cell.offset_y - rendered.offset[1])?;
            for row in 0..32 {
                let to = ((y + row) * width + x) * 4;
                expected[to..to + 128].copy_from_slice(&pixels[row * 128..(row + 1) * 128]);
            }
        }
        // RGB under alpha0 has no visible meaning. Alpha-over onto a clear canvas
        // canonicalizes it to0; the independent source-cell oracle must do likewise.
        for pixel in expected.chunks_exact_mut(4) {
            if pixel[3] == 0 {
                pixel.fill(0);
            }
        }
        assert_eq!(rendered.atlas.rgba(), expected);
        let decoded = subject.sheet_decode_count();
        assert_eq!(
            subject.compose(
                OutfitPixels {
                    direction,
                    ..look()
                },
                &[[255; 3]; 133]
            )?,
            rendered
        );
        assert_eq!(subject.sheet_decode_count(), decoded);
    }
    Ok(())
}

#[test]
fn native_composed_cells_match_renderer_placement_and_culling_with_elevation() -> TestResult {
    let r = fixture_root();
    let store = AssetStore::open_pinned(
        &r.join("content/assets/files"),
        &r.join("imports/official/client-assets/15.30/manifest.json"),
        MANIFEST_SHA256,
    )?;
    let catalog = Catalog::load(&store)?;
    let index = AppearanceIndex::load(&store, &catalog)?;
    let appearance = index.outfit(128).ok_or("actual look128 absent")?;
    assert!(
        appearance.displacement_x > 0 || appearance.displacement_y > 0,
        "the native fixture must distinguish displacement signs"
    );
    let mut sheets = SpriteSheets::new(store, catalog);
    let mut subject = assets()?;
    let (mut drawn, mut culled) = (0, 0);
    for (direction_index, direction) in [
        OutfitDirection::North,
        OutfitDirection::East,
        OutfitDirection::South,
        OutfitDirection::West,
    ]
    .into_iter()
    .enumerate()
    {
        let composed = subject.compose(
            OutfitPixels {
                direction,
                ..look()
            },
            &[[255; 3]; 133],
        )?;
        let native = index.resolve_outfit(&sheets, 128, direction_index as u32)?;
        assert_eq!(native.cells.len(), composed.atlas.cell_count() as usize);
        let columns = composed.atlas.width() / CELL_PX;
        for cell in native.cells {
            let native_atlas = AtlasImage::new(
                CELL_PX,
                1,
                1,
                sheets.cell_rgba(cell.sprite_id, cell.cell_x, cell.cell_y)?,
            )?;
            let composed_cell = u16::try_from(cell.cell_y * columns + cell.cell_x)?;
            for tile_px in [16, 32, 64] {
                let view = TileView::new(TileCoord::new(100, -40), tile_px, 8, 8)?;
                for tile in [
                    TileCoord::new(103, -37),
                    TileCoord::new(100, -40),
                    TileCoord::new(99, -41),
                    TileCoord::new(108, -32),
                ] {
                    for elevation in [0, 24] {
                        // The oracle is the existing native resolver's DrawCell, passed
                        // through the actual renderer. No anchor/displacement formula is
                        // reproduced here. Elevation follows Scene's caller contract.
                        let mut expected = SpriteBatch::new();
                        let native_drawn = expected.push_offset(
                            &view,
                            &native_atlas,
                            tile,
                            0,
                            [cell.offset_x - elevation, cell.offset_y - elevation],
                            CELL_PX,
                        )?;
                        let mut actual = SpriteBatch::new();
                        let composed_drawn = actual.push_offset(
                            &view,
                            &composed.atlas,
                            tile,
                            composed_cell,
                            [
                                composed.offset[0] + (cell.cell_x * CELL_PX) as i32 - elevation,
                                composed.offset[1] + (cell.cell_y * CELL_PX) as i32 - elevation,
                            ],
                            CELL_PX,
                        )?;
                        assert_eq!(composed_drawn, native_drawn);
                        assert_eq!(actual.len(), expected.len());
                        for (actual, expected) in
                            actual.instances().iter().zip(expected.instances())
                        {
                            assert_eq!(actual.position, expected.position);
                            assert_eq!(actual.size, expected.size);
                        }
                        if native_drawn {
                            drawn += 1;
                        } else {
                            culled += 1;
                        }
                    }
                }
            }
        }
    }
    assert!(drawn > 0 && culled > 0);
    Ok(())
}

#[test]
fn native_exact_mask_pixels_tint_actual_hash_verified_base_cells() -> TestResult {
    let r = fixture_root();
    let store = AssetStore::open_pinned(
        &r.join("content/assets/files"),
        &r.join("imports/official/client-assets/15.30/manifest.json"),
        MANIFEST_SHA256,
    )?;
    let catalog = Catalog::load(&store)?;
    let index = AppearanceIndex::load(&store, &catalog)?;
    let a = index.outfit(128).ok_or("actual look128 absent")?;
    assert_eq!(a.layers, 2);
    let base = a.sprite_ids[(2 * a.layers) as usize];
    let mask = a.sprite_ids[(2 * a.layers + 1) as usize];
    let mut sheets = SpriteSheets::new(store, catalog);
    let (w, h) = sheets.layout(base)?.cells();
    let mut subject = assets()?;
    let rendered = subject.compose(look(), &[[128, 64, 32]; 133])?;
    let mut roles = [0; 4];
    for cy in 0..h {
        for cx in 0..w {
            let b = sheets.cell_rgba(base, cx, cy)?;
            let m = sheets.cell_rgba(mask, cx, cy)?;
            for i in 0..1024 {
                let role = match &m[i * 4..i * 4 + 4] {
                    [255, 255, 0, 255] => 0,
                    [255, 0, 0, 255] => 1,
                    [0, 255, 0, 255] => 2,
                    [0, 0, 255, 255] => 3,
                    _ => continue,
                };
                if b[i * 4 + 3] != 255 {
                    continue;
                }
                let x = cx * 32 + (i % 32) as u32;
                let y = cy * 32 + (i / 32) as u32;
                let at = ((y * rendered.atlas.width() + x) * 4) as usize;
                assert_eq!(
                    &rendered.atlas.rgba()[at..at + 3],
                    &[
                        (u16::from(b[i * 4]) * 128 / 255) as u8,
                        (u16::from(b[i * 4 + 1]) * 64 / 255) as u8,
                        (u16::from(b[i * 4 + 2]) * 32 / 255) as u8
                    ]
                );
                roles[role] += 1;
            }
        }
    }
    assert!(
        roles.iter().all(|n| *n > 0),
        "actual citizen fixture must exercise all four opaque mask roles"
    );
    Ok(())
}
#[test]
fn native_addons_and_failed_requests_return_no_substituted_pixels() -> TestResult {
    let mut subject = assets()?;
    let palette = [[255; 3]; 133];
    let base = subject.compose(look(), &palette)?;
    let addon1 = subject.compose(
        OutfitPixels {
            addons: 1,
            ..look()
        },
        &palette,
    )?;
    let addon2 = subject.compose(
        OutfitPixels {
            addons: 2,
            ..look()
        },
        &palette,
    )?;
    let both = subject.compose(
        OutfitPixels {
            addons: 3,
            ..look()
        },
        &palette,
    )?;
    assert_ne!(base.atlas.rgba(), addon1.atlas.rgba());
    assert_ne!(base.atlas.rgba(), addon2.atlas.rgba());
    assert_ne!(both.atlas.rgba(), addon1.atlas.rgba());
    assert!(matches!(
        subject.compose(
            OutfitPixels {
                look_type: 0,
                ..look()
            },
            &palette
        ),
        Err(AppearanceRenderError::UnknownLook)
    ));
    assert!(matches!(
        subject.compose(
            OutfitPixels {
                addons: 4,
                ..look()
            },
            &palette
        ),
        Err(AppearanceRenderError::InvalidAddons)
    ));
    assert!(matches!(
        subject.compose(
            OutfitPixels {
                colours: [133; 4],
                ..look()
            },
            &palette
        ),
        Err(AppearanceRenderError::InvalidPalette)
    ));
    assert_eq!(subject.compose(look(), &palette)?, base);
    Ok(())
}
#[test]
fn native_open_requires_the_independent_exact_manifest_pin() {
    let r = fixture_root();
    let result = PinnedOutfitAssets::open(
        &r.join("content/assets/files"),
        &r.join("imports/official/client-assets/15.30/manifest.json"),
        &"0".repeat(64),
    );
    assert!(matches!(
        result,
        Err(AppearanceRenderError::Asset(
            AssetError::HashMismatch { .. }
        ))
    ));
}
