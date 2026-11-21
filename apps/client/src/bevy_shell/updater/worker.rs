//! Background staging only: activation belongs to the launcher after client shutdown.
use oteryn_platform_client::updates::{UpdateClient, UpdateConfig, UpdateError};
use std::{
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, SyncSender},
};

pub(super) enum Event {
    Progress(String),
    Finished(Result<String, UpdateError>),
}
pub(super) fn installation(executable: &Path) -> Option<PathBuf> {
    let release = executable.parent()?;
    let releases = release.parent()?;
    let root = releases.parent()?;
    (releases.file_name()? == "releases" && root.join("current.txt").is_file())
        .then(|| root.to_path_buf())
}
pub(super) fn start(directory: PathBuf, download: bool) -> Result<Receiver<Event>, String> {
    let (tx, rx) = mpsc::sync_channel(8);
    std::thread::Builder::new()
        .name("client-updater".into())
        .spawn(move || {
            let result = tokio::runtime::Runtime::new()
                .map_err(|_| UpdateError::Unavailable)
                .and_then(|runtime| runtime.block_on(stage(&directory, download, &tx)));
            let _ = tx.send(Event::Finished(result));
        })
        .map_err(|_| "Could not start updater worker. Retry the check.".to_owned())?;
    Ok(rx)
}
async fn stage(
    directory: &Path,
    download: bool,
    tx: &SyncSender<Event>,
) -> Result<String, UpdateError> {
    let config = UpdateConfig::new(
        "stable",
        crate::updater::pinned_root("stable"),
        crate::updater::compiled_platform(),
        crate::updater::compiled_arch(),
    )?;
    let state = directory.join(".updates");
    std::fs::create_dir_all(&state).map_err(|_| UpdateError::Storage)?;
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(state.join("lock"))
        .map_err(|_| UpdateError::Storage)?;
    lock.try_lock().map_err(|_| UpdateError::Unavailable)?;
    let _ = tx.try_send(Event::Progress(
        "Checking signed stable release metadata…".into(),
    ));
    let update = UpdateClient::check(config, &state).await?;
    let status = if update.policy.current_release_id == crate::RELEASE_ID {
        "Up to date"
    } else {
        "Update available"
    };
    let report = format!(
        "{status}\nRunning: {}\nStable: {}\nRelease: {}\nPolicy: {}\nPackage: {} bytes",
        crate::RELEASE_ID,
        update.policy.current_version_display,
        update.policy.current_release_id,
        update.policy.update_mode,
        update.length
    );
    if !download {
        return Ok(report);
    }
    if fs2::available_space(directory).map_err(|_| UpdateError::Storage)? < update.length {
        return Err(UpdateError::Storage);
    }
    let package = state.join("package.zip");
    // A prior interrupted download is never resumed or treated as trusted bytes.
    match std::fs::remove_file(package.with_extension("partial")) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err(UpdateError::Storage),
    }
    let _ = tx.try_send(Event::Progress(format!(
        "Downloading {} bytes; verification pending…",
        update.length
    )));
    update
        .download_with_progress(&package, |received, total| {
            let _ = tx.try_send(Event::Progress(format!(
                "Downloading {received}/{total} bytes; verification pending…"
            )));
        })
        .await?;
    Ok(format!(
        "{report}\nDownload verified. The active release is unchanged.\nClose client and maintain to install or repair; the launcher rechecks trust."
    ))
}
