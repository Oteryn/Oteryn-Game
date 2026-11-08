//! Literal source surface truth is separate from conservative path blocking.
use super::*;
type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;
#[test]
fn source_spawn_surface_requires_proven_immovable_facts_and_keeps_known_absence() -> TestResult {
    let (mut runtime, mut map, _, _) = super::source_teleport_tests::fixture()?;
    let at = MovementLocalPosition {
        x: 101,
        y: 100,
        floor: 13,
    };
    let mut tile = map.tile(at).ok_or("retained current tile")?;
    tile.source_placement.exists = true;
    tile.source_placement_proven = true;
    tile.source_placement.immovable_solid = false;
    tile.source_placement.ground = false;
    tile.source_placement.protection_zone = true;
    tile.source_placement.floor_change = true;
    tile.source_placement.teleport = true;
    tile.solid = true;
    tile.projectile = true;
    map.tiles.insert((at.x, at.y, at.floor), Some(tile));
    assert_eq!(
        map.source_spawn_surface_admits(&runtime, at),
        Some(true),
        "literal Lua excludes only IMM_SOLID, not optional movement rules"
    );
    tile.source_placement.immovable_solid = true;
    map.tiles.insert((at.x, at.y, at.floor), Some(tile));
    assert_eq!(map.source_spawn_surface_admits(&runtime, at), Some(false));
    tile.source_placement_proven = false;
    map.tiles.insert((at.x, at.y, at.floor), Some(tile));
    assert_eq!(
        map.source_spawn_surface_admits(&runtime, at),
        None,
        "unknown IMM evidence cannot reject an RNG draw as known blocked"
    );
    tile.source_placement_proven = true;
    tile.source_placement.exists = false;
    map.tiles.insert((at.x, at.y, at.floor), Some(tile));
    assert_eq!(
        map.source_spawn_surface_admits(&runtime, at),
        Some(false),
        "qualified absence is a known nil Tile branch"
    );
    assert_eq!(
        map.source_spawn_surface_admits(
            &runtime,
            MovementLocalPosition {
                x: 999,
                y: 999,
                floor: 13
            }
        ),
        None
    );
    runtime.retire_owner_cycle();
    assert_eq!(
        map.source_spawn_surface_admits(&runtime, at),
        None,
        "retired scope is not a source fact"
    );
    Ok(())
}
