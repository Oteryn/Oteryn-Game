#[cfg(all(test, bevy_shell))]
#[test]
fn foreground_preferences_apply_does_not_join_the_writer() -> Result<(), Box<dyn std::error::Error>> {
    use super::*;
    use std::{sync::mpsc, time::Duration};

    let pool = bevy_tasks::IoTaskPool::get_or_init(|| bevy_tasks::TaskPoolBuilder::new().num_threads(1).build());
    let folder = tempfile::tempdir()?;
    let mut store = PreferenceStore::defaults(folder.path().join("preferences.json"));
    let (release, blocked) = tokio::sync::oneshot::channel();
    store.save_task = Some(pool.spawn(async move {
        vec![blocked.await.map(|()| None).map_err(io::Error::other)]
    }));
    let (finished, observed) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let result = store.apply(ReferencePreferences::default());
        let _ = finished.send(result);
        store
    });
    let returned = observed.recv_timeout(Duration::from_secs(2));
    let _ = release.send(());
    let _store = worker.join().map_err(|_| "foreground apply")?;
    returned??;
    Ok(())
}
