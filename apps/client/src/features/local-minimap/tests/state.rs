use super::*;
#[test]
fn exploration_and_markers_survive_reopen_and_bindings_stay_separate()
-> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::temp_dir().join(format!("minimap-{}", std::process::id()));
    let mut map = Minimap::default();
    map.root = Some(root.clone());
    map.bind_world("11111111-1111-4111-8111-111111111111");
    use oteryn_session::{
        ActorPosition,
        world_map::{MapTile, MapViewHeader, WorldMapViewSnapshot},
    };
    let position = ActorPosition {
        x: 100,
        y: 200,
        floor: -7,
    };
    let mut snapshot = WorldMapViewSnapshot {
        header: MapViewHeader {
            content_generation: [1; 32],
            bundle_digest: [2; 32],
            reset_epoch: 1,
            origin: position,
            first_visible_floor: -7,
        },
        tiles: vec![MapTile {
            position,
            ground_speed: 100,
            more: false,
            items: vec![],
        }],
    };
    map.observe(&snapshot);
    map.edit_marker(100, 200, -7, "Home".into(), 1)?;
    // Legacy item-only cells must not return as terrain when an older cache is reopened.
    map.cells.insert((-7, 999, 999), 86);
    map.dirty = true;
    map.save()?;
    let binding = map.binding;
    let mut restored = Minimap::default();
    restored.root = Some(root.clone());
    restored.bind_world("11111111-1111-4111-8111-111111111111");
    restored.observe(&snapshot);
    assert_eq!(restored.markers[0].label, "Home");
    assert!(restored.cells.contains_key(&(-7, 100, 200)));
    assert!(!restored.cells.contains_key(&(-7, 999, 999)));
    snapshot.header.reset_epoch += 1;
    restored.observe(&snapshot);
    assert!(restored.markers.is_empty());
    assert_ne!(restored.binding, binding);
    drop(restored);
    drop(map);
    std::fs::remove_dir_all(root)?;
    Ok(())
}
#[test]
fn camera_and_marker_validation_do_not_change_gameplay_positions()
-> Result<(), Box<dyn std::error::Error>> {
    let mut map = Minimap::default();
    map.follow(100, 200, -7);
    assert_eq!(map.position(15, 12), Some((100, 200, -7)));
    map.pan(5, -5);
    map.zoom = 4;
    assert_eq!(map.position(16, 12), Some((109, 195, -7)));
    map.floor = -6;
    assert_eq!(map.position(15, 12), Some((105, 195, -6)));
    assert!(map.edit_marker(0, 0, 2, "x".into(), 0).is_err());
    assert!(map.edit_marker(0, 0, -7, "\0".into(), 0).is_err());
    map.edit_marker(100, 200, -7, "Gate".into(), 2)?;
    map.edit_marker(100, 200, -7, "Exit".into(), 3)?;
    assert_eq!(map.markers.len(), 1);
    assert_eq!(map.markers[0].label, "Exit");
    map.remove_marker(100, 200, -7)?;
    assert!(map.markers.is_empty());
    Ok(())
}
#[test]
fn authoritative_window_clears_ground_but_preserves_distant_exploration() {
    use oteryn_session::{
        ActorPosition,
        world_map::{MapTile, MapViewHeader, WorldMapViewSnapshot},
    };
    let snapshot = |x, ground_speed| {
        let position = ActorPosition {
            x,
            y: 200,
            floor: -7,
        };
        WorldMapViewSnapshot {
            header: MapViewHeader {
                content_generation: [1; 32],
                bundle_digest: [2; 32],
                reset_epoch: 1,
                origin: position,
                first_visible_floor: -7,
            },
            tiles: vec![MapTile {
                position,
                ground_speed,
                more: false,
                items: vec![],
            }],
        }
    };
    let mut map = Minimap::default();
    map.observe(&snapshot(100, 100));
    map.observe(&snapshot(1000, 100));
    assert!(map.cells.contains_key(&(-7, 100, 200)));
    assert!(map.cells.contains_key(&(-7, 1000, 200)));
    let mut cleared = snapshot(1000, 100);
    cleared.tiles.clear();
    map.observe(&cleared);
    assert!(map.cells.contains_key(&(-7, 100, 200)));
    assert!(!map.cells.contains_key(&(-7, 1000, 200)));
    map.observe(&snapshot(100, 0));
    assert!(!map.cells.contains_key(&(-7, 100, 200)));
}

#[test]
fn create_mark_hotkey_adds_mark_at_player_position() -> Result<(), Box<dyn std::error::Error>> {
    let mut map = Minimap::default();
    assert!(map.create_mark_at((100, 200, -7))?);
    assert_eq!(map.markers.len(), 1);
    assert_eq!(
        (map.markers[0].x, map.markers[0].y, map.markers[0].floor),
        (100, 200, -7)
    );
    assert_eq!(map.markers[0].label, "Mark");
    Ok(())
}
#[test]
fn create_mark_hotkey_edits_existing_mark() -> Result<(), Box<dyn std::error::Error>> {
    let mut map = Minimap::default();
    map.edit_marker(100, 200, -7, "Gate".into(), 2)?;
    assert!(!map.create_mark_at((100, 200, -7))?);
    assert_eq!(map.markers.len(), 1);
    assert_eq!(map.markers[0].label, "Gate");
    Ok(())
}
