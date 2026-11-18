//! Portable exploration is bound to the same world and content generation as the active cache.
use super::*;
use std::{
    io::{BufWriter, Read, Write},
    path::Path,
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Portable {
    world: String,
    binding: Binding,
    saved: Saved,
}
pub(super) fn cache_busy() -> io::Error {
    io::Error::new(
        io::ErrorKind::WouldBlock,
        "The minimap is busy in another client",
    )
}

#[cfg(bevy_shell)]
impl crate::play::PlayView {
    pub(crate) fn require_minimap_preferences_writer(&self) -> io::Result<()> {
        let map = lock(&self.minimap);
        if let Some(path) = map.path()
            && matches!(map.cache_access.get(&path), Some(CacheAccess::ReadOnly(_)))
        {
            return map.require_cache_owner(&path);
        }
        Ok(())
    }
}

impl Saved {
    pub(super) fn lock_cache(path: &Path) -> io::Result<std::fs::File> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // The cache is atomically replaced, so lock a stable sibling across writes.
        let lock = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path.with_extension("lock"))?;
        lock.try_lock().map_err(|error| match error {
            std::fs::TryLockError::WouldBlock => cache_busy(),
            std::fs::TryLockError::Error(error) => error,
        })?;
        Ok(lock)
    }

    pub(super) fn write_to(&self, path: &Path) -> io::Result<()> {
        write_atomic(path, self)
    }
}

pub(super) fn read_json<T: serde::de::DeserializeOwned>(file: std::fs::File) -> io::Result<T> {
    if file.metadata()?.len() > MAX_BYTES {
        return Err(io::Error::other(tr!("Minimap file is too large")));
    }
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(io::Error::other(tr!("Minimap file is too large")));
    }
    serde_json::from_slice(&bytes).map_err(io::Error::other)
}

fn write_atomic(path: &Path, value: &impl Serialize) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("tmp");
    let result = (|| {
        let mut file = BufWriter::new(std::fs::File::create(&temporary)?);
        serde_json::to_writer(&mut file, value).map_err(io::Error::other)?;
        file.flush()?;
        file.get_ref().sync_all()?;
        drop(file);
        std::fs::rename(&temporary, path)
    })();
    if result.is_err() && temporary.is_file() {
        let _ = std::fs::remove_file(temporary);
    }
    result
}

impl Minimap {
    pub fn export_to(&self, path: &Path) -> io::Result<()> {
        let world = self
            .world
            .clone()
            .ok_or_else(|| io::Error::other(tr!("Join a world before exporting its minimap")))?;
        let binding = self
            .binding
            .ok_or_else(|| io::Error::other(tr!("Wait for the server map before exporting")))?;
        let active = self
            .path()
            .ok_or_else(|| io::Error::other("Minimap storage is unavailable"))?;
        self.require_cache_owner(&active)?;
        let saved = self.snapshot(self.markers.clone());
        saved.validate()?;
        let mut file = BufWriter::new(
            std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(path)?,
        );
        let result = serde_json::to_writer(
            &mut file,
            &Portable {
                world,
                binding,
                saved,
            },
        )
        .map_err(io::Error::other)
        .and_then(|()| file.flush())
        .and_then(|()| file.get_ref().sync_all());
        if result.is_err() {
            drop(file);
            let _ = std::fs::remove_file(path);
        }
        result
    }

    pub fn import_from(&mut self, path: &Path) -> io::Result<()> {
        let active = self.path().ok_or_else(|| {
            io::Error::other(tr!("Join a world with minimap storage before importing"))
        })?;
        self.require_cache_owner(&active)?;
        let portable: Portable = read_json(std::fs::File::open(path)?)?;
        if self.world.as_ref() != Some(&portable.world) || self.binding != Some(portable.binding) {
            return Err(io::Error::other(tr!(
                "This minimap belongs to a different world or content generation"
            )));
        }
        portable.saved.validate()?;
        portable.saved.write_to(&active)?;
        self.cells = saved_cells(portable.saved.cells);
        self.markers = portable.saved.markers;
        self.load_failed = false;
        self.cells_dirty = false;
        self.saved_at = Instant::now();
        Ok(())
    }
}

impl Saved {
    pub(super) fn validate(&self) -> io::Result<()> {
        if self.version != 1
            || self.cells.len() > MAX_CELLS
            || (!self.legacy_observed_at.is_empty()
                && self.legacy_observed_at.len() != self.cells.len())
            || self.markers.len() > MAX_MARKERS
            || self
                .cells
                .iter()
                .any(|[f, x, y, c]| !super::cache_position(*f, *x, *y) || u8::try_from(*c).is_err())
            || self.markers.iter().any(|m| {
                !(0..=65535).contains(&m.x)
                    || !(0..=65535).contains(&m.y)
                    || !(-15..=0).contains(&m.floor)
                    || m.color > 3
                    || m.label.trim().is_empty()
                    || m.label.len() > 128
                    || m.label.chars().any(char::is_control)
            })
        {
            return Err(io::Error::other(tr!("Invalid minimap file")));
        }
        Ok(())
    }
}

#[cfg(all(test, bevy_shell))]
impl crate::play::PlayView {
    /// Binds the minimap to a test world and content generation under `root`, with one
    /// remembered cell, as a joined world does.
    pub(crate) fn bind_test_minimap(&mut self, root: std::path::PathBuf) {
        let mut map = Minimap::default();
        map.root = Some(root);
        map.bind_world("11111111-1111-4111-8111-111111111111");
        map.binding = Some(([1; 32], [2; 32], 1));
        if let Err(error) = map.load() {
            map.load_failed = true;
            map.feedback = error.to_string();
        }
        map.cells.insert((-7, 500, 500), 55);
        self.minimap = std::sync::Arc::new(std::sync::Mutex::new(map));
    }
}
