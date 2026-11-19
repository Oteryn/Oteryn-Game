#[cfg(bevy_shell)]
#[test]
fn replacement_minimap_reloads_after_the_outgoing_writer_releases_ownership()
-> Result<(), Box<dyn std::error::Error>> {
    use std::sync::mpsc;
    let pool = bevy_tasks::IoTaskPool::get_or_init(|| {
        bevy_tasks::TaskPoolBuilder::new().num_threads(1).build()
    });
    let root = tempfile::tempdir()?;
    let mut map = Minimap::default();
    map.root = Some(root.path().to_path_buf());
    let world = "11111111-1111-4111-8111-111111111111";
    map.bind_world(world);
    let window = joined_window(100, 200);
    map.observe(&window);
    map.enable_background_save();
    let path = map.path().ok_or("active cache")?;
    let Some(CacheAccess::Owner { _lock }) = map.cache_access.get(&path) else {
        return Err("cache owner".into());
    };
    let lease = _lock.clone();
    let pending = map.snapshot(vec![Marker {
        x: 100,
        y: 200,
        floor: -7,
        label: "Outgoing".into(),
        color: 1,
    }]);
    let (release, blocked) = tokio::sync::oneshot::channel();
    let (finished, observed) = mpsc::channel();
    let outgoing = path.clone();
    map.cells_dirty = false;
    map.save_task = Some(pool.spawn(async move {
        blocked.await.map_err(io::Error::other)?;
        pending.write_to(&outgoing)?;
        drop(lease);
        let _ = finished.send(());
        Ok(())
    }));
    drop(map);
    let mut replacement = Minimap::default();
    replacement.root = Some(root.path().to_path_buf());
    replacement.bind_world(world);
    replacement.observe(&window);
    replacement.enable_background_save();
    assert!(replacement.is_read_only());
    release.send(()).map_err(|()| "writer dropped")?;
    observed.recv_timeout(Duration::from_secs(2))?;
    let deadline = Instant::now() + Duration::from_secs(2);
    while replacement.is_read_only() && Instant::now() < deadline {
        replacement.poll_background_save();
        std::thread::yield_now();
    }
    assert!(!replacement.is_read_only());
    assert_eq!(replacement.markers[0].label, "Outgoing");
    Ok(())
}

#[cfg(bevy_shell)]
#[test]
fn background_import_keeps_the_old_map_when_the_new_cache_cannot_be_written()
-> Result<(), Box<dyn std::error::Error>> {
    bevy_tasks::IoTaskPool::get_or_init(|| {
        bevy_tasks::TaskPoolBuilder::new().num_threads(1).build()
    });
    let root = tempfile::tempdir()?;
    let mut map = Minimap::default();
    map.root = Some(root.path().into());
    map.bind_world("11111111-1111-4111-8111-111111111111");
    map.observe(&joined_window(100, 200));
    let active = map.path().ok_or("cache path")?;
    assert!(map.create_mark_at((100, 200, -7))?);
    map.edit_marker(100, 200, -7, "Imported".into(), 1)?;
    let source = root.path().join("source.json");
    map.export_to(&source)?;
    map.edit_marker(100, 200, -7, "On disk".into(), 1)?;
    let previous = std::fs::read(&active)?;
    std::fs::create_dir(active.with_extension("tmp"))?;
    map.enable_background_save();
    assert!(map.start_background_import(&source)?);
    assert_eq!(map.markers[0].label, "On disk");
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(result) = map.poll_background_import() {
            assert!(result.is_err());
            break;
        }
        if Instant::now() >= deadline {
            return Err("import did not finish".into());
        }
        std::thread::yield_now();
    }
    assert_eq!(map.markers[0].label, "On disk");
    assert_eq!(std::fs::read(&active)?, previous);
    Ok(())
}
