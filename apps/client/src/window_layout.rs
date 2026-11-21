//! Local window and HUD-pane geometry, independent of server state and general settings writes.
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, io, path::Path};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WindowLayout(BTreeMap<String, [i32; 4]>);

impl WindowLayout {
    pub fn load(path: &Path) -> io::Result<Self> {
        use io::Read;
        let file = match std::fs::File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(error) => return Err(error),
        };
        let mut bytes = Vec::new();
        file.take((1 << 20) + 1).read_to_end(&mut bytes)?;
        if bytes.len() > 1 << 20 {
            return Err(io::Error::other(tr!("Window layout exceeds 1 MiB")));
        }
        let layout: Self = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
        let mut checked = Self::default();
        for (id, bounds) in layout.0 {
            checked.record(&id, bounds)?;
        }
        Ok(checked)
    }

    #[must_use]
    pub fn bounds(&self, id: &str) -> Option<[i32; 4]> {
        self.0.get(id).copied()
    }

    pub fn record(&mut self, id: &str, [x, y, width, height]: [i32; 4]) -> io::Result<()> {
        if id.is_empty()
            || id.len() > 128
            || id.contains('\0')
            || !(-32768..=32768).contains(&x)
            || !(-32768..=32768).contains(&y)
            || !(1..=4096).contains(&width)
            || !(1..=4096).contains(&height)
            || (self.0.len() >= 128 && !self.0.contains_key(id))
        {
            return Err(io::Error::other(tr!("Invalid window geometry")));
        }
        self.0.insert(id.to_owned(), [x, y, width, height]);
        Ok(())
    }

    #[cfg(bevy_shell)]
    pub(crate) fn save_edits(&self, path: &Path, before: &Self) -> io::Result<()> {
        let _lock = crate::atomic_file::try_lock(path)?;
        let mut active = Self::load(path)?;
        for (id, &bounds) in &self.0 {
            let previous = before.bounds(id);
            if previous != Some(bounds) {
                let merged = match (previous, active.bounds(id)) {
                    (Some(previous), Some(latest)) => std::array::from_fn(|index| {
                        if bounds[index] == previous[index] {
                            latest[index]
                        } else {
                            bounds[index]
                        }
                    }),
                    _ => bounds,
                };
                active.record(id, merged)?;
            }
        }
        active.save(path)
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        let bytes = serde_json::to_vec(self).map_err(io::Error::other)?;
        crate::atomic_file::replace(path, &bytes, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(bevy_shell)]
    #[test]
    fn review_3376_geometry_merges_move_and_resize() -> io::Result<()> {
        let root = tempfile::tempdir()?;
        let path = root.path().join("window.json");
        let mut baseline = WindowLayout::default();
        baseline.record("window", [10, 20, 640, 480])?;
        baseline.save(&path)?;
        let mut moved = baseline.clone();
        moved.record("window", [31, 42, 640, 480])?;
        moved.save_edits(&path, &baseline)?;
        let mut resized = baseline.clone();
        resized.record("window", [10, 20, 800, 600])?;
        resized.save_edits(&path, &baseline)?;
        assert_eq!(
            WindowLayout::load(&path)?.bounds("window"),
            Some([31, 42, 800, 600])
        );
        Ok(())
    }

    #[test]
    fn layout_save_uses_its_own_staging_path() -> io::Result<()> {
        let root = tempfile::tempdir()?;
        let path = root.path().join("window.json");
        let in_progress = path.with_extension("layout.tmp");
        std::fs::write(&in_progress, b"another writer")?;
        let mut layout = WindowLayout::default();
        layout.record("window", [0, 0, 640, 480])?;
        layout.save(&path)?;
        assert_eq!(std::fs::read(&in_progress)?, b"another writer");
        assert_eq!(
            WindowLayout::load(&path)?.bounds("window"),
            Some([0, 0, 640, 480])
        );
        Ok(())
    }
}
