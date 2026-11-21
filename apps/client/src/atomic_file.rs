use std::{
    io::{self, Write},
    path::Path,
};

pub(crate) fn replace(path: &Path, bytes: &[u8], sync: bool) -> io::Result<()> {
    replace_with(path, sync, |file| file.write_all(bytes))
}

pub(crate) fn replace_with(
    path: &Path,
    sync: bool,
    write: impl FnOnce(&mut std::fs::File) -> io::Result<()>,
) -> io::Result<()> {
    let directory = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(directory)?;
    let mut temporary = tempfile::NamedTempFile::new_in(directory)?;
    write(temporary.as_file_mut())?;
    if sync {
        temporary.as_file().sync_all()?;
    }
    temporary.persist(path).map_err(|error| error.error)?;
    Ok(())
}
