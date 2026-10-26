use super::*;
include!("tutorial_window_tests.rs");

fn repo() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn the_builtin_world_has_only_the_builtin_cells() -> Result<(), BatchError> {
    let world = World::builtin()?;
    assert_eq!(world.atlas().cell_count() as usize, BUILTIN_CELLS);
    assert_eq!(world.tile_count(), 0);
    assert_eq!(world.player(StepDir::South)[0].cell, MARKER_CELL);
    Ok(())
}

#[test]
fn the_temple_area_resolves_to_real_cells_inside_the_atlas() -> Result<(), String> {
    let world = World::load(&repo())?;
    let cells = world.atlas().cell_count();
    assert!(cells as usize > BUILTIN_CELLS + 100, "{cells} cells");
    assert!(world.tile_count() > 10_000, "{} tiles", world.tile_count());
    let start = world.tile(START.0, START.1).ok_or("no start tile")?;
    assert!(!start.under.is_empty(), "the start tile has ground");
    for tile in world.tiles.values() {
        for draw in tile.under.iter().chain(&tile.over) {
            assert!(draw.cell >= BUILTIN_CELLS as u16 && u32::from(draw.cell) < cells);
            world
                .atlas()
                .uv_rect(draw.cell)
                .map_err(|e| e.to_string())?;
        }
    }
    for facing in FACINGS {
        let player = world.player(facing);
        assert!(!player.is_empty());
        assert!(player.iter().all(|draw| draw.cell >= BUILTIN_CELLS as u16));
    }
    assert_ne!(world.player(StepDir::North), world.player(StepDir::South));
    Ok(())
}

#[test]
fn a_manifest_off_the_pin_is_refused() -> Result<(), String> {
    let dir = std::env::temp_dir().join(format!("oteryn-manifest-pin-{}", std::process::id()));
    let manifest = dir.join(MANIFEST_PATH);
    let parent = manifest.parent().ok_or("manifest has no parent")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    std::fs::write(&manifest, br#"{"files":[]}"#).map_err(|e| e.to_string())?;
    let result = World::load(&dir);
    std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    let error = result.err().ok_or("a substituted manifest loaded")?;
    assert!(
        error.contains("does not match its manifest sha256"),
        "{error}"
    );
    Ok(())
}

#[test]
fn an_encoded_fluid_value_selects_the_fluid_pattern() -> Result<(), String> {
    let root = repo();
    let store = AssetStore::open(
        &root.join("content/assets/files"),
        &root.join("imports/official/client-assets/15.30/manifest.json"),
    )
    .map_err(|e| e.to_string())?;
    let catalog = Catalog::load(&store).map_err(|e| e.to_string())?;
    let index = AppearanceIndex::load(&store, &catalog).map_err(|e| e.to_string())?;
    let sheets = SpriteSheets::new(store, catalog);
    let (x, y) = START;
    assert_eq!(placement(x, y, Some(8), START_FLOOR).sub_type, 8);
    assert_eq!(placement(x, y, None, START_FLOOR).count, 1);
    // Fluid appearance 2524 is on the start area with encoded values 1 and 8. Sub-types 8
    // and 12 share colour 5; sub-type 1 is colour 1.
    let mut resolved = Vec::new();
    for encoded in [1, 8, 12] {
        let entry = index
            .resolve(&sheets, 2524, placement(x, y, Some(encoded), START_FLOOR))
            .map_err(|e| e.to_string())?;
        resolved.push(entry.cells.iter().map(|c| c.sprite_id).collect::<Vec<_>>());
    }
    assert_ne!(resolved[0], resolved[1]);
    assert_eq!(resolved[1], resolved[2]);
    Ok(())
}

#[test]
fn an_item_is_lifted_by_its_own_height_and_the_heights_below_it() {
    // Ground, a height-8 table, an item on it, another height-8 item, a top item of height 8.
    let (lifts, elevation) = stack_lifts([(0, 0), (3, 8), (3, 0), (3, 8), (4, 8)]);
    assert_eq!(lifts, [0, 8, 8, 16, 8]);
    assert_eq!(elevation, 16);
    assert_eq!(item_height(8), 8);
    assert_eq!(item_height(u32::MAX), MAX_ELEVATION);
    assert_eq!(item_height(i32::MAX as u32 + 1), MAX_ELEVATION);
    let (lifts, elevation) = stack_lifts([(3, 16), (3, 16)]);
    assert_eq!(lifts, [16, MAX_ELEVATION]);
    assert_eq!(elevation, MAX_ELEVATION);
}

#[test]
fn a_cell_beyond_the_scan_margin_is_not_drawn() {
    assert!(within_reach([-MAX_REACH_PX, 0]));
    assert!(!within_reach([0, -MAX_REACH_PX - 1]));
}

#[test]
fn a_tile_over_the_draw_cap_is_refused() {
    assert!(within_window_cap(MAX_WINDOW_CELLS).is_ok());
    assert!(within_window_cap(MAX_WINDOW_CELLS + 1).is_err());
    assert!(within_draw_cap(MAX_TILE_DRAWS).is_ok());
    assert!(within_draw_cap(MAX_TILE_DRAWS + 1).is_err());
}

#[test]
fn a_region_that_does_not_match_its_digest_is_refused() -> Result<(), String> {
    let bad = decode_region(
        b"OTRB\x02\x07\x7e\x00\x7d\x00\x01\x00",
        (7, 126, 125),
        (0, 0, 0, 0),
        &mut Budget {
            tiles: 1,
            entries: 1,
        },
    );
    assert!(bad.is_err());
    let missing = World::load(&repo().join("no-such-root"));
    assert!(missing.is_err());
    Ok(())
}
