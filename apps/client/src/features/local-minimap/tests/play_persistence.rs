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
    // A failed atomic write must also preserve edits and removals of existing marks.
    let temporary = lock(&play.minimap)
        .path()
        .ok_or("active cache")?
        .with_extension("tmp");
    std::fs::create_dir(&temporary)?;
    {
        let mut map = lock(&play.minimap);
        let original = map.markers.clone();
        assert!(map.edit_marker(100, 200, -7, "Lost".into(), 0).is_err());
        assert_eq!(map.markers, original);
        assert!(map.remove_marker(100, 200, -7).is_err());
        assert_eq!(map.markers, original);
    }
    std::fs::remove_dir(temporary)?;
    lock(&play.minimap).remove_marker(100, 200, -7)?;
    assert!(lock(&play.minimap).markers.is_empty());
    let cache = lock(&play.minimap).path().ok_or("active cache")?;
    play.bind_minimap_world("invalid world");
    assert!(
        lock(&play.minimap).cells.is_empty(),
        "a refused binding cannot display another world's exploration"
    );
    assert!(lock(&play.minimap).binding.is_none());
    std::fs::write(&cache, b"malformed")?;
    play.bind_minimap_world(world);
    play.set_live_map(&snapshot(100))?;
    {
        let mut map = lock(&play.minimap);
        assert!(map.save().is_err());
        for _ in 0..2 {
            assert!(map.create_mark_at((100, 200, -7)).is_err());
            assert!(map.markers.is_empty());
        }
        assert_eq!(std::fs::read(cache)?, b"malformed");
    }
    drop(play);
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn dropping_play_closes_and_joins_the_editor_before_reopening_the_cache()
-> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::temp_dir().join(format!("minimap-editor-session-{}", std::process::id()));
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
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn minimap_editor_survives_temporary_play_view_replacement()
-> Result<(), Box<dyn std::error::Error>> {
    let mut play = PlayView::new(TileCoord::new(100, 200), -7, &[])?;
    let shared = play.minimap.clone();
    let (done, finished) = std::sync::mpsc::channel();
    open_editor(&shared, move |stopped| {
        while !stopped.load(std::sync::atomic::Ordering::Acquire) {
            std::thread::park();
        }
        let _ = done.send(());
    });
    drop(play.clone());
    let clone_kept_editor = lock(&shared).editor.is_some();
    play.set_preferences(play.preferences().clone())?;
    let preferences_kept_editor = lock(&shared).editor.is_some();
    play.set_world_viewport([600, 330])?;
    let viewport_kept_editor = lock(&shared).editor.is_some();
    let position = ActorPosition {
        x: 100,
        y: 200,
        floor: -7,
    };
    play.set_live_map(&WorldMapViewSnapshot {
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

#[test]
fn review_stale_minimap_clients_preserve_each_others_markers()
-> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::temp_dir().join(format!(
        "minimap-shared-clients-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    let position = ActorPosition {
        x: 100,
        y: 200,
        floor: -7,
    };
    let snapshot = WorldMapViewSnapshot {
        header: MapViewHeader {
            origin: position,
            first_visible_floor: -7,
            content_generation: [1; 32],
            bundle_digest: [2; 32],
            reset_epoch: 1,
        },
        tiles: vec![MapTile {
            position,
            ground_speed: 100,
            more: false,
            items: vec![],
        }],
    };
    let open = || -> Result<PlayView, oteryn_renderer::BatchError> {
        let mut play = PlayView::new(TileCoord::new(100, 200), -7, &[])?;
        lock(&play.minimap).root = Some(root.clone());
        play.bind_minimap_world("11111111-1111-4111-8111-111111111111");
        play.set_live_map(&snapshot)?;
        Ok(play)
    };
    let (first, second, autosave) = (open()?, open()?, open()?);
    assert!(lock(&first.minimap).create_mark_at((100, 200, -7))?);
    assert!(lock(&second.minimap).create_mark_at((101, 200, -7))?);
    lock(&autosave.minimap).save()?;
    let reopened = open()?;
    assert_eq!(
        lock(&reopened.minimap)
            .markers
            .iter()
            .map(|mark| mark.x)
            .collect::<Vec<_>>(),
        [100, 101]
    );
    lock(&reopened.minimap).remove_marker(100, 200, -7)?;
    lock(&second.minimap).edit_marker(101, 200, -7, "Gate".into(), 2)?;
    let final_view = open()?;
    let map = lock(&final_view.minimap);
    assert_eq!(map.markers.len(), 1);
    assert_eq!(map.markers[0].label, "Gate");
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn review_truncated_live_map_preserves_cached_minimap_palette()
-> Result<(), Box<dyn std::error::Error>> {
    check_live_map_palette(
        false,
        "review_truncated_live_map_preserves_cached_minimap_palette",
    )
}

#[test]
fn review_resolved_top_item_sets_minimap_palette_above_unknown_appearance()
-> Result<(), Box<dyn std::error::Error>> {
    check_live_map_palette(
        true,
        "review_resolved_top_item_sets_minimap_palette_above_unknown_appearance",
    )
}

fn check_live_map_palette(
    unresolved_lower: bool,
    test: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_session::world_map::{MapDefinition, MapItem, MapOrigin};
    let assets = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    if std::env::var_os("OTERYN_TEST_STACK_PALETTE").is_none() {
        let status = std::process::Command::new(std::env::current_exe()?)
            .args([test, "--nocapture"])
            .env("OTERYN_TEST_STACK_PALETTE", "1")
            .env(crate::map::ASSET_DIR_ENV, &assets)
            .status()?;
        assert!(status.success());
        return Ok(());
    }
    // The pinned catalog gives ground 106 colour 24, item 1081 colour 86,
    // and item 102 no automap colour.
    let (ground, top, plain) = (106, 1081, 102);
    let item = |appearance_id| MapItem {
        definition: MapDefinition::Item(std::num::NonZeroU32::MIN),
        count: 1,
        sub_type: 0,
        origin: MapOrigin::DisplayOnly,
        appearance_id,
    };
    let root = std::env::temp_dir().join(format!("minimap-stack-palette-{}", std::process::id()));
    let position = ActorPosition {
        x: 100,
        y: 200,
        floor: -7,
    };
    let lower = if unresolved_lower { 0 } else { ground };
    let mut snapshot = WorldMapViewSnapshot {
        header: MapViewHeader {
            origin: position,
            first_visible_floor: -7,
            content_generation: [1; 32],
            bundle_digest: [2; 32],
            reset_epoch: 1,
        },
        tiles: vec![MapTile {
            position,
            ground_speed: 100,
            more: false,
            items: vec![item(lower), item(top)],
        }],
    };
    let open = || -> Result<PlayView, oteryn_renderer::BatchError> {
        let mut view = PlayView::new(TileCoord::new(100, 200), -7, &[])?;
        lock(&view.minimap).root = Some(root.clone());
        view.bind_minimap_world("11111111-1111-4111-8111-111111111111");
        Ok(view)
    };
    let mut view = open()?;
    view.set_live_map(&snapshot)?;
    assert_eq!(lock(&view.minimap).cells.get(&(-7, 100, 200)), Some(&86));
    lock(&view.minimap).save()?;
    if lower == ground {
        snapshot.tiles[0].items = std::iter::once(item(ground))
            .chain(std::iter::repeat_n(item(plain), 9))
            .collect();
        snapshot.tiles[0].more = true;
        view.set_live_map(&snapshot)?;
        lock(&view.minimap).save()?;
        drop(view);
        let mut rejoined = open()?;
        rejoined.set_live_map(&snapshot)?;
        assert_eq!(
            lock(&rejoined.minimap).cells.get(&(-7, 100, 200)),
            Some(&86)
        );
    }
    std::fs::remove_dir_all(root)?;
    Ok(())
}
