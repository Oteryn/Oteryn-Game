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
    let fixture = tempfile::tempdir()?;
    let root = fixture.path().join("oteryn-map-caller");
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
    Ok(())
}

#[test]
fn dropping_play_closes_and_joins_the_editor_before_reopening_the_cache()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = tempfile::tempdir()?;
    let root = fixture.path().join("minimap-editor-session");
    let world = "11111111-1111-4111-8111-111111111111";
    let mut play = PlayView::new(TileCoord::new(100, 200), -7, &[])?;
    {
        let mut map = lock(&play.minimap);
        map.root = Some(root.clone());
        map.bind_world(world);
        map.binding = Some(([1; 32], [2; 32], 1));
    }
    let shared = play.minimap.clone();
    let old_map = shared.clone();
    let (ready, started) = std::sync::mpsc::channel();
    let (finished, saved) = std::sync::mpsc::channel();
    open_editor(&shared, move |stopped| {
        let _ = ready.send(());
        while !stopped.load(std::sync::atomic::Ordering::Acquire) {
            std::thread::park();
        }
        let mut map = lock(&old_map);
        let result = map.edit_marker(100, 200, -7, "Old session final save".into(), 0);
        map.editor_open = false;
        let _ = finished.send(result);
    });
    started.recv()?;
    drop(play);
    let closed = lock(&shared).editor.is_none();
    let finished_before_replacement = saved.try_recv();
    // Always release the worker, including on the unfixed tree.
    close_editor(&shared);
    assert!(closed, "ending play must release its editor thread");
    finished_before_replacement??;

    play = PlayView::new(TileCoord::new(100, 200), -7, &[])?;
    {
        let mut map = lock(&play.minimap);
        map.root = Some(root.clone());
        map.bind_world(world);
        map.binding = Some(([1; 32], [2; 32], 1));
        map.load()?;
        assert_eq!(map.markers[0].label, "Old session final save");
        map.edit_marker(101, 200, -7, "New session".into(), 1)?;
    }
    drop(shared);
    let mut restored = Minimap::default();
    restored.root = Some(root.clone());
    restored.bind_world(world);
    restored.binding = Some(([1; 32], [2; 32], 1));
    restored.load()?;
    assert_eq!(restored.markers.len(), 2);
    assert!(
        restored
            .markers
            .iter()
            .any(|marker| marker.label == "New session")
    );
    drop(play);
    drop(restored);
    Ok(())
}

#[test]
fn minimap_editor_survives_temporary_play_view_replacement() -> Result<(), Box<dyn std::error::Error>> {
    let mut play = PlayView::new(TileCoord::new(100, 200), -7, &[])?;
    let shared = play.minimap.clone();
    let (done, finished) = std::sync::mpsc::channel();
    open_editor(&shared, move |stopped| {
        while !stopped.load(std::sync::atomic::Ordering::Acquire) { std::thread::park(); }
        let _ = done.send(());
    });
    drop(play.clone());
    let clone_kept_editor = lock(&shared).editor.is_some();
    play.set_preferences(play.preferences().clone())?;
    let preferences_kept_editor = lock(&shared).editor.is_some();
    play.set_world_viewport([600, 330])?;
    let viewport_kept_editor = lock(&shared).editor.is_some();
    let position = ActorPosition { x: 100, y: 200, floor: -7 };
    play.set_live_map(&WorldMapViewSnapshot {
        header: MapViewHeader { content_generation: [1; 32], bundle_digest: [2; 32], reset_epoch: 1, origin: position, first_visible_floor: -7 },
        tiles: vec![MapTile { position, ground_speed: 100, more: false, items: vec![] }],
    })?;
    let map_kept_editor = lock(&shared).editor.is_some();
    drop(play);
    finished.recv_timeout(Duration::from_secs(2))?;
    assert!(clone_kept_editor);
    assert!(preferences_kept_editor);
    assert!(viewport_kept_editor);
    assert!(map_kept_editor);
    assert!(lock(&shared).editor.is_none());
    Ok(())
}
