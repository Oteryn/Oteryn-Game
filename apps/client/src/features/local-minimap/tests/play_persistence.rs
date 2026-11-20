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
            floor: 7,
        };
        WorldMapViewSnapshot {
            header: MapViewHeader {
                content_generation: [1; 32],
                bundle_digest: [2; 32],
                reset_epoch: 1,
                origin: position,
                first_visible_floor: 7,
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
        let mut play = PlayView::new(TileCoord::new(100, 200), 7, &[])?;
        lock(&play.minimap).root = Some(root.clone());
        play.bind_minimap_world(world);
        play.set_live_map(&snapshot(100))?;
        lock(&play.minimap).edit_marker(100, 200, 7, "Home".into(), 1)?;
        play.set_live_map(&snapshot(1000))?;
        lock(&play.minimap).save()?;
    }
    let mut play = PlayView::new(TileCoord::new(100, 200), 7, &[])?;
    lock(&play.minimap).root = Some(root.clone());
    play.bind_minimap_world(world);
    play.set_live_map(&snapshot(1000))?;
    {
        let mut map = lock(&play.minimap);
        assert!(map.cells.contains_key(&(7, 100, 200)));
        assert_eq!(map.markers[0].label, "Home");
        map.edit_marker(100, 200, 7, "Return".into(), 2)?;
        assert_eq!(map.markers.len(), 1);
        map.pan(5, 0);
        map.zoom = 4;
        map.floor = 6;
    }
    assert!(play.minimap_click(66.0, 50.0));
    assert!(
        play.inputs.is_empty(),
        "browsing floors cannot move the actor"
    );
    lock(&play.minimap).floor = 7;
    assert!(play.minimap_click(66.0, 50.0));
    assert_eq!(
        play.inputs.pop_front(),
        Some(Input::TargetWalk(ActorPosition {
            x: 109,
            y: 200,
            floor: 7
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
        assert!(map.edit_marker(100, 200, 7, "Lost".into(), 0).is_err());
        assert_eq!(map.markers, original);
        assert!(map.remove_marker(100, 200, 7).is_err());
        assert_eq!(map.markers, original);
    }
    std::fs::remove_dir(temporary)?;
    lock(&play.minimap).remove_marker(100, 200, 7)?;
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
            assert!(map.create_mark_at((100, 200, 7)).is_err());
            assert!(map.markers.is_empty());
        }
        assert_eq!(std::fs::read(cache)?, b"malformed");
    }
    drop(play);
    Ok(())
}

#[test]
fn dropping_play_closes_and_joins_the_editor_before_reopening_the_cache()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = tempfile::tempdir()?;
    let root = fixture.path().join("minimap-editor-session");
    let world = "11111111-1111-4111-8111-111111111111";
    let mut play = PlayView::new(TileCoord::new(100, 200), 7, &[])?;
    {
        let mut map = lock(&play.minimap);
        map.root = Some(root.clone());
        map.bind_world(world);
        map.binding = Some(([1; 32], [2; 32], 1));
        map.load()?;
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
        let result = map.edit_marker(100, 200, 7, "Old session final save".into(), 0);
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

    drop(shared);
    play = PlayView::new(TileCoord::new(100, 200), 7, &[])?;
    {
        let mut map = lock(&play.minimap);
        map.root = Some(root.clone());
        map.bind_world(world);
        map.binding = Some(([1; 32], [2; 32], 1));
        map.load()?;
        assert_eq!(map.markers[0].label, "Old session final save");
        map.edit_marker(101, 200, 7, "New session".into(), 1)?;
    }
    drop(play);
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
    drop(restored);
    Ok(())
}

#[test]
fn minimap_editor_survives_temporary_play_view_replacement()
-> Result<(), Box<dyn std::error::Error>> {
    let mut play = PlayView::new(TileCoord::new(100, 200), 7, &[])?;
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
        floor: 7,
    };
    play.set_live_map(&WorldMapViewSnapshot {
        header: MapViewHeader {
            content_generation: [1; 32],
            bundle_digest: [2; 32],
            reset_epoch: 1,
            origin: position,
            first_visible_floor: 7,
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
fn review_successive_minimap_owners_preserve_each_others_markers()
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
        floor: 7,
    };
    let snapshot = WorldMapViewSnapshot {
        header: MapViewHeader {
            origin: position,
            first_visible_floor: 7,
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
        let mut play = PlayView::new(TileCoord::new(100, 200), 7, &[])?;
        lock(&play.minimap).root = Some(root.clone());
        play.bind_minimap_world("11111111-1111-4111-8111-111111111111");
        play.set_live_map(&snapshot)?;
        Ok(play)
    };
    let mut first = open()?;
    let mut distant = snapshot.clone();
    distant.header.origin.x = 1000;
    distant.tiles[0].position.x = 1000;
    first.set_live_map(&distant)?;
    lock(&first.minimap).save()?;
    assert!(lock(&first.minimap).create_mark_at((100, 200, 7))?);
    drop(first);
    let second = open()?;
    assert!(lock(&second.minimap).create_mark_at((101, 200, 7))?);
    drop(second);
    let mut reopened = open()?;
    assert!(lock(&reopened.minimap).cells.contains_key(&(7, 1000, 200)));
    assert_eq!(
        lock(&reopened.minimap)
            .markers
            .iter()
            .map(|mark| mark.x)
            .collect::<Vec<_>>(),
        [100, 101]
    );
    lock(&reopened.minimap).remove_marker(100, 200, 7)?;
    distant.header.origin.x = 2000;
    distant.tiles.clear();
    reopened.set_live_map(&distant)?;
    lock(&reopened.minimap).save()?;
    lock(&reopened.minimap).edit_marker(101, 200, 7, "Gate".into(), 2)?;
    drop(reopened);
    let final_view = open()?;
    let map = lock(&final_view.minimap);
    assert!(map.cells.contains_key(&(7, 1000, 200)));
    assert_eq!(map.markers.len(), 1);
    assert_eq!(map.markers[0].label, "Gate");
    drop(map);
    drop(final_view);
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn review_truncated_live_map_preserves_cached_minimap_palette()
-> Result<(), Box<dyn std::error::Error>> {
    check_live_map_palette(
        false,
        false,
        "review_truncated_live_map_preserves_cached_minimap_palette",
    )
}

#[test]
fn review_resolved_top_item_sets_minimap_palette_above_unknown_appearance()
-> Result<(), Box<dyn std::error::Error>> {
    check_live_map_palette(
        true,
        false,
        "review_resolved_top_item_sets_minimap_palette_above_unknown_appearance",
    )
}

#[test]
fn review_hidden_floor_preserves_cached_minimap_palette() -> Result<(), Box<dyn std::error::Error>>
{
    check_live_map_palette(
        false,
        true,
        "review_hidden_floor_preserves_cached_minimap_palette",
    )
}

fn check_live_map_palette(
    unresolved_lower: bool,
    hidden_floor: bool,
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
        floor: 7,
    };
    let floor = if hidden_floor { 6 } else { 7 };
    let visible = ActorPosition {
        x: position.x + i32::from(floor - position.floor),
        y: position.y + i32::from(floor - position.floor),
        floor,
    };
    let lower = if unresolved_lower { 0 } else { ground };
    let mut snapshot = WorldMapViewSnapshot {
        header: MapViewHeader {
            origin: position,
            first_visible_floor: floor,
            content_generation: [1; 32],
            bundle_digest: [2; 32],
            reset_epoch: 1,
        },
        tiles: vec![MapTile {
            position: visible,
            ground_speed: 100,
            more: false,
            items: vec![item(lower), item(top)],
        }],
    };
    let open = || -> Result<PlayView, oteryn_renderer::BatchError> {
        let mut view = PlayView::new(TileCoord::new(100, 200), 7, &[])?;
        lock(&view.minimap).root = Some(root.clone());
        view.bind_minimap_world("11111111-1111-4111-8111-111111111111");
        Ok(view)
    };
    let mut view = open()?;
    view.set_live_map(&snapshot)?;
    let key = (floor, visible.x, visible.y);
    assert_eq!(lock(&view.minimap).cells.get(&key), Some(&86));
    lock(&view.minimap).save()?;
    if hidden_floor {
        snapshot.header.first_visible_floor = 7;
        snapshot.tiles[0].items = vec![item(ground)];
        lock(&view.minimap).saved_at -= Duration::from_secs(5);
        view.set_live_map(&snapshot)?;
        drop(view);
        let mut rejoined = open()?;
        rejoined.set_live_map(&snapshot)?;
        assert_eq!(lock(&rejoined.minimap).cells.get(&key), Some(&86));
        snapshot.header.first_visible_floor = floor;
        rejoined.set_live_map(&snapshot)?;
        assert_eq!(lock(&rejoined.minimap).cells.get(&key), Some(&24));
        lock(&rejoined.minimap).save()?;
    } else if lower == ground {
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
            lock(&rejoined.minimap).cells.get(&(7, 100, 200)),
            Some(&86)
        );
    }
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn review_busy_cache_returns_from_live_map_and_marker_creation()
-> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::temp_dir().join(format!("minimap-busy-{}", std::process::id()));
    let world = "11111111-1111-4111-8111-111111111111";
    let position = ActorPosition {
        x: 100,
        y: 200,
        floor: 7,
    };
    let snapshot = WorldMapViewSnapshot {
        header: MapViewHeader {
            content_generation: [1; 32],
            bundle_digest: [2; 32],
            reset_epoch: 1,
            origin: position,
            first_visible_floor: 7,
        },
        tiles: vec![MapTile {
            position,
            ground_speed: 100,
            more: false,
            items: vec![],
        }],
    };
    let mut view = PlayView::new(TileCoord::new(100, 200), 7, &[])?;
    lock(&view.minimap).root = Some(root.clone());
    view.bind_minimap_world(world);
    let owner = review_view(&root)?;
    let (send, receive) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(
        move || -> Result<_, Box<dyn std::error::Error + Send + Sync>> {
            lock(&view.minimap).saved_at -= Duration::from_secs(5);
            view.set_live_map(&snapshot)?;
            let visible = lock(&view.minimap).cells.contains_key(&(7, 100, 200));
            let mark = lock(&view.minimap).create_mark_at((100, 200, 7));
            send.send((visible, mark))?;
            Ok((view, snapshot))
        },
    );
    let result = receive.recv_timeout(Duration::from_secs(2));
    drop(owner);
    let worker_result = worker.join().map_err(|_| "live map worker panicked")?;
    let (visible, mark) = result?;
    assert!(visible);
    assert_eq!(
        mark.err().map(|error| error.kind()),
        Some(io::ErrorKind::WouldBlock)
    );
    let (mut view, snapshot) = worker_result.map_err(|error| error.to_string())?;
    view.set_live_map(&snapshot)?;
    assert!(!lock(&view.minimap).cells_dirty);
    let refused = lock(&view.minimap).create_mark_at((100, 200, 7));
    assert_eq!(refused.err().map(|e| e.kind()), Some(io::ErrorKind::WouldBlock));
    drop(view);
    let later = review_view(&root)?;
    assert!(lock(&later.minimap).create_mark_at((100, 200, 7))?);
    drop(later);
    std::fs::remove_dir_all(root)?;
    Ok(())
}

fn review_snapshot(x: i32, ground: bool) -> WorldMapViewSnapshot {
    let position = ActorPosition {
        x,
        y: 200,
        floor: 7,
    };
    WorldMapViewSnapshot {
        header: MapViewHeader {
            origin: position,
            first_visible_floor: 7,
            content_generation: [1; 32],
            bundle_digest: [2; 32],
            reset_epoch: 1,
        },
        tiles: ground
            .then_some(MapTile {
                position,
                ground_speed: 100,
                more: false,
                items: vec![],
            })
            .into_iter()
            .collect(),
    }
}

fn review_view(root: &std::path::Path) -> Result<PlayView, oteryn_renderer::BatchError> {
    let mut view = PlayView::new(TileCoord::new(100, 200), 7, &[])?;
    lock(&view.minimap).root = Some(root.to_path_buf());
    view.bind_minimap_world("11111111-1111-4111-8111-111111111111");
    view.set_live_map(&review_snapshot(900, false))?;
    Ok(view)
}

#[test]
fn review_owner_teardown_preserves_old_binding() -> Result<(), Box<dyn std::error::Error>> {
    for switch_binding in [false, true] {
        let root = std::env::temp_dir().join(format!(
            "minimap-teardown-{}-{switch_binding}",
            std::process::id()
        ));
        let mut view = review_view(&root)?;
        view.set_live_map(&review_snapshot(100, true))?;
        let path = lock(&view.minimap).path().ok_or("missing cache path")?;
        let refused = Saved::lock_cache(&path);
        assert_eq!(refused.err().map(|e| e.kind()), Some(io::ErrorKind::WouldBlock));
        let (send, receive) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || -> Result<(), String> {
            if switch_binding {
                let mut next = review_snapshot(300, true);
                next.header.reset_epoch = 2;
                view.set_live_map(&next)
                    .map_err(|error| error.to_string())?;
                let refused = Saved::lock_cache(&path);
                assert_eq!(refused.err().map(|e| e.kind()), Some(io::ErrorKind::WouldBlock));
            }
            drop(view);
            send.send(()).map_err(|error| error.to_string())
        });
        let closed = receive.recv_timeout(Duration::from_secs(2));
        worker.join().map_err(|_| "teardown worker panicked")??;
        closed?;
        let reopened = review_view(&root)?;
        assert!(
            lock(&reopened.minimap).cells.contains_key(&(7, 100, 200)),
            "binding switch: {switch_binding}"
        );
        drop(reopened);
        std::fs::remove_dir_all(root)?;
    }
    Ok(())
}

#[test]
fn review_stale_clear_preserves_newer_restoration() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::temp_dir().join(format!("minimap-stale-clear-{}", std::process::id()));
    let mut first = review_view(&root)?;
    first.set_live_map(&review_snapshot(100, true))?;
    lock(&first.minimap).save()?;
    let mut second = review_view(&root)?;
    let empty = review_snapshot(100, false);
    first.set_live_map(&empty)?;
    second.set_live_map(&empty)?;
    first.set_live_map(&review_snapshot(200, true))?;
    lock(&first.minimap).save()?;
    first.set_live_map(&review_snapshot(100, true))?;
    lock(&first.minimap).save()?;
    assert!(lock(&first.minimap).create_mark_at((200, 200, 7))?);
    drop(first);
    let reopened = review_view(&root)?;
    let map = lock(&reopened.minimap);
    assert_eq!(map.cells.get(&(7, 100, 200)), Some(&55));
    assert_eq!(map.cells.get(&(7, 200, 200)), Some(&55));
    drop(map);
    drop((second, reopened));
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn review_empty_window_preserves_older_unseen_cache_cell() -> Result<(), Box<dyn std::error::Error>>
{
    let root = std::env::temp_dir().join(format!("minimap-unseen-clear-{}", std::process::id()));
    let mut first = review_view(&root)?;
    first.set_live_map(&review_snapshot(100, true))?;
    lock(&first.minimap).save()?;
    drop(first);
    let mut second = review_view(&root)?;
    second.set_live_map(&review_snapshot(200, false))?;
    assert!(lock(&second.minimap).create_mark_at((100, 200, 7))?);
    assert_eq!(lock(&second.minimap).cells.get(&(7, 100, 200)), Some(&55));
    let reopened = review_view(&root)?;
    assert_eq!(lock(&reopened.minimap).cells.get(&(7, 100, 200)), Some(&55));
    drop((second, reopened));
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn review_concurrent_exploration_limit_allows_marker_writes()
-> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::temp_dir().join(format!("minimap-merge-limit-{}", std::process::id()));
    let bootstrap = review_view(&root)?;
    let path = lock(&bootstrap.minimap)
        .path()
        .ok_or("missing cache path")?;
    let cells = (0..i32::try_from(MAX_CELLS - 1)?)
        .map(|index| [15, index / 65536, index % 65536, 55])
        .collect();
    drop(bootstrap);
    Saved {
        cells,
        ..Saved::default()
    }
    .write_to(&path)?;
    let mut first = review_view(&root)?;
    let mut second = review_view(&root)?;
    first.set_live_map(&review_snapshot(100, true))?;
    lock(&second.minimap).saved_at = Instant::now();
    second.set_live_map(&review_snapshot(200, true))?;
    lock(&first.minimap).save()?;
    assert!(lock(&first.minimap).create_mark_at((200, 200, 7))?);
    lock(&first.minimap).edit_marker(200, 200, 7, "Gate".into(), 1)?;
    assert!(lock(&second.minimap).cells.contains_key(&(7, 200, 200)));
    lock(&second.minimap).save()?;
    drop(first);
    let reopened = review_view(&root)?;
    let map = lock(&reopened.minimap);
    assert_eq!(map.cells.len(), MAX_CELLS);
    assert!(map.cells.contains_key(&(7, 100, 200)));
    assert_eq!(map.markers[0].label, "Gate");
    assert!(!map.cells.contains_key(&(7, 200, 200)));
    drop(map);
    drop((second, reopened));
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn monotonic_empty_view_cannot_erase_an_identical_confirmation()
-> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::temp_dir().join(format!("minimap-confirmed-{}", std::process::id()));
    let ground = review_snapshot(100, true);
    let mut first = review_view(&root)?;
    first.set_live_map(&ground)?;
    lock(&first.minimap).save()?;
    let mut second = review_view(&root)?;
    second.set_live_map(&review_snapshot(100, false))?;
    first.set_live_map(&review_snapshot(100, false))?;
    first.set_live_map(&review_snapshot(200, true))?;
    first.set_live_map(&ground)?;
    lock(&first.minimap).save()?;
    assert!(lock(&first.minimap).create_mark_at((200, 200, 7))?);
    drop(first);
    let reopened = review_view(&root)?;
    assert_eq!(lock(&reopened.minimap).cells.get(&(7, 100, 200)), Some(&55));
    drop((second, reopened));
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn monotonic_discovery_survives_clock_rollback() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::temp_dir().join(format!("minimap-clock-{}", std::process::id()));
    let view = review_view(&root)?;
    let cache = lock(&view.minimap).path().ok_or("cache path")?;
    let export = root.join("portable.json");
    std::fs::create_dir_all(&root)?;
    {
        let mut map = lock(&view.minimap);
        map.export_to(&export)?;
        map.import_from(&export)?;
    }
    drop(view);
    // An older client imported this cache before the host clock was corrected.
    let mut saved: serde_json::Value = serde_json::from_slice(&std::fs::read(&cache)?)?;
    saved["imported_at"] = u64::MAX.into();
    std::fs::write(&cache, serde_json::to_vec(&saved)?)?;
    let mut view = review_view(&root)?;
    view.set_live_map(&review_snapshot(100, true))?;
    assert!(lock(&view.minimap).create_mark_at((100, 200, 7))?);
    let reopened = review_view(&root)?;
    assert_eq!(lock(&reopened.minimap).cells.get(&(7, 100, 200)), Some(&55));
    drop((view, reopened));
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn monotonic_older_discovery_survives_empty_save() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::temp_dir().join(format!("minimap-older-discovery-{}", std::process::id()));
    let mut first = review_view(&root)?;
    let mut second = review_view(&root)?;
    first.set_live_map(&review_snapshot(100, true))?;
    first.set_live_map(&review_snapshot(200, false))?;
    second.set_live_map(&review_snapshot(100, false))?;
    assert!(!lock(&second.minimap).cells_dirty);
    lock(&second.minimap).save()?;
    assert!(lock(&first.minimap).create_mark_at((200, 200, 7))?);
    let reopened = review_view(&root)?;
    assert_eq!(lock(&reopened.minimap).cells.get(&(7, 100, 200)), Some(&55));
    drop((first, second, reopened));
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn monotonic_resolved_colour_wins_over_later_fallback() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_session::world_map::{MapDefinition, MapItem, MapOrigin};
    let test = "monotonic_resolved_colour_wins_over_later_fallback";
    if std::env::var_os("OTERYN_TEST_STACK_PALETTE").is_none() {
        let status = std::process::Command::new(std::env::current_exe()?)
            .args([test, "--nocapture"])
            .env("OTERYN_TEST_STACK_PALETTE", "1")
            .env(crate::map::ASSET_DIR_ENV, PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."))
            .status()?;
        assert!(status.success());
        return Ok(());
    }
    let root = std::env::temp_dir().join(format!("minimap-fallback-{}", std::process::id()));
    for resolved_first in [false, true] {
        let root = root.join(resolved_first.to_string());
        let mut owner = review_view(&root)?;
        let mut resolved = review_snapshot(100, true);
        resolved.tiles[0].items.push(MapItem {
            definition: MapDefinition::Item(std::num::NonZeroU32::MIN),
            count: 1,
            sub_type: 0,
            origin: MapOrigin::DisplayOnly,
            appearance_id: 1081,
        });
        let mut fallback = review_snapshot(100, true);
        fallback.tiles[0].more = true;
        lock(&owner.minimap).saved_at = Instant::now();
        if resolved_first {
            owner.set_live_map(&resolved)?;
            lock(&owner.minimap).save()?;
            owner.set_live_map(&fallback)?;
            lock(&owner.minimap).save()?;
        } else {
            owner.set_live_map(&fallback)?;
            lock(&owner.minimap).save()?;
            owner.set_live_map(&resolved)?;
            assert!(lock(&owner.minimap).create_mark_at((100, 200, 7))?);
        }
        assert_eq!(lock(&owner.minimap).cells.get(&(7, 100, 200)), Some(&86));
        drop(owner);
        let reopened = review_view(&root)?;
        assert_eq!(lock(&reopened.minimap).cells.get(&(7, 100, 200)), Some(&86));
    }
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn monotonic_teardown_saves_only_discoveries_without_a_journal() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::temp_dir().join(format!("minimap-journal-bounds-{}", std::process::id()));
    let mut view = review_view(&root)?;
    let path = lock(&view.minimap).path().ok_or("cache path")?;
    view.set_live_map(&review_snapshot(100, true))?;
    // Eight visible floors across disjoint windows exceeded 64 MiB of old clears.
    for x in (30_000..57_200).step_by(32) {
        let mut empty = review_snapshot(x, false);
        empty.header.first_visible_floor = 0;
        view.set_live_map(&empty)?;
    }
    drop(view);
    assert!(!path.with_extension("pending").exists());
    assert_eq!(Saved::read_from(&path)?.ok_or("missing cache")?.cells.len(), 1);
    let reopened = review_view(&root)?;
    assert_eq!(lock(&reopened.minimap).cells.get(&(7, 100, 200)), Some(&55));
    drop(reopened);
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn monotonic_identical_snapshots_and_marker_noops_skip_cache_reads()
-> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::temp_dir().join(format!("minimap-noop-{}", std::process::id()));
    let mut view = review_view(&root)?;
    let ground = review_snapshot(100, true);
    view.set_live_map(&ground)?;
    {
        let mut map = lock(&view.minimap);
        map.edit_marker(100, 200, 7, "Home".into(), 1)?;
        // Any attempted cache read will fail.
        let path = map.path().ok_or("cache path")?;
        std::fs::write(&path, b"unreadable cache")?;
        map.saved_at -= Duration::from_secs(5);
        drop(map);
        view.set_live_map(&ground)?;
        view.set_live_map(&ground)?;
        let mut map = lock(&view.minimap);
        assert!(!map.cells_dirty);
        assert!(map.feedback.is_empty());
        assert!(!map.create_mark_at((100, 200, 7))?);
        map.edit_marker(100, 200, 7, "Home".into(), 1)?;
        map.remove_marker(101, 200, 7)?;
    }
    drop(view);
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn monotonic_legacy_journal_cannot_override_the_cache()
-> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::temp_dir().join(format!("minimap-legacy-journal-{}", std::process::id()));
    let mut view = review_view(&root)?;
    view.set_live_map(&review_snapshot(100, true))?;
    lock(&view.minimap).save()?;
    let path = lock(&view.minimap).path().ok_or("cache path")?;
    drop(view);
    let mut saved: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    saved["applied_journals"] = serde_json::json!(["already-applied.json"]);
    std::fs::write(&path, serde_json::to_vec(&saved)?)?;
    let pending = path.with_extension("pending");
    std::fs::create_dir_all(&pending)?;
    std::fs::write(
        pending.join("legacy.json"),
        br#"[[[-7,100,200],[18446744073709551615,"Clear"]],[[-7,200,200],[1,{"Set":86}]]]"#,
    )?;
    let reopened = review_view(&root)?;
    let map = lock(&reopened.minimap);
    assert_eq!(map.cells.get(&(7, 100, 200)), Some(&55));
    assert!(!map.cells.contains_key(&(7, 200, 200)));
    assert!(pending.join("legacy.json").exists());
    drop(map);
    drop(reopened);
    std::fs::remove_dir_all(root)?;
    Ok(())
}

// Exclusive (read-only snapshot) owner cases.
#[path = "play_persistence_exclusive.rs"]
mod exclusive;
