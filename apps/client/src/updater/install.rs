use oteryn_platform_client::updates::UpdateError;
use std::{
    collections::BTreeSet,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};
fn archive(path: &Path) -> Result<zip::ZipArchive<fs::File>, UpdateError> {
    zip::ZipArchive::new(fs::File::open(path).map_err(|_| UpdateError::Storage)?)
        .map_err(|_| UpdateError::Verification)
}
fn entry_path(entry: &zip::read::ZipFile<'_, fs::File>) -> Result<PathBuf, UpdateError> {
    let name = entry.name();
    if name.contains(['\\', ':'])
        || name.split('/').any(|s| matches!(s, ".." | "."))
        || entry
            .unix_mode()
            .is_some_and(|m| m & 0o170000 != 0 && !matches!(m & 0o170000, 0o100000 | 0o040000))
    {
        return Err(UpdateError::Verification);
    }
    entry
        .enclosed_name()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or(UpdateError::Verification)
}
/// Compares all installed bytes with a freshly verified signed ZIP; no local checksum authority.
pub fn verify_archive(package: &Path, directory: &Path) -> Result<(), UpdateError> {
    let mut zip = archive(package)?;
    let mut seen = BTreeSet::new();
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|_| UpdateError::Verification)?;
        let relative = entry_path(&entry)?;
        if !seen.insert(relative.clone()) {
            return Err(UpdateError::Verification);
        }
        if entry.is_dir() {
            continue;
        }
        let path = directory.join(relative);
        let root = directory.canonicalize().map_err(|_| UpdateError::Storage)?;
        if !path
            .canonicalize()
            .map_err(|_| UpdateError::Verification)?
            .starts_with(root)
        {
            return Err(UpdateError::Verification);
        }
        let mut file = fs::File::open(path).map_err(|_| UpdateError::Verification)?;
        if file.metadata().map_err(|_| UpdateError::Storage)?.len() != entry.size() {
            return Err(UpdateError::Verification);
        }
        let mut expected = [0_u8; 65536];
        let mut actual = [0_u8; 65536];
        loop {
            let n = entry
                .read(&mut expected)
                .map_err(|_| UpdateError::Verification)?;
            if n == 0 {
                break;
            }
            file.read_exact(&mut actual[..n])
                .map_err(|_| UpdateError::Verification)?;
            if actual[..n] != expected[..n] {
                return Err(UpdateError::Verification);
            }
        }
    }
    fn collect(root: &Path, at: &Path, files: &mut BTreeSet<PathBuf>) -> Result<(), UpdateError> {
        for child in fs::read_dir(at).map_err(|_| UpdateError::Storage)? {
            let child = child.map_err(|_| UpdateError::Storage)?;
            let path = child.path();
            let kind = child.file_type().map_err(|_| UpdateError::Storage)?;
            if kind.is_symlink() {
                return Err(UpdateError::Verification);
            }
            if kind.is_dir() {
                collect(root, &path, files)?;
            } else {
                files.insert(
                    path.strip_prefix(root)
                        .map_err(|_| UpdateError::Verification)?
                        .to_owned(),
                );
            }
        }
        Ok(())
    }
    let mut actual = BTreeSet::new();
    collect(directory, directory, &mut actual)?;
    let expected: BTreeSet<_> = seen
        .into_iter()
        .filter(|p| directory.join(p).is_file())
        .collect();
    if actual != expected {
        return Err(UpdateError::Verification);
    }
    Ok(())
}
pub(super) fn install(
    package: &Path,
    directory: &Path,
    release: &str,
) -> Result<String, UpdateError> {
    if !super::release_directory(release) || release.contains('~') {
        return Err(UpdateError::Verification);
    }
    let releases = directory.join("releases");
    fs::create_dir_all(&releases).map_err(|_| UpdateError::Storage)?;
    let mut slot = release.to_owned();
    for n in 1..=u32::MAX {
        if !releases.join(&slot).exists() {
            break;
        }
        slot = format!("{release}~{n}");
    }
    let stage = releases.join(format!(".stage-{}", std::process::id()));
    fs::create_dir(&stage).map_err(|_| UpdateError::Storage)?;
    let result = (|| {
        let mut zip = archive(package)?;
        let total = (0..zip.len()).try_fold(0_u64, |sum, i| {
            zip.by_index(i)
                .ok()
                .and_then(|f| sum.checked_add(f.size()))
                .ok_or(UpdateError::Verification)
        })?;
        if fs2::available_space(&stage).map_err(|_| UpdateError::Storage)? < total {
            return Err(UpdateError::Storage);
        }
        let mut seen = BTreeSet::new();
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i).map_err(|_| UpdateError::Verification)?;
            let path = entry_path(&entry)?;
            // ZIP names are case-insensitive on supported Windows installations.
            if !seen.insert(path.to_string_lossy().to_lowercase()) {
                return Err(UpdateError::Verification);
            }
            let target = stage.join(path);
            if entry.is_dir() {
                fs::create_dir_all(target).map_err(|_| UpdateError::Storage)?;
                continue;
            }
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|_| UpdateError::Storage)?;
            }
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(target)
                .map_err(|_| UpdateError::Storage)?;
            let length = entry.size();
            let copied = std::io::copy(&mut (&mut entry).take(length.saturating_add(1)), &mut file)
                .map_err(|_| UpdateError::Verification)?;
            if copied != length {
                return Err(UpdateError::Verification);
            }
            file.sync_all().map_err(|_| UpdateError::Storage)?;
        }
        if !stage.join("oteryn-client.exe").is_file() {
            return Err(UpdateError::Verification);
        }
        verify_archive(package, &stage)?;
        fs::rename(&stage, releases.join(&slot)).map_err(|_| UpdateError::Storage)?;
        let mut pointer =
            fs::File::create(directory.join("current.next")).map_err(|_| UpdateError::Storage)?;
        writeln!(pointer, "{slot}").map_err(|_| UpdateError::Storage)?;
        pointer.sync_all().map_err(|_| UpdateError::Storage)?;
        fs::rename(
            directory.join("current.next"),
            directory.join("current.txt"),
        )
        .map_err(|_| UpdateError::Storage)?;
        Ok(slot)
    })();
    if stage.exists() {
        let _ = fs::remove_dir_all(stage);
    }
    result
}
