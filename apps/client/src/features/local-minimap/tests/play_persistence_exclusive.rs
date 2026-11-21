use super::*;

#[test]
fn exclusive_snapshot_refuses_marker_edits_and_transfers() -> Result<(), Box<dyn std::error::Error>>
{
    use crate::play::SettingsState;
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};

    let root =
        std::env::temp_dir().join(format!("minimap-exclusive-snapshot-{}", std::process::id()));
    let owner = review_view(&root)?;
    lock(&owner.minimap).edit_marker(100, 200, 7, "Home".into(), 1)?;
    let mut reader = review_view(&root)?;
    let portable = root.join("portable.json");
    lock(&owner.minimap).export_to(&portable)?;
    lock(&owner.minimap).edit_marker(100, 200, 7, "Gate".into(), 2)?;

    #[cfg(bevy_shell)]
    let submitted = reader.edit_minimap_marker((100, 200, 7), "Home".into(), 1);
    #[cfg(not(bevy_shell))]
    let submitted = lock(&reader.minimap).edit_marker(100, 200, 7, "Home".into(), 1);
    assert_eq!(
        submitted.err().map(|e| e.kind()),
        Some(io::ErrorKind::WouldBlock)
    );
    assert!(reader.minimap_input(&[NormalizedInputEvent::Key {
        code: KeyCode::new(16)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::NONE.with(Modifier::Alt).with(Modifier::Shift),
        repeat: false,
    }]));
    {
        let mut map = lock(&reader.minimap);
        assert!(map.pending_edit.is_none());
        assert!(map.feedback.contains("another client"));
        assert_eq!(map.markers[0].label, "Home");
        let remove = map.remove_marker(100, 200, 7);
        assert_eq!(
            remove.err().map(|e| e.kind()),
            Some(io::ErrorKind::WouldBlock)
        );
    }
    let mut settings = SettingsState::new(None);
    settings.request_minimap_transfer(true, portable);
    let error = settings
        .apply_minimap_transfer(&mut reader)
        .err()
        .ok_or("import succeeded")?;
    assert!(error.to_string().contains("another client"));
    let output = root.join("reader.json");
    settings.request_minimap_transfer(false, output.clone());
    assert!(settings.apply_minimap_transfer(&mut reader).is_err());
    assert!(!output.exists());

    let active = lock(&owner.minimap).path().ok_or("cache path")?;
    let before = std::fs::read(&active)?;
    lock(&reader.minimap).saved_at -= Duration::from_secs(5);
    reader.set_live_map(&review_snapshot(201, true))?;
    assert_eq!(lock(&reader.minimap).cells.get(&(7, 201, 200)), Some(&55));
    assert_eq!(std::fs::read(&active)?, before);

    drop(owner);
    reader.set_live_map(&review_snapshot(200, true))?;
    assert_eq!(lock(&reader.minimap).cells.get(&(7, 200, 200)), Some(&55));
    lock(&reader.minimap).save()?;
    drop(reader);
    let later = review_view(&root)?;
    {
        let mut map = lock(&later.minimap);
        assert_eq!(map.markers[0].label, "Gate");
        assert!(!map.cells.contains_key(&(7, 200, 200)));
        assert!(!map.cells.contains_key(&(7, 201, 200)));
        map.edit_marker(100, 200, 7, "Later".into(), 0)?;
    }
    drop(later);
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn exclusive_reader_does_not_persist_character_exploration()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::{PreferenceStore, SettingsState};

    let root = std::env::temp_dir().join(format!("minimap-reader-settings-{}", std::process::id()));
    let owner = review_view(&root)?;
    let mut reader = review_view(&root)?;
    let base = root.join("settings.json");
    let character = [3; 16];
    let mut settings = SettingsState::new(Some(base.clone()));
    settings.select_character(Some(character));
    let path = PreferenceStore::character_path(&base, character);
    let before = std::fs::read(&path)?;
    settings.restore(&mut reader, [960, 720])?;
    reader.set_live_map(&review_snapshot(201, true))?;
    settings.restore(&mut reader, [960, 720])?;
    assert_eq!(lock(&reader.minimap).cells.get(&(7, 201, 200)), Some(&55));
    assert!(std::fs::read(path)? == before, "reader changed character settings");
    drop(reader);
    drop(owner);
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn exclusive_reader_loads_snapshot_when_lock_cannot_open()
-> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::temp_dir().join(format!("minimap-lock-open-{}", std::process::id()));
    let mut owner = review_view(&root)?;
    owner.set_live_map(&review_snapshot(201, true))?;
    let path = lock(&owner.minimap).path().ok_or("cache path")?;
    lock(&owner.minimap).edit_marker(100, 200, 7, "Home".into(), 1)?;
    drop(owner);
    let before = std::fs::read(&path)?;
    let lock_path = path.with_extension("lock");
    // A directory at the lock path forces open to fail even for privileged test runners.
    std::fs::remove_file(&lock_path)?;
    std::fs::create_dir(&lock_path)?;

    let mut reader = review_view(&root)?;
    assert_eq!(lock(&reader.minimap).cells.get(&(7, 201, 200)), Some(&55));
    assert_eq!(lock(&reader.minimap).markers[0].label, "Home");
    assert!(lock(&reader.minimap).create_mark_at((101, 200, 7)).is_err());
    reader.set_live_map(&review_snapshot(202, true))?;
    drop(reader);
    assert_eq!(std::fs::read(path)?, before);
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn exclusive_import_is_not_overwritten_by_pre_import_recovery()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::SettingsState;

    let root =
        std::env::temp_dir().join(format!("minimap-exclusive-import-{}", std::process::id()));
    let mut owner = review_view(&root)?;
    let path = lock(&owner.minimap).path().ok_or("cache path")?;
    std::fs::create_dir_all(&root)?;
    let portable = root.join("portable.json");
    {
        let mut map = lock(&owner.minimap);
        map.set_cell_colour((7, 100, 200), 24);
        map.export_to(&portable)?;
        map.set_cell_colour((7, 100, 200), 42);
    }
    let pending = path.with_extension("pending");
    std::fs::create_dir_all(&pending)?;
    std::fs::write(pending.join("old.json"), br#"[[[-7,100,200],{"Set":86}]]"#)?;
    let mut settings = SettingsState::new(None);
    settings.request_minimap_transfer(true, portable);
    assert!(settings.apply_minimap_transfer(&mut owner)?);
    assert_eq!(lock(&owner.minimap).cells.get(&(7, 100, 200)), Some(&24));
    drop(owner);
    let reopened = review_view(&root)?;
    assert_eq!(
        lock(&reopened.minimap).cells.get(&(7, 100, 200)),
        Some(&24)
    );
    drop(reopened);
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn exclusive_load_uses_only_the_cache_even_with_pending_recovery()
-> Result<(), Box<dyn std::error::Error>> {
    let root =
        std::env::temp_dir().join(format!("minimap-exclusive-recovery-{}", std::process::id()));
    let mut owner = review_view(&root)?;
    owner.set_live_map(&review_snapshot(100, true))?;
    lock(&owner.minimap).save()?;
    let path = lock(&owner.minimap).path().ok_or("cache path")?;
    let pending = path.with_extension("pending");
    std::fs::create_dir_all(&pending)?;
    std::fs::write(pending.join("old.json"), br#"[[[-7,200,200],{"Set":86}]]"#)?;
    // The old client allows this probe to own the lock temporarily. With session
    // ownership, the owner already holds it and the probe returns WouldBlock.
    let probe = Saved::lock_cache(&path);
    let mut reader = review_view(&root)?;
    assert!(!lock(&reader.minimap).cells.contains_key(&(7, 200, 200)));
    drop((owner, probe));
    lock(&reader.minimap).saved_at -= Duration::from_secs(5);
    reader.set_live_map(&review_snapshot(100, true))?;
    assert!(!lock(&reader.minimap).cells.contains_key(&(7, 200, 200)));
    let output = root.join("deferred.json");
    assert_eq!(
        lock(&reader.minimap)
            .export_to(&output)
            .err()
            .map(|e| e.kind()),
        Some(io::ErrorKind::WouldBlock)
    );
    assert!(!output.exists());
    let later = review_view(&root)?;
    assert!(!lock(&later.minimap).cells.contains_key(&(7, 200, 200)));
    drop((reader, later));
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn exclusive_busy_teardown_creates_no_journal() -> Result<(), Box<dyn std::error::Error>> {
    let root =
        std::env::temp_dir().join(format!("minimap-exclusive-teardown-{}", std::process::id()));
    let owner = review_view(&root)?;
    let path = lock(&owner.minimap).path().ok_or("cache path")?;
    let probe = Saved::lock_cache(&path);
    let mut reader = review_view(&root)?;
    reader.set_live_map(&review_snapshot(200, true))?;
    drop(reader);
    assert!(!path.with_extension("pending").exists());
    assert_eq!(
        probe.err().map(|e| e.kind()),
        Some(io::ErrorKind::WouldBlock)
    );
    drop(owner);
    std::fs::remove_dir_all(root)?;
    Ok(())
}
