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
    file.lock()?;
    Ok(file)
}

pub(crate) fn try_lock(path: &Path) -> io::Result<File> {
    let file = open_lock(path)?;
    file.try_lock().map_err(io::Error::from)?;
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
