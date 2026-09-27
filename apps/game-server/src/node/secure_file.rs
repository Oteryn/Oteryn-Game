//! Descriptor-based, no-follow file checks and exclusive creation
//! (OPS-NODE-BOOT-01 D1/D2).
//!
//! Every check runs on the opened descriptor, never on the path, and every
//! failure rejects before content is read. Errors name the failed property,
//! never file content.

use rustix::fd::{AsFd, OwnedFd};
use rustix::fs::{AtFlags, CWD, FileType, Mode, OFlags, Uid};
use std::io::{Read, Write};
use std::path::{Component, Path};

/// What a checked file may be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileClass {
    /// Secret or credential material: owned by the reading user, no group or
    /// other permission bits (0600 or 0400).
    Secret,
    /// Trusted, non-secret input (configuration, public trust material):
    /// owned by the reading user or root, not writable by group or others.
    Trusted,
}

/// Closed failure classes. They never carry file content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileError {
    /// The path is not an absolute path to a named file.
    InvalidPath,
    /// The file or its directory could not be opened or read.
    Unavailable,
    /// The final component is a symbolic link.
    SymbolicLink,
    /// The file is not a regular file.
    NotRegular,
    /// The file or directory has an unexpected owner.
    Owner,
    /// The file or directory permissions are too broad.
    Mode,
    /// The parent directory is owned by another user or writable by others.
    Parent,
    /// The content exceeds its bound.
    TooLarge,
    /// Exclusive creation found an existing entry.
    Exists,
    /// The named entry does not exist.
    Missing,
}

const GROUP_OTHER_ALL: u32 = 0o077;
const GROUP_OTHER_WRITE: u32 = 0o022;
const STICKY: u32 = 0o1000;

/// The effective user of this process.
#[must_use]
pub fn effective_uid() -> u32 {
    rustix::process::geteuid().as_raw()
}

fn file_name(path: &Path) -> Result<(&Path, &str), FileError> {
    if !path.is_absolute() {
        return Err(FileError::InvalidPath);
    }
    let name = match path.components().next_back() {
        Some(Component::Normal(name)) => name.to_str().ok_or(FileError::InvalidPath)?,
        _ => return Err(FileError::InvalidPath),
    };
    let parent = path.parent().ok_or(FileError::InvalidPath)?;
    Ok((parent, name))
}

fn valid_entry_name(name: &str) -> bool {
    !name.is_empty() && name.len() <= 255 && name != "." && name != ".." && !name.contains('/')
}

/// Open a directory and require that it is owned by `owner` or root and not
/// writable by group or others.
fn open_checked_directory(path: &Path, owner: u32) -> Result<OwnedFd, FileError> {
    let directory = rustix::fs::openat(
        CWD,
        path,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|_| FileError::Unavailable)?;
    let stat = rustix::fs::fstat(&directory).map_err(|_| FileError::Unavailable)?;
    if (stat.st_uid != owner && stat.st_uid != 0) || stat.st_mode & GROUP_OTHER_WRITE != 0 {
        return Err(FileError::Parent);
    }
    Ok(directory)
}

fn check_file(stat: &rustix::fs::Stat, class: FileClass, owner: u32) -> Result<(), FileError> {
    if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile {
        return Err(FileError::NotRegular);
    }
    match class {
        FileClass::Secret => {
            if stat.st_uid != owner {
                return Err(FileError::Owner);
            }
            if stat.st_mode & GROUP_OTHER_ALL != 0 {
                return Err(FileError::Mode);
            }
        }
        FileClass::Trusted => {
            if stat.st_uid != owner && stat.st_uid != 0 {
                return Err(FileError::Owner);
            }
            if stat.st_mode & GROUP_OTHER_WRITE != 0 {
                return Err(FileError::Mode);
            }
        }
    }
    Ok(())
}

fn read_bounded(file: OwnedFd, max: usize) -> Result<Vec<u8>, FileError> {
    let mut bytes = Vec::new();
    std::fs::File::from(file)
        .take(u64::try_from(max).map_err(|_| FileError::TooLarge)? + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| FileError::Unavailable)?;
    if bytes.len() > max {
        return Err(FileError::TooLarge);
    }
    Ok(bytes)
}

/// Read one checked file of at most `max` bytes. `owner` is the expected
/// owning user (the reading process's user).
pub fn read_checked(
    path: &Path,
    class: FileClass,
    owner: u32,
    max: usize,
) -> Result<Vec<u8>, FileError> {
    let (parent, name) = file_name(path)?;
    let directory = open_checked_directory(parent, owner)?;
    read_in(&directory, name, class, owner, max)
}

/// Read one checked file relative to an already validated directory.
pub fn read_in(
    directory: &OwnedFd,
    name: &str,
    class: FileClass,
    owner: u32,
    max: usize,
) -> Result<Vec<u8>, FileError> {
    let (file, stat) = open_entry(directory, name).map_err(|error| match error {
        FileError::Missing => FileError::Unavailable,
        other => other,
    })?;
    check_file(&stat, class, owner)?;
    read_bounded(file, max)
}

/// Read one checked file relative to a validated directory when its owner is
/// any of `owners`. The owner is taken from the opened descriptor, never from
/// a separate path lookup. A missing entry is `Missing`.
pub fn read_in_owned_by_any(
    directory: &OwnedFd,
    name: &str,
    class: FileClass,
    owners: &[u32],
    max: usize,
) -> Result<Vec<u8>, FileError> {
    let (file, stat) = open_entry(directory, name)?;
    if !owners.contains(&stat.st_uid) {
        return Err(FileError::Owner);
    }
    check_file(&stat, class, stat.st_uid)?;
    read_bounded(file, max)
}

fn open_entry(directory: &OwnedFd, name: &str) -> Result<(OwnedFd, rustix::fs::Stat), FileError> {
    if !valid_entry_name(name) {
        return Err(FileError::InvalidPath);
    }
    let file = match rustix::fs::openat(
        directory,
        name,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
        Mode::empty(),
    ) {
        Ok(file) => file,
        Err(rustix::io::Errno::LOOP) => return Err(FileError::SymbolicLink),
        Err(rustix::io::Errno::NOENT) => return Err(FileError::Missing),
        Err(_) => return Err(FileError::Unavailable),
    };
    let stat = rustix::fs::fstat(&file).map_err(|_| FileError::Unavailable)?;
    Ok((file, stat))
}

/// Validate the operator state directory and every ancestor up to `/`, each
/// opened relative to its validated parent without following symbolic links.
/// Each is owned by root or `invoking`, and none is writable by group or
/// others, except a root-owned sticky ancestor. The directory itself must not
/// be writable by group or others.
pub fn open_state_directory(path: &Path, invoking: u32) -> Result<OwnedFd, FileError> {
    if !path.is_absolute() {
        return Err(FileError::InvalidPath);
    }
    let mut current = rustix::fs::openat(
        CWD,
        "/",
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|_| FileError::Unavailable)?;
    let components: Vec<_> = path.components().collect();
    let last = components.len().saturating_sub(1);
    for (index, component) in components.iter().enumerate() {
        match component {
            Component::RootDir => {}
            Component::Normal(name) => {
                current = match rustix::fs::openat(
                    &current,
                    *name,
                    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                    Mode::empty(),
                ) {
                    Ok(next) => next,
                    Err(rustix::io::Errno::LOOP | rustix::io::Errno::NOTDIR) => {
                        return Err(FileError::SymbolicLink);
                    }
                    Err(_) => return Err(FileError::Unavailable),
                };
            }
            _ => return Err(FileError::InvalidPath),
        }
        let stat = rustix::fs::fstat(&current).map_err(|_| FileError::Unavailable)?;
        if stat.st_uid != 0 && stat.st_uid != invoking {
            return Err(FileError::Owner);
        }
        let sticky_root = stat.st_uid == 0 && stat.st_mode & STICKY != 0;
        if stat.st_mode & GROUP_OTHER_WRITE != 0 && (index == last || !sticky_root) {
            return Err(FileError::Mode);
        }
    }
    Ok(current)
}

/// Create a new file relative to a validated directory: exclusive create,
/// no symbolic-link following, mode 0600. When `hand_to` names a user, the
/// ownership is transferred through the open descriptor before any content is
/// written. The file and directory are synchronized. A failure after creation
/// removes the partial file.
pub fn create_exclusive(
    directory: &OwnedFd,
    name: &str,
    hand_to: Option<u32>,
    content: &[u8],
) -> Result<(), FileError> {
    if !valid_entry_name(name) {
        return Err(FileError::InvalidPath);
    }
    let file = match rustix::fs::openat(
        directory,
        name,
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::from_raw_mode(0o600),
    ) {
        Ok(file) => file,
        Err(rustix::io::Errno::EXIST) => return Err(FileError::Exists),
        Err(_) => return Err(FileError::Unavailable),
    };
    let written = (|| {
        if let Some(owner) = hand_to {
            rustix::fs::fchown(&file, Some(Uid::from_raw(owner)), None)
                .map_err(|_| FileError::Owner)?;
        }
        let mut handle = std::fs::File::from(file);
        handle
            .write_all(content)
            .map_err(|_| FileError::Unavailable)?;
        handle.sync_all().map_err(|_| FileError::Unavailable)?;
        rustix::fs::fsync(directory.as_fd()).map_err(|_| FileError::Unavailable)
    })();
    if written.is_err() {
        let _ = rustix::fs::unlinkat(directory, name, AtFlags::empty());
        let _ = rustix::fs::fsync(directory.as_fd());
    }
    written
}

/// Remove one entry relative to a validated directory and synchronize it.
pub fn remove_in(directory: &OwnedFd, name: &str) -> Result<(), FileError> {
    if !valid_entry_name(name) {
        return Err(FileError::InvalidPath);
    }
    rustix::fs::unlinkat(directory, name, AtFlags::empty()).map_err(|_| FileError::Unavailable)?;
    rustix::fs::fsync(directory.as_fd()).map_err(|_| FileError::Unavailable)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use std::os::unix::fs::{PermissionsExt, symlink};
    use std::path::PathBuf;

    fn scratch(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "oteryn-secure-file-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|elapsed| elapsed.as_nanos())
                .unwrap_or_default()
        ));
        std::fs::create_dir(&path).expect("scratch directory");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))
            .expect("scratch mode");
        path
    }

    fn write(path: &Path, mode: u32) {
        // A prior read-only mode must not block the rewrite for a non-root user.
        if path.exists() {
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
                .expect("writable mode");
        }
        std::fs::write(path, b"value").expect("write");
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).expect("mode");
    }

    #[test]
    fn secret_files_require_owner_mode_regular_and_no_symlink() {
        let me = effective_uid();
        let directory = scratch("secret");
        let secret = directory.join("secret");
        write(&secret, 0o600);
        assert_eq!(
            read_checked(&secret, FileClass::Secret, me, 64),
            Ok(b"value".to_vec())
        );
        write(&secret, 0o400);
        assert!(read_checked(&secret, FileClass::Secret, me, 64).is_ok());
        for mode in [0o640, 0o604, 0o660, 0o644] {
            write(&secret, mode);
            assert_eq!(
                read_checked(&secret, FileClass::Secret, me, 64),
                Err(FileError::Mode)
            );
        }
        write(&secret, 0o600);
        // The file owner check, relative to a directory valid for `me`.
        let parent = open_checked_directory(&directory, me).expect("parent");
        assert_eq!(
            read_in(&parent, "secret", FileClass::Secret, me.wrapping_add(1), 64),
            Err(FileError::Owner)
        );
        assert_eq!(
            read_checked(&secret, FileClass::Secret, me, 4),
            Err(FileError::TooLarge)
        );
        let link = directory.join("link");
        symlink(&secret, &link).expect("symlink");
        assert_eq!(
            read_checked(&link, FileClass::Secret, me, 64),
            Err(FileError::SymbolicLink)
        );
        let nested = directory.join("nested");
        std::fs::create_dir(&nested).expect("nested");
        assert!(matches!(
            read_checked(&nested, FileClass::Secret, me, 64),
            Err(FileError::NotRegular | FileError::Unavailable)
        ));
        assert_eq!(
            read_checked(Path::new("relative"), FileClass::Secret, me, 64),
            Err(FileError::InvalidPath)
        );
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o770))
            .expect("parent mode");
        assert_eq!(
            read_checked(&secret, FileClass::Secret, me, 64),
            Err(FileError::Parent)
        );
        std::fs::remove_dir_all(&directory).expect("cleanup");
    }

    #[test]
    fn trusted_files_allow_read_but_not_write_by_others() {
        let me = effective_uid();
        let directory = scratch("trusted");
        let config = directory.join("config.toml");
        write(&config, 0o644);
        assert!(read_checked(&config, FileClass::Trusted, me, 64).is_ok());
        write(&config, 0o664);
        assert_eq!(
            read_checked(&config, FileClass::Trusted, me, 64),
            Err(FileError::Mode)
        );
        std::fs::remove_dir_all(&directory).expect("cleanup");
    }

    #[test]
    fn exclusive_creation_hands_off_and_never_overwrites() {
        let me = effective_uid();
        let directory = scratch("create");
        let handle = open_state_directory(&directory, me).expect("state directory");
        create_exclusive(&handle, "request", Some(me), b"exact").expect("create");
        assert_eq!(
            create_exclusive(&handle, "request", Some(me), b"other"),
            Err(FileError::Exists)
        );
        assert_eq!(
            read_in(&handle, "request", FileClass::Secret, me, 64),
            Ok(b"exact".to_vec())
        );
        let mode = std::fs::metadata(directory.join("request"))
            .expect("metadata")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
        assert_eq!(
            create_exclusive(&handle, "../escape", None, b""),
            Err(FileError::InvalidPath)
        );
        // The owner comes from the opened descriptor.
        assert_eq!(
            read_in_owned_by_any(&handle, "request", FileClass::Secret, &[me], 64),
            Ok(b"exact".to_vec())
        );
        assert_eq!(
            read_in_owned_by_any(
                &handle,
                "request",
                FileClass::Secret,
                &[me.wrapping_add(1)],
                64
            ),
            Err(FileError::Owner)
        );
        remove_in(&handle, "request").expect("remove");
        assert_eq!(
            read_in(&handle, "request", FileClass::Secret, me, 64),
            Err(FileError::Unavailable)
        );
        assert_eq!(
            read_in_owned_by_any(&handle, "request", FileClass::Secret, &[me], 64),
            Err(FileError::Missing)
        );
        std::fs::remove_dir_all(&directory).expect("cleanup");
    }

    #[test]
    fn state_directory_rejects_writable_or_symlinked_components() {
        let me = effective_uid();
        let directory = scratch("state");
        let state = directory.join("state");
        std::fs::create_dir(&state).expect("state");
        std::fs::set_permissions(&state, std::fs::Permissions::from_mode(0o700)).expect("mode");
        assert!(open_state_directory(&state, me).is_ok());
        std::fs::set_permissions(&state, std::fs::Permissions::from_mode(0o770)).expect("mode");
        assert_eq!(
            open_state_directory(&state, me).err(),
            Some(FileError::Mode)
        );
        std::fs::set_permissions(&state, std::fs::Permissions::from_mode(0o700)).expect("mode");
        let link = directory.join("link");
        symlink(&state, &link).expect("symlink");
        assert_eq!(
            open_state_directory(&link, me).err(),
            Some(FileError::SymbolicLink)
        );
        assert_eq!(
            open_state_directory(Path::new("relative"), me).err(),
            Some(FileError::InvalidPath)
        );
        std::fs::remove_dir_all(&directory).expect("cleanup");
    }
}
