//! Actual retained map fields; optional Lua restrictions must not leak into the
//! default call, and Primal's different source predicate stays unchanged.
use super::*;
type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;
#[test]
fn srx3_source_is_walkable_default_ignores_optional_restrictions() -> TestResult {
    let (_, mut map, _, _) = super::source_teleport_tests::fixture()?;
    let at = MovementLocalPosition {
        x: 101,
        y: 100,
        floor: 13,
    };
    let mut tile = map.tile(at).ok_or("actual retained tile")?;
    tile.solid = false;
    tile.projectile = false;
    tile.source_placement_proven = true;
    tile.source_placement.ground = true;
    tile.source_placement.immovable_solid = false;
    tile.source_placement.protection_zone = true;
    tile.source_placement.teleport = true;
    tile.source_placement.floor_change = true;
    map.tiles.insert((at.x, at.y, at.floor), Some(tile));
    assert!(!map.admits(at), "normal movement restrictions stay intact");
    assert_eq!(map.source_is_walkable(at), Some(true));
    Ok(())
}
#[test]
fn srx3_source_is_walkable_requires_ground_raw_solid_projectile_and_known_facts() -> TestResult {
    let (_, mut map, _, _) = super::source_teleport_tests::fixture()?;
    let at = MovementLocalPosition {
        x: 101,
        y: 100,
        floor: 13,
    };
    let mut tile = map.tile(at).ok_or("actual retained tile")?;
    tile.source_spawn_admits = true;
    tile.source_placement_proven = true;
    tile.source_placement.exists = true;
    tile.source_placement.ground = true;
    tile.source_placement.immovable_solid = false;
    tile.solid = true;
    tile.projectile = false;
    map.tiles.insert((at.x, at.y, at.floor), Some(tile));
    assert!(
        map.source_spawn_admits(at),
        "Primal immutable-only source predicate is unchanged"
    );
    assert_eq!(
        map.source_is_walkable(at),
        Some(false),
        "movable SOLID still blocks Lua's default call"
    );
    tile.solid = false;
    tile.projectile = true;
    map.tiles.insert((at.x, at.y, at.floor), Some(tile));
    assert_eq!(map.source_is_walkable(at), Some(false));
    tile.projectile = false;
    tile.source_placement.ground = false;
    map.tiles.insert((at.x, at.y, at.floor), Some(tile));
    assert_eq!(map.source_is_walkable(at), Some(false));
    tile.source_placement.ground = true;
    tile.source_placement_proven = false;
    map.tiles.insert((at.x, at.y, at.floor), Some(tile));
    assert_eq!(map.source_is_walkable(at), None);
    tile.source_placement_proven = true;
    tile.source_placement.exists = false;
    map.tiles.insert((at.x, at.y, at.floor), Some(tile));
    assert_eq!(
        map.source_is_walkable(at),
        None,
        "an absent sentinel alone grants no source coverage"
    );
    map.tiles.insert((at.x, at.y, at.floor), None);
    assert_eq!(map.source_is_walkable(at), None);
    Ok(())
}
