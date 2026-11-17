use super::*;

#[test]
fn srx3_source_landing_solid_ignores_unrelated_walk_restrictions()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (_, mut map, _, _) = super::source_teleport_tests::fixture()?;
    let at = MovementLocalPosition {
        x: 101,
        y: 100,
        floor: 13,
    };
    let mut tile = map.tile(at).ok_or("actual cached tile fixture")?;
    assert_eq!(map.blocks_solid(at), Some(true));
    tile.solid = false;
    tile.projectile = false;
    tile.source_placement.protection_zone = true;
    tile.source_placement.floor_change = true;
    tile.source_placement.teleport = true;
    map.tiles.insert((at.x, at.y, at.floor), Some(tile));
    assert!(!map.admits(at), "ordinary movement stays prohibited");
    assert_eq!(
        map.blocks_solid(at),
        Some(false),
        "source landing reads SOLID only"
    );
    assert_eq!(map.blocks_projectile(at), Some(false));
    Ok(())
}

#[test]
fn srx3_source_landing_solid_never_fills_unknown_or_absent_cells()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (_, mut map, _, _) = super::source_teleport_tests::fixture()?;
    let at = MovementLocalPosition {
        x: 101,
        y: 100,
        floor: 13,
    };
    let mut tile = map.tile(at).ok_or("actual cached tile fixture")?;
    tile.source_placement.exists = false;
    map.tiles.insert((at.x, at.y, at.floor), Some(tile));
    assert_eq!(
        map.blocks_solid(at),
        None,
        "an absent tile cannot receive source landing"
    );
    map.tiles.insert((at.x, at.y, at.floor), None);
    assert_eq!(
        map.blocks_solid(at),
        None,
        "unavailable facts stay unavailable"
    );
    Ok(())
}
