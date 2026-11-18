use std::{
    hash::{BuildHasher, RandomState},
    io,
    path::{Path, PathBuf},
};

pub(crate) struct TestDir(PathBuf);

impl TestDir {
    pub(crate) fn new() -> io::Result<Self> {
        Self::new_in(&std::env::temp_dir().canonicalize()?)
    }

    pub(crate) fn new_in(root: &Path) -> io::Result<Self> {
        let names = RandomState::new();
        reserve(
            (0..32)
                .map(|attempt| root.join(format!("oteryn-test-{:016x}", names.hash_one(attempt)))),
        )
    }

    #[cfg(bevy_shell)]
    pub(crate) fn keep(self) -> PathBuf {
        let Self(path) = &self;
        let path = path.clone();
        std::mem::forget(self);
        path
    }

    pub(crate) fn path(&self) -> &Path {
        &self.0
    }

    pub(crate) fn join(&self, name: impl AsRef<Path>) -> PathBuf {
        self.path().join(name)
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn reserve(paths: impl IntoIterator<Item = PathBuf>) -> io::Result<TestDir> {
    let mut builder = std::fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    for path in paths {
        match builder.create(&path) {
            Ok(()) => return Ok(TestDir(path)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "No unused fixture directory",
    ))
}

#[test]
fn fixture_reservation_leaves_existing_directory_alone() -> io::Result<()> {
    let parent = TestDir::new()?;
    let occupied = parent.join("occupied");
    let available = parent.join("available");
    std::fs::create_dir(&occupied)?;
    std::fs::write(occupied.join("sentinel"), b"untouched")?;
    let reserved = reserve([occupied.clone(), available.clone()])?;
    assert_eq!(reserved.path(), available);
    drop(reserved);
    assert!(!available.exists());
    assert_eq!(std::fs::read(occupied.join("sentinel"))?, b"untouched");
    Ok(())
}
