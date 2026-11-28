use super::*;
#[test]
fn exploration_and_markers_survive_reopen_and_bindings_stay_separate()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = tempfile::tempdir()?;
    let root = fixture.path().join("minimap");
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
        floor: 7,
    };
    let mut snapshot = WorldMapViewSnapshot {
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
    map.observe(&snapshot);
    map.edit_marker(100, 200, 7, "Home".into(), 1)?;
    // A saved cell outside the live window retains its full palette on reopen.
    map.set_cell_colour((7, 1000, 200), 86);
    map.save()?;
    let binding = map.binding;
    let mut restored = Minimap::default();
    restored.root = Some(root.clone());
    restored.bind_world("11111111-1111-4111-8111-111111111111");
    restored.observe(&snapshot);
    assert_eq!(restored.markers[0].label, "Home");
    assert!(restored.cells.contains_key(&(7, 100, 200)));
    assert_eq!(restored.cells.get(&(7, 1000, 200)), Some(&86));
    snapshot.header.reset_epoch += 1;
    restored.observe(&snapshot);
    assert!(restored.markers.is_empty());
    assert_ne!(restored.binding, binding);
    drop(restored);
    drop(map);
    Ok(())
}
#[test]
fn empty_description_mark_persists_and_reloads() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let open = || {
        let mut map = Minimap::default();
        map.root = Some(root.path().to_owned());
        map.bind_world("11111111-1111-4111-8111-111111111111");
        map.observe(&joined_window(100, 200));
        map
    };
    let mut map = open();
    map.edit_marker(100, 200, 7, String::new(), 2)?;
    let expected = Marker {
        x: 100,
        y: 200,
        floor: 7,
        label: String::new(),
        color: 2,
    };
    drop(map);

    let reopened = open();
    assert_eq!(reopened.markers, [expected]);
    Ok(())
}

#[test]
fn camera_and_marker_validation_do_not_change_gameplay_positions()
-> Result<(), Box<dyn std::error::Error>> {
    let mut map = Minimap::default();
    map.follow(100, 200, 7);
    assert_eq!(map.position(15, 12), Some((100, 200, 7)));
    map.pan(5, -5);
    map.zoom = 4;
    assert_eq!(map.position(16, 12), Some((109, 195, 7)));
    map.floor = 6;
    assert_eq!(map.position(15, 12), Some((105, 195, 6)));
    assert!(map.edit_marker(0, 0, -2, "x".into(), 0).is_err());
    assert!(map.edit_marker(0, 0, 7, "\0".into(), 0).is_err());
    assert!(map.edit_marker(0, 0, 7, "é".repeat(65), 0).is_err());
    map.edit_marker(100, 200, 7, "Gate".into(), 2)?;
    let kept = map.markers.clone();
    assert!(
        map.edit_marker(100, 200, -5, "Invalid floor".into(), 0)
            .is_err()
    );
    assert_eq!(map.markers, kept);
    map.edit_marker(100, 200, 7, "Exit".into(), 3)?;
    assert_eq!(map.markers.len(), 1);
    assert_eq!(map.markers[0].label, "Exit");
    map.remove_marker(100, 200, 7)?;
    assert!(map.markers.is_empty());
    Ok(())
}
#[test]
fn visible_window_clears_empty_and_item_only_cells_and_keeps_distant_exploration() {
    use oteryn_session::{
        ActorPosition,
        world_map::{MapTile, MapViewHeader, WorldMapViewSnapshot},
    };
    let snapshot = |x, ground_speed| {
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
                ground_speed,
                more: false,
                items: vec![],
            }],
        }
    };
    let mut map = Minimap::default();
    map.observe(&snapshot(100, 100));
    map.observe(&snapshot(1000, 100));
    assert!(map.cells.contains_key(&(7, 100, 200)));
    assert!(map.cells.contains_key(&(7, 1000, 200)));
    let mut cleared = snapshot(1000, 100);
    cleared.tiles.clear();
    map.observe(&cleared);
    assert!(map.cells.contains_key(&(7, 100, 200)));
    assert!(!map.cells.contains_key(&(7, 1000, 200)));
    map.observe(&snapshot(100, 0));
    assert!(!map.cells.contains_key(&(7, 100, 200)));
}

#[test]
fn create_mark_hotkey_adds_mark_at_player_position() -> Result<(), Box<dyn std::error::Error>> {
    let mut map = Minimap::default();
    assert!(map.create_mark_at((100, 200, 7))?);
    assert_eq!(map.markers.len(), 1);
    assert_eq!(
        (map.markers[0].x, map.markers[0].y, map.markers[0].floor),
        (100, 200, 7)
    );
    assert_eq!(map.markers[0].label, "Mark");
    Ok(())
}
#[test]
fn create_mark_hotkey_edits_existing_mark() -> Result<(), Box<dyn std::error::Error>> {
    let mut map = Minimap::default();
    map.edit_marker(100, 200, 7, "Gate".into(), 2)?;
    assert!(!map.create_mark_at((100, 200, 7))?);
    assert_eq!(map.markers.len(), 1);
    assert_eq!(map.markers[0].label, "Gate");
    Ok(())
}
#[test]
fn t076_floor_and_zoom_controls_stop_following_and_recentre_on_the_player() {
    use oteryn_client_ui::minimap::{floor_down, floor_up, zoom_out};
    let mut map = Minimap::default();
    map.follow(100, 200, 7);
    map.tracking = false;
    map.floor = floor_down(map.floor);
    map.zoom = zoom_out(map.zoom);
    map.follow(100, 200, 7);
    assert_eq!((map.x, map.y, map.floor, map.zoom), (100, 200, 8, 2));
    assert_eq!(map.position(15, 12), Some((100, 200, 8)));
    assert_eq!(floor_up(map.floor), 7);
    map.follow(101, 200, 7);
    assert_eq!((map.x, map.y, map.floor, map.zoom), (101, 200, 7, 2));
    assert!(map.tracking);
}

#[test]
fn version_one_palette_survives_rejoin_save_and_transfer() -> Result<(), Box<dyn std::error::Error>>
{
    let fixture = tempfile::tempdir()?;
    let root = fixture.path().join("minimap-palette");
    let mut map = Minimap::default();
    map.root = Some(root.clone());
    map.bind_world("11111111-1111-4111-8111-111111111111");
    map.binding = Some(([1; 32], [2; 32], 1));
    let active = map.path().ok_or("active map")?;
    std::fs::create_dir_all(active.parent().ok_or("map directory")?)?;
    std::fs::write(
        &active,
        r#"{"version":1,"cells":[[-7,500,500,0],[-7,501,500,86]],"markers":[{"x":500,"y":500,"floor":-7,"label":"Home","color":1}]}"#,
    )?;
    map.load()?;
    map.edit_marker(502, 500, 7, "Gate".into(), 2)?;
    map.load()?;
    let portable = root.join("portable.json");
    map.export_to(&portable)?;
    map.cells.clear();
    map.markers.clear();
    map.import_from(&portable)?;
    assert_eq!(map.cells.get(&(7, 500, 500)), Some(&0));
    assert_eq!(map.cells.get(&(7, 501, 500)), Some(&86));
    assert_eq!(
        map.markers
            .iter()
            .map(|marker| marker.label.as_str())
            .collect::<Vec<_>>(),
        ["Home", "Gate"]
    );
    Ok(())
}

fn joined_window(x: i32, y: i32) -> WorldMapViewSnapshot {
    use oteryn_session::{
        ActorPosition,
        world_map::{MapDefinition, MapItem, MapOrigin, MapTile, MapViewHeader},
    };
    let origin = ActorPosition { x, y, floor: 7 };
    WorldMapViewSnapshot {
        header: MapViewHeader {
            content_generation: [1; 32],
            bundle_digest: [2; 32],
            reset_epoch: 1,
            origin,
            first_visible_floor: 7,
        },
        tiles: vec![MapTile {
            position: origin,
            ground_speed: 100,
            more: false,
            items: vec![MapItem {
                definition: MapDefinition::Item(std::num::NonZeroU32::MIN),
                count: 1,
                sub_type: 0,
                origin: MapOrigin::DisplayOnly,
                appearance_id: 0,
            }],
        }],
    }
}

#[test]
fn review_minimap_default_root_round_trips_a_marker() -> Result<(), Box<dyn std::error::Error>> {
    const CHILD: &str = "OTERYN_MINIMAP_ROOT_TEST";
    if std::env::var_os(CHILD).is_none() {
        let root = std::env::temp_dir().join(format!("minimap-root-{}", std::process::id()));
        let mut child = std::process::Command::new(std::env::current_exe()?);
        child
            .arg("review_minimap_default_root_round_trips_a_marker")
            .env(CHILD, "1")
            .env("XDG_CONFIG_HOME", &root)
            .env("HOME", &root)
            .env("LOCALAPPDATA", &root);
        if !cfg!(windows) {
            child.env_remove("LOCALAPPDATA");
        }
        let status = child.status()?;
        if root.exists() {
            std::fs::remove_dir_all(root)?;
        }
        assert!(status.success());
        return Ok(());
    }
    let mut map = Minimap::default();
    let world = format!(
        "{:032x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    );
    map.bind_world(&world);
    let snapshot = joined_window(100, 200);
    map.observe(&snapshot);
    assert!(map.create_mark_at((100, 200, 7))?);
    let mut reopened = Minimap::default();
    reopened.bind_world(&world);
    reopened.observe(&snapshot);
    assert_eq!(reopened.markers, map.markers);
    if let Some(root) = reopened.root.as_ref() {
        std::fs::remove_dir_all(root.join(world))?;
    }
    Ok(())
}

#[test]
fn review_minimap_live_window_preserves_cached_palette() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::temp_dir().join(format!("minimap-live-{}", std::process::id()));
    let world = "11111111-1111-4111-8111-111111111111";
    let mut map = Minimap::default();
    map.root = Some(root.clone());
    map.bind_world(world);
    let snapshot = joined_window(0, 0);
    map.observe(&snapshot);
    map.set_cell_colour((7, 0, 0), 86);
    map.edit_marker(0, 0, 7, "Home".into(), 1)?;
    drop(map);

    let mut reopened = Minimap::default();
    reopened.root = Some(root.clone());
    reopened.bind_world(world);
    let mut view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(0, 0), 7, &[])?;
    let shared = std::sync::Arc::new(std::sync::Mutex::new(reopened));
    view.replace_test_minimap(shared.clone());
    view.set_live_map(&snapshot)?;
    {
        let mut map = lock(&shared);
        map.edit_marker(1, 0, 7, "Gate".into(), 2)?;
        map.load()?;
        assert_eq!(map.cells.get(&(7, 0, 0)), Some(&86));
    }

    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn review_minimap_live_window_keeps_cache_readable_at_map_edges()
-> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::temp_dir().join(format!("minimap-edge-{}", std::process::id()));
    let mut map = Minimap::default();
    map.root = Some(root.clone());
    map.bind_world("11111111-1111-4111-8111-111111111111");
    map.observe(&joined_window(20, 20));
    map.edit_marker(20, 20, 7, "Home".into(), 1)?;
    let shared = std::sync::Arc::new(std::sync::Mutex::new(map));
    let mut view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(20, 20), 7, &[])?;
    view.replace_test_minimap(shared.clone());
    for (x, y) in [(-1_i32, 0_i32), (0, -1), (65536, 0), (0, 65536)] {
        let mut snapshot = joined_window(x.clamp(0, 65535), y.clamp(0, 65535));
        snapshot.tiles[0].position.x = x;
        snapshot.tiles[0].position.y = y;
        use oteryn_session::world_map::{decode_world_map_snapshot, encode_world_map_snapshot};
        let payload = encode_world_map_snapshot(&snapshot).map_err(|e| format!("{e:?}"))?;
        let decoded = decode_world_map_snapshot(&payload).map_err(|e| format!("{e:?}"))?;
        view.set_live_map(&decoded)?;
        lock(&shared).save()?;
    }
    {
        let mut map = lock(&shared);
        map.load()?;
        assert_eq!(map.markers[0].label, "Home");
        assert!(map.cells.contains_key(&(7, 20, 20)));
    }
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn minimap_periodic_save_backs_off_after_a_failed_write() -> Result<(), Box<dyn std::error::Error>>
{
    let root = tempfile::tempdir()?;
    let mut map = Minimap::default();
    map.root = Some(root.path().to_path_buf());
    map.bind_world("11111111-1111-4111-8111-111111111111");
    let snapshot = joined_window(20, 20);
    map.observe(&snapshot);
    let cache = map.path().ok_or("bound cache")?;
    let staging = cache.with_extension("tmp");
    std::fs::create_dir(&staging)?;
    map.saved_at -= Duration::from_secs(5);
    map.observe(&snapshot);
    assert!(map.feedback.contains("Could not save minimap"));
    std::fs::remove_dir(&staging)?;
    map.observe(&snapshot);
    assert!(
        !cache.exists(),
        "failed saves must not retry on the next map frame"
    );
    assert_eq!(map.retry_delay, Duration::from_secs(10));
    map.saved_at -= Duration::from_secs(10);
    std::fs::create_dir(&staging)?;
    map.observe(&snapshot);
    assert_eq!(map.retry_delay, Duration::from_secs(20));
    std::fs::remove_dir(&staging)?;
    map.saved_at -= Duration::from_secs(20);
    map.observe(&snapshot);
    assert!(cache.is_file());
    assert_eq!(map.retry_delay, Duration::from_secs(5));
    Ok(())
}

#[cfg(bevy_shell)]
#[test]
fn minimap_background_save_polls_the_failed_write_and_keeps_dirty_cells()
-> Result<(), Box<dyn std::error::Error>> {
    use bevy_tasks::{IoTaskPool, TaskPoolBuilder};
    IoTaskPool::get_or_init(|| TaskPoolBuilder::new().num_threads(1).build());
    let root = tempfile::tempdir()?;
    let mut map = Minimap::default();
    map.root = Some(root.path().to_path_buf());
    map.enable_background_save();
    map.bind_world("11111111-1111-4111-8111-111111111111");
    let snapshot = joined_window(20, 20);
    map.observe(&snapshot);
    let load_deadline = Instant::now() + Duration::from_secs(2);
    while map.background_io_pending() && Instant::now() < load_deadline {
        map.poll_background_save();
        std::thread::yield_now();
    }
    assert!(!map.background_io_pending(), "cache load did not finish");
    let cache = map.path().ok_or("bound cache")?;
    let staging = cache.with_extension("tmp");
    std::fs::create_dir(&staging)?;
    map.saved_at -= Duration::from_secs(5);
    map.observe(&snapshot);
    assert!(map.save_task.is_some());
    assert!(!map.feedback.contains("Could not save minimap"));
    let deadline = Instant::now() + Duration::from_secs(10);
    while map.save_task.is_some() && Instant::now() < deadline {
        map.poll_background_save();
        std::thread::yield_now();
    }
    assert!(map.save_task.is_none(), "save worker did not finish");
    assert!(map.feedback.contains("Could not save minimap"));
    assert!(map.cells_dirty);
    assert_eq!(map.retry_delay, Duration::from_secs(10));
    std::fs::remove_dir(&staging)?;
    map.save()?;
    assert!(cache.is_file());
    Ok(())
}

#[cfg(bevy_shell)]
fn hold_background_save(map: &mut Minimap) -> tokio::sync::oneshot::Sender<()> {
    let pool = bevy_tasks::IoTaskPool::get_or_init(|| {
        bevy_tasks::TaskPoolBuilder::new().num_threads(1).build()
    });
    let (release, blocked) = tokio::sync::oneshot::channel();
    map.save_task = Some(pool.spawn(async move { blocked.await.map_err(io::Error::other) }));
    release
}

#[cfg(bevy_shell)]
fn while_writer_is_held(
    mut map: Minimap,
    operation: impl FnOnce(&mut Minimap) -> io::Result<()> + Send + 'static,
) -> Result<Minimap, Box<dyn std::error::Error>> {
    let release = hold_background_save(&mut map);
    let (finished, observed) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        let result = operation(&mut map);
        let _ = finished.send((map, result));
    });
    let deadline = Instant::now() + Duration::from_secs(2);
    let returned = observed.recv_timeout(deadline.saturating_duration_since(Instant::now()));
    release.send(()).map_err(|()| "save worker dropped")?;
    while !worker.is_finished() {
        if Instant::now() >= deadline {
            return Err("minimap observation worker stalled".into());
        }
        std::thread::yield_now();
    }
    worker.join().map_err(|_| "minimap observation")?;
    let (map, result) = returned?;
    result?;
    Ok(map)
}

#[cfg(bevy_shell)]
fn poll_mark_save(map: &mut Minimap) -> io::Result<()> {
    let deadline = Instant::now() + Duration::from_secs(2);
    while map.save_task.is_some() {
        map.poll_background_save();
        if Instant::now() >= deadline {
            return Err(io::Error::other("marker save did not finish"));
        }
        std::thread::yield_now();
    }
    Ok(())
}

#[cfg(bevy_shell)]
#[test]
fn removing_a_persisted_mark_defers_the_cache_write() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let mut map = Minimap::default();
    map.root = Some(root.path().to_path_buf());
    map.bind_world("11111111-1111-4111-8111-111111111111");
    map.observe(&joined_window(100, 200));
    map.edit_marker(100, 200, 7, "Camp".into(), 0)?;
    let cache = map.path().ok_or("bound cache")?;
    map.enable_background_save();
    let mut map = while_writer_is_held(map, |map| {
        map.remove_marker(100, 200, 7)?;
        assert!(map.markers.is_empty());
        assert!(map.save_task.is_some());
        let cache = map.path().ok_or_else(|| io::Error::other("bound cache"))?;
        assert_eq!(
            Saved::read_from(&cache)?
                .ok_or_else(|| io::Error::other("saved cache"))?
                .markers
                .len(),
            1
        );
        Ok(())
    })?;
    poll_mark_save(&mut map)?;
    assert!(
        Saved::read_from(&cache)?
            .ok_or("saved cache")?
            .markers
            .is_empty()
    );
    Ok(())
}

#[cfg(bevy_shell)]
#[test]
fn creating_and_editing_persisted_marks_defer_the_cache_write()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let mut map = Minimap::default();
    map.root = Some(root.path().to_path_buf());
    map.bind_world("11111111-1111-4111-8111-111111111111");
    map.observe(&joined_window(100, 200));
    let cache = map.path().ok_or("bound cache")?;
    map.enable_background_save();
    let mut map = while_writer_is_held(map, |map| {
        assert!(map.create_mark_at((100, 200, 7))?);
        assert!(
            !map.path()
                .ok_or_else(|| io::Error::other("bound cache"))?
                .exists()
        );
        Ok(())
    })?;
    poll_mark_save(&mut map)?;
    let created = Saved::read_from(&cache)?.ok_or("created cache")?.markers;
    assert_eq!(created.len(), 1);
    assert_eq!(created[0].label, "Mark");
    let mut map = while_writer_is_held(map, move |map| {
        map.edit_marker(100, 200, 7, "Camp".into(), 3)?;
        assert_eq!(map.markers[0].label, "Camp");
        let cache = map.path().ok_or_else(|| io::Error::other("bound cache"))?;
        assert_eq!(
            Saved::read_from(&cache)?
                .ok_or_else(|| io::Error::other("created cache"))?
                .markers,
            created
        );
        Ok(())
    })?;
    poll_mark_save(&mut map)?;
    assert_eq!(
        Saved::read_from(&cache)?.ok_or("saved cache")?.markers[0].label,
        "Camp"
    );
    Ok(())
}

#[cfg(bevy_shell)]
#[test]
fn map_binding_change_does_not_join_the_background_save() -> Result<(), Box<dyn std::error::Error>>
{
    let root = tempfile::tempdir()?;
    let mut map = Minimap::default();
    map.root = Some(root.path().to_path_buf());
    map.bind_world("11111111-1111-4111-8111-111111111111");
    map.observe(&joined_window(100, 200));
    map.enable_background_save();
    let path = map.path().ok_or("outgoing cache")?;
    let _live_map = while_writer_is_held(map, |map| {
        map.edit_marker(100, 200, 7, "Outgoing".into(), 1)?;
        let mut next = joined_window(100, 200);
        next.header.reset_epoch += 1;
        map.observe(&next);
        Ok(())
    })?;
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(saved) = Saved::read_from(&path)? {
            assert_eq!(saved.markers[0].label, "Outgoing");
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err("outgoing minimap was not published".into());
        }
        std::thread::yield_now();
    }
}

#[cfg(bevy_shell)]
include!("background_io.rs");
