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
// Each valid encoded entry, including its separator, fits in 32 bytes.
// Bound each atomic flush well below the reader's byte limit.
const JOURNAL_CHUNK_ENTRIES: usize = MAX_BYTES as usize / (4 * 32);

#[derive(Deserialize)]
#[serde(untagged)]
enum JournalObservation {
    Current(CellChange),
    Legacy(serde::de::IgnoredAny, LegacyChange),
}

#[derive(Deserialize)]
enum LegacyChange {
    Discover,
    Set(u8),
    #[serde(rename = "Clear")]
    IgnoredClear,
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
            std::fs::TryLockError::WouldBlock => io::Error::new(
                io::ErrorKind::WouldBlock,
                "The minimap is busy in another client",
            ),
            std::fs::TryLockError::Error(error) => error,
        })?;
        Ok(lock)
    }

    pub(super) fn write_to(&self, path: &Path) -> io::Result<()> {
        write_atomic(path, self)
    }

    // Teardown cannot wait for another client. Publish the delta separately so
    // the next lock holder can fold it into the latest cache after this view is gone.
    pub(super) fn stage_changes(path: &Path, changes: &CellChanges) -> io::Result<()> {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let pending = path.with_extension("pending");
        // Randomize the process prefix so a reused PID cannot reuse a receipt.
        use std::{collections::hash_map::RandomState, hash::BuildHasher, sync::LazyLock};
        static NONCE: LazyLock<u64> = LazyLock::new(|| RandomState::new().hash_one("minimap"));
        let mut changes = changes.iter().peekable();
        while changes.peek().is_some() {
            let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let staged = pending.join(format!(
                "{}-{:016x}-{sequence}.json",
                std::process::id(),
                *NONCE,
            ));
            let chunk: Vec<_> = changes.by_ref().take(JOURNAL_CHUNK_ENTRIES).collect();
            write_atomic(&staged, &chunk)?;
        }
        Ok(())
    }

    pub(super) fn staged_names(path: &Path) -> io::Result<BTreeSet<String>> {
        let entries = match std::fs::read_dir(path.with_extension("pending")) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(BTreeSet::new()),
            Err(error) => return Err(error),
        };
        let mut names = BTreeSet::new();
        for entry in entries {
            if let Ok(name) = entry?.file_name().into_string()
                && name.ends_with(".json")
            {
                names.insert(name);
            }
        }
        Ok(names)
    }

    pub(super) fn staged_changes<'a>(
        path: &Path,
        names: impl Iterator<Item = &'a String>,
    ) -> io::Result<CellChanges> {
        let pending = path.with_extension("pending");
        let mut changes: CellChanges = BTreeMap::new();
        for name in names {
            let staged: Vec<(CellPosition, JournalObservation)> =
                read_json(std::fs::File::open(pending.join(name))?)?;
            for (key @ (floor, x, y), change) in staged {
                if !cache_position(i32::from(floor), x, y) {
                    return Err(io::Error::other(tr!("Invalid minimap file")));
                }
                let change = match change {
                    JournalObservation::Current(change) => change,
                    JournalObservation::Legacy(_, LegacyChange::Discover) => CellChange::Discover,
                    JournalObservation::Legacy(_, LegacyChange::Set(colour)) => CellChange::Set(colour),
                    JournalObservation::Legacy(_, LegacyChange::IgnoredClear) => continue,
                };
                change.merge(&mut changes, key);
            }
        }
        Ok(changes)
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
        let saved = Saved {
            cells: self
                .cells
                .iter()
                .map(|(&(floor, x, y), &color)| [i32::from(floor), x, y, i32::from(color)])
                .collect(),
            markers: self.markers.clone(),
            ..Saved::default()
        };
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
        let mut portable: Portable = read_json(std::fs::File::open(path)?)?;
        if self.world.as_ref() != Some(&portable.world) || self.binding != Some(portable.binding) {
            return Err(io::Error::other(tr!(
                "This minimap belongs to a different world or content generation"
            )));
        }
        portable.saved.validate()?;
        let _lock = Saved::lock_cache(&active)?;
        portable.saved.applied_journals.clear();
        portable.saved.write_to(&active)?;
        self.cells = saved_cells(portable.saved.cells);
        self.markers = portable.saved.markers;
        self.load_failed = false;
        self.cell_changes.clear();
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
        map.cells.insert((-7, 500, 500), 55);
        self.minimap = std::sync::Arc::new(std::sync::Mutex::new(map));
    }
}
