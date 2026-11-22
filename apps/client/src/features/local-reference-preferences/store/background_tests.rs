#[cfg(all(test, bevy_shell))]
#[test]
fn foreground_preferences_apply_does_not_join_the_writer() -> Result<(), Box<dyn std::error::Error>>
{
    use super::*;
    use std::{sync::mpsc, time::Duration};

    let pool = bevy_tasks::IoTaskPool::get_or_init(|| {
        bevy_tasks::TaskPoolBuilder::new().num_threads(1).build()
    });
    let folder = tempfile::tempdir()?;
    let mut store = PreferenceStore::defaults(folder.path().join("preferences.json"));
    let (release, blocked) = tokio::sync::oneshot::channel();
    store.save_task = Some(
        pool.spawn(async move { vec![blocked.await.map(|()| None).map_err(io::Error::other)] }),
    );
    let (finished, observed) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let result = store.apply(ReferencePreferences::default());
        let _ = finished.send(result);
        store
    });
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    let returned = observed.recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()));
    let _ = release.send(());
    while !worker.is_finished() {
        if std::time::Instant::now() >= deadline {
            return Err("foreground preference apply worker stalled".into());
        }
        std::thread::yield_now();
    }
    let _store = worker.join().map_err(|_| "foreground apply")?;
    returned??;
    Ok(())
}

#[cfg(all(test, bevy_shell))]
#[test]
fn two_queued_restores_swap_the_active_and_backup_documents()
-> Result<(), Box<dyn std::error::Error>> {
    use super::*;
    use std::time::{Duration, Instant};
    let pool = bevy_tasks::IoTaskPool::get_or_init(|| {
        bevy_tasks::TaskPoolBuilder::new().num_threads(1).build()
    });
    let folder = tempfile::tempdir()?;
    let path = folder.path().join("preferences.json");
    let mut store = PreferenceStore::defaults(path.clone());
    let active = ReferencePreferences {
        audio_device: "Active device".into(),
        ..Default::default()
    };
    let backup = ReferencePreferences {
        audio_device: "Backup device".into(),
        ..Default::default()
    };
    store.apply(active.clone())?;
    PreferenceStore::export_to(&backup, &store.backup_path())?;
    let (send, receive) = tokio::sync::oneshot::channel::<()>();
    store.save_task = Some(
        pool.spawn(async move { vec![receive.await.map(|()| None).map_err(io::Error::other)] }),
    );
    store.restore_in_background(&active)?;
    store.restore_in_background(&active)?;
    send.send(()).map_err(|()| "writer dropped")?;
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(result) = store.poll_background_save() {
            for item in result {
                item?;
            }
            break;
        }
        if Instant::now() >= deadline {
            return Err("restore did not complete".into());
        }
        std::thread::yield_now();
    }
    assert_eq!(
        PreferenceStore::read_transfer(&path)?.audio_device,
        active.audio_device
    );
    assert_eq!(
        PreferenceStore::read_transfer(&store.backup_path())?.audio_device,
        backup.audio_device
    );
    Ok(())
}

#[cfg(all(test, bevy_shell))]
#[test]
fn review_3376_failed_background_edits_are_sent_again() -> Result<(), Box<dyn std::error::Error>> {
    use super::*;
    use std::time::{Duration, Instant};
    bevy_tasks::IoTaskPool::get_or_init(|| {
        bevy_tasks::TaskPoolBuilder::new().num_threads(1).build()
    });
    let root = tempfile::tempdir()?;
    let path = root.path().join("preferences.json");
    let mut store = PreferenceStore::load(path.clone())?;
    let blocked = path.with_added_extension("lock");
    std::fs::create_dir(&blocked)?;
    let mut edited = store.preferences().clone();
    edited.audio.master_volume = 24;
    for succeeds in [false, true] {
        store.apply_in_background(edited.clone())?;
        let deadline = Instant::now() + Duration::from_secs(2);
        let result = loop {
            if let Some(result) = store.poll_background_save() {
                break result;
            }
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        };
        assert_eq!(result.iter().all(Result::is_ok), succeeds);
        if !succeeds {
            std::fs::remove_dir(&blocked)?;
        }
    }
    assert_eq!(
        PreferenceStore::read_transfer(&path)?.audio.master_volume,
        24
    );
    Ok(())
}
