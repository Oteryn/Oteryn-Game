use std::{
    fs::{File, OpenOptions},
    io::{self, Write},
    path::Path,
};

pub(crate) fn replace(path: &Path, bytes: &[u8], sync: bool) -> io::Result<()> {
    let directory = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(directory)?;
    let mut temporary = tempfile::NamedTempFile::new_in(directory)?;
    temporary.write_all(bytes)?;
    if sync {
        temporary.as_file().sync_all()?;
    }
    temporary.persist(path).map_err(|error| error.error)?;
    Ok(())
}

// Lock a separate inode, since publishing the data file replaces its inode.
pub(crate) fn lock(path: &Path) -> io::Result<File> {
    let file = open_lock(path)?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(file),
            Err(std::fs::TryLockError::WouldBlock) => {
                let remaining = deadline.saturating_duration_since(std::time::Instant::now());
                if remaining.is_zero() {
                    return Err(io::Error::new(
                        io::ErrorKind::WouldBlock,
                        "Local profile is busy",
                    ));
                }
                std::thread::sleep(remaining.min(std::time::Duration::from_millis(10)));
            }
            Err(error) => return Err(error.into()),
        }
    }
}

pub(crate) fn try_lock(path: &Path) -> io::Result<File> {
    let file = open_lock(path)?;
    file.try_lock().map_err(io::Error::from)?;
    Ok(file)
}

pub(crate) fn wait_for_lock(path: &Path) -> io::Result<File> {
    let file = open_lock(path)?;
    file.lock()?;
    Ok(file)
}

fn open_lock(path: &Path) -> io::Result<File> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path.with_added_extension("lock"))
}

/// Preserve the first malformed document while holding its publication lock.
pub(crate) fn preserve_malformed(path: &Path) -> io::Result<()> {
    let backup = path.with_added_extension("malformed");
    match std::fs::symlink_metadata(&backup) {
        Ok(_) => return Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    let mut source = File::open(path)?;
    let mut copy = tempfile::NamedTempFile::new_in(
        path.parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new(".")),
    )?;
    io::copy(&mut source, &mut copy)?;
    copy.as_file().sync_all()?;
    match copy.persist_noclobber(&backup) {
        Ok(_) => {
            let name = path
                .file_name()
                .unwrap_or(path.as_os_str())
                .to_string_lossy();
            let message =
                format!("Recovered damaged {name}. The original was saved as {name}.malformed.");
            #[cfg(any(test, bevy_shell))]
            if let Ok(mut notices) = MALFORMED_NOTICES.lock() {
                notices.push(message.clone());
            }
            eprintln!("{message}");
            Ok(())
        }
        Err(error) if error.error.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(error.error),
    }
}

#[cfg(any(test, bevy_shell))]
static MALFORMED_NOTICES: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

#[cfg(any(test, bevy_shell))]
pub(crate) fn take_malformed_notices() -> Vec<String> {
    MALFORMED_NOTICES
        .lock()
        .map(|mut notices| std::mem::take(&mut *notices))
        .unwrap_or_default()
}

#[cfg(test)]
#[test]
fn review_3376_malformed_backup_and_notice_are_created_once() -> io::Result<()> {
    let root = tempfile::tempdir()?;
    let path = root.path().join("profile.json");
    let backup = path.with_added_extension("malformed");
    let _lock = lock(&path)?;
    std::fs::write(&path, b"first damaged file")?;
    preserve_malformed(&path)?;
    assert_eq!(std::fs::read(&backup)?, b"first damaged file");
    let notices = take_malformed_notices();
    assert_eq!(notices.len(), 1);
    assert!(notices[0].contains("profile.json.malformed"));
    std::fs::write(&path, b"second damaged file")?;
    preserve_malformed(&path)?;
    assert_eq!(std::fs::read(&backup)?, b"first damaged file");
    assert!(take_malformed_notices().is_empty());
    Ok(())
}

#[cfg(test)]
#[test]
fn review_3376_held_profile_lock_has_a_deadline() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let path = root.path().join("profile.json");
    let held = lock(&path)?;
    let (send, receive) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        let _ = send.send(lock(&path).err().map(|error| error.kind()));
    });
    let observed = receive.recv_timeout(std::time::Duration::from_secs(2));
    drop(held);
    worker.join().map_err(|_| "lock thread panicked")?;
    assert!(matches!(observed, Ok(Some(io::ErrorKind::WouldBlock))));
    Ok(())
}
