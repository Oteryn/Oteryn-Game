use super::*;
use oteryn_client_assets::world::{Budget, MANIFEST_PATH, decode_region};

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

#[test]
#[allow(deprecated)]
fn the_legacy_facade_keeps_the_composed_world_type_and_constants() -> Result<(), BatchError> {
    let world: World = crate::world::World::builtin()?;
    assert_eq!(crate::world::ASSET_DIR_ENV, crate::map::ASSET_DIR_ENV);
    assert_eq!(crate::world::START, START);
    assert_eq!(crate::world::START_FLOOR, START_FLOOR);
    assert_eq!(crate::world::MAX_TILE_DRAWS, MAX_TILE_DRAWS);
    assert_eq!(crate::world::PLAYER_LOOK_TYPE, PLAYER_LOOK_TYPE);
    assert_eq!(crate::world::RADIUS, RADIUS);
    assert_eq!(crate::world::SCAN_MARGIN, SCAN_MARGIN);
    assert_eq!(crate::world::BLACK_CELL, BLACK_CELL);
    assert_eq!(crate::world::MARKER_CELL, MARKER_CELL);
    assert_eq!(crate::world::TARGET_CELL, TARGET_CELL);
    let tile: crate::world::MapTile = MapTile::default();
    let draws: &[crate::world::Draw] = world.player(StepDir::South);
    assert!(tile.under.is_empty());
    assert_eq!(draws[0].cell, MARKER_CELL);
    Ok(())
}
