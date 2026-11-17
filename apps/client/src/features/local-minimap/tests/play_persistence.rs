use super::*;
use crate::play::{Input, PlayView};
use oteryn_renderer::TileCoord;
use oteryn_session::{
    ActorPosition,
    world_map::{MapTile, MapViewHeader, WorldMapViewSnapshot},
};

#[test]
fn live_minimap_reopens_exploration_edits_markers_and_isolates_invalid_world_binding()
-> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::temp_dir().join(format!(
        "oteryn-map-caller-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    let snapshot = |x| {
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
                ground_speed: 100,
                more: false,
                items: vec![],
            }],
        }
    };
    let world = "11111111-1111-4111-8111-111111111111";
    {
        let mut play = PlayView::new(TileCoord::new(100, 200), -7, &[])?;
        lock(&play.minimap).root = Some(root.clone());
        play.bind_minimap_world(world);
        play.set_live_map(&snapshot(100))?;
        lock(&play.minimap).edit_marker(100, 200, -7, "Home".into(), 1)?;
        play.set_live_map(&snapshot(1000))?;
        lock(&play.minimap).save()?;
    }
    let mut play = PlayView::new(TileCoord::new(100, 200), -7, &[])?;
    lock(&play.minimap).root = Some(root.clone());
    play.bind_minimap_world(world);
    play.set_live_map(&snapshot(1000))?;
    {
        let mut map = lock(&play.minimap);
        assert!(map.cells.contains_key(&(-7, 100, 200)));
        assert_eq!(map.markers[0].label, "Home");
        map.edit_marker(100, 200, -7, "Return".into(), 2)?;
        assert_eq!(map.markers.len(), 1);
        map.pan(5, 0);
        map.zoom = 4;
        map.floor = -6;
    }
    assert!(play.minimap_click(66.0, 50.0));
    assert!(
        play.inputs.is_empty(),
        "browsing floors cannot move the actor"
    );
    lock(&play.minimap).floor = -7;
    assert!(play.minimap_click(66.0, 50.0));
    assert_eq!(
        play.inputs.pop_front(),
        Some(Input::TargetWalk(ActorPosition {
            x: 109,
            y: 200,
            floor: -7
        }))
    );
    lock(&play.minimap).remove_marker(100, 200, -7)?;
    assert!(lock(&play.minimap).markers.is_empty());
    play.bind_minimap_world("invalid world");
    assert!(
        lock(&play.minimap).cells.is_empty(),
        "a refused binding cannot display another world's exploration"
    );
    assert!(lock(&play.minimap).binding.is_none());
    drop(play);
    std::fs::remove_dir_all(root)?;
    Ok(())
}
